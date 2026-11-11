//! The records the daemon holds for the shell (serve.rs): each to-do as
//! its owner wrote it, with what the daemon found of it and what Joel
//! said of it; each look the agents left him; and what waits counted
//! whole, so the bar's For You mark can light from one record.
//!
//! A to-do's record is `to-do:<quest>:<id>`: `item`, the item whole as
//! `mesh_wait` wrote it; `verdict`, what this daemon found, as
//! `mission/verdicts.json` says it; `joel`, his word from the shell's
//! `mission/settled.json`, done or not now; `state`, For You's list
//! for it, worked out as Waiting.js does: `done` once found done or Joel
//! says so, `now` once ready and not put off, `later` otherwise; and
//! `order`, its place in that list, as Waiting.js's `before` puts it. An
//! item with no title is left out, as For You says nothing of one. A look's
//! record is `look:<id>`, its file whole as the harness wrote it, under
//! `look`. `waiting:summary` counts the to-dos in each state and the looks
//! in each of theirs. Given the launcher's words, each to-do's record is
//! worded for For You beside its item too (worded.rs).
//!
//! Left out: whether a to-do's quest has ended, which only the harness's
//! index of quests says and the mycelium node serves; For You drops one
//! whose quest has ended, so a record of it still counts here until its
//! owner takes it out of its file, as a quest's ending does.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::interface::SUMMARY;
use crate::todo::{ToDo, as_javascript};
use crate::verdicts::{State, Verdict, iso};
use crate::worded::worded;
use crate::words::Words;

/// What an item says, so one Joel put off comes back once its owner says
/// something new of it: the fields Waiting.js's `said` names, in its
/// order, written as `JSON.stringify` writes them. Its numbers are
/// written as JavaScript writes them (`as_javascript`); an object among
/// them would have its keys sorted, where `JSON.stringify` keeps them as
/// written, but `mesh_wait` writes none there.
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

/// Joel's word, from settled.json's text; none where it isn't an object,
/// as a file cut short.
pub fn settled(text: &str) -> Option<Settled> {
    let said = serde_json::from_str::<Value>(text).ok()?;
    said.is_object().then(|| {
        let field = |name: &str| said.get(name).and_then(Value::as_object).cloned();
        Settled {
            done: field("done").unwrap_or_default(),
            not_now: field("notNow").unwrap_or_default(),
        }
    })
}

/// What the shell is told beside the to-dos, read once at the start and
/// again only as inotify says their files changed: the looks, Joel's
/// word, and how each of their sources stands.
#[derive(Debug)]
pub struct Beside {
    pub looks: Vec<Value>,
    pub settled: Settled,
    pub looks_standing: Value,
    pub settled_standing: Value,
}

impl Default for Beside {
    fn default() -> Self {
        Beside {
            looks: Vec::new(),
            settled: Settled::default(),
            looks_standing: json!({"state": "absent"}),
            settled_standing: json!({"state": "absent"}),
        }
    }
}

impl Beside {
    /// Reads the looks in `looks` and Joel's word in `settled` again.
    /// A look that can't be read as one is named on the journal and left
    /// out. Joel's word that can't be read keeps the last he said, rather
    /// than bringing back all he marked done or put off, and says its
    /// source is failing; one gone is none.
    pub fn read(&mut self, looks: Option<&Path>, settled: Option<&Path>) {
        if let Some(folder) = looks {
            let (read, unread) = looks_in(folder);
            for name in unread {
                eprintln!("<4>{name} couldn't be read as a look, so it's left out");
            }
            self.looks = read;
            self.looks_standing = standing(folder.is_dir());
        }
        let Some(path) = settled else {
            return;
        };
        match fs::read_to_string(path) {
            Ok(text) => {
                if let Some(word) = self::settled(&text) {
                    self.settled = word;
                    self.settled_standing = standing(true);
                } else {
                    eprintln!("<4>Joel's word can't be read, so the last he said is kept");
                    self.settled_standing = json!({"state": "failing",
                        "reason": "settled.json isn't an object"});
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.settled = Settled::default();
                self.settled_standing = standing(false);
            }
            Err(error) => {
                eprintln!("<4>Joel's word can't be read, so the last he said is kept: {error}");
                self.settled_standing = json!({"state": "failing",
                    "reason": error.to_string()});
            }
        }
    }
}

/// A source that answers, or isn't there.
pub fn standing(there: bool) -> Value {
    if there {
        json!({"state": "answering"})
    } else {
        json!({"state": "absent"})
    }
}

/// What `item` says, as Waiting.js's `said` writes it.
pub fn said(item: &Value) -> String {
    let fields: Vec<Value> = SAID
        .iter()
        .map(|name| item.get(*name).map_or(Value::Null, as_javascript))
        .collect();
    Value::Array(fields).to_string()
}

/// Every look in `folder`, each its file whole, in the order of their
/// names, and the names of those that aren't a look with an id. A file
/// gone between listing and reading, as one replaced, is neither.
fn looks_in(folder: &Path) -> (Vec<Value>, Vec<String>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return (Vec::new(), Vec::new());
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut looks = Vec::new();
    let mut unread = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let look = serde_json::from_str::<Value>(&text).ok().filter(|look| {
            look.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty())
        });
        match look {
            Some(look) => looks.push(look),
            None => unread.push(path.display().to_string()),
        }
    }
    (looks, unread)
}

