//! The agents' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words.json, the `agents` thing's `said`),
//! read rather than written again here, so a row says what the launcher
//! would and one word changes in one place.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

/// The agents' words, by their keys.
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
}

impl Said {
    /// The agents' words from the vocabulary's text; none where it names
    /// no agents.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        let vocabulary: Vocabulary = serde_json::from_str(text)?;
        let said = vocabulary
            .things
            .into_iter()
            .find(|thing| thing.id == "agents")
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

    /// Words given outright, as a test gives only those it reads.
    pub fn of(words: &[(&str, &str)]) -> Self {
        Self(
            words
                .iter()
                .map(|(key, word)| ((*key).to_owned(), (*word).to_owned()))
                .collect(),
        )
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
    }
}
