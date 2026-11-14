//! What the daemon serves, record by record, in the launcher's one form
//! for every daemon's rows (Y2YBTS's ready-rows.md): one kind, `row`, each
//! record keyed `row:quests:<id>` with the row's fields under `row`,
//! placed by a spaced `order` the shell never sorts, and how many are
//! open a record of its own kind, `summary`, read by its key. What only
//! Quests reads, what its acts and facts need, is under the row's `held`.
//! Each making is held against the last by ready-rows' `Kept`, so a
//! change sends only the records that read differently and the keys gone:
//! a list the harness writes again, every quest told anew, sends nothing.
//! Only the open quests' rows are served: an ended one's is made beside
//! them, kept for a search to find and answer with (search.rs), so the
//! shell holds 230 of Joel's 1,237 quests rather than all of them, whose
//! rows were 1.6 MB that every collection of its heap walked (2026-11-13).

use std::collections::{BTreeMap, BTreeSet};

use daemon_serve::watch::Held;
use ready_rows::time::{self, Form};
use serde_json::{Value, json};

use crate::holder::{self, Holder};
use crate::order;
use crate::quest::{self, Quest};
use crate::words::Words;

/// The launcher's place the rows are in.
pub const PLACE: &str = "quests";
/// The key of the record saying how many quests are open and driving.
pub const SUMMARY: &str = "summary:quests";

/// What every row is made from: the quests open and ended, by the node's
/// key, the sessions, the folder at home and Quests' words. Never the
/// clock: a day is sent as its moment, for the shell to word.
pub struct Inputs<'a> {
    pub open: &'a Held,
    pub ended: &'a Held,
    pub sessions: &'a Held,
    pub home: &'a str,
    pub words: &'a Words,
}

/// Every quest once, the open one where a quest is in both lists, as it
/// ends or reopens, until both are written.
fn every<'a>(inputs: &Inputs<'a>) -> Vec<Quest<'a>> {
    let mut seen = BTreeSet::new();
    inputs
        .open
        .records()
        .chain(inputs.ended.records())
        .map(|(_, record)| Quest(record))
        .filter(|quest| seen.insert(quest.id()))
        .collect()
}

/// A quest's row in the launcher's form, not yet given its `order`.
fn row(
    quest: Quest,
    holder: Option<&Holder>,
    named: &BTreeMap<&str, &str>,
    inputs: &Inputs,
) -> Value {
    let open = quest.open();
    let touched = quest::moment(quest.updated());
    let said = |words: &str| (!words.is_empty()).then(|| json!({ "said": words }));
    let kind = (quest.kind() != "quest")
        .then_some(quest.kind())
        .and_then(said);
    // An open quest says its priority; one ended, the day it was last
    // touched, as the shell's calendar says it.
    let when = if open {
        said(quest.priority())
    } else {
        touched.map(|ms| time::at(ms, Form::Day))
    };
    let address = holder.and_then(|holder| said(holder.address.as_str()));
    let side: Vec<Value> = [kind, when, address].into_iter().flatten().collect();
    let parent = named
        .get(quest.parent())
        .map(|name| json!({"id": quest.parent(), "name": name}));
    let session = holder.map(|holder| json!({"address": holder.address, "parked": holder.parked}));
    let held = json!({
        "id": quest.id(), "kind": quest.kind(), "status": quest.status(),
        "priority": quest.priority(), "updated": quest.updated(), "touched": touched,
        "folder": quest.folder(inputs.home), "open": open,
        "also": format!("{} {}", quest.id(), quest.kind()),
        "session": session, "parent": parent,
    });
    json!({
        "key": format!("row:{PLACE}:{}", quest.id()), "thing": format!("quest:{}", quest.id()),
        "place": PLACE, "kind": "quest", "name": quest.name(), "glyph": quest.glyph(),
        "hue": "program", "side": side, "note": quest.id(), "reads": quest.summary(),
        "unfinished": open, "held": held,
    })
}

