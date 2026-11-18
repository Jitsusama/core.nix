//! What a message Joel typed to pi says of how it's going, read from its
//! words in memory alone: whether it repeats his last, reads as a
//! correction, asks where things stand, only sets a session going again,
//! or wasn't typed by him at all.
//! Each answer is a flag; no word or length of the text leaves here.

/// How alike two messages must be, by their letter pairs, to be one said
/// again.
pub const ALIKE: f64 = 0.6;

/// Words a swear can sharpen into praise, as in "looks damn good".
pub const PRAISE: [&str; 10] = [
    "amazing",
    "awesome",
    "beautiful",
    "brilliant",
    "cool",
    "fantastic",
    "good",
    "great",
    "gorgeous",
    "nice",
];

/// Whether a message reads as a correction: the last reply didn't do it.
pub fn correcting(text: &str) -> bool {
    let low = lowered(text);
    let words: Vec<&str> = low
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .collect();
    let said = |w: &str| words.contains(&w);
    words
        .first()
        .is_some_and(|w| ["no", "nope", "wrong"].contains(w))
        || ["again", "still", "keeps", "wtf", "ffs", "broken", "stuck"]
            .iter()
            .any(|w| said(w))
        || words.iter().enumerate().any(|(at, w)| {
            // A swear before a word of praise is delight, not a fault.
            ["fuck", "shit", "damn"].iter().any(|s| w.starts_with(s))
                && !words.get(at + 1).is_some_and(|next| PRAISE.contains(next))
        })
        || [
            "not that",
            "every time",
            "doesn't work",
            "not working",
            "that's not",
            "why did",
            "why didn't",
            "why is",
            "why isn't",
        ]
        .iter()
        .any(|phrase| low.contains(phrase))
}

/// Whether a message asks where things stand.
pub fn asking_status(text: &str) -> bool {
    let low = lowered(text);
    [
        "where are we",
        "where we at",
        "where are things",
        "status?",
        "status update",
        "where do things stand",
        "what's the latest",
        "what's the status",
        "what's happening",
        "what is happening",
        "what's left",
        "any progress",
        "how's it going",
        "how is it going",
    ]
    .iter()
    .any(|phrase| low.contains(phrase))
}

/// Whether a message only sets a session going again, as a "." or a
/// "carry on" typed into one that went quiet: what follows is work he
/// handed it, not a reply he waits on.
pub fn nudging(text: &str) -> bool {
    let low = lowered(text);
    let words: Vec<&str> = low
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        // Only marks: a "." nudges, a "?" asks.
        return low.contains('.') && !low.contains('?');
    }
    [
        "continue",
        "carry on",
        "go on",
        "go ahead",
        "keep going",
        "keep at it",
        "proceed",
        "resume",
    ]
    .contains(&words.join(" ").as_str())
}

/// How alike two messages are, from 0 to 1: the share of letter pairs
/// they have in common (Dice's coefficient).
pub fn alike(a: &str, b: &str) -> f64 {
    let pairs = |s: &str| {
        let letters: Vec<char> = lowered(s).chars().filter(|c| !c.is_whitespace()).collect();
        let mut pairs: Vec<(char, char)> = letters.windows(2).map(|w| (w[0], w[1])).collect();
        pairs.sort_unstable();
        pairs
    };
    let (a, b) = (pairs(a), pairs(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let (mut i, mut j, mut shared) = (0, 0, 0_u32);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                shared += 1;
                i += 1;
                j += 1;
            }
        }
    }
    2.0 * f64::from(shared) / f64::from(u32::try_from(a.len() + b.len()).unwrap_or(u32::MAX))
}

/// Whether two pains say one thing: whether more than half their words
/// that carry meaning agree. Letter pairs can't tell: on 17 November any
/// two long messages of his shared about half theirs, so a page that read
/// worse and a call he couldn't share counted as one pain hit twice.
pub fn same(a: &str, b: &str) -> bool {
    let (a, b) = (meaning(a), meaning(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let shared = a.intersection(&b).count();
    #[expect(clippy::cast_precision_loss, reason = "a message has few words")]
    let agree = (2 * shared) as f64 / (a.len() + b.len()) as f64;
    agree > 0.5
}

/// Words too common to say what a pain is about.
const NO_MEANING: &[&str] = &[
    "the", "and", "but", "for", "with", "are", "was", "this", "that", "not", "does", "have", "has",
    "just", "can", "how", "what", "when", "its", "it's", "i'm", "all", "any", "from", "then",
    "there", "their", "them", "they", "too", "very", "our", "you",
];

/// A message's words that carry meaning: lowered, past two letters, and
/// not among [`NO_MEANING`].
fn meaning(text: &str) -> std::collections::BTreeSet<String> {
    lowered(text)
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '\''))
        .map(|word| word.trim_matches('\''))
        .filter(|word| word.len() > 2 && !NO_MEANING.contains(word))
        .map(str::to_owned)
        .collect()
}

