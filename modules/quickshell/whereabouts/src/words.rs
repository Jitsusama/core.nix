//! The words Location is said in, from the launcher's vocabulary
//! (launcher/words.json), handed to the daemon by store path, so its rows
//! reach the launcher already worded and the shell words none of them
//! (Joel, 2026-11-10: "the daemons need to be the data layer, and they
//! need to serve the launcher"). Location's own words are `location`'s:
//! its name, its phrases (`said`) and its sentences (`tells`); each of its
//! rows and facts is `location:<id>`, and each act `verb:location:<id>`.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};

/// Location's rows and its plate's facts, by their ids.
pub const NAMES: [&str; 10] = [
    "town",
    "country",
    "knows",
    "located",
    "accuracy",
    "coordinates",
    "altitude",
    "speed",
    "heading",
    "zone",
];

/// The acts on Location, by their ids.
pub const ACTS: [&str; 3] = ["set", "again", "copy"];

/// An act as the launcher names it, and what it asks Joel, where it asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Act {
    pub name: String,
    pub verb: String,
    pub glyph: String,
    pub asks: Option<String>,
}

/// Location's words.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    pub name: String,
    said: Map<String, Value>,
    tells: Map<String, Value>,
    names: HashMap<String, String>,
    acts: HashMap<String, Act>,
}

impl Words {
    /// Location's words from the vocabulary at `path`, or none, said on
    /// the journal, so here is still served, though no rows are.
    pub fn in_file(path: &Path) -> Option<Words> {
        let read = std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
            .and_then(|vocabulary| Words::read(&vocabulary));
        read.map_err(|why| eprintln!("<4>Location's rows go unserved: {why}"))
            .ok()
    }

    /// Location's words from the whole vocabulary, or the first it lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let things = vocabulary["things"]
            .as_array()
            .ok_or("the vocabulary has no things")?;
        let thing = |id: &str| {
            things
                .iter()
                .find(|thing| thing["id"] == id)
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let text = |thing: &Value, field: &str| {
            thing[field].as_str().map(str::to_owned).ok_or_else(|| {
                format!(
                    "{} has no {field}",
                    thing["id"].as_str().unwrap_or_default()
                )
            })
        };
        let location = thing("location")?;
        let mut words = Words {
            name: text(location, "name")?,
            said: location["said"].as_object().cloned().unwrap_or_default(),
            tells: location["tells"].as_object().cloned().unwrap_or_default(),
            ..Words::default()
        };
        for id in NAMES {
            let named = thing(&format!("location:{id}"))?;
            words.names.insert(id.to_owned(), text(named, "name")?);
        }
        for id in ACTS {
            let named = thing(&format!("verb:location:{id}"))?;
            let act = Act {
                name: text(named, "name")?,
                verb: text(named, "verb")?,
                glyph: text(named, "glyph")?,
                asks: named["ask"].as_str().map(str::to_owned),
            };
            words.acts.insert(id.to_owned(), act);
        }
        Ok(words)
    }

    /// One of Location's phrases, its blanks filled: the id itself where
    /// the vocabulary has none, as the launcher's `Words.of` leaves it.
    pub fn said(&self, id: &str, blanks: &[(&str, &str)]) -> String {
        let words = self.said.get(id).and_then(Value::as_str).unwrap_or(id);
        filled(words, blanks)
    }

    /// Whether the vocabulary has a phrase of this id, as a source's own
    /// eyebrow is only where its words say it.
    pub fn has(&self, id: &str) -> bool {
        self.said.get(id).is_some_and(Value::is_string)
    }

    /// One of Location's sentences, its blanks filled, or none.
    pub fn tells(&self, id: &str, blanks: &[(&str, &str)]) -> Option<String> {
        self.tells
            .get(id)
            .and_then(Value::as_str)
            .map(|words| filled(words, blanks))
    }

    /// A row's or a fact's name, by its id in `NAMES`.
    pub fn name(&self, id: &str) -> &str {
        self.names.get(id).map_or("", String::as_str)
    }

    /// An act's names, by its id in `ACTS`.
    pub fn act(&self, id: &str) -> Act {
        self.acts.get(id).cloned().unwrap_or_default()
    }
}

/// Location's words as the launcher has them, copied beside the tests so
/// the daemon isn't built again for every word the launcher changes.
#[cfg(test)]
pub fn fixture() -> Words {
    let vocabulary = serde_json::from_str(include_str!("../tests/words.json"));
    Words::read(&vocabulary.expect("the words parse")).expect("every word Location needs is there")
}

/// A phrase with each blank, `{name}`, filled, the first of each name, as
/// JavaScript's `replace` fills it.
fn filled(words: &str, blanks: &[(&str, &str)]) -> String {
    let mut words = words.to_owned();
    for (name, value) in blanks {
        words = words.replacen(&format!("{{{name}}}"), value, 1);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn locations_words_are_read_from_the_launchers_own() {
        let words = fixture();
        assert_eq!(words.name, "Location", "its name");
        assert_eq!(words.name("knows"), "How It Knows", "a row's name");
        assert_eq!(
            words.said("metres", &[("n", "80")]),
            "within 80 m",
            "a phrase with its blank filled"
        );
        assert_eq!(
            words.tells("leftOut-wifi", &[("missing", "speed")]),
            Some("Wi-Fi gives no speed.".to_owned()),
            "a sentence"
        );
        assert_eq!(
            words.act("set").asks.as_deref(),
            Some("where are you?"),
            "what an act asks"
        );
    }

    #[test]
    fn a_vocabulary_lacking_a_word_is_refused_by_what_it_lacks() {
        let refused = Words::read(&json!({"things": [{"id": "location", "name": "Location"}]}));
        assert_eq!(
            refused,
            Err("the vocabulary has no location:town".to_owned()),
            "the first word missing is named"
        );
    }
}
