//! The records the daemon holds for the shell (serve.rs): each to-do as
//! its owner wrote it, with what the daemon found of it and what Joel
//! said of it; each look the agents left him; and what waits counted
//! whole, so the bar's For You mark can light from one record.
//!
//! A to-do's record is `to-do:<quest>:<id>`: `item`, the item whole as
//! `mesh_wait` wrote it; `verdict`, what this daemon found, as
//! `mission/verdicts.json` says it; `joel`, his word from the shell's
//! `mission/settled.json`, done or not now; and `state`, For You's list
//! for it, worked out as Waiting.js does: `done` once found done or Joel
//! says so, `now` once ready and not put off, `later` otherwise. A look's
//! record is `look:<id>`, its file whole as the harness wrote it, under
//! `look`. `waiting:summary` counts the to-dos in each state and the looks
//! in each of theirs.
//!
//! Left out: whether a to-do's quest has ended, which only the harness's
//! index of quests says and the mycelium node serves; For You drops one
//! whose quest has ended, so a record of it still counts here until its
//! owner takes it out of its file, as a quest's ending does.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::interface::SUMMARY;
use crate::todo::ToDo;
use crate::verdicts::{State, Verdict, iso};

/// What an item says, so one Joel put off comes back once its owner says
/// something new of it: the fields Waiting.js's `said` names, in its
/// order, written as `JSON.stringify` writes them.
const SAID: [&str; 10] = [
    "title", "why", "detail", "when", "ready", "minutes", "window", "steps", "at", "deadline",
];

/// Joel's word on each to-do, as the shell keeps it in settled.json:
/// `done`, by identity, when he said it; `notNow`, by identity, what the
/// item said when he put it off, and when.
#[derive(Debug, Default)]
pub struct Settled {
    done: Map<String, Value>,
    not_now: Map<String, Value>,
}

/// Joel's word, from settled.json's text; none where it can't be read.
pub fn settled(text: &str) -> Settled {
    let Ok(said) = serde_json::from_str::<Value>(text) else {
        return Settled::default();
    };
    let field = |name: &str| said.get(name).and_then(Value::as_object).cloned();
    Settled {
        done: field("done").unwrap_or_default(),
        not_now: field("notNow").unwrap_or_default(),
    }
}

/// What `item` says, as Waiting.js's `said` writes it.
pub fn said(item: &Value) -> String {
    let fields: Vec<Value> = SAID
        .iter()
        .map(|name| item.get(*name).cloned().unwrap_or(Value::Null))
        .collect();
    Value::Array(fields).to_string()
}

/// Every look in `folder`, each its file whole, in the order of their
/// names; a file that can't be read, as one being replaced, is none.
pub fn looks(folder: &Path) -> Vec<Value> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(|look| {
            look.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty())
        })
        .collect()
}

/// Whether Joel can do `item` now, as Waiting.js's `readyOf` says: once
/// found ready, else as its owner says, or in a file older than that,
/// its when saying now.
fn ready(item: &Value, verdict: Option<&Verdict>) -> bool {
    if verdict.is_some_and(|v| v.state == State::Ready) {
        return true;
    }
    match item.get("ready") {
        Some(Value::Bool(ready)) => *ready,
        _ => item
            .get("when")
            .and_then(Value::as_str)
            .is_some_and(|when| when.trim().eq_ignore_ascii_case("now")),
    }
}

/// Joel's word on one to-do, and whether he put it off while it said
/// what it says now.
fn joel(todo: &ToDo, settled: &Settled) -> (Option<Value>, bool) {
    if let Some(at) = settled.done.get(&todo.key) {
        return (Some(json!({"said": "done", "at": at})), false);
    }
    let Some(put) = settled.not_now.get(&todo.key) else {
        return (None, false);
    };
    let current = put.get("said").and_then(Value::as_str) == Some(said(&todo.item).as_str());
    let at = put.get("at").cloned().unwrap_or(Value::Null);
    (
        Some(json!({"said": "not now", "at": at, "current": current})),
        current,
    )
}

