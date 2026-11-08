//! Each Slack message that wants Joel, held as a record of its own for For
//! You's Slack row, so its notice clears itself once he's read it in Slack
//! (Joel, 2026-11-08: "i shouldn't be required to mark a notice as read if
//! I looked at the slack message in slack"; Y2YBTS and FUW70D settled the
//! shape). A message wants him as Now's rows do: each unread in a direct
//! message or a group's, and each in a channel that mentions him.
//!
//! Each is read with `conversations.history` from his read mark onwards,
//! only in the round slack-unreads already makes, and only for a
//! conversation whose unreads changed since it was last read, so nothing
//! more is asked while nothing comes. Once Slack's read mark for its
//! conversation, `client.counts`' `last_read`, reaches it, it's said done,
//! read in Slack, and kept a day, so whatever holds its notice hears that
//! before it goes. A mention in a thread isn't held: a thread's replies
//! have no read mark in `client.counts` to hold them against, so Threads
//! stays a count. Slack's read mark for each conversation holding one is
//! held too, as when it was seen to move. It only ever reads.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use serde_json::Value;

use crate::unreads::{Conversation, Kind, answered};
use crate::{Account, FIRST_WORDS, LONGEST, Refused, Slack, cut, millis, plain, search};

/// How many messages of one conversation are read at most, from his read
/// mark on: more than any direct message's unreads he'd wade through.
const HISTORY: &str = "50";

/// How long a message read in Slack is still said, in milliseconds.
pub const KEPT: u64 = 24 * 60 * 60 * 1000;

/// How a message read in Slack is said done.
pub const READ_IN_SLACK: &str = "read in Slack";

/// One message wanting Joel, as the shell is told it: Search Slack's
/// fields but whether it's his, since none of his wants him; the
/// conversation and Slack's time for it, which its key names; and once
/// it's read, how and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Raised {
    /// `slack-message:<channel>:<ts>`, as a search names a message, so
    /// asking for it gets the messages around it.
    #[serde(skip)]
    pub key: String,
    pub channel: String,
    pub ts: String,
    /// The conversation it's in, as Now names it.
    pub place: String,
    /// Who said it, by their name in Slack.
    pub who: String,
    /// Its first words, which title it.
    pub first: String,
    /// It all, in plain words, as far as a drawer shows.
    pub text: String,
    /// In milliseconds since the epoch.
    pub at: u64,
    /// The `slack://` link opening its conversation in Slack.
    pub link: String,
    pub team: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<Done>,
}

/// How and when a message stopped wanting Joel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Done {
    pub how: &'static str,
    /// When it was seen read, in ISO 8601, in UTC.
    pub at: String,
    /// The same, in milliseconds since the epoch, for how long it's kept.
    #[serde(skip)]
    pub millis: u64,
}

/// Slack's read mark for a conversation holding a message wanting Joel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mark {
    pub channel: String,
    /// Slack's timestamp of the last message he read there.
    pub read: String,
    /// When it was seen there, in ISO 8601, in UTC: when it moved, or when
    /// the daemon first saw it.
    pub at: String,
}

/// One unread message a conversation's history says, not yet named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unread {
    pub ts: String,
    /// Who said it, by their id, when a person did.
    pub user: String,
    /// Who said it, when Slack names them itself, as a bot's messages are.
    pub by: String,
    pub text: String,
}

/// The messages of a conversation's history wanting Joel, oldest first:
/// each after `read`, his read mark, that he didn't say himself, and in a
/// channel only those mentioning him or everyone there. One that only
/// says someone joined or left, or holds no words, isn't one. `me` is
/// Joel's own id.
///
/// # Errors
///
/// Slack's refusal, as the sign-in being refused or Slack being out of
/// reach.
pub fn unread(said: &Value, read: &str, me: &str, mentions: bool) -> Result<Vec<Unread>, Refused> {
    answered(said)?;
    let messages = said.get("messages").and_then(Value::as_array);
    let mut found: Vec<Unread> = messages
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default();
            let ts = text("ts");
            let words = text("text");
            let kept = matches!(
                text("subtype"),
                "" | "bot_message" | "thread_broadcast" | "file_share" | "me_message"
            );
            let mine = !me.is_empty() && text("user") == me;
            let wanted = !mentions || mentioned(words, me);
            let after = ts > read && millis(ts).is_some();
            (kept && !mine && wanted && after && !words.trim().is_empty()).then(|| Unread {
                ts: ts.to_owned(),
                user: text("user").to_owned(),
                by: one
                    .pointer("/bot_profile/name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| text("username"))
                    .to_owned(),
                text: words.to_owned(),
            })
        })
        .collect();
    found.sort_by(|one, other| one.ts.cmp(&other.ts));
    Ok(found)
}

