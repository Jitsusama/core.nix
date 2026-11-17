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
/// says.
fn names_a_failure(words: &str) -> bool {
    words.contains("\u{2715} a check the change touched failed")
        || words.contains("Reason: builder failed with exit code")
}

/// The words of the gate whose log is at `log` that speak for its run.
pub(super) fn words_of(log: &Path) -> String {
    fs::read_to_string(log).unwrap_or_default()
}
