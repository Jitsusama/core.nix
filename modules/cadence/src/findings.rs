//! The loop's ledger of findings, kept as `findings.jsonl` in cadence's
//! folder, a line each in the shape the loop's other readings keep theirs
//! in, so one reader reads them all: each finding a day names stays open, with its owner and the
//! date its answer is due, until its figure has been back for two days.
//! Each owner is told once a day of what's new for it, in one message,
//! and a finding still open past its date goes to the orchestrator once,
//! so nothing a day found is lost and nobody is asked twice. A must a
//! quest owes, filed with `cadence owe` (owe.rs), is kept here too, and
//! stays open until its owner says it's done.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::day::{self, Day};
use crate::judge::Finding;
use crate::mesh::Presence;
use crate::{DAY, holders, write_whole};

/// One finding as the ledger keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The finding's key: a measure's, or `crashes:biome`.
    pub key: String,
    /// The mesh address it was given to, or none when no owner matched.
    pub owner: Option<String>,
    /// The date it first appeared, and the last one it was found on.
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
}

/// Whether a count is nought, so a line never reopened stays as it was.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
fn is_zero(count: &u32) -> bool {
    *count == 0
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

/// Keeps `ledger` at `path` whole, so a reader never sees half of it.
pub fn keep(path: &Path, ledger: &[Entry]) -> io::Result<()> {
    let lines: Vec<String> = ledger
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<_, _>>()?;
    write_whole(path, &(lines.join("\n") + "\n"))
}

/// The rule every finding of cadence's closes by.
const FELL_WHEN: &str = "not found by two days' reads running";

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
/// days closes, its figure back. `known` are the sessions the mesh knows,
/// whose addresses a unit's name may carry.
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
            }
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
            });
        }
    }
    // The owners are Joel's config as it reads today, so an owner named
    // after a finding opened takes it, with a day of its own to answer,
    // rather than the watcher keeping it for as long as it stays open. A
    // must keeps the quest that filed it.
    for entry in ledger.iter_mut().filter(|e| e.open && e.kind.is_none()) {
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
        if day.to - entry.last_to >= 2 * DAY - DAY / 2 {
            entry.open = false;
        }
    }
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
        by_owner.entry(to).or_default().push(entry.words.clone());
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
                 or that it's expected and why. It closes by itself once its figure is back \
                 two days.",
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

/// The open findings past their date at `now` that the orchestrator
/// hasn't heard of, each marked heard, as the note's lines.
pub fn late(ledger: &mut [Entry], now: i64) -> Vec<String> {
    ledger
        .iter_mut()
        .filter(|e| e.open && !e.late && e.due < now)
        .map(|entry| {
            entry.late = true;
            format!(
                "Late: {}, open since {}, with {}",
                entry.words,
                entry.first,
                entry.owner.as_deref().unwrap_or("no owner")
            )
        })
        .collect()
}

/// How long an owner's session may say nothing, parked or not running,
/// before its findings go to the watcher: half a day, so one a crash or a
/// reboot took down is first given back by its holder, and one the day's
/// run finds still reaches a session that can answer before it falls due.
pub const QUIET_AT_MOST: i64 = DAY / 2;

/// The owners with no session to answer them, of those `owners` names and
/// those holding an open finding of the ledger's, found by a unit's name:
/// by each address's newest presence, none at all, or one quiet past
/// `QUIET_AT_MOST` that is parked or isn't `running`. A parked session is
/// told its findings as findings, which start no turn, so one parked all
/// day reads none of them: 7QEY1S's, parked from 00:31 on 18 November,
/// went late.
pub fn gone(
    owners: &BTreeMap<String, String>,
    ledger: &[Entry],
    presences: &[Presence],
    running: impl Fn(&Presence) -> bool,
    now: i64,
) -> BTreeSet<String> {
    let said_at = |p: &Presence| {
        p.at.as_str()
            .and_then(|t| t.parse::<jiff::Timestamp>().ok())
            .map(jiff::Timestamp::as_second)
    };
    let holding = ledger
        .iter()
        .filter(|e| e.open && e.kind.is_none())
        .filter_map(|e| e.owner.as_ref());
    owners
        .values()
        .chain(holding)
        .filter(|address| {
            // One with no readable time ranks below every dated one.
            let newest = presences
                .iter()
                .filter(|p| p.addresses.iter().any(|a| a.eq_ignore_ascii_case(address)))
                .max_by_key(|p| said_at(p));
            newest.is_none_or(|p| {
                let quiet = said_at(p).is_none_or(|at| now - at > QUIET_AT_MOST);
                quiet && (p.state == "parked" || !running(p))
            })
        })
        .cloned()
        .collect()
}

