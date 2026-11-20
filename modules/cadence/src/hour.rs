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

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::Failure;

use crate::findings;
use crate::mesh::Mesh;
use crate::tracking::Tracking;
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
    let kept_at = folder.join("hour.json");
    let mut kept: Kept = fs::read_to_string(&kept_at)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    // Read once, so the misses measured and the words told for them come
    // from the same ledger; one not read whole leaves the misses unjudged.
    // A miss gone from it isn't carried, so the hour after judges the
    // ledger as it then stands rather than finding it gone again for good.
    let (misses, gone) = measures::ledger(&state_home.join("mission/misses.jsonl"), &kept.open);
    let whole = gone.as_ref().is_some_and(Vec::is_empty);
    kept.open
        .retain(|open| !gone.as_ref().is_some_and(|gone| gone.contains(open)));
    let mut measures = gather(
        &folder,
        &state_home,
        &misses,
        now,
        checkouts,
        &mut kept.fetched,
    )?;
    for m in measures
        .iter_mut()
        .filter(|m| !whole && m.class == Class::Miss)
    {
        m.target = None;
        // A torn line has already said so.
        if !m.says.ends_with(measures::UNREAD) {
            m.says.push_str(measures::UNREAD);
        }
    }
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
        // The kit's reader, which every tool matching mycelium's names shares.
        let here = terminal_kit::environment::machine();
        measures.extend(measures::cells(
            &sessions,
            quests.as_deref(),
            &here,
            &TimeZone::system(),
            now,
            fallback,
        ));
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
        measures.push(spent_beside_landed(&state_home, now));
        measures.extend(spend_watch_measures(now));
    }
    // Once every measure is read, a miss whose check names one stays open
    // until it judges, so Joel isn't told it's fixed before it is; one
    // closed before last hour stays so.
    measures::held(&misses, &mut measures, &kept.open);
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
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    for (owner, text) in told.into_iter().filter(|(owner, _)| !owner.is_empty()) {
        mesh.send(&message(&owner, "finding", text))?;
    }
    let routes = fs::read_to_string(state_home.join("pain-points/routes.json")).ok();
    let keeper = measures::keeper(routes.as_deref());
    for (whom, intent) in card_to(to, keeper.as_deref()) {
        mesh.send(&message(whom, intent, card.clone()))?;
    }
    // Kept once the card has gone, so a send that fails tells its findings
    // and "fixed" lines again next hour rather than never: an owner told
    // twice is cheaper than one never told.
    findings::keep(&ledger_at, &ledger)?;
    write_whole(&kept_at, &serde_json::to_string(&next)?)?;
    Ok(())
}

/// Whom the card goes to, and as what: the orchestrator for context, and
/// the keeper of Joel's wish list, which re-ranks it each hour by what
/// the card says (WCJQSG, 19 November), as a question, so its turn starts
/// on it. A keeper who is the orchestrator is sent it once.
fn card_to<'a>(to: &'a str, keeper: Option<&'a str>) -> Vec<(&'a str, &'static str)> {
    let mut whom = vec![(to, "fyi")];
    if let Some(keeper) = keeper.filter(|keeper| !keeper.is_empty() && *keeper != to) {
        whom.push((keeper, "question"));
    }
    whom
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
    fetched: &mut Vec<i64>,
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
    let card_text = fresh_card(folder, now);
    let [core_log, harness_log, machine_log] = axes::repository::mains(folder, now);
    let asks = text(&mission.join("asks.jsonl"));
    let mut all = vec![
        measures::checked(
            misses,
            folder,
            checkouts,
            &text(&state_home.join("integration/switches.jsonl")),
        )?,
        measures::slips(&asks, now, Tracking::His),
        measures::to_first_slice(&asks, now, Tracking::His),
        measures::slips(&asks, now, Tracking::Derived),
        measures::to_first_slice(&asks, now, Tracking::Derived),
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
    all.extend(rounds_per_merge(
        state_home,
        &core,
        &core_log,
        &harness_log,
        now,
    ));
    all.extend(measures::merge_to_on(&card_text, NET_RULES));
    all.extend(measures::unlocked(&core, checkouts, now, BUMPS, fetched));
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
    let apps = fs::read_to_string(state_home.join(measures::apps::FILE)).ok();
    all.extend(measures::apps::apps(apps.as_deref(), now));
    // The balance service names a day's file by the date here, so read yesterday's too.
    let day = |at| text(&state_home.join(format!("mycelium/balance/{}.jsonl", today(at))));
    let minutes = [day(now - DAY), day(now)].join("\n");
    all.extend(measures::balanced(&minutes, now, NET_RULES));
    let hearer = state_home.join("pain-points");
    let pains = fs::read_to_string(hearer.join("hour.json")).ok();
    let routes = fs::read_to_string(hearer.join("routes.json")).ok();
    let keeper = measures::keeper(routes.as_deref());
    let owner = keeper.as_deref().unwrap_or(measures::ASKS);
    all.push(measures::pains(pains.as_deref(), now, owner));
    Ok(all)
}