/// How many are open and how many of those drive, or that none is.
fn summary(quests: &[Quest], words: &Words) -> String {
    let open = quests.iter().filter(|quest| quest.open()).count();
    let driving = quests
        .iter()
        .filter(|quest| quest.open() && quest.priority() == "driving")
        .count();
    match (open, driving) {
        (0, _) => words.fill("empty", &[]),
        (open, 0) => words.fill("open", &[("open", &open.to_string())]),
        (open, driving) => words.fill(
            "driving",
            &[
                ("open", &open.to_string()),
                ("driving", &driving.to_string()),
            ],
        ),
    }
}

/// One making: the rows served, the open quests', and those of the ended
/// ones, each in Quests' order, and the summary's words.
#[derive(Debug, Default)]
pub struct Made {
    pub open: Vec<Value>,
    pub ended: Vec<Value>,
    pub said: String,
}

/// Every row in Quests' order, open before ended, and the summary's words.
pub fn made(inputs: &Inputs) -> Made {
    let quests = every(inputs);
    let holders = holder::holders(inputs.sessions.records().map(|(_, record)| record));
    let named: BTreeMap<&str, &str> = quests
        .iter()
        .map(|quest| (quest.id(), quest.name()))
        .collect();
    let (open, ended) = order::sorted(&quests)
        .into_iter()
        .map(|quest| row(quest, holders.get(quest.id()), &named, inputs))
        .partition(|row| row["unfinished"] == true);
    Made {
        open,
        ended,
        said: summary(&quests, inputs.words),
    }
}

/// The record saying how many quests are open, served beside the rows.
pub fn summary_record(said: &str) -> Value {
    json!({"key": SUMMARY, "kind": "summary", "summary": {"place": PLACE, "said": said}})
}

#[cfg(test)]
mod tests {
    use ready_rows::kept::Kept;

    use super::*;

    fn listed(records: &[Value]) -> Held {
        let mut held = Held::default();
        held.took(&json!({"records": records}));
        held
    }

    fn quest(id: &str, fields: &Value) -> Value {
        let mut record = json!({"key": format!("quest:{id}"), "kind": "quest", "heard": 1,
            "id": id, "title": format!("Title {id}"), "quest.kind": "quest"});
        if let (Some(record), Some(fields)) = (record.as_object_mut(), fields.as_object()) {
            record.extend(fields.clone());
        }
        record
    }

    /// Every row made, open before ended, and the summary's words.
    fn making(open: &[Value], ended: &[Value], sessions: &[Value]) -> (Vec<Value>, String) {
        let made = made_of(open, ended, sessions);
        let rows = made.open.into_iter().chain(made.ended).collect();
        (rows, made.said)
    }

    fn made_of(open: &[Value], ended: &[Value], sessions: &[Value]) -> Made {
        let (open, ended, sessions) = (listed(open), listed(ended), listed(sessions));
        // The launcher's own words, so a case reads as Quests says it.
        let words = Words::from_vocabulary(include_str!("../../quickshell/launcher/words.json"))
            .expect("core's vocabulary reads");
        made(&Inputs {
            open: &open,
            ended: &ended,
            sessions: &sessions,
            home: "/home/joel",
            words: &words,
        })
    }

    #[test]
    fn a_row_says_its_kind_priority_and_session() {
        let open = [
            quest(
                "A",
                &json!({"status": "active", "priority": "driving", "rank": 1}),
            ),
            quest(
                "B",
                &json!({"quest.kind": "subquest", "parent": "A", "status": "active",
                "priority": "queued", "start": "/w"}),
            ),
        ];
        let sessions = [json!({"key": "session:S", "standing": "parked",
            "presence": {"addresses": ["S"], "owns": ["B"]}})];
        let (rows, said) = making(&open, &[], &sessions);
        assert_eq!(said, "2 open, 1 driving");
        let b = &rows[1];
        assert_eq!(b["key"], "row:quests:B");
        assert_eq!(b["thing"], "quest:B");
        assert_eq!(b["side"], json!([{"said": "subquest"}, {"said": "queued"}]));
        assert_eq!(b["held"]["parent"], json!({"id": "A", "name": "Title A"}));
        assert_eq!(b["held"]["folder"], "/w");
        assert_eq!(
            rows[0]["held"]["folder"], "/home/joel",
            "home where none is said"
        );
        assert_eq!(
            b["held"]["session"],
            Value::Null,
            "a session owning no quest id holds none"
        );
    }

