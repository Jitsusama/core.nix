//! What in Slack wants Joel, for the shell's Now (15J7N3's piece reads it):
//! each direct message with unreads, by how many; each channel or group
//! where he's mentioned, by how many times; and his mentions in threads,
//! as one. A channel with unreads but no mention of him doesn't want him,
//! so it isn't said.
//!
//! It's read from `client.counts`, which is what Slack's own client asks
//! and what a grid answers to the browser's sign-in the harness keeps,
//! where the documented `users.counts` is refused. Each conversation is
//! named by `conversations.info`, a direct message by whose it is through
//! `users.info`, asked once a run. It only ever reads.

use serde::Serialize;
use serde_json::Value;

use crate::{Account, Refused, Slack};

/// What the shell is told, as one JSON document: when it was last asked,
/// how asking went, and what wants Joel, only while asking went well, so
/// an old count is never shown as a current one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Said {
    /// When Slack last answered, or was last asked when it didn't, in
    /// milliseconds since the epoch.
    pub checked: u64,
    pub state: State,
    pub conversations: Vec<Unread>,
}

/// How asking Slack went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Ok,
    /// Slack refused the sign-in: signing in again from pi mends it.
    SignedOut,
    /// Slack couldn't be reached for a while.
    Unreachable,
}

/// One conversation that wants Joel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unread {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub count: u64,
    /// The `slack://` link that opens it in Slack.
    pub link: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// A direct message, one person's or a group's, counted by its
    /// unread messages.
    Dm,
    /// A channel, counted by its mentions of him.
    Mention,
    /// His mentions in threads, all together.
    Thread,
}

/// A conversation `client.counts` says wants Joel, not yet named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    pub id: String,
    pub kind: Kind,
    pub count: u64,
}

/// What wants Joel in an answer of `client.counts`, direct messages first,
/// then channels, then threads, each newest first by its latest message
/// (VKCF3Z, f0c8ef61). Slack's `latest` is a timestamp of fixed width,
/// seconds and microseconds, so it sorts as text.
///
/// # Errors
///
/// Slack's refusal, as the sign-in being refused or Slack being out of
/// reach.
pub fn counted(said: &Value) -> Result<Vec<Counted>, Refused> {
    answered(said)?;
    let listed = |key: &str| {
        said.get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
    };
    // A direct message's unreads are all his, and come as its mentions.
    let mut wanting = newest(Kind::Dm, listed("ims").chain(listed("mpims")));
    wanting.extend(newest(Kind::Mention, listed("channels")));
    let threads = said
        .pointer("/threads/mention_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if threads > 0 {
        wanting.push(Counted {
            id: "threads".to_owned(),
            kind: Kind::Thread,
            count: threads,
        });
    }
    Ok(wanting)
}

