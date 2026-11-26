//! One entry as a password manager gives it, held with every hidden
//! detail's value taken off: its id, title and details, the one-time
//! code's detail, the words a search reads, and the record a client
//! is told.

use serde_json::{Map, Value, json};

use crate::{KIND, key};

/// One entry as a manager gives it, its secrets already taken off.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry(pub(crate) Map<String, Value>);

impl Entry {
    /// An entry a manager gave, with every hidden detail's value taken
    /// off, or none for one with no id.
    pub fn without_secrets(given: &Value) -> Option<Entry> {
        let Value::Object(given) = given else {
            return None;
        };
        let id = given.get("id").and_then(Value::as_str)?;
        if id.is_empty() {
            return None;
        }
        let mut entry = given.clone();
        if let Some(Value::Array(details)) = entry.get_mut("details") {
            for detail in details.iter_mut() {
                if let Value::Object(detail) = detail {
                    let hidden = detail.get("hidden").and_then(Value::as_bool) == Some(true)
                        || matches!(
                            detail.get("type").and_then(Value::as_str),
                            Some("hidden" | "code")
                        );
                    if hidden {
                        detail.remove("value");
                        detail.insert("hidden".to_owned(), Value::Bool(true));
                    }
                }
            }
        }
        Some(Entry(entry))
    }

    pub fn id(&self) -> &str {
        self.0.get("id").and_then(Value::as_str).unwrap_or("")
    }

    pub fn title(&self) -> &str {
        self.0.get("title").and_then(Value::as_str).unwrap_or("")
    }

    /// Whether it holds every detail, rather than the summary a list gives.
    pub fn detailed(&self) -> bool {
        self.0.get("details").is_some_and(Value::is_array)
    }

    /// A detail it holds by its id, or, given none, its password: the one
    /// whose purpose is a password, else the first hidden one.
    pub fn detail(&self, wanted: Option<&str>) -> Option<&Map<String, Value>> {
        let details = self.0.get("details").and_then(Value::as_array)?;
        let each = || details.iter().filter_map(Value::as_object);
        match wanted {
            Some(wanted) => each().find(|detail| text(detail, "id") == Some(wanted)),
            None => each()
                .find(|detail| text(detail, "purpose") == Some("password"))
                .or_else(|| {
                    each().find(|detail| {
                        detail.get("hidden").and_then(Value::as_bool) == Some(true)
                            && text(detail, "type") != Some("code")
                    })
                }),
        }
    }

    /// Its one-time code's detail, if it has one.
    pub fn code(&self) -> Option<&Map<String, Value>> {
        self.0
            .get("details")
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_object)
            .find(|detail| text(detail, "type") == Some("code"))
    }

    /// The words a search reads it by: its title, username, websites'
    /// addresses and tags, and its category.
    pub(crate) fn words(&self) -> Vec<String> {
        let mut words = Vec::new();
        for name in ["title", "username", "category"] {
            if let Some(text) = self.0.get(name).and_then(Value::as_str) {
                words.push(text.to_owned());
            }
        }
        for website in self
            .0
            .get("websites")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(address) = website.get("address").and_then(Value::as_str) {
                words.push(address.to_owned());
            }
        }
        for tag in self
            .0
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(tag) = tag.as_str() {
                words.push(tag.to_owned());
            }
        }
        words
    }

    /// As a client reads it, keyed and stamped.
    pub fn record(&self, manager: &str, heard: u64) -> Value {
        let mut record = self.0.clone();
        record.insert("key".to_owned(), Value::String(key(manager, self.id())));
        record.insert("kind".to_owned(), Value::String(KIND.to_owned()));
        record.insert("manager".to_owned(), Value::String(manager.to_owned()));
        record.insert("heard".to_owned(), json!(heard));
        Value::Object(record)
    }
}

pub(crate) fn text<'a>(map: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    map.get(name).and_then(Value::as_str)
}
