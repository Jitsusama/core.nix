//! The loop's open findings on each axis, from the ledger `cadence day`
//! keeps as findings.jsonl (findings.rs): what a day found worse, with its
//! owner and when its answer is due. Each axis's page lists its own beside
//! its readings, so what's out of spec and what's being done about it sit
//! together. It only reads the ledger; the daily run alone writes it.

use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::Axis;
use crate::findings::Entry;

/// An open finding as its axis keeps it: said, and whether it's late.
#[derive(Debug, Clone, PartialEq)]
pub struct Open {
    pub late: bool,
    pub said: String,
}

/// Gives each of `axes` its open findings from `all`, the soonest due
/// first, each said with `sep` between its parts.
pub fn attach(axes: &mut [Axis], all: &[Entry], now: i64, tz: &TimeZone, sep: &str) {
    for axis in axes {
        axis.findings = on(all, axis.name)
            .iter()
            .map(|entry| Open {
                late: late(entry, now),
                said: said(entry, now, tz, sep),
            })
            .collect();
    }
}

/// The open findings in the ledger in `folder`. A line that isn't an entry
/// is left out, as the daily run leaves it out, and no ledger is none.
pub fn gather(folder: &Path) -> Vec<Entry> {
    fs::read_to_string(folder.join("findings.jsonl"))
        .map(|text| open(&text))
        .unwrap_or_default()
}

fn open(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Entry>(line).ok())
        .filter(|entry| entry.open)
        .collect()
}

/// The open findings on `axis`, the soonest due first.
fn on(all: &[Entry], axis: &str) -> Vec<Entry> {
    let mut on: Vec<Entry> = all.iter().filter(|e| e.axis == axis).cloned().collect();
    on.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.key.cmp(&b.key)));
    on
}

/// Whether its answer is past due at `now`.
fn late(entry: &Entry, now: i64) -> bool {
    entry.due < now
}

/// One finding as its axis's page says it: what the day said, who has it,
/// and when it was due or is.
fn said(entry: &Entry, now: i64, tz: &TimeZone, sep: &str) -> String {
    let owner = entry.owner.as_deref().unwrap_or("no owner yet");
    let when = Timestamp::from_second(entry.due)
        .map(|t| t.to_zoned(tz.clone()).strftime("%a %-d %H:%M").to_string())
        .unwrap_or_default();
    let due = if late(entry, now) {
        format!("late since {when}")
    } else {
        format!("due {when}")
    };
    format!(
        "{}{sep}{owner}{sep}found {}{sep}{due}",
        entry.words, entry.first
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(key: &str, axis: &str, due: i64, open: bool) -> String {
        format!(
            concat!(
                r#"{{"key":"{}","owner":"80ECO2","first":"2026-11-12","last":"2026-11-12","#,
                r#""last_to":0,"due":{},"words":"{} rose","axis":"{}","evidence":"","#,
                r#""fell_when":"","open":{},"told":null,"late":false}}"#
            ),
            key, due, key, axis, open
        )
    }

    #[test]
    fn only_open_entries_are_read_and_a_bad_line_is_left_out() {
        let text = [
            line("crashes:chrome", "quality", 20, true),
            "not an entry".to_owned(),
            line("crashes:nix", "quality", 10, false),
        ]
        .join("\n");
        let keys: Vec<String> = open(&text).into_iter().map(|e| e.key).collect();
        assert_eq!(keys, ["crashes:chrome"]);
    }

    #[test]
    fn an_axis_has_its_own_findings_the_soonest_due_first() {
        let text = [
            line("crashes:chrome", "quality", 20, true),
            line("units", "quality", 10, true),
            line("merge-to-on", "time to done", 5, true),
        ]
        .join("\n");
        let all = open(&text);
        let keys: Vec<String> = on(&all, "quality").into_iter().map(|e| e.key).collect();
        assert_eq!(keys, ["units", "crashes:chrome"]);
        assert!(on(&all, "stress").is_empty(), "stress has no findings");
    }

    #[test]
    fn a_finding_says_who_has_it_and_whether_it_is_late() {
        let mut entry = open(&line("units", "quality", 1_794_506_400, true)).remove(0);
        let tz = TimeZone::UTC;
        assert_eq!(
            said(&entry, 1_794_506_000, &tz, " · "),
            "units rose · 80ECO2 · found 2026-11-12 · due Thu 12 18:00"
        );
        entry.owner = None;
        assert_eq!(
            said(&entry, 1_794_510_000, &tz, " · "),
            "units rose · no owner yet · found 2026-11-12 · late since Thu 12 18:00"
        );
    }

    #[test]
    fn no_ledger_is_no_findings() {
        assert!(
            gather(Path::new("/nonexistent")).is_empty(),
            "no ledger, no findings"
        );
    }
}
