//! What in Slack is unread, for the shell's Now and Slack's own Unread:
//! each direct message with unreads, by how many; each channel or group
//! where he's mentioned, by how many times; his mentions in threads, as
//! one; and after them each channel merely unread, with no mention of
//! him. The first three want Joel, so Now says them (15J7N3's piece); a
//! channel merely unread doesn't, so only Slack's Unread lists it (VKCF3Z,
//! d7c0f933). Slack gives no count of a channel's unreads without a
//! mention, so it's said with none. Beside them, Slack's Recent: the nine
//! conversations he read last.
//!
//! It's read from `client.counts`, which is what Slack's own client asks
//! and what a grid answers to the browser's sign-in the harness keeps,
//! where the documented `users.counts` is refused. Each conversation is
//! named by `conversations.info`, a direct message by whose it is through
//! `users.info`, asked once a run. It only ever reads.

use serde::Serialize;
use serde_json::{Value, json};

use crate::{Account, Refused, Slack};

/// What the shell is told, as one JSON document: when it was last asked,
/// how asking went, what's unread and what he read last, only while
/// asking went well, so an old count is never shown as a current one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Said {
    /// When Slack last answered, or was last asked when it didn't, in
    /// milliseconds since the epoch.
    pub checked: u64,
    pub state: State,
    /// What's unread, those wanting Joel first.
    pub conversations: Vec<Conversation>,
    /// What he read last, newest first, at most `RECENT`.
    pub recent: Vec<Conversation>,
}

/// How many conversations Slack's Recent holds, as every program's does,
/// so 1 to 9 pick one.
pub const RECENT: usize = 9;

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

/// One conversation, as the shell lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Conversation {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub count: u64,
    /// The `slack://` link that opens it in Slack.
    pub link: String,
    /// Slack's timestamp it's ordered by, as `Counted`'s.
    #[serde(skip)]
    pub at: String,
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
    /// A channel with unreads but no mention of him, with no count, since
    /// Slack gives none.
    Unread,
    /// A channel he read lately, as Recent lists it.
    Channel,
}

impl Kind {
    /// Whether it wants Joel, as Now lists only those that do.
    pub fn wants(self) -> bool {
        matches!(self, Kind::Dm | Kind::Mention | Kind::Thread)
    }
}

/// A conversation `client.counts` names, not yet named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    pub id: String,
    pub kind: Kind,
    pub count: u64,
    /// Slack's timestamp it's ordered by: an unread one's latest
    /// message, or when one in Recent was read last; none for threads.
    pub at: String,
}

/// What's unread and what he read last, as `client.counts` says them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    pub unread: Vec<Counted>,
    pub recent: Vec<Counted>,
}

/// What's unread in an answer of `client.counts`: direct messages first,
/// then channels mentioning him, then threads, then channels merely
/// unread, each newest first by its latest message (VKCF3Z, f0c8ef61 and
/// d7c0f933); and the conversations he read last, newest first. Slack's
/// `latest` and `last_read` are timestamps of fixed width, seconds and
/// microseconds, so they sort as text.
///
/// # Errors
///
/// Slack's refusal, as the sign-in being refused or Slack being out of
/// reach.
pub fn counted(said: &Value) -> Result<Counts, Refused> {
    answered(said)?;
    let mentions = |each: &Value| {
        each.get("mention_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let unread = |each: &Value| each.get("has_unreads").and_then(Value::as_bool) == Some(true);
    // A direct message's unreads are all his, and come as its mentions.
    let mut wanting = newest(
        Kind::Dm,
        listed(said, "ims")
            .chain(listed(said, "mpims"))
            .map(|each| (each, mentions(each))),
    );
    wanting.extend(newest(
        Kind::Mention,
        listed(said, "channels").map(|each| (each, mentions(each))),
    ));
    let threads = said
        .pointer("/threads/mention_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if threads > 0 {
        wanting.push(Counted {
            id: "threads".to_owned(),
            kind: Kind::Thread,
            count: threads,
            at: String::new(),
        });
    }
    let mut merely: Vec<(&str, Counted)> = listed(said, "channels")
        .filter(|each| unread(each) && mentions(each) == 0)
        .filter_map(|each| {
            let at = stamp(each, "latest");
            Some((at, named(each, Kind::Unread, 0, at)?))
        })
        .collect();
    merely.sort_by(|(one, _), (other, _)| other.cmp(one));
    wanting.extend(merely.into_iter().map(|(_, counted)| counted));
    Ok(Counts {
        unread: wanting,
        recent: lately(said),
    })
}

/// The conversations of one of the answer's lists.
fn listed<'a>(said: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    said.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// Those of one kind that mention Joel, each with its count of mentions,
/// newest first.
fn newest<'a>(kind: Kind, listed: impl Iterator<Item = (&'a Value, u64)>) -> Vec<Counted> {
    let mut found: Vec<(&str, Counted)> = listed
        .filter(|(_, count)| *count > 0)
        .filter_map(|(each, count)| {
            let at = stamp(each, "latest");
            Some((at, named(each, kind, count, at)?))
        })
        .collect();
    found.sort_by(|(one, _), (other, _)| other.cmp(one));
    found.into_iter().map(|(_, counted)| counted).collect()
}

