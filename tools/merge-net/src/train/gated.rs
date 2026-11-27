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

/// Whether a red gate that exited `code` ran every part it was given, so
/// the failures its words name are all it has to say: nix-offload's exit
/// when nix or a build failed. A red with a part no builder took, or
/// asked wrongly, leaves checks unbuilt, so nothing in it is excused.
pub(super) fn ran_whole(code: Option<i32>) -> bool {
    matches!(code, Some(NIX_FAILED | BUILD_FAILED))
}

/// nix-offload's exit when nix failed, a build's failure among them.
const NIX_FAILED: i32 = 1;

/// nix-offload's exit when a build failed.
const BUILD_FAILED: i32 = 100;

/// Whether a gate's `words` say a check failed: changed-checks' verdict,
/// or nix's for a build that ran and failed, which a lost builder never
/// says.
fn names_a_failure(words: &str) -> bool {
    words.contains("\u{2715} a check the change touched failed")
        || words.contains("Reason: builder failed with exit code")
}

/// What nix-offload says when it sets aside a replay of the change onto a
/// newer main that failed, since the failure may be main's, and runs the
/// change again on its own base.
pub(super) const RUNS_AGAIN: &str = "so the change runs again on its own base";

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