/// Every record: each to-do, each look, and the count of what waits.
pub fn records(
    todos: &[ToDo],
    found: &BTreeMap<String, Verdict>,
    looks: &[Value],
    settled: &Settled,
) -> Vec<Value> {
    let mut records = Vec::new();
    let mut states: BTreeMap<&str, u64> = [("now", 0), ("later", 0), ("done", 0)].into();
    for todo in todos {
        let verdict = found.get(&todo.key);
        let (word, put_off) = joel(todo, settled);
        let done = verdict.is_some_and(|v| v.state == State::Done)
            || word.as_ref().is_some_and(|w| w["said"] == "done");
        let ready = ready(&todo.item, verdict);
        let state = if done {
            "done"
        } else if ready && !put_off {
            "now"
        } else {
            "later"
        };
        *states.entry(state).or_default() += 1;
        let mut record = json!({
            "key": format!("to-do:{}", todo.key), "kind": "to-do",
            "state": state, "ready": ready && !done, "item": todo.item,
        });
        if let Some(verdict) = verdict {
            record["verdict"] = json!({
                "state": verdict.state.name(), "at": iso(verdict.at), "how": verdict.how,
            });
        }
        if let Some(word) = word {
            record["joel"] = word;
        }
        records.push(record);
    }
    let mut seen: BTreeMap<String, u64> = [("waiting".to_owned(), 0)].into();
    for look in looks {
        let id = look["id"].as_str().unwrap_or_default();
        let state = look
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("waiting");
        *seen.entry(state.to_owned()).or_default() += 1;
        records.push(json!({"key": format!("look:{id}"), "kind": "look", "look": look}));
    }
    records.push(json!({
        "key": SUMMARY, "kind": "waiting", "to_dos": states, "looks": seen,
    }));
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::in_file;

    fn found(key: &str, state: State) -> BTreeMap<String, Verdict> {
        [(
            key.to_owned(),
            Verdict {
                state,
                at: 1_794_200_400,
                how: json!({"done": {"check": "true"}}),
            },
        )]
        .into()
    }

    fn summary(records: &[Value]) -> &Value {
        records
            .iter()
            .find(|r| r["key"] == SUMMARY)
            .expect("the count is always held")
    }

    #[test]
    fn what_an_item_says_is_written_as_the_shell_writes_it() {
        let item = json!({"title": "Sign in", "why": "Only you can", "minutes": 2,
            "ready": true, "steps": ["Open it", "Type"], "owner": {"address": "ABC123"}});
        assert_eq!(
            said(&item),
            r#"["Sign in","Only you can",null,null,true,2,null,["Open it","Type"],null,null]"#,
            "the fields Waiting.js's said names, in its order, nulls for the rest"
        );
    }

    #[test]
    fn a_to_do_is_now_later_or_done_as_for_you_lists_it() {
        let todos = in_file(
            r#"{"quest":"Q","items":[
            {"id":"a","title":"Ready","ready":true},
            {"id":"b","title":"Not yet","ready":false,"readyWhen":{"signal":"boot"}},
            {"id":"c","title":"Found ready","ready":false},
            {"id":"d","title":"Found done","ready":true},
            {"id":"e","title":"Older file","when":" Now "}]}"#,
        );
        let mut verdicts = found("Q:c", State::Ready);
        verdicts.extend(found("Q:d", State::Done));
        let records = records(&todos, &verdicts, &[], &Settled::default());
        let states: Vec<&str> = records[..5]
            .iter()
            .map(|r| r["state"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            states,
            ["now", "later", "now", "done", "now"],
            "each to-do in the list For You puts it in"
        );
        assert_eq!(records[0]["key"], "to-do:Q:a", "a to-do's key");
        assert_eq!(records[0]["item"]["title"], "Ready", "the item whole");
        assert_eq!(records[3]["ready"], false, "a done to-do isn't ready");
        assert_eq!(
            records[2]["verdict"],
            json!({"state": "ready", "at": "2026-11-09T05:00:00Z",
                "how": {"done": {"check": "true"}}}),
            "what the daemon found, as verdicts.json says it"
        );
        assert_eq!(
            summary(&records)["to_dos"],
            json!({"now": 3, "later": 1, "done": 1}),
            "the to-dos counted by state"
        );
    }

    #[test]
    fn joels_word_puts_one_off_until_it_says_something_new() {
        let todos = in_file(
            r#"{"quest":"Q","items":[
            {"id":"a","title":"Put off","ready":true},
            {"id":"b","title":"Said again","ready":true},
            {"id":"c","title":"Done by Joel","ready":true}]}"#,
        );
        let before = said(&json!({"title": "Said before", "ready": true}));
        let now = said(&todos[0].item);
        let settled = settled(
            &json!({"done": {"Q:c": 1_794_000_000_000_u64},
                "notNow": {"Q:a": {"said": now, "at": 1_794_000_000_001_u64},
                    "Q:b": {"said": before, "at": 1_794_000_000_002_u64}}})
            .to_string(),
        );
        let records = records(&todos, &BTreeMap::new(), &[], &settled);
        let states: Vec<&str> = records[..3]
            .iter()
            .map(|r| r["state"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            states,
            ["later", "now", "done"],
            "put off while it says the same, back once it says more, done by his word"
        );
        assert_eq!(
            records[0]["joel"],
            json!({"said": "not now", "at": 1_794_000_000_001_u64, "current": true}),
            "his word, and that it holds"
        );
        assert_eq!(
            records[1]["joel"]["current"], false,
            "a word on what it said before"
        );
        assert_eq!(
            records[2]["joel"],
            json!({"said": "done", "at": 1_794_000_000_000_u64}),
            "done, and when"
        );
    }

    #[test]
    fn each_look_is_held_whole_and_counted_by_its_state() {
        let looks = [
            json!({"id": "a1", "kind": "pr", "title": "Fix", "state": "waiting"}),
            json!({"id": "b2", "kind": "page", "title": "Site", "state": "opened"}),
            json!({"id": "c3", "kind": "file", "title": "Notes", "state": "waiting"}),
        ];
        let records = records(&[], &BTreeMap::new(), &looks, &Settled::default());
        assert_eq!(
            records[0],
            json!({"key": "look:a1", "kind": "look", "look": looks[0]}),
            "a look's file whole, under its own key"
        );
        assert_eq!(
            summary(&records)["looks"],
            json!({"waiting": 2, "opened": 1}),
            "looks counted by state"
        );
    }

    #[test]
    fn nothing_waiting_still_counts_none() {
        let records = records(&[], &BTreeMap::new(), &[], &settled("{\"done\":"));
        assert_eq!(
            records,
            [json!({"key": SUMMARY, "kind": "waiting",
                "to_dos": {"now": 0, "later": 0, "done": 0}, "looks": {"waiting": 0}})],
            "the count, at none, is the only record"
        );
    }
}
