//! A candidate's gate, run, and how it ended, read from nix-offload's exit
//! and words: passed, red for a rider among them, or untaken by any
//! builder, which says nothing of its riders.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Run the gate in `checkout`, its words going to `log`: how it ended.
pub(super) fn gate(checkout: &Path, log: &Path) -> Gated {
    let Ok(words) = fs::File::create(log) else {
        return Gated::Red { whole: false };
    };
    let Ok(errors) = words.try_clone() else {
        return Gated::Red { whole: false };
    };
    let ran = Command::new("nix-offload")
        .args(["there", "--lent-only", "--changed"])
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status();
    ran.map_or(Gated::Red { whole: false }, |status| {
        gated(status.code(), &words_of(log))
    })
}

/// How a candidate's gate ended.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Gated {
    /// Every check it reached passed.
    Passed,
    /// A check failed, so a rider among them breaks it. `whole` when
    /// nix-offload said every part of the gate ran to its end, so its
    /// words name all it broke; not when a part no builder took, a lost
    /// builder or a signal cut it short, or nix-offload didn't say.
    Red { whole: bool },
    /// No builder or evaluator took it, so it says nothing of its riders.
    Unsent,
}

/// How a gate that exited `code`, saying `words`, ended. A run no builder
/// or evaluator took says nothing of its riders: on 16 November, while the
/// machine's uploads were held, one made a rider fall that broke nothing.
/// But nix-offload ends with the first part that failed, a VM part no
/// builder took coming first, so words naming a failed check outrank it.
/// For the same reason a failed build's code can hide a part that never
/// ran to its end, so a red is whole only when nix-offload says every
/// part did, and no line says one went unbuilt: a list of the ways a
/// part is cut short would miss one it doesn't name (round 3, 23
/// November), where a missing line, reworded or never printed, fails
/// closed. So too the code: only one a failed build ends with, 1 or nix's
/// 100 to 104, can be whole (round 4).
pub(super) fn gated(code: Option<i32>, words: &str) -> Gated {
    match code {
        Some(0) => Gated::Passed,
        _ if crate::watch::untaken(code) && !names_a_failure(words) => Gated::Unsent,
        _ => Gated::Red {
            whole: matches!(code, Some(1 | NIX_FAILED_FIRST..=NIX_FAILED_LAST))
                && ran_whole(words)
                && !left_unbuilt(words),
        },
    }
}

/// The statuses `nix build` ends a failed build with: 100, and 101 to 104
/// for a timeout, a hash or a check that differed, alone or together.
const NIX_FAILED_FIRST: i32 = 100;
const NIX_FAILED_LAST: i32 = 104;

/// What nix-offload says once a run on a lent builder ends with every
/// part of it run to its end (`RAN_WHOLE` in
/// `modules/nix-offload/offload/src/main.rs`), word for word.
const RAN_WHOLE: &str = "nix-offload: every part of the run ran to its end";

/// Whether a gate's `words` hold nix-offload's word that every part of
/// its run ran to its end.
fn ran_whole(words: &str) -> bool {
    words.lines().any(|line| line.trim() == RAN_WHOLE)
}

/// What nix-offload says of a part of its run that no builder took or
/// built, or that was cut short, whatever code the run ended with.
const UNBUILT: [&str; 8] = [
    "but no builder took their build",
    "weren't built",
    "no lent builder builds",
    "no lent builder takes",
    "no lent builder took",
    "so no builder can build it",
    "couldn't send the VM tests on",
    "was cut short",
];

/// Whether a gate's `words` say nix-offload left part of it unbuilt, a
/// part no builder took, or say a nearby machine's part ended other than
/// passing. nix-offload names a nearby part only when it didn't pass, and
/// a check that failed there is named without its system, so the excuse
/// would run it again as this system's check of the same name, which can
/// pass (round 3's F9, 23 November): such a red holds rather than be
/// excused.
fn left_unbuilt(words: &str) -> bool {
    words
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nix-offload: "))
        .any(|said| {
            UNBUILT.iter().any(|unbuilt| said.contains(unbuilt))
                || (said.contains("'s part, ") && said.contains(" ended "))
        })
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
/// those after the line of nix-offload's own that last set a replay
/// aside, any byte that isn't UTF-8 read as a stand-in rather than losing
/// the rest. A check's output saying the same words, which nix prefixes
/// with its name, cuts nothing (round 4's F10).
pub(super) fn words_of(log: &Path) -> String {
    let bytes = fs::read(log).unwrap_or_default();
    let words = String::from_utf8_lossy(&bytes);
    let mut from = 0;
    let mut at = 0;
    for line in words.split_inclusive('\n') {
        at += line.len();
        let said = line.trim();
        if said.starts_with("nix-offload: ") && said.contains(RUNS_AGAIN) {
            from = at;
        }
    }
    words[from..].to_owned()
}
