//! Every call heard, a record each, kept for the shell, which asks for
//! them over a socket in version 1 of its daemons' interface
//! (dictation's daemon.rs): hello, get, list, watch, unwatch and health.
//!
//! A record is kind "call", keyed `call:<event>` for a meeting, by its
//! calendar event, `call:person:<address>:<started>` for a Tuple call, by
//! who it's with, or `call:at:<started>` for a call with neither; its
//! fields are the title, when it started and ended (null while it's on),
//! the event or person, its folder, its transcript's path once there's one, its summary
//! once written, and whether it's being transcribed. Each call's folder
//! holds a meta.json saying what isn't in its files, so the calls are read
//! back as the daemon starts, and only then: the shell never reads the
//! folder.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::UNIX_EPOCH;

use dictation::daemon::{self, Answers, VERSION, Watches, error};
pub use dictation::daemon::{now, write_whole};
use serde_json::{Value, json};

/// The book, shared between the daemon's threads.
pub type Shared = daemon::Shared<Book>;

/// The one kind of record this daemon keeps.
const KIND: &str = "call";

/// What a call's folder says of it beside its files.
const META: &str = "meta.json";

/// The transcript's file in a call's folder.
pub const TRANSCRIPT: &str = "transcript.md";

/// The summary's file in a call's folder.
pub const SUMMARY: &str = "summary.md";

/// One call.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub key: String,
    pub title: String,
    pub started: u64,
    pub ended: Option<u64>,
    pub event: Option<String>,
    pub person: Option<String>,
    pub folder: PathBuf,
    pub transcript: Option<PathBuf>,
    pub summary: Option<String>,
    pub transcribing: bool,
}

impl Record {
    /// The key of a call started at `started`, in a meeting, with a
    /// person, or neither.
    pub fn key_for(event: Option<&str>, person: Option<&str>, started: u64) -> String {
        match (event, person) {
            (Some(event), _) => format!("call:{event}"),
            (None, Some(person)) => format!("call:person:{person}:{started}"),
            (None, None) => format!("call:at:{started}"),
        }
    }

    fn json(&self) -> Value {
        json!({
            "key": self.key, "kind": KIND, "title": self.title, "started": self.started,
            "ended": self.ended, "event": self.event, "person": self.person,
            "folder": self.folder, "transcript": self.transcript, "summary": self.summary,
            "transcribing": self.transcribing,
        })
    }

    fn meta(&self) -> Value {
        json!({
            "key": self.key, "title": self.title, "started": self.started,
            "ended": self.ended, "event": self.event, "person": self.person,
        })
    }

    /// The call a folder holds, from its meta.json and its files. One the
    /// daemon stopped during is taken as ended when its folder last
    /// changed.
    fn read(folder: &Path) -> Option<Self> {
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(META)).ok()?).ok()?;
        let changed = || {
            std::fs::metadata(folder.join(TRANSCRIPT))
                .or_else(|_| std::fs::metadata(folder))
                .and_then(|data| data.modified())
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .and_then(|since| u64::try_from(since.as_millis()).ok())
        };
        let transcript = folder.join(TRANSCRIPT);
        Some(Self {
            key: meta["key"].as_str()?.to_owned(),
            title: meta["title"].as_str().unwrap_or_default().to_owned(),
            started: meta["started"].as_u64()?,
            ended: meta["ended"].as_u64().or_else(changed),
            event: meta["event"].as_str().map(str::to_owned),
            person: meta["person"].as_str().map(str::to_owned),
            folder: folder.to_owned(),
            transcript: transcript.exists().then_some(transcript),
            summary: std::fs::read_to_string(folder.join(SUMMARY)).ok(),
            transcribing: false,
        })
    }
}

/// Every call, newest first, and who watches them.
#[derive(Default)]
pub struct Book {
    records: Vec<Record>,
    watches: Watches,
    since: u64,
}

impl Book {
    /// The calls kept in `calls`, each folder's own.
    pub fn read(calls: &Path) -> Self {
        let mut records: Vec<Record> = std::fs::read_dir(calls)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| Record::read(&entry.path()))
            .collect();
        records.sort_by_key(|record| std::cmp::Reverse(record.started));
        Self {
            records,
            watches: Watches::default(),
            since: now(),
        }
    }

    pub fn record(&self, key: &str) -> Option<&Record> {
        self.records.iter().find(|record| record.key == key)
    }

    /// Keeps `record`, in place of the one with its key, writing its
    /// meta.json and telling each watch of it.
    pub fn put(&mut self, record: Record) {
        if let Err(why) = std::fs::create_dir_all(&record.folder)
            .and_then(|()| write_whole(&record.folder.join(META), &record.meta().to_string()))
        {
            eprintln!(
                "call-transcripts: keeping {}: {why}",
                record.folder.display()
            );
        }
        self.watches.tell(&record.key, &record.json());
        self.records.retain(|held| held.key != record.key);
        let at = self
            .records
            .iter()
            .position(|held| held.started < record.started)
            .unwrap_or(self.records.len());
        self.records.insert(at, record);
    }
}

