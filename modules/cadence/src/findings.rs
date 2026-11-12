//! The loop's ledger of findings, kept as `findings.jsonl` in cadence's
//! folder, a line each in the shape the loop's other readings keep theirs
//! in, so one reader reads them all: each finding a day names stays open, with its owner and the
//! date its answer is due, until its figure has been back for two days.
//! Each owner is told once a day of what's new for it, in one message,
//! and a finding still open past its date goes to the orchestrator once,
//! so nothing a day found is lost and nobody is asked twice. A must a
//! quest owes, filed with `cadence owe` (owe.rs), is kept here too, and
//! stays open until its owner says it's done.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::day::{self, Day};
use crate::judge::Finding;
use crate::{DAY, write_whole};

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

/// Brings `ledger` up to `day`, whose worse findings are `worse`: one
/// found again is kept open with the day's words, a new one is opened
/// with its owner and due a day on, and an open one not found for two
/// days closes, its figure back.
pub fn update(
    ledger: &mut Vec<Entry>,
    day: &Day,
    worse: &[Finding],
    owners: &BTreeMap<String, String>,
) {
    for finding in worse {
        if let Some(entry) = ledger.iter_mut().find(|e| e.open && e.key == finding.key) {
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
                owner: owner_of(&finding.key, owners),
                first: day.date.clone(),
                last: day.date.clone(),
                last_to: day.to,
                due: day.to + DAY,
                words: finding.words.clone(),
                open: true,
                told: None,
                late: false,
                kind: None,
            });
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
