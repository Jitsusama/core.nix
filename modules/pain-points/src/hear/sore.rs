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
/// hears "still failing" too. A bare "again" is no fault, since "can you
/// run it again?" is a plain ask, and "still looks" is one only when what
/// follows is.
const FAULTS: [&str; 31] = [
    "not that",
    "that's not",
    "doesn't work",
    "not working",
    "isn't working",
    "didn't work",
    "broken",
    "stuck",
    "yet again",
    "not again",
    "why did",
    "why didn't",
    "why isn't",
    "still look* like",
    "still look* off",
    "still look* wrong",
    "still look* bad",
    "still look* broken",
    "still not",
    "still nothing",
    "still none",
    "still fail*",
    "still doesn't",
    "still don't",
    "still can't",
    "still isn't",
    "still won't",
    "still broken",
    "still slow*",
    "went off the rails",
    "keeps *ing",
];

/// The words before a "slow" that make it a wait, as in "so slow" or "the
/// build is slow"; "can you slow the scroll" is a design ask.
const SLOW_AFTER: [&str; 11] = [
    "so",
    "too",
    "very",
    "is",
    "are",
    "was",
    "really",
    "still",
    "painfully",
    "it's",
    "that's",
];

/// The starts of words that are swears.
const SWEARS: [&str; 6] = ["fuck", "shit", "crap", "wtf", "ffs", "damn"];

/// Phrases that tell an agent all is well, each matched as whole words: a
/// swear said with one is his ease, as on 24 November when he swore while
/// saying a hang "is bound to happen" and to carry on, not a hurt. One
/// asked, a swear asked, or either said beside giving up, eases nothing;
/// a fault said with ease is still heard as one.
const REASSURING: [&str; 10] = [
    "that's fine",
    "that is fine",
    "it's fine",
    "it is fine",
    "all good",
    "no worries",
    "don't worry",
    "no big deal",
    "bound to happen",
    "don't let that stop you",
];

/// Phrases that give up on the agent, which no ease beside them softens:
/// "it's fine, i'll do it myself" is no reassurance. Each is whole words,
/// so "i blame myself" gives up on nothing; "whatever" gives up only as
/// a sentence's last word, so "fine, whatever" gives up and "whatever
/// works for you" doesn't.
const GIVING_UP: [&str; 4] = ["do it myself", "do this myself", "forget it", "never mind"];

/// Words that hand the agent a choice before a closing "whatever": "use
/// whatever" leaves it to the agent rather than giving up on it. "do
/// whatever" and "try whatever" are as often resigned, so they still give
/// up, as a pain sent is cheaper than one lost.
const LEAVING_IT: [&str; 4] = ["use", "pick", "take", "choose"];

/// Whether a sentence's words give up with a closing "whatever".
fn whatever_then(words: &[&str]) -> bool {
    match words {
        [.., before, "whatever"] => !LEAVING_IT.contains(before),
        ["whatever"] => true,
        _ => false,
    }
}

/// The marks that end a sentence, a full-width question mark among them.
const ENDS: [char; 5] = ['.', '!', '?', '\u{ff1f}', ';'];

/// The marks that ask.
const ASKS: [char; 2] = ['?', '\u{ff1f}'];

/// A message's words, lowered, as whole words with their apostrophes.
fn words_of(low: &str) -> Vec<&str> {
    low.split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .collect()
}

/// The swears that close a list as "and the rest", as in "consider the
/// tax brackets and shit": three of his asks heard as hurt were this,
/// each an ask for more, and no owner found a fault in them.
const FILLERS: [&str; 2] = ["shit", "crap"];

/// The words a filler swear follows, read back to the "and" or "or"
/// that joins it, so "and some shit" and "or other crap" are fillers too.
const FILLER_JOINS: [&str; 2] = ["and", "or"];

/// The words that may stand between a filler's join and its swear.
const FILLER_BETWEEN: [&str; 2] = ["some", "other"];

/// Whether the swear at `at` in a clause's words closes a list rather
/// than swearing at it: a filler after a join that follows something
/// listed in the same sentence, so "tax brackets, rates and shit" closes
/// a list and "and shit, it wiped my branch" swears.
fn filler(clause: &[&str], at: usize) -> bool {
    let before = at.checked_sub(1).map(|i| clause[i]);
    let join = if before.is_some_and(|w| FILLER_BETWEEN.contains(&w)) {
        at.checked_sub(2)
    } else {
        at.checked_sub(1)
    };
    FILLERS.contains(&clause[at])
        && join.is_some_and(|j| j > 0 && FILLER_JOINS.contains(&clause[j]))
}

/// The sentences of lowered text, split after stops and line ends, each
/// as whether it exclaims and its words; a comma ends none, since a list
/// runs across them.
fn sentences(low: &str) -> Vec<(bool, Vec<&str>)> {
    low.split_inclusive(|c: char| ENDS.contains(&c) || c == '\n')
        .map(|sentence| (sentence.trim_end().ends_with('!'), words_of(sentence)))
        .collect()
}

