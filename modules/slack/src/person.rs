//! Someone Slack found, made ready for People: what slack-people says of
//! them (lib.rs's `Found`) with their drawer's facts and their act beside
//! it, worded from the launcher's vocabulary, so the people daemon merges
//! Slack's part of a person as it is and no QML source words it
//! (Y2YBTS's data rule, 2026-11-10). The facts are on ready-rows.md's
//! forms: `{ name, said }`, and the time zone as `{ name, zone, as:
//! "zone" }`, unworded, since the time there reads the clock and the shell
//! says it as it ticks. How they stand is the people daemon's to word,
//! from Slack's `stands` in its sources file.
//!
//! Their direct messages with Joel are one conversation in their Recent,
//! a ready row with its drawer under `conversations`, which the people
//! daemon places among mail and meetings by its `at` (XY9H5C,
//! 2026-11-10): named by its newest message's first words, with "Slack"
//! and then how long ago beside it, a time the shell words, and its last
//! few messages in its drawer. Every act is on ready-rows.md's forms, one
//! of `{ key, ask }`, `{ key, open }` or `{ key, copy }` (Y2YBTS,
//! 2026-11-10).

use serde_json::{Value, json};

use crate::Found;
use crate::message_rows::{capitalised, copy, open, word};

/// The act that writes to them in Slack, as words.json keys it.
pub const WRITE: &str = "verb:person:slack";

/// The words a person's facts and act are said in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
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
            missing: word(vocabulary, WRITE, &["said", "missing"])?,
            conversation: word(vocabulary, "slack:conversation", &["name"])?,
            glyph: word(vocabulary, "slack:conversation", &["glyph"])?,
            source: word(vocabulary, "slack:person:source", &["name"])?,
            you: capitalised(&word(vocabulary, "slack:message", &["said", "you"])?),
        })
    }
}

/// What slack-people says of someone: `found` as it was said before, its
/// fields unchanged for the shell's source while it still reads them,
/// with `facts`, `acts` and `conversations` beside them.
#[must_use]
pub fn ready(found: &Found, words: &Words) -> Value {
    let mut said = serde_json::to_value(found).unwrap_or_else(|_| json!({}));
    said["facts"] = Value::Array(facts(found, words));
    said["acts"] = Value::Array(acts(found));
    said["conversations"] = Value::Array(conversations(found, words));
    said
}

/// Their direct messages with Joel, while Slack finds any: one row,
/// opening it in Slack, then copying its link where it has one, and its
/// drawer, their last few messages, newest last, each after who said it.
fn conversations(found: &Found, words: &Words) -> Vec<Value> {
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
    vec![json!({
        "at": talk.at,
        "key": format!("slack:conversation:{}", talk.channel),
        "row": {
            "name": name,
            "glyph": words.glyph,
            "hue": "program",
            "side": [{"said": words.source}, {"at": talk.at, "as": "ago"}],
            "acts": acts,
        },
        "drawer": {"title": found.name, "facts": [], "body": body},
    })]
}

/// Their drawer's facts, each only while Slack says it, in the order
/// Slack.js gave them: status, presence, then time zone; or, for an
/// address Slack knows nobody by, that it knows no one there, where the
/// status would stand, since the two never show together.
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

