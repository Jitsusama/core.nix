//! A place's rows as a daemon last told them. Each making hands over its
//! rows in the order shown, and is answered with only the records that
//! read differently and the keys that went, so a daemon tells its
//! watches one row when one row moved, however many it makes. A daemon
//! serving several places keeps one of these for each, since an order
//! is only ever weighed against its own place's.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::order::spaced;

/// What a making changed: the records to send again, whole, and the keys
/// no longer served.
#[derive(Debug, Default, PartialEq)]
pub struct Moved {
    pub changed: Vec<Value>,
    pub gone: Vec<String>,
}

impl Moved {
    /// Whether the making changed nothing a watch would hear.
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.gone.is_empty()
    }
}

/// The records of one place as last told, by key: each row's as
/// `{ "key", "kind": "row", "row" }`, its `order` in the row, and any
/// beside them (a headline, a summary) as they were handed over.
#[derive(Debug, Default)]
pub struct Kept {
    records: BTreeMap<String, Value>,
}

impl Kept {
    /// The record told for a key, if any.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.records.get(key)
    }

    /// Every record told, in key order, as a watch is answered whole.
    pub fn all(&self) -> impl Iterator<Item = &Value> {
        self.records.values()
    }

    /// Takes a making: its rows in the order shown, each with its `key`
    /// and without an `order`, and the whole records served beside them,
    /// each with its own `key`. Each row keeps the order it had where it
    /// can (see [`spaced`]), and what's answered is what a watch must be
    /// told: the records that read differently, and the keys that went.
    pub fn take(&mut self, rows: Vec<Value>, beside: Vec<Value>) -> Moved {
        let was: Vec<Option<i64>> = rows
            .iter()
            .map(|row| {
                self.records
                    .get(key_of(row))
                    .and_then(|was| was["row"]["order"].as_i64())
            })
            .collect();
        let mut records = BTreeMap::new();
        for (mut row, order) in rows.into_iter().zip(spaced(&was)) {
            row["order"] = json!(order);
            let key = key_of(&row).to_owned();
            records.insert(
                key.clone(),
                json!({ "key": key, "kind": "row", "row": row }),
            );
        }
        for record in beside {
            records.insert(key_of(&record).to_owned(), record);
        }
        let gone = self
            .records
            .keys()
            .filter(|key| !records.contains_key(*key))
            .cloned()
            .collect();
        let changed = records
            .iter()
            .filter(|(key, record)| self.records.get(*key) != Some(record))
            .map(|(_, record)| record.clone())
            .collect();
        self.records = records;
        Moved { changed, gone }
    }

    /// Lets every record go, as when nothing watches the place any more,
    /// answering the keys that went.
    pub fn let_go(&mut self) -> Vec<String> {
        std::mem::take(&mut self.records).into_keys().collect()
    }
}

/// A row's or a record's key, or "" where it has none.
fn key_of(record: &Value) -> &str {
    record["key"].as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, name: &str) -> Value {
        json!({ "key": key, "place": "heat", "kind": "part", "name": name })
    }

    fn orders(kept: &Kept) -> Vec<(String, i64)> {
        kept.all()
            .filter_map(|record| {
                let order = record["row"]["order"].as_i64()?;
                Some((key_of(record).to_owned(), order))
            })
            .collect()
    }

    #[test]
    fn the_first_making_is_told_whole_and_spaced() {
        let mut kept = Kept::default();
        let head = json!({ "key": "head:heat", "kind": "head", "head": { "said": "cool" } });
        let moved = kept.take(
            vec![row("row:heat:a", "A"), row("row:heat:b", "B")],
            vec![head.clone()],
        );
        assert_eq!(moved.changed.len(), 3);
        assert!(moved.gone.is_empty(), "nothing was told before");
        assert_eq!(
            orders(&kept),
            [
                ("row:heat:a".to_owned(), 1024),
                ("row:heat:b".to_owned(), 2048)
            ]
        );
        assert_eq!(kept.get("head:heat"), Some(&head));
        assert_eq!(kept.get("row:heat:a").unwrap()["kind"], "row");
    }

    #[test]
    fn only_what_reads_differently_is_told_again() {
        let mut kept = Kept::default();
        let rows = || vec![row("row:heat:a", "A"), row("row:heat:b", "B")];
        kept.take(rows(), vec![]);
        assert!(kept.take(rows(), vec![]).is_empty(), "nothing moved");
        let renamed = kept.take(
            vec![row("row:heat:a", "A"), row("row:heat:b", "Bee")],
            vec![],
        );
        assert_eq!(renamed.changed.len(), 1);
        assert_eq!(renamed.changed[0]["row"]["name"], "Bee");
        assert_eq!(
            renamed.changed[0]["row"]["order"], 2048,
            "it keeps its place"
        );
    }

    #[test]
    fn a_moved_row_alone_is_told_and_a_gone_one_said_gone() {
        let mut kept = Kept::default();
        kept.take(
            vec![
                row("row:heat:a", "A"),
                row("row:heat:b", "B"),
                row("row:heat:c", "C"),
            ],
            vec![],
        );
        let moved = kept.take(
            vec![
                row("row:heat:a", "A"),
                row("row:heat:c", "C"),
                row("row:heat:b", "B"),
            ],
            vec![],
        );
        assert_eq!(moved.changed.len(), 1);
        assert_eq!(moved.changed[0]["key"], "row:heat:c");
        assert_eq!(moved.changed[0]["row"]["order"], 1536);
        let went = kept.take(vec![row("row:heat:a", "A"), row("row:heat:b", "B")], vec![]);
        assert_eq!(went.gone, ["row:heat:c"]);
        assert!(went.changed.is_empty(), "the rest keep their orders");
        assert_eq!(kept.let_go(), ["row:heat:a", "row:heat:b"]);
        assert_eq!(kept.all().count(), 0);
    }
}