/// Whether lowered text swears: a swear not before a word of praise, nor
/// one closing a list in a clause that doesn't exclaim, since "it broke
/// and shit!" is said in anger.
fn swore(low: &str) -> bool {
    sentences(low).iter().any(|(exclaims, words)| {
        words.iter().enumerate().any(|(at, w)| {
            (SWEARS.iter().any(|s| w.starts_with(s)) || *w == "ass")
                && !words.get(at + 1).is_some_and(|next| PRAISE.contains(next))
                && (*exclaims || !filler(words, at))
        })
    })
}

/// The word that opens a warning of what might come: "make sure it is
/// accurate, if you fuck up my wife i will fuck you up" was said before
/// the work, of nothing that had happened, and its owner found no hurt in
/// it on 26 November.
const WARNS: &str = "if";

/// The words that say a warned thing has happened already, so "if you
/// fuck this up again" still hurts.
const ALREADY: [&str; 4] = ["again", "still", "another", "keep*"];

/// Whether a clause's words warn of what might come rather than swear at
/// what came.
fn warns(words: &[&str]) -> bool {
    words.first() == Some(&WARNS) && !ALREADY.iter().any(|already| said(words, already))
}

/// Whether a message swears at what it's about: a swear outside a clause
/// warning of what might come, in a message that doesn't put the agent at
/// ease by a sentence not asked, nor asks with a swear, nor gives up on
/// the agent.
fn swears(low: &str) -> bool {
    let each = words_of(low);
    if !swore(low) {
        return false;
    }
    let at_what_came = low
        .split_inclusive(|c: char| ENDS.contains(&c) || c == ',')
        .any(|clause| swore(clause) && !warns(&words_of(clause)));
    if !at_what_came {
        return false;
    }
    let sentences: Vec<(bool, &str, Vec<&str>)> = low
        .split_inclusive(ENDS)
        .map(|sentence| {
            (
                sentence.trim_end().ends_with(ASKS),
                sentence,
                words_of(sentence),
            )
        })
        .collect();
    let eased = sentences
        .iter()
        .any(|(asked, _, words)| !asked && REASSURING.iter().any(|ease| said(words, ease)));
    let asked_swearing = sentences
        .iter()
        .any(|(asked, sentence, _)| *asked && swore(sentence));
    let gave_up = GIVING_UP.iter().any(|up| said(&each, up))
        || sentences.iter().any(|(_, _, words)| whatever_then(words));
    !eased || asked_swearing || gave_up
}

/// How soon a message alike to an earlier one is that one said again.
pub const AGAIN_MS: i64 = 30 * 60_000;

/// A message shorter than this is too short to be heard as said again.
const AGAIN_LEAST: usize = 20;

/// Whether `fault` is said among a message's words, as whole words, so
/// "against" holds no "again". A word of it ending in `*` takes any
/// ending, and one starting with `*` any start, so "keeps *ing" hears
/// "keeps closing" but not "keeps its place".
fn said(each: &[&str], fault: &str) -> bool {
    let words: Vec<&str> = fault.split(' ').collect();
    each.windows(words.len()).any(|run| {
        run.iter().zip(&words).all(|(w, f)| {
            if let Some(stem) = f.strip_suffix('*') {
                w.starts_with(stem)
            } else if let Some(ending) = f.strip_prefix('*') {
                w.len() > ending.len() && w.ends_with(ending)
            } else {
                w == f
            }
        })
    })
}