/// Those of one kind that mention Joel, newest first.
fn newest<'a>(kind: Kind, listed: impl Iterator<Item = &'a Value>) -> Vec<Counted> {
    let count = |each: &Value| {
        each.get("mention_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let mut found: Vec<(&str, Counted)> = listed
        .filter(|each| count(each) > 0)
        .filter_map(|each| {
            let id = each.get("id").and_then(Value::as_str)?;
            let latest = each.get("latest").and_then(Value::as_str).unwrap_or("");
            Some((
                latest,
                Counted {
                    id: id.to_owned(),
                    kind,
                    count: count(each),
                },
            ))
        })
        .collect();
    found.sort_by(|(one, _), (other, _)| other.cmp(one));
    found.into_iter().map(|(_, counted)| counted).collect()
}

/// What a conversation is called, from `conversations.info`: a channel's
/// name, a group's members' handles, or, for one person's direct message,
/// whose it is, to be named by `users.info`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Called {
    Named(String),
    Someone(String),
}

/// What `conversations.info` says a conversation is called.
///
/// # Errors
///
/// Slack's refusal, as the sign-in being refused or Slack being out of
/// reach.
pub fn called(said: &Value) -> Result<Called, Refused> {
    answered(said)?;
    let channel = said.get("channel").unwrap_or(&Value::Null);
    let text = |key: &str| channel.get(key).and_then(Value::as_str).unwrap_or("");
    if channel.get("is_im").and_then(Value::as_bool) == Some(true) {
        return Ok(Called::Someone(text("user").to_owned()));
    }
    if channel.get("is_mpim").and_then(Value::as_bool) == Some(true) {
        return Ok(Called::Named(group(text("name"))));
    }
    Ok(Called::Named(format!("#{}", text("name"))))
}

/// A group's name as its members' handles: Slack names one
/// `mpdm-ada--grace--joel-1`.
fn group(name: &str) -> String {
    let inside = name.strip_prefix("mpdm-").unwrap_or(name);
    let inside = inside
        .rsplit_once('-')
        .filter(|(_, last)| last.chars().all(|c| c.is_ascii_digit()))
        .map_or(inside, |(members, _)| members);
    inside.split("--").collect::<Vec<_>>().join(", ")
}

/// The link opening a conversation in Slack, or for threads, the
/// workspace, since no link opens Slack's threads view.
pub fn link(team: &str, counted: &Counted) -> String {
    match counted.kind {
        Kind::Thread => format!("slack://open?team={team}"),
        _ => format!("slack://channel?team={team}&id={}", counted.id),
    }
}

/// Slack's answer, unless it refused.
fn answered(said: &Value) -> Result<&Value, Refused> {
    if said.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(said);
    }
    match said.get("error").and_then(Value::as_str).unwrap_or("") {
        "invalid_auth" | "not_authed" | "account_inactive" | "token_revoked" | "token_expired" => {
            Err(Refused::SignedOut)
        }
        other => Err(Refused::Unreachable(format!("Slack refused: {other}"))),
    }
}

impl Slack {
    /// What wants Joel now, as `client.counts` says.
    ///
    /// # Errors
    ///
    /// Slack's refusal, as the sign-in being refused or Slack being out of
    /// reach.
    pub fn counts(&self, account: &Account) -> Result<Vec<Counted>, Refused> {
        let said = self.ask(
            account,
            "client.counts",
            &[("thread_counts_by_channel", "true")],
        )?;
        counted(&said)
    }

    /// What a conversation is called, a person's direct message by their
    /// name in Slack.
    ///
    /// # Errors
    ///
    /// Slack's refusal, as the sign-in being refused or Slack being out of
    /// reach.
    pub fn called(&self, account: &Account, id: &str) -> Result<String, Refused> {
        let said = self.ask(account, "conversations.info", &[("channel", id)])?;
        match called(&said)? {
            Called::Named(name) => Ok(name),
            Called::Someone(user) => {
                let said = self.ask(account, "users.info", &[("user", &user)])?;
                answered(&said)?;
                let profile = |key: &str| {
                    said.pointer(&format!("/user/{key}"))
                        .and_then(Value::as_str)
                        .filter(|name| !name.is_empty())
                };
                Ok(profile("profile/real_name")
                    .or_else(|| profile("real_name"))
                    .or_else(|| profile("name"))
                    .unwrap_or_default()
                    .to_owned())
            }
        }
    }
}