/// Write in Slack, opening them where Slack shows them, for someone
/// Slack found; none for an address it knows nobody by, whose facts say
/// so. Its name, verb and glyph are words.json's, by its key.
fn acts(found: &Found) -> Vec<Value> {
    found
        .id
        .iter()
        .map(|id| open(WRITE, &format!("slack://user?team={}&id={id}", found.team)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Conversation, Message};

    fn words() -> Words {
        Words {
            status: "Status".into(),
            presence: "Presence".into(),
            zone: "Time Zone".into(),
            missing: "no one at this address".into(),
            conversation: "Conversation".into(),
            glyph: "chat".into(),
            source: "Slack".into(),
            you: "You".into(),
        }
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
            conversation: None,
        }
    }

    #[test]
    fn someone_found_has_their_facts_worded_and_their_zone_left_to_the_clock() {
        let said = ready(&ada(), &words());
        assert_eq!(
            said["facts"],
            json!([
                {"name": "Status", "said": "\u{1f96a} at lunch"},
                {"name": "Presence", "said": "In a huddle"},
                {"name": "Time Zone", "zone": "Europe/London", "as": "zone"}
            ])
        );
        // What the shell's source reads today stays as it was.
        assert_eq!(
            (&said["id"], &said["zone"], &said["presence"]),
            (&json!("U1"), &json!("Europe/London"), &json!("in a huddle"))
        );
    }

    #[test]
    fn a_fact_slack_doesnt_say_is_left_out() {
        let quiet = Found {
            presence: String::new(),
            status: String::new(),
            emoji: String::new(),
            ..ada()
        };
        assert_eq!(
            ready(&quiet, &words())["facts"],
            json!([{"name": "Time Zone", "zone": "Europe/London", "as": "zone"}])
        );
    }

    #[test]
    fn write_in_slack_opens_them_there_or_a_fact_says_slack_knows_nobody() {
        assert_eq!(
            ready(&ada(), &words())["acts"],
            json!([{"key": WRITE, "open": "slack://user?team=T1&id=U1"}])
        );
        let nobody = Found { id: None, ..ada() };
        let said = ready(&nobody, &words());
        assert_eq!(said["acts"], json!([]));
        assert_eq!(
            said["facts"],
            json!([{"name": "Slack", "said": "no one at this address"}])
        );
        assert_eq!(said["conversations"], json!([]));
    }

    #[test]
    fn their_conversation_is_a_row_named_by_its_newest_words_with_its_messages_in_its_drawer() {
        let talking = Found {
            conversation: Some(Conversation {
                channel: "D4".into(),
                link: "https://slack.example/archives/D4/p2".into(),
                last: "See you there".into(),
                at: 2_000,
                messages: vec![
                    Message {
                        mine: true,
                        text: "Lunch?".into(),
                        at: 1_000,
                    },
                    Message {
                        mine: false,
                        text: "See you there".into(),
                        at: 2_000,
                    },
                ],
            }),
            ..ada()
        };
        assert_eq!(
            ready(&talking, &words())["conversations"],
            json!([{
                "at": 2_000,
                "key": "slack:conversation:D4",
                "row": {
                    "name": "See you there",
                    "glyph": "chat",
                    "hue": "program",
                    "side": [{"said": "Slack"}, {"at": 2_000, "as": "ago"}],
                    "acts": [
                        {"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D4"},
                        {"key": "verb:slack:link", "copy": "https://slack.example/archives/D4/p2"}
                    ]
                },
                "drawer": {
                    "title": "Ada L",
                    "facts": [],
                    "body": "You: Lunch?\n\nAda L: See you there"
                }
            }])
        );
    }

    #[test]
    fn a_conversation_whose_newest_says_nothing_is_named_as_one_and_copies_no_link() {
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
        let row = &ready(&talking, &words())["conversations"][0]["row"];
        assert_eq!(row["name"], "Conversation");
        assert_eq!(
            row["acts"],
            json!([{"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D4"}])
        );
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let vocabulary = json!({"things": [
            {"id": "slack:person:status", "name": "Status"},
            {"id": "slack:person:presence", "name": "Presence"},
            {"id": "slack:person:zone", "name": "Time Zone"},
            {"id": WRITE, "said": {"looking": "looking in Slack", "missing": "no one at this address"}},
            {"id": "slack:conversation", "name": "Conversation", "glyph": "chat"},
            {"id": "slack:person:source", "name": "Slack"},
            {"id": "slack:message", "said": {"you": "you"}}
        ]});
        assert_eq!(Words::read(&vocabulary), Ok(words()));
        let without = json!({"things": [{"id": "slack:person:status", "name": "Status"}]});
        assert_eq!(
            Words::read(&without),
            Err("the vocabulary has no slack:person:presence".to_owned())
        );
    }

    #[test]
    fn the_launchers_own_vocabulary_has_every_word() {
        let path = std::env::var_os("LAUNCHER_WORDS").expect("LAUNCHER_WORDS names words.json");
        let text = std::fs::read_to_string(path).expect("words.json reads");
        let vocabulary: Value = serde_json::from_str(&text).expect("words.json is JSON");
        assert_eq!(Words::read(&vocabulary), Ok(words()));
    }
}
