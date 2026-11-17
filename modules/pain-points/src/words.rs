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

/// Whether a user message came from somewhere other than his keys: the
/// harness, a peer, the launcher's steering or a compaction's carry-on.
pub fn automated(text: &str) -> bool {
    let text = text.trim_start();
    peers_note(text)
        || [
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
        ]
        .iter()
        .any(|opening| text.starts_with(opening))
}

/// The verbs a peer opens its note with when it starts, restarts or wakes
/// a session in the user's place. Only these, so Joel writing "Fixed by
/// ABC123, still broken" about a cell stays his.
const PEERS_VERBS: [&str; 6] = ["Started", "Restarted", "Woken", "Resumed", "Reopened", "Spawned"];

/// Whether a message is a peer's note in the user's place, as one starting,
/// restarting or waking a session writes: "Woken by 1JQPPC on ...", one of
/// those verbs, "by", and a cell's six-character address, which may be all
/// letters (WCJQSG) and may end in a possessive or a mark.
fn peers_note(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let (Some(verb), Some("by"), Some(address)) = (words.next(), words.next(), words.next()) else {
        return false;
    };
    let marks = [',', ':', '.', ';'];
    let address = address.trim_end_matches(marks);
    let address = address
        .strip_suffix("'s")
        .or_else(|| address.strip_suffix("\u{2019}s"))
        .unwrap_or(address)
        .trim_end_matches(marks);
    PEERS_VERBS.contains(&verb)
        && address.len() == 6
        && address
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
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