/// The council rounds each change merged to core's and the harness's
/// mains in the last day took, from the harness's review runs and the
/// fixes their findings led to, reading only the records written since
/// the oldest round that could count, with who landed each change from
/// the mesh's landing notes of the last day, read only when a merge is to
/// be filed. Each note's sender is taken to the quest its presence works
/// on, as the spend measure does, since a session can end before its
/// drag is due while its quest's cell answers.
fn rounds_per_merge(
    state_home: &Path,
    core_git: &Path,
    core: &str,
    harness: &str,
    now: i64,
) -> Vec<Measure> {
    use measures::rounds::{BEFORE, Main, rounds};
    let harness_git = axes::landings::harness_mirror();
    // Cadence keeps core's clone, so it may fetch into it; the harness's
    // mirror is its installer's, so it is only read.
    let core_moved = |merge: &str, witness: &str| moved(core_git, true, merge, witness);
    let harness_moved = |merge: &str, witness: &str| moved(&harness_git, false, merge, witness);
    let since = u64::try_from(now - DAY - BEFORE).map_or(std::time::UNIX_EPOCH, |s| {
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(s)
    });
    let review = state_home.join("pi/agentic-harness.pi/review");
    let written = |folder: &str| -> Vec<String> {
        fs::read_dir(review.join(folder))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .is_ok_and(|at| at >= since)
            })
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .collect()
    };
    let landed = || {
        let mesh = Mesh {
            root: state_home.join("pi/agentic-harness.pi"),
        };
        let notes = mesh.sent_since((now - DAY) * 1000);
        let presences = mesh.all();
        let mut landed = BTreeMap::new();
        for (quest, changes) in measures::spent::landed_by(
            notes.iter().map(|m| (m.from.as_str(), m.text.as_str())),
            |address| measures::spent::quest_of(&presences, address),
        ) {
            for change in changes {
                landed.insert(change, quest.clone());
            }
        }
        landed
    };
    let mains = [
        Main {
            repo: "github:Jitsusama/core.nix",
            log: core,
            moved: &core_moved,
        },
        Main {
            repo: "github:Jitsusama/agentic-harness.pi",
            log: harness,
            moved: &harness_moved,
        },
    ];
    rounds(
        &mains,
        &written("runs"),
        &written("fixes"),
        landed,
        now,
        measures::ASKS,
    )
}

/// Whether the change `merge` merged, its second parent, changed after a
/// round read `witness`, from the `clone` its log came from, `None` where
/// git can't say, which goes to the journal with why. Only full commit
/// names reach git, so none reads as an option.
fn moved(clone: &Path, fetch: bool, merge: &str, witness: &str) -> Option<bool> {
    let sha = |name: &str| name.len() == 40 && name.bytes().all(|b| b.is_ascii_hexdigit());
    if !sha(merge) || !sha(witness) {
        eprintln!("cadence: rounds: {merge} or {witness} is no full commit name");
        return None;
    }
    changed_since(clone, fetch, merge, witness)
        .inspect_err(|err| {
            eprintln!("cadence: rounds: can't tell whether {merge} moved after {witness}: {err}");
        })
        .ok()
}

