//! Search Slack, for the launcher's "Search Slack for ‹words›" (VKCF3Z,
//! d7c0f933): one ask of `search.messages` for what Joel typed, each match
//! a Slack message, and for one of them the few messages around it, read
//! with `conversations.history`. Both answer the browser's sign-in on a
//! grid. Only ever read.

use serde::Serialize;
use serde_json::Value;

use crate::{Account, FIRST_WORDS, LONGEST, Refused, Slack, cut, millis, plain};

/// How many matches a search says.
pub const MATCHES: &str = "20";

/// How many messages either side of one its drawer shows.
pub const AROUND: &str = "3";

/// What an HTTP 429 from Slack is said as, so a search can tell Slack too
/// busy from Slack out of reach.
pub const BUSY: &str = "Slack answered 429";

/// One message found, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Found {
    /// `slack-message:<channel>:<ts>`, which names it to ask what's around.
    pub key: String,
    pub kind: &'static str,
    /// The conversation it's in: `#name` for a channel, its members for a
    /// group, and for a direct message the other person's handle.
    pub place: String,
    /// Who said it, by their handle.
    pub who: String,
    /// Whether Joel said it.
    pub mine: bool,
    /// Its first words, which title it.
    pub first: String,
    /// It all, in plain words, as far as the drawer shows.
    pub text: String,
    /// In milliseconds since the epoch.
    pub at: u64,
    /// Where Slack opens it.
    pub link: String,
    /// For a direct message, the other person's id, by which the search
    /// names its place.
    #[serde(skip)]
    pub with: String,
}

/// One message around another, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Near {
    pub mine: bool,
    pub text: String,
    pub at: u64,
    /// Whether it's the one asked about.
    pub asked: bool,
}

/// Why a search said nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failed {
    SignedOut,
    /// Slack's limit on searches was reached; asking again in a minute works.
    Busy,
    Unreachable(String),
}

impl From<Refused> for Failed {
    fn from(refused: Refused) -> Failed {
        match refused {
            Refused::SignedOut => Failed::SignedOut,
            Refused::Unreachable(why) if why == BUSY => Failed::Busy,
            Refused::Unreachable(why) => Failed::Unreachable(why),
        }
    }
}

/// The key naming a message.
pub fn key(channel: &str, ts: &str) -> String {
    format!("slack-message:{channel}:{ts}")
}

/// The channel and timestamp a key names.
pub fn named(key: &str) -> Option<(&str, &str)> {
    let (channel, ts) = key.strip_prefix("slack-message:")?.split_once(':')?;
    (!channel.is_empty() && millis(ts).is_some()).then_some((channel, ts))
}

/// Slack's answer, unless it refused.
fn answered(said: &Value) -> Result<&Value, Failed> {
    if said.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(said);
    }
    match said.get("error").and_then(Value::as_str).unwrap_or("") {
        "invalid_auth" | "not_authed" | "account_inactive" | "token_revoked" | "token_expired" => {
            Err(Failed::SignedOut)
        }
        "ratelimited" => Err(Failed::Busy),
        other => Err(Failed::Unreachable(format!("Slack refused: {other}"))),
    }
}

/// The messages a search found, in Slack's order, best first. `me` is
/// Joel's own id.
///
/// # Errors
///
/// Why Slack said nothing.
pub fn found(said: &Value, me: &str) -> Result<Vec<Found>, Failed> {
    answered(said)?;
    let matches = said.pointer("/messages/matches").and_then(Value::as_array);
    Ok(matches
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default();
            let channel = one.get("channel").unwrap_or(&Value::Null);
            let id = channel.get("id").and_then(Value::as_str)?;
            let ts = text("ts");
            let at = millis(ts)?;
            let words = plain(text("text"), &[]);
            if words.is_empty() {
                return None;
            }
            let flag = |key: &str| channel.get(key).and_then(Value::as_bool) == Some(true);
            let name = channel
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // A direct message is named by the other person, whose id is its
            // name, once the search asks who that is.
            let place = if flag("is_im") {
                String::new()
            } else if flag("is_mpim") {
                crate::unreads::group(name)
            } else {
                format!("#{name}")
            };
            Some(Found {
                key: key(id, ts),
                kind: "slack-message",
                place,
                who: text("username").to_owned(),
                mine: !me.is_empty() && text("user") == me,
                first: cut(words.lines().next().unwrap_or_default(), FIRST_WORDS),
                text: cut(&words, LONGEST),
                at,
                link: text("permalink").to_owned(),
                with: if flag("is_im") {
                    name.to_owned()
                } else {
                    String::new()
                },
            })
        })
        .collect())
}

/// A person's name in Slack from `users.info`, their handle when their
/// profile names nobody, or Slack's own word when it says nothing.
fn named_in(user: &Value) -> String {
    let said = |at: &str| {
        user.pointer(at)
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
    };
    said("/user/profile/real_name")
        .or_else(|| said("/user/real_name"))
        .or_else(|| said("/user/name"))
        .unwrap_or("Slack")
        .to_owned()
}

/// The messages of a conversation's history, newest first as Slack gives
/// them, in plain words, marking the one at `ts`.
fn history(said: &Value, ts: &str, me: &str) -> Result<Vec<Near>, Failed> {
    answered(said)?;
    let messages = said.get("messages").and_then(Value::as_array);
    Ok(messages
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default();
            let at = millis(text("ts"))?;
            let words = plain(text("text"), &[]);
            (!words.is_empty()).then(|| Near {
                mine: !me.is_empty() && text("user") == me,
                text: cut(&words, LONGEST),
                at,
                asked: text("ts") == ts,
            })
        })
        .collect())
}

