//! Time to done from Joel's side, from the record of his asks the
//! orchestrator keeps in `mission/asks.jsonl` (WCJQSG, from 13 November):
//! each ask with the time of his words, its owner, the date the owner
//! gave, and when it reached him, the switch that put it on his laptop.
//! Both ends are his, his words and what he can try, so these read his
//! side where merge to on starts at a merge he never sees.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use serde_json::Value;

use super::{Bar, Reading, shown};
use crate::DAY;
use crate::hour::ledger::records;
use crate::hour::measures::parked;
use crate::hour::measures::promised::{Promise, promised};
use crate::tracking::{self, Tracking};

/// How far back the asks are read: a week, as the card reads its merges.
const WEEK: i64 = 7 * DAY;

/// One of his asks, its times in seconds, and whether any date its owner
/// gave it passed unmet by `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    at: i64,
    owner: String,
    expected: Option<i64>,
    landed: Option<i64>,
    missed: bool,
}

/// The record as written, beside why it's unread when it is; or nothing
/// before it's started. Each ask is read from its latest line, so a line
/// left out could stand an earlier one in its place, landed as not, or a
/// date as never missed: a line that isn't text or isn't a record leaves
/// the list unread, naming the first, through the hour's own reading of
/// its lines, and the journal says so. The hour also leaves a list unread
/// while it holds fewer asks than it has; the axes keep no count of what
/// they've seen, so a list written over reads here as it stands.
#[must_use]
pub fn gather(state_home: &Path) -> (String, Option<String>) {
    let path = state_home.join("mission/asks.jsonl");
    let read = match fs::read(&path) {
        Ok(bytes) => records(&bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return (String::new(), None),
        Err(err) => Err(format!("it can't be read: {err}")),
    };
    read.map_or_else(
        |why| {
            eprintln!(
                "cadence: the asks list at {} is unread, as {why}, so the axes read none of his \
                 asks",
                path.display()
            );
            (String::new(), Some(why))
        },
        |text| (text, None),
    )
}

/// Each ask of his own that says when he asked, read from its latest line
/// as the hour's measures read it: the list writes an ask again to move its
/// date or to land, re-mark or put it down, and folds in twins. A line with
/// no time is no ask, and one the list set itself or carries for another is
/// not his. It missed a date, as the slips measure reads its dates, when
/// one passed before it landed or was met, unless it was moved before it
/// passed, so a date missed and then moved is still missed. Its dates are
/// every one any of its lines gave, each as the latest line giving it
/// says, since a date moved by writing `expected` again with no
/// `promised` is on no later line; such a move says no time, so the date
/// it left counts as missed once it passed. An ask parked on a ruling
/// misses none, as its owner owes no date while the ruling stands.
fn asks(text: &str, now: i64) -> Vec<Ask> {
    let time = |v: &Value| {
        v.as_str()
            .and_then(|t| t.parse::<Timestamp>().ok())
            .map(Timestamp::as_second)
    };
    let lines = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    let mut latest: BTreeMap<[String; 2], (Value, BTreeMap<i64, Promise>)> = BTreeMap::new();
    for ask in tracking::untwinned(lines) {
        let (line, dates) = latest.entry(tracking::asked(&ask)).or_default();
        dates.extend(promised(&ask).into_iter().map(|p| (p.expected, p)));
        *line = ask;
    }
    latest
        .into_values()
        .filter(|(ask, _)| tracking::of(ask) == Some(Tracking::His))
        .filter_map(|(ask, dates)| {
            let landed = time(&ask["landed"]);
            // Late at its date while open, as slips reads it, and landed
            // on its date is kept.
            let passed = |expected: i64| landed.map_or(expected <= now, |l| expected < l);
            let missed = parked::ruling(&ask).is_none()
                && dates.values().any(|p| {
                    passed(p.expected)
                        && p.met.is_none_or(|met| met > p.expected)
                        && p.moved.is_none_or(|moved| moved >= p.expected)
                });
            Some(Ask {
                at: time(&ask["at"])?,
                owner: ask["owner"].as_str().unwrap_or("").to_owned(),
                expected: time(&ask["expected"]),
                landed,
                missed,
            })
        })
        .collect()
}

/// The week's two: how long his asks take to reach him, at the middle,
/// and how many are past the date their owner gave. An ask still open
/// counts at its age so far, which it has taken at least, so asks left
/// waiting raise the middle rather than drop out of it. Half within four
/// hours is the bar, from "the smallest slice he can try within hours"
/// (Joel, 11 November); none past its date is the other, since a date is
/// when the owner expects it to land (Joel, 13 November). A list left
/// `unread` gives neither, saying why.
#[must_use]
pub fn readings(text: &str, unread: Option<&str>, now: i64) -> Vec<Reading> {
    let week: Vec<Ask> = asks(text, now)
        .into_iter()
        .filter(|ask| unread.is_none() && ask.at > now - WEEK && ask.at <= now)
        .collect();
    let read = !week.is_empty();
    let unsaid = || unread.map(|why| format!("the list of his asks is unread, as {why}"));
    let mut ages: Vec<f64> = week
        .iter()
        .map(|ask| hours(ask.landed.unwrap_or(now) - ask.at))
        .collect();
    ages.sort_by(f64::total_cmp);
    let open: Vec<&Ask> = week.iter().filter(|ask| ask.landed.is_none()).collect();
    let oldest = open.iter().map(|ask| hours(now - ask.at)).reduce(f64::max);
    let mut late: BTreeMap<&str, u32> = BTreeMap::new();
    for ask in week.iter().filter(|ask| ask.missed) {
        *late.entry(ask.owner.as_str()).or_default() += 1;
    }
    let undated = week.iter().filter(|ask| ask.expected.is_none()).count();
    let from = "the orchestrator's record of his asks";
    vec![
        Reading::new(
            "his asks reaching him in {} at the middle, from his words",
            middle(&ages),
            " h",
            "WCJQSG",
        )
        .bar(Bar::AtMost(4.0))
        .detail(
            read.then(|| {
                let reached = week.len() - open.len();
                let mut said = format!("{reached} of {} this week reached him", week.len());
                if let Some(oldest) = oldest {
                    let _ = write!(
                        said,
                        "; {} still open, the oldest {}, each counted at its age so far",
                        open.len(),
                        shown(oldest, " h")
                    );
                }
                said
            })
            .or_else(unsaid),
        )
        .from(from)
        .his(),
        Reading::new(
            "{} of his asks past the date their owner gave",
            read.then(|| f64::from(late.values().sum::<u32>())),
            "",
            "WCJQSG",
        )
        .bar(Bar::AtMost(0.0))
        .detail(
            read.then(|| {
                let owners: Vec<String> = late.iter().map(|(o, n)| format!("{o} {n}")).collect();
                let mut said = if owners.is_empty() {
                    "none late this week".to_owned()
                } else {
                    format!("late: {}", owners.join(", "))
                };
                if undated > 0 {
                    let _ = write!(said, "; {undated} with no date given yet");
                }
                said
            })
            .or_else(unsaid),
        )
        .from(from)
        .his(),
    ]
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a week's seconds are far inside f64's exact integers"
)]
fn hours(seconds: i64) -> f64 {
    seconds.max(0) as f64 / 3600.0
}

