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
    /// The p99 of each window's most any one thread was blocked in the
    /// kernel, in ms: on a lock, a page read back or reclaim.
    pub blocked_p99: Option<f64>,
    /// How many windows kept that time; none where the kernel doesn't.
    pub blocked_windows: usize,
}

/// The waits of the app lag-watch labels `label` over the hour before the
/// test call `began`, from the history in `folder`, leaving out the
/// test's own minutes at either end: windows ending after it began, and
/// those up to `longest` after the last hour's began, an hour before, when
/// that one may still have been running. Those windows hold a test's own
/// two browsers, and a test's minutes are more than the hour's top 1%, so
/// its p99 could be a test's load alone. Every earlier test `ran` names,
/// as when the timer fired late after a resume, is left out too. Its files
/// are named by the UTC day, so the newest two hold the last hour whenever
/// it falls.
pub fn waits(folder: &Path, label: &str, began: u64, longest: u64, ran: &[Ran]) -> Waits {
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
    of(&text, label, began, longest, ran)
}

/// When an earlier test call began and ended, in seconds.
pub type Ran = (u64, u64);

/// How long a lag-watch window is, in seconds: one ending up to this long
/// after a test ended still holds some of it.
const WINDOW: u64 = 10;

/// The earlier test calls the hour's own history names, from its lines:
/// each row's `run.started` and `at`, or, in a stand-in's rows, `at` less
/// `took_s`.
pub fn ran(history: &str) -> Vec<Ran> {
    history
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|row| {
            let at = row["at"].as_u64()?;
            let took = row["took_s"]
                .as_f64()
                .and_then(|took| std::time::Duration::try_from_secs_f64(took).ok())
                .map(|took| at.saturating_sub(took.as_secs()));
            let began = row["run"]["started"].as_u64().or(took)?;
            Some((began, at))
        })
        .collect()
}

/// The same, from the history's lines.
pub fn of(lines: &str, label: &str, began: u64, longest: u64, ran: &[Ran]) -> Waits {
    let after = began.saturating_sub(HOUR) + longest;
    let tested = |ended: u64| {
        ran.iter()
            .any(|&(from, to)| ended > from && ended <= to + WINDOW)
    };
    let mut worst = Vec::new();
    let mut blocked = Vec::new();
    let mut found = Waits::default();
    for window in lines
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        let ended = window["ended"].as_u64().unwrap_or(0);
        if ended <= after || ended > began || tested(ended) {
            continue;
        }
        if let Some(us) = window["waits"][label]["worst_us"].as_f64() {
            worst.push(us / 1000.0);
        }
        if let Some(ms) = window["waits"][label]["blocked_ms"].as_f64() {
            blocked.push(ms);
        }
        found.lowered += usize::from(window["lowered"][label].as_u64().unwrap_or(0) > 0);
        found.unwatched += usize::from(window["unwatched"][label].as_u64().unwrap_or(0) > 0);
    }
    found.windows = worst.len();
    found.worst_p99 = percentile(&worst, 99);
    found.blocked_windows = blocked.len();
    found.blocked_p99 = percentile(&blocked, 99);
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
        let waits = of(&lines.join("\n"), "chrome-ui", now, 0, &[]);
        assert_eq!(waits.windows, 100);
        assert_eq!(waits.worst_p99, Some(1.0));
        lines.push(window(now - 6, Some(30_000.0)));
        assert_eq!(
            of(&lines.join("\n"), "chrome-ui", now, 0, &[]).worst_p99,
            Some(30.0)
        );
    }

    #[test]
    fn the_hour_s_windows_give_the_p99_of_the_most_a_thread_was_blocked() {
        let now = 100_000;
        let blocked = |ended: u64, ms: f64| {
            let mut w: Value = serde_json::from_str(&window(ended, Some(1_000.0))).unwrap();
            w["waits"]["chrome-ui"]["blocked_ms"] = json!(ms);
            w.to_string()
        };
        // Each window blocked a different time, 1 to 99 ms, and one 5 s, so
        // the p99 (99) reads apart from the most (5000) and the mean (about
        // 60), and the windows' order isn't the times'.
        let mut lines: Vec<String> = (1..=99_u32)
            .map(|i| blocked(now - u64::from(i) * 10, f64::from((i * 37) % 99 + 1)))
            .collect();
        lines.push(blocked(now - 3, 5_000.0));
        // A window from before the time was kept counts toward the waits alone.
        lines.push(window(now - 5, Some(1_000.0)));
        let waits = of(&lines.join("\n"), "chrome-ui", now, 0, &[]);
        assert_eq!(
            (waits.blocked_windows, waits.blocked_p99),
            (100, Some(99.0)),
            "the windows' time blocked wasn't read as their p99"
        );
        assert_eq!(
            waits.windows, 101,
            "a window without the time blocked was dropped"
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
        // A test the timer fired late, after a resume, half an hour ago.
        let late = (now - 1950, now - 1810);
        lines.extend((0..15).map(|i| window(now - 1800 - i * 10, Some(60_000.0))));
        let waits = of(&lines.join("\n"), "chrome-ui", began, 600, &[late]);
        assert_eq!(
            (waits.windows, waits.worst_p99),
            (84, Some(1.0)),
            "a window holding a test's own browsers was counted"
        );
    }

    #[test]
    fn the_runs_are_read_from_the_module_s_rows_and_a_stand_in_s() {
        let history = [
            json!({"at": 5_000, "run": {"started": 4_800, "seconds": 150}}).to_string(),
            json!({"at": 9_000, "took_s": 163.4}).to_string(),
            json!({"at": 9_500}).to_string(),
            "not json".into(),
        ]
        .join("\n");
        assert_eq!(
            ran(&history),
            vec![(4_800, 5_000), (8_837, 9_000)],
            "a run in the history wasn't read as it was"
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
        let chrome = of(&lines, "chrome-ui", now, 0, &[]);
        assert_eq!(
            (chrome.windows, chrome.lowered, chrome.unwatched),
            (1, 1, 0)
        );
        let slack = of(&lines, "slack-ui", now, 0, &[]);
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
        let waits = waits(&root, "chrome-ui", now, 0, &[]);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!((waits.windows, waits.worst_p99), (2, Some(5.0)));
    }
}
