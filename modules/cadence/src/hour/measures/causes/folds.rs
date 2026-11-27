//! The classes a cause is folded into, each by the words its many
//! wordings share. On 26 November one class came in at least twenty
//! wordings from eighteen cells, so names matched exactly would split it
//! into ones each found on fewer than three changes, and the measure
//! would read nought on a week that found it on forty. The keeper folded
//! that day's 169 lines by hand with these patterns, and a wording none of
//! them names is counted under its own words, marked unfolded, so a
//! pattern missing from here shows on the card rather than hiding.

/// The mark an unfolded class's name opens with.
pub const UNFOLDED: &str = "(unfolded) ";

/// A test passes with the fix undone, or can't fail.
pub const TEST_PASSES_UNDONE: &str = "a test passes with the fix undone, or can't fail";
/// Words say other than the code: a comment, description or commit.
pub const WORDS_DIFFER: &str = "words say other than the code";
/// An error or partial answer read as empty or quiet.
pub const ERROR_READ_EMPTY: &str = "an error or partial answer read as empty or quiet";
/// A fix made on one path, not on its twin.
pub const TWIN: &str = "a fix on one path, not its twin";
/// Main moves under a gate or run that passed.
pub const MAIN_MOVES: &str = "main moves under a passed gate or run";
/// Behaviour on the laptops checked only with GNU or Linux tools.
pub const GNU_ONLY: &str = "mac behaviour checked only with gnu or linux tools";
/// A pinned copy lags the tree it was taken from.
pub const PIN_LAGS: &str = "a pinned copy lags the tree";
/// A timing check fails under load.
pub const TIMING: &str = "a timing check fails under load";
/// A result waited on isn't sent to the one waiting.
pub const UNSENT: &str = "a result waited on isn't sent to the waiter";
/// A lent builder behaves unlike the run done locally.
pub const LENT_BUILDER: &str = "lent builder unlike the local run";

