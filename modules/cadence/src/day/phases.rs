//! Where a gate's time went, for the day, from changed-checks' own record
//! of each run, which nix-offload brings home from the lent builder it
//! ran on (`changed-runs.jsonl`): evaluating the change's tree, evaluating
//! main's again, and building. passed.rs says how long a gate took; this
//! says which part grew. On the morning of 12 November, read on a builder
//! by hand, evaluating the change's own tree was half of a gate's time, a
//! median of 7.6 minutes, and building took 1.5; a third of runs also
//! evaluated main's side again.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::day::{Day, Measure, median};

const EVIDENCE: &str = "nix-offload's changed-runs.jsonl, each changed-checks run's phases";

/// Where nix-offload keeps what it brought home, under XDG's state.
const RUNS: &str = "nix-offload/changed-runs.jsonl";

/// The gates' phases, all read from the same lines.
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "gate.evaluating",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "a gate's median minutes evaluating the change's own tree",
        unit: " min",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "gate.evaluating-main",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "the share of gates that evaluated main's side again",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    Measure {
        key: "gate.building",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "a gate's median minutes building, among gates that built",
        unit: " min",
        bar: None,
        floor: 1.0,
    },
];

/// Puts the day's phases into `day`, from what nix-offload brought home
/// under `state_home`; with nothing brought home, there are none.
pub fn read(day: &mut Day, state_home: &Path) {
    let text = fs::read_to_string(state_home.join(RUNS)).unwrap_or_default();
    day.figures.extend(figures(&text, day.from, day.to));
}

/// One run's line, as much of it as the phases need.
#[derive(Deserialize)]
struct Run {
    at: i64,
    built: Option<Built>,
    evaluated: Vec<Evaluated>,
    /// How many checks it named, or none for a keep of main's side, which
    /// compares nothing and is no gate.
    named: Option<u64>,
}

#[derive(Deserialize)]
struct Built {
    checks: u64,
    seconds: f64,
}

/// One side a run evaluated: `tree`, the change's, or `base`, main's.
#[derive(Deserialize)]
struct Evaluated {
    of: String,
    seconds: f64,
}

/// The phases' figures of the runs in `text` that started within the
/// day; a day with none has none.
#[expect(clippy::cast_precision_loss, reason = "a day's count of runs")]
pub fn figures(text: &str, from: i64, to: i64) -> BTreeMap<String, f64> {
    let runs: Vec<Run> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Run>(line).ok())
        .filter(|run| (from..to).contains(&run.at))
        // A keep evaluates main's side by design, ahead of any gate: on 12
        // November 19 of the 134 lines home were keeps, 14 evaluating main,
        // which counted put the gates' share at 50 in 134 for 36 in 115.
        .filter(|run| run.named.is_some())
        .collect();
    let minutes = |of: &str| -> Vec<f64> {
        runs.iter()
            .flat_map(|run| &run.evaluated)
            .filter(|side| side.of == of)
            .map(|side| side.seconds / 60.0)
            .collect()
    };
    let building: Vec<f64> = runs
        .iter()
        .filter_map(|run| run.built.as_ref())
        .filter(|built| built.checks > 0)
        .map(|built| built.seconds / 60.0)
        .collect();
    let mut figures = BTreeMap::new();
    if runs.is_empty() {
        return figures;
    }
    let again = runs
        .iter()
        .filter(|run| run.evaluated.iter().any(|side| side.of == "base"))
        .count();
    figures.insert(
        "gate.evaluating-main".to_owned(),
        100.0 * again as f64 / runs.len() as f64,
    );
    for (key, values) in [
        ("gate.evaluating", minutes("tree")),
        ("gate.building", building),
    ] {
        if let Some(middle) = median(&values) {
            figures.insert(key.to_owned(), middle);
        }
    }
    figures
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run's line as changed-checks writes it: when it started, each
    /// side it evaluated, and what it built.
    fn run(at: i64, tree: f64, base: Option<f64>, built: Option<f64>) -> String {
        let side = |of: &str, seconds: f64| {
            serde_json::json!({ "of": of, "chunks": [{ "checks": 9, "seconds": seconds }],
                "seconds": seconds })
        };
        let evaluated: Vec<_> = base
            .map(|seconds| side("base", seconds))
            .into_iter()
            .chain([side("tree", tree)])
            .collect();
        let built = built.map_or(
            serde_json::json!({ "checks": 0, "passed": true, "seconds": 0.0 }),
            |seconds| serde_json::json!({ "checks": 4, "passed": true, "seconds": seconds }),
        );
        serde_json::json!({ "at": at, "base": "a", "built": built, "evaluated": evaluated,
            "named": 9, "seconds": 1.0, "system": "x86_64-linux" })
        .to_string()
    }

    #[test]
    fn each_phase_is_read_from_the_days_runs() {
        let text = [
            run(1_000, 300.0, Some(120.0), Some(60.0)),
            run(2_000, 600.0, None, None),
            run(3_000, 900.0, None, Some(180.0)),
            run(4_000, 420.0, None, Some(600.0)),
            // Started the day before.
            run(500, 6_000.0, Some(6_000.0), Some(6_000.0)),
            "a line half written".to_owned(),
        ]
        .join("\n");
        let figures = figures(&text, 1_000, 10_000);
        assert_eq!(
            figures.get("gate.evaluating"),
            Some(&8.5),
            "the tree's evaluations' median, the day before's left out"
        );
        assert_eq!(
            figures.get("gate.evaluating-main"),
            Some(&25.0),
            "one in four of the day's runs evaluated main again"
        );
        assert_eq!(
            figures.get("gate.building"),
            Some(&3.0),
            "the builds' median, among runs that built"
        );
        assert!(
            super::figures("", 0, 10).is_empty(),
            "a day with nothing brought home had figures"
        );
    }

    /// A keep's line: main's side evaluated ahead of any gate, naming and
    /// building nothing.
    fn kept(at: i64) -> String {
        serde_json::json!({ "at": at, "base": "a", "built": null, "evaluated": [{ "of": "base",
            "chunks": [], "seconds": 400.0 }], "named": null, "seconds": 400.0,
            "system": "x86_64-linux" })
        .to_string()
    }

    #[test]
    fn a_keep_of_mains_side_is_no_gate() {
        let text = [
            run(1_000, 300.0, Some(120.0), Some(60.0)),
            run(2_000, 600.0, None, None),
            kept(3_000),
            kept(4_000),
        ]
        .join("\n");
        let figures = figures(&text, 1_000, 10_000);
        assert_eq!(
            figures.get("gate.evaluating-main"),
            Some(&50.0),
            "one of the day's two gates evaluated main again; keeps do so by design"
        );
        assert!(
            super::figures(&kept(3_000), 1_000, 10_000).is_empty(),
            "a day of keeps alone had gates"
        );
    }
}
