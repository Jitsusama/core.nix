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
use crate::tracking::{self, Tracking};
use crate::whole::text_lines;

/// How far back the asks are read: a week, as the card reads its merges.
const WEEK: i64 = 7 * DAY;

/// One of his asks, its times in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    at: i64,
    owner: String,
    expected: Option<i64>,
    landed: Option<i64>,
}

/// The record as written, its lines that are text, so one torn byte
/// leaves out its own line rather than all of his asks; or nothing before
/// it's started. The week's readings judge no ask closed, so a line left
/// out costs them only that ask.
#[must_use]
pub fn gather(state_home: &Path) -> String {
    fs::read(state_home.join("mission/asks.jsonl"))
        .map(|bytes| text_lines(&bytes).0)
        .unwrap_or_default()
}

/// Each ask of his own that says when he asked, read from its latest line
/// as the hour's measures read it: the list writes an ask again to move its
/// date or to land, re-mark or put it down, and folds in twins. A line with
/// no time is no ask, and one the list set itself or carries for another is
/// not his.
fn asks(text: &str) -> Vec<Ask> {
    let time = |v: &Value| {
        v.as_str()
            .and_then(|t| t.parse::<Timestamp>().ok())
            .map(Timestamp::as_second)
    };
    let lines = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    let mut latest: BTreeMap<(String, String), Value> = BTreeMap::new();
    for ask in tracking::untwinned(lines) {
        latest.insert((ask["at"].to_string(), ask["words"].to_string()), ask);
    }
    latest
        .into_values()
        .filter(|ask| tracking::of(ask) == Some(Tracking::His))
        .filter_map(|ask| {
            Some(Ask {
                at: time(&ask["at"])?,
                owner: ask["owner"].as_str().unwrap_or("").to_owned(),
                expected: time(&ask["expected"]),
                landed: time(&ask["landed"]),
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
/// when the owner expects it to land (Joel, 13 November).
#[must_use]
pub fn readings(text: &str, now: i64) -> Vec<Reading> {
    let week: Vec<Ask> = asks(text)
        .into_iter()
        .filter(|ask| ask.at > now - WEEK && ask.at <= now)
        .collect();
    let read = !week.is_empty();
    let mut ages: Vec<f64> = week
        .iter()
        .map(|ask| hours(ask.landed.unwrap_or(now) - ask.at))
        .collect();
    ages.sort_by(f64::total_cmp);
    let open: Vec<&Ask> = week.iter().filter(|ask| ask.landed.is_none()).collect();
    let oldest = open.iter().map(|ask| hours(now - ask.at)).reduce(f64::max);
    let mut late: BTreeMap<&str, u32> = BTreeMap::new();
    for ask in &week {
        if ask
            .expected
            .is_some_and(|by| ask.landed.unwrap_or(now) > by)
        {
            *late.entry(ask.owner.as_str()).or_default() += 1;
        }
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
        .detail(read.then(|| {
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
        }))
        .from(from)
        .his(),
        Reading::new(
            "{} of his asks past the date their owner gave",
            read.then(|| f64::from(late.values().sum::<u32>())),
            "",
            "WCJQSG",
        )
        .bar(Bar::AtMost(0.0))
        .detail(read.then(|| {
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
        }))
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
        let [middle, late] = readings(&text, NOW).try_into().unwrap();
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
            let [middle, late] = readings(text, NOW).try_into().unwrap();
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
        assert_eq!(readings(&text, NOW)[0].now, None);
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
        let owners: Vec<String> = asks(&text).into_iter().map(|ask| ask.owner).collect();
        assert_eq!(owners, [""], "his alone, of the three");
    }

    #[test]
    fn an_ask_written_again_or_twinned_counts_once_at_its_latest_line() {
        let at = "2026-11-13T12:00:00-05:00";
        let mut moved: Value =
            serde_json::from_str(&line(at, "A", Some("2026-11-13T13:00:00-05:00"), None))
                .expect("an ask line");
        moved["id"] = "0000000a".into();
        let mut now_dated = moved.clone();
        now_dated["expected"] = "2026-11-13T22:00:00-05:00".into();
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
        let [middle, late] = readings(&text, NOW).try_into().unwrap();
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
            Some(0.0),
            "the date it was moved from was counted late"
        );
    }

    #[test]
    fn a_torn_byte_leaves_out_its_line_and_no_other_of_the_week() {
        let home = tempfile::tempdir().expect("a scratch home");
        fs::create_dir_all(home.path().join("mission")).expect("the mission folder");
        let first = line("2026-11-13T12:00:00-05:00", "A", None, None);
        let last = line("2026-11-13T14:00:00-05:00", "B", None, None);
        let mut bytes = format!("{first}\n").into_bytes();
        bytes.extend_from_slice(b"{\"at\":\"2026-11-13T13:00:00-05:00\",\"owner\":\"\xff\"}\n");
        bytes.extend_from_slice(format!("{last}\n").as_bytes());
        fs::write(home.path().join("mission/asks.jsonl"), bytes).expect("the asks list");
        let owners: Vec<String> = asks(&gather(home.path()))
            .into_iter()
            .map(|ask| ask.owner)
            .collect();
        assert_eq!(
            owners,
            ["A", "B"],
            "a torn byte left out more than its own line, or kept it"
        );
    }
}
