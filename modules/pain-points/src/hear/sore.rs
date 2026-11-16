//! Which ways a message of Joel's says something hurt: he corrected an
//! agent or swore at it, asked where things stand (the system didn't tell
//! him first), only set a session going again, held it up, said a thing
//! again soon after, or said he was kept waiting. A message of none of
//! these, praise or a plain ask, hurt nothing.

/// How soon a message alike to an earlier one is that one said again.
pub const AGAIN_MS: i64 = 30 * 60_000;

/// A message shorter than this is too short to be heard as said again.
const AGAIN_LEAST: usize = 12;

/// The ways a message says something hurt, each a word for the finding,
/// or none.
pub fn kinds(words: &str, at: i64, earlier: &[(i64, String)]) -> Vec<&'static str> {
    let _ = (words, at, earlier, AGAIN_LEAST);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: i64 = 1_800_000_000_000;

    #[test]
    fn a_correction_a_swear_and_a_status_ask_each_hurt() {
        assert_eq!(kinds("no, the other window", AT, &[]), vec!["corrected"]);
        assert_eq!(kinds("this fucking thing", AT, &[]), vec!["corrected"]);
        assert_eq!(kinds("status update?", AT, &[]), vec!["asked how it stands"]);
        assert_eq!(kinds("where are we at?", AT, &[]), vec!["asked how it stands"]);
        assert_eq!(kinds("continue", AT, &[]), vec!["nudged"]);
    }

    #[test]
    fn holding_up_and_waiting_hurt() {
        assert_eq!(kinds("hold up, that's not it", AT, &[]), vec!["corrected", "held up"]);
        assert_eq!(kinds("wait. what are you doing", AT, &[]), vec!["held up"]);
        assert_eq!(kinds("why is the launcher so slow", AT, &[]), vec!["corrected", "waited"]);
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
            "an hour on, it is a new ask"
        );
        let short = vec![(AT - 60_000, "yes".to_string())];
        assert!(kinds("yes", AT, &short).is_empty(), "a yes twice is no pain");
    }

    #[test]
    fn praise_and_plain_asks_hurt_nothing() {
        assert!(kinds("I love this page!", AT, &[]).is_empty());
        assert!(kinds("make it look fucking awesome", AT, &[]).is_empty());
        assert!(kinds("now do the backlight", AT, &[]).is_empty());
        assert!(kinds("waiting room design looks good", AT, &[]).is_empty());
    }
}
