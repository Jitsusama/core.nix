//! The words a to-do is said in, from the launcher's vocabulary
//! (modules/quickshell/launcher/words.json), handed to the daemon by
//! store path, so each to-do reaches For You already worded and the shell
//! words none of it (Joel, 2026-11-10: "the daemons need to be the data
//! layer, and they need to serve the launcher"). For You's own words are
//! `on:waiting`'s: its phrases (`said`) and its sentences (`tells`); each
//! fact a to-do's drawer names is `on:waiting:<id>`, and each act on one
//! `verb:on:waiting:<id>`.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};

/// The facts of a to-do's drawer this daemon says, by their ids.
pub const FACTS: [&str; 10] = [
    "due", "set-for", "takes", "since", "checked", "clears", "machine", "address", "check", "item",
];

/// The acts on a to-do this daemon answers, by their ids.
pub const ACTS: [&str; 2] = ["done", "notNow"];

/// An act as the launcher names it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Act {
    pub name: String,
    pub verb: String,
    pub glyph: String,
}

/// For You's words, as far as a to-do needs them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    said: Map<String, Value>,
    tells: Map<String, Value>,
    facts: HashMap<String, String>,
    acts: HashMap<String, Act>,
}

impl Words {
    /// For You's words from the vocabulary at `path`, or none, said on the
    /// journal, so the to-dos are served unworded rather than not at all.
    pub fn in_file(path: &Path) -> Option<Words> {
        let read = std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
            .and_then(|vocabulary| Words::read(&vocabulary));
        read.map_err(|why| eprintln!("<4>the to-dos go unworded: {why}"))
            .ok()
    }

    /// For You's words from the whole vocabulary, or the first it lacks.
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
        let waiting = thing("on:waiting")?;
        let mut words = Words {
            said: waiting["said"].as_object().cloned().unwrap_or_default(),
            tells: waiting["tells"].as_object().cloned().unwrap_or_default(),
            ..Words::default()
        };
        for fact in FACTS {
            let named = thing(&format!("on:waiting:{fact}"))?;
            words.facts.insert(fact.to_owned(), text(named, "name")?);
        }
        for act in ACTS {
            let named = thing(&format!("verb:on:waiting:{act}"))?;
            words.acts.insert(
                act.to_owned(),
                Act {
                    name: text(named, "name")?,
                    verb: text(named, "verb")?,
                    glyph: text(named, "glyph")?,
                },
            );
        }
        Ok(words)
    }

    /// One of For You's phrases, its blanks filled, as Now.qml's
    /// `waitingSaid` fills it: the id itself where the vocabulary has none.
    pub fn said(&self, id: &str, blanks: &[(&str, &str)]) -> String {
        let words = self.said.get(id).and_then(Value::as_str).unwrap_or(id);
        filled(words, blanks)
    }

    /// One of For You's phrases as written, its blanks left for whoever
    /// fills them, as a time's are by the shell's clock.
    pub fn phrase(&self, id: &str) -> String {
        self.said(id, &[])
    }

    /// One of For You's sentences, its blanks filled; none where the
    /// vocabulary has none.
    pub fn tells(&self, id: &str, blanks: &[(&str, &str)]) -> String {
        self.tells
            .get(id)
            .and_then(Value::as_str)
            .map_or_else(String::new, |words| filled(words, blanks))
    }

    /// A fact's name, by its id in `FACTS`.
    pub fn fact(&self, id: &str) -> &str {
        self.facts.get(id).map_or("", String::as_str)
    }

    /// An act's names, by its id in `ACTS`.
    pub fn act(&self, id: &str) -> Act {
        self.acts.get(id).cloned().unwrap_or_default()
    }
}

/// A phrase with each blank, `{name}`, filled, the first of each name.
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

    /// For You's words as the launcher has them, those a to-do needs.
    fn vocabulary() -> Value {
        serde_json::from_str(include_str!("../tests/words.json")).expect("the words parse")
    }

    #[test]
    fn for_yous_words_are_read_from_the_launchers_own() {
        let words = Words::read(&vocabulary()).expect("every word a to-do needs is there");
        assert_eq!(words.fact("due"), "Due", "a fact's name");
        assert_eq!(
            words.said("signin", &[("value", "Slack")]),
            "once signed in to Slack",
            "a phrase with its blank filled"
        );
        assert_eq!(
            words.phrase("waitedFor"),
            "waiting {span}",
            "a phrase whose blank the shell fills"
        );
        assert_eq!(
            words.tells("known", &[("how", "once its check passes")]),
            "Known done once its check passes.",
            "a sentence"
        );
        assert_eq!(
            words.act("notNow"),
            Act {
                name: "Put It Off".into(),
                verb: "later".into(),
                glyph: "queued".into()
            },
            "an act's names"
        );
    }

    #[test]
    fn a_vocabulary_lacking_a_word_is_refused_by_what_it_lacks() {
        let refused = Words::read(&json!({"things": [{"id": "on:waiting", "said": {}}]}));
        assert_eq!(
            refused,
            Err("the vocabulary has no on:waiting:due".to_owned()),
            "the first word missing is named"
        );
        assert!(
            Words::read(&json!({})).is_err(),
            "a vocabulary of no things is refused"
        );
    }
}
