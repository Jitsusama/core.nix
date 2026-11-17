//! Which ways a message of Joel's says something hurt: he corrected an
//! agent or swore at it, asked where things stand (the system didn't tell
//! him first), only set a session going again, held it up, said a thing
//! again soon after, or said he was kept waiting. A message of none of
//! these, praise or a plain ask, hurt nothing.
//!
//! It is stricter than words.rs's `correcting`, which `read` counts as a
//! sign: what this hears goes to a cell as a finding, so a "still" that
//! only means "yet", or a swear, is no correction by itself, and a swear
//! is said as one.

use crate::words::{ALIKE, PRAISE, alike, asking_status, nudging};

/// Phrases that say the last go didn't do it, each matched as whole words;
/// one ending in `*` takes any ending on its last word, so "still fail*"
/// hears "still failing" too, and "again?" is heard only ending a
/// question or a cry.
const FAULTS: [&str; 21] = [
    "not that",
    "that's not",
    "doesn't work",
    "not working",
    "isn't working",
    "didn't work",
    "broken",
    "stuck",
    "again?",
    "yet again",
    "keeps",
    "why did",
    "why didn't",
    "why isn't",
    "still look*",
    "still not",
    "still no",
    "still fail*",
    "still doesn't",
    "still slow*",
    "went off the rails",
];

/// The starts of words that are swears.
const SWEARS: [&str; 6] = ["fuck", "shit", "crap", "wtf", "ffs", "damn"];

/// How soon a message alike to an earlier one is that one said again.
pub const AGAIN_MS: i64 = 30 * 60_000;

/// A message shorter than this is too short to be heard as said again.
const AGAIN_LEAST: usize = 20;

/// Whether `fault` is said among a message's words, as whole words, so
/// "against" holds no "again".
fn said(each: &[&str], low: &str, fault: &str) -> bool {
    if let Some(word) = fault.strip_suffix('?') {
        let mut ends = low.split(['?', '!']);
        ends.next_back();
        return ends.any(|before| {
            before
                .trim_end()
                .rsplit(|c: char| !c.is_alphanumeric())
                .next()
                == Some(word)
        });
    }
    let (fault, stem) = fault
        .strip_suffix('*')
        .map_or((fault, false), |f| (f, true));
    let words: Vec<&str> = fault.split(' ').collect();
    each.windows(words.len()).any(|run| {
        run.iter()
            .zip(&words)
            .enumerate()
            .all(|(at, (w, f))| w == f || (stem && at + 1 == words.len() && w.starts_with(f)))
    })
}