/// When an item is for, in milliseconds, as Waiting.js's `timeOf` reads
/// it; none for one for no time in particular.
fn time_of(item: &Value) -> Option<i64> {
    item.get("at")
        .and_then(Value::as_str)
        .and_then(|at| at.parse::<jiff::Timestamp>().ok())
        .map(jiff::Timestamp::as_millisecond)
}

/// Waiting.js's `before`: anything for a time first, soonest first, then
/// the quickest, then as their files list them.
fn before(one: (&Value, usize), other: (&Value, usize)) -> Ordering {
    let time = |item: &Value| time_of(item).unwrap_or(i64::MAX);
    let minutes = |item: &Value| {
        item.get("minutes")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY)
    };
    time(one.0)
        .cmp(&time(other.0))
        .then(minutes(one.0).total_cmp(&minutes(other.0)))
        .then(one.1.cmp(&other.1))
}

/// Each to-do's place among `todos`, as For You lists them: by file, then
/// when raised, then as the file lists them (Waiting.js's `served`), and
/// then by `before`.
fn places(todos: &[&ToDo]) -> Vec<usize> {
    let raised = |todo: &ToDo| {
        todo.item
            .get("raisedAt")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut listed: Vec<usize> = (0..todos.len()).collect();
    listed.sort_by(|a, b| {
        let (one, other) = (todos[*a], todos[*b]);
        one.file
            .cmp(&other.file)
            .then_with(|| raised(one).cmp(&raised(other)))
            .then(a.cmp(b))
    });
    let mut at = vec![0; todos.len()];
    for (place, index) in listed.iter().enumerate() {
        at[*index] = place;
    }
    let mut ordered: Vec<usize> = (0..todos.len()).collect();
    ordered.sort_by(|a, b| before((&todos[*a].item, at[*a]), (&todos[*b].item, at[*b])));
    ordered
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
    words: Option<&Words>,
) -> Vec<Value> {
    let mut records = Vec::new();
    let mut states: BTreeMap<&str, u64> = [("now", 0), ("later", 0), ("done", 0)].into();
    let titled: Vec<&ToDo> = todos
        .iter()
        .filter(|todo| {
            todo.item
                .get("title")
                .and_then(Value::as_str)
                .is_some_and(|title| !title.is_empty())
        })
        .collect();
    for index in places(&titled) {
        let todo = titled[index];
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
        let order = states.entry(state).or_default();
        let mut record = json!({
            "key": format!("to-do:{}", todo.key), "kind": "to-do",
            "state": state, "order": *order, "ready": ready && !done, "item": todo.item,
        });
        *order += 1;
        if let Some(verdict) = verdict {
            record["verdict"] = json!({
                "state": verdict.state.name(), "at": iso(verdict.at), "how": verdict.how,
            });
        }
        if let Some(word) = word {
            record["joel"] = word;
        }
        if let Some(words) = words {
            worded(&mut record, &todo.item, &todo.key, state == "now", words);
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
        let records = records(&todos, &verdicts, &[], &Settled::default(), None);
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
        )
        .expect("Joel's word reads");
        let records = records(&todos, &BTreeMap::new(), &[], &settled, None);
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
        let records = records(&[], &BTreeMap::new(), &looks, &Settled::default(), None);
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
    fn each_list_is_in_the_order_joel_does_them_and_an_untitled_one_is_left_out() {
        let todos = in_file(
            r#"{"quest":"Q","items":[
            {"id":"slow","title":"Slow","ready":true,"minutes":30},
            {"id":"later","title":"Later","ready":true,"at":"2026-11-10T15:00:00Z"},
            {"id":"soon","title":"Soon","ready":true,"at":"2026-11-10T09:00:00Z"},
            {"id":"quick","title":"Quick","ready":true,"minutes":2},
            {"id":"untitled","ready":true},
            {"id":"waits","title":"Waits","ready":false,"minutes":1}]}"#,
        );
        let records = records(&todos, &BTreeMap::new(), &[], &Settled::default(), None);
        let listed: Vec<(&str, &str, u64)> = records
            .iter()
            .filter(|r| r["kind"] == "to-do")
            .map(|r| {
                (
                    r["item"]["id"].as_str().unwrap_or_default(),
                    r["state"].as_str().unwrap_or_default(),
                    r["order"].as_u64().unwrap_or(u64::MAX),
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("soon", "now", 0),
                ("later", "now", 1),
                ("waits", "later", 0),
                ("quick", "now", 2),
                ("slow", "now", 3),
            ],
            "for a time first, soonest first, then the quickest, each list counted apart"
        );
    }

    #[test]
    fn a_number_is_said_as_javascript_says_it() {
        assert_eq!(
            said(&json!({"title": "T", "minutes": 5.0})),
            said(&json!({"title": "T", "minutes": 5})),
            "5.0 is written 5, as JSON.stringify writes it"
        );
        assert!(
            said(&json!({"title": "T", "minutes": 2.5})).contains("2.5"),
            "a fraction is kept"
        );
    }

    #[test]
    fn joels_word_unread_keeps_the_last_he_said() {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("settled.json");
        let mut beside = Beside::default();
        fs::write(&path, r#"{"done":{"Q:a":1},"notNow":{}}"#).expect("written");
        beside.read(None, Some(&path));
        assert_eq!(
            beside.settled_standing,
            json!({"state": "answering"}),
            "his word read"
        );
        fs::write(&path, r#"{"done":"#).expect("written");
        beside.read(None, Some(&path));
        assert_eq!(
            beside.settled_standing["state"], "failing",
            "a word that can't be read fails"
        );
        assert!(
            beside.settled.done.contains_key("Q:a"),
            "and the last he said is kept"
        );
        fs::remove_file(&path).expect("removed");
        beside.read(None, Some(&path));
        assert_eq!(
            (
                beside.settled_standing["state"].clone(),
                beside.settled.done.len()
            ),
            (json!("absent"), 0),
            "his word gone is none"
        );
    }

    #[test]
    fn a_look_that_cant_be_read_is_named() {
        let dir = tempfile::tempdir().expect("a folder");
        fs::write(
            dir.path().join("a1.json"),
            r#"{"id":"a1","state":"waiting"}"#,
        )
        .expect("written");
        fs::write(dir.path().join("b2.json"), r#"{"id":"#).expect("written");
        fs::write(dir.path().join("c3.json"), r#"{"state":"waiting"}"#).expect("written");
        let (looks, unread) = looks_in(dir.path());
        assert_eq!(looks.len(), 1, "only a look with an id is held: {looks:?}");
        let names: Vec<bool> = ["b2.json", "c3.json"]
            .iter()
            .map(|name| unread.iter().any(|path| path.ends_with(name)))
            .collect();
        assert_eq!(names, [true, true], "each other is named: {unread:?}");
    }

    #[test]
    fn nothing_waiting_still_counts_none() {
        let records = records(&[], &BTreeMap::new(), &[], &Settled::default(), None);
        assert_eq!(
            records,
            [json!({"key": SUMMARY, "kind": "waiting",
                "to_dos": {"now": 0, "later": 0, "done": 0}, "looks": {"waiting": 0}})],
            "the count, at none, is the only record"
        );
    }
}
