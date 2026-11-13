//! Whether a notice Slack's desktop app gave from a channel Joel set to
//! tell him every message was one of that channel's posts, so it's one of
//! the many he'd only like to know are there, or a reply in a thread he
//! follows, which is to him (Joel, 2026-11-12: "i get major FOMO if I
//! don't know"). Both come as "[<workspace>] in <channel>", so only the
//! channel's history tells them apart: a reply is never one of its
//! top-level messages, and a reply also sent to the channel is said as
//! one, a `thread_broadcast`, but is still a reply. Of the 36 such
//! replies on 2026-11-12, none was among the channel's posts.

use serde_json::Value;

/// How far from when the notice came a post may be and still be the one
/// it told, in seconds: Slack tells a post within a second or two, and a
/// notice read from its queue a little later.
pub const NEAR: f64 = 120.0;

/// How many of a post's first letters and digits are matched against the
/// notice's, as Slack's notice may cut a long one short.
const MATCHED: usize = 40;

/// Whether `history`, Slack's `conversations.history` answer for the
/// channel, holds a top-level post near `at`, in seconds since the epoch,
/// whose words are the notice's `words`, the text after its sender's name.
#[must_use]
pub fn posted(history: &Value, words: &str, at: f64) -> bool {
    let told = letters(&unmarked_notice(words));
    let Some(messages) = history
        .get("messages")
        .and_then(Value::as_array)
        .filter(|_| history.get("ok").and_then(Value::as_bool) == Some(true))
    else {
        return false;
    };
    messages.iter().any(|message| {
        let when = field(message, "ts").parse::<f64>();
        let near = when.is_ok_and(|when| (when - at).abs() <= NEAR);
        let reply = field(message, "subtype") == "thread_broadcast";
        let text = letters(&unmarked_post(field(message, "text")));
        near && !reply && same(&text, &told)
    })
}

/// A message's field as text, or none.
fn field<'a>(message: &'a Value, name: &str) -> &'a str {
    message
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Whether a post's letters are the notice's: all of them when there are
/// few, else its first [`MATCHED`], as the notice may cut it short. A
/// post with none, as a file's, matches only a notice with none.
fn same(post: &str, told: &str) -> bool {
    if told.is_empty() || post.is_empty() {
        return told.is_empty() && post.is_empty();
    }
    let first: String = told.chars().take(MATCHED).collect();
    post.starts_with(&first)
}

/// A post's text without Slack's markup: a mention, `<@U123>`, or one of
/// everyone, `<!here>`, goes, as the notice names them otherwise, and a
/// link, `<url|label>`, is its label, or its address with none.
fn unmarked_post(text: &str) -> String {
    let mut plain = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        plain.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = &rest[open..];
            break;
        };
        let inside = &rest[open + 1..open + close];
        if !inside.starts_with('@') && !inside.starts_with('!') {
            plain.push_str(inside.rsplit_once('|').map_or(inside, |(_, label)| label));
        }
        rest = &rest[open + close + 1..];
    }
    plain.push_str(rest);
    plain
}

/// A notice's words without those it names, `@ada`, as the post holds
/// them by their ids.
fn unmarked_notice(words: &str) -> String {
    words
        .split_whitespace()
        .filter(|word| !word.starts_with('@'))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Only the letters and digits, lowered, so spacing, punctuation and an
/// ellipsis it was cut short with never keep two from matching.
fn letters(text: &str) -> String {
    text.chars()
        .filter(|one| one.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// When the notice came.
    const AT: f64 = 1_794_150_000.0;

    fn history(messages: &Value) -> Value {
        json!({"ok": true, "messages": messages})
    }

    #[test]
    fn a_post_with_the_notices_words_near_its_time_is_a_post() {
        let said = history(&json!([{"ts": "1794149998.000100", "text": "lunch at noon?"}]));
        assert!(posted(&said, "lunch at noon?", AT), "the post was missed");
    }

    #[test]
    fn words_not_among_the_posts_are_a_replys() {
        let said = history(&json!([{"ts": "1794149998.000100", "text": "lunch at noon?"}]));
        let reply = posted(&said, "sure, see you there", AT);
        assert!(!reply, "a reply read as a post");
    }

    #[test]
    fn a_reply_also_sent_to_the_channel_is_still_a_reply() {
        let said = history(&json!([{"ts": "1794149998.000100", "text": "on it",
            "subtype": "thread_broadcast", "thread_ts": "1794140000.000100"}]));
        let reply = posted(&said, "on it", AT);
        assert!(!reply, "a broadcast reply read as a post");
    }

    #[test]
    fn a_post_starting_a_thread_is_still_a_post() {
        let said = history(&json!([{"ts": "1794149998.000100", "text": "deploy?",
            "thread_ts": "1794149998.000100"}]));
        let post = posted(&said, "deploy?", AT);
        assert!(post, "a thread's first post was missed");
    }

    #[test]
    fn the_same_words_long_before_are_not_the_post_it_told() {
        let said = history(&json!([{"ts": "1794149000.000100", "text": "+1"}]));
        assert!(!posted(&said, "+1", AT), "an old post matched");
    }

    #[test]
    fn a_mention_and_a_link_read_as_slack_writes_them() {
        let text = "<@U123> see <https://example.com/a|the doc> now";
        let said = history(&json!([{"ts": "1794149999.000100", "text": text}]));
        let post = posted(&said, "@ada see the doc now", AT);
        assert!(post, "markup kept it from matching");
    }

    #[test]
    fn a_long_post_cut_short_by_the_notice_is_a_post() {
        let text = "the deploy is out to every region and the canary looks clean so far";
        let said = history(&json!([{"ts": "1794149999.000100", "text": text}]));
        let cut = "the deploy is out to every region and the canary…";
        assert!(posted(&said, cut, AT), "a cut-short notice was missed");
    }

    #[test]
    fn a_post_with_no_words_matches_by_its_time_alone() {
        let said = history(&json!([{"ts": "1794149999.000100", "text": ""}]));
        assert!(posted(&said, "", AT), "a file's post was missed");
        let reply = posted(&said, "a reply", AT);
        assert!(!reply, "a reply matched a wordless post");
    }

    #[test]
    fn an_answer_slack_refused_holds_no_post() {
        let refused = posted(&json!({"ok": false}), "lunch", AT);
        assert!(!refused, "a refusal held a post");
    }
}
