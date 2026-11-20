//! Every call heard, a record each, kept for the shell, which asks for
//! them over a socket in version 1 of its daemons' interface
//! (dictation's daemon.rs): hello, get, list, watch, unwatch and health.
//!
//! A record is kind "call", keyed `call:<event>` for a meeting, by its
//! calendar event, `call:person:<address>:<started>` for a Tuple call, by
//! who it's with, or `call:at:<started>` for a call with neither; its
//! fields are the title, when it started and ended (null while it's on),
//! the event or person, its folder, its transcript's path once there's one, how the
//! transcript opens (its first few paragraphs, so the launcher can show
//! them without reading the folder), its summary once written, and
//! whether it's being transcribed. Each call's folder holds its one
//! file, its summary over its transcript (filed.rs), and a meta.json
//! saying what isn't in it, so the calls are read back as the daemon
//! starts, and only then: the shell never reads the folder. A folder
//! from before keeps its summary in summary.md, beside its transcript.

use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::UNIX_EPOCH;

pub use dictation::daemon::now;
use dictation::daemon::{self, Answers, VERSION, Watches, error};

use crate::filed;
use crate::meeting::{self, Known};
use crate::private::make_folder;
pub use crate::private::write_whole;
use serde_json::{Value, json};

/// The book, shared between the daemon's threads.
pub type Shared = daemon::Shared<Book>;

/// The one kind of record this daemon keeps.
const KIND: &str = "call";

/// What a call's folder says of it beside its files.
const META: &str = "meta.json";

/// The most of a meta.json read: it holds a title and a few keys, so one
/// larger was written by something else, and is skipped.
const LARGEST_META: u64 = 64 * 1024;

/// A call's folder whose meta.json couldn't be read: too large, not JSON,
/// or lacking its key or its start.
#[derive(Debug)]
struct Unreadable;

/// A call's one file in its folder, its summary over its transcript. Its
/// name is the transcript's, as it was before the summary joined it, so
/// what opened, copied or followed it before still finds it.
pub const TRANSCRIPT: &str = "transcript.md";

/// The summary's own file in the folder of a call from before the one
/// file, read while it's there and let go once the call's summed up
/// again.
pub const SUMMARY: &str = "summary.md";

/// How many of the transcript's paragraphs its opening holds, and how many
/// characters at most: enough to tell one call from another in the
/// launcher's drawer, and few enough that the record stops changing once
/// the call is a minute or two in.
const OPENING_PARAGRAPHS: usize = 4;
const OPENING_CHARACTERS: usize = 400;

