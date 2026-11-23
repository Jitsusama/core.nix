//! A candidate's gate, run, and how it ended, read from nix-offload's exit
//! and words: passed, red for a rider among them, or untaken by any
//! builder, which says nothing of its riders.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

/// Run the gate in `checkout`, its words going to `log`: how it ended.
/// The log opens with a mark fresh to this run, which nix-offload signs
/// the lines the train trusts with, so the same words in a rider's output
/// stand for nothing (`words_of`).
pub(super) fn gate(checkout: &Path, log: &Path) -> Gated {
    let Ok(mut words) = fs::File::create(log) else {
        return Gated::Red { whole: false };
    };
    let mark = fresh_mark();
    if let Some(mark) = &mark
        && writeln!(words, "{MARKED}{mark}").is_err()
    {
        return Gated::Red { whole: false };
    }
    let Ok(errors) = words.try_clone() else {
        return Gated::Red { whole: false };
    };
    let mut command = Command::new("nix-offload");
    command
        .args(["there", "--lent-only", "--changed"])
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors);
    if let Some(mark) = &mark {
        command.env(MARK, mark);
    }
    let ran = command.status();
    ran.map_or(Gated::Red { whole: false }, |status| {
        gated(status.code(), &words_of(log))
    })
}

/// What the log of a gate opens with, before its mark.
const MARKED: &str = "merge-net: this gate's mark is ";

/// The variable nix-offload reads a run's mark in
/// (`modules/nix-offload/offload/src/marked.rs`).
const MARK: &str = "NIX_OFFLOAD_MARK";

/// The digits a mark is written in.
const HEX: &[u8; 16] = b"0123456789abcdef";

/// A mark no rider can guess: 16 bytes of the kernel's randomness, as
/// hex. None when it can't be read, so the gate runs unmarked and
/// nothing it says is trusted.
fn fresh_mark() -> Option<String> {
    let mut bytes = [0_u8; 16];
    fs::File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(&mut bytes))
        .ok()?;
    Some(
        bytes
            .iter()
            .flat_map(|byte| [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0xf)]])
            .map(char::from)
            .collect(),
    )
}

/// The mark a gate's `words` open with, if they do.
fn mark_of(words: &str) -> Option<&str> {
    words
        .lines()
        .next()?
        .strip_prefix(MARKED)
        .filter(|mark| mark.len() == 32 && mark.bytes().all(|byte| byte.is_ascii_hexdigit()))
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
/// 100 to 104, can be whole (round 4). changed-checks ends 1 only when
/// every build ran and one failed, and a panic in either ends 70 through
/// terminal-kit, never 101 (round 6); nix itself still ends 1 when it is
/// interrupted, which its `error: interrupted` line names, and
/// `excused::broken` refuses every `error:` line it doesn't know.
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
/// the rest. Only a line signed with the mark the log opens with is
/// nix-offload's own: a rider's evaluation writes to the log raw, and a
/// traced string's second line reads as any line it likes (round 6's
/// F17). So an unsigned replay line cuts nothing, an unsigned
/// `RAN_WHOLE` is dropped and a signed one kept bare, and a log with no
/// mark trusts neither.
pub(super) fn words_of(log: &Path) -> String {
    let bytes = fs::read(log).unwrap_or_default();
    let words = String::from_utf8_lossy(&bytes);
    let signature = mark_of(&words).map(|mark| format!(" [{mark}]"));
    let mut kept = String::new();
    for line in words.split_inclusive('\n') {
        let said = line.trim();
        let own = signature
            .as_deref()
            .and_then(|signature| said.strip_suffix(signature));
        match own {
            Some(own) if own.starts_with("nix-offload: ") && own.contains(RUNS_AGAIN) => {
                kept.clear();
            }
            Some(RAN_WHOLE) => {
                kept.push_str(RAN_WHOLE);
                kept.push('\n');
            }
            _ if said == RAN_WHOLE || said.starts_with(MARKED) => {}
            _ => kept.push_str(line),
        }
    }
    kept
}

#[cfg(test)]
mod tests;
