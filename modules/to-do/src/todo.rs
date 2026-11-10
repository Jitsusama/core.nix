//! The to-dos, as each quest's file in `mission/waiting` says them: what
//! makes each done, and what makes it ready, as its owner wrote it through
//! `mesh_wait`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Number, Value};

/// How something will be known: a command that succeeds once it's so, a
/// signal the machine gives, or only Joel's word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Knowing {
    Check(String),
    Signal(Signal),
    Person,
}

/// A signal a to-do waits on, as `mesh_wait` names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// A boot after the to-do was raised.
    Boot,
    /// A switch after it was raised, or with a commit, the machine running
    /// a core at that commit or past it.
    Switch(Option<String>),
    /// A sign-in to a service after it was raised.
    Signin(String),
    /// A device of that name present.
    Device(String),
    /// A file at that absolute path.
    File(PathBuf),
}

/// One to-do, by the key the shell and the harness both give it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDo {
    /// `<quest>:<id>`, or `<quest>:<title>` for one with no id, as
    /// Waiting.js's `identity` gives it.
    pub key: String,
    /// The name of the file it's in, without `.json`, which For You lists
    /// them by first.
    pub file: String,
    /// When it was raised, in seconds since 1970, which a boot, a switch or
    /// a sign-in must come after.
    pub raised: Option<i64>,
    /// Whether its owner already says it can be done now.
    pub ready: bool,
    pub done_when: Option<Knowing>,
    pub ready_when: Option<Knowing>,
    /// What the harness says of its owner's sandbox, which decides whether
    /// its checks run here, outside any.
    pub sandbox: Sandbox,
    /// The item whole, as its owner wrote it, naming its quest, for the
    /// shell (stream.rs).
    pub item: Value,
}

/// The sandbox a to-do's owner runs in, as `mesh_wait` writes it in
/// `owner.sandbox`. A strict session's commands can't write
/// `mission/waiting`, and its file tools are held to the same fence, so
/// only `mesh_wait`, in pi's own process, writes its to-dos and says
/// strict; a session outside the sandbox runs as Joel already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    /// None: its checks run here as they would in its own shell.
    Open,
    /// The strict sandbox, whose checks would run here outside it.
    Strict,
    /// Not said, as by a harness from before it said so, or anything else
    /// written there; its checks don't run, since that can't be told from
    /// strict.
    Unsaid,
}

impl Sandbox {
    fn of(item: &Value) -> Self {
        match item.pointer("/owner/sandbox").and_then(Value::as_str) {
            Some("none") => Self::Open,
            Some("strict") => Self::Strict,
            _ => Self::Unsaid,
        }
    }
}

impl ToDo {
    /// Whether anything here judges it, rather than only Joel or its owner.
    pub fn judged(&self) -> bool {
        let judged = |knowing: &Option<Knowing>| {
            knowing
                .as_ref()
                .is_some_and(|k| !matches!(k, Knowing::Person))
        };
        judged(&self.done_when) || (!self.ready && judged(&self.ready_when))
    }
}

/// How `value` says a thing will be known, or none where it says nothing
/// this reads.
pub fn knowing(value: &Value) -> Option<Knowing> {
    if let Some(check) = value.get("check").and_then(Value::as_str) {
        return Some(Knowing::Check(check.to_owned()));
    }
    if let Some(signal) = value.get("signal").and_then(Value::as_str) {
        return signal_of(signal).map(Knowing::Signal);
    }
    (value.get("by").and_then(Value::as_str) == Some("person")).then_some(Knowing::Person)
}

/// A signal from its words: `boot`, `switch`, `switch:<commit>`,
/// `signin:<service>`, `device:<name>` or `file:<absolute path>`.
pub fn signal_of(words: &str) -> Option<Signal> {
    let (kind, value) = words.split_once(':').unwrap_or((words, ""));
    match (kind, value) {
        ("boot", "") => Some(Signal::Boot),
        ("switch", "") => Some(Signal::Switch(None)),
        ("switch", commit) => Some(Signal::Switch(Some(commit.to_owned()))),
        ("signin", service) if !service.is_empty() => Some(Signal::Signin(service.to_owned())),
        ("device", name) if !name.is_empty() => Some(Signal::Device(name.to_owned())),
        ("file", path) if path.starts_with('/') => Some(Signal::File(PathBuf::from(path))),
        _ => None,
    }
}