/// The middle of sorted figures, the mean of the two middle ones when
/// there's an even count.
fn middle(sorted: &[f64]) -> Option<f64> {
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some(f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fri 13 November 2026 at 20:00, Toronto.
    const NOW: i64 = 1_794_618_000;

    fn line(at: &str, owner: &str, expected: Option<&str>, landed: Option<&str>) -> String {
        serde_json::json!({
            "at": at, "words": "", "owner": owner,
            "expected": expected, "landed": landed, "pr": null
        })
        .to_string()
    }

    #[test]
    fn an_ask_reached_counts_to_its_switch_and_an_open_one_to_now() {
        let text = [
            // Reached him two hours after his words.
            line(
                "2026-11-13T12:00:00-05:00",
                "A",
                Some("2026-11-13T15:00:00-05:00"),
                Some("2026-11-13T14:00:00-05:00"),
            ),
            // Open six hours, its date passed.
            line(
                "2026-11-13T14:00:00-05:00",
                "B",
                Some("2026-11-13T18:00:00-05:00"),
                None,
            ),
            // Open one hour, no date yet.
            line("2026-11-13T19:00:00-05:00", "C", None, None),
        ]
        .join("\n");
        let [middle, late] = readings(&text, None, NOW).try_into().unwrap();
        assert_eq!(middle.now, Some(2.0));
        assert!(middle.his, "his asks' time was read from our side");
        assert_eq!(
            middle.detail.as_deref(),
            Some(
                "1 of 3 this week reached him; 2 still open, the oldest 6 h, each \
                 counted at its age so far"
            )
        );
        assert_eq!(late.now, Some(1.0));
        assert!(late.out(), "an ask past its date was in spec");
        assert_eq!(
            late.detail.as_deref(),
            Some("late: B 1; 1 with no date given yet")
        );
    }

    #[test]
    fn no_record_reads_unread_and_a_line_with_no_time_is_no_ask() {
        for text in ["", "{\"owner\":\"A\"}\nnot json"] {
            let [middle, late] = readings(text, None, NOW).try_into().unwrap();
            assert_eq!((middle.now, late.now), (None, None));
            assert_eq!((middle.detail, late.detail), (None, None));
        }
    }

    #[test]
    fn an_ask_from_before_the_week_is_left_out() {
        let text = line(
            "2026-11-01T12:00:00-05:00",
            "A",
            None,
            Some("2026-11-02T12:00:00-05:00"),
        );
        assert_eq!(readings(&text, None, NOW)[0].now, None);
    }

    #[test]
    fn only_his_own_asks_are_read_for_the_week() {
        let kind = |tracking: &str| {
            let mut ask: Value =
                serde_json::from_str(&line("2026-11-13T12:00:00-05:00", tracking, None, None))
                    .unwrap();
            ask["tracking"] = tracking.into();
            ask.to_string()
        };
        let text = [kind(""), kind("derived"), kind("owner-tracked")]
            .iter()
            .enumerate()
            .map(|(n, line)| line.replace("\"words\":\"\"", &format!("\"words\":\"ask {n}\"")))
            .collect::<Vec<_>>()
            .join("\n");
        let owners: Vec<String> = asks(&text, NOW).into_iter().map(|ask| ask.owner).collect();
        assert_eq!(owners, [""], "his alone, of the three");
    }

    #[test]
    fn an_ask_written_again_or_twinned_counts_once_at_its_latest_line() {
        let at = "2026-11-13T12:00:00-05:00";
        let mut moved: Value =
            serde_json::from_str(&line(at, "A", Some("2026-11-13T13:00:00-05:00"), None))
                .expect("an ask line");
        moved["id"] = "0000000a".into();
        moved["promised"] = serde_json::json!([
            {"expected": "2026-11-13T13:00:00-05:00", "given": "2026-11-13T12:05:00-05:00"}
        ]);
        // Its 13:00 passed, and only then was it moved to 22:00.
        let mut now_dated = moved.clone();
        now_dated["expected"] = "2026-11-13T22:00:00-05:00".into();
        now_dated["promised"] = serde_json::json!([
            {"expected": "2026-11-13T13:00:00-05:00", "given": "2026-11-13T12:05:00-05:00"},
            {"expected": "2026-11-13T22:00:00-05:00", "given": "2026-11-13T13:30:00-05:00"}
        ]);
        let mut landed = now_dated.clone();
        landed["landed"] = "2026-11-13T14:00:00-05:00".into();
        let mut twin = landed.clone();
        twin["id"] = "0000000a~1".into();
        twin["landed"] = "2026-11-13T19:00:00-05:00".into();
        let text = [moved, now_dated, landed, twin]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let [middle, late] = readings(&text, None, NOW).try_into().unwrap();
        assert_eq!(
            middle.detail.as_deref(),
            Some("1 of 1 this week reached him"),
            "an ask written again or twinned was counted more than once"
        );
        assert_eq!(
            middle.now,
            Some(2.0),
            "the twin's reaching stood for the ask's"
        );
        assert_eq!(
            late.now,
            Some(1.0),
            "a date missed and only then moved wasn't counted late"
        );
    }

    #[test]
    fn a_date_moved_before_it_passed_or_met_by_it_is_not_late() {
        let ask = |promised: Value, landed: Option<&str>| {
            let mut ask: Value = serde_json::from_str(&line(
                "2026-11-13T12:00:00-05:00",
                "A",
                Some("2026-11-13T22:00:00-05:00"),
                landed,
            ))
            .expect("an ask line");
            ask["promised"] = promised;
            ask.to_string()
        };
        let moved_early = ask(
            serde_json::json!([
                {"expected": "2026-11-13T13:00:00-05:00", "given": "2026-11-13T12:05:00-05:00"},
                {"expected": "2026-11-13T22:00:00-05:00", "given": "2026-11-13T12:50:00-05:00"}
            ]),
            Some("2026-11-13T14:00:00-05:00"),
        );
        let met = ask(
            serde_json::json!([
                {"expected": "2026-11-13T13:00:00-05:00", "given": "2026-11-13T12:05:00-05:00",
                 "met": "2026-11-13T12:55:00-05:00"},
                {"expected": "2026-11-13T22:00:00-05:00", "given": "2026-11-13T13:30:00-05:00"}
            ]),
            Some("2026-11-13T14:00:00-05:00"),
        );
        for (text, why) in [
            (moved_early, "moved before it passed"),
            (met, "met by its date"),
        ] {
            assert_eq!(
                readings(&text, None, NOW)[1].now,
                Some(0.0),
                "a date {why} was counted late"
            );
        }
    }

    #[test]
    fn a_date_moved_by_writing_its_expected_again_is_still_missed() {
        let at = "2026-11-13T12:00:00-05:00";
        let text = [
            line(at, "A", Some("2026-11-13T13:00:00-05:00"), None),
            line(at, "A", Some("2026-11-13T22:00:00-05:00"), None),
        ]
        .join("\n");
        assert_eq!(
            readings(&text, None, NOW)[1].now,
            Some(1.0),
            "a 13:00 that passed, then rewritten as 22:00 with no promised, read as never missed"
        );
    }

    #[test]
    fn an_ask_parked_on_a_ruling_misses_no_date() {
        let mut ask: Value = serde_json::from_str(&line(
            "2026-11-13T12:00:00-05:00",
            "A",
            Some("2026-11-13T13:00:00-05:00"),
            None,
        ))
        .expect("an ask line");
        ask["parked"] = "until load is spread".into();
        assert_eq!(
            readings(&ask.to_string(), None, NOW)[1].now,
            Some(0.0),
            "an ask parked on a ruling read as late for a date nobody owes"
        );
    }

    #[test]
    fn an_ask_put_down_or_re_marked_at_its_latest_line_drops_out() {
        let at = "2026-11-13T12:00:00-05:00";
        let open = line(at, "A", Some("2026-11-13T13:00:00-05:00"), None);
        let mut down: Value = serde_json::from_str(&open).expect("an ask line");
        down["closed"] = "2026-11-13T15:00:00-05:00".into();
        let mut derived: Value = serde_json::from_str(&open).expect("an ask line");
        derived["tracking"] = "derived".into();
        for (later, why) in [(down, "put down"), (derived, "re-marked derived")] {
            let text = [open.clone(), later.to_string()].join("\n");
            let [middle, late] = readings(&text, None, NOW).try_into().unwrap();
            assert_eq!(
                (middle.now, late.now),
                (None, None),
                "an ask {why} at its latest line was read from its earlier open line"
            );
        }
    }

    #[test]
    fn an_open_ask_due_now_is_late_and_one_landed_on_its_date_is_not() {
        let due = "2026-11-13T20:00:00-05:00";
        let open = line("2026-11-13T12:00:00-05:00", "A", Some(due), None);
        let landed = line("2026-11-13T13:00:00-05:00", "B", Some(due), Some(due));
        assert_eq!(
            readings(&[open, landed].join("\n"), None, NOW)[1]
                .detail
                .as_deref(),
            Some("late: A 1"),
            "an open ask due this second wasn't late, or one landed on its date was"
        );
    }

    #[test]
    fn a_list_said_unread_gives_no_figures_even_beside_its_text() {
        let text = line("2026-11-13T12:00:00-05:00", "A", None, None);
        let [middle, late] = readings(&text, Some("its line 2 isn't text"), NOW)
            .try_into()
            .unwrap();
        assert_eq!(
            (middle.now, late.now),
            (None, None),
            "a list said unread was read for figures"
        );
    }

    #[test]
    fn a_list_that_cant_be_read_is_unread_saying_why() {
        let home = tempfile::tempdir().expect("a scratch home");
        fs::create_dir_all(home.path().join("mission/asks.jsonl")).expect("a folder in its place");
        let (text, unread) = gather(home.path());
        assert!(
            text.is_empty() && unread.is_some_and(|why| why.starts_with("it can't be read: ")),
            "a list that can't be read wasn't said unread with why"
        );
    }

    #[test]
    fn a_torn_line_leaves_the_list_unread_naming_it() {
        let home = tempfile::tempdir().expect("a scratch home");
        fs::create_dir_all(home.path().join("mission")).expect("the mission folder");
        let first = line("2026-11-13T12:00:00-05:00", "A", None, None);
        let last = line("2026-11-13T14:00:00-05:00", "B", None, None);
        let mut bytes = format!("{first}\n").into_bytes();
        bytes.extend_from_slice(b"{\"at\":\"2026-11-13T13:00:00-05:00\",\"owner\":\"\xff\"}\n");
        bytes.extend_from_slice(format!("{last}\n").as_bytes());
        fs::write(home.path().join("mission/asks.jsonl"), bytes).expect("the asks list");
        let (text, unread) = gather(home.path());
        let [middle, late] = readings(&text, unread.as_deref(), NOW).try_into().unwrap();
        let said = Some("the list of his asks is unread, as its line 2 isn't text");
        assert_eq!(
            (
                middle.now,
                late.now,
                middle.detail.as_deref(),
                late.detail.as_deref()
            ),
            (None, None, said, said),
            "a torn byte was read past, which could stand an earlier line in its place"
        );
        let mut record = format!("{first}\n").into_bytes();
        record.extend_from_slice(b"[1]\n");
        fs::write(home.path().join("mission/asks.jsonl"), record).expect("the asks list");
        assert_eq!(
            gather(home.path()).1.as_deref(),
            Some("its line 2 isn't a record"),
            "a line that isn't a record was read past"
        );
        assert_eq!(
            gather(&home.path().join("nowhere")),
            (String::new(), None),
            "a list not yet started read as unread"
        );
    }
}