/// Whether what's said is worth writing again: it changed, or the last
/// write is old enough that the shell would take it for stale.
pub fn due(written: Option<&Said>, now: &Said, every: u64) -> bool {
    written.is_none_or(|written| {
        written.state != now.state
            || written.conversations != now.conversations
            || now.checked.saturating_sub(written.checked) >= every
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn what_wants_joel_is_unread_messages_and_mentions_newest_first_not_unread_channels() {
        let said = json!({
            "ok": true,
            "ims": [
                {"id": "D1", "has_unreads": true, "mention_count": 2,
                 "latest": "1730300000.000100"},
                {"id": "D2", "has_unreads": false, "mention_count": 0}
            ],
            "mpims": [{"id": "G1", "has_unreads": true, "mention_count": 1,
                        "latest": "1730400000.000100"}],
            "channels": [
                {"id": "C1", "has_unreads": true, "mention_count": 0},
                {"id": "C2", "has_unreads": true, "mention_count": 3,
                 "latest": "1730200000.000100"},
                {"id": "C3", "has_unreads": true, "mention_count": 1,
                 "latest": "1730500000.000100"}
            ],
            "threads": {"has_unreads": true, "mention_count": 4}
        });
        let wanting = counted(&said).unwrap();
        let shown: Vec<(&str, Kind, u64)> = wanting
            .iter()
            .map(|each| (each.id.as_str(), each.kind, each.count))
            .collect();
        assert_eq!(
            shown,
            [
                ("G1", Kind::Dm, 1),
                ("D1", Kind::Dm, 2),
                ("C3", Kind::Mention, 1),
                ("C2", Kind::Mention, 3),
                ("threads", Kind::Thread, 4)
            ]
        );
    }

    #[test]
    fn a_refused_sign_in_is_signed_out_and_anything_else_unreachable() {
        assert_eq!(
            counted(&json!({"ok": false, "error": "invalid_auth"})),
            Err(Refused::SignedOut)
        );
        assert!(matches!(
            counted(&json!({"ok": false, "error": "ratelimited"})),
            Err(Refused::Unreachable(_))
        ));
    }

    #[test]
    fn each_is_called_by_its_channel_its_group_or_whose_it_is() {
        let info = |channel: Value| json!({"ok": true, "channel": channel});
        assert_eq!(
            called(&info(json!({"is_channel": true, "name": "launcher"}))),
            Ok(Called::Named("#launcher".to_owned()))
        );
        assert_eq!(
            called(&info(
                json!({"is_mpim": true, "name": "mpdm-ada--grace--joel-1"})
            )),
            Ok(Called::Named("ada, grace, joel".to_owned()))
        );
        assert_eq!(
            called(&info(json!({"is_im": true, "user": "U7"}))),
            Ok(Called::Someone("U7".to_owned()))
        );
    }

    #[test]
    fn a_conversation_opens_in_slack_and_threads_open_the_workspace() {
        let dm = Counted {
            id: "D1".to_owned(),
            kind: Kind::Dm,
            count: 1,
        };
        let threads = Counted {
            id: "threads".to_owned(),
            kind: Kind::Thread,
            count: 1,
        };
        assert_eq!(link("T1", &dm), "slack://channel?team=T1&id=D1");
        assert_eq!(link("T1", &threads), "slack://open?team=T1");
    }

    #[test]
    fn said_as_the_shell_reads_it() {
        let said = Said {
            checked: 5,
            state: State::SignedOut,
            conversations: vec![Unread {
                id: "D1".to_owned(),
                name: "Ada".to_owned(),
                kind: Kind::Dm,
                count: 2,
                link: "slack://channel?team=T1&id=D1".to_owned(),
            }],
        };
        assert_eq!(
            serde_json::to_value(&said).unwrap(),
            json!({"checked": 5, "state": "signedOut", "conversations": [{
                "id": "D1", "name": "Ada", "kind": "dm", "count": 2,
                "link": "slack://channel?team=T1&id=D1"
            }]})
        );
    }

    #[test]
    fn written_again_only_once_it_changes_or_goes_old() {
        let at = |checked: u64, count: u64| Said {
            checked,
            state: State::Ok,
            conversations: vec![Unread {
                id: "D1".to_owned(),
                name: "Ada".to_owned(),
                kind: Kind::Dm,
                count,
                link: String::new(),
            }],
        };
        assert!(due(None, &at(0, 1), 300_000));
        assert!(!due(Some(&at(0, 1)), &at(30_000, 1), 300_000));
        assert!(due(Some(&at(0, 1)), &at(30_000, 2), 300_000));
        assert!(due(Some(&at(0, 1)), &at(300_000, 1), 300_000));
    }
}
