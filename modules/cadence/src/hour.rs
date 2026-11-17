//! `cadence hour`: the hourly loop Joel asked for on 14 November, "something
//! constantly evaluating how we can ship more and faster while also
//! delivering insane quality, both functionally, and by maintaining clean
//! zones". Each hour it measures what shipped, what broke, what slipped
//! and how clean the code is, each against a target, from what the
//! machine already keeps; names the one biggest drag and files it with
//! its owner in the loop's ledger, due a day on; says the next hour
//! whether it moved; and sends the card to the orchestrator. A miss Joel
//! found himself outranks every figure, and stays open until a check
//! that would have caught it merges.
//!
//! This file reads the measures and runs the hour; `card` composes what it
//! says and keeps, and `owing` files its drag with each owner.

mod card;
mod lines;
mod measures;
mod owing;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::Failure;

use crate::findings;
use crate::mesh::Mesh;
use crate::{DAY, axes, clock, command, failed, folders, message, now_ms, write_whole, wrongly};
use card::{Kept, compose};
use measures::{Class, Measure};
use owing::file;

/// Who moves the measures no file names an owner for: the merge net's
/// rules and its quarantine are cadence's keeper's, the net that finds a
/// broken check is its own owner's, and core's lock is the bumps' keeper's.
const NET_RULES: &str = "80ECO2";
const NET: &str = "D5QEKR";
const BUMPS: &str = "0FX11G";

/// `cadence hour [--to ADDRESS] [--checkout NAME=PATH]... [--realm NAME]...`:
/// prints the hour's card, and sends it to the address given, the
/// orchestrator's; one without `--to` is a look, keeping nothing. Each
/// checkout is another repository whose main says which of the misses'
/// checks merged, beside core's and the harness's; each realm's node says
/// whether a machine is crowded beside one with room.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Failure> {
    let mut to = None;
    let mut checkouts = Vec::new();
    let mut realms = Vec::new();
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--to", Some(address)) => to = Some(address),
            ("--checkout", Some(named)) if named.contains('=') => checkouts.push(named),
            ("--realm", Some(realm)) => realms.push(realm),
            _ => {
                return Err(wrongly(
                    "hour",
                    format!("{flag} isn't a flag it takes, or has no value"),
                ));
            }
        }
    }
    hour(to.as_deref(), &checkouts, &realms).map_err(failed)
}

fn hour(to: Option<&str>, checkouts: &[String], realms: &[String]) -> io::Result<()> {
    let (_, state_home, folder) = folders()?;
    let now = now_ms() / 1000;
    // Read once, so the misses measured and the words told for them come
    // from the same ledger.
    let misses = fs::read_to_string(state_home.join("mission/misses.jsonl")).unwrap_or_default();
    let mut measures = gather(&folder, &state_home, &misses, now, checkouts)?;
    // A realm whose node doesn't answer says nothing of its machines.
    if !realms.is_empty() {
        let answers: Vec<String> = realms
            .iter()
            .filter_map(|realm| command("mycelium", &["--realm", realm, "machines"]).ok())
            .collect();
        measures.push(measures::crowded(&answers, NET_RULES));
        // The cells are the orchestrator's to move where no orchestration
        // or parent owns one; a look by hand names no orchestrator.
        // A realm whose node doesn't answer is kept, unread, so the cells
        // aren't judged from the others alone.
        let sessions: Vec<(String, String)> = realms
            .iter()
            .map(|realm| {
                let answer =
                    command("mycelium", &["--realm", realm, "sessions"]).unwrap_or_default();
                (realm.clone(), answer)
            })
            .collect();
        let fallback = to.unwrap_or_default();
        let quests = quests();
        measures.extend(measures::cells(&sessions, quests.as_deref(), now, fallback));
        let (exit, out) = exit_and_out("mycelium-map-check", &["--json"]);
        measures.push(measures::mapped(exit, &out, fallback));
        let looks = looks(&state_home.join("pi/agentic-harness.pi/looks"));
        measures.push(measures::unasked(
            // The measure judges the last 3 days alone, so older files
            // aren't opened.
            &reports(now - measures::REPORTS_SINCE),
            looks.as_deref(),
            quests.as_deref(),
            now,
            fallback,
        ));
    }
    let kept_at = folder.join("hour.json");
    let kept: Kept = fs::read_to_string(&kept_at)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let said = measures::said(&misses, &TimeZone::system());
    let (card, next) = compose(
        &clock(now),
        &measures,
        &kept,
        &said,
        &format!("{} tomorrow", clock(now + DAY)),
    );
    print!("{card}");
    // A look by hand lacks the timer's checkouts, so it can read a closed
    // miss as open: only the run sending the card keeps the hour and owes.
    let Some(to) = to else {
        return Ok(());
    };
    let ledger_at = folder.join("findings.jsonl");
    let mut ledger = findings::read(&ledger_at);
    let told = file(&mut ledger, &measures, &today(now), now);
    findings::keep(&ledger_at, &ledger)?;
    write_whole(&kept_at, &serde_json::to_string(&next)?)?;
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    for (owner, text) in told.into_iter().filter(|(owner, _)| !owner.is_empty()) {
        mesh.send(&message(&owner, "finding", text))?;
    }
    mesh.send(&message(to, "fyi", card))?;
    Ok(())
}