/// The to-dos one file holds, keyed by its own quest unless an item names
/// another, as Waiting.js reads them.
pub fn in_file(text: &str) -> Vec<ToDo> {
    let Ok(file) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let quest = file
        .get("quest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(items) = file.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let id = identity(item)?;
            let quest = item.get("quest").and_then(Value::as_str).unwrap_or(quest);
            let mut whole = item.clone();
            whole["quest"] = Value::String(quest.to_owned());
            Some(ToDo {
                key: format!("{quest}:{id}"),
                file: quest.to_owned(),
                raised: item
                    .get("raisedAt")
                    .and_then(Value::as_str)
                    .and_then(seconds),
                ready: item.get("ready").and_then(Value::as_bool) == Some(true),
                done_when: item.get("doneWhen").and_then(knowing),
                ready_when: item.get("readyWhen").and_then(knowing),
                sandbox: Sandbox::of(item),
                item: whole,
            })
        })
        .collect()
}

/// Every to-do in `folder`'s files, in the order of their names; a file
/// that can't be read holds none, as one being replaced.
pub fn in_folder(folder: &Path) -> Vec<ToDo> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|path| {
            let file = path.file_stem()?.to_string_lossy().into_owned();
            let text = fs::read_to_string(path).ok()?;
            Some(in_file(&text).into_iter().map(move |todo| ToDo {
                file: file.clone(),
                ..todo
            }))
        })
        .flatten()
        .collect()
}

/// Who an item is within its quest, as Waiting.js's `identity` writes it:
/// its id, a number as JavaScript writes one, or its title where it has no
/// id. An id of any other kind is none, where JavaScript would write
/// `true` or `[object Object]`, since `mesh_wait` writes only text.
fn identity(item: &Value) -> Option<String> {
    match item.get("id") {
        Some(Value::String(id)) => Some(id.clone()),
        Some(Value::Number(id)) => Some(as_javascript(&Value::Number(id.clone())).to_string()),
        None | Some(Value::Null) => item.get("title").and_then(Value::as_str).map(str::to_owned),
        Some(_) => None,
    }
}

/// `value` with each number as JavaScript writes it, a whole one without
/// its point, so `5.0` is `5`.
pub fn as_javascript(value: &Value) -> Value {
    match value {
        Value::Number(number) => Value::Number(whole(number).unwrap_or_else(|| number.clone())),
        Value::Array(values) => values.iter().map(as_javascript).collect(),
        Value::Object(fields) => fields
            .iter()
            .map(|(name, field)| (name.clone(), as_javascript(field)))
            .collect(),
        other => other.clone(),
    }
}

/// A float that is whole, as the integer it is.
fn whole(number: &Number) -> Option<Number> {
    let float = number.as_f64().filter(|_| number.is_f64())?;
    (float.fract() == 0.0)
        .then(|| format!("{float:.0}").parse::<i64>().ok())
        .flatten()
        .map(Number::from)
}

