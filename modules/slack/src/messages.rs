//! Each Slack message that wants Joel, held as a record of its own for For
//! You's Slack row, so its notice clears itself once he's read it in Slack
//! (Joel, 2026-11-08: "i shouldn't be required to mark a notice as read if
//! I looked at the slack message in slack"; Y2YBTS and FUW70D settled the
//! shape). A message wants him as Now's rows do: each unread in a direct
//! message or a group's, and each in a channel that mentions him.
//!
//! Each is read with `conversations.history` from his read mark onwards,
//! or all of it in a conversation he's never opened, only in the round
//! slack-unreads already makes, and only for a conversation whose unreads
//! changed since it was last read, so nothing more is asked while nothing
//! comes. Once Slack's read mark for its conversation, `client.counts`'
//! `last_read`, reaches it, it's said done, read in Slack, and kept a day,
//! so whatever holds its notice hears that before it goes; one whose
//! conversation Slack no longer lists, as when he leaves a channel, is
//! said done too, gone from Slack. Whether every message wanting him is
//! held is said too (`whole`), since a history Slack didn't give, or one
//! longer than is read, leaves some out. A mention in a thread isn't held,
//! nor a reply sent to its channel too: a thread's replies have no read
//! mark in `client.counts` to hold them against, so Threads stays a count.
//! Slack's read mark for each conversation holding one is held too, as
//! when it was seen to move. It only ever reads.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use serde_json::Value;

use crate::unreads::{Conversation, Kind, answered};
use crate::{Account, FIRST_WORDS, LONGEST, Refused, Slack, cut, millis, plain, search};

/// How many messages of one conversation a page of its history holds, the
/// most Slack advises.
const HISTORY: &str = "200";

/// How many pages of one conversation's history are read at most, from
/// his read mark on: past them, its messages aren't said whole.
const PAGES: usize = 5;

/// How long a message done is still said, in milliseconds.
pub const KEPT: u64 = 24 * 60 * 60 * 1000;

/// How a message read in Slack is said done, by an id the shell words
/// (words.json's slack:message), as Google's daemon says how mail went.
pub const READ_IN_SLACK: &str = "readInSlack";

/// How a message is said done whose conversation Slack no longer lists.
pub const GONE_FROM_SLACK: &str = "goneFromSlack";

/// Who said a message when Slack can't say, or nobody did.
const SLACK: &str = "Slack";

/// One message wanting Joel, as the shell is told it: Search Slack's
/// fields, but whose conversation and Slack's time for it, which its key
/// names; and once it's done, how and when.
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
    /// Who said it, by their id, so a name Slack couldn't say is asked
    /// again.
    #[serde(skip)]
    pub user: String,
    /// Whether `who` is what Slack said, rather than a stand-in for it.
    #[serde(skip)]
    pub named: bool,
    /// Whether Joel said it, which none held is, as a search says it.
    pub mine: bool,
    /// Its first words, which title it.
    pub first: String,
    /// It all, in plain words, as far as a drawer shows.
    pub text: String,
    /// In milliseconds since the epoch.
    pub at: u64,
    /// Where Slack shows it on the web, for its link to be copied, as a
    /// search's is; empty until Slack has said where its workspace is.
    pub link: String,
    /// The workspace it's in, which a `slack://` link opening its
    /// conversation in Slack names.
    pub team: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<Done>,
}

/// How and when a message stopped wanting Joel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Done {
    pub how: &'static str,
    /// When it was seen done, in ISO 8601, in UTC.
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

/// A Slack timestamp as its seconds and millionths, so two are told apart
/// by when they are rather than by how they're written; none for one that
/// isn't a time, as an empty read mark.
pub fn instant(ts: &str) -> Option<(u64, u32)> {
    let (seconds, fraction) = ts.split_once('.').unwrap_or((ts, ""));
    if fraction.len() > 6 || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let millionths: String = fraction.chars().chain("000000".chars()).take(6).collect();
    Some((seconds.parse().ok()?, millionths.parse().ok()?))
}

