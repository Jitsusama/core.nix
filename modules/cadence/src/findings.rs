//! The loop's ledger of findings, kept as `findings.jsonl` in cadence's
//! folder, a line each in the shape the loop's other readings keep theirs
//! in, so one reader reads them all: each finding a day names stays open,
//! with its owner and the date its answer is due, until its owner has said
//! what fixed it and its figure has been back for two days. A figure back
//! alone closes nothing, since a number gone quiet isn't a fix shown: of
//! 1,282 findings closed by 23 November, 1,088 closed that way.
//! Each owner is told once a day of what's new for it, in one message,
//! and a finding still open past its date goes back to its subject's hub
//! (hubs.rs), or to the orchestrator once where no hub takes it, so
//! nothing a day found is lost and nobody is asked twice. A must a
//! quest owes, filed with `cadence owe` (owe.rs), is kept here too, and
//! stays open until its owner says it's done.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::day::{self, Day};
use crate::judge::Finding;
use crate::{DAY, holders, write_whole};

pub mod gone;
pub mod hubs;

/// One finding as the ledger keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The finding's key: a measure's, or `crashes:biome`.
    pub key: String,
    /// The mesh address it was given to, or none when no owner matched.
    pub owner: Option<String>,
    /// The date it first appeared, and the last one it was found on. A
    /// drag reopened takes the day it reopened as its first, so "open
    /// since" reads from then and the queue's once-a-day ask holds, and
    /// `reopened` counts the times before; a finding of cadence's own
    /// keeps its first day through a reopening.
    pub first: String,
    pub last: String,
    /// When the last day it was found on ended, in seconds since the epoch.
    pub last_to: i64,
    /// When its answer is due: the run a day after it first appeared.
    pub due: i64,
    /// What the latest day said of it.
    pub words: String,
    /// Which of the loop's axes it's on, where it's read from, and the
    /// rule that says it's fixed.
    pub axis: String,
    pub evidence: String,
    pub fell_when: String,
    pub open: bool,
    /// The date its owner was told of it, so a run again that day tells
    /// nobody twice.
    pub told: Option<String>,
    /// Whether the orchestrator has been told it's past its date.
    pub late: bool,
    /// What a must its owner filed is, such as `hole` for a known exposure
    /// not yet closed; none for a finding of cadence's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// How often a day found it again after it closed, so a fix that
    /// didn't hold counts against it rather than opening a second line.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reopened: u32,
    /// The subject whose hub it was put to once it went late, so its
    /// holder keeps it over the owner lines until it closes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub: Option<String>,
    /// Each it was given to once late, its owner then first, so it goes
    /// to each holder once and then to the orchestrator, never round them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub put_to: Vec<String>,
    /// The date a finding of cadence's own was first back two days
    /// running, waiting on its owner's word of what fixed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,
    /// What its owner said fixed it, or why it's expected, with
    /// `cadence owe --done`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<String>,
    /// The ids of the questions its owner was asked it in, the latest
    /// last, pruned to those within `ASKED_FOR` whenever the line is asked
    /// again, so each question of the two days the mesh is read over is
    /// known as the hour's. Nothing reads a
    /// reply against them yet: a line closes when its measure is back in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<String>,
    /// What the hour's line named as its owner's, each cell, file, miss
    /// or date by the name that stays while it's red, with when each was
    /// last asked, so none is asked again within a day of its own ask.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub names: BTreeMap<String, i64>,
    /// Whether the hour filed it as a red line, which nothing but its
    /// being past its target asks: those go late as one count, and which
    /// keep a late line of their own is listed once, at `owing::red`. Its
    /// kind on disk stays `drag`, so a cadence from before red lines
    /// closes, holds and reopens it as a drag rather than keeping it open.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub red: bool,
    /// When its owner closed a drag with `cadence owe --done`, in
    /// seconds, so one its measure counts over a week isn't filed again
    /// the next hour from the cases the fix came after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<i64>,
}

/// Whether a count is nought, so a line never reopened stays as it was.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
fn is_zero(count: &u32) -> bool {
    *count == 0
}

/// How long a line keeps the id of a question it was asked in, by the
/// moment the id is stamped with: the two days the mesh is read over, so
/// a line asked again for each new cell or file of its owner's keeps
/// every one of them. By count, the younger ones fell out within a day.
pub const ASKED_FOR: i64 = 2 * DAY;

