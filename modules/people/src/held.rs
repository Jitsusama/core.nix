//! The place a source holds for its act while it's asked about someone,
//! as Slack holds Write in Slack's, dimmed and looking, from the first
//! rows of whoever it hasn't said a word of yet. So the act leads the ways
//! to reach them from the start and lights there as the source answers,
//! since a row coming later goes below those drawn (the shell's
//! Steady.js). It's served with everything else of them, so the shell
//! makes no row of its own.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{Book, KIND, Record, Standing, address, calendar, stamp};

/// Each source's held act, by the source's name.
#[derive(Clone, Debug, Default)]
pub struct Holds(BTreeMap<String, Value>);

impl Holds {
    /// The act each source's settings say it `holds`, by its key, worded
    /// with what the launcher's words say it while looking at its side.
    #[must_use]
    pub fn of<'a>(
        holding: impl IntoIterator<Item = (&'a String, &'a String)>,
        vocabulary: &Value,
    ) -> Holds {
        let looking = |key: &str| {
            vocabulary
                .get("things")
                .and_then(Value::as_array)
                .and_then(|things| things.iter().find(|thing| thing["id"] == key))
                .and_then(|thing| thing["said"]["looking"].as_str())
                .unwrap_or_default()
                .to_owned()
        };
        Holds(
            holding
                .into_iter()
                .map(|(source, key)| {
                    let act = json!({"key": key, "side": looking(key), "gone": true, "main": true});
                    (source.clone(), act)
                })
                .collect(),
        )
    }

    /// The places held for someone by their key: one for each source that
    /// holds an act, is answering, and has said nothing of this address.
    /// Its entry is as the source's own would be while it looks, knowing
    /// nobody by it yet, so the shell draws it as any served act. Someone
    /// known by no address, as a contact by its id alone, or a calendar's
    /// own address, is asked nothing, so holds none.
    #[must_use]
    pub fn of_key(
        &self,
        book: &Book,
        key: &str,
        said: &BTreeMap<String, Value>,
    ) -> Map<String, Value> {
        let mailed = address(key).filter(|address| address.contains('@'));
        if mailed.is_none_or(calendar) {
            return Map::new();
        }
        self.0
            .iter()
            .filter(|(source, _)| !said.contains_key(*source))
            .filter(|(source, _)| {
                matches!(book.standings().get(*source), Some(Standing::Answering))
            })
            .map(|(source, act)| (source.clone(), json!({"known": false, "acts": [act]})))
            .collect()
    }

    /// A record as it's served: what each source said of them, and each
    /// place held beside it.
    #[must_use]
    pub fn served(&self, book: &Book, record: &Record) -> Value {
        let mut served = record.json();
        self.beside(book, record, &mut served);
        served
    }

    /// Adds the places held for someone to what's served of them, whose
    /// `sources` say what each source said, and stamps it anew as
    /// `Record::json` does, so the shell makes them again as a place comes
    /// or goes, though no source said anything.
    pub fn beside(&self, book: &Book, record: &Record, served: &mut Value) {
        let held = self.of_key(book, &record.key, &record.sources);
        if held.is_empty() {
            return;
        }
        let mut all = record.sources.clone();
        all.extend(held.clone());
        served["stamp"] = Value::String(stamp(&all));
        if let Some(sources) = served["sources"].as_object_mut() {
            sources.extend(held);
        }
    }

    /// Those held for these keys, as served, with one made for an address
    /// no source has said anything of while a place is held for it, as
    /// someone Joel steps into whom nobody lists.
    #[must_use]
    pub fn known(&self, book: &Book, keys: &[String]) -> Vec<Value> {
        keys.iter()
            .filter_map(|key| match book.record(key) {
                Some(record) => Some(self.served(book, record)),
                None => self.unheard(book, key),
            })
            .collect()
    }

    /// What a watch of someone by their key is told of them: as served,
    /// made while a place is held for them, or gone.
    #[must_use]
    pub fn told(&self, book: &Book, key: &str) -> Value {
        match book.record(key) {
            Some(record) => self.served(book, record),
            None => self.unheard(book, key).unwrap_or_else(|| Record::gone(key)),
        }
    }

    /// Someone no source has said anything of, as served while a place is
    /// held for them, or nothing.
    fn unheard(&self, book: &Book, key: &str) -> Option<Value> {
        let held = self.of_key(book, key, &BTreeMap::new());
        (!held.is_empty())
            .then(|| json!({"key": key, "kind": KIND, "stamp": stamp(&held), "sources": held}))
    }
}