/// Whether a user message came from somewhere other than his keys: the
/// harness, a peer, the launcher's steering or a compaction's carry-on.
pub fn automated(text: &str) -> bool {
    let text = text.trim_start();
    [
        "[",
        "<",
        "Mesh message",
        "Reloaded",
        "At a glance",
        "The conversation history",
        "Carry on",
        "Continue",
        "You are ",
        "Your ",
        "Woken by ",
    ]
    .iter()
    .any(|opening| text.starts_with(opening))
}

fn lowered(text: &str) -> String {
    text.to_lowercase().replace('\u{2019}', "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrections_and_status_questions_are_told_apart() {
        assert!(correcting("it's still slow"), "still");
        assert!(correcting("No, the other window"), "an opening no");
        assert!(correcting("wtf happened"), "a swear");
        assert!(!correcting("now do the backlight"), "a plain ask");
        assert!(!correcting("know this"), "no as part of a word");
        assert!(
            !correcting("make it look fucking awesome."),
            "a swear that praises"
        );
        assert!(!correcting("that's damn good"), "a swear before praise");
        assert!(
            correcting("this fucking thing"),
            "a swear before anything else"
        );
        assert!(asking_status("where are we at?"), "where are we");
        assert!(asking_status("What\u{2019}s left"), "a curly apostrophe");
        assert!(asking_status("status update"), "his most usual ask");
        assert!(
            asking_status("where do things stand?"),
            "where things stand"
        );
        assert!(asking_status("what's the latest"), "the latest");
        assert!(
            !asking_status("make the status line green"),
            "status as a thing"
        );
    }

    #[test]
    fn a_nudge_only_sets_a_session_going_again() {
        assert!(nudging("."), "a dot");
        assert!(nudging(" ... "), "dots and spaces");
        assert!(nudging("continue"), "continue");
        assert!(nudging("Carry on!"), "carry on");
        assert!(nudging("keep going"), "keep going");
        assert!(nudging("go ahead"), "go ahead");
        assert!(!nudging(""), "nothing typed is no nudge");
        assert!(!nudging("go on with the backlight"), "an ask of its own");
        assert!(!nudging("why?"), "a question");
    }

    #[test]
    fn pains_are_one_when_their_words_agree_not_their_letters() {
        // Pairs heard on 17 November that letter pairs scored above half.
        assert!(
            same("status update pleeease", "status update"),
            "the one he said twice"
        );
        assert!(
            !same(
                "i am not able to show launcher when screen sharing over tuple; \
                 it won't show up for others",
                "i'm at the state where I have no clue how to start new work, \
                 it's just a mess"
            ),
            "two long messages about different things"
        );
        assert!(
            !same(
                "FYI; this page is worse than never. now text takes over the whole \
                 thing",
                "readying this, it has the foundations of a good idea, but it \
                 doesn't work yet"
            ),
            "nor these"
        );
        assert!(
            same(
                "The launcher is slow to open",
                "launcher so slow to open again"
            ),
            "the same words, said differently"
        );
        assert!(!same("", "the launcher"), "nothing says nothing");
        assert!(!same("it is so", "it is so"), "nor do words of no meaning");
    }

    #[test]
    fn a_wake_a_peer_sent_is_automated() {
        assert!(
            automated("Woken by 1JQPPC on WCJQSG's ruling da5a35fb (the quest ..."),
            "a peer's wake"
        );
        assert!(!automated("woken by the alarm again"), "his own words");
    }

    #[test]
    fn alike_messages_share_most_letter_pairs() {
        assert!(
            alike("fix the launcher", "fix the launcher!") > ALIKE,
            "nearly the same"
        );
        assert!(
            alike("fix the launcher", "look at the backlight") < ALIKE,
            "different asks"
        );
        assert!(
            alike("", "x").abs() < f64::EPSILON,
            "an empty message is like nothing"
        );
    }
}