/// Whether what the change `merge` merged does differs from what it did at
/// `witness`, each its whole diff from main with no lines of context or
/// line numbers. So a rebase, beside main's edits or not, a merge from
/// main and an empty commit leave it as it was, and a fix or a conflict
/// settled otherwise, in a rebase or a merge, changes it. A rebase leaves
/// the head a round read on no branch, so a clone that keeps only main
/// lacks it: with `fetch`, it is asked of origin by its name, as GitHub
/// gives a commit it holds.
fn changed_since(clone: &Path, fetch: bool, merge: &str, witness: &str) -> io::Result<bool> {
    let clone = clone.to_string_lossy();
    let git = |args: &[&str]| {
        let mut all = vec!["-C", &clone];
        all.extend_from_slice(args);
        command("git", &all)
    };
    let head = git(&["rev-parse", "--verify", "--quiet", &format!("{merge}^2")])?;
    if head.trim().eq_ignore_ascii_case(witness) {
        return Ok(false);
    }
    let commit = format!("{witness}^{{commit}}");
    if fetch && git(&["cat-file", "-e", &commit]).is_err() {
        git(&[
            "fetch",
            "--quiet",
            "--no-write-fetch-head",
            "origin",
            witness,
        ])?;
    }
    git(&["cat-file", "-e", &commit])?;
    let main = format!("{merge}^1");
    let base = git(&["merge-base", witness, &main])?;
    let change = |from: &str, to: &str| {
        let diff = git(&["diff", "-U0", "--no-color", "--no-ext-diff", from, to])?;
        io::Result::Ok(
            diff.lines()
                .filter(|line| !line.starts_with("@@") && !line.starts_with("index "))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
    };
    Ok(change(base.trim(), witness)? != change(&main, merge)?)
}

/// Each quest's spend today beside what it landed since midnight here,
/// from spend-watch's collection and the mesh's landing notes, each
/// sender taken to the quest its presence works on. spend-watch writes
/// the collection every few minutes and dates nothing in it, so one not
/// written within the hour, or written before midnight, is yesterday's or
/// stuck, and isn't read as today's.
fn spent_beside_landed(state_home: &Path, now: i64) -> Measure {
    let midnight = midnight_ms(now, &TimeZone::system());
    let at = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|r| PathBuf::from(r).join("spend-watch/collection.json"));
    let written = at
        .as_ref()
        .and_then(|at| fs::metadata(at).and_then(|m| m.modified()).ok())
        .and_then(|when| when.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_millis()).ok());
    let fresh = written.is_some_and(|ms| ms >= midnight && ms >= (now - 3600) * 1000);
    let collection = at
        .filter(|_| fresh)
        .and_then(|at| fs::read_to_string(at).ok());
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    let notes = mesh.sent_since(midnight);
    let presences = mesh.all();
    let landed = measures::spent::landed_by(
        notes.iter().map(|m| (m.from.as_str(), m.text.as_str())),
        |address| measures::spent::quest_of(&presences, address),
    );
    measures::spent::beside(collection.as_deref(), &landed, measures::ASKS)
}

/// What spend-watch's collection says at `now` of budgets: those billed
/// under what they logged, and those sessions couldn't draw on. It
/// rewrites its collection only when its text changes, so its heartbeat,
/// not the file's time, says whether it's current.
fn spend_watch_measures(now: i64) -> [Measure; 2] {
    let collection = spend_watch_file("collection.json");
    let status = spend_watch_file("status.json");
    [
        measures::unbilled::unbilled(
            collection.as_deref(),
            status.as_deref(),
            now,
            measures::ASKS,
        ),
        measures::unresolved::unresolved(
            collection.as_deref(),
            status.as_deref(),
            now,
            measures::ASKS,
        ),
    ]
}

/// One of the files spend-watch's daemon keeps in its runtime folder.
fn spend_watch_file(name: &str) -> Option<String> {
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    fs::read_to_string(runtime.join("spend-watch").join(name)).ok()
}

/// The last midnight in `zone` at or before `now`, in milliseconds; `now`
/// itself where it can't be told.
fn midnight_ms(now: i64, zone: &TimeZone) -> i64 {
    Timestamp::from_second(now)
        .and_then(|at| at.to_zoned(zone.clone()).start_of_day())
        .map_or(now * 1000, |day| day.timestamp().as_millisecond())
}

