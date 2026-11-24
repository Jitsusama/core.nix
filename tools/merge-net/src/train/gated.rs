//! How a candidate's gate ended, read from nix-offload's exit and words:
//! passed, red for a rider among them, or untaken by any builder, which
//! says nothing of its riders.

use std::fs;
use std::path::Path;

/// How a candidate's gate ended.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Gated {
    /// Every check it reached passed.
    Passed,
    /// A check failed, so a rider among them breaks it.
    Red,
    /// No builder or evaluator took it, so it says nothing of its riders.
    Unsent,
}

/// How a gate that exited `code`, saying `words`, ended. A run no builder
/// or evaluator took says nothing of its riders: on 16 November, while the
/// machine's uploads were held, one made a rider fall that broke nothing.
/// But nix-offload ends with the first part that failed, a VM part no
/// builder took coming first, so words naming a failed check outrank it.
pub(super) fn gated(code: Option<i32>, words: &str) -> Gated {
    match code {
        Some(0) => Gated::Passed,
        _ if crate::watch::untaken(code) && !names_a_failure(words) => Gated::Unsent,
        _ => Gated::Red,
    }
}

/// Whether a gate's `words` say a check failed: changed-checks' verdict,
/// or nix's for a build that ran and failed, which a lost builder never
/// says, of a check main's quarantine doesn't excuse. On 23 November the
/// train's riders were built beside two checks main quarantines, whose
/// failures blocked nothing, yet nix's words for them alone would have
/// read an untaken run as red.
fn names_a_failure(words: &str) -> bool {
    let excused = excused(words);
    words.contains("\u{2715} a check the change touched failed")
        || failed_builds(words).any(|name| {
            !excused.iter().any(|check| {
                name.strip_prefix(check)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with("-part-"))
            })
        })
}

/// The checks changed-checks says failed but block nothing, since main
/// quarantines them.
fn excused(words: &str) -> Vec<&str> {
    words
        .lines()
        .filter_map(|line| {
            let (name, why) = line
                .strip_prefix("changed-checks: ")?
                .split_once(" failed, but ")?;
            why.ends_with("so it blocks nothing").then_some(name)
        })
        .collect()
}

/// The names of the derivations nix says it couldn't build because their
/// own builder failed, not a dependency: a check's, or one of its parts'.
fn failed_builds(words: &str) -> impl Iterator<Item = &str> {
    let mut building = None;
    words.lines().filter_map(move |line| {
        if let Some(path) = line.trim().strip_prefix("error: Cannot build '") {
            building = path
                .split_once(".drv'")
                .and_then(|(path, _)| path.rsplit('/').next())
                .and_then(|file| file.split_once('-'))
                .map(|(_, name)| name);
            return None;
        }
        line.trim()
            .starts_with("Reason: builder failed with exit code")
            .then(|| building.take())
            .flatten()
    })
}

/// What nix-offload says when it sets aside a replay of the change onto a
/// newer main that failed, since the failure may be main's, and runs the
/// change again on its own base.
const RUNS_AGAIN: &str = "so the change runs again on its own base";

/// The words of the gate whose log is at `log` that speak for its run:
/// those after the last replay nix-offload set aside, any byte that isn't
/// UTF-8 read as a stand-in rather than losing the rest.
pub(super) fn words_of(log: &Path) -> String {
    let bytes = fs::read(log).unwrap_or_default();
    let words = String::from_utf8_lossy(&bytes);
    words
        .rfind(RUNS_AGAIN)
        .map_or(&*words, |at| &words[at + RUNS_AGAIN.len()..])
        .to_owned()
}
