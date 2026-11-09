//! The to-dos, as each quest's file in `mission/waiting` says them: what
//! makes each done, and what makes it ready, as its owner wrote it through
//! `mesh_wait`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

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
    /// `<quest>:<id>`.
    pub key: String,
    /// When it was raised, in seconds since 1970, which a boot, a switch or
    /// a sign-in must come after.
    pub raised: Option<i64>,
    /// Whether its owner already says it can be done now.
    pub ready: bool,
    pub done_when: Option<Knowing>,
    pub ready_when: Option<Knowing>,
    /// Whether its owner runs in the strict sandbox, whose checks would
    /// run here outside it, and so aren't run.
    pub strict: bool,
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
            let id = item
                .get("id")
                .or_else(|| item.get("title"))
                .and_then(Value::as_str)?;
            let quest = item.get("quest").and_then(Value::as_str).unwrap_or(quest);
            Some(ToDo {
                key: format!("{quest}:{id}"),
                raised: item
                    .get("raisedAt")
                    .and_then(Value::as_str)
                    .and_then(seconds),
                ready: item.get("ready").and_then(Value::as_bool) == Some(true),
                done_when: item.get("doneWhen").and_then(knowing),
                ready_when: item.get("readyWhen").and_then(knowing),
                strict: item.pointer("/owner/sandbox").and_then(Value::as_str) == Some("strict"),
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
        .filter_map(|path| fs::read_to_string(path).ok())
        .flat_map(|text| in_file(&text))
        .collect()
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
    fn no_json_holds_no_to_dos() {
        assert_eq!(in_file("{\"v\":2,"), Vec::<ToDo>::new());
        assert_eq!(in_file("[]"), Vec::<ToDo>::new());
    }
}
