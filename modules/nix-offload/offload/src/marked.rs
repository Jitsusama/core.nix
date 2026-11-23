//! The mark a caller hands a run, so the lines it trusts can be told
//! apart from the same words in a check's or an evaluation's output, which
//! the same log carries.

use std::env;
use std::sync::OnceLock;

/// The variable a caller names its run's mark in. merge-net's train gives
/// one to each gate, since a rider's evaluation writes to the gate's log
/// raw: a traced string's second line reads as a line of nix-offload's own
/// (round 6's F17, 23 November).
pub const MARK: &str = "NIX_OFFLOAD_MARK";

/// The shortest and longest mark taken, of letters and digits alone.
const SHORTEST: usize = 16;
const LONGEST: usize = 128;

/// The run's mark, once taken.
static TAKEN: OnceLock<Option<String>> = OnceLock::new();

/// Take the run's mark out of the environment, so nothing this run starts,
/// a rider's evaluation least of all, can read it.
///
/// # Safety
///
/// Call it before any thread starts, since it changes the environment.
pub unsafe fn take() {
    let mark = env::var(MARK).ok().filter(|mark| well_formed(mark));
    // SAFETY: the caller calls this before any thread starts, so nothing
    // reads the environment while it changes.
    unsafe { env::remove_var(MARK) };
    let _ = TAKEN.set(mark);
}

/// Whether `mark` is one this takes: letters and digits, of a length a
/// guess can't reach and a line can hold.
fn well_formed(mark: &str) -> bool {
    (SHORTEST..=LONGEST).contains(&mark.len())
        && mark.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

/// `line`, signed with the run's mark when its caller gave one.
pub fn signed(line: &str) -> String {
    signed_by(line, TAKEN.get().and_then(Option::as_deref))
}

/// `line`, signed with `mark` when there is one.
fn signed_by(line: &str, mark: Option<&str>) -> String {
    mark.map_or_else(|| line.to_owned(), |mark| format!("{line} [{mark}]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_signed_only_with_a_mark() {
        // tools/merge-net/src/train/gated.rs reads this shape word for word.
        assert_eq!(
            signed_by("nix-offload: x", Some("0123456789abcdef")),
            "nix-offload: x [0123456789abcdef]",
            "a marked line isn't signed as merge-net reads it"
        );
        assert_eq!(
            signed_by("nix-offload: x", None),
            "nix-offload: x",
            "an unmarked run signed its line"
        );
    }

    #[test]
    fn only_a_mark_of_letters_and_digits_long_enough_is_taken() {
        assert!(
            well_formed(&"a".repeat(SHORTEST)),
            "a mark of the shortest length was refused"
        );
        assert!(
            well_formed(&"Z9".repeat(LONGEST / 2)),
            "a mark of the longest length was refused"
        );
        for refused in [
            "",
            "short",
            &"a".repeat(LONGEST + 1),
            "0123456789abcdef]\nx",
            "0123456789 abcdef",
        ] {
            assert!(!well_formed(refused), "the mark {refused:?} was taken");
        }
    }
}
