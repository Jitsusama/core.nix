//! Someone Slack found, made ready for People: what slack-people says of
//! them (lib.rs's `Found`) with their drawer's facts and their act beside
//! it, worded from the launcher's vocabulary, so the people daemon merges
//! Slack's part of a person as it is and no QML source words it
//! (Y2YBTS's data rule, 2026-11-10). The facts are on ready-rows.md's
//! forms: `{ name, said }`, and the time zone as `{ name, zone, as:
//! "zone" }`, unworded, since the time there reads the clock and the shell
//! says it as it ticks. How they stand is the people daemon's to word,
//! from Slack's `stands` in its sources file.

use serde_json::{Value, json};

use crate::Found;
use crate::message_rows::{capitalised, word};

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
    /// "not in Slack", said in Write in Slack's place for an address
    /// Slack knows nobody by.
    pub missing: String,
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
        })
    }
}

/// What slack-people says of someone: `found` as it was said before, its
/// fields unchanged for the shell's source while it still reads them,
/// with `facts` and `acts` beside them.
#[must_use]
pub fn ready(found: &Found, words: &Words) -> Value {
    let mut said = serde_json::to_value(found).unwrap_or_else(|_| json!({}));
    said["facts"] = Value::Array(facts(found, words));
    said["acts"] = Value::Array(acts(found, words));
    said
}

/// Their drawer's facts, each only while Slack says it, in the order
/// Slack.js gave them: status, presence, then time zone.
fn facts(found: &Found, words: &Words) -> Vec<Value> {
    if found.id.is_none() {
        return Vec::new();
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

/// Write in Slack: opening them where Slack shows them, or, for an
/// address Slack knows nobody by, dimmed and saying so, with nothing to
/// ask. Its name, verb and glyph are words.json's, by its key.
fn acts(found: &Found, words: &Words) -> Vec<Value> {
    let act = match &found.id {
        Some(id) => json!({
            "key": WRITE,
            "ask": {"open": {"link": format!("slack://user?team={}&id={id}", found.team)}},
        }),
        None => json!({"key": WRITE, "gone": true, "side": [{"said": words.missing}]}),
    };
    vec![act]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words {
            status: "Status".into(),
            presence: "Presence".into(),
            zone: "Time Zone".into(),
            missing: "not in Slack".into(),
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
    fn write_in_slack_opens_them_there_or_says_slack_knows_nobody() {
        assert_eq!(
            ready(&ada(), &words())["acts"],
            json!([{"key": WRITE, "ask": {"open": {"link": "slack://user?team=T1&id=U1"}}}])
        );
        let nobody = Found { id: None, ..ada() };
        let said = ready(&nobody, &words());
        assert_eq!(
            said["acts"],
            json!([{"key": WRITE, "gone": true, "side": [{"said": "not in Slack"}]}])
        );
        assert_eq!(said["facts"], json!([]));
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let vocabulary = json!({"things": [
            {"id": "slack:person:status", "name": "Status"},
            {"id": "slack:person:presence", "name": "Presence"},
            {"id": "slack:person:zone", "name": "Time Zone"},
            {"id": WRITE, "said": {"looking": "looking in Slack", "missing": "not in Slack"}}
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
