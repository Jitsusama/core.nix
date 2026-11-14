//! Someone Slack found, made ready for People: what slack-people says of
//! them (lib.rs's `Found`) with their drawer's facts, their act and what's
//! between them and Joel beside it, worded from the launcher's vocabulary,
//! so the people daemon merges Slack's part of a person as it is and People
//! draws it as it's served, with no QML source of Slack's (Y2YBTS's data
//! rule, 2026-11-10, and 67770006). The facts are on ready-rows.md's
//! forms: `{ name, said }`, and the time zone as `{ name, zone, as:
//! "zone" }`, unworded, since the time there reads the clock and the shell
//! says it as it ticks. How they stand is the people daemon's to word,
//! from Slack's `stands` in its sources file.
//!
//! Their act leads the ways to reach them (`main`): Reply in Slack, opening
//! their conversation, while Joel owes it an answer (VKCF3Z, 6ea9cd26), or
//! Write in Slack, opening them there. It never arrives late or moves under
//! the keys: for someone Joel steps into whom Slack is still being asked
//! about, it stands in its place dimmed, with no form, "looking in Slack",
//! and once a step-in finds nobody by the address, "no one at this address"
//! (VKCF3Z, 3bbad443). Someone known not to be in Slack by an ask ahead has
//! no such row (8a0f14ea). An entry for an address Slack knows nobody by,
//! or is still asked about, says `"known": false`, so People counts no one
//! of Slack's by it.
//!
//! Their direct messages with Joel are one conversation `between` them, a
//! ready row with its drawer, which People places among mail and meetings
//! by its `at` (XY9H5C, 2026-11-10): what they wrote each other, `mine`
//! where Joel wrote last, named by its newest message's first words, with
//! "Slack" and then how long ago beside it, a time the shell words, and its
//! last few messages in its drawer. It `waits` while one owes the other an
//! answer: theirs last waits on Joel however long ago, and his last on them
//! for two days, `until` then, past which a conversation he wrote last is
//! most often one he ended. Every act is on ready-rows.md's forms, one of
//! `{ key, ask }`, `{ key, open }` or `{ key, copy }` (Y2YBTS, 2026-11-10).

use serde_json::{Value, json};

use crate::Found;
use crate::message_rows::{capitalised, copy, open, word};

/// The act that writes to them in Slack, as words.json keys it.
pub const WRITE: &str = "verb:person:slack";
/// The act that answers their message in Slack, as words.json keys it.
pub const REPLY: &str = "verb:person:slack:reply";
/// What their title says it is, as People's drawer finds its head line.
pub const TITLE: &str = "people:title";
/// How long what Joel wrote last waits on them, in milliseconds.
pub const AWAITED: u64 = 2 * 86_400_000;

/// The words a person's facts and act are said in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    /// "Title", the fact of what their profile says they do.
    pub title: String,
    /// "Status", the fact of their status's words and emoji.
    pub status: String,
    /// "Presence", the fact of whether they're active or away.
    pub presence: String,
    /// "Time Zone", the fact of where their clock is.
    pub zone: String,
    /// "no one at this address", said in Write in Slack's place, and as
    /// the Slack fact, for an address Slack knows nobody by: "not in
    /// Slack" beside the label Slack said Slack twice (VKCF3Z,
    /// 2026-11-11).
    pub missing: String,
    /// "looking in Slack", said in Write in Slack's place while Slack is
    /// asked who an address is.
    pub looking: String,
    /// "Conversation", the name of one whose newest message says nothing.
    pub conversation: String,
    /// A conversation's glyph, by the theme's name.
    pub glyph: String,
    /// "Slack", said beside a conversation among mail and meetings.
    pub source: String,
    /// "You", who Joel is in a conversation's drawer.
    pub you: String,
}

impl Words {
    /// A person's words from the whole vocabulary, or the first one it
    /// lacks.
    ///
    /// # Errors
    ///
    /// The first word the vocabulary's `slack:person:*` facts or
    /// `verb:person:slack` lack.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        Ok(Words {
            status: word(vocabulary, "slack:person:status", &["name"])?,
            presence: word(vocabulary, "slack:person:presence", &["name"])?,
            zone: word(vocabulary, "slack:person:zone", &["name"])?,
            title: word(vocabulary, "slack:person:title", &["name"])?,
            missing: word(vocabulary, WRITE, &["said", "missing"])?,
            looking: word(vocabulary, WRITE, &["said", "looking"])?,
            conversation: word(vocabulary, "slack:conversation", &["name"])?,
            glyph: word(vocabulary, "slack:conversation", &["glyph"])?,
            source: word(vocabulary, "slack:person:source", &["name"])?,
            you: capitalised(&word(vocabulary, "slack:message", &["said", "you"])?),
        })
    }
}

