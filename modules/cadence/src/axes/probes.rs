//! The probes run for Joel's asks, each run a line of the record the
//! orchestration keeps in `mission/probes.jsonl` (J7XHQN, 2026-11-26,
//! 775a3aa8): `{"ask": id, "at": time, "passed": bool, "switch": n, "by":
//! runner}`, the ask's `id` as its line in `mission/asks.jsonl` names it.
//! The newest run of an ask is how it stands, and how long it has failed
//! is told from the first fail after its last pass, which no line says.
//! The record's reader is here alone, so every measure and row that reads
//! a probe reads it the same (1JQPPC, 2026-11-26).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use serde_json::Value;

use crate::whole::text_lines;

/// How an ask's probe stands, by its newest run, its times in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    /// When the newest run ran.
    pub at: i64,
    /// Whether the newest run passed.
    pub passed: bool,
    /// The first fail after the last pass, while it still fails.
    pub since: Option<i64>,
    /// The switch the newest run read, where it said.
    pub switch: Option<u64>,
    /// Who ran it.
    pub by: String,
}

/// The record whole, or nothing before any probe has run; or why it's
/// unread: it couldn't be read, or a line of it isn't text or isn't a run.
/// Any line lost may be an ask's newest run, so reading the rest would
/// take the run before it as how that probe stands, a failing probe read
/// as passing (round 3 on #6875, F42).
///
/// # Errors
///
/// Why the record is unread, in words that follow "as".
pub fn gather(state_home: &Path) -> Result<String, String> {
    let bytes = match fs::read(state_home.join("mission/probes.jsonl")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(format!("it couldn't be read: {error}")),
    };
    let (text, first_not) = text_lines(&bytes);
    if let Some(line) = first_not {
        return Err(format!("its line {line} isn't text"));
    }
    match unread(&text) {
        0 => Ok(text),
        1 => Err("a line of it isn't a run".to_owned()),
        lines => Err(format!("{lines} lines of it aren't runs")),
    }
}

/// One run as a line of the record gives it: its ask, when, whether it
/// passed, the switch it read and who ran it.
type Run = (String, i64, bool, Option<u64>, String);

/// The run `line` records, or none where it has no ask, time or verdict.
fn run(line: &str) -> Option<Run> {
    let run = serde_json::from_str::<Value>(line).ok()?;
    Some((
        run["ask"].as_str().filter(|id| !id.is_empty())?.to_owned(),
        seconds(&run["at"])?,
        run["passed"].as_bool()?,
        run["switch"].as_u64(),
        run["by"].as_str().unwrap_or("").to_owned(),
    ))
}

/// How many lines of `text` hold something but no run, so a writer using
/// another shape is said rather than read as every probe not run yet.
#[must_use]
pub fn unread(text: &str) -> usize {
    text.lines()
        .filter(|line| !line.trim().is_empty() && run(line).is_none())
        .count()
}

/// How each ask's probe stands, by the ask's id, from the runs in `text`
/// in any order. A line with no ask, time or verdict is no run.
#[must_use]
pub fn read(text: &str) -> BTreeMap<String, Probed> {
    let mut runs: Vec<Run> = text.lines().filter_map(run).collect();
    runs.sort_by_key(|(id, at, ..)| (id.clone(), *at));
    let mut probed: BTreeMap<String, Probed> = BTreeMap::new();
    for (id, at, passed, switch, by) in runs {
        let failing = probed.get(&id).and_then(|before| before.since);
        let since = if passed { None } else { failing.or(Some(at)) };
        probed.insert(
            id,
            Probed {
                at,
                passed,
                since,
                switch,
                by,
            },
        );
    }
    probed
}

/// Below this, a run timed by a number is timed in seconds, since in
/// milliseconds it would have run in 1973, and in seconds in 5138.
const SECONDS_UNDER: i64 = 100_000_000_000;