/// One way of saying a class: a run of steps, each a set of words, one of
/// which must come after the step before it, as a pattern `a.*(b|c)` does.
type Pattern = &'static [&'static [&'static str]];

/// Each class and the ways of saying it, tried in order, the first naming
/// a cause taking it, so a narrow class comes before a broad one that
/// shares a word with it. A word matches only where one starts, so "test"
/// isn't found in "latest"; it may end inside one, so "pass" finds
/// "passes", unless it ends with a space, as "pin " does, which matches
/// only a whole word, so it isn't found in "ping".
const FOLDS: &[(&str, &[Pattern])] = &[
    (TWIN, &[&[&["twin"]]]),
    (
        LENT_BUILDER,
        &[&[&["lent builder", "builder kernel", "lender"]]],
    ),
    (
        TEST_PASSES_UNDONE,
        &[
            &[
                &["test"],
                &[
                    "pass",
                    "can't fail",
                    "cant fail",
                    "cannot fail",
                    "undone",
                    "without",
                    "stand-in",
                    "wrong behaviour",
                    "pinning",
                    "supplies",
                    "off the filesystem",
                    "claims",
                    "less than",
                ],
            ],
            &[&["fix with no test", "red never tried", "planted"]],
            &[&["error path", "error branch"], &["untested"]],
            &[&["untested"], &["error"]],
            &[&["test fixture unlike"]],
            &[&["fixture"], &["differ"]],
            &[&["stand-in that differs"]],
        ],
    ),
    (
        WORDS_DIFFER,
        &[&[
            &[
                "comment",
                "description",
                "doc ",
                "docs ",
                "documentation",
                "commit message",
                "pr text",
                "guide",
                "test name",
                "status words",
                "failure line",
                "failure message",
            ],
            &[
                "say",
                "claim",
                "stale",
                "out of date",
                "contradict",
                "misdescrib",
                "other",
                "more",
                "names no",
                "hides",
                "disagree",
                "dropped",
                "before a rebase",
                "check",
            ],
        ]],
    ),
    (
        ERROR_READ_EMPTY,
        &[&[&[
            "error",
            "partial",
            "listing failure",
            "torn",
            "unreadable",
            "discarded",
            "never began",
            "reports success",
        ]]],
    ),
    (
        MAIN_MOVES,
        &[&[&[
            "main mov",
            "voided by main",
            "shared readme",
            "shared budget",
            "readme map",
            "parallel prs",
            "parallel changes",
            "retarget or rebase",
            "lone merge",
            "stale tracking main",
        ]]],
    ),
    (GNU_ONLY, &[&[&["mac ", "macs ", "macos", "bsd ", "gnu "]]]),
    (
        PIN_LAGS,
        &[&[&[
            "pin ",
            "pins ",
            "pinned",
            "pinning",
            "older mycelium",
            "old binary",
            "override in a prebuild",
            "older than the target",
        ]]],
    ),
    (
        TIMING,
        &[&[&[
            "under load",
            "under builder load",
            "loaded builder",
            "timeout",
            "times out",
            "deadline",
            "flaky",
            "flakiness",
            "retry schedule",
            "suspend",
            "monotonic",
            "clock",
        ]]],
    ),
    (
        UNSENT,
        &[&[&["unseen", "unheard", "fyi", "wake", "journaled, never shown"]]],
    ),
];

/// The class `plain`, a cause written plain and lower case, is folded
/// into, or its own words marked unfolded where no pattern names it.
#[must_use]
pub fn fold(plain: &str) -> String {
    let padded = format!(" {plain} ");
    FOLDS
        .iter()
        .find(|(_, ways)| ways.iter().any(|way| says(&padded, way)))
        .map_or_else(
            || format!("{UNFOLDED}{plain}"),
            |(class, _)| (*class).to_owned(),
        )
}

/// Whether `text` says each step of `way` in turn, each after the one
/// before it ends.
fn says(text: &str, way: Pattern) -> bool {
    let mut from = 0;
    way.iter().all(|step| {
        let end = step.iter().filter_map(|word| found(text, from, word)).min();
        end.inspect(|&end| from = end).is_some()
    })
}

/// Where `word` first ends in `text` at or after `from`, found only where
/// a word starts, and, for one ending in a space, only where a word ends:
/// a letter or digit never stands either side of it.
fn found(text: &str, from: usize, word: &str) -> Option<usize> {
    let whole = word.strip_suffix(' ');
    let needle = whole.unwrap_or(word);
    let inside = |c: char| c.is_alphanumeric();
    text[from..].match_indices(needle).find_map(|(at, _)| {
        let start = from + at;
        let end = start + needle.len();
        let starts =
            !needle.starts_with(inside) || !text[..start].chars().next_back().is_some_and(inside);
        let ends = whole.is_none() || !text[end..].chars().next().is_some_and(inside);
        (starts && ends).then_some(end)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wordings cells sent the keeper on 26 November, each beside the
    /// class the keeper folded it into by hand.
    const SAID: &[(&str, &str)] = &[
        ("test passes with the fix undone", TEST_PASSES_UNDONE),
        ("test passes without the code it names", TEST_PASSES_UNDONE),
        ("a red never tried", TEST_PASSES_UNDONE),
        (
            "comment or description says other than the code",
            WORDS_DIFFER,
        ),
        ("test name misdescribes what it plants", WORDS_DIFFER),
        ("failure message names no cause", WORDS_DIFFER),
        ("listing failure reads as empty", ERROR_READ_EMPTY),
        ("git's error discarded and read as absent", ERROR_READ_EMPTY),
        ("fix applied to one call site but not its twin", TWIN),
        ("host run voided by main moving", MAIN_MOVES),
        ("shared budget line clashes across parallel prs", MAIN_MOVES),
        ("mac checks run only with gnu tools", GNU_ONLY),
        (
            "a test with no platform gate breaks the mac's build",
            GNU_ONLY,
        ),
        (
            "runtime drop-in pins an old binary across a switch",
            PIN_LAGS,
        ),
        ("image ships an older mycelium than the host", PIN_LAGS),
        ("a timing test that gives up on a loaded builder", TIMING),
        ("a time read once that a suspend ages", TIMING),
        ("date moved late, a train split went unseen", UNSENT),
        ("background wake reports late after a pi restart", UNSENT),
        ("lent builder lacks what the local run has", LENT_BUILDER),
    ];

    #[test]
    fn each_wording_cells_sent_folds_into_the_class_the_keeper_gave_it() {
        for &(said, class) in SAID {
            assert_eq!(fold(said), class, "{said:?} folded into the wrong class");
        }
    }

    /// The hand fold took the first two for laptop and pinning causes, by
    /// "mac" inside "machine" and "pin" inside "stopping"; a review of
    /// the fold found each of the rest misfolded by a word inside another.
    #[test]
    fn a_word_inside_another_folds_nothing_and_an_unnamed_cause_shows_unfolded() {
        for said in [
            "after-check proves machine health, not the change",
            "a walk that stops to give way when stopping frees nothing",
            "a module left out of flake.nix",
            "nix flake check skipped on darwin",
            "the attestation step bypasses signing",
            "uses the latest main without rebasing",
            "a docker image built with another base",
            "an awake host refuses a terror",
        ] {
            assert_eq!(
                fold(said),
                format!("{UNFOLDED}{said}"),
                "{said:?} was folded by a word found inside another"
            );
        }
    }

    /// A whole word still folds beside punctuation, a word ending inside
    /// a longer one folds as before, and a narrow class comes before a
    /// broad one sharing its words.
    #[test]
    fn a_whole_word_folds_beside_punctuation_and_a_narrow_class_comes_first() {
        for (said, class) in [
            ("a ping probe times out on a loaded builder", TIMING),
            ("the mac's build breaks", GNU_ONLY),
            ("a pin, kept past the switch", PIN_LAGS),
            ("a fix on the error path, not its twin", TWIN),
            ("a flaky check under a busy builder", TIMING),
            ("the docs say more than the code does", WORDS_DIFFER),
        ] {
            assert_eq!(fold(said), class, "{said:?} folded into the wrong class");
        }
    }
}
