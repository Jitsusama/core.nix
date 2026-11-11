//! A person's drawer, made ready here as every daemon's are
//! (ready-rows.md), so the shell merges nobody to show one: with sources
//! merged on the GUI thread, showing someone's drawer merged everyone for
//! their facts (Y2YBTS, 2026-11-10). It's their address, then the facts
//! each source says of them, in the order the sources are consulted, and
//! after a time zone the time there, left to the shell's clock to word.
//!
//! It's a record of its own, `drawer:people:<known>`, beside their row,
//! which names it, so a fact that changes, as Slack's presence does, is
//! told without the row.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Record;
use crate::listing::{self, Person, Stands};

/// The word for their address, in the launcher's vocabulary.
pub const EMAIL: &str = "people:email";
/// The word for the time where their zone is.
pub const THERE: &str = "people:there";

/// The names a drawer gives what it adds to the sources' facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    /// "Email", their address.
    pub email: String,
    /// "Time There", the time in their zone, after the zone.
    pub there: String,
}

impl Named {
    /// As the launcher's vocabulary (words.json) names them, each by its
    /// id where it doesn't, so a drawer never says nothing.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Named {
        let name = |id: &str| {
            vocabulary
                .get("things")
                .and_then(Value::as_array)
                .and_then(|things| {
                    things
                        .iter()
                        .find(|thing| thing.get("id").and_then(Value::as_str) == Some(id))
                })
                .and_then(|thing| thing.get("name"))
                .and_then(Value::as_str)
                .map_or_else(|| id.to_owned(), str::to_owned)
        };
        Named {
            email: name(EMAIL),
            there: name(THERE),
        }
    }
}

/// People's words and a drawer's names, from the launcher's vocabulary at
/// `path`, the daemon's WORDS; each said by its name where there's none,
/// or it can't be read.
#[must_use]
pub fn words_at(path: Option<&str>) -> (BTreeMap<String, String>, Named) {
    let read = path.map(|path| {
        std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
    });
    match read {
        Some(Ok(vocabulary)) => (listing::words(&vocabulary), Named::read(&vocabulary)),
        Some(Err(why)) => {
            eprintln!("<4>couldn't read the launcher's words, so each is said by name: {why}");
            (BTreeMap::default(), Named::read(&Value::Null))
        }
        None => (BTreeMap::default(), Named::read(&Value::Null)),
    }
}

/// The key someone's drawer goes by, as their row names it.
#[must_use]
pub fn key(person: &Person) -> String {
    let known = person.key.strip_prefix("person:").unwrap_or(&person.key);
    format!("drawer:people:{known}")
}

/// Their drawer's record: their address, then each source's facts in the
/// order the sources are consulted, the first of each name, a time zone
/// said by its name and followed by the time there.
#[must_use]
pub fn drawer(
    person: &Person,
    record: &Record,
    sources: &[(String, Option<Stands>)],
    named: &Named,
) -> Value {
    let mut facts: Vec<Value> = Vec::new();
    if !person.email.is_empty() {
        facts.push(json!({"name": named.email, "said": person.email}));
    }
    let mut seen: Vec<&Value> = Vec::new();
    let told = sources
        .iter()
        .filter_map(|(name, _)| record.sources.get(name))
        .filter_map(|entry| entry.get("facts").and_then(Value::as_array))
        .flatten();
    for fact in told {
        if let Some(name) = fact.get("name") {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
        }
        match fact.get("zone").and_then(Value::as_str) {
            Some(zone) if fact.get("as").and_then(Value::as_str) == Some("zone") => {
                facts.push(json!({"name": fact["name"], "said": zone}));
                facts.push(json!({"name": named.there, "zone": zone, "as": "zone"}));
            }
            _ => facts.push(fact.clone()),
        }
    }
    json!({
        "key": key(person), "kind": "drawer",
        "drawer": {"title": person.name, "facts": facts, "body": ""},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named() -> Named {
        Named {
            email: "Email".into(),
            there: "Time There".into(),
        }
    }

    fn sources() -> Vec<(String, Option<Stands>)> {
        vec![
            ("slack".to_owned(), None),
            ("pages".to_owned(), None),
            ("tuple".to_owned(), None),
        ]
    }

    fn record(key: &str, entries: &[(&str, Value)]) -> Record {
        let mut record = Record::of(key.to_owned(), 0);
        for (source, entry) in entries {
            record.sources.insert((*source).to_owned(), entry.clone());
        }
        record
    }

    fn drawn(record: &Record) -> Value {
        drawer(
            &listing::person(record, &sources()),
            record,
            &sources(),
            &named(),
        )
    }

    #[test]
    fn their_address_comes_first_then_each_sources_facts_in_order() {
        let ada = record(
            "person:ada@example.com",
            &[
                (
                    "tuple",
                    json!({"name": "Ada", "facts": [{"name": "Team", "said": "Calls"}]}),
                ),
                (
                    "slack",
                    json!({"facts": [
                        {"name": "Status", "said": "🌴 away"},
                        {"name": "Time Zone", "zone": "Asia/Kolkata", "as": "zone"}
                    ]}),
                ),
                (
                    "pages",
                    json!({"facts": [{"name": "Team", "said": "Shell"}]}),
                ),
            ],
        );
        assert_eq!(
            drawn(&ada),
            json!({
                "key": "drawer:people:ada@example.com", "kind": "drawer",
                "drawer": {"title": "Ada", "body": "", "facts": [
                    {"name": "Email", "said": "ada@example.com"},
                    {"name": "Status", "said": "🌴 away"},
                    {"name": "Time Zone", "said": "Asia/Kolkata"},
                    {"name": "Time There", "zone": "Asia/Kolkata", "as": "zone"},
                    {"name": "Team", "said": "Shell"},
                ]},
            }),
            "Slack's facts first, as it's consulted first, the time there after the zone, and \
             the first source's Team over Tuple's"
        );
    }

    #[test]
    fn one_known_by_no_address_says_none() {
        let gus = record(
            "person:tuple:7",
            &[("tuple", json!({"name": "Gus", "id": 7}))],
        );
        assert_eq!(
            (
                drawn(&gus)["key"].clone(),
                drawn(&gus)["drawer"]["facts"].clone()
            ),
            (json!("drawer:people:tuple:7"), json!([])),
            "known by Tuple's id, as their row names their drawer, with nothing to say"
        );
    }

    #[test]
    fn a_drawers_names_are_the_launchers_or_their_ids() {
        let vocabulary = json!({"things": [{"id": EMAIL, "name": "Address"}]});
        assert_eq!(
            Named::read(&vocabulary),
            Named {
                email: "Address".into(),
                there: THERE.into()
            },
            "the launcher's word, and a word it lacks said by its id"
        );
    }
}