/// How many ids a line keeps at most, one an hour over those two days,
/// so an id with no stamp can't grow it without end.
pub const ASKED_AT_MOST: usize = 48;

/// The ids of the questions the hour asked its lines in, from the ledger
/// as `text`, a line an entry; a line that isn't one names none. Only
/// cadence writes them, so they tell its hourly questions from its others
/// and from a peer's claiming to be its.
#[must_use]
pub fn asked_in(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Entry>(line).ok())
        .flat_map(|e| e.asked)
        .collect()
}

/// The ledger kept at `path`, a line each; a line that isn't an entry is
/// left out rather than failing the run.
#[must_use]
pub fn read(path: &Path) -> Vec<Entry> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Holds the ledger at `path` for one writer until what it returns is
/// dropped: the day, the hour and `cadence owe` each read it, change it
/// and keep it whole, so one waits for another rather than losing its
/// change, as an owner's word of a fix would be.
///
/// # Errors
///
/// When the lock beside the ledger can't be opened or taken.
pub fn lock(path: &Path) -> io::Result<fs::File> {
    let held = fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_extension("jsonl.lock"))?;
    held.lock()?;
    Ok(held)
}

/// Keeps `ledger` at `path` whole, so a reader never sees half of it.
pub fn keep(path: &Path, ledger: &[Entry]) -> io::Result<()> {
    let lines: Vec<String> = ledger
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<_, _>>()?;
    write_whole(path, &(lines.join("\n") + "\n"))
}

/// The rule every finding of cadence's closes by.
const FELL_WHEN: &str =
    "its owner's word of the fix, with `cadence owe --done`, and two days' reads not finding it";

/// The owner of `key`: the address of the longest prefix of it in
/// `owners`, so `crashes:chrome` can have its own owner beside `crashes`.
#[must_use]
pub fn owner_of(key: &str, owners: &BTreeMap<String, String>) -> Option<String> {
    owners
        .iter()
        .filter(|(prefix, _)| key.starts_with(prefix.as_str()))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, address)| address.clone())
}

/// Who holds `key`: its owner line, or, for a unit that failed or
/// restarted with no ready line, whoever holds the unit as the unit
/// watch asks (holders.rs), so one unit has one holder wherever it's
/// found.
fn holder(
    key: &str,
    owners: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
) -> Option<String> {
    owner_of(key, owners).or_else(|| {
        key.strip_prefix("failed:")
            .or_else(|| key.strip_prefix("unstamped:"))
            .and_then(|unit| holders::owner(unit, owners, known))
    })
}

/// Brings `ledger` up to `day`, whose worse findings are `worse`: one
/// found again is kept open with the day's words, a new one is opened
/// with its owner and due a day on, and an open one not found for two
/// days closes if its owner said what fixed it since it was last found,
/// and otherwise is marked back and told again, once, to ask for that
/// word; one found again forgets a fix said before. `known` are the
/// sessions the mesh knows, whose addresses a unit's name may carry.
pub fn update(
    ledger: &mut Vec<Entry>,
    day: &Day,
    worse: &[Finding],
    owners: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
) {
    for finding in worse {
        // A fix of cadence's own that comes back reopens its line, counted,
        // so the loop reads how many fixes didn't hold; a must its owner
        // closed stays closed, and the finding opens a line of its own.
        let newest = ledger.iter_mut().rev().find(|e| e.key == finding.key);
        if let Some(entry) = newest.filter(|e| e.open || e.kind.is_none()) {
            if !entry.open {
                entry.open = true;
                entry.reopened += 1;
                entry.due = day.to + DAY;
                entry.told = None;
                entry.late = false;
                entry.hub = None;
                entry.put_to.clear();
            }
            // A fix said for a figure found worse again didn't hold, so
            // its owner is asked again, and only a word the figure then
            // bears out closes it.
            if entry.fixed.take().is_some() {
                entry.told = None;
            }
            entry.back = None;
            entry.last.clone_from(&day.date);
            entry.last_to = day.to;
            entry.words.clone_from(&finding.words);
        } else {
            let (axis, evidence) = day::about(&finding.key).unwrap_or_default();
            ledger.push(Entry {
                key: finding.key.clone(),
                axis: axis.to_owned(),
                evidence: evidence.to_owned(),
                fell_when: FELL_WHEN.to_owned(),
                owner: holder(&finding.key, owners, known),
                first: day.date.clone(),
                last: day.date.clone(),
                last_to: day.to,
                due: day.to + DAY,
                words: finding.words.clone(),
                open: true,
                told: None,
                late: false,
                kind: None,
                reopened: 0,
                hub: None,
                put_to: Vec::new(),
                back: None,
                fixed: None,
                asked: Vec::new(),
                names: BTreeMap::new(),
                red: false,
                closed: None,
            });
        }
    }
    // The owners are Joel's config as it reads today, so an owner named
    // after a finding opened takes it, with a day of its own to answer,
    // rather than the watcher keeping it for as long as it stays open. A
    // must keeps the quest that filed it, and one put to a hub its holder.
    for entry in ledger
        .iter_mut()
        .filter(|e| e.open && e.kind.is_none() && e.hub.is_none())
    {
        let owner = holder(&entry.key, owners, known);
        if owner.is_some() && owner != entry.owner {
            entry.owner = owner;
            entry.due = day.to + DAY;
            entry.told = None;
            entry.late = false;
        }
    }
    // Two runs a day apart without it, with a little slack for a timer
    // that fired late; a run again the same evening closes nothing. A
    // must its owner filed no day's read finds, so only its owner closes it.
    for entry in ledger.iter_mut().filter(|e| e.open && e.kind.is_none()) {
        if day.to - entry.last_to < 2 * DAY - DAY / 2 {
            continue;
        }
        if entry.fixed.is_some() {
            entry.open = false;
        } else if entry.back.is_none() {
            entry.back = Some(day.date.clone());
            entry.told = None;
        }
    }
}

