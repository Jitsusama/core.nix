//! The agents' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words.json, a thing's `said`: `agents`'s,
//! and `agents:machine`'s and `agents:wants`' for a session elsewhere and
//! what it wants),
//! read rather than written again here, so a row says what the launcher
//! would and one word changes in one place.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

/// The launcher's own words for what this crate's tests say, the entries
/// they read copied whole from its vocabulary (tests/words.json), which
/// the-daemons-test-the-launchers-words holds to words.json, so no test
/// passes on words Joel never reads: Machine's had drifted in 11 places
/// (#3367).
#[cfg(test)]
pub const LAUNCHERS: &str = include_str!("../tests/words.json");

/// One thing's words, by their keys.
#[derive(Debug, Default)]
pub struct Said(HashMap<String, String>);

#[derive(Deserialize)]
struct Vocabulary {
    things: Vec<Thing>,
}

#[derive(Deserialize)]
struct Thing {
    #[serde(default)]
    id: String,
    /// Read loosely, since other things' words nest further than the
    /// agents' do.
    #[serde(default)]
    said: Value,
    /// Its name, where it's a word; a name by state is read as none.
    #[serde(default)]
    name: Value,
    /// Its mark's name, where it has one.
    #[serde(default)]
    glyph: Value,
}

impl Said {
    /// The agents' words from the vocabulary's text; none where it names
    /// no agents.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        Self::of_thing(text, "agents")
    }

    /// The words of the thing going by `id`; none where there's none.
    pub fn of_thing(text: &str, id: &str) -> serde_json::Result<Self> {
        let vocabulary: Vocabulary = serde_json::from_str(text)?;
        let said = vocabulary
            .things
            .into_iter()
            .find(|thing| thing.id == id)
            .and_then(|thing| match thing.said {
                Value::Object(words) => Some(words),
                _ => None,
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(key, word)| match word {
                Value::String(word) => Some((key, word)),
                _ => None,
            })
            .collect();
        Ok(Self(said))
    }

    /// The names of every thing whose id starts `{prefix}:`, by the rest
    /// of its id, as a drawer names each fact (`agents:step` is Doing).
    pub fn names_under(text: &str, prefix: &str) -> serde_json::Result<Self> {
        Self::under(text, prefix, |thing| thing.name)
    }

    /// The marks of every thing whose id starts `{prefix}:`, by the rest
    /// of its id, as an act's row is marked (`verb:agent:talk` is reply).
    pub fn glyphs_under(text: &str, prefix: &str) -> serde_json::Result<Self> {
        Self::under(text, prefix, |thing| thing.glyph)
    }

    fn under(text: &str, prefix: &str, said: fn(Thing) -> Value) -> serde_json::Result<Self> {
        let vocabulary: Vocabulary = serde_json::from_str(text)?;
        let lead = format!("{prefix}:");
        let words = vocabulary
            .things
            .into_iter()
            .filter_map(|thing| {
                let id = thing.id.strip_prefix(&lead)?.to_owned();
                match said(thing) {
                    Value::String(word) => Some((id, word)),
                    _ => None,
                }
            })
            .collect();
        Ok(Self(words))
    }

    /// The words a key says, or none, as the launcher says nothing for
    /// a word it lacks.
    pub fn word(&self, key: &str) -> &str {
        self.0.get(key).map_or("", String::as_str)
    }

    /// The words a key says with each `{part}` filled, the first of each
    /// as the launcher's own `replace` fills them.
    pub fn fill(&self, key: &str, parts: &[(&str, &str)]) -> String {
        fill(self.word(key), parts)
    }
}

/// Words with each `{part}` filled, the first of each.
pub fn fill(words: &str, parts: &[(&str, &str)]) -> String {
    parts.iter().fold(words.to_owned(), |text, (part, value)| {
        text.replacen(&format!("{{{part}}}"), value, 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agents_words_come_from_the_vocabulary() {
        let said = Said::from_vocabulary(
            r#"{"things": [{"id": "quests", "said": {"today": {"nested": "no"}}},
                {"id": "agents", "said": {"today": "{spent} today"}}]}"#,
        )
        .expect("the vocabulary reads");
        assert_eq!(said.fill("today", &[("spent", "$4")]), "$4 today");
        assert_eq!(said.word("absent"), "", "a word it lacks says nothing");
    }

    #[test]
    fn the_names_under_a_thing_are_kept_by_the_rest_of_their_ids() {
        let names = Said::names_under(
            r#"{"things": [{"id": "agents:step", "name": "Doing"},
                {"id": "agents", "name": "Agents"},
                {"id": "agentsish:step", "name": "Not this"},
                {"id": "agents:by-state", "name": {"on": "On"}}]}"#,
            "agents",
        )
        .expect("the vocabulary reads");
        assert_eq!(names.word("step"), "Doing");
        assert_eq!(names.word("by-state"), "", "a name by state is none");
        assert_eq!(names.word(""), "", "the thing itself isn't under itself");
    }

    #[test]
    fn a_vocabulary_without_agents_says_nothing() {
        let said = Said::from_vocabulary(r#"{"things": [{"id": "quests"}]}"#)
            .expect("the vocabulary reads");
        assert_eq!(said.word("today"), "");
    }

    #[test]
    fn the_core_vocabulary_names_every_word_a_row_says() {
        let said = Said::from_vocabulary(include_str!("../../quickshell/launcher/words.json"))
            .expect("core's vocabulary reads");
        for key in [
            "today",
            "needCount",
            "needsCount",
            "workingCount",
            "full",
            "under",
        ] {
            assert_ne!(
                said.word(key),
                "",
                "core's vocabulary lacks the agents' {key}"
            );
        }
        let text = include_str!("../../quickshell/launcher/words.json");
        for (thing, key) in [
            ("agents:machine", "on"),
            ("agents:machine", "away"),
            ("agents:wants", "question"),
        ] {
            let said = Said::of_thing(text, thing).expect("core's vocabulary reads");
            assert_ne!(
                said.word(key),
                "",
                "core's vocabulary lacks {thing}'s {key}"
            );
        }
    }
}
