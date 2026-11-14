//! The safe word's words in the launcher, as its vocabulary says them
//! (modules/quickshell/launcher/words/on.json, the `said` of For You's
//! source `for-you:source:pineapple`), read rather than written again
//! here, so a word changes in one place, as secret-asks reads Asks'.

use std::collections::BTreeMap;

use serde_json::Value;

/// The thing in the vocabulary the safe word's words are under.
const THING: &str = "for-you:source:pineapple";

/// The safe word's words, by their keys.
#[derive(Debug, Default)]
pub struct Words(BTreeMap<String, String>);

impl Words {
    /// The words from the vocabulary's text; none where it names none,
    /// and an error where it isn't JSON.
    ///
    /// # Errors
    ///
    /// Where the text isn't JSON.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        let vocabulary: Value = serde_json::from_str(text)?;
        let said = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|thing| thing["id"] == THING)
            .filter_map(|thing| thing["said"].as_object())
            .flatten()
            .filter_map(|(key, word)| Some((key.clone(), word.as_str()?.to_owned())));
        Ok(Self(said.collect()))
    }

    /// The words an object of strings gives by its keys, as the replier's
    /// own file (replies.json) does; a value that isn't a string is none.
    ///
    /// # Errors
    ///
    /// Where the text isn't JSON.
    pub fn from_object(text: &str) -> serde_json::Result<Self> {
        let object: Value = serde_json::from_str(text)?;
        let words = object
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(key, word)| Some((key.clone(), word.as_str()?.to_owned())));
        Ok(Self(words.collect()))
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