/// Says the open finding of cadence's own filed as `key` was fixed by
/// `says`, the change or why it's expected: it closes at once if its
/// figure is back, and otherwise once it has been back two days. None when
/// no such finding is open; whether it closed otherwise.
pub fn fixed(ledger: &mut [Entry], key: &str, says: &str) -> Option<bool> {
    let entry = ledger
        .iter_mut()
        .rev()
        .find(|e| e.open && e.kind.is_none() && e.key == key)?;
    entry.fixed = Some(says.to_owned());
    entry.open = entry.back.is_none();
    Some(!entry.open)
}

/// What each owner is told today: one message for each, of its open
/// findings it hasn't been told of, each marked told, so one a typed run
/// opened is told with the next run that tells. A finding
/// with no owner goes to `watcher`, when one is given.
pub fn tell(ledger: &mut [Entry], date: &str, watcher: Option<&str>) -> Vec<(String, String)> {
    let answer = watcher.map_or_else(|| "please say".to_owned(), |w| format!("please tell {w}"));
    let mut by_owner: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in ledger.iter_mut().filter(|e| e.open && e.told.is_none()) {
        let Some(to) = entry.owner.clone().or_else(|| watcher.map(str::to_owned)) else {
            continue;
        };
        entry.told = Some(date.to_owned());
        let back = match &entry.back {
            Some(back) => format!(", its figure back since {back} but no fix said"),
            None => String::new(),
        };
        let words = match &entry.hub {
            Some(subject) => format!(
                "{}: {}, open since {} and past its date{back}, put to you as a holder of \
                 {subject}",
                entry.key, entry.words, entry.first
            ),
            None => format!("{}: {}{back}", entry.key, entry.words),
        };
        by_owner.entry(to).or_default().push(words);
    }
    by_owner
        .into_iter()
        .map(|(to, words)| {
            let found = words
                .iter()
                .map(|w| format!("- {w}."))
                .collect::<Vec<_>>()
                .join("\n");
            let text = format!(
                "cadence day, {date}: the day's read found {} for you, against the days \
                 before.\n{found}\nBy tomorrow's 18:00 read, {answer} the cause and the fix, \
                 or that it's expected and why. Each closes only once you've said what fixed \
                 it, with `cadence owe --done <its key> <the change, or why it's expected>` \
                 on the machine that found it, and its figure is back two days.",
                if words.len() == 1 {
                    "one thing".to_owned()
                } else {
                    format!("{} things", words.len())
                }
            );
            (to, text)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The lines late findings give with no hub standing, as every late
    /// finding was told before hubs.
    pub(crate) fn late(ledger: &mut [Entry], now: i64) -> Vec<String> {
        hubs::put_late(ledger, &[], |_| true, now).1
    }

    pub(crate) fn found(key: &str) -> Finding {
        Finding {
            key: key.to_owned(),
            bar: None,
            before: None,
            now: 1.0,
            words: format!("{key} got worse"),
        }
    }

    /// A day that ended at `to`, for the ledger's tests and owe's.
    pub(crate) fn day(date: &str, to: i64) -> Day {
        Day {
            date: date.to_owned(),
            to,
            ..Day::default()
        }
    }

    /// The ledger brought up to `day`, the mesh knowing the sessions
    /// holders.rs's tests do.
    fn update(
        ledger: &mut Vec<Entry>,
        day: &Day,
        worse: &[Finding],
        owners: &BTreeMap<String, String>,
    ) {
        super::update(ledger, day, worse, owners, &holders::tests::known());
    }

    fn owners() -> BTreeMap<String, String> {
        [("crashes", "A"), ("crashes:chrome", "B"), ("switch", "C")]
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .into()
    }

    #[test]
    fn the_longest_prefix_owns_a_finding() {
        let owners = owners();
        assert_eq!(
            [
                owner_of("crashes:chrome", &owners),
                owner_of("crashes:tec", &owners),
                owner_of("oom.machine", &owners),
            ],
            [Some("B".to_owned()), Some("A".to_owned()), None],
            "a finding went to the wrong owner"
        );
    }

    #[test]
    fn a_unit_named_for_a_session_is_held_by_it_when_no_line_names_it() {
        let mut ledger = Vec::new();
        let mut named = owners();
        named.insert(
            "failed:world-cadence-mt-2af2dh".to_owned(),
            "80ECO2".to_owned(),
        );
        named.insert("unit:google.service".to_owned(), "RTOIMQ".to_owned());
        update(
            &mut ledger,
            &day("d1", DAY),
            &[
                found("failed:garbage-bound-mon-2af2dh.service"),
                found("unstamped:s99ia2-relink-watch.service"),
                found("failed:world-cadence-mt-2af2dh.service"),
                found("failed:camera.service"),
                found("crashes:2af2dh"),
                found("unstamped:google.service"),
            ],
            &named,
        );
        let held: Vec<(&str, Option<&str>)> = ledger
            .iter()
            .map(|e| (e.key.as_str(), e.owner.as_deref()))
            .collect();
        assert_eq!(
            held,
            [
                ("failed:garbage-bound-mon-2af2dh.service", Some("2AF2DH")),
                ("unstamped:s99ia2-relink-watch.service", Some("S99IA2")),
                ("failed:world-cadence-mt-2af2dh.service", Some("80ECO2")),
                ("failed:camera.service", None),
                ("crashes:2af2dh", Some("A")),
                ("unstamped:google.service", Some("RTOIMQ")),
            ],
            "a unit's address didn't hold it, or came before an owner line"
        );
    }

    #[test]
    fn a_unit_found_before_its_session_was_known_is_held_once_it_is() {
        let mut ledger = Vec::new();
        let unit = "failed:garbage-bound-mon-2af2dh.service";
        super::update(
            &mut ledger,
            &day("d1", DAY),
            &[found(unit)],
            &owners(),
            &BTreeSet::new(),
        );
        assert_eq!(
            ledger[0].owner, None,
            "no session known, so nobody holds it"
        );
        update(&mut ledger, &day("d2", 2 * DAY), &[found(unit)], &owners());
        assert_eq!(
            (ledger[0].owner.as_deref(), ledger[0].due),
            (Some("2AF2DH"), 3 * DAY),
            "an open finding didn't go to the session its unit names, with a day of its own"
        );
    }

    #[test]
    fn a_finding_found_again_stays_one_and_waits_two_days_after_its_last() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("crashes:tec")],
            &owners(),
        );
        update(
            &mut ledger,
            &day("d2", 2 * DAY),
            &[found("crashes:tec")],
            &owners(),
        );
        assert_eq!(
            ledger.len(),
            1,
            "a finding found again was opened twice: {ledger:?}"
        );
        assert_eq!(
            (ledger[0].first.as_str(), ledger[0].last.as_str()),
            ("d1", "d2"),
            "a finding found again didn't keep its first day and take the last"
        );
        update(&mut ledger, &day("d3", 3 * DAY), &[], &owners());
        assert!(
            ledger[0].open,
            "a finding closed after one day back: {ledger:?}"
        );
        update(&mut ledger, &day("d3", 3 * DAY + 3600), &[], &owners());
        assert!(
            ledger[0].open,
            "a run again the same evening closed it: {ledger:?}"
        );
        update(&mut ledger, &day("d4", 4 * DAY), &[], &owners());
        assert_eq!(
            (ledger[0].open, ledger[0].back.as_deref()),
            (true, Some("d4")),
            "a finding two days back closed with no fix said: {ledger:?}"
        );
        assert_eq!(
            fixed(&mut ledger, "crashes:tec", "core #6000"),
            Some(true),
            "its owner's fix didn't close a finding already back"
        );
        assert!(!ledger[0].open, "it stayed open: {ledger:?}");
    }

    #[test]
    fn a_figure_back_alone_asks_its_owner_again_for_the_fix() {
        let mut ledger = Vec::new();
        let worse = [found("crashes:tec")];
        update(&mut ledger, &day("d1", DAY), &worse, &owners());
        tell(&mut ledger, "d1", None);
        let mut asked = Vec::new();
        for at in 2..9 {
            let date = format!("d{at}");
            update(&mut ledger, &day(&date, at * DAY), &[], &owners());
            let told = tell(&mut ledger, &date, None);
            asked.extend(told.into_iter().map(|(_, text)| text));
        }
        assert_eq!(
            (ledger[0].open, ledger[0].back.as_deref()),
            (true, Some("d3")),
            "a week of its figure back closed it, or moved when it was back: {ledger:?}"
        );
        assert!(
            asked.len() == 1
                && asked[0].contains("its figure back since d3 but no fix said")
                && asked[0].contains("cadence owe --done <its key>"),
            "its owner wasn't asked again, once in the week, how to close it: {asked:?}"
        );
        assert!(
            late(&mut ledger, 9 * DAY).is_empty() && !ledger[0].late,
            "a finding back, waiting only on its owner's word, went on as late"
        );
        update(&mut ledger, &day("d9", 9 * DAY), &worse, &owners());
        update(&mut ledger, &day("d11", 11 * DAY), &[], &owners());
        assert_eq!(
            ledger[0].back.as_deref(),
            Some("d11"),
            "found again, it kept the day it was first back"
        );
    }

    #[test]
    fn a_fix_said_before_its_figure_is_back_waits_for_it() {
        let mut ledger = Vec::new();
        let worse = [found("crashes:tec")];
        update(&mut ledger, &day("d1", DAY), &worse, &owners());
        tell(&mut ledger, "d1", None);
        fixed(&mut ledger, "crashes:tec", "core #5999");
        update(&mut ledger, &day("d2", 2 * DAY), &worse, &owners());
        let told = tell(&mut ledger, "d2", None).len();
        assert_eq!(
            (ledger[0].fixed.as_deref(), told),
            (None, 1),
            "a fix found worse again was kept, or its owner wasn't asked again"
        );
        assert_eq!(
            fixed(&mut ledger, "crashes:tec", "core #6000"),
            Some(false),
            "a fix said closed a finding whose figure isn't back"
        );
        update(&mut ledger, &day("d3", 3 * DAY), &[], &owners());
        assert!(ledger[0].open, "one day back closed it: {ledger:?}");
        update(&mut ledger, &day("d4", 4 * DAY), &[], &owners());
        assert_eq!(
            (ledger[0].open, ledger[0].fixed.as_deref()),
            (false, Some("core #6000")),
            "two days back after its fix was said didn't close it: {ledger:?}"
        );
        assert_eq!(
            fixed(&mut ledger, "crashes:tec", "core #6001"),
            None,
            "a closed finding took a fix"
        );
    }

    #[test]
    fn a_fix_that_comes_back_reopens_its_line_and_counts_it() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("crashes:tec")],
            &owners(),
        );
        tell(&mut ledger, "d1", None);
        fixed(&mut ledger, "crashes:tec", "core #6000");
        update(&mut ledger, &day("d3", 3 * DAY), &[], &owners());
        assert!(!ledger[0].open, "the finding didn't close: {ledger:?}");
        update(
            &mut ledger,
            &day("d5", 5 * DAY),
            &[found("crashes:tec")],
            &owners(),
        );
        assert_eq!(
            ledger.len(),
            1,
            "a fix that came back opened a second line: {ledger:?}"
        );
        let entry = &ledger[0];
        assert_eq!(
            (
                entry.open,
                entry.reopened,
                entry.first.as_str(),
                entry.last.as_str()
            ),
            (true, 1, "d1", "d5"),
            "a fix that came back wasn't reopened and counted: {entry:?}"
        );
        assert_eq!(
            (
                entry.due,
                entry.told.as_deref(),
                entry.late,
                entry.fixed.as_deref()
            ),
            (6 * DAY, None, false, None),
            "a reopened finding wasn't due a day on, told afresh and unfixed: {entry:?}"
        );
    }

    #[test]
    fn a_closed_must_is_never_reopened_by_a_day() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("hole:ssh")],
            &owners(),
        );
        ledger[0].kind = Some("hole".to_owned());
        ledger[0].open = false;
        update(
            &mut ledger,
            &day("d2", 2 * DAY),
            &[found("hole:ssh")],
            &owners(),
        );
        assert!(
            !ledger[0].open && ledger[0].reopened == 0,
            "a day reopened a must only its owner closes: {ledger:?}"
        );
    }

    #[test]
    fn each_owner_is_told_once_in_one_message() {
        let mut ledger = Vec::new();
        let worse = [
            found("crashes:tec"),
            found("crashes:biome"),
            found("oom.machine"),
        ];
        update(&mut ledger, &day("d1", DAY), &worse, &owners());
        let told = tell(&mut ledger, "d1", Some("W"));
        let to: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            to,
            vec!["A", "W"],
            "an owner wasn't told once, or no owner's wasn't the watcher's"
        );
        assert!(
            told[0].1.contains("2 things")
                && told[0]
                    .1
                    .contains("- crashes:biome: crashes:biome got worse."),
            "an owner's message didn't hold both its findings: {}",
            told[0].1
        );
        assert!(
            tell(&mut ledger, "d1", Some("W")).is_empty(),
            "a run again the same day told an owner twice"
        );
    }

    #[test]
    fn an_owner_named_after_a_finding_opened_takes_it_and_is_told_afresh() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("oom.machine")],
            &owners(),
        );
        tell(&mut ledger, "d1", Some("W"));
        late(&mut ledger, 2 * DAY + 1);
        let mut named = owners();
        named.insert("oom.".to_owned(), "D".to_owned());
        // Not found that day, so only the new owner line can move it.
        update(&mut ledger, &day("d2", 2 * DAY + 3600), &[], &named);
        let entry = &ledger[0];
        assert_eq!(
            (
                entry.owner.as_deref(),
                entry.told.as_deref(),
                entry.late,
                entry.due
            ),
            (Some("D"), None, false, 3 * DAY + 3600),
            "a finding kept the owner it opened with, or its new owner got no day: {entry:?}"
        );
        let told = tell(&mut ledger, "d2", Some("W"));
        assert_eq!(
            told.iter().map(|(to, _)| to.as_str()).collect::<Vec<_>>(),
            vec!["D"],
            "the new owner wasn't told of what it now owns"
        );
        update(&mut ledger, &day("d2", 2 * DAY + 7200), &[], &named);
        assert_eq!(
            ledger[0].told.as_deref(),
            Some("d2"),
            "an owner that didn't change was told again"
        );
    }

    #[test]
    fn a_must_keeps_the_owner_that_filed_it() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("crashes:tec")],
            &owners(),
        );
        ledger[0].kind = Some("hole".to_owned());
        ledger[0].owner = Some("Q".to_owned());
        update(&mut ledger, &day("d2", 2 * DAY), &[], &owners());
        assert_eq!(
            ledger[0].owner.as_deref(),
            Some("Q"),
            "a must was given to the prefix's owner rather than the quest that owes it"
        );
    }

    #[test]
    fn a_finding_past_its_date_is_late_once() {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("switch.minutes.longest")],
            &owners(),
        );
        assert!(
            late(&mut ledger, 2 * DAY).is_empty(),
            "a finding at its date was late"
        );
        let lines = late(&mut ledger, 2 * DAY + 1);
        assert_eq!(
            lines,
            vec!["Late: switch.minutes.longest got worse, open since d1, with C"],
            "a finding past its date wasn't said late with its owner"
        );
        assert!(
            late(&mut ledger, 3 * DAY).is_empty(),
            "a late finding was said twice"
        );
    }
}