/// The tick's card in `folder`, by when its file was written, if fresh as
/// [`measures::card_if_fresh`] says, else nothing: the hour and the axes
/// both read it, and neither may repeat a stopped tick's waits.
pub fn fresh_card(folder: &Path, now: i64) -> String {
    let card = folder.join("card.txt");
    let drawn = fs::metadata(&card)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok());
    measures::card_if_fresh(fs::read_to_string(&card).unwrap_or_default(), drawn, now)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs git in `at` with a config of its own, so the test reads and
    /// signs with nobody's, giving what it printed.
    fn git(at: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(at)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "here")
            .env("GIT_AUTHOR_EMAIL", "here@example.org")
            .env("GIT_COMMITTER_NAME", "here")
            .env("GIT_COMMITTER_EMAIL", "here@example.org")
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// Commits `text` as `file` in `at`, giving the commit.
    fn commit(at: &Path, file: &str, text: &str) -> String {
        std::fs::write(at.join(file), text).expect("a file written");
        git(at, &["add", file]);
        git(at, &["commit", "-q", "-m", file]);
        git(at, &["rev-parse", "HEAD"])
    }

    #[test]
    fn a_change_moved_on_from_its_round_only_by_what_it_changes() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let dir = &root.path().join("here");
        std::fs::create_dir(dir).expect("a repository folder");
        git(dir, &["init", "-q", "-b", "main"]);
        // It gives a commit on no branch by its name, as GitHub does.
        git(dir, &["config", "uploadpack.allowAnySHA1InWant", "true"]);
        commit(dir, "shared", "one\ntwo\nthree\nfour\n");
        // A change making `edit`, read there, then given `after` and merged
        // to main past main's own `beside`, giving the merge and the read.
        let merged = |change: &str, edit: (&str, &str), beside: (&str, &str), after: &dyn Fn()| {
            git(dir, &["checkout", "-q", "-b", change, "main"]);
            let read = commit(dir, edit.0, edit.1);
            git(dir, &["checkout", "-q", "main"]);
            commit(dir, beside.0, beside.1);
            git(dir, &["checkout", "-q", change]);
            after();
            git(dir, &["checkout", "-q", "main"]);
            git(dir, &["merge", "-q", "--no-ff", "-m", change, change]);
            (git(dir, &["rev-parse", "HEAD"]), read)
        };
        let own =
            |change, after: &dyn Fn()| merged(change, (change, change), ("main", change), after);
        let rebase = || drop(git(dir, &["rebase", "-q", "main"]));
        let cases = [
            ("as read", own("as-read", &|| ()), Some(false)),
            (
                "merged from main",
                own("in", &|| {
                    git(dir, &["merge", "-q", "--no-ff", "-m", "in", "main"]);
                }),
                Some(false),
            ),
            ("rebased", own("re", &rebase), Some(false)),
            (
                "rebased beside main's edit",
                merged(
                    "near",
                    ("shared", "one\ntwo\nthree\nFOUR\n"),
                    ("shared", "ONE\ntwo\nthree\nfour\n"),
                    &rebase,
                ),
                Some(false),
            ),
            (
                "an empty commit",
                own("empty", &|| {
                    git(dir, &["commit", "-q", "--allow-empty", "-m", "nothing"]);
                }),
                Some(false),
            ),
            (
                "fixed",
                own("fixed", &|| {
                    commit(dir, "a-fix", "a fix");
                }),
                Some(true),
            ),
            (
                "fixed in a merge from main",
                own("evil", &|| {
                    git(dir, &["merge", "-q", "--no-ff", "--no-commit", "main"]);
                    commit(dir, "in-the-merge", "a fix");
                }),
                Some(true),
            ),
        ];
        for (case, (merge, read), want) in &cases {
            assert_eq!(moved(dir, false, merge, read), *want, "{case}");
        }
        // A clone of main alone lacks the head the rebased change was read
        // at, until it is fetched.
        let there = &root.path().join("there");
        let url = format!("file://{}", dir.display());
        let at = there.to_string_lossy();
        let clone = ["clone", "-q", "--bare", "--single-branch", &url, &at];
        git(root.path(), &clone);
        let (merge, read) = &cases[2].1;
        assert_eq!(
            moved(there, false, merge, read),
            None,
            "a witness the clone lacks"
        );
        assert_eq!(
            moved(there, true, merge, read),
            Some(false),
            "a witness fetched"
        );
        let (merge, _) = &cases[0].1;
        assert_eq!(
            moved(dir, true, merge, &"0".repeat(40)),
            None,
            "a witness nobody has"
        );
        assert_eq!(
            moved(dir, false, merge, "as-read"),
            None,
            "a witness that's no name"
        );
        // main's second parent is a real witness, so only the guard keeps
        // the ref from reading as a merge.
        let last = git(dir, &["rev-parse", "main^2"]);
        assert_eq!(
            moved(dir, false, "main", &last),
            None,
            "a merge that's no name"
        );
    }

    #[test]
    fn the_card_asks_the_wish_lists_keeper_besides_telling_the_orchestrator() {
        assert_eq!(
            card_to("ORCH", Some("KEEPER")),
            [("ORCH", "fyi"), ("KEEPER", "question")]
        );
        assert_eq!(card_to("ORCH", Some("ORCH")), [("ORCH", "fyi")], "once");
        assert_eq!(card_to("ORCH", None), [("ORCH", "fyi")]);
        assert_eq!(card_to("ORCH", Some("")), [("ORCH", "fyi")]);
    }

    #[test]
    fn today_starts_at_midnight_where_the_machine_is() {
        let toronto = TimeZone::fixed(jiff::tz::offset(-5));
        // 19:48:16 UTC on 18 November, 14:48 in Toronto.
        let now = 1_795_031_296;
        assert_eq!(
            midnight_ms(now, &toronto),
            1_794_978_000_000,
            "not 05:00 UTC, Toronto's midnight"
        );
        // 03:00 UTC on 19 November is still the 18th in Toronto.
        assert_eq!(
            midnight_ms(1_795_057_200, &toronto),
            1_794_978_000_000,
            "the day turned at midnight UTC rather than here"
        );
    }
}