/// The messages around one, oldest first, from the history up to it and
/// the history after it, each newest first, the one asked about once.
///
/// # Errors
///
/// Why Slack said nothing.
pub fn around(before: &Value, after: &Value, ts: &str, me: &str) -> Result<Vec<Near>, Failed> {
    let mut near = history(before, ts, me)?;
    near.reverse();
    let mut later = history(after, ts, me)?;
    later.reverse();
    near.extend(later.into_iter().filter(|one| !one.asked));
    Ok(near)
}

impl Slack {
    /// What Joel's words find in Slack.
    ///
    /// # Errors
    ///
    /// Why Slack said nothing.
    pub fn search(&self, account: &Account, words: &str) -> Result<Vec<Found>, Failed> {
        let said = self.ask(
            account,
            "search.messages",
            &[("query", words), ("count", MATCHES), ("highlight", "false")],
        )?;
        let mut found = found(&said, &account.user)?;
        // Each direct message's other person, asked once a search.
        let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for one in &mut found {
            if one.with.is_empty() {
                continue;
            }
            if !names.contains_key(&one.with) {
                let user = self.ask(account, "users.info", &[("user", &one.with)])?;
                names.insert(one.with.clone(), named_in(&user));
            }
            one.place = names[&one.with].clone();
        }
        Ok(found)
    }

    /// The few messages around one a search found.
    ///
    /// # Errors
    ///
    /// Why Slack said nothing.
    pub fn around(&self, account: &Account, channel: &str, ts: &str) -> Result<Vec<Near>, Failed> {
        let up_to = [
            ("channel", channel),
            ("latest", ts),
            ("inclusive", "true"),
            ("limit", AROUND),
        ];
        let before = self.ask(account, "conversations.history", &up_to)?;
        let since = [("channel", channel), ("oldest", ts), ("limit", AROUND)];
        let after = self.ask(account, "conversations.history", &since)?;
        around(&before, &after, ts, &account.user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_search_says_each_message_by_where_it_is_who_said_it_and_its_first_words() {
        let said = json!({"ok": true, "messages": {"matches": [
            {"channel": {"id": "C1", "is_channel": true, "name": "launcher"},
             "ts": "1730300000.000100", "user": "U9", "username": "ada",
             "text": "The launcher &amp; its rows\nmore", "permalink": "https://slack/p1"},
            {"channel": {"id": "D2", "is_im": true, "name": "U7"},
             "ts": "1730300001.000100", "user": "U0", "username": "joel",
             "text": "mine", "permalink": "https://slack/p2"},
            {"channel": {"id": "G3", "is_mpim": true, "name": "mpdm-ada--joel-1"},
             "ts": "1730300002.000100", "user": "U9", "username": "ada", "text": ""}
        ]}});
        let found = found(&said, "U0").unwrap();
        assert_eq!(found.len(), 2, "one with no words is left out");
        assert_eq!(
            found[0],
            Found {
                key: "slack-message:C1:1730300000.000100".to_owned(),
                kind: "slack-message",
                place: "#launcher".to_owned(),
                who: "ada".to_owned(),
                mine: false,
                first: "The launcher & its rows".to_owned(),
                text: "The launcher & its rows\nmore".to_owned(),
                at: 1_730_300_000_000,
                link: "https://slack/p1".to_owned(),
                with: String::new(),
            }
        );
        // A direct message is named by its other person once asked who.
        assert_eq!(
            (
                found[1].place.as_str(),
                found[1].with.as_str(),
                found[1].mine
            ),
            ("", "U7", true)
        );
        let ada = json!({"ok": true, "user": {"name": "ada", "profile": {"real_name": "Ada L"}}});
        assert_eq!(named_in(&ada), "Ada L");
        assert_eq!(named_in(&json!({"ok": false})), "Slack");
    }

    #[test]
    fn slack_too_busy_is_told_apart_from_signed_out_and_out_of_reach() {
        let refused = |error: &str| found(&json!({"ok": false, "error": error}), "U0");
        assert_eq!(refused("ratelimited"), Err(Failed::Busy));
        assert_eq!(refused("invalid_auth"), Err(Failed::SignedOut));
        assert!(matches!(
            refused("fatal_error"),
            Err(Failed::Unreachable(_))
        ));
        assert_eq!(
            Failed::from(Refused::Unreachable(BUSY.to_owned())),
            Failed::Busy
        );
    }

    #[test]
    fn the_messages_around_one_are_oldest_first_with_it_once() {
        let message =
            |ts: &str, user: &str, text: &str| json!({"ts": ts, "user": user, "text": text});
        let before = json!({"ok": true, "messages": [
            message("1730300002.000000", "U9", "asked"),
            message("1730300001.000000", "U0", "before")
        ]});
        let after = json!({"ok": true, "messages": [
            message("1730300003.000000", "U9", "after"),
            message("1730300002.000000", "U9", "asked")
        ]});
        let near = around(&before, &after, "1730300002.000000", "U0").unwrap();
        let said: Vec<(&str, bool, bool)> = near
            .iter()
            .map(|one| (one.text.as_str(), one.mine, one.asked))
            .collect();
        assert_eq!(
            said,
            [
                ("before", true, false),
                ("asked", false, true),
                ("after", false, false)
            ]
        );
    }

    #[test]
    fn a_key_names_its_channel_and_time_and_nothing_else() {
        assert_eq!(
            named(&key("C1", "1730300000.000100")),
            Some(("C1", "1730300000.000100"))
        );
        assert_eq!(named("slack-message:C1:soon"), None);
        assert_eq!(named("person:ada@example.com"), None);
    }
}