/// Reads every measure from the machine: the orchestrator's misses and
/// asks, the net's quarantine, reverts and results, the tick's card, the
/// three mains, code health's hourly read, and the realm's balance.
fn gather(
    folder: &Path,
    state_home: &Path,
    misses: &str,
    now: i64,
    checkouts: &[String],
) -> io::Result<Vec<Measure>> {
    let text = |path: &Path| fs::read_to_string(path).unwrap_or_default();
    let mission = state_home.join("mission");
    let core = folder.join("core.git");
    let quarantine = command(
        "git",
        &[
            "-C",
            &core.to_string_lossy(),
            "show",
            "main:tools/merge-net/quarantine.json",
        ],
    )
    .unwrap_or_default();
    let card_text = text(&folder.join("card.txt"));
    let [core_log, harness_log, machine_log] = axes::repository::mains(folder, now);
    let mut all = vec![
        measures::checked(misses, folder, checkouts)?,
        measures::slips(&text(&mission.join("asks.jsonl")), now),
        measures::to_first_slice(&text(&mission.join("asks.jsonl")), now),
        measures::undated(&quarantine, &today(now), NET_RULES),
        measures::reverts(&text(&folder.join("reverted.json")), now, NET_RULES),
        measures::broken(
            &text(&state_home.join("net/results.jsonl")),
            now,
            NET,
            &measures::Main::read(&core, &quarantine),
            &measures::failed_in,
        ),
        landed(&core_log, &harness_log, &machine_log, now),
    ];
    all.extend(measures::merge_to_on(&card_text, NET_RULES));
    all.extend(measures::unlocked(&core, checkouts, now, BUMPS));
    if let Some((median, target)) = measures::card_median(&card_text, "Last commit to merge") {
        all.push(Measure {
            key: "commit-to-merge".to_owned(),
            class: Class::Ship,
            says: "median minutes from last commit to merge".to_owned(),
            now: median,
            target: Some(target),
            owner: NET_RULES.to_owned(),
            behind: Vec::new(),
        });
    }
    all.extend(measures::clean(&text(
        &state_home.join("code-health/hour.json"),
    )));
    // The balance service names a day's file by the date here, so read yesterday's too.
    let day = |at| text(&state_home.join(format!("mycelium/balance/{}.jsonl", today(at))));
    let minutes = [day(now - DAY), day(now)].join("\n");
    all.extend(measures::balanced(&minutes, now, NET_RULES));
    Ok(all)
}

/// Each quest's folder the harness keeps on this machine.
fn quest_folders() -> Option<Vec<PathBuf>> {
    let data = std::env::var_os("XDG_DATA_HOME").map_or_else(
        || Path::new(&std::env::var_os("HOME").unwrap_or_default()).join(".local/share"),
        PathBuf::from,
    );
    let store = data.join("pi/agentic-harness.pi/quest-workflow/quests");
    Some(
        fs::read_dir(store)
            .ok()?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect(),
    )
}

/// Every quest the harness keeps on this machine, from its README; none
/// where the store can't be read, so it doesn't read as no quests.
fn quests() -> Option<Vec<measures::Quest>> {
    Some(
        quest_folders()?
            .iter()
            .filter_map(|folder| fs::read_to_string(folder.join("README.md")).ok())
            .filter_map(|readme| measures::quest(&readme))
            .collect(),
    )
}

/// Every report and review the quests keep written since `since`, with
/// when each file was last written, since the frontmatter dates it only by
/// day; an older file isn't opened.
fn reports(since: i64) -> Vec<measures::Report> {
    quest_folders()
        .unwrap_or_default()
        .iter()
        .filter_map(|folder| fs::read_dir(folder.join("reports")).ok())
        .flatten()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let written = fs::metadata(&path).ok()?.modified().ok()?;
            let written = written
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            let written = i64::try_from(written).ok()?;
            (written >= since).then_some(())?;
            measures::report(&fs::read_to_string(&path).ok()?, written)
        })
        .collect()
}

/// The looks put on Joel's list, each its own file; none where the list
/// can't be read, so a missing folder doesn't read as nothing asked.
fn looks(folder: &Path) -> Option<Vec<serde_json::Value>> {
    let entries = fs::read_dir(folder).ok()?;
    Some(
        entries
            .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
            .filter_map(|text| serde_json::from_str(&text).ok())
            .collect(),
    )
}

/// A program's exit and what it printed, its exit none where it didn't
/// run, for a check whose failing exit still says something.
fn exit_and_out(program: &str, args: &[&str]) -> (Option<i32>, String) {
    Command::new(program).args(args).output().map_or_else(
        |_| (None, String::new()),
        |out| {
            (
                out.status.code(),
                String::from_utf8_lossy(&out.stdout).into_owned(),
            )
        },
    )
}

/// The date at `now` here, as `2026-11-14`.
fn today(now: i64) -> String {
    Timestamp::from_second(now)
        .map(|at| at.to_zoned(TimeZone::system()).date().to_string())
        .unwrap_or_default()
}

/// A measure for the card's and the owing's tests, said as "`key` said"
/// and owned by `OWNER`.
#[cfg(test)]
fn measure(key: &str, class: Class, now: f64, target: Option<f64>) -> Measure {
    Measure {
        key: key.to_owned(),
        class,
        says: format!("{key} said"),
        now,
        target,
        owner: "OWNER".to_owned(),
        behind: Vec::new(),
    }
}

/// The changes the three mains landed in the hour, shown unjudged: an
/// hour's count swings with the clock, so no one target holds for it.
fn landed(core: &str, harness: &str, machine: &str, now: i64) -> Measure {
    let [c, h, m] = [core, harness, machine].map(|log| measures::landed(log, now));
    Measure {
        key: "landed".to_owned(),
        class: Class::Ship,
        says: format!(
            "changes landed in the hour (core {c}, the harness {h}, the machine's repository {m})"
        ),
        now: f64::from(u32::try_from(c + h + m).unwrap_or(u32::MAX)),
        target: None,
        owner: String::new(),
        behind: Vec::new(),
    }
}
