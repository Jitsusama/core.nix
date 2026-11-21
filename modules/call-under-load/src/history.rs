//! What Joel's Chrome and Slack drawing threads waited for a CPU over the
//! last hour, read from lag-watch's history rather than measured again:
//! lag-watch reads them every ten seconds, all day, and writes each
//! window's worst wait by app.

use crate::figures::percentile;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// How far back the hour reads, in seconds.
const HOUR: u64 = 3600;

/// One app's waits over the hour.
#[derive(Debug, Default, PartialEq)]
pub struct Waits {
    /// The p99 of each window's worst wait, in ms.
    pub worst_p99: Option<f64>,
    /// How many windows had the app's threads to read.
    pub windows: usize,
    /// How many windows had a tab of it hidden, so left out.
    pub lowered: usize,
    /// How many windows had a process of it nobody watched.
    pub unwatched: usize,
}

/// The waits of the app lag-watch labels `label` over the hour before the
/// test call `began`, from the history in `folder`, leaving out the
/// test's own minutes at either end: windows ending after it began, and
/// those up to `longest` after the last hour's began, an hour before, when
/// that one may still have been running. Those windows hold a test's own
/// two browsers, and a test's minutes are more than the hour's top 1%, so
/// its p99 could be a test's load alone. Its files are named by the UTC
/// day, so the newest two hold the last hour whenever it falls.
pub fn waits(folder: &Path, label: &str, began: u64, longest: u64) -> Waits {
    let mut days: Vec<_> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    days.sort();
    let text: String = days
        .iter()
        .rev()
        .take(2)
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n");
    of(&text, label, began, longest)
}

/// The same, from the history's lines.
pub fn of(lines: &str, label: &str, began: u64, longest: u64) -> Waits {
    let after = began.saturating_sub(HOUR) + longest;
    let mut worst = Vec::new();
    let mut found = Waits::default();
    for window in lines
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        let ended = window["ended"].as_u64().unwrap_or(0);
        if ended <= after || ended > began {
            continue;
        }
        if let Some(us) = window["waits"][label]["worst_us"].as_f64() {
            worst.push(us / 1000.0);
        }
        found.lowered += usize::from(window["lowered"][label].as_u64().unwrap_or(0) > 0);
        found.unwatched += usize::from(window["unwatched"][label].as_u64().unwrap_or(0) > 0);
    }
    found.windows = worst.len();
    found.worst_p99 = percentile(&worst, 99);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn window(ended: u64, chrome_us: Option<f64>) -> String {
        let mut w = json!({"ended": ended, "waits": {"niri": {"mean_us": 1.0, "worst_us": 90.0}}});
        if let Some(us) = chrome_us {
            w["waits"]["chrome-ui"] = json!({"mean_us": us / 4.0, "worst_us": us});
        }
        w.to_string()
    }

    #[test]
    fn the_hour_s_windows_give_the_p99_of_their_worst_waits() {
        let now = 100_000;
        let mut lines: Vec<String> = (1..=99)
            .map(|i| window(now - i * 10, Some(1_000.0)))
            .collect();
        lines.push(window(now - 5, Some(40_000.0)));
        // An hour and more ago, and lag-watch's other waits, are left out.
        lines.push(window(now - HOUR - 1, Some(900_000.0)));
        let waits = of(&lines.join("\n"), "chrome-ui", now, 0);
        assert_eq!(waits.windows, 100);
        assert_eq!(waits.worst_p99, Some(1.0));
        lines.push(window(now - 6, Some(30_000.0)));
        assert_eq!(
            of(&lines.join("\n"), "chrome-ui", now, 0).worst_p99,
            Some(30.0)
        );
    }

    #[test]
    fn windows_that_hold_the_test_call_s_own_load_are_left_out() {
        let now = 100_000;
        let began = now - 150;
        let mut lines: Vec<String> = (16..=99)
            .map(|i| window(now - i * 10, Some(1_000.0)))
            .collect();
        lines.extend((0..15).map(|i| window(now - i * 10, Some(80_000.0))));
        // The last hour's test, begun an hour before this one, ran 300 s.
        lines.extend((1..=30).map(|i| window(began - HOUR + i * 10, Some(70_000.0))));
        let waits = of(&lines.join("\n"), "chrome-ui", began, 600);
        assert_eq!(
            (waits.windows, waits.worst_p99),
            (84, Some(1.0)),
            "a window holding a test's own browsers was counted"
        );
    }

    #[test]
    fn a_window_without_the_app_counts_nothing_and_says_what_was_left_out() {
        let now = 100_000;
        let mut hidden: Value = serde_json::from_str(&window(now - 10, Some(2_000.0))).unwrap();
        hidden["lowered"] = json!({"chrome-ui": 3});
        hidden["unwatched"] = json!({"slack-ui": 1});
        let lines = [
            window(now - 20, None),
            hidden.to_string(),
            "not json".into(),
        ]
        .join("\n");
        let chrome = of(&lines, "chrome-ui", now, 0);
        assert_eq!(
            (chrome.windows, chrome.lowered, chrome.unwatched),
            (1, 1, 0)
        );
        let slack = of(&lines, "slack-ui", now, 0);
        assert_eq!(
            (slack.windows, slack.worst_p99, slack.unwatched),
            (0, None, 1)
        );
    }

    #[test]
    fn the_newest_two_days_are_read_whichever_they_are() {
        let root = std::env::temp_dir().join(format!("call-under-load-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let now = 200_000;
        fs::write(
            root.join("2026-11-18.jsonl"),
            window(now - 10, Some(99_000.0)),
        )
        .unwrap();
        fs::write(
            root.join("2026-11-19.jsonl"),
            window(now - 3_000, Some(3_000.0)),
        )
        .unwrap();
        fs::write(
            root.join("2026-11-20.jsonl"),
            window(now - 10, Some(5_000.0)),
        )
        .unwrap();
        fs::write(root.join("status.json"), window(now - 1, Some(77_000.0))).unwrap();
        let waits = waits(&root, "chrome-ui", now, 0);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!((waits.windows, waits.worst_p99), (2, Some(5.0)));
    }
}
