//! The builders' figures for the day, from the runs nix-offload sends them
//! (`there.jsonl`): how long a VM run waited for a builder, against
//! docs/delivery.md's two minutes, and how long a passing one took, so a
//! test suite getting slower is named the day it does rather than felt
//! weeks later in every change's wait. Most runs wait for nothing, so the
//! median wait sits at nought on a slow day too; the share that waited
//! past the target is what moves when the builders run short.
//!
//! A check that failed at a commit and passed at that same commit failed
//! for nothing the commit did: it's flaky. The share of failed checks that
//! were is how often a red run was noise, and each one is named, so a
//! check that turns flaky is a finding for its owner the day it does.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::day::{Day, Measure, median};
use crate::records::{self, Run};

const EVIDENCE: &str = "nix-offload's ~/.local/state/nix-offload/there.jsonl, its VM runs";

/// The builders' measures, all read from the same file.
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "vm.wait.median",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "a VM run's median wait for a builder",
        unit: " min",
        bar: Some(2.0),
        floor: 0.5,
    },
    Measure {
        key: "vm.wait.past-target",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "the share of VM runs that waited past two minutes for a builder",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    Measure {
        key: "vm.run.median",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "a passing VM run's median length",
        unit: " min",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "checks.flaky",
        axis: "quality",
        evidence: "nix-offload's there.jsonl, its runs of one check at a commit",
        says: "the share of failed checks that passed at the same commit",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
];

/// Puts the day's builder figures and flaky checks into `day`, from
/// nix-offload's record under `state_home`; with no record, there are none.
pub fn read(day: &mut Day, state_home: &Path) {
    let runs = fs::read_to_string(state_home.join("nix-offload/there.jsonl"));
    let runs = records::parse_runs(&runs.unwrap_or_default(), day.from);
    day.figures.extend(figures(&runs, day.from, day.to));
    day.named
        .insert("flaky".to_owned(), flaky(&runs, day.from, day.to));
}

/// docs/delivery.md's target for a VM run's wait for a builder, in minutes.
const TARGET: f64 = 2.0;

/// The day's figures from the runs that started within it: the median
/// wait of the VM runs that recorded one and the share of them that
/// waited past the target, and the median length of those
/// that passed, since a failed run's length says when it stopped, not how
/// long the test takes. A day with no VM run has neither.
pub fn figures(runs: &[Run], from: i64, to: i64) -> BTreeMap<String, f64> {
    let vm: Vec<&Run> = runs
        .iter()
        .filter(|run| run.vm() && (from..to).contains(&run.at))
        .collect();
    let waits: Vec<f64> = vm
        .iter()
        .filter_map(|run| run.waited)
        .map(|seconds| seconds / 60.0)
        .collect();
    let lengths: Vec<f64> = vm
        .iter()
        .filter(|run| run.status == 0)
        .map(|run| run.seconds / 60.0)
        .collect();
    let mut figures = BTreeMap::new();
    if let Some(wait) = median(&waits) {
        figures.insert("vm.wait.median".to_owned(), wait);
        let past = waits.iter().filter(|wait| **wait > TARGET).count();
        #[expect(clippy::cast_precision_loss, reason = "a day's count of runs")]
        let share = 100.0 * past as f64 / waits.len() as f64;
        figures.insert("vm.wait.past-target".to_owned(), share);
    }
    if let Some(length) = median(&lengths) {
        figures.insert("vm.run.median".to_owned(), length);
    }
    let (failed, flaky) = flakes(runs, from, to);
    if failed > 0 {
        let flakes: u32 = flaky.values().sum();
        #[expect(clippy::cast_precision_loss, reason = "a day's count of checks")]
        let share = 100.0 * f64::from(flakes) / failed as f64;
        figures.insert("checks.flaky".to_owned(), share);
    }
    figures
}

/// Each check that failed and passed at one commit within the day, and at
/// how many commits it did.
pub fn flaky(runs: &[Run], from: i64, to: i64) -> BTreeMap<String, u32> {
    flakes(runs, from, to).1
}

/// How many checks failed at a commit within the day, and which of those
/// passed there too. Only a run of one check counts, since a run of several
/// doesn't say which failed, and only one pinned to a commit, since a
/// checkout as it stood may have changed between its runs. A builder that
/// ran short (75) or a run that never started is no check's failure.
fn flakes(runs: &[Run], from: i64, to: i64) -> (usize, BTreeMap<String, u32>) {
    // Whether each check passed and whether it failed, at each commit.
    let mut seen: BTreeMap<(&str, &str), (bool, bool)> = BTreeMap::new();
    for run in runs {
        let [check] = run.attributes.as_slice() else {
            continue;
        };
        if !(from..to).contains(&run.at) || !run.flake.starts_with("github:") {
            continue;
        }
        let (passed, failed) = seen
            .entry((run.flake.as_str(), check.as_str()))
            .or_default();
        match run.status {
            0 => *passed = true,
            1 => *failed = true,
            _ => {}
        }
    }
    let failed = seen.values().filter(|(_, failed)| *failed).count();
    let mut flaky = BTreeMap::new();
    for ((_, check), _) in seen
        .iter()
        .filter(|(_, (passed, failed))| *passed && *failed)
    {
        let name = check.rsplit_once('.').map_or(*check, |(_, name)| name);
        *flaky.entry(name.to_owned()).or_insert(0) += 1;
    }
    (failed, flaky)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(at: i64, on: &str, seconds: f64, waited: Option<f64>, status: i64) -> Run {
        Run {
            at,
            on: on.to_owned(),
            seconds,
            waited,
            status,
            ..Run::default()
        }
    }

    /// A run of `checks` built from `flake` on a builder that isn't a VM's.
    fn checked(at: i64, flake: &str, checks: &[&str], status: i64) -> Run {
        Run {
            at,
            on: "nix-offload".to_owned(),
            status,
            flake: flake.to_owned(),
            attributes: checks
                .iter()
                .map(|check| format!("checks.x86_64-linux.{check}"))
                .collect(),
            ..Run::default()
        }
    }

    /// The day's runs of one check at a commit: `fickle` failed and passed
    /// at two commits and passed alone at a third; `broken` only failed.
    fn checks() -> Vec<Run> {
        let (a, b, c) = (
            "github:o/core/aaa",
            "github:o/core/bbb",
            "github:o/core/ccc",
        );
        vec![
            checked(100, a, &["fickle"], 1),
            checked(110, a, &["fickle"], 0),
            checked(120, b, &["fickle"], 0),
            checked(130, c, &["fickle"], 1),
            checked(140, c, &["fickle"], 0),
            checked(150, a, &["broken"], 1),
            checked(160, a, &["broken"], 1),
            // A checkout as it stood may have changed between its runs.
            checked(170, ".", &["unpinned"], 1),
            checked(180, ".", &["unpinned"], 0),
            // Which of several checks failed isn't recorded.
            checked(190, a, &["one", "two"], 1),
            checked(200, a, &["one", "two"], 0),
            // A builder that ran short isn't a check's failure.
            checked(210, b, &["short"], 75),
            checked(220, b, &["short"], 0),
            checked(5_000, b, &["late"], 1),
            checked(5_010, b, &["late"], 0),
        ]
    }

    #[test]
    fn a_check_that_failed_and_passed_at_one_commit_is_flaky() {
        let runs = checks();
        assert_eq!(
            figures(&runs, 0, 1_000).get("checks.flaky"),
            Some(&(200.0 / 3.0)),
            "of the three failed checks at a commit, two passed there too"
        );
        assert_eq!(
            flaky(&runs, 0, 1_000),
            BTreeMap::from([("fickle".to_owned(), 2)]),
            "a check only failing, one unpinned, one of several, one cut \
             short or one outside the day was named flaky"
        );
    }

    #[test]
    fn the_days_vm_runs_give_their_median_wait_and_passing_length() {
        let runs = [
            run(100, "nix-offload-vm-3", 600.0, Some(60.0), 0),
            run(200, "nix-offload-vm", 300.0, Some(180.0), 0),
            run(300, "nix-offload-vm-4", 30.0, Some(300.0), 1),
            run(400, "nix-offload-vm-5", 420.0, None, 0),
            run(500, "nix-offload", 3_000.0, Some(3_000.0), 0),
            run(5_000, "nix-offload-vm", 6_000.0, Some(6_000.0), 0),
        ];
        assert_eq!(
            figures(&runs, 0, 1_000),
            BTreeMap::from([
                ("vm.run.median".to_owned(), 7.0),
                ("vm.wait.median".to_owned(), 3.0),
                ("vm.wait.past-target".to_owned(), 200.0 / 3.0),
            ]),
            "a failed run's length, a run on a builder that isn't a VM's, \
             or one outside the day was counted, or the share past the \
             target was wrong"
        );
    }

    #[test]
    fn a_day_with_no_vm_run_has_no_figure() {
        let runs = [run(100, "nix-offload", 60.0, Some(10.0), 0)];
        assert!(
            figures(&runs, 0, 1_000).is_empty(),
            "a day with no VM run and no failed check gave a figure"
        );
    }
}