/// How what's said of someone came to be asked: ahead, or kept, for someone
/// Joel may step into, or as he steps into them, when an address Slack knows
/// nobody by says so in Write in Slack's place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    Ahead,
    SteppedIn,
}

/// What slack-people says of someone: `found` as it was said before, its
/// fields unchanged, with `facts`, `acts` and `between` beside them, and
/// `known` false for an address Slack knows nobody by.
#[must_use]
pub fn ready(found: &Found, words: &Words, asked: Asked) -> Value {
    let mut said = serde_json::to_value(found).unwrap_or_else(|_| json!({}));
    said["facts"] = Value::Array(facts(found, words));
    said["acts"] = Value::Array(acts(found, words, asked));
    said["between"] = Value::Array(between(found, words));
    if found.id.is_none() {
        said["known"] = json!(false);
    }
    said
}

/// What's said at once of someone Joel steps into whom Slack has yet to
/// say anything of: Write in Slack's place held, dimmed, while it's asked.
#[must_use]
pub fn looking(email: &str, words: &Words) -> Value {
    json!({"email": email, "known": false, "acts": [placeholder(&words.looking)]})
}

/// What's said of someone Slack was being asked about once it couldn't
/// answer: nothing held in Write in Slack's place, since no answer is coming.
#[must_use]
pub fn unanswered(email: &str) -> Value {
    json!({"email": email, "known": false, "acts": []})
}

/// Write in Slack's place, dimmed and doing nothing, since it has no form,
/// saying why at its side.
fn placeholder(side: &str) -> Value {
    json!({"key": WRITE, "side": side, "gone": true, "main": true})
}

/// Whether Joel wrote the newest message of their conversation.
fn wrote_last(found: &Found) -> bool {
    found
        .conversation
        .as_ref()
        .and_then(|talk| talk.messages.last())
        .is_some_and(|newest| newest.mine)
}

/// Whether Joel owes their conversation an answer: something was said, and
/// they said it last.
fn owed(found: &Found) -> bool {
    found
        .conversation
        .as_ref()
        .is_some_and(|talk| !talk.messages.is_empty())
        && !wrote_last(found)
}

