//! Quests' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words/launcher.json, the `quests` thing's
//! `said`),
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
    /// it lacks, as the launcher says nothing for a word it lacks. The
    /// words are read once, left to right, so a value holding a part's
    /// name, as an ask's own words may, is said as it is and never filled.
    #[must_use]
    pub fn fill(&self, key: &str, parts: &[(&str, &str)]) -> String {
        let mut rest = self.0.get(key).map_or("", String::as_str);
        let mut said = String::with_capacity(rest.len());
        while let Some(open) = rest.find('{') {
            said.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let value = after.find('}').and_then(|close| {
                let name = &after[..close];
                let (_, value) = parts.iter().find(|(part, _)| *part == name)?;
                Some((*value, close))
            });
            if let Some((value, close)) = value {
                said.push_str(value);
                rest = &after[close + 1..];
            } else {
                said.push('{');
                rest = after;
            }
        }
        said.push_str(rest);
        said
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_naming_a_part_is_said_as_it_is() {
        let words = Words::of(&[("more", "landed: {words}, and {more} more")]);
        assert_eq!(
            words.fill("more", &[("words", "fix {more}"), ("more", "2")]),
            "landed: fix {more}, and 2 more",
            "an ask's words are never filled"
        );
    }

    #[test]
    fn quests_words_come_from_the_vocabulary() {
        let words = Words::from_vocabulary(
            r#"{"things": [{"id": "agents", "said": {"open": "no"}},
                {"id": "quests", "said": {"open": "{open} open", "nested": {"no": "no"}}}]}"#,
        )
        .expect("the vocabulary reads");
        assert_eq!(words.fill("open", &[("open", "3")]), "3 open");
        assert_eq!(
            words.fill("open", &[]),
            "{open} open",
            "a part not given stays"
        );
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
        let moved = [
            "moved-landed",
            "moved-landed-more",
            "moved-dated",
            "moved-taken",
            "moved-next",
        ];
        for key in ["empty", "open", "driving"].into_iter().chain(moved) {
            assert_ne!(
                words.fill(key, &[]),
                "",
                "core's vocabulary lacks Quests' {key}"
            );
        }
    }
}
