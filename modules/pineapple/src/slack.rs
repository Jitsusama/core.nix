//! Slack's answers, read for the safe word: the messages a search found,
//! each with the text Slack keeps (a tag is `<@ID>` there, whatever name
//! it shows), who wrote it and the thread it sits in, and the message a
//! thread starts with, which says what the thread is about. Only reading;
//! the program asks (src/bin/pineapple-slack.rs).

use serde_json::Value;

/// One message a search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The conversation it's in, as `C0123`.
    pub channel: String,
    /// Its timestamp, as Slack names it, `1794581016.836649`.
    pub ts: String,
    /// The thread it's in, by the timestamp of the message starting it,
    /// unless it starts one or is in none.
    pub thread: Option<String>,
    /// Where Slack shows it.
    pub link: String,
    /// Who wrote it, by their id; for a bot, the bot's.
    pub author: String,
    /// Whether a bot or an app wrote it.
    pub by_bot: bool,
    /// What it says, as Slack keeps it.
    pub text: String,
}

impl Message {
    /// The key naming it, `slack:<channel>:<ts>`, which names its record.
    #[must_use]
    pub fn key(&self) -> String {
        format!("slack:{}:{}", self.channel, self.ts)
    }
}

/// A timestamp as microseconds since the epoch, which orders them; none
/// for one that isn't a timestamp.
#[must_use]
pub fn micros(ts: &str) -> Option<u128> {
    let (seconds, fraction) = ts.split_once('.').unwrap_or((ts, ""));
    let seconds: u128 = seconds.parse().ok()?;
    let fraction = format!("{fraction:0<6}");
    let fraction: u128 = fraction.get(..6)?.parse().ok()?;
    Some(seconds * 1_000_000 + fraction)
}

/// The thread a permalink says its message sits in, from its `thread_ts`.
fn thread_of(link: &str) -> Option<String> {
    let (_, query) = link.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("thread_ts="))
        .map(str::to_owned)
}

/// The messages a `search.messages` answer holds, oldest first, or none
/// when Slack refused; the refusal's word, when it gave one, is the error.
///
/// # Errors
///
/// Slack's error, as `invalid_auth` or `ratelimited`.
pub fn found(said: &Value) -> Result<Vec<Message>, String> {
    if said.get("ok").and_then(Value::as_bool) != Some(true) {
        let why = said
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("not ok");
        return Err(why.to_owned());
    }
    let matches = said.pointer("/messages/matches").and_then(Value::as_array);
    let mut messages: Vec<Message> = matches
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default();
            let channel = one.pointer("/channel/id").and_then(Value::as_str)?;
            let ts = text("ts");
            micros(ts)?;
            let link = text("permalink");
            let bot = text("bot_id");
            let by_bot = !bot.is_empty() || one.get("bot_profile").is_some_and(Value::is_object);
            Some(Message {
                channel: channel.to_owned(),
                ts: ts.to_owned(),
                thread: thread_of(link).filter(|thread| thread != ts),
                link: link.to_owned(),
                author: if text("user").is_empty() {
                    bot
                } else {
                    text("user")
                }
                .to_owned(),
                by_bot,
                text: text("text").to_owned(),
            })
        })
        .collect();
    messages.sort_by_key(|message| micros(&message.ts));
    Ok(messages)
}

/// What the message starting a thread says, from a
/// `conversations.replies` answer asked for that thread.
#[must_use]
pub fn root(said: &Value) -> Option<String> {
    said.pointer("/messages/0/text")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_search_gives_each_message_its_thread_and_writer_oldest_first() {
        let said = json!({"ok": true, "messages": {"matches": [
            {"channel": {"id": "C1"}, "ts": "1794574195.000200", "user": "U2",
             "text": "<@U1> pineapple override",
             "permalink": "https://example.slack.com/archives/C1/p1794574195000200?thread_ts=1794574128.837949&cid=C1"},
            {"channel": {"id": "C1"}, "ts": "1794574128.837949", "bot_id": "B1",
             "bot_profile": {"name": "github"}, "text": "a PR",
             "permalink": "https://example.slack.com/archives/C1/p1794574128837949?thread_ts=1794574128.837949"}
        ]}});
        let found = found(&said).expect("an ok answer is read");
        assert_eq!(found.len(), 2, "both matches are read");
        assert_eq!(found[0].ts, "1794574128.837949", "the older comes first");
        assert!(found[0].by_bot, "a bot_id marks a bot");
        assert_eq!(found[0].author, "B1", "a bot is named by its id");
        assert_eq!(found[0].thread, None, "a thread's first message is in none");
        assert_eq!(
            found[1].thread.as_deref(),
            Some("1794574128.837949"),
            "a reply names its thread"
        );
        assert_eq!(found[1].author, "U2", "a person is named by their id");
        assert_eq!(
            found[1].key(),
            "slack:C1:1794574195.000200",
            "the key names it"
        );
    }

    #[test]
    fn a_refusal_says_why() {
        let refused = found(&json!({"ok": false, "error": "ratelimited"}));
        assert_eq!(refused, Err("ratelimited".to_owned()), "the word is said");
    }

    #[test]
    fn timestamps_order_by_their_whole_value() {
        assert!(
            micros("1794574128.9") > micros("1794574128.837949"),
            "a short fraction is tenths"
        );
        assert_eq!(micros("nope"), None, "a word isn't a timestamp");
    }

    #[test]
    fn a_thread_root_is_its_first_message() {
        let said = json!({"ok": true, "messages": [{"text": "review shop/world#12"}]});
        assert_eq!(
            root(&said).as_deref(),
            Some("review shop/world#12"),
            "its text"
        );
    }
}
