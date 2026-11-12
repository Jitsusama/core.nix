//! How long a gate took, for the day, from what nix-offload keeps of each
//! changed-checks run that passed (`nix-offload-passed`): its median and
//! its ninetieth percentile, start to pass. A gate is what every change
//! waits on between its last commit and its merge, so one growing slower
//! is named the day it does, before every change feels it. Each run's
//! line says when it started and when it passed; checks passed later at
//! the same commit join the same file, so the file's own time says
//! nothing of the gate's end. On 12 November, read from the files' times,
//! a day's 716 gates took a median of 16.5 minutes and 34 at the p90.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;

use crate::day::{Day, Measure};
use crate::stats::quantile;

const EVIDENCE: &str = "nix-offload's passed records, each changed-checks run's start and end";

/// Where the keeper keeps what passed, where the machine runs one.
const KEPT: &str = "/var/lib/nix-offload-passed";

/// The gates' measures, both read from the same records.
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "gate.median",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "a passing gate's median length, start to pass",
        unit: " min",
        bar: None,
        floor: 2.0,
    },
    Measure {
        key: "gate.p90",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "the length nine in ten passing gates came within",
        unit: " min",
        bar: None,
        floor: 4.0,
    },
];

/// One line of a commit's record. Only a run's line has both times.
#[derive(Deserialize)]
struct Line {
    started: Option<i64>,
    ended: Option<i64>,
    /// What the run was given: `--name-only` is merge-net naming a
    /// merge's checks, in seconds, not a gate.
    #[serde(default)]
    run: Vec<String>,
}

/// Puts the day's gate lengths into `day`, from the keeper's records, or
/// those under `state_home` where no keeper runs; with none, there are
/// none.
pub fn read(day: &mut Day, state_home: &Path) {
    let kept = Path::new(KEPT);
    let folder = if kept.is_dir() {
        kept.to_path_buf()
    } else {
        state_home.join("nix-offload/passed")
    };
    let lengths = lengths(&records(&folder, day.from), day.from, day.to);
    day.figures.extend(figures(&lengths));
}

/// The text of each record in `folder` written to since `from`, since one
/// older has no gate that passed within the day.
fn records(folder: &Path, from: i64) -> Vec<String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let since = |path: &PathBuf| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|since| i64::try_from(since.as_secs()).ok())
            .is_some_and(|seconds| seconds >= from)
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(since)
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect()
}

/// The length in minutes of each gate in `records` that passed within
/// the day.
#[expect(clippy::cast_precision_loss, reason = "a gate's seconds")]
pub fn lengths(records: &[String], from: i64, to: i64) -> Vec<f64> {
    records
        .iter()
        .flat_map(|text| text.lines())
        .filter_map(|line| serde_json::from_str::<Line>(line).ok())
        .filter(|line| !line.run.iter().any(|given| given == "--name-only"))
        .filter_map(|line| Some((line.started?, line.ended?)))
        .filter(|(started, ended)| (from..to).contains(ended) && ended >= started)
        .map(|(started, ended)| (ended - started) as f64 / 60.0)
        .collect()
}

/// The median and the p90 of `lengths`; a day no gate passed has neither.
pub fn figures(lengths: &[f64]) -> BTreeMap<String, f64> {
    let mut figures = BTreeMap::new();
    if let (Some(median), Some(p90)) = (quantile(lengths, 0.5), quantile(lengths, 0.9)) {
        figures.insert("gate.median".to_owned(), median);
        figures.insert("gate.p90".to_owned(), p90);
    }
    figures
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A commit's record as nix-offload keeps it: a check's line, the
    /// gate's run, and a check kept later at the same commit.
    fn record(started: i64, ended: i64) -> String {
        let check = |name: &str| serde_json::json!({ "check": name, "dir": null });
        let run = serde_json::json!({
            "dir": null,
            "run": ["--build"],
            "printed": "",
            "started": started,
            "ended": ended,
        });
        format!("{}\n{run}\n{}\n", check("checks.x.a"), check("checks.x.b"))
    }

    #[test]
    fn a_gate_runs_from_its_start_to_its_pass() {
        let records = vec![
            record(1_000, 1_600),
            record(2_000, 2_300),
            record(3_000, 4_800),
            // Passed the day before, though its file was written to since.
            record(0, 900),
            // A run kept before the field, with no end to read.
            r#"{"dir":null,"run":[],"printed":"","started":1500}"#.to_owned(),
            // merge-net naming a merge's checks, which is no gate.
            r#"{"dir":null,"run":["--name-only"],"started":1500,"ended":1520}"#.to_owned(),
        ];
        assert_eq!(
            lengths(&records, 1_000, 10_000),
            [10.0, 5.0, 30.0],
            "a gate was read from a check's line, the day before, with no end or naming"
        );
        let figures = figures(&[10.0, 5.0, 30.0]);
        assert_eq!(
            figures.get("gate.median"),
            Some(&10.0),
            "the middle gate's length"
        );
        assert_eq!(
            figures.get("gate.p90"),
            Some(&30.0),
            "the length nine in ten came within"
        );
        assert!(
            super::figures(&[]).is_empty(),
            "a day no gate passed had a length"
        );
    }

    #[test]
    fn only_records_written_within_the_day_are_read() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("a"), record(1, 2)).unwrap();
        assert_eq!(
            records(folder.path(), 0).len(),
            1,
            "today's record went unread"
        );
        assert!(
            records(folder.path(), i64::MAX).is_empty(),
            "a record untouched since before the day was read"
        );
        assert!(
            records(&folder.path().join("none"), 0).is_empty(),
            "a missing folder gave records"
        );
    }
}
