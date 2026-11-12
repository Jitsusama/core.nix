//! The merge gate's figures for the day, from what merge-net's `ready`
//! answered each change asking to merge (`net/answers.jsonl`): the share
//! held back for an earlier gate sharing a file, the share sent to run
//! their checks again for a file main moved, and the files each turned
//! on. A file that holds changes back is one every change queues on, so
//! one newly doing so is a finding the day it starts. On 12 November, by
//! 07:00, four lists a new module adds a line to had held 23 changes back.
//!
//! A change is counted by its head, once whatever it was answered how
//! often: a change held back asks again every few minutes until it goes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::day::{Day, Measure};

const EVIDENCE: &str = "merge-net's ~/.local/state/net/answers.jsonl, each head's answers";

/// The gate's measures, both read from the same file.
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "merge.held",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "the share of changes asking to merge that waited on another's gate",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    Measure {
        key: "merge.again",
        axis: "time to done",
        evidence: EVIDENCE,
        says: "the share of changes asking to merge sent to run their checks again",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
];

/// One answer `ready` gave: when, of which head, which way, and the files
/// it turned on.
#[derive(Deserialize)]
pub struct Answer {
    pub at: i64,
    pub head: String,
    pub verdict: String,
    #[serde(default)]
    pub files: Vec<String>,
}

/// Puts the day's gate figures and the files that held changes back into
/// `day`, from merge-net's answers under `state_home`; with none, there
/// are none.
pub fn read(day: &mut Day, state_home: &Path) {
    let text = fs::read_to_string(state_home.join("net/answers.jsonl")).unwrap_or_default();
    let answers: Vec<Answer> = text
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    day.figures.extend(figures(&answers, day.from, day.to));
    day.named.insert(
        "code.queued-on".to_owned(),
        queued_on(&answers, day.from, day.to),
    );
}

/// The share of the heads answered within the day that were held back
/// once or more, and the share sent to run again. A day nothing asked has
/// neither.
#[expect(clippy::cast_precision_loss, reason = "a day's count of changes")]
pub fn figures(answers: &[Answer], from: i64, to: i64) -> BTreeMap<String, f64> {
    // Each head's verdicts within the day.
    let mut heads: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for answer in answers
        .iter()
        .filter(|answer| (from..to).contains(&answer.at))
    {
        heads
            .entry(answer.head.as_str())
            .or_default()
            .insert(answer.verdict.as_str());
    }
    let mut figures = BTreeMap::new();
    if heads.is_empty() {
        return figures;
    }
    let share = |verdict: &str| {
        let given = heads.values().filter(|said| said.contains(verdict)).count();
        100.0 * given as f64 / heads.len() as f64
    };
    figures.insert("merge.held".to_owned(), share("hold"));
    figures.insert("merge.again".to_owned(), share("regate"));
    figures
}

/// Each file that held a change back or sent it to run again within the
/// day, and how many heads it did so to.
pub fn queued_on(answers: &[Answer], from: i64, to: i64) -> BTreeMap<String, u32> {
    let costly = answers.iter().filter(|answer| {
        (from..to).contains(&answer.at) && matches!(answer.verdict.as_str(), "hold" | "regate")
    });
    let mut heads: BTreeSet<(&str, &str)> = BTreeSet::new();
    for answer in costly {
        for file in &answer.files {
            heads.insert((file.as_str(), answer.head.as_str()));
        }
    }
    let mut named = BTreeMap::new();
    for (file, _) in heads {
        *named.entry(file.to_owned()).or_insert(0) += 1;
    }
    named
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(at: i64, head: &str, verdict: &str, files: &[&str]) -> Answer {
        Answer {
            at,
            head: head.to_owned(),
            verdict: verdict.to_owned(),
            files: files.iter().map(|file| (*file).to_owned()).collect(),
        }
    }

    /// 12 November's shape: a change held twice on flake.nix, then passed;
    /// one sent to run again for imports.nix; two that went straight
    /// through; and an answer from the day before.
    fn day() -> Vec<Answer> {
        vec![
            answer(5, "old", "hold", &["flake.nix"]),
            answer(10, "a", "hold", &["flake.nix", "checks.nix"]),
            answer(15, "a", "hold", &["flake.nix"]),
            answer(20, "a", "ok", &["modules/a/x.rs"]),
            answer(
                30,
                "b",
                "regate",
                &["modules/quickshell/checks/imports.nix"],
            ),
            answer(40, "c", "ok", &[]),
            answer(50, "d", "ok", &[]),
        ]
    }

    #[test]
    fn each_head_counts_once_however_often_it_was_answered() {
        let figures = figures(&day(), 10, 100);
        assert_eq!(
            figures.get("merge.held"),
            Some(&25.0),
            "one of four heads waited on another's gate"
        );
        assert_eq!(
            figures.get("merge.again"),
            Some(&25.0),
            "one of four heads was sent to run again"
        );
        assert!(
            super::figures(&day(), 100, 200).is_empty(),
            "a day nothing asked had a share"
        );
    }

    #[test]
    fn each_file_names_the_heads_it_held_back_or_sent_again() {
        let named = queued_on(&day(), 10, 100);
        let expected: BTreeMap<String, u32> = [
            ("checks.nix", 1),
            ("flake.nix", 1),
            ("modules/quickshell/checks/imports.nix", 1),
        ]
        .into_iter()
        .map(|(file, heads)| (file.to_owned(), heads))
        .collect();
        assert_eq!(
            named, expected,
            "a file was counted for a pass, the day before or twice for one head"
        );
    }
}
