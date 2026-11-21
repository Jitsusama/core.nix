//! Slack's answers as a tool gateway gives them, read for the safe word.
//!
//! A gateway answers in prose meant for a model rather than in Slack's
//! JSON: a search, asked for its detailed form, is a list of results,
//! each a few `Name: value` lines (the conversation's id, the writer's,
//! the message's timestamp and permalink) and then its text, which runs
//! to the result's closing rule and keeps its blank lines; a thread is its
//! first message, then its replies. These read both into the same
//! [`Message`] Slack's own answers make (src/slack.rs), so the reader
//! judges a message the same whichever way it was heard.
//!
//! Two things Slack's JSON says a gateway's prose doesn't: whether a bot
//! wrote the message, which is read here from a bot's id (`B…`) and so
//! misses an app writing as a user, and whether it was edited, which
//! reads as never. Both are said where the reader judges.

use crate::slack::{Message, micros};

/// What opens each result of a search.
const RESULT: &str = "\n### Result ";

/// What closes each result, before the next one opens.
const RULE: &str = "\n---";

/// What opens a thread's first message, and what follows it.
const PARENT: &str = "=== THREAD PARENT MESSAGE ===";
const REPLIES: &str = "=== THREAD REPLIES";

/// The messages a detailed search's results hold, oldest first; one
/// missing its conversation, timestamp or writer is left out, as no
/// record could name it.
#[must_use]
pub fn found(results: &str) -> Vec<Message> {
    let mut found: Vec<Message> = results.split(RESULT).skip(1).filter_map(message).collect();
    found.sort_by_key(|message| micros(&message.ts));
    found
}

/// One result, read: its fields, then its text.
fn message(result: &str) -> Option<Message> {
    let (fields, text) = result.split_once("\nText:")?;
    let field = |name: &str| {
        fields
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .map(str::trim)
    };
    let channel = field("Channel:").and_then(id_of)?;
    let author = field("From:").and_then(id_of)?;
    let ts = field("Message_ts:")?.to_owned();
    let link = field("Permalink:").map_or_else(String::new, link_of);
    let text = text.split(RULE).next().unwrap_or_default();
    Some(Message {
        thread: thread_of(&link).filter(|thread| *thread != ts),
        by_bot: author.starts_with('B'),
        edited: false,
        channel,
        author,
        ts,
        link,
        text: text.trim().to_owned(),
    })
}

/// The id a field ends with, as `#name (ID: C0123)` or a writer's
/// `Name <mail> (ID: U0123)` give it, or `(U0123)` as a thread does.
fn id_of(value: &str) -> Option<String> {
    let open = value.rfind('(')?;
    let inside = value[open + 1..].strip_suffix(')')?;
    let id = inside.strip_prefix("ID:").unwrap_or(inside).trim();
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())).then(|| id.to_owned())
}

/// The address in a permalink's markdown, `[link](https://…)`, or the
/// field as it stands when it isn't markdown.
fn link_of(value: &str) -> String {
    value
        .split_once("](")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or(value)
        .to_owned()
}

/// The thread a permalink says its message sits in, from its `thread_ts`.
fn thread_of(link: &str) -> Option<String> {
    let (_, query) = link.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("thread_ts="))
        .map(str::to_owned)
}