/// Seconds since 1970 from an ISO time.
pub fn seconds(iso: &str) -> Option<i64> {
    iso.parse::<jiff::Timestamp>()
        .ok()
        .map(jiff::Timestamp::as_second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_signal_is_read_from_its_words() {
        assert_eq!(signal_of("boot"), Some(Signal::Boot));
        assert_eq!(signal_of("switch"), Some(Signal::Switch(None)));
        assert_eq!(
            signal_of("switch:1d839c4"),
            Some(Signal::Switch(Some("1d839c4".into())))
        );
        assert_eq!(
            signal_of("signin:google"),
            Some(Signal::Signin("google".into()))
        );
        assert_eq!(
            signal_of("device:Dock"),
            Some(Signal::Device("Dock".into()))
        );
        assert_eq!(
            signal_of("file:/run/x"),
            Some(Signal::File("/run/x".into()))
        );
    }

    #[test]
    fn a_signal_it_doesnt_know_or_a_relative_file_is_none() {
        assert_eq!(signal_of("thunder"), None);
        assert_eq!(signal_of("file:here.txt"), None);
        assert_eq!(signal_of("signin:"), None);
        assert_eq!(signal_of("boot:now"), None);
    }

    #[test]
    fn a_file_says_its_to_dos_by_quest_and_id() {
        let file = r#"{"v":2,"quest":"QEST-1","items":[
            {"id":"a","title":"A","ready":true,"doneWhen":{"check":"true"},
             "raisedAt":"2026-11-09T05:00:00Z"},
            {"id":"b","title":"B","quest":"QEST-2","doneWhen":{"by":"person"},
             "readyWhen":{"signal":"switch:abc"}},
            {"title":"Only a title"}]}"#;
        let found = in_file(file);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].key, "QEST-1:a");
        assert_eq!(found[0].raised, Some(1_794_200_400));
        assert!(found[0].ready, "a to-do marked ready reads as ready");
        assert_eq!(found[0].done_when, Some(Knowing::Check("true".into())));
        assert_eq!(found[1].key, "QEST-2:b");
        assert_eq!(found[1].done_when, Some(Knowing::Person));
        assert_eq!(
            found[1].ready_when,
            Some(Knowing::Signal(Signal::Switch(Some("abc".into()))))
        );
        assert_eq!(
            found[0].item["quest"], "QEST-1",
            "an item held whole names its file's quest"
        );
        assert_eq!(
            found[1].item["quest"], "QEST-2",
            "an item naming its own quest keeps it"
        );
        assert_eq!(found[1].item["title"], "B", "the item is held whole");
        assert_eq!(found[2].key, "QEST-1:Only a title");
        assert_eq!(found[2].done_when, None);
    }

    #[test]
    fn only_one_something_here_can_see_is_judged() {
        let file = r#"{"quest":"Q","items":[
            {"id":"a","doneWhen":{"by":"person"}},
            {"id":"b","doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}},
            {"id":"c","ready":true,"doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}},
            {"id":"d","doneWhen":{"check":"true"}}]}"#;
        let judged: Vec<bool> = in_file(file).iter().map(ToDo::judged).collect();
        assert_eq!(judged, [false, true, false, true]);
    }

    #[test]
    fn only_an_owner_said_to_be_outside_any_sandbox_is_open() {
        let file = r#"{"quest":"Q","items":[
            {"id":"a","owner":{"sandbox":"none"}},
            {"id":"b","owner":{"sandbox":"strict"}},
            {"id":"c","owner":{"address":"ABC123"}},
            {"id":"d","owner":{"sandbox":"loose"}},
            {"id":"e"}]}"#;
        let sandboxes: Vec<Sandbox> = in_file(file).iter().map(|t| t.sandbox).collect();
        assert_eq!(
            sandboxes,
            [
                Sandbox::Open,
                Sandbox::Strict,
                Sandbox::Unsaid,
                Sandbox::Unsaid,
                Sandbox::Unsaid
            ]
        );
    }

    #[test]
    fn no_json_holds_no_to_dos() {
        assert_eq!(in_file("{\"v\":2,"), Vec::<ToDo>::new());
        let keys: Vec<String> = in_file(
            r#"{"quest":"Q","items":[{"id":5,"title":"A"},{"id":6.0,"title":"B"},
                {"id":null,"title":"C"},{"title":"D"},{"id":true,"title":"E"}]}"#,
        )
        .into_iter()
        .map(|todo| todo.key)
        .collect();
        assert_eq!(
            keys,
            ["Q:5", "Q:6", "Q:C", "Q:D"],
            "an id as Waiting.js's identity writes it, its title where it has none"
        );
        assert_eq!(in_file("[]"), Vec::<ToDo>::new());
    }
}
