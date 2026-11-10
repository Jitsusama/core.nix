//! What the daemon serves, record by record: every row by its key, a
//! fold's members among them and the fold holding their keys, the order
//! of the rows at the top (`agents:order`) and how they stand together
//! (`agents:summary`). Each making is held against the last, so a change
//! sends only the records that read differently and the keys gone, and a
//! session whose step alone moved sends nothing.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::row::Shown;

/// The key of the record listing the rows at the top, in their order.
pub const ORDER: &str = "agents:order";
/// The key of the record saying how the sessions stand together.
pub const SUMMARY: &str = "agents:summary";

/// The records as last served, by key.
#[derive(Debug, Default)]
pub struct Served {
    records: BTreeMap<String, Value>,
}

/// What a making moved: the records that read differently, whole, and
/// the keys gone.
#[derive(Debug, Default, PartialEq)]
pub struct Moved {
    pub changed: Vec<Value>,
    pub gone: Vec<String>,
}

impl Moved {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.gone.is_empty()
    }
}

/// Every record of a making: each row, by its key, and the order and
/// summary records.
pub fn records(shown: &[Shown], summary: &str) -> BTreeMap<String, Value> {
    let mut records = BTreeMap::new();
    let top = laid(shown, &mut records);
    records.insert(ORDER.to_owned(), json!({"key": ORDER, "keys": top}));
    records.insert(SUMMARY.to_owned(), json!({"key": SUMMARY, "said": summary}));
    records
}

/// Puts each row of `shown` among `records`, a fold's members too, and
/// answers their keys in order.
fn laid(shown: &[Shown], records: &mut BTreeMap<String, Value>) -> Vec<String> {
    shown
        .iter()
        .map(|one| {
            let record = match one {
                Shown::Session(row) => serde_json::to_value(row).unwrap_or(Value::Null),
                Shown::Fold(fold) => {
                    let held = laid(&fold.held, records);
                    json!({"key": fold.key, "name": fold.name, "glyph": fold.glyph,
                        "hue": fold.hue, "depth": fold.depth, "held": held})
                }
            };
            records.insert(one.key().to_owned(), record);
            one.key().to_owned()
        })
        .collect()
}

impl Served {
    /// A record as last served.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.records.get(key)
    }

    /// Every record as last served, in the order of their keys.
    pub fn all(&self) -> impl Iterator<Item = &Value> {
        self.records.values()
    }

    /// Takes a making's records in place of the last, answering what
    /// moved.
    pub fn take(&mut self, made: BTreeMap<String, Value>) -> Moved {
        let gone = self
            .records
            .keys()
            .filter(|key| !made.contains_key(*key))
            .cloned()
            .collect();
        let changed = made
            .iter()
            .filter(|(key, record)| self.records.get(*key) != Some(record))
            .map(|(_, record)| record.clone())
            .collect();
        self.records = made;
        Moved { changed, gone }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::{Fold, Ranked, Row};

    fn row(key: &str, side: &str) -> Shown {
        Shown::Session(Box::new(Row {
            key: key.to_owned(),
            name: key.to_owned(),
            also: String::new(),
            glyph: "agent",
            hue: "program",
            side: side.to_owned(),
            side_items: Vec::new(),
            note: String::new(),
            depth: 0,
            gone: false,
            state: "working",
            returns: "goTo",
            acts: vec!["goTo"],
            ranked: Ranked {
                note: String::new(),
                side: side.to_owned(),
                side_items: Vec::new(),
                belongs: String::new(),
            },
        }))
    }

    fn fold(held: Vec<Shown>) -> Shown {
        Shown::Fold(Fold {
            key: "agents:fold:O".into(),
            name: "2 listening".into(),
            glyph: "agent",
            hue: "program",
            depth: 1,
            held,
        })
    }

    #[test]
    fn a_fold_holds_its_members_keys_and_each_is_served_by_its_own() {
        let made = records(
            &[
                row("agent:O", "working"),
                fold(vec![row("agent:A", ""), row("agent:B", "")]),
            ],
            "1 working",
        );
        let keys: Vec<_> = made.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "agent:A",
                "agent:B",
                "agent:O",
                "agents:fold:O",
                ORDER,
                SUMMARY
            ]
        );
        assert_eq!(made[ORDER]["keys"], json!(["agent:O", "agents:fold:O"]));
        assert_eq!(made["agents:fold:O"]["held"], json!(["agent:A", "agent:B"]));
        assert_eq!(made[SUMMARY]["said"], "1 working");
        assert_eq!(
            made["agent:O"]["sideItems"],
            json!([]),
            "named as the launcher reads"
        );
    }

    #[test]
    fn only_what_reads_differently_is_sent_again() {
        let mut served = Served::default();
        let first = served.take(records(
            &[row("agent:O", "working"), row("agent:P", "")],
            "",
        ));
        assert_eq!(first.changed.len(), 4, "everything, the first time");
        let again = served.take(records(
            &[row("agent:O", "working"), row("agent:P", "")],
            "",
        ));
        assert!(
            again.is_empty(),
            "a making that reads the same sends nothing"
        );
        let moved = served.take(records(&[row("agent:O", "listening")], ""));
        let keys: Vec<_> = moved
            .changed
            .iter()
            .map(|record| record["key"].clone())
            .collect();
        assert_eq!(keys, [json!("agent:O"), json!(ORDER)]);
        assert_eq!(moved.gone, ["agent:P"]);
        assert_eq!(
            served.get("agent:O").map(|o| o["side"].clone()),
            Some(json!("listening"))
        );
    }
}