/// The conversations he read last, newest first, at most `RECENT`: one
/// never read, whose `last_read` is all noughts, isn't among them.
fn lately(said: &Value) -> Vec<Counted> {
    let mut read: Vec<(&str, Counted)> = listed(said, "ims")
        .chain(listed(said, "mpims"))
        .map(|each| (each, Kind::Dm))
        .chain(listed(said, "channels").map(|each| (each, Kind::Channel)))
        .filter_map(|(each, kind)| {
            let at = stamp(each, "last_read");
            Some((at, named(each, kind, 0, at)?))
        })
        .filter(|(when, _)| when.chars().any(|c| c.is_ascii_digit() && c != '0'))
        .collect();
    read.sort_by(|(one, _), (other, _)| other.cmp(one));
    read.into_iter()
        .take(RECENT)
        .map(|(_, counted)| counted)
        .collect()
}

/// One of a conversation's timestamps, or nothing.
fn stamp<'a>(each: &'a Value, key: &str) -> &'a str {
    each.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A conversation as counted, by its id, when it has one.
fn named(each: &Value, kind: Kind, count: u64, at: &str) -> Option<Counted> {
    let id = each.get("id").and_then(Value::as_str)?;
    Some(Counted {
        id: id.to_owned(),
        kind,
        count,
        at: at.to_owned(),
    })
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
pub(crate) fn group(name: &str) -> String {
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
    /// What's unread now and what he read last, as `client.counts` says.
    ///
    /// # Errors
    ///
    /// Slack's refusal, as the sign-in being refused or Slack being out of
    /// reach.
    pub fn counts(&self, account: &Account) -> Result<Counts, Refused> {
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
            || written.recent != now.recent
            || now.checked.saturating_sub(written.checked) >= every
    })
}

/// What Slack refusing the sign-in asks of Joel, as Needs You reads it
/// (VKCF3Z, 347abdb7), in the shape `mesh_wait` writes each quest's file of
/// what waits on him: one item he can do now, known done by the signal
/// `signin:slack`. slack-unreads takes the file away once Slack answers
/// again, so it goes whether or not anything hears the signal.
pub fn sign_in_waits(at: u64) -> Value {
    json!({
        "v": 2,
        "updated": iso(at),
        "quest": "slack",
        "items": [{
            "id": "slack-sign-in",
            "title": "Sign In to Slack",
            "why": "Slack refused pi's sign-in, so nothing can say what in Slack wants you",
            "minutes": 1,
            "ready": true,
            "doneWhen": {"signal": "signin:slack"},
            // Slack's program row signs in again in one key, so that comes
            // first, and pi's own command after it for where the launcher
            // isn't at hand (VKCF3Z, 67b9e042).
            "steps": [
                "Press Sign In on Slack in Programs",
                "Sign in to Slack in the browser it opens",
                "Or type /slack-auth in a pi session"
            ]
        }]
    })
}