/// The messages of a page of a conversation's history wanting Joel: each
/// after `read`, his read mark, or any when he's never read it; that he
/// didn't say himself; and in a channel only those mentioning him or
/// everyone there. One that only says someone joined or left, a thread's
/// reply sent to its channel too, or one holding no words, isn't one. `me`
/// is Joel's own id.
///
/// # Errors
///
/// Slack's refusal, as the sign-in being refused or Slack being out of
/// reach.
pub fn unread(said: &Value, read: &str, me: &str, mentions: bool) -> Result<Vec<Unread>, Refused> {
    answered(said)?;
    let mark = instant(read);
    let messages = said.get("messages").and_then(Value::as_array);
    Ok(messages
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default();
            let ts = text("ts");
            let words = text("text");
            let kept = matches!(
                text("subtype"),
                "" | "bot_message" | "file_share" | "me_message"
            );
            let mine = !me.is_empty() && text("user") == me;
            let wanted = !mentions || mentioned(words, me);
            let after = instant(ts).is_some_and(|at| mark.is_none_or(|mark| at > mark));
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
        .collect())
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

/// Each held message his read mark has reached said done at `now`, read
/// in Slack, and each whose conversation `read` doesn't list said done,
/// gone from Slack; and each done more than [`KEPT`] ago let go. `read`
/// holds every conversation Slack lists, by its read mark, empty for one
/// never read.
pub fn settle(held: &mut BTreeMap<String, Raised>, read: &BTreeMap<String, String>, now: u64) {
    for one in held.values_mut().filter(|one| one.done.is_none()) {
        let how = match read.get(&one.channel) {
            None => GONE_FROM_SLACK,
            Some(mark) if instant(mark).is_some_and(|mark| Some(mark) >= instant(&one.ts)) => {
                READ_IN_SLACK
            }
            Some(_) => continue,
        };
        one.done = Some(Done {
            how,
            at: iso(now),
            millis: now,
        });
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

/// Where Slack shows a message on the web, from where its workspace is,
/// as Slack's own permalinks are made.
pub fn link(web: &str, channel: &str, ts: &str) -> String {
    let web = web.trim_end_matches('/');
    format!("{web}/archives/{channel}/p{}", ts.replace('.', ""))
}

/// The messages wanting Joel that slack-unreads holds between rounds.
#[derive(Default)]
pub struct Messages {
    held: BTreeMap<String, Raised>,
    /// Each conversation's unreads when its history was last read, by
    /// their latest and count, so it's read again only once they change,
    /// and whether it was read to its end.
    asked: HashMap<String, ((String, u64), bool)>,
    /// Who each person who said one is, asked once a run.
    people: People,
    /// Each read mark held, and when it was seen there.
    marks: BTreeMap<String, (String, u64)>,
    /// Where the workspace is on the web, asked once a run.
    web: Option<String>,
    /// Whether the last round held every message wanting him.
    whole: bool,
}

/// Who each person is, by their Slack id, asked of `users.info` once a run
/// and shared by the rounds and Search Slack, so one message has one name
/// wherever it's said, and a search asks only for people no round has.
#[derive(Clone, Default)]
pub struct People(Arc<Mutex<HashMap<String, String>>>);

impl People {
    /// Who the person with `id` is, when they've been asked.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<String> {
        self.lock().get(id).cloned()
    }

    /// Keeps who the person with `id` is.
    pub fn keep(&self, id: &str, name: &str) {
        self.lock().insert(id.to_owned(), name.to_owned());
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Messages {
    /// Messages held, naming people from `people`, which a search shares.
    #[must_use]
    pub fn sharing(people: People) -> Messages {
        Messages {
            people,
            ..Messages::default()
        }
    }

    /// What a round of asking Slack found, taken at `now`: the unread of
    /// each conversation wanting Joel whose unreads changed, read from its
    /// history, and each held one his read mark, `read`, has reached, said
    /// done. Slack out of reach for one conversation leaves it to be read
    /// next round and the rest as they are, said once a round, and the
    /// round not whole.
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
        self.whole = false;
        let mut whole = true;
        let mut missed = None;
        for conversation in wanting {
            let mentions = match conversation.kind {
                Kind::Dm => false,
                Kind::Mention => true,
                _ => continue,
            };
            let unreads = (conversation.at.clone(), conversation.count);
            if conversation.count == 0 {
                continue;
            }
            if let Some((was, all)) = self.asked.get(&conversation.id)
                && *was == unreads
            {
                whole &= *all;
                continue;
            }
            let mark = read.get(&conversation.id).map_or("", String::as_str);
            let (found, all) = match history(slack, account, &conversation.id, mark, mentions) {
                Ok(found) => found,
                Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                Err(Refused::Unreachable(why)) => {
                    missed.get_or_insert(why);
                    whole = false;
                    continue;
                }
            };
            whole &= all;
            for one in found {
                let key = search::key(&conversation.id, &one.ts);
                if self.held.contains_key(&key) {
                    continue;
                }
                let who = self.who(slack, account, &one.by, &one.user)?;
                let words = plain(&one.text, &[]);
                let link = self
                    .web
                    .as_deref()
                    .map(|web| link(web, &conversation.id, &one.ts))
                    .unwrap_or_default();
                self.held.insert(
                    key.clone(),
                    Raised {
                        key,
                        channel: conversation.id.clone(),
                        place: conversation.name.clone(),
                        named: who.is_some(),
                        who: who.unwrap_or_else(|| SLACK.to_owned()),
                        user: one.user,
                        mine: false,
                        first: cut(words.lines().next().unwrap_or_default(), FIRST_WORDS),
                        text: cut(&words, LONGEST),
                        at: millis(&one.ts).unwrap_or_default(),
                        ts: one.ts,
                        link,
                        team: account.team.clone(),
                        done: None,
                    },
                );
            }
            self.asked.insert(conversation.id.clone(), (unreads, all));
        }
        if let Some(why) = missed {
            eprintln!("<4>{why}");
        }
        // A conversation no longer wanting Joel is read afresh should it
        // want him again.
        self.asked
            .retain(|id, _| wanting.iter().any(|conversation| &conversation.id == id));
        self.finish(slack, account)?;
        settle(&mut self.held, read, now);
        let holding: Vec<&String> = self.held.values().map(|one| &one.channel).collect();
        self.marks.retain(|channel, _| holding.contains(&channel));
        for channel in holding {
            let Some(mark) = read.get(channel).filter(|mark| !mark.is_empty()) else {
                continue;
            };
            let moved = self.marks.get(channel).is_none_or(|(was, _)| was != mark);
            if moved {
                self.marks.insert(channel.clone(), (mark.clone(), now));
            }
        }
        self.whole = whole;
        Ok(())
    }

    /// Each held message Slack couldn't yet name the sayer of, or say the
    /// workspace's place on the web for, asked again, each at most once a
    /// round and none once Slack doesn't answer.
    fn finish(&mut self, slack: &Slack, account: &Account) -> Result<(), Refused> {
        let unnamed: Vec<(String, String)> = self
            .held
            .values()
            .filter(|one| !one.named && one.done.is_none())
            .map(|one| (one.key.clone(), one.user.clone()))
            .collect();
        for (key, user) in unnamed {
            let Some(name) = self.who(slack, account, "", &user)? else {
                break;
            };
            if let Some(one) = self.held.get_mut(&key) {
                one.who = name;
                one.named = true;
            }
        }
        if self.held.values().all(|one| !one.link.is_empty()) {
            return Ok(());
        }
        if self.web.is_none() {
            self.web = match slack.ask(account, "auth.test", &[]) {
                Ok(said) => match answered(&said) {
                    Ok(said) => said.get("url").and_then(Value::as_str).map(str::to_owned),
                    Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                    Err(Refused::Unreachable(_)) => None,
                },
                Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                Err(Refused::Unreachable(_)) => None,
            };
        }
        if let Some(web) = &self.web {
            for one in self.held.values_mut().filter(|one| one.link.is_empty()) {
                one.link = link(web, &one.channel, &one.ts);
            }
        }
        Ok(())
    }

    /// Who said a message: the name Slack gives a bot's, `by`, or a
    /// person's name, asked once a run; one nobody said is said as Slack,
    /// and none when Slack is too busy or out of reach to say.
    fn who(
        &mut self,
        slack: &Slack,
        account: &Account,
        by: &str,
        user: &str,
    ) -> Result<Option<String>, Refused> {
        if !by.is_empty() {
            return Ok(Some(by.to_owned()));
        }
        if user.is_empty() {
            return Ok(Some(SLACK.to_owned()));
        }
        if let Some(name) = self.people.get(user) {
            return Ok(Some(name));
        }
        match slack.ask(account, "users.info", &[("user", user)]) {
            // Only a name Slack gave is kept, so a person it couldn't say
            // is asked again next run.
            Ok(said) => Ok(Some(search::name_in(&said).map_or_else(
                || SLACK.to_owned(),
                |name| {
                    self.people.keep(user, &name);
                    name
                },
            ))),
            Err(Refused::SignedOut) => Err(Refused::SignedOut),
            Err(Refused::Unreachable(_)) => Ok(None),
        }
    }

    /// Each message held, newest first.
    pub fn raised(&self) -> Vec<Raised> {
        let mut raised: Vec<Raised> = self.held.values().cloned().collect();
        raised.sort_by(|one, other| {
            instant(&other.ts)
                .cmp(&instant(&one.ts))
                .then_with(|| one.key.cmp(&other.key))
        });
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

    /// Whether the last round held every message wanting Joel: each
    /// conversation's history Slack gave, each read to its end. Until a
    /// round has, as just after a start, or while Slack doesn't answer,
    /// one missing from what's held isn't known to be read.
    pub fn whole(&self) -> bool {
        self.whole
    }
}

/// The messages of a conversation's history wanting Joel, oldest first,
/// from his read mark, `mark`, or all of it when it's empty, a page at a
/// time up to [`PAGES`]; and whether it was read to its end.
fn history(
    slack: &Slack,
    account: &Account,
    channel: &str,
    mark: &str,
    mentions: bool,
) -> Result<(Vec<Unread>, bool), Refused> {
    let mut found = Vec::new();
    let mut cursor = String::new();
    let mut all = false;
    for _ in 0..PAGES {
        let mut query = vec![("channel", channel), ("limit", HISTORY)];
        if !mark.is_empty() {
            query.push(("oldest", mark));
        }
        if !cursor.is_empty() {
            query.push(("cursor", cursor.as_str()));
        }
        let said = slack.ask(account, "conversations.history", &query)?;
        found.extend(unread(&said, mark, &account.user, mentions)?);
        let next = said
            .pointer("/response_metadata/next_cursor")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if said.get("has_more").and_then(Value::as_bool) != Some(true) || next.is_empty() {
            all = true;
            break;
        }
        next.clone_into(&mut cursor);
    }
    found.sort_by_key(|one| instant(&one.ts));
    Ok((found, all))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn raised(channel: &str, ts: &str) -> Raised {
        Raised {
            key: search::key(channel, ts),
            channel: channel.to_owned(),
            ts: ts.to_owned(),
            place: "Ada".to_owned(),
            who: "Ada".to_owned(),
            user: "U7".to_owned(),
            named: true,
            mine: false,
            first: "hi".to_owned(),
            text: "hi".to_owned(),
            at: millis(ts).unwrap(),
            link: String::new(),
            team: "T1".to_owned(),
            done: None,
        }
    }

    /// Read marks by id, as `client.counts` says them.
    pub(crate) fn marks(marks: &[(&str, &str)]) -> BTreeMap<String, String> {
        marks
            .iter()
            .map(|(id, mark)| ((*id).to_owned(), (*mark).to_owned()))
            .collect()
    }

    #[test]
    fn a_conversations_unread_are_those_after_his_mark_not_his_oldest_first() {
        let said = json!({"ok": true, "messages": [
            {"ts": "1730300003.000100", "user": "U7", "text": "third"},
            {"ts": "1730300002.000100", "user": "U0", "text": "his own"},
            {"ts": "1730300001.000100", "subtype": "channel_join", "user": "U8",
             "text": "<@U8> has joined"},
            {"ts": "1730300001.500000", "subtype": "thread_broadcast", "user": "U7",
             "thread_ts": "1730200000.000100", "text": "a reply sent to the channel too"},
            {"ts": "1730300000.900000", "subtype": "bot_message", "username": "deploys",
             "bot_profile": {"name": "Merge Garden"}, "text": "merged"},
            {"ts": "1730300000.000100", "user": "U7", "text": "read already"},
            {"ts": "1730300000.500000", "user": "U7", "text": "  "}
        ]});
        let mut found = unread(&said, "1730300000.000100", "U0", false).unwrap();
        found.sort_by_key(|one| instant(&one.ts));
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
    fn in_a_conversation_he_never_read_every_message_but_his_wants_him() {
        let said = json!({"ok": true, "messages": [
            {"ts": "1730300002.000100", "user": "U0", "text": "his own"},
            {"ts": "1730300001.000100", "user": "U7", "text": "hello, new here"}
        ]});
        let found: Vec<String> = unread(&said, "", "U0", false)
            .unwrap()
            .into_iter()
            .map(|one| one.text)
            .collect();
        assert_eq!(found, ["hello, new here"]);
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
        assert_eq!(found, ["<!here> standup", "hey <@U0|joel>", "<@U0> look"]);
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
    fn a_timestamp_is_told_by_when_it_is_not_how_its_written() {
        assert!(instant("1730300001.5") > instant("1730300001.000100"));
        assert!(instant("999999999.000000") < instant("1000000000.000000"));
        assert_eq!(instant("1730300001"), Some((1_730_300_001, 0)));
        assert_eq!(instant(""), None);
        assert_eq!(instant("1730300001.1234567"), None);
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
        let read = marks(&[("D1", "1730300001.000100"), ("C2", "")]);
        let now = 1_730_400_000_000;
        settle(&mut held, &read, now);
        let done = |key: &str| held[key].done.clone();
        assert_eq!(
            done("slack-message:D1:1730300001.000100"),
            Some(Done {
                how: "readInSlack",
                at: "2024-10-31T18:40:00Z".to_owned(),
                millis: now,
            })
        );
        // Past his mark, and in a conversation he's never read, it still
        // wants him.
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
    fn a_message_whose_conversation_slack_no_longer_lists_is_done_gone_from_slack() {
        let mut held: BTreeMap<String, Raised> = [raised("C3", "1730300001.000100")]
            .into_iter()
            .map(|one| (one.key.clone(), one))
            .collect();
        settle(
            &mut held,
            &marks(&[("D1", "1730300009.000100")]),
            1_730_400_000_000,
        );
        assert_eq!(
            held["slack-message:C3:1730300001.000100"]
                .done
                .as_ref()
                .map(|done| done.how),
            Some("goneFromSlack")
        );
    }

    #[test]
    fn a_time_is_said_in_iso_8601_in_utc() {
        assert_eq!(iso(1_730_300_000_000), "2024-10-30T14:53:20Z");
        assert_eq!(iso(951_782_400_999), "2000-02-29T00:00:00Z");
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_messages_link_is_where_slack_shows_it_on_the_web() {
        assert_eq!(
            link("https://ada.slack.com/", "C2", "1730100003.000100"),
            "https://ada.slack.com/archives/C2/p1730100003000100"
        );
    }

    #[test]
    fn a_held_message_is_said_with_searchs_fields_and_done_only_once_read() {
        let mut one = raised("D1", "1730300001.000100");
        one.link = link("https://ada.slack.com", "D1", "1730300001.000100");
        assert_eq!(
            serde_json::to_value(&one).unwrap(),
            json!({"channel": "D1", "ts": "1730300001.000100", "place": "Ada", "who": "Ada",
                   "mine": false, "first": "hi", "text": "hi", "at": 1_730_300_001_000_u64,
                   "link": "https://ada.slack.com/archives/D1/p1730300001000100",
                   "team": "T1"})
        );
        one.done = Some(Done {
            how: READ_IN_SLACK,
            at: iso(1_730_400_000_000),
            millis: 1_730_400_000_000,
        });
        assert_eq!(
            serde_json::to_value(&one).unwrap()["done"],
            json!({"how": "readInSlack", "at": "2024-10-31T18:40:00Z"})
        );
    }
}