impl Answers for Book {
    /// What one line from a client asks, answered.
    fn answer(&mut self, line: &str, connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "not a JSON line");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said["ask"].as_str().unwrap_or_default();
        let kind = said["kind"].as_str().unwrap_or(KIND);
        if matches!(asked, "list" | "watch") && kind != KIND {
            let why = format!("this daemon keeps only {KIND}, not {kind}");
            return error(&id, "unknown-kind", &why);
        }
        let keys = daemon::keys(&said);
        let records = |keys: Option<&Vec<String>>| -> Vec<Value> {
            self.records
                .iter()
                .filter(|record| keys.is_none_or(|keys| keys.contains(&record.key)))
                .map(Record::json)
                .collect()
        };
        match asked {
            "hello" => json!({
                "id": id, "version": VERSION, "domain": "calls", "state": "answering",
                "since": self.since,
            }),
            "get" => json!({"id": id, "records": records(Some(&keys.unwrap_or_default()))}),
            "list" => {
                let mut listed = records(None);
                listed.truncate(daemon::limit(&said));
                json!({"id": id, "records": listed})
            }
            "watch" => {
                let held = records(keys.as_ref());
                self.watches.watch(connection, &id, keys, told);
                json!({"id": id, "records": held})
            }
            "unwatch" => {
                self.watches.unwatch(connection, &id);
                json!({"id": id, "unwatched": true})
            }
            "health" => json!({
                "id": id, "version": VERSION, "state": "answering",
                "calls": self.records.len(), "watching": self.watches.len(),
                "transcribing": self.records.iter().filter(|record| record.transcribing).count(),
            }),
            other => error(&id, "unknown-ask", &format!("no ask {other:?}")),
        }
    }

    fn gone(&mut self, connection: u64) {
        self.watches.gone(connection);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn call(folder: &Path, key: &str, started: u64) -> Record {
        Record {
            key: key.to_owned(),
            title: "Planning".to_owned(),
            started,
            ended: None,
            event: Some("e1".to_owned()),
            person: None,
            folder: folder.join(key),
            transcript: None,
            summary: None,
            transcribing: true,
        }
    }

    fn ask(book: &mut Book, line: &str) -> (Value, mpsc::Receiver<String>) {
        let (told, heard) = mpsc::channel();
        (book.answer(line, 1, &told), heard)
    }

    #[test]
    fn a_watch_hears_each_call_as_it_changes_and_lists_are_newest_first() {
        let folder = tempfile::tempdir().unwrap();
        let mut book = Book::default();
        let (told, heard) = mpsc::channel();
        let held = book.answer(r#"{"id": 4, "ask": "watch", "kind": "call"}"#, 1, &told);
        assert_eq!(held, json!({"id": 4, "records": []}));
        book.put(call(folder.path(), "call:e1", 10));
        book.put(call(folder.path(), "call:e2", 20));
        let changed: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(changed["id"], 4);
        assert_eq!(changed["changed"][0]["key"], "call:e1");
        assert_eq!(changed["changed"][0]["transcribing"], true);
        let (listed, _) = ask(&mut book, r#"{"id": 5, "ask": "list", "limit": 1}"#);
        assert_eq!(listed["records"].as_array().unwrap().len(), 1);
        assert_eq!(listed["records"][0]["key"], "call:e2");
    }

    #[test]
    fn a_call_is_read_back_from_its_folder_and_one_cut_short_has_ended() {
        let folder = tempfile::tempdir().unwrap();
        let mut book = Book::default();
        let mut record = call(folder.path(), "call:e1", 10);
        book.put(record.clone());
        std::fs::write(record.folder.join(TRANSCRIPT), "# Transcript\n").unwrap();
        let read = Book::read(folder.path());
        let back = read.record("call:e1").unwrap();
        assert!(back.ended.is_some(), "the daemon stopped during it");
        assert!(!back.transcribing);
        assert_eq!(back.transcript, Some(record.folder.join(TRANSCRIPT)));
        record.ended = Some(99);
        book.put(record);
        assert_eq!(
            Book::read(folder.path()).record("call:e1").unwrap().ended,
            Some(99)
        );
    }

    #[test]
    fn another_kind_or_ask_is_refused_by_its_code() {
        let mut book = Book::default();
        let (kind, _) = ask(&mut book, r#"{"id": 1, "ask": "list", "kind": "person"}"#);
        assert_eq!(kind["error"]["code"], "unknown-kind");
        let (other, _) = ask(&mut book, r#"{"id": 2, "ask": "shout"}"#);
        assert_eq!(other["error"]["code"], "unknown-ask");
        let (hello, _) = ask(&mut book, r#"{"id": 3, "ask": "hello", "version": 1}"#);
        assert_eq!(hello["domain"], "calls");
    }
}