/// What the message starting a thread says, from a detailed reading of
/// that thread: the lines after its `From`, `Time` and `Message TS`, up to
/// its reactions or files, or its replies.
#[must_use]
pub fn root(thread: &str) -> Option<String> {
    let (_, parent) = thread.split_once(PARENT)?;
    let parent = parent.split(REPLIES).next().unwrap_or_default();
    let said: Vec<&str> = parent
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .skip_while(|line| {
            ["From:", "Time:", "Message TS:"]
                .iter()
                .any(|field| line.starts_with(field))
        })
        .take_while(|line| !line.starts_with("Reactions:") && !line.starts_with("Files:"))
        .collect();
    let said = said.join("\n");
    let said = said.trim();
    (!said.is_empty()).then(|| said.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two results as a gateway gives them: a reply naming its thread, and
    /// the message starting it, written by a bot, over two paragraphs.
    const SEARCH: &str = "# Search Results for: \n\n## Messages (2 results)\n\
        ### Result 1 of 2\n\
        Channel: #team (ID: C1)\n\
        From: Someone Else <someone@example.com> (ID: U2) \n\
        Time: 2026-11-13 09:49:55 EST\n\
        Message_ts: 1794581395.231099\n\
        Permalink: [link](https://example.slack.com/archives/C1/p1794581395231099\
        ?thread_ts=1794574128.837949&cid=C1)\n\
        Text: \n<@U1|Joel> pineapple override\n\n---\n\n\
        ### Result 2 of 2\n\
        Channel: #team (ID: C1)\n\
        From: A Bot (ID: B1) \n\
        Time: 2026-11-13 07:48:48 EST\n\
        Message_ts: 1794574128.837949\n\
        Reply count: 5\n\
        Permalink: [link](https://example.slack.com/archives/C1/p1794574128837949\
        ?thread_ts=1794574128.837949&cid=C1)\n\
        Text: \nreview shop/world#12\n\nand merge it\n\n---\n\n";

    #[test]
    fn a_search_gives_each_message_its_thread_and_writer_oldest_first() {
        let found = found(SEARCH);
        assert_eq!(found.len(), 2, "both results are read");
        assert_eq!(found[0].ts, "1794574128.837949", "the older comes first");
        assert!(found[0].by_bot, "a bot's id marks a bot");
        assert_eq!(found[0].thread, None, "a thread's first message is in none");
        assert_eq!(
            found[0].text, "review shop/world#12\n\nand merge it",
            "a text keeps its paragraphs and stops at the rule"
        );
        assert_eq!(found[1].author, "U2", "a person is named by their id");
        assert!(!found[1].by_bot, "a person's id is no bot");
        assert_eq!(found[1].channel, "C1", "the conversation is its id");
        assert_eq!(
            found[1].thread.as_deref(),
            Some("1794574128.837949"),
            "a reply names its thread"
        );
        assert_eq!(
            found[1].link,
            "https://example.slack.com/archives/C1/p1794581395231099\
             ?thread_ts=1794574128.837949&cid=C1",
            "the permalink is its address"
        );
        assert_eq!(
            found[1].text, "<@U1|Joel> pineapple override",
            "a tag keeps its id"
        );
        assert_eq!(
            found[1].key(),
            "slack:C1:1794581395.231099",
            "the key names it as Slack's own answer would"
        );
        assert!(!found[1].edited, "an edit can't be seen, so reads as none");
    }

    #[test]
    fn a_result_without_its_ids_is_left_out() {
        let nameless = "# Search Results\n### Result 1 of 1\nChannel: #team\n\
            From: Someone (ID: U2)\nMessage_ts: 1794581395.231099\nText: \nhi\n\n---\n";
        assert_eq!(found(nameless), Vec::new(), "a conversation with no id");
        assert_eq!(
            found("# Search Results for: \n\n## Messages (0 results)\n"),
            Vec::new(),
            "no results"
        );
    }

    #[test]
    fn a_thread_root_is_its_first_message_without_its_reactions() {
        let thread = "=== THREAD PARENT MESSAGE ===\n\
            From: Someone <someone@example.com> (W0123)\n\
            Time: 2026-11-13 07:48:48 EST\n\
            Message TS: 1794574128.837949\n\
            review shop/world#12\nplease\n\
            Reactions: eyes (1)\n\n\
            === THREAD REPLIES (1 total) ===\n\n--- Reply 1 of 1 ---\n\
            From: Joel <joel@example.com> (U1)\nMessage TS: 1794612122.818429\nsure\n";
        assert_eq!(
            root(thread).as_deref(),
            Some("review shop/world#12\nplease"),
            "its text, not its reactions or replies"
        );
        assert_eq!(
            root("nothing here"),
            None,
            "a reading with no first message"
        );
    }

    #[test]
    fn an_id_is_read_from_either_form() {
        assert_eq!(id_of("#team (ID: C1)").as_deref(), Some("C1"), "a field's");
        assert_eq!(
            id_of("Someone (W0123)").as_deref(),
            Some("W0123"),
            "a thread's"
        );
        assert_eq!(id_of("#team"), None, "none given");
        assert_eq!(id_of("odd (ID: C1; x)"), None, "not an id");
    }
}
