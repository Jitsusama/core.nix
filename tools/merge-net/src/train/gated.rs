//! A candidate's gate, run, and how it ended, read from nix-offload's exit
//! and words: passed, red for a rider among them, or untaken by any
//! builder, which says nothing of its riders.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Run the gate in `checkout`, its words going to `log`: how it ended.
pub(super) fn gate(checkout: &Path, log: &Path) -> Gated {
    let Ok(words) = fs::File::create(log) else {
        return Gated::Red;
    };
    let Ok(errors) = words.try_clone() else {
        return Gated::Red;
    };
    let ran = Command::new("nix-offload")
        .args(["there", "--lent-only", "--changed"])
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status();
    ran.map_or(Gated::Red, |status| gated(status.code(), &words_of(log)))
}

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