/// Their direct messages with Joel, while Slack finds any: one row,
/// opening it in Slack, then copying its link where it has one, and its
/// drawer, their last few messages, newest last, each after who said it.
fn between(found: &Found, words: &Words) -> Vec<Value> {
    let (Some(_), Some(talk)) = (&found.id, &found.conversation) else {
        return Vec::new();
    };
    let name = if talk.last.is_empty() {
        &words.conversation
    } else {
        &talk.last
    };
    let opens = format!("slack://channel?team={}&id={}", found.team, talk.channel);
    let mut acts = vec![open("verb:slack:open", &opens)];
    if !talk.link.is_empty() {
        acts.push(copy("verb:slack:link", &talk.link));
    }
    let body = talk
        .messages
        .iter()
        .map(|one| {
            let who = if one.mine { &words.you } else { &found.name };
            format!("{who}: {}", one.text)
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let mine = wrote_last(found);
    let mut thing = json!({
        "at": talk.at,
        "key": format!("slack:conversation:{}", talk.channel),
        "kind": "wrote",
        "mine": mine,
        "what": words.source,
        "row": {
            "name": name,
            "glyph": words.glyph,
            "hue": "program",
            "side": [{"said": words.source}, {"at": talk.at, "as": "ago"}],
            "acts": acts,
            "drawer": {"title": found.name, "facts": [], "body": body},
        },
    });
    if !talk.messages.is_empty() {
        thing["waits"] = if mine {
            json!({"until": talk.at + AWAITED})
        } else {
            json!({})
        };
    }
    vec![thing]
}

/// Their drawer's facts, each only while Slack says it: their title,
/// which says who they are, first, under their name as People's drawer
/// heads it (VKCF3Z, 0cea8a93), found there by the People word it says it
/// is, `wordsId`, not by its label (Y2YBTS, 2026-11-12); then status,
/// presence and time zone; or, for an address Slack knows nobody by, that
/// it knows no one there, where the status would stand, since the two
/// never show together.
fn facts(found: &Found, words: &Words) -> Vec<Value> {
    if found.id.is_none() {
        return vec![json!({"name": words.source, "said": words.missing})];
    }
    let status = [found.emoji.as_str(), found.status.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut facts = Vec::new();
    if !found.title.is_empty() {
        facts.push(json!({"name": words.title, "said": found.title, "wordsId": TITLE}));
    }
    if !status.is_empty() {
        facts.push(json!({"name": words.status, "said": status}));
    }
    if !found.presence.is_empty() {
        facts.push(json!({"name": words.presence, "said": capitalised(&found.presence)}));
    }
    if !found.zone.is_empty() {
        facts.push(json!({"name": words.zone, "zone": found.zone, "as": "zone"}));
    }
    facts
}

/// Their one way to reach them through Slack, first of all (`main`): Reply
/// in Slack while Joel owes their conversation an answer, else Write in
/// Slack, opening them where Slack shows them; for an address Slack knows
/// nobody by, its place held saying so where Joel stepped in, and nothing
/// where it was asked ahead. Its name, verb and glyph are words.json's, by
/// its key.
fn acts(found: &Found, words: &Words, asked: Asked) -> Vec<Value> {
    let Some(id) = &found.id else {
        return match asked {
            Asked::SteppedIn => vec![placeholder(&words.missing)],
            Asked::Ahead => Vec::new(),
        };
    };
    let mut act = match &found.conversation {
        Some(talk) if owed(found) => open(
            REPLY,
            &format!("slack://channel?team={}&id={}", found.team, talk.channel),
        ),
        _ => open(WRITE, &format!("slack://user?team={}&id={id}", found.team)),
    };
    act["main"] = json!(true);
    vec![act]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_rows::launchers_vocabulary;
    use crate::{Conversation, Message};

    fn words() -> Words {
        Words::read(&launchers_vocabulary()).expect("the launcher's words say a person")
    }

    fn ada() -> Found {
        Found {
            email: "ada@example.com".into(),
            id: Some("U1".into()),
            team: "T1".into(),
            name: "Ada L".into(),
            zone: "Europe/London".into(),
            presence: "in a huddle".into(),
            status: "at lunch".into(),
            emoji: "\u{1f96a}".into(),
            handle: "ada".into(),
            title: "Analyst".into(),
            conversation: None,
        }
    }

    /// Ada, with a conversation whose newest message `mine` says who wrote.
    fn talking(mine: bool) -> Found {
        Found {
            conversation: Some(Conversation {
                channel: "D4".into(),
                link: "https://slack.example/archives/D4/p2".into(),
                last: "See you there".into(),
                at: 2_000,
                messages: vec![
                    Message {
                        mine: !mine,
                        text: "Lunch?".into(),
                        at: 1_000,
                    },
                    Message {
                        mine,
                        text: "See you there".into(),
                        at: 2_000,
                    },
                ],
            }),
            ..ada()
        }
    }

    #[test]
    fn someone_found_has_their_facts_worded_and_their_zone_left_to_the_clock() {
        let said = ready(&ada(), &words(), Asked::Ahead);
        assert_eq!(
            said["facts"],
            json!([
                {"name": "Title", "said": "Analyst", "wordsId": "people:title"},
                {"name": "Status", "said": "\u{1f96a} at lunch"},
                {"name": "Presence", "said": "In a huddle"},
                {"name": "Time Zone", "zone": "Europe/London", "as": "zone"}
            ])
        );
        // What the daemon reads of how they stand stays as it was.
        assert_eq!(
            (
                &said["id"],
                &said["zone"],
                &said["presence"],
                &said["known"]
            ),
            (
                &json!("U1"),
                &json!("Europe/London"),
                &json!("in a huddle"),
                &Value::Null
            )
        );
    }

    #[test]
    fn a_fact_slack_doesnt_say_is_left_out() {
        let quiet = Found {
            title: String::new(),
            presence: String::new(),
            status: String::new(),
            emoji: String::new(),
            ..ada()
        };
        assert_eq!(
            ready(&quiet, &words(), Asked::Ahead)["facts"],
            json!([{"name": "Time Zone", "zone": "Europe/London", "as": "zone"}])
        );
    }

    #[test]
    fn write_in_slack_opens_them_there_first_of_their_ways() {
        assert_eq!(
            ready(&ada(), &words(), Asked::Ahead)["acts"],
            json!([{"key": WRITE, "open": "slack://user?team=T1&id=U1", "main": true}])
        );
    }

    #[test]
    fn reply_in_slack_opens_their_conversation_while_joel_owes_it_an_answer() {
        assert_eq!(
            ready(&talking(false), &words(), Asked::Ahead)["acts"],
            json!([{"key": REPLY, "open": "slack://channel?team=T1&id=D4", "main": true}])
        );
        assert_eq!(
            ready(&talking(true), &words(), Asked::Ahead)["acts"][0]["key"],
            WRITE,
            "his own last is no answer owed"
        );
    }

    #[test]
    fn an_address_slack_knows_nobody_by_says_so_where_joel_stepped_in_alone() {
        let nobody = Found { id: None, ..ada() };
        let stepped = ready(&nobody, &words(), Asked::SteppedIn);
        assert_eq!(
            (&stepped["acts"], &stepped["known"]),
            (
                &json!([{"key": WRITE, "side": "no one at this address", "gone": true,
                    "main": true}]),
                &json!(false)
            )
        );
        assert_eq!(
            stepped["facts"],
            json!([{"name": "Slack", "said": "no one at this address"}])
        );
        assert_eq!(stepped["between"], json!([]));
        let ahead = ready(&nobody, &words(), Asked::Ahead);
        assert_eq!(
            (&ahead["acts"], &ahead["known"]),
            (&json!([]), &json!(false))
        );
    }

    #[test]
    fn write_in_slack_holds_its_place_while_slack_is_asked_and_none_once_it_cant_answer() {
        assert_eq!(
            looking("ada@example.com", &words()),
            json!({"email": "ada@example.com", "known": false, "acts": [
                {"key": WRITE, "side": "looking in Slack", "gone": true, "main": true}]})
        );
        assert_eq!(
            unanswered("ada@example.com"),
            json!({"email": "ada@example.com", "known": false, "acts": []})
        );
    }

    #[test]
    fn their_conversation_is_between_them_named_by_its_newest_words_with_its_messages_drawn() {
        assert_eq!(
            ready(&talking(false), &words(), Asked::Ahead)["between"],
            json!([{
                "at": 2_000,
                "key": "slack:conversation:D4",
                "kind": "wrote",
                "mine": false,
                "what": "Slack",
                "waits": {},
                "row": {
                    "name": "See you there",
                    "glyph": "chat",
                    "hue": "program",
                    "side": [{"said": "Slack"}, {"at": 2_000, "as": "ago"}],
                    "acts": [
                        {"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D4"},
                        {"key": "verb:slack:link", "copy": "https://slack.example/archives/D4/p2"}
                    ],
                    "drawer": {
                        "title": "Ada L",
                        "facts": [],
                        "body": "You: Lunch?\n\nAda L: See you there"
                    }
                }
            }])
        );
    }

    #[test]
    fn joels_own_last_waits_on_them_for_two_days() {
        let said = ready(&talking(true), &words(), Asked::Ahead);
        assert_eq!(
            (&said["between"][0]["mine"], &said["between"][0]["waits"]),
            (&json!(true), &json!({"until": 2_000 + AWAITED}))
        );
    }

    #[test]
    fn a_conversation_whose_newest_says_nothing_is_named_as_one_waits_on_nobody_and_copies_no_link()
    {
        let talking = Found {
            conversation: Some(Conversation {
                channel: "D4".into(),
                link: String::new(),
                last: String::new(),
                at: 2_000,
                messages: Vec::new(),
            }),
            ..ada()
        };
        let thing = &ready(&talking, &words(), Asked::Ahead)["between"][0];
        assert_eq!(thing["row"]["name"], "Conversation");
        assert_eq!(thing["waits"], Value::Null);
        assert_eq!(
            thing["row"]["acts"],
            json!([{"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D4"}])
        );
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let read = Words::read(&launchers_vocabulary()).expect("the launcher's words say a person");
        assert_eq!(
            (
                read.zone.as_str(),
                read.missing.as_str(),
                read.looking.as_str(),
                read.you.as_str()
            ),
            (
                "Time Zone",
                "no one at this address",
                "looking in Slack",
                "You"
            ),
            "a person's words, as the launcher says them"
        );
        let without = json!({"things": [{"id": "slack:person:status", "name": "Status"}]});
        assert_eq!(
            Words::read(&without),
            Err("the vocabulary has no slack:person:presence".to_owned())
        );
    }
}