/// How a transcript opens: its first few paragraphs of what was said, as
/// markdown, cut at a word once it's long; none while nothing's said. Its
/// heading and its note of who was on the call aren't what was said.
pub fn opening(transcript: &str) -> Option<String> {
    let said: Vec<&str> = transcript
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| paragraph.starts_with("**"))
        .take(OPENING_PARAGRAPHS)
        .collect();
    if said.is_empty() {
        return None;
    }
    let whole = said.join("\n\n");
    if whole.chars().count() <= OPENING_CHARACTERS {
        return Some(whole);
    }
    let cut: String = whole.chars().take(OPENING_CHARACTERS).collect();
    let kept = cut
        .rfind(char::is_whitespace)
        .map_or(cut.as_str(), |at| &cut[..at]);
    Some(format!("{}\u{2026}", kept.trim_end()))
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
    /// The Google account its meeting was on, by address, as the google
    /// program's daemon said it both before and after listing the
    /// meetings (meeting.rs); none when either read went unheard or they
    /// differed, or for a call before this was kept. This daemon writes
    /// it, beside the call, from nothing an agent sends, and the shell is
    /// never served it. It's read back as written: Joel's own files hold
    /// no secret from what runs as him, which could run mycelium itself.
    pub account: Option<String>,
    /// The others on it, by name, and how they're known (meeting.rs);
    /// none for a call nothing said who was on, or kept before this was.
    pub people: Vec<String>,
    pub known: Option<Known>,
    pub folder: PathBuf,
    pub transcript: Option<PathBuf>,
    pub opening: Option<String>,
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
            "folder": self.folder, "transcript": self.transcript, "opening": self.opening,
            "summary": self.summary, "transcribing": self.transcribing,
        })
    }

    fn meta(&self) -> Value {
        json!({
            "key": self.key, "title": self.title, "started": self.started,
            "ended": self.ended, "event": self.event, "person": self.person,
            "account": self.account, "people": self.people,
            "known": self.known.map(Known::as_str),
        })
    }

    /// What the others on the call are called in its transcript and
    /// summary, where that's certain: the one other person on it, as long
    /// as they aren't called what Joel, `me`, is. A call kept before its
    /// people were falls back on a Tuple call with one person, titled by
    /// their name. With several, or none known, they're Others.
    pub fn them(&self, me: &str) -> Option<String> {
        let one = match (self.people.as_slice(), self.known) {
            ([one], Some(_)) => Some(one.clone()),
            ([], None) => self
                .person
                .as_ref()
                .filter(|_| !self.title.ends_with(" more"))
                .and_then(|_| meeting::speaker(&self.title)),
            _ => None,
        };
        one.filter(|name| !name.is_empty() && name != me)
    }

    /// What the transcript says first of who the others were: where the
    /// name their side is under came from, or who was invited or on the
    /// Tuple call when their side stays Others; none when nobody's known.
    pub fn note(&self, me: &str) -> Option<String> {
        let known = self.known?;
        if let Some(them) = self.them(me).filter(|_| self.people.len() == 1) {
            let whose = match known {
                Known::Tuple => "on the Tuple call",
                Known::Invitation => "invited",
            };
            return Some(format!(
                "Their side is {them}, the one other person {whose}."
            ));
        }
        let listed = listing(&self.people)?;
        Some(match known {
            Known::Tuple => format!("On the Tuple call: {listed}."),
            Known::Invitation => format!("Invited: {listed}."),
        })
    }

    /// The call a folder holds, from its meta.json and its files, or none
    /// for a folder with no meta.json. One the daemon stopped during is
    /// taken as ended when its folder last changed.
    fn read(folder: &Path) -> Result<Option<Self>, Unreadable> {
        let mut text = String::new();
        // Not followed if it's a link, and not waited on if it's a pipe:
        // only a plain file is a record.
        let opened = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(folder.join(META));
        match opened {
            Ok(file) => {
                if !file.metadata().is_ok_and(|held| held.is_file()) {
                    return Err(Unreadable);
                }
                file.take(LARGEST_META + 1)
                    .read_to_string(&mut text)
                    .map_err(|_| Unreadable)?;
            }
            // A folder with no record yet, or a file beside the calls'
            // folders, holds no call.
            Err(why)
                if matches!(
                    why.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(None);
            }
            Err(_) => return Err(Unreadable),
        }
        if !u64::try_from(text.len()).is_ok_and(|read| read <= LARGEST_META) {
            return Err(Unreadable);
        }
        let meta: Value = serde_json::from_str(&text).map_err(|_| Unreadable)?;
        let (Some(key), Some(started)) = (meta["key"].as_str(), meta["started"].as_u64()) else {
            return Err(Unreadable);
        };
        let changed = || {
            std::fs::metadata(folder.join(TRANSCRIPT))
                .or_else(|_| std::fs::metadata(folder))
                .and_then(|data| data.modified())
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .and_then(|since| u64::try_from(since.as_millis()).ok())
        };
        let transcript = folder.join(TRANSCRIPT);
        let text = std::fs::read_to_string(&transcript).ok();
        let opening = text
            .as_deref()
            .and_then(|text| opening(filed::transcript(text)));
        // A summary.md beside a call's one file is left from before, when
        // a call rejoined isn't summed up again yet, so only the one file
        // says it's summed up. Either is kept without its title, which
        // the launcher's drawer says itself.
        let summary = match text.as_deref() {
            Some(text) if filed::is_one(text) => filed::summary(text),
            _ => std::fs::read_to_string(folder.join(SUMMARY))
                .ok()
                .map(|summary| filed::untitled(&summary).trim().to_owned()),
        };
        Ok(Some(Self {
            key: key.to_owned(),
            title: meta["title"].as_str().unwrap_or_default().to_owned(),
            started,
            ended: meta["ended"].as_u64().or_else(changed),
            event: meta["event"].as_str().map(str::to_owned),
            person: meta["person"].as_str().map(str::to_owned),
            account: meta["account"].as_str().map(str::to_owned),
            people: meta["people"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect(),
            known: meta["known"].as_str().and_then(Known::read),
            folder: folder.to_owned(),
            transcript: transcript.exists().then_some(transcript),
            opening,
            summary,
            transcribing: false,
        }))
    }
}