/// Whether a message's markup mentions Joel, by his id, or everyone in
/// its channel, as Slack counts a mention.
fn mentioned(text: &str, me: &str) -> bool {
    let his = !me.is_empty()
        && (text.contains(&format!("<@{me}>")) || text.contains(&format!("<@{me}|")));
    his || ["<!here", "<!channel", "<!everyone"]
        .iter()
        .any(|everyone| text.contains(everyone))
}

/// Each held message his read mark has reached said done at `now`, and
/// each done more than [`KEPT`] ago let go. A message whose conversation
/// Slack names no read mark for is left as it is.
pub fn settle(held: &mut BTreeMap<String, Raised>, read: &BTreeMap<String, String>, now: u64) {
    for one in held.values_mut().filter(|one| one.done.is_none()) {
        if read
            .get(&one.channel)
            .is_some_and(|mark| mark.as_str() >= one.ts.as_str())
        {
            one.done = Some(Done {
                how: READ_IN_SLACK,
                at: iso(now),
                millis: now,
            });
        }
    }
    held.retain(|_, one| {
        one.done
            .as_ref()
            .is_none_or(|done| now.saturating_sub(done.millis) <= KEPT)
    });
}

/// A time in milliseconds since the epoch, in ISO 8601, in UTC, to the
/// second, by the proleptic Gregorian calendar (Howard Hinnant's
/// `civil_from_days`).
pub fn iso(millis: u64) -> String {
    let seconds = millis / 1000;
    let (days, of_day) = (seconds / 86_400, seconds % 86_400);
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let of_era = shifted % 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// The messages wanting Joel that slack-unreads holds between rounds.
#[derive(Default)]
pub struct Messages {
    held: BTreeMap<String, Raised>,
    /// Each conversation's unreads when its history was last read, by
    /// their latest and count, so it's read again only once they change.
    asked: HashMap<String, (String, u64)>,
    /// Who each person who said one is, asked once a run.
    people: HashMap<String, String>,
    /// Each read mark held, and when it was seen there.
    marks: BTreeMap<String, (String, u64)>,
}

impl Messages {
    /// What a round of asking Slack found, taken at `now`: the unread of
    /// each conversation wanting Joel whose unreads changed, read from its
    /// history, and each held one his read mark, `read`, has reached, said
    /// done. Slack out of reach for one conversation leaves it to be read
    /// next round and the rest as they are, said once a round.
    ///
    /// # Errors
    ///
    /// Slack refusing the sign-in, so the round is said signed out.
    pub fn heard(
        &mut self,
        slack: &Slack,
        account: &Account,
        wanting: &[Conversation],
        read: &BTreeMap<String, String>,
        now: u64,
    ) -> Result<(), Refused> {
        let mut missed = None;
        for conversation in wanting {
            let mentions = match conversation.kind {
                Kind::Dm => false,
                Kind::Mention => true,
                _ => continue,
            };
            // With no read mark, nothing says which of its messages are
            // unread.
            let Some(mark) = read.get(&conversation.id) else {
                continue;
            };
            let unreads = (conversation.at.clone(), conversation.count);
            if conversation.count == 0 || self.asked.get(&conversation.id) == Some(&unreads) {
                continue;
            }
            let query = [
                ("channel", conversation.id.as_str()),
                ("oldest", mark.as_str()),
                ("limit", HISTORY),
            ];
            let found = slack
                .ask(account, "conversations.history", &query)
                .and_then(|said| unread(&said, mark, &account.user, mentions));
            let found = match found {
                Ok(found) => found,
                Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                Err(Refused::Unreachable(why)) => {
                    missed.get_or_insert(why);
                    continue;
                }
            };
            for one in found {
                let key = search::key(&conversation.id, &one.ts);
                if self.held.contains_key(&key) {
                    continue;
                }
                let who = self.who(slack, account, &one)?;
                let words = plain(&one.text, &[]);
                self.held.insert(
                    key.clone(),
                    Raised {
                        key,
                        channel: conversation.id.clone(),
                        place: conversation.name.clone(),
                        who,
                        first: cut(words.lines().next().unwrap_or_default(), FIRST_WORDS),
                        text: cut(&words, LONGEST),
                        at: millis(&one.ts).unwrap_or_default(),
                        ts: one.ts,
                        link: conversation.link.clone(),
                        team: account.team.clone(),
                        done: None,
                    },
                );
            }
            self.asked.insert(conversation.id.clone(), unreads);
        }
        if let Some(why) = missed {
            eprintln!("<4>{why}");
        }
        // A conversation no longer wanting Joel is read afresh should it
        // want him again.
        self.asked
            .retain(|id, _| wanting.iter().any(|conversation| &conversation.id == id));
        settle(&mut self.held, read, now);
        let holding: Vec<&String> = self.held.values().map(|one| &one.channel).collect();
        self.marks.retain(|channel, _| holding.contains(&channel));
        for channel in holding {
            let Some(mark) = read.get(channel) else {
                continue;
            };
            let moved = self.marks.get(channel).is_none_or(|(was, _)| was != mark);
            if moved {
                self.marks.insert(channel.clone(), (mark.clone(), now));
            }
        }
        Ok(())
    }

    /// Who said a message: the name Slack gives a bot's, or a person's
    /// name, asked once a run; Slack too busy to say is said as Slack.
    fn who(&mut self, slack: &Slack, account: &Account, one: &Unread) -> Result<String, Refused> {
        if !one.by.is_empty() {
            return Ok(one.by.clone());
        }
        if one.user.is_empty() {
            return Ok("Slack".to_owned());
        }
        if let Some(name) = self.people.get(&one.user) {
            return Ok(name.clone());
        }
        match slack.ask(account, "users.info", &[("user", &one.user)]) {
            Ok(user) => {
                let name = search::named_in(&user);
                self.people.insert(one.user.clone(), name.clone());
                Ok(name)
            }
            Err(Refused::SignedOut) => Err(Refused::SignedOut),
            Err(Refused::Unreachable(_)) => Ok("Slack".to_owned()),
        }
    }

    /// Each message held, newest first.
    pub fn raised(&self) -> Vec<Raised> {
        let mut raised: Vec<Raised> = self.held.values().cloned().collect();
        raised.sort_by(|one, other| other.ts.cmp(&one.ts).then_with(|| one.key.cmp(&other.key)));
        raised
    }

    /// Each read mark held, by its conversation.
    pub fn marks(&self) -> Vec<Mark> {
        self.marks
            .iter()
            .map(|(channel, (read, at))| Mark {
                channel: channel.clone(),
                read: read.clone(),
                at: iso(*at),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raised(channel: &str, ts: &str) -> Raised {
        Raised {
            key: search::key(channel, ts),
            channel: channel.to_owned(),
            ts: ts.to_owned(),
            place: "Ada".to_owned(),
            who: "Ada".to_owned(),
            first: "hi".to_owned(),
            text: "hi".to_owned(),
            at: millis(ts).unwrap(),
            link: String::new(),
            team: "T1".to_owned(),
            done: None,
        }
    }

    #[test]
    fn a_conversations_unread_are_those_after_his_mark_not_his_oldest_first() {
        let said = json!({"ok": true, "messages": [
            {"ts": "1730300003.000100", "user": "U7", "text": "third"},
            {"ts": "1730300002.000100", "user": "U0", "text": "his own"},
            {"ts": "1730300001.000100", "subtype": "channel_join", "user": "U8",
             "text": "<@U8> has joined"},
            {"ts": "1730300000.900000", "subtype": "bot_message", "username": "deploys",
             "bot_profile": {"name": "Merge Garden"}, "text": "merged"},
            {"ts": "1730300000.000100", "user": "U7", "text": "read already"},
            {"ts": "1730300000.500000", "user": "U7", "text": "  "}
        ]});
        let found = unread(&said, "1730300000.000100", "U0", false).unwrap();
        assert_eq!(
            found,
            [
                Unread {
                    ts: "1730300000.900000".to_owned(),
                    user: String::new(),
                    by: "Merge Garden".to_owned(),
                    text: "merged".to_owned(),
                },
                Unread {
                    ts: "1730300003.000100".to_owned(),
                    user: "U7".to_owned(),
                    by: String::new(),
                    text: "third".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn in_a_channel_only_a_mention_of_him_or_everyone_wants_him() {
        let said = json!({"ok": true, "messages": [
            {"ts": "1730300004.000100", "user": "U7", "text": "<!here> standup"},
            {"ts": "1730300003.000100", "user": "U7", "text": "hey <@U0|joel>"},
            {"ts": "1730300002.000100", "user": "U7", "text": "<@U0> look"},
            {"ts": "1730300001.000100", "user": "U7", "text": "<@U01> not him"}
        ]});
        let found: Vec<String> = unread(&said, "0000000000.000000", "U0", true)
            .unwrap()
            .into_iter()
            .map(|one| one.text)
            .collect();
        assert_eq!(found, ["<@U0> look", "hey <@U0|joel>", "<!here> standup"]);
    }

    #[test]
    fn a_refused_history_is_said_as_slack_refused_it() {
        let refused = |error: &str| unread(&json!({"ok": false, "error": error}), "", "U0", false);
        assert_eq!(refused("invalid_auth"), Err(Refused::SignedOut));
        assert!(matches!(
            refused("channel_not_found"),
            Err(Refused::Unreachable(_))
        ));
    }

    #[test]
    fn a_message_his_mark_reaches_is_done_read_in_slack_and_let_go_a_day_on() {
        let mut held: BTreeMap<String, Raised> = [
            raised("D1", "1730300001.000100"),
            raised("D1", "1730300002.000100"),
            raised("C2", "1730300001.000100"),
        ]
        .into_iter()
        .map(|one| (one.key.clone(), one))
        .collect();
        let read: BTreeMap<String, String> = [("D1", "1730300001.000100")]
            .into_iter()
            .map(|(id, mark)| (id.to_owned(), mark.to_owned()))
            .collect();
        let now = 1_730_400_000_000;
        settle(&mut held, &read, now);
        let done = |key: &str| held[key].done.clone();
        assert_eq!(
            done("slack-message:D1:1730300001.000100"),
            Some(Done {
                how: "read in Slack",
                at: "2024-10-31T18:40:00Z".to_owned(),
                millis: now,
            })
        );
        // Past his mark, and in a conversation with none, it still wants him.
        assert_eq!(done("slack-message:D1:1730300002.000100"), None);
        assert_eq!(done("slack-message:C2:1730300001.000100"), None);
        // Seen done again later, it's kept as first done, until a day on.
        settle(&mut held, &read, now + KEPT);
        assert_eq!(
            held["slack-message:D1:1730300001.000100"]
                .done
                .as_ref()
                .map(|done| done.millis),
            Some(now)
        );
        settle(&mut held, &read, now + KEPT + 1);
        assert!(!held.contains_key("slack-message:D1:1730300001.000100"));
        assert_eq!(held.len(), 2);
    }

    #[test]
    fn a_time_is_said_in_iso_8601_in_utc() {
        assert_eq!(iso(1_730_300_000_000), "2024-10-30T14:53:20Z");
        assert_eq!(iso(951_782_400_999), "2000-02-29T00:00:00Z");
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_held_message_is_said_with_searchs_fields_and_done_only_once_read() {
        let mut one = raised("D1", "1730300001.000100");
        assert_eq!(
            serde_json::to_value(&one).unwrap(),
            json!({"channel": "D1", "ts": "1730300001.000100", "place": "Ada", "who": "Ada",
                   "first": "hi", "text": "hi", "at": 1_730_300_001_000_u64, "link": "",
                   "team": "T1"})
        );
        one.done = Some(Done {
            how: READ_IN_SLACK,
            at: iso(1_730_400_000_000),
            millis: 1_730_400_000_000,
        });
        assert_eq!(
            serde_json::to_value(&one).unwrap()["done"],
            json!({"how": "read in Slack", "at": "2024-10-31T18:40:00Z"})
        );
    }
}
