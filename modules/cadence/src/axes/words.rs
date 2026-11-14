//! The launcher's words for what Spec says in fixed words, read from the
//! words.json every daemon is handed by store path, so a word changes in
//! one place and the-launcher-means-one-thing-a-word holds it. A reading's
//! own sentence and an owner's quest title are data, served as said; only
//! the frame around them (Y2YBTS, 2026-11-14, 6632bb36) is looked up here.

use std::collections::BTreeMap;

use serde_json::Value;

/// The vocabulary's things by id.
#[derive(Clone, Debug, Default)]
pub struct Words {
    things: BTreeMap<String, Value>,
}

impl Words {
    /// The things of a words.json's `things`, each by its id.
    #[must_use]
    pub fn new(vocabulary: &Value) -> Words {
        let things = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thing| Some((thing["id"].as_str()?.to_owned(), thing.clone())))
            .collect();
        Words { things }
    }

    /// A thing's name, or its id where it has none, so a word missing
    /// from the vocabulary still shows as something rather than nothing.
    #[must_use]
    pub fn name(&self, id: &str) -> String {
        self.things
            .get(id)
            .and_then(|thing| thing["name"].as_str())
            .unwrap_or(id)
            .to_owned()
    }

    /// A phrase a thing says, its braces filled, or `id:phrase` where it
    /// says none, for the same reason.
    #[must_use]
    pub fn said(&self, id: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        let Some(said) = self
            .things
            .get(id)
            .and_then(|thing| thing["said"][phrase].as_str())
        else {
            return format!("{id}:{phrase}");
        };
        fill.iter().fold(said.to_owned(), |said, (name, value)| {
            said.replace(&format!("{{{name}}}"), value)
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_thing_is_said_by_its_id_and_a_missing_one_by_its_id() {
        let words = Words::new(&json!({"things": [
            {"id": "spec", "name": "Spec", "said": {"head": "{count} out of spec"}}
        ]}));
        assert_eq!(words.name("spec"), "Spec");
        assert_eq!(
            words.said("spec", "head", &[("count", "13")]),
            "13 out of spec"
        );
        assert_eq!(words.name("spec:gone"), "spec:gone");
        assert_eq!(words.said("spec", "none", &[]), "spec:none");
    }
}
