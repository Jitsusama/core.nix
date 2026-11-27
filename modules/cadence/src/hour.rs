//! `cadence hour`: the hourly loop Joel asked for on 14 November, "something
//! constantly evaluating how we can ship more and faster while also
//! delivering insane quality, both functionally, and by maintaining clean
//! zones". Each hour it measures what shipped, what broke, what slipped
//! and how clean the code is, each against a target, from what the
//! machine already keeps; names the one biggest drag; files it and every
//! other measure past its target with its owner in the loop's ledger, due
//! a day on, put to each owner as one question; says the next hour
//! whether it moved; and sends the card to the orchestrator. A miss Joel
//! found himself outranks every figure, and stays open until a check
//! that would have caught it merges. The queues the work waits in show
//! apart, and the worst is asked for its next change once a day.
//!
//! This file reads the measures and runs the hour; `card` composes what it
//! says and keeps, `good` holds each measure to the definition of good,
//! `owing` files its drag with each owner, `put` asks each owner of it, `records` reads the queues'
//! records, and `reviews` the harness's review records.
//!
//! Long because the run reads each record the measures need and keeps the
//! hour, its ledger and its sends in order, and its tests sit beside it.

mod card;
mod falling;
mod good;
mod ledger;
mod lines;
mod measures;
mod moved;
mod owing;
mod parted;
mod put;
mod records;
mod reviews;
mod stuck;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::Failure;