/// The ways a message says something hurt, each a word for the finding,
/// or none.
pub fn kinds(words: &str, at: i64, earlier: &[(i64, String)]) -> Vec<&'static str> {
    let low = words.to_lowercase().replace('\u{2019}', "'");
    let each: Vec<&str> = low
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .collect();
    let mut kinds = Vec::new();
    // A "no" or "wrong" set off by a comma or stop turns down what came
    // before; "no stone unturned" or "nope, i think we just do it" don't.
    let opening_no = ["no,", "no.", "no!", "wrong,", "wrong.", "wrong!"]
        .iter()
        .any(|opening| low.trim_start().starts_with(opening));
    let spaced = format!(" {} ", each.join(" "));
    if opening_no || FAULTS.iter().any(|fault| said(&each, &low, fault)) {
        kinds.push("corrected");
    }
    let swore = each.iter().enumerate().any(|(at, w)| {
        (SWEARS.iter().any(|s| w.starts_with(s)) || *w == "ass")
            && !each.get(at + 1).is_some_and(|next| PRAISE.contains(next))
    });
    if swore {
        kinds.push("swore");
    }
    if asking_status(words) {
        kinds.push("asked how it stands");
    }
    if nudging(words) {
        kinds.push("nudged");
    }
    if each
        .first()
        .is_some_and(|first| ["wait", "stop"].contains(first))
        || ["hold up", "hold on"]
            .iter()
            .any(|opening| low.trim_start().starts_with(opening))
    {
        kinds.push("held up");
    }
    if words.len() >= AGAIN_LEAST
        && earlier
            .iter()
            .any(|(then, said)| at - then < AGAIN_MS && alike(said, words) > ALIKE)
    {
        kinds.push("said again");
    }
    // Waits are words too, so "changing" holds no "hanging". A "slow"
    // after another word, or "slowly", is a wait; "make it slower" or
    // "slow the scroll down" is a design ask.
    if each.iter().enumerate().any(|(at, w)| {
        (*w == "slow" && at > 0) || ["slowly", "forever", "hanging", "frozen", "hung"].contains(w)
    }) || [" so long ", " still waiting "]
        .iter()
        .any(|phrase| spaced.contains(phrase))
    {
        kinds.push("waited");
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: i64 = 1_800_000_000_000;

    #[test]
    fn a_correction_a_swear_and_a_status_ask_each_hurt() {
        assert_eq!(kinds("no, the other window", AT, &[]), vec!["corrected"]);
        assert_eq!(
            kinds("no stone unturned, find it", AT, &[]),
            Vec::<&str>::new()
        );
        assert_eq!(
            kinds("nope, i think we just do it", AT, &[]),
            Vec::<&str>::new()
        );
        assert_eq!(kinds("this fucking thing", AT, &[]), vec!["swore"]);
        assert_eq!(
            kinds("quest backup failed yet again.", AT, &[]),
            vec!["corrected"]
        );
        assert_eq!(kinds("it broke yet again", AT, &[]), vec!["corrected"]);
        for ask in [
            "run the tests again",
            "can you run the tests again?",
            "do it again!",
            "make the fade slower",
            "slow the scroll down",
            "can you slow the scroll down",
            "wait for the build to finish, then merge",
            "stop the old timer first",
            "make sure the drawer keeps its place",
        ] {
            assert_eq!(kinds(ask, AT, &[]), Vec::<&str>::new(), "{ask}");
        }
        assert_eq!(
            kinds("why does it still look like ass?", AT, &[]),
            vec!["corrected", "swore"]
        );
        assert_eq!(
            kinds("status update?", AT, &[]),
            vec!["asked how it stands"]
        );
        assert_eq!(
            kinds("where are we at?", AT, &[]),
            vec!["asked how it stands"]
        );
        assert_eq!(kinds("continue", AT, &[]), vec!["nudged"]);
    }

    #[test]
    fn holding_up_and_waiting_hurt() {
        assert_eq!(
            kinds("hold up, that's not it", AT, &[]),
            vec!["corrected", "held up"]
        );
        assert_eq!(kinds("wait. what are you doing", AT, &[]), vec!["held up"]);
        assert_eq!(kinds("stop!", AT, &[]), vec!["held up"]);
        assert_eq!(kinds("wait", AT, &[]), vec!["held up"]);
        assert_eq!(kinds("the build is slow", AT, &[]), vec!["waited"]);
        assert_eq!(
            kinds("why is the launcher so slow", AT, &[]),
            vec!["waited"]
        );
        assert_eq!(kinds("it takes forever to open", AT, &[]), vec!["waited"]);
    }

    #[test]
    fn a_thing_said_again_soon_hurts_and_said_again_later_does_not() {
        let earlier = vec![(AT - 60_000, "open the notices from the bar".to_string())];
        assert_eq!(
            kinds("open the notices from the bar!", AT, &earlier),
            vec!["said again"]
        );
        assert!(
            kinds("open the notices from the bar!", AT + AGAIN_MS, &earlier).is_empty(),
            "half an hour on, it is a new ask"
        );
        let short = vec![(AT - 60_000, "yes".to_string())];
        assert!(
            kinds("yes", AT, &short).is_empty(),
            "a yes twice is no pain"
        );
    }

    #[test]
    fn praise_and_plain_asks_hurt_nothing() {
        let none: Vec<&str> = Vec::new();
        assert_eq!(kinds("I love this page!", AT, &[]), none);
        assert_eq!(kinds("make it look fucking awesome", AT, &[]), none);
        assert_eq!(kinds("now do the backlight", AT, &[]), none);
        assert_eq!(kinds("waiting room design looks good", AT, &[]), none);
        assert!(
            kinds(
                "don't worry about breaking sandboxes, this is still beta",
                AT,
                &[]
            )
            .is_empty(),
            "still alone says nothing went wrong"
        );
        assert_eq!(
            kinds("I want it shorter, but still stated well", AT, &[]),
            none
        );
        assert_eq!(
            kinds("lean it against the wall", AT, &[]),
            none,
            "a fault is a word"
        );
        assert_eq!(kinds("keep the keepsake", AT, &[]), none, "keeps is a word");
        assert_eq!(kinds("it still looks great", AT, &[]), none, "still praise");
        assert_eq!(
            kinds("why do we use toml here?", AT, &[]),
            none,
            "a plain why"
        );
        assert_eq!(
            kinds("we are changing the theme", AT, &[]),
            none,
            "a wait is a word"
        );
        assert_eq!(kinds("make it snowy", AT, &[]), none);
        assert_eq!(kinds("it opens slowly", AT, &[]), vec!["waited"]);
        for waited in [
            "the tab is hanging",
            "the window hung",
            "the bar is frozen",
            "it took so long",
            "i'm still waiting",
        ] {
            assert_eq!(kinds(waited, AT, &[]), vec!["waited"], "{waited}");
        }
        for failed in [
            "the build is still failing",
            "it still looks off",
            "it still isn't showing",
            "i still can't see it",
            "still nothing",
            "it still won't open",
            "it keeps closing",
        ] {
            assert_eq!(kinds(failed, AT, &[]), vec!["corrected"], "{failed}");
        }
    }
}
