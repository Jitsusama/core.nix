//! Quests' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words.json, the `quests` thing's `said`),
//! read rather than written again here, so the summary says what the
//! launcher would and a word changes in one place (Y2YBTS, 2026-11-11).

use std::collections::BTreeMap;

use serde_json::Value;

/// The thing in the vocabulary Quests' words are under.
const THING: &str = "quests";

/// Quests' words, by their keys.
#[derive(Debug, Default)]
pub struct Words(BTreeMap<String, String>);

impl Words {
    /// Quests' words from the vocabulary's text; none where it names no
    /// quests, and an error where it isn't JSON.
    ///
    /// # Errors
    ///
    /// Where the text isn't JSON.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        let vocabulary: Value = serde_json::from_str(text)?;
        let things = vocabulary["things"].as_array().into_iter().flatten();
        let said = things
            .filter(|thing| thing["id"] == THING)
            .filter_map(|thing| thing["said"].as_object())
            .flatten()
            .filter_map(|(key, word)| Some((key.clone(), word.as_str()?.to_owned())))
            .collect();
        Ok(Self(said))
    }

    /// Words given outright, as a test gives only those it reads.
    #[must_use]
    pub fn of(words: &[(&str, &str)]) -> Self {
        Self(
            words
                .iter()
                .map(|(key, word)| ((*key).to_owned(), (*word).to_owned()))
                .collect(),
        )
    }

    /// The words a key says with each `{part}` filled, or none for a key
    /// it lacks, as the launcher says nothing for a word it lacks.
    #[must_use]
    pub fn fill(&self, key: &str, parts: &[(&str, &str)]) -> String {
        let words = self.0.get(key).map_or("", String::as_str);
        parts.iter().fold(words.to_owned(), |text, (part, value)| {
            text.replacen(&format!("{{{part}}}"), value, 1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quests_words_come_from_the_vocabulary() {
        let words = Words::from_vocabulary(
            r#"{"things": [{"id": "agents", "said": {"open": "no"}},
                {"id": "quests", "said": {"open": "{open} open", "nested": {"no": "no"}}}]}"#,
        )
        .expect("the vocabulary reads");
        assert_eq!(words.fill("open", &[("open", "3")]), "3 open");
        assert_eq!(words.fill("nested", &[]), "", "only words are words");
        assert_eq!(
            words.fill("absent", &[]),
            "",
            "a word it lacks says nothing"
        );
    }

    #[test]
    fn the_core_vocabulary_names_every_word_the_summary_says() {
        let words = Words::from_vocabulary(include_str!("../../quickshell/launcher/words.json"))
            .expect("core's vocabulary reads");
        for key in ["empty", "open", "driving"] {
            assert_ne!(
                words.fill(key, &[]),
                "",
                "core's vocabulary lacks Quests' {key}"
            );
        }
    }
}
