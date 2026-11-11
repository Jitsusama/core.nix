//! The notices' words, as the launcher's vocabulary says them
//! (modules/quickshell/launcher/words.json), read rather than written
//! again here, so a row says what the launcher would and a word changes in
//! one place: each thing's name and glyph, and what it says, in each of
//! its states, as the shell's `Words.name`, `Words.glyph` and `Words.of`
//! read them.

use std::collections::BTreeMap;

use serde_json::Value;

/// Every thing in the vocabulary, by its id.
#[derive(Debug, Default)]
pub struct Words(BTreeMap<String, Value>);

impl Words {
    /// The vocabulary's things from its text; an error where it isn't JSON.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        let vocabulary: Value = serde_json::from_str(text)?;
        let things = vocabulary["things"].as_array().into_iter().flatten();
        Ok(Self(
            things
                .filter_map(|thing| Some((thing["id"].as_str()?.to_owned(), thing.clone())))
                .collect(),
        ))
    }

    /// A field of a thing in a state, else of the thing itself, as a word;
    /// "" where neither says one.
    fn field(&self, id: &str, state: &str, field: &str) -> &str {
        let Some(thing) = self.0.get(id) else {
            return "";
        };
        thing["states"][state][field]
            .as_str()
            .or_else(|| thing[field].as_str())
            .unwrap_or("")
    }

    /// A thing's name, in a state where it has one ("" for none).
    pub fn name(&self, id: &str, state: &str) -> &str {
        self.field(id, state, "name")
    }

    /// A thing's glyph role, in a state where it has one.
    pub fn glyph(&self, id: &str, state: &str) -> &str {
        self.field(id, state, "glyph")
    }

    /// What a thing says in a state, its `said` there.
    pub fn state_said(&self, id: &str, state: &str) -> &str {
        self.0
            .get(id)
            .and_then(|thing| thing["states"][state]["said"].as_str())
            .unwrap_or("")
    }

    /// One of a thing's words, its `said` by key.
    pub fn said(&self, id: &str, key: &str) -> &str {
        self.0
            .get(id)
            .and_then(|thing| thing["said"][key].as_str())
            .unwrap_or("")
    }

    /// The `said` of each of a thing's states, by state.
    pub fn states_said(&self, id: &str) -> BTreeMap<&str, &str> {
        self.0
            .get(id)
            .and_then(|thing| thing["states"].as_object())
            .into_iter()
            .flatten()
            .filter_map(|(state, said)| Some((state.as_str(), said["said"].as_str()?)))
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The launcher's own words, so a case reads as the launcher says it.
    pub(crate) fn core() -> Words {
        Words::from_vocabulary(include_str!("../../quickshell/launcher/words.json"))
            .expect("core's vocabulary reads")
    }

    #[test]
    fn a_thing_is_said_in_its_state_else_as_itself() {
        let words = core();
        assert_eq!(words.name("verb:notice:read", ""), "Mark It Read");
        assert_eq!(words.name("verb:notice:read", "read"), "Mark It Unread");
        assert_eq!(words.glyph("verb:notice:read", "read"), "unread");
        assert_eq!(words.glyph("verb:notice:read", "absent"), "read");
        assert_eq!(words.said("notices", "from"), "from {who}");
        assert_eq!(words.state_said("notices:urgency", "low"), "low");
        assert_eq!(
            words.states_said("notices:kind").get("im"),
            Some(&"message")
        );
        assert_eq!(
            words.name("absent", ""),
            "",
            "a thing it lacks says nothing"
        );
    }
}