/// A run's time in seconds: written as a time, or as a number of
/// milliseconds, as the first runs were, or of seconds.
fn seconds(at: &Value) -> Option<i64> {
    if let Some(number) = at.as_i64() {
        return Some(if number < SECONDS_UNDER {
            number
        } else {
            number / 1000
        });
    }
    at.as_str()?
        .parse::<Timestamp>()
        .ok()
        .map(Timestamp::as_second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_with_a_line_lost_is_unread_rather_than_read_without_it() {
        let home = tempfile::tempdir().expect("a home");
        let record = home.path().join("mission/probes.jsonl");
        assert_eq!(
            gather(home.path()),
            Ok(String::new()),
            "no record is no runs yet"
        );
        fs::create_dir_all(record.parent().expect("its folder")).expect("its folder made");
        let passed = r#"{"ask": "a1", "at": "2026-11-26T12:00:00-05:00", "passed": true}"#;
        fs::write(&record, format!("{passed}\n")).expect("written");
        assert_eq!(gather(home.path()), Ok(format!("{passed}\n")));
        // A fail torn as it was appended, which may be a1's newest run.
        let torn = r#"{"ask": "a1", "at": "2026-11-26T13:00:00-05:00", "passed": fal"#;
        fs::write(&record, format!("{passed}\n{torn}\n")).expect("written");
        assert_eq!(
            gather(home.path()),
            Err("a line of it isn't a run".to_owned())
        );
        let mut bytes = format!("{passed}\n").into_bytes();
        bytes.extend_from_slice(b"{\"ask\": \"a1\", \xff}\n");
        fs::write(&record, bytes).expect("written");
        assert_eq!(gather(home.path()), Err("its line 2 isn't text".to_owned()));
    }

    #[test]
    fn the_newest_run_is_how_a_probe_stands() {
        let text = [
            r#"{"ask": "a1", "at": "2026-11-26T12:00:00-05:00", "passed": false}"#,
            concat!(
                r#"{"ask": "a1", "at": "2026-11-26T14:00:00-05:00", "passed": true,"#,
                r#" "switch": 1201, "by": "J7XHQN"}"#,
            ),
            r#"{"ask": "b2", "at": "2026-11-26T13:00:00-05:00", "passed": false}"#,
        ]
        .join("\n");
        let probed = read(&text);
        let a1 = &probed["a1"];
        assert!(a1.passed, "a later pass outweighs an earlier fail");
        assert_eq!(a1.since, None, "a passing probe fails since nothing");
        assert_eq!(a1.switch, Some(1201));
        assert_eq!(a1.by, "J7XHQN");
        assert!(!probed["b2"].passed, "a probe run only failing has failed");
    }

    #[test]
    fn a_probe_fails_since_its_first_fail_after_its_last_pass() {
        let at = |hour: u8| format!("2026-11-26T{hour}:00:00-05:00");
        let runs = [(10, false), (11, true), (12, false), (13, false)];
        // Written out of order, as runners on two machines may append them.
        let text = runs
            .iter()
            .rev()
            .map(|(hour, passed)| {
                format!(
                    r#"{{"ask": "c3", "at": "{}", "passed": {passed}}}"#,
                    at(*hour)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let c3 = &read(&text)["c3"];
        let twelve: Timestamp = at(12).parse().expect("a time");
        let thirteen: Timestamp = at(13).parse().expect("a time");
        assert_eq!(
            c3.since,
            Some(twelve.as_second()),
            "not 10:00, before its pass"
        );
        assert_eq!(c3.at, thirteen.as_second(), "the newest run's time");
    }

    #[test]
    fn a_run_timed_in_milliseconds_reads_as_one_timed_by_the_clock() {
        let text = r#"{"ask": "d4", "at": 1764176400000, "passed": true}"#;
        assert_eq!(read(text)["d4"].at, 1_764_176_400);
        let text = r#"{"ask": "d4", "at": 1764176400, "passed": true}"#;
        assert_eq!(
            read(text)["d4"].at,
            1_764_176_400,
            "one timed in seconds isn't read as run in 1970"
        );
    }

    #[test]
    fn a_line_missing_its_ask_time_or_verdict_is_no_run() {
        let text = [
            r#"{"at": "2026-11-26T12:00:00-05:00", "passed": true}"#,
            r#"{"ask": "", "at": "2026-11-26T12:00:00-05:00", "passed": true}"#,
            r#"{"ask": "e5", "passed": true}"#,
            r#"{"ask": "e5", "at": "2026-11-26T12:00:00-05:00", "passed": "yes"}"#,
            "not json",
            "",
        ]
        .join("\n");
        assert!(
            read(&text).is_empty(),
            "no line lacking an ask, a time or a verdict is a run"
        );
        assert_eq!(
            unread(&text),
            5,
            "each line holding no run is counted, a blank one not"
        );
    }
}