/// A time in milliseconds since the epoch as ISO 8601 in UTC, as
/// JavaScript's `toISOString` writes it, by the civil calendar's days from
/// the epoch (Howard Hinnant's `civil_from_days`).
fn iso(at: u64) -> String {
    const DAY: u64 = 86_400_000;
    let rest = at % DAY;
    let shifted = i64::try_from(at / DAY).unwrap_or(0) + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * of_year + 2) / 153;
    let day = of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_is_written_as_javascript_writes_it() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_730_300_000_123), "2024-10-30T14:53:20.123Z");
        // A leap day, and the last moment of a year.
        assert_eq!(iso(1_709_164_800_000), "2024-02-29T00:00:00.000Z");
        assert_eq!(iso(1_735_689_599_999), "2024-12-31T23:59:59.999Z");
    }

    #[test]
    fn a_refused_sign_in_waits_on_joel_until_slack_hears_him_sign_in() {
        let waits = sign_in_waits(0);
        let item = &waits["items"][0];
        assert_eq!(
            (
                &waits["v"],
                &waits["updated"],
                &item["title"],
                &item["ready"],
                &item["doneWhen"]
            ),
            (
                &json!(2),
                &json!("1970-01-01T00:00:00.000Z"),
                &json!("Sign In to Slack"),
                &json!(true),
                &json!({"signal": "signin:slack"})
            )
        );
        // The one key first, pi's command only as the way round it.
        let steps = item["steps"].as_array().unwrap();
        assert_eq!(steps[0], "Press Sign In on Slack in Programs");
        assert_eq!(
            steps[steps.len() - 1],
            "Or type /slack-auth in a pi session"
        );
    }

    /// An answer of `client.counts` with one of each: direct messages
    /// read and unread, channels mentioning him, channels merely unread,
    /// one never read, and threads.
    fn counts() -> Value {
        json!({
            "ok": true,
            "ims": [
                {"id": "D1", "has_unreads": true, "mention_count": 2,
                 "latest": "1730300000.000100", "last_read": "1730100000.000100"},
                {"id": "D2", "has_unreads": false, "mention_count": 0,
                 "last_read": "1730600000.000100"}
            ],
            "mpims": [{"id": "G1", "has_unreads": true, "mention_count": 1,
                        "latest": "1730400000.000100"}],
            "channels": [
                {"id": "C1", "has_unreads": true, "mention_count": 0,
                 "latest": "1730100000.000100", "last_read": "1730700000.000100"},
                {"id": "C2", "has_unreads": true, "mention_count": 3,
                 "latest": "1730200000.000100"},
                {"id": "C3", "has_unreads": true, "mention_count": 1,
                 "latest": "1730500000.000100"},
                {"id": "C4", "has_unreads": true, "mention_count": 0,
                 "latest": "1730800000.000100", "last_read": "0000000000.000000"},
                {"id": "C5", "has_unreads": false, "mention_count": 0}
            ],
            "threads": {"has_unreads": true, "mention_count": 4}
        })
    }

    fn shown(counted: &[Counted]) -> Vec<(&str, Kind, u64)> {
        counted
            .iter()
            .map(|each| (each.id.as_str(), each.kind, each.count))
            .collect()
    }

    #[test]
    fn what_wants_joel_comes_newest_first_and_channels_merely_unread_after_it() {
        let counts = counted(&counts()).unwrap();
        assert_eq!(
            shown(&counts.unread),
            [
                ("G1", Kind::Dm, 1),
                ("D1", Kind::Dm, 2),
                ("C3", Kind::Mention, 1),
                ("C2", Kind::Mention, 3),
                ("threads", Kind::Thread, 4),
                ("C4", Kind::Unread, 0),
                ("C1", Kind::Unread, 0)
            ]
        );
        let wanting: Vec<bool> = counts.unread.iter().map(|each| each.kind.wants()).collect();
        assert_eq!(wanting, [true, true, true, true, true, false, false]);
    }

    #[test]
    fn recent_is_what_he_read_last_never_one_he_never_read() {
        let counts = counted(&counts()).unwrap();
        assert_eq!(
            shown(&counts.recent),
            [
                ("C1", Kind::Channel, 0),
                ("D2", Kind::Dm, 0),
                ("D1", Kind::Dm, 0)
            ]
        );
    }

    #[test]
    fn recent_holds_nine() {
        let channels: Vec<Value> = (10..30)
            .map(|at| json!({"id": format!("C{at}"), "last_read": format!("17300000{at}.000100")}))
            .collect();
        let counts = counted(&json!({"ok": true, "channels": channels})).unwrap();
        let ids: Vec<&str> = counts.recent.iter().map(|each| each.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "C29", "C28", "C27", "C26", "C25", "C24", "C23", "C22", "C21"
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
            at: String::new(),
        };
        let threads = Counted {
            id: "threads".to_owned(),
            kind: Kind::Thread,
            count: 1,
            at: String::new(),
        };
        assert_eq!(link("T1", &dm), "slack://channel?team=T1&id=D1");
        assert_eq!(link("T1", &threads), "slack://open?team=T1");
    }

    #[test]
    fn said_as_the_shell_reads_it() {
        let said = Said {
            checked: 5,
            state: State::SignedOut,
            conversations: vec![Conversation {
                id: "D1".to_owned(),
                name: "Ada".to_owned(),
                kind: Kind::Dm,
                count: 2,
                link: "slack://channel?team=T1&id=D1".to_owned(),
                at: "1730300000.000100".to_owned(),
            }],
            recent: vec![Conversation {
                id: "C1".to_owned(),
                name: "#launcher".to_owned(),
                kind: Kind::Channel,
                count: 0,
                link: "slack://channel?team=T1&id=C1".to_owned(),
                at: "1730700000.000100".to_owned(),
            }],
        };
        assert_eq!(
            serde_json::to_value(&said).unwrap(),
            json!({"checked": 5, "state": "signedOut", "conversations": [{
                "id": "D1", "name": "Ada", "kind": "dm", "count": 2,
                "link": "slack://channel?team=T1&id=D1"
            }], "recent": [{
                "id": "C1", "name": "#launcher", "kind": "channel", "count": 0,
                "link": "slack://channel?team=T1&id=C1"
            }]})
        );
    }

    #[test]
    fn written_again_only_once_it_changes_or_goes_old() {
        let one = |id: &str, count: u64| Conversation {
            id: id.to_owned(),
            name: "Ada".to_owned(),
            kind: Kind::Dm,
            count,
            link: String::new(),
            at: String::new(),
        };
        let at = |checked: u64, count: u64, read: &str| Said {
            checked,
            state: State::Ok,
            conversations: vec![one("D1", count)],
            recent: vec![one(read, 0)],
        };
        assert!(due(None, &at(0, 1, "D1"), 300_000));
        assert!(!due(Some(&at(0, 1, "D1")), &at(30_000, 1, "D1"), 300_000));
        assert!(due(Some(&at(0, 1, "D1")), &at(30_000, 2, "D1"), 300_000));
        assert!(due(Some(&at(0, 1, "D1")), &at(30_000, 1, "D2"), 300_000));
        assert!(due(Some(&at(0, 1, "D1")), &at(300_000, 1, "D1"), 300_000));
    }
}