/// Brings `ledger` up to `day` as `update` does, as though the owners in
/// `gone` weren't named: their open findings go back to nobody, keeping
/// their dates, so the watcher is told them once and one already late
/// isn't given a fresh day, and stay with it until a flag names a session
/// that answers. A must keeps the quest that filed it.
pub fn update_without(
    ledger: &mut Vec<Entry>,
    day: &Day,
    worse: &[Finding],
    owners: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
    gone: &BTreeSet<String>,
) {
    let is_gone = |address: &str| gone.iter().any(|g| g.eq_ignore_ascii_case(address));
    let there: BTreeMap<String, String> = owners
        .iter()
        .filter(|(_, address)| !is_gone(address))
        .map(|(key, address)| (key.clone(), address.clone()))
        .collect();
    let known: BTreeSet<String> = known.iter().filter(|k| !is_gone(k)).cloned().collect();
    update(ledger, day, worse, &there, &known);
    for entry in ledger.iter_mut().filter(|e| e.open && e.kind.is_none()) {
        if entry.owner.as_deref().is_some_and(is_gone) {
            entry.owner = None;
            entry.told = None;
        }
    }
}

/// What the orchestrator is told of the owners in `gone`, in one line
/// whatever their number: the flags that name each, so they can be
/// changed, and who has their findings meanwhile.
pub fn flags_gone(
    owners: &BTreeMap<String, String>,
    gone: &BTreeSet<String>,
    watcher: Option<&str>,
) -> Option<String> {
    if gone.is_empty() {
        return None;
    }
    let each: Vec<String> = gone
        .iter()
        .map(|address| {
            let keys: Vec<&str> = owners
                .iter()
                .filter(|(_, a)| a.eq_ignore_ascii_case(address))
                .map(|(key, _)| key.as_str())
                .collect();
            if keys.is_empty() {
                format!("{address} (by a unit's name)")
            } else {
                format!("{address} ({})", keys.join(", "))
            }
        })
        .collect();
    let told = watcher.map_or_else(
        || "nobody is told their findings".to_owned(),
        |w| format!("{w} has their findings"),
    );
    Some(format!(
        "Owner flags name no live session for {}; {told} until the flags change",
        each.join(", ")
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn found(key: &str) -> Finding {
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
    fn a_finding_found_again_stays_one_and_closes_two_days_after_its_last() {
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
        assert!(
            !ledger[0].open,
            "a finding two days back stayed open: {ledger:?}"
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
            (entry.due, entry.told.as_deref(), entry.late),
            (6 * DAY, None, false),
            "a reopened finding wasn't due a day on and told afresh: {entry:?}"
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
            told[0].1.contains("2 things") && told[0].1.contains("- crashes:biome got worse."),
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

    /// A session's presence as the mesh keeps it, last said at `at`.
    fn presence(address: &str, state: &str, at: i64, pid: u32) -> Presence {
        let at = jiff::Timestamp::from_second(at).expect("a time in range");
        serde_json::from_value(serde_json::json!({
            "addresses": [address],
            "state": state,
            "at": at.to_string(),
            "owner": {"pid": pid},
        }))
        .expect("a presence is read from its record")
    }

    fn named() -> BTreeMap<String, String> {
        [
            ("checks", "DEAD01"),
            ("code", "CRASHD"),
            ("crashes", "GONE01"),
            ("crashes:chrome", "LIVE01"),
            ("health", "RESTED"),
            ("switch", "PARKED"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .into()
    }

    #[test]
    fn an_owner_with_no_session_or_quiet_past_half_a_day_is_gone() {
        let now = 10 * DAY;
        let quiet = now - QUIET_AT_MOST - 1;
        let undated: Presence = serde_json::from_value(serde_json::json!({
            "addresses": ["LIVE01"],
            "state": "parked",
        }))
        .expect("a presence is read from its record");
        let presences = [
            // The newest of an address's presences counts, whatever the
            // order they're read in, and one with no time never does.
            presence("PARKED", "parked", quiet, 3),
            presence("PARKED", "busy", now - DAY, 1),
            undated,
            presence("LIVE01", "busy", now - DAY, 1),
            presence("DEAD01", "busy", quiet, 2),
            // Down an hour, as a crash or a reboot leaves it: its holder
            // gives it back first.
            presence("CRASHD", "busy", now - 3600, 2),
            presence("RESTED", "parked", now - QUIET_AT_MOST, 3),
        ];
        let running = |p: &Presence| p.owner.as_ref().and_then(|o| o.pid) == Some(1);
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("failed:camera.service")],
            &named(),
        );
        ledger[0].owner = Some("UNIT01".to_owned());
        let gone = gone(&named(), &ledger, &presences, running, now);
        assert_eq!(
            gone.iter().map(String::as_str).collect::<Vec<_>>(),
            ["DEAD01", "GONE01", "PARKED", "UNIT01"],
            "an owner that can't answer was kept, or one that can was taken"
        );
    }

    #[test]
    fn a_gone_owners_findings_go_to_the_watcher_until_it_answers_again() {
        let mut ledger = Vec::new();
        let unit = "failed:garbage-bound-mon-2af2dh.service";
        let worse = [found("crashes:tec"), found("crashes:chrome"), found(unit)];
        let known = holders::tests::known();
        // An address found from a unit's name is matched whatever its case.
        let gone: BTreeSet<String> = ["GONE01".to_owned(), "2af2dh".to_owned()].into();
        let none = BTreeSet::new();
        let on = |ledger: &mut Vec<Entry>, date: &str, at: i64, gone: &BTreeSet<String>| {
            update_without(ledger, &day(date, at * DAY), &worse, &named(), &known, gone);
            let told = tell(ledger, date, Some("WATCH1"));
            told.into_iter().map(|(to, _)| to).collect::<Vec<String>>()
        };
        on(&mut ledger, "d1", 1, &none);
        ledger.push(Entry {
            key: "security:s2".to_owned(),
            kind: Some("hole".to_owned()),
            ..ledger[0].clone()
        });
        ledger[0].late = true;
        assert_eq!(
            on(&mut ledger, "d2", 2, &gone),
            ["WATCH1"],
            "the watcher wasn't told"
        );
        let owners: Vec<Option<&str>> = ledger.iter().map(|e| e.owner.as_deref()).collect();
        assert_eq!(
            (owners, ledger[0].due, ledger[0].late),
            (
                vec![None, Some("LIVE01"), None, Some("GONE01")],
                2 * DAY,
                true
            ),
            "a gone owner kept its finding, a late one got a fresh day, or a must moved"
        );
        assert!(
            on(&mut ledger, "d3", 3, &gone).is_empty(),
            "the watcher was told twice"
        );
        assert_eq!(
            on(&mut ledger, "d4", 4, &none),
            ["2AF2DH", "GONE01"],
            "an owner back on the mesh didn't take its findings again"
        );
    }

    #[test]
    fn the_orchestrator_is_told_in_a_line_which_flags_name_a_gone_owner() {
        let gone: BTreeSet<String> = [
            "GONE01".to_owned(),
            "PARKED".to_owned(),
            "UNIT01".to_owned(),
        ]
        .into();
        assert_eq!(
            [
                flags_gone(&named(), &gone, Some("WATCH1")),
                flags_gone(&named(), &["GONE01".to_owned()].into(), None),
                flags_gone(&named(), &BTreeSet::new(), Some("WATCH1")),
            ],
            [
                Some(
                    "Owner flags name no live session for GONE01 (crashes), PARKED (switch), \
                     UNIT01 (by a unit's name); WATCH1 has their findings until the flags change"
                        .to_owned()
                ),
                Some(
                    "Owner flags name no live session for GONE01 (crashes); nobody is told \
                     their findings until the flags change"
                        .to_owned()
                ),
                None,
            ],
            "the orchestrator wasn't told which flags to change, or who has their findings"
        );
    }
}