/// The most people a note names before it says how many more there are.
const LISTED: usize = 8;

/// People as a sentence lists them, "Ada, Bo and Cy", the first few of
/// many and how many more; or none for nobody.
fn listing(people: &[String]) -> Option<String> {
    let (named, more) = if people.len() > LISTED {
        (&people[..LISTED], people.len() - LISTED)
    } else {
        (people, 0)
    };
    let mut listed = named.to_vec();
    if more > 0 {
        listed.push(format!("{more} more"));
    }
    let last = listed.pop()?;
    Some(if listed.is_empty() {
        last
    } else {
        format!("{} and {last}", listed.join(", "))
    })
}

/// Every call, newest first, and who watches them.
#[derive(Default)]
pub struct Book {
    records: Vec<Record>,
    watches: Watches,
    since: u64,
    /// Whether the calls' folder couldn't be listed at the start.
    unlisted: bool,
    /// How many calls' folders held a meta.json that couldn't be read.
    unreadable: usize,
}

impl Book {
    /// The calls kept in `calls`, each folder's own.
    pub fn read(calls: &Path) -> Self {
        let (listed, unlisted) = match std::fs::read_dir(calls) {
            Ok(listed) => (Some(listed), false),
            // None yet, before the first call.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => (None, false),
            Err(_) => (None, true),
        };
        let mut records = Vec::new();
        let mut unreadable = 0;
        for entry in listed.into_iter().flatten().filter_map(Result::ok) {
            match Record::read(&entry.path()) {
                Ok(Some(record)) => records.push(record),
                Ok(None) => {}
                Err(Unreadable) => unreadable += 1,
            }
        }
        records.sort_by_key(|record| std::cmp::Reverse(record.started));
        Self {
            records,
            watches: Watches::default(),
            since: now(),
            unlisted,
            unreadable,
        }
    }

    pub fn record(&self, key: &str) -> Option<&Record> {
        self.records.iter().find(|record| record.key == key)
    }

    /// When each call started, for the calls receptor, which is given
    /// nothing else of them.
    pub fn starts(&self) -> impl Iterator<Item = u64> + '_ {
        self.records.iter().map(|record| record.started)
    }

    /// The calendar event of each ended call whose summary was written and
    /// whose meeting was on an account `on` takes, with when it ended and
    /// its folder, for the calls receptor, which says the event alone and
    /// looks in the folder only to see the summary is still there. A call
    /// with no account kept is never listed, whatever `on` takes.
    pub fn summarized<'a>(
        &'a self,
        on: impl Fn(&str) -> bool + 'a,
    ) -> impl Iterator<Item = (&'a str, u64, &'a Path)> + 'a {
        self.records.iter().filter_map(move |record| {
            record.summary.as_ref()?;
            if !on(record.account.as_deref()?) {
                return None;
            }
            Some((
                record.event.as_deref()?,
                record.ended?,
                record.folder.as_path(),
            ))
        })
    }

    /// Whether the calls' folder couldn't be listed, or a call's record in
    /// it couldn't be read, as it was read at the start.
    pub fn failing(&self) -> bool {
        self.unlisted || self.unreadable > 0
    }

    /// Keeps `record`, in place of the one with its key, writing its
    /// meta.json and telling each watch of it.
    pub fn put(&mut self, record: Record) {
        if let Err(why) = make_folder(&record.folder)
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
#[path = "book_tests.rs"]
mod tests;