/// The ways a message says something hurt, each a word for the finding,
/// or none.
pub fn kinds(words: &str, at: i64, earlier: &[(i64, String)]) -> Vec<&'static str> {
    let low = words.to_lowercase().replace('\u{2019}', "'");
    let each = words_of(&low);
    let mut kinds = Vec::new();
    // A "no" or "wrong" set off by a comma or stop turns down what came
    // before; "no stone unturned" or "nope, i think we just do it" don't.
    let opening_no = ["no,", "no.", "no!", "wrong,", "wrong.", "wrong!"]
        .iter()
        .any(|opening| low.trim_start().starts_with(opening));
    let spaced = format!(" {} ", each.join(" "));
    if opening_no || FAULTS.iter().any(|fault| said(&each, fault)) {
        kinds.push("corrected");
    }
    if swears(&low) {
        kinds.push("swore");
    }
    if asking_status(words) {
        kinds.push("asked how it stands");
    }
    if nudging(words) {
        kinds.push("nudged");
    }
    // A "wait" or "stop" holds him up set off as the opening "no" is, or
    // said alone; "wait for the build" is an instruction.
    let opening_hold = ["wait", "stop"].iter().any(|word| {
        let start = low.trim_start();
        start.trim_end_matches(['.', '!', ' ']) == *word
            || [",", ".", "!"]
                .iter()
                .any(|mark| start.starts_with(&format!("{word}{mark}")))
    });
    if opening_hold
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
    // after a word that says how it is, or "slowly", is a wait; "make it
    // slower" or "slow the scroll down" is a design ask.
    if each.iter().enumerate().any(|(at, w)| {
        (*w == "slow" && at > 0 && SLOW_AFTER.contains(&each[at - 1]))
            || ["slowly", "forever", "hanging", "frozen", "hung"].contains(w)
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
        assert_eq!(
            kinds(
                "shit, that hang is bound to happen. that is fine, don't let that stop you",
                AT,
                &[]
            ),
            none,
            "a swear said to put him at ease"
        );
        assert_eq!(
            kinds("it's fine but this fucking bar is broken", AT, &[]),
            vec!["corrected"],
            "ease spares the swear, not the fault said with it"
        );
        assert_eq!(kinds("now do the backlight", AT, &[]), none);
        for filler in [
            "consider canadian tax brackets and shit. help me understand the take home",
            "track the work, sizes or some shit, so people can use it",
            "add the costs and crap to the page",
        ] {
            assert_eq!(
                kinds(filler, AT, &[]),
                none,
                "a swear closing a list: {filler}"
            );
        }
        assert_eq!(
            kinds("this and shit is broken", AT, &[]),
            vec!["corrected"],
            "a filler spares the swear, not the fault said with it"
        );
        assert_eq!(
            kinds("the page and fucking bar", AT, &[]),
            vec!["swore"],
            "only a list's closing noun is a filler"
        );
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

    #[test]
    fn a_swear_warning_of_what_might_come_hurts_nothing() {
        let none: Vec<&str> = Vec::new();
        for warned in [
            "make sure it is insanely accurate, if you fuck up my wife i will fuck you up.",
            "if this shit breaks, tell me first",
        ] {
            assert_eq!(
                kinds(warned, AT, &[]),
                none,
                "a warning before the work: {warned}"
            );
        }
        for hurt in [
            "if you fuck this up again i will scream",
            "if it keeps fucking up, stop",
            "if it breaks, fuck it",
            "fuck, if it breaks tell me",
        ] {
            assert!(
                kinds(hurt, AT, &[]).contains(&"swore"),
                "a swear at what came, or outside the warning, was spared: {hurt}"
            );
        }
    }

    #[test]
    fn a_filler_closes_only_a_list_said_before_it_and_never_an_exclaimed_one() {
        let none: Vec<&str> = Vec::new();
        for filler in [
            "add the costs and other crap",
            "add the costs or crap",
            "consider the brackets, deductions, and shit",
        ] {
            assert_eq!(
                kinds(filler, AT, &[]),
                none,
                "a list's filler swore: {filler}"
            );
        }
        for hurt in [
            "And shit, it wiped my branch",
            "and shit",
            "the costs some shit",
            "the costs.\nand shit",
            "it wiped my branch and shit!",
            "add the costs and shit, this fucking page",
        ] {
            assert!(
                kinds(hurt, AT, &[]).contains(&"swore"),
                "a swear with no list before its join, no join, exclaimed, or beside a \
                 filler was heard as a filler: {hurt}"
            );
        }
    }

    #[test]
    fn ease_spares_a_swear_unless_asked_or_given_up() {
        let none: Vec<&str> = Vec::new();
        for ease in REASSURING {
            let eased = format!("shit, {ease}");
            assert_eq!(kinds(&eased, AT, &[]), none, "{eased}");
        }
        for hurt in [
            "shit, that's fine?",
            "shit. that's fine?",
            "shit, that's fine\u{ff1f}",
            "that's fine. why the fuck is it slow?",
            "it's fine, i'll do it myself, fuck this",
            "shit, it's fine. whatever.",
            "shit, it's fine, whatever",
            "shit, it's fine whatever",
            "shit, it's fine\nwhatever",
            "shit, no worries, do whatever",
            "shit, it's fine, try whatever",
        ] {
            assert!(
                kinds(hurt, AT, &[]).contains(&"swore"),
                "ease elsewhere, asked or beside giving up spared a swear: {hurt}"
            );
        }
        // Each phrase as he'd say it, so one dropped from the list is caught.
        for up in ["do it myself", "do this myself", "forget it", "never mind"] {
            let hurt = format!("shit, it's fine, {up}");
            assert!(
                kinds(&hurt, AT, &[]).contains(&"swore"),
                "ease beside giving up spared a swear: {hurt}"
            );
        }
        // Each word that leaves the choice, so one dropped from the list is caught.
        for word in ["use", "pick", "take", "choose"] {
            let eased = format!("shit, it's fine, {word} whatever");
            assert_eq!(kinds(&eased, AT, &[]), none, "leaving it undid: {eased}");
        }
        for eased in [
            "shit, it's fine. can you check the logs?",
            "shit, it's fine, i blame myself",
            "shit, that's fine, whatever works for you",
            "shit, no worries, i'll take whatever",
            "shit, it's fine, you choose whatever",
        ] {
            assert_eq!(
                kinds(eased, AT, &[]),
                none,
                "a plain ask, self-blame, or a whatever passing or leaving it to the agent \
                 undid ease: {eased}"
            );
        }
    }
}