use crate::findings;
use crate::mesh::{Mesh, Presence, Rosters, running};
use crate::tracking::Tracking;
use crate::{DAY, axes, clock, command, failed, folders, message, now_ms, write_whole, wrongly};
use card::{Kept, compose};
use measures::{Class, Measure};
use moved::{HARNESS, moved};
use owing::file;
use records::{RECORD_READ, aboard, tail};

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
/// whether a machine is crowded beside one with room, and where its
/// sessions run.
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
    let mut kept = Kept::last(&kept_at);
    // Read once, so the misses measured and the words told for them come
    // from the same ledger; one not read whole leaves the misses unjudged.
    // A miss gone from it isn't carried, so the hour after judges the
    // ledger as it then stands rather than finding it gone again for good.
    let (misses, gone) = measures::ledger(&state_home.join("mission/misses.jsonl"), &kept.open);
    let whole = gone.as_ref().is_some_and(Vec::is_empty);
    kept.open
        .retain(|open| !gone.as_ref().is_some_and(|gone| gone.contains(open)));
    // A miss its owner said was none isn't fixed, nor one carried into the
    // asks, so each leaves without Joel being told it is; the card shows a
    // none's why for a day, and the asks track a carried one's fix.
    measures::forget_unfixed(&misses, &mut kept.open);
    let mut measures = gather(&folder, &state_home, &misses, now, checkouts, &mut kept, to)?;
    if !whole {
        unread_misses(&mut measures);
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
        measures.push(measures::carried(&sessions, &answers, &here));
        measures.extend(measures::cells(
            &sessions,
            quests.as_deref(),
            &here,
            &TimeZone::system(),
            now,
            fallback,
        ));
        // A host just started still has cells the node hasn't listed again.
        measures::mid_resume(&mut measures, measures::hosts_started(now).as_deref(), now);
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
    // The loop itself, judged by its red lines falling against a week back,
    // counted once the misses are held, so it counts the lines the card
    // shows; only the definition's own count, which must hold this one to
    // its line too, comes after it.
    let this_hour = falling::hour(&measures, now);
    measures.push(falling::falling(&kept.red, this_hour));
    // Every measure read, each is held to its line on the definition of
    // good, and one with none, or another target, is counted.
    measures.push(good::undefined(&measures));
    let said = measures::said(&misses, &TimeZone::system());
    let (mut card, mut next) = compose(
        &clock(now),
        &measures,
        &kept,
        &said,
        &format!("{} tomorrow", clock(now + DAY)),
    );
    stuck::daily(&mut card, &mut next, &measures, now, &TimeZone::system());
    next.red = falling::kept(&kept.red, this_hour);
    print!("{card}");
    // A look by hand lacks the timer's checkouts, so it can read a closed
    // miss as open: only the run sending the card keeps the hour and owes.
    let Some(to) = to else {
        return Ok(());
    };
    let ledger_at = folder.join("findings.jsonl");
    let _held = findings::lock(&ledger_at)?;
    let mut ledger = findings::read(&ledger_at);
    let date = today(now);
    let told = file(&mut ledger, &measures, &date, now);
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    // A question, so the line wakes its owner and wants an answer. Its
    // lines carry its id only once it's gone, so a send that fails leaves
    // none naming a question nobody had.
    for (owner, text) in put::asks(told) {
        let ask = message(&owner, "question", text);
        mesh.send(&ask)?;
        put::mark_asked(&mut ledger, &owner, now, &ask.id);
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

/// The misses measured from a ledger not read whole, left unjudged, and
/// nor whether they name tests: an emptied ledger names none, so judging
/// it would close every owner's ask, to be filed again after.
fn unread_misses(measures: &mut [Measure]) {
    for m in measures
        .iter_mut()
        .filter(|m| m.class == Class::Miss || m.key == measures::UNTESTED)
    {
        m.target = None;
        // A torn line has already said so.
        if !m.says.ends_with(measures::UNREAD) {
            m.says.push_str(measures::UNREAD);
        }
    }
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

/// The net's quarantine on core's main, or nothing when it can't be read.
fn quarantine(core: &Path) -> String {
    command(
        "git",
        &[
            "-C",
            &core.to_string_lossy(),
            "show",
            "main:tools/merge-net/quarantine.json",
        ],
    )
    .unwrap_or_default()
}

/// Reads every measure from the machine: the orchestrator's misses and
/// asks, the work system's findings on the mesh, the net's quarantine,
/// reverts and results, the tick's card, the three mains, code health's
/// hourly read, and the realm's balance.
fn gather(
    folder: &Path,
    state_home: &Path,
    misses: &str,
    now: i64,
    checkouts: &[String],
    kept: &mut Kept,
    orchestrator: Option<&str>,
) -> io::Result<Vec<Measure>> {
    let text = |path: &Path| fs::read_to_string(path).unwrap_or_default();
    let mission = state_home.join("mission");
    let core = folder.join("core.git");
    let quarantine = quarantine(&core);
    let card_text = fresh_card(folder, now);
    let [core_log, harness_log, machine_log] = axes::repository::mains(folder, now);
    // Read even when what the last hour kept of it is lost, so the next
    // hour has this one's count to read against.
    let (asks, asks_unread) = ledger::whole(&mission.join("asks.jsonl"), &mut kept.asks, now);
    let asks_unread = kept.unread.clone().or(asks_unread);
    let switches = text(&state_home.join("integration/switches.jsonl"));
    // The work system's findings count against their owners, and one
    // naming none is the orchestrator's to assign.
    let findings = measures::findings(
        &state_home.join("pi/agentic-harness.pi"),
        folder,
        now,
        orchestrator,
    );
    let (checked, found) =
        measures::checked_beside(misses, folder, checkouts, &switches, findings.as_ref(), now)?;
    let mut all = vec![
        checked,
        measures::untested(misses, folder, checkouts, &switches),
        measures::slips(&asks, now, Tracking::His),
        measures::parked(&asks, Tracking::His),
        measures::told_late(&asks, now, Tracking::His),
        measures::to_first_slice(&asks, now, Tracking::His),
        measures::slips(&asks, now, Tracking::Derived),
        measures::parked(&asks, Tracking::Derived),
        measures::told_late(&asks, now, Tracking::Derived),
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
    all.extend(measures::said_none(misses, now));
    all.push(measures::cited::read(
        state_home,
        now,
        &today(now),
        orchestrator,
    ));
    all.push(measures::holds::read(state_home, &asks, now));
    all.extend(parted::dates_at_risk(state_home, &core, &asks, now));
    all.extend(found);
    let per_merge = rounds_per_merge(state_home, &core, &core_log, &harness_log, now);
    all.extend(per_merge);
    all.extend(measures::merge_to_on(&card_text, NET_RULES));
    let fetched = &mut kept.fetched;
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
    let wifid = measures::wifi::read(now);
    all.push(measures::wifi::video(wifid.as_deref(), &TimeZone::system()));
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
    let outcomes = tail(
        &state_home.join("pi/agentic-harness.pi/priority-workflow/outcomes.jsonl"),
        RECORD_READ,
    );
    let sends = tail(&state_home.join("nix-offload/there.jsonl"), RECORD_READ);
    let train = state_home.join("net/train");
    let landed = tail(&train.join("landed.jsonl"), RECORD_READ);
    let aboard = aboard(&train.join("riders"));
    all.extend(measures::queues::queues(
        &measures::queues::Records {
            outcomes: outcomes.as_deref(),
            sends: sends.as_deref(),
            landed: landed.as_deref(),
            aboard: aboard.as_deref(),
        },
        now,
    ));
    ledger::unless_whole(&mut all, asks_unread.as_deref());
    Ok(all)
}

/// The council rounds each change merged to core's and the harness's
/// mains in the last day took, from the harness's review runs, those it
/// mirrors from other machines under `runs-from`, and the fixes their
/// findings led to, reading only the records written since
/// the oldest round that could count, with who landed each change from
/// the mesh's landing notes of the last day, read only when a merge is to
/// be filed. A merge's rounds are owed to the session that merged it while
/// it is there, running or parked, since a quest's short id reaches the
/// quest's first session; once it has ended, to the quest its presence
/// worked on, whose cell answers for it.
fn rounds_per_merge(
    state_home: &Path,
    core_git: &Path,
    core: &str,
    harness: &str,
    now: i64,
) -> Vec<Measure> {
    use measures::rounds::{BEFORE, Main, rounds};
    // The harness's mirror is its installer's and keeps only refs, so a
    // head a rebase left behind is gone from it. Cadence asks its own
    // clone beside core's, made from the mirror the first time a late
    // round needs it, and fetches into that.
    let harness_git = core_git.with_file_name("harness.git");
    let mirror = axes::landings::harness_mirror();
    let mirror = mirror.to_string_lossy();
    let origin = crate::CORE;
    let core_moved = |merge: &str, witness: &str| moved(core_git, origin, origin, merge, witness);
    let harness_moved =
        |merge: &str, witness: &str| moved(&harness_git, &mirror, HARNESS, merge, witness);
    let since = u64::try_from(now - DAY - BEFORE).map_or(std::time::UNIX_EPOCH, |s| {
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(s)
    });
    let review = state_home.join("pi/agentic-harness.pi/review");
    let here = reviews::rounds(&review.join("runs"), since);
    // One folder for each machine whose rounds the harness mirrors here.
    let mirrored = reviews::mirrored(&review.join("runs-from"), since);
    let fixes = reviews::fixes(&review.join("fixes"), since);
    // A trouble names files a mirror chose, so a name holding a newline
    // could otherwise write a journal line that reads as cadence's own.
    for trouble in [&here, &mirrored, &fixes].iter().flat_map(|r| &r.trouble) {
        eprintln!(
            "cadence: rounds: {}",
            trouble.replace(char::is_control, "?")
        );
    }
    let landed = || {
        let mesh = Mesh {
            root: state_home.join("pi/agentic-harness.pi"),
        };
        let notes = mesh.sent_since((now - DAY) * 1000);
        let presences = mesh.all();
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map(|b| b.trim().to_owned())
            .unwrap_or_default();
        let rosters = mesh.rostered(now);
        let there = |address: &str| {
            presences
                .iter()
                .filter(|p| p.addresses.iter().any(|a| a == address))
                .max_by_key(|p| p.at.to_string())
                .is_some_and(|p| {
                    reachable(p, &boot, &rosters, |pid| {
                        Path::new(&format!("/proc/{pid}")).exists()
                    })
                })
        };
        landed_by_session(
            notes.iter().map(|m| (m.from.as_str(), m.text.as_str())),
            there,
            |address| measures::spent::quest_of(&presences, address),
        )
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
        &here,
        &mirrored,
        &fixes,
        landed,
        now,
        measures::ASKS,
    )
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

/// Each change the mesh's landing notes, given as sender and text, say
/// landed, beside whom its rounds are owed: the session that merged it,
/// which the mesh sends the note from, while it is `there`, since a
/// quest's short id reaches the quest's first session, not the one that
/// ran them; else the quest it worked on, by `quest_of`, which outlives it.
fn landed_by_session<'a>(
    notes: impl IntoIterator<Item = (&'a str, &'a str)>,
    there: impl Fn(&str) -> bool,
    quest_of: impl Fn(&str) -> String,
) -> BTreeMap<String, String> {
    let owed = |session: &str| {
        if there(session) {
            session.to_owned()
        } else {
            quest_of(session)
        }
    };
    let mut landed = BTreeMap::new();
    for (session, changes) in measures::spent::landed_by(notes, owed) {
        for change in changes {
            landed.insert(change, session.clone());
        }
    }
    landed
}

/// A measure for the card's, the owing's and the stuck runs' tests, said
/// as "`key` said" and owned by `OWNER`.
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

/// Whether the session `presence` describes can still be told of a merge's
/// rounds: running in this `boot` and held by its host, as `running` says,
/// since a pid from another boot may name some other process now; or
/// parked where it will start again: on its host's roster, read, which the
/// host resumes, or, held by no host, parked in this boot, since its
/// holder from an earlier one is gone. A host with no roster to read can't
/// say it holds one parked, so its rounds go to its quest instead.
fn reachable(
    presence: &Presence,
    boot: &str,
    rosters: &Rosters,
    process_exists: impl Fn(u32) -> bool,
) -> bool {
    let resumes = match &presence.hosted {
        Some(hosted) => rosters
            .holders(&presence.session)
            .contains(&hosted.host.as_str()),
        None => presence.owner.as_ref().and_then(|o| o.boot_id.as_deref()) == Some(boot),
    };
    (presence.state == "parked" && resumes) || running(presence, boot, rosters, process_exists)
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

    #[test]
    fn a_merges_rounds_go_to_the_session_that_merged_it_not_its_quest() {
        // 6EAC3DA6 works on ZINAWZ, whose short id reaches another session.
        let note = "6EAC3DA6 landed core.nix#6020, Keep the Shaper's Queues. \
                    Said by the mesh; feedback to 6EAC3DA6.";
        let quest_of = |address: &str| {
            if address == "6EAC3DA6" {
                "ZINAWZ".to_owned()
            } else {
                address.to_owned()
            }
        };
        assert_eq!(
            landed_by_session([("6EAC3DA6", note)], |a| a == "6EAC3DA6", quest_of),
            BTreeMap::from([("core.nix#6020".to_owned(), "6EAC3DA6".to_owned())]),
            "a merge's rounds went to someone other than the session that merged it"
        );
        assert_eq!(
            landed_by_session([("6EAC3DA6", note)], |a| a != "6EAC3DA6", quest_of),
            BTreeMap::from([("core.nix#6020".to_owned(), "ZINAWZ".to_owned())]),
            "a merge's rounds went to a session that has ended rather than its quest"
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
    fn a_merging_session_is_there_only_where_it_can_take_a_turn() {
        // A parked session's pi has exited, so only a busy one's runs.
        let presence = |state: &str, boot: &str, hosted: bool| -> Presence {
            let pid = if state == "parked" { 8 } else { 7 };
            let mut said = serde_json::json!({
                "session": "s1",
                "addresses": ["AB12"],
                "state": state,
                "owner": {"pid": pid, "bootId": boot},
            });
            if hosted {
                said["hosted"] = serde_json::json!({"host": "h", "conversation": "c"});
            }
            serde_json::from_value(said).expect("a presence the test writes")
        };
        let holding = |held: &[&str]| {
            Rosters(BTreeMap::from([(
                "h".to_owned(),
                Some(held.iter().map(|s| (*s).to_owned()).collect()),
            )]))
        };
        let (held, dropped) = (holding(&["s1"]), holding(&[]));
        let torn = Rosters(BTreeMap::from([("h".to_owned(), None)]));
        let up = |pid: u32| pid == 7;
        let cases = [
            ("busy", "now", false, &held, true, "running here"),
            ("busy", "then", false, &held, false, "from another boot"),
            ("busy", "now", true, &held, true, "running on its host"),
            ("busy", "now", true, &dropped, false, "its host dropped it"),
            ("parked", "now", false, &held, true, "parked this boot"),
            ("parked", "then", false, &held, false, "before a reboot"),
            ("parked", "then", true, &held, true, "on its host's roster"),
            ("parked", "now", true, &dropped, false, "off its roster"),
            ("parked", "then", true, &torn, false, "its roster unread"),
        ];
        for (state, boot, hosted, rosters, there, why) in cases {
            let presence = presence(state, boot, hosted);
            assert_eq!(reachable(&presence, "now", rosters, up), there, "{why}");
        }
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
