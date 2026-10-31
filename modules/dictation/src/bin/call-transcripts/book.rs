//! Every call heard, a record each, kept for the shell, which asks for
//! them over a socket in version 1 of its daemons' interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"), as the people daemon
//! answers it: JSON lines, each ask with an `id` echoed in its answers;
//! hello, get, list, watch, unwatch and health.
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

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// The interface's version this speaks.
const VERSION: u64 = 1;

/// The one kind of record this daemon keeps.
const KIND: &str = "call";

/// What a call's folder says of it beside its files.
const META: &str = "meta.json";

/// The transcript's file in a call's folder.
pub const TRANSCRIPT: &str = "transcript.md";

/// The summary's file in a call's folder.
pub const SUMMARY: &str = "summary.md";

/// The milliseconds since the epoch, as the interface gives times.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

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

/// Writes `text` to `file` whole: into a file beside it, then renamed over
/// it, so nothing ever reads half of it.
pub fn write_whole(file: &Path, text: &str) -> std::io::Result<()> {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let part = file.with_file_name(format!(".{name}.part"));
    std::fs::write(&part, text)?;
    std::fs::rename(&part, file)
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The keys watched, or none for every call.
    keys: Option<Vec<String>>,
    told: Sender<String>,
}

/// Every call, newest first, and who watches them.
#[derive(Default)]
pub struct Book {
    records: Vec<Record>,
    watches: Vec<Watch>,
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
            watches: Vec::new(),
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
        let line = |id: &Value| json!({"id": id, "changed": [record.json()]}).to_string();
        self.watches.retain(|watch| {
            let watched = watch
                .keys
                .as_ref()
                .is_none_or(|keys| keys.contains(&record.key));
            !watched || watch.told.send(line(&watch.id)).is_ok()
        });
        self.records.retain(|held| held.key != record.key);
        let at = self
            .records
            .iter()
            .position(|held| held.started < record.started)
            .unwrap_or(self.records.len());
        self.records.insert(at, record);
    }

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
        let keys = said["keys"].as_array().map(|keys| {
            keys.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });
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
                let limit = said["limit"]
                    .as_u64()
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(usize::MAX);
                let mut listed = records(None);
                listed.truncate(limit);
                json!({"id": id, "records": listed})
            }
            "watch" => {
                let held = records(keys.as_ref());
                // A watch asked again by its id replaces the one before.
                self.watches
                    .retain(|watch| !(watch.connection == connection && watch.id == id));
                self.watches.push(Watch {
                    connection,
                    id: id.clone(),
                    keys,
                    told: told.clone(),
                });
                json!({"id": id, "records": held})
            }
            "unwatch" => {
                self.watches
                    .retain(|watch| !(watch.connection == connection && watch.id == id));
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
}

/// An error answer, by the interface's codes.
fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// The book, shared between the threads.
pub struct Shared(Mutex<Book>);

impl Shared {
    pub fn new(book: Book) -> Self {
        Self(Mutex::new(book))
    }

    pub fn lock(&self) -> MutexGuard<'_, Book> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The socket, in a folder only this account reaches.
pub fn listen(socket: &Path) -> Result<UnixListener, String> {
    let made = || -> std::io::Result<UnixListener> {
        if let Some(folder) = socket.parent() {
            std::fs::create_dir_all(folder)?;
            std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
        }
        let _ = std::fs::remove_file(socket);
        UnixListener::bind(socket)
    };
    made().map_err(|why| format!("listening on {}: {why}", socket.display()))
}

/// Answers every client, each on a thread of its own.
pub fn answer_all(book: &std::sync::Arc<Shared>, listener: &UnixListener) {
    for (connection, stream) in (0..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let book = std::sync::Arc::clone(book);
        std::thread::spawn(move || serve(&book, stream, connection));
    }
}

/// Answers one client until it goes.
fn serve(book: &Shared, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let (told, lines) = mpsc::channel::<String>();
    let mut writing = stream;
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let answer = book.lock().answer(&line, connection, &told);
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    book.lock()
        .watches
        .retain(|watch| watch.connection != connection);
}

#[cfg(test)]
mod tests {
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
