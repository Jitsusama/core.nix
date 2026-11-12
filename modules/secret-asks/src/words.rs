//! Asks' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words.json, the `asks` thing's `said`),
//! read rather than written again here, so a word changes in one place,
//! as the quests daemon reads Quests' (modules/quests/src/words.rs).

use std::collections::BTreeMap;

use serde_json::Value;

/// The thing in the vocabulary Asks' words are under.
const THING: &str = "asks";

/// Asks' words, by their keys.
#[derive(Debug, Default)]
pub struct Words(BTreeMap<String, String>);

impl Words {
    /// Asks' words from the vocabulary's text; none where it names no
    /// asks, and an error where it isn't JSON.
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

    /// Words given outright, as a test gives them.
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