    #[test]
    fn one_ended_says_its_day_and_one_loaded_its_session() {
        let open = [quest(
            "QEST-A",
            &json!({"status": "active", "priority": "bench"}),
        )];
        let ended = [quest(
            "QEST-E",
            &json!({"kind": "ended-quest", "status": "concluded", "updated": "2025-01-05"}),
        )];
        let sessions = [json!({"key": "session:S", "standing": "running",
            "presence": {"addresses": ["S"], "owns": ["QEST-A"]}})];
        let (rows, said) = making(&open, &ended, &sessions);
        assert_eq!(said, "1 open");
        assert_eq!(rows[0]["side"], json!([{"said": "bench"}, {"said": "S"}]));
        assert_eq!(
            rows[0]["held"]["session"],
            json!({"address": "S", "parked": false})
        );
        assert_eq!(
            rows[1]["side"],
            json!([{"at": 20_093 * 86_400_000_i64 + 43_200_000, "as": "day"}]),
            "its day as a moment, for the shell to word by its own calendar"
        );
        assert_eq!(rows[1]["held"]["touched"], rows[1]["side"][0]["at"]);
        assert_eq!(rows[1]["glyph"], "done");
        assert_eq!(rows[1]["unfinished"], false);
    }

    #[test]
    fn a_quest_in_both_lists_is_the_open_one() {
        let open = [quest("A", &json!({"status": "active"}))];
        let ended = [quest("A", &json!({"status": "concluded"}))];
        let (rows, _) = making(&open, &ended, &[]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["unfinished"], true);
        assert_eq!(making(&[], &[], &[]).1, "empty");
    }

    #[test]
    fn only_the_open_quests_are_served_and_the_ended_made_beside_them() {
        let open = [
            quest("A", &json!({"status": "active"})),
            quest("C", &json!({"status": "concluded"})),
        ];
        let ended = [quest("E", &json!({"status": "retired"}))];
        let made = made_of(&open, &ended, &[]);
        let keys =
            |rows: &[Value]| -> Vec<Value> { rows.iter().map(|row| row["key"].clone()).collect() };
        assert_eq!(
            keys(&made.open),
            [json!("row:quests:A")],
            "the open alone served"
        );
        assert_eq!(
            keys(&made.ended),
            [json!("row:quests:C"), json!("row:quests:E")],
            "one ended while the open list still holds it is made beside them too"
        );
        assert_eq!(made.said, "1 open");
    }

    #[test]
    fn only_what_reads_differently_is_sent_again() {
        let mut served = Kept::default();
        let mut take =
            |(rows, said): (Vec<Value>, String)| served.take(rows, vec![summary_record(&said)]);
        let open = [
            quest("A", &json!({"status": "active"})),
            quest("B", &json!({"status": "active"})),
        ];
        assert_eq!(take(making(&open, &[], &[])).changed.len(), 3);
        let mut again = open.clone();
        again[0]["heard"] = json!(2);
        assert!(
            take(making(&again, &[], &[])).is_empty(),
            "a list written again, only when heard moved, sends nothing"
        );
        let moved = take(making(&open[1..], &[], &[]));
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(keys, [json!(SUMMARY)], "B keeps its order");
        assert_eq!(moved.gone, ["row:quests:A"]);
        let b = served.get("row:quests:B").expect("B is served");
        assert_eq!(b["row"]["order"], 2048);
    }
}
