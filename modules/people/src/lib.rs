//! The people daemon's own logic, apart from its sockets and processes so
//! its tests reach it: what a client asks ([`Ask`]), what a source says
//! ([`Heard`]), and the book of everyone heard of ([`Book`]), each person
//! a record keyed by their lowercased address, holding what each source
//! said of them as it said it, unmerged: the shell's Person.js merges.
//!
//! The interface is version 1 of the shell's daemons
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use ready_rows::fuzzy::{Item, Typed};
use serde_json::{Value, json};

pub mod between;
pub mod drawer;
pub mod held;
pub mod listing;
pub mod name;

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The one kind of record this daemon keeps.
pub const KIND: &str = "person";

/// Those listed with nothing typed, as People shows them, each a person
/// record of this kind and one record of their keys in order (listing.rs).
pub const LISTING: &str = "listing";

/// A person's key, by an address in any case.
pub fn key(address: &str) -> String {
    format!("person:{}", address.trim().to_lowercase())
}

/// The address a person's key names, or none for a key of another kind.
pub fn address(key: &str) -> Option<&str> {
    key.strip_prefix("person:")
        .filter(|address| !address.is_empty())
}

/// Whether an address is a calendar's own rather than anyone's, as Google
/// Calendar gives a shared calendar or a room that organizes a meeting.
/// Nobody is reached there, so it's held as no one.
pub fn calendar(address: &str) -> bool {
    const CALENDARS: [&str; 4] = [
        "@group.calendar.google.com",
        "@group.v.calendar.google.com",
        "@import.calendar.google.com",
        "@resource.calendar.google.com",
    ];
    CALENDARS.iter().any(|domain| address.ends_with(domain))
}

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Records by key, waiting at most `patience` milliseconds for sources
    /// still asking about one.
    Get {
        id: Value,
        keys: Vec<String>,
        patience: u64,
    },
    /// Records by key as they're held, asking no source, as the shell asks
    /// who an address is that nothing it holds names.
    Known {
        id: Value,
        keys: Vec<String>,
    },
    /// Every record held, in the order heard, at most `limit`, or only
    /// those a source lists.
    List {
        id: Value,
        limit: Option<usize>,
        listed: bool,
    },
    /// Records as they change, these keys or all of them, or only those a
    /// source lists, and each that stops being listed.
    Watch {
        id: Value,
        keys: Option<Vec<String>>,
        listed: bool,
    },
    /// Those whose name or address the words find, best first, at most
    /// `limit`.
    Search {
        id: Value,
        words: String,
        limit: usize,
    },
    /// Those listed, merged, stood and in order, once, or watched as they
    /// change.
    Listing {
        id: Value,
        watch: bool,
    },
    Unwatch {
        id: Value,
    },
    Health {
        id: Value,
    },
    /// A kind this daemon doesn't keep.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// An ask this daemon doesn't know, or a line that isn't one.
    Unknown {
        id: Value,
        ask: String,
    },
}

/// What a line from a client asks.
pub fn ask(line: &str) -> Ask {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Ask::Unknown {
            id: Value::Null,
            ask: String::new(),
        };
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
    let keys = || {
        said.get("keys").and_then(Value::as_array).map(|keys| {
            keys.iter()
                .filter_map(Value::as_str)
                .filter_map(address)
                .map(key)
                .collect::<Vec<_>>()
        })
    };
    let kind = said.get("kind").and_then(Value::as_str).unwrap_or(KIND);
    let listed = said.get("listed").and_then(Value::as_bool) == Some(true);
    if matches!(asked, "list" | "watch") && kind == LISTING {
        return Ask::Listing {
            id,
            watch: asked == "watch",
        };
    }
    if matches!(asked, "list" | "watch" | "search") && kind != KIND {
        return Ask::UnknownKind {
            id,
            kind: kind.to_owned(),
        };
    }
    match asked {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys().unwrap_or_default(),
            patience: said.get("patience").and_then(Value::as_u64).unwrap_or(0),
        },
        "known" => Ask::Known {
            id,
            keys: keys().unwrap_or_default(),
        },
        "list" => Ask::List {
            id,
            limit: said
                .get("limit")
                .and_then(Value::as_u64)
                .and_then(|limit| usize::try_from(limit).ok()),
            listed,
        },
        "watch" => Ask::Watch {
            id,
            keys: keys(),
            listed,
        },
        "search" => Ask::Search {
            id,
            words: said
                .get("words")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            limit: said
                .get("limit")
                .and_then(Value::as_u64)
                .and_then(|limit| usize::try_from(limit).ok())
                .unwrap_or(SEARCHED),
        },
        "unwatch" => Ask::Unwatch { id },
        "health" => Ask::Health { id },
        other => Ask::Unknown {
            id,
            ask: other.to_owned(),
        },
    }
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// How many a search answers when it doesn't say.
pub const SEARCHED: usize = 50;

/// What a source said, one line of its protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    /// What it knows of someone, its line less the address. One it knows
    /// by no address is known by its `id`, as that source's alone.
    Person { address: String, entry: Value },
    /// It no longer knows someone, as when a contact is deleted.
    Gone { address: String, id: Option<String> },
    /// It says everyone it knows without being asked, so it's never
    /// waited on: its first line, `{"roster": true}`.
    Roster,
    /// It has nothing to say of someone yet, and will be asked again.
    Later { address: String },
    /// Its sign-in was refused, or there's none: signing in again mends it.
    SignedOut,
    /// A line it says that this doesn't read.
    Other,
}

/// What a source's line says.
pub fn heard(line: &str) -> Heard {
    let Ok(Value::Object(mut said)) = serde_json::from_str::<Value>(line) else {
        return Heard::Other;
    };
    if said.get("signedOut").and_then(Value::as_bool) == Some(true)
        || said.get("signedIn").and_then(Value::as_bool) == Some(false)
    {
        return Heard::SignedOut;
    }
    if said.get("roster").and_then(Value::as_bool) == Some(true) {
        return Heard::Roster;
    }
    let Some(Value::String(address)) = said.remove("email") else {
        return Heard::Other;
    };
    let address = address.trim().to_lowercase();
    let id = match said.get("id") {
        Some(Value::String(id)) if !id.is_empty() => Some(id.clone()),
        Some(Value::Number(id)) => Some(id.to_string()),
        _ => None,
    };
    if address.is_empty() && id.is_none() {
        return Heard::Other;
    }
    if said.get("gone").and_then(Value::as_bool) == Some(true) {
        return Heard::Gone { address, id };
    }
    if said.get("later").and_then(Value::as_bool) == Some(true) {
        return Heard::Later { address };
    }
    Heard::Person {
        address,
        entry: Value::Object(said),
    }
}

/// How a source stands, in the interface's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Starting,
    Answering,
    SignedOut,
    Refused(String),
}

impl Standing {
    pub fn word(&self) -> &'static str {
        match self {
            Standing::Starting => "starting",
            Standing::Answering => "answering",
            Standing::SignedOut => "signed-out",
            Standing::Refused(_) => "refused",
        }
    }
}

/// A stamp of what's served, as what every source says of someone:
/// FNV-1a over it as JSON, whose fields `serde_json` keeps sorted, so
/// it's the same in every run and every build, as std's hashers aren't
/// promised to be. Said as hex, since 64 bits are past what a JavaScript
/// number holds whole. Named `stamp` rather than `said`, which in these
/// same records means words, as how someone stands is said.
pub(crate) fn stamp<T: serde::Serialize + ?Sized>(what: &T) -> String {
    let text = serde_json::to_string(what).unwrap_or_default();
    let stamp = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |stamp, byte| {
        (stamp ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{stamp:016x}")
}

/// A person, as every source said of them last.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub key: String,
    pub sources: BTreeMap<String, Value>,
    /// The name their row shows (`name`), as the shell matches a row by,
    /// made ready to search as they're heard, since a search reads
    /// everyone.
    named: Text,
    /// Their address, the other text the shell matches a row by, or
    /// nothing for one a source keeps by its own id.
    mailed: Option<Text>,
    listed: bool,
    /// When the first thing with them began, a meeting or a mail, after
    /// which those listed list them too (listing.rs's `listed`).
    began: Option<f64>,
}

impl Record {
    fn of(key: String) -> Record {
        let mailed = address(&key)
            .filter(|kept| kept.contains('@'))
            .map(Text::of);
        Record {
            key,
            sources: BTreeMap::new(),
            named: Text::of(""),
            mailed,
            listed: false,
            began: None,
        }
    }

    /// Reads again what a search and a list read, as what's said changes,
    /// by the sources in the order they're consulted.
    fn indexed(&mut self, consulted: &[String]) {
        // Made ready again only once it reads otherwise, since most of
        // what a source says again leaves the name as it was.
        let name = self.name(consulted.iter().map(String::as_str));
        if name != self.named.text {
            self.named = Text::of(&name);
        }
        self.listed = self.sources.values().any(|entry| {
            ["favourite", "recent"]
                .iter()
                .any(|word| entry.get(word).and_then(Value::as_bool) == Some(true))
        });
        self.began = consulted
            .iter()
            .filter_map(|source| self.sources.get(source)?.get("spans")?.as_array())
            .flatten()
            .filter_map(|span| {
                let from = span.get("from")?.as_f64()?;
                let to = span.get("to")?.as_f64()?;
                (from.is_finite() && to.is_finite()).then_some(from)
            })
            .reduce(f64::min);
    }

    /// The name their row shows: the first a source gives, in the order
    /// the sources are consulted, that isn't only an address; or else one
    /// made of their address, or their key for one who has none.
    pub fn name<'a>(&self, consulted: impl IntoIterator<Item = &'a str>) -> String {
        let known = self.key.strip_prefix("person:").unwrap_or(&self.key);
        consulted
            .into_iter()
            .filter_map(|source| self.sources.get(source)?.get("name")?.as_str())
            .map(str::trim)
            .find(|said| !name::bare(said))
            .map_or_else(
                || match address(&self.key).filter(|kept| kept.contains('@')) {
                    Some(email) => name::by_address(email),
                    None => known.to_owned(),
                },
                str::to_owned,
            )
    }

    /// Whether those listed list them at `now`: as `listed`, or once
    /// something with them has begun.
    fn listed_at(&self, now: f64) -> bool {
        self.listed || self.began.is_some_and(|from| from <= now)
    }

    /// Served with a stamp of what its sources say, `stamp`, which moves
    /// only when that does, so the shell keeps what it made of them while
    /// it's the same (People.qml). Stamped with the moment they were
    /// heard instead, a daemon started again told everyone as changed,
    /// and Joel's next first letter made 136 and 137 people's rows again
    /// (his journal, 2026-11-14 09:38 and 17:07).
    pub fn json(&self) -> Value {
        json!({"key": self.key, "kind": KIND, "stamp": stamp(&self.sources),
               "sources": self.sources})
    }

    /// The answer for a key no source knows any more.
    pub fn gone(key: &str) -> Value {
        json!({"key": key, "kind": KIND, "gone": true})
    }

    /// Whether a source lists them with nothing typed: a favourite, or
    /// someone Joel dealt with lately.
    pub fn listed(&self) -> bool {
        self.listed
    }

    /// How well what's typed finds them, as the shell's Fuzzy.js scores
    /// a row: the better of the name it shows and their address, or
    /// nothing when neither is found. Each word has to start a word of
    /// one text, its letters unbroken, in any order and any case.
    fn score(&self, typed: &Typed) -> Option<i32> {
        let address = self
            .mailed
            .as_ref()
            .and_then(|text| text.ready.score(typed));
        self.named.ready.score(typed).max(address)
    }
}

/// A text made ready to match as the shell's Fuzzy.js matches it (the
/// ready rows' fuzzy.rs, held to its cases), kept with the text itself,
/// by which a record reads and compares. Shared, since a record is cloned
/// as it's served and the text is made ready once, as it's heard.
#[derive(Clone)]
struct Text {
    text: String,
    ready: Arc<Item>,
}

impl Text {
    fn of(text: &str) -> Text {
        Text {
            text: text.to_owned(),
            ready: Arc::new(Item::of(text)),
        }
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        self.text == other.text
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.text, f)
    }
}

/// Everyone heard of, and how each source stands.
#[derive(Debug, Default)]
pub struct Book {
    records: HashMap<String, Record>,
    /// Keys in the order they were first heard of, for `list`.
    order: Vec<String>,
    /// Which sources have answered for each key, a later aside.
    answered: HashMap<String, HashSet<String>>,
    standings: BTreeMap<String, Standing>,
    /// The sources that say everyone unasked, never waited on.
    rosters: HashSet<String>,
    /// The sources in the order they're consulted, for the name each
    /// person's row shows.
    consulted: Vec<String>,
}

/// The key of someone a source knows by an address, or else by its own
/// id for them, as Person.js keys them: `person:tuple:7`.
fn key_of(source: &str, address: &str, id: Option<&str>) -> String {
    match id {
        Some(id) if address.is_empty() => key(&format!("{source}:{id}")),
        _ => key(address),
    }
}

fn id_of(entry: &Value) -> Option<String> {
    match entry.get("id") {
        Some(Value::String(id)) if !id.is_empty() => Some(id.clone()),
        Some(Value::Number(id)) => Some(id.to_string()),
        _ => None,
    }
}

impl Book {
    /// A book that knows these sources, each starting.
    pub fn of(sources: &[String]) -> Book {
        Book {
            standings: sources
                .iter()
                .map(|source| (source.clone(), Standing::Starting))
                .collect(),
            consulted: sources.to_vec(),
            ..Book::default()
        }
    }

    /// Takes what a source said, and gives the key of a person whose
    /// record changed.
    pub fn take(&mut self, source: &str, heard: Heard) -> Option<String> {
        match heard {
            Heard::Person { address, entry } => {
                let entry = between::ready(entry);
                self.stand(source, Standing::Answering);
                let key = key_of(source, &address, id_of(&entry).as_deref());
                self.answered
                    .entry(key.clone())
                    .or_default()
                    .insert(source.to_owned());
                if calendar(&address) {
                    return None;
                }
                let record = self
                    .records
                    .entry(key.clone())
                    .or_insert_with(|| Record::of(key.clone()));
                if record.sources.get(source) == Some(&entry) {
                    return None;
                }
                if record.sources.is_empty() {
                    self.order.push(key.clone());
                }
                record.sources.insert(source.to_owned(), entry);
                record.indexed(&self.consulted);
                Some(key)
            }
            Heard::Later { .. } => {
                self.stand(source, Standing::Answering);
                None
            }
            Heard::Gone { address, id } => {
                let key = key_of(source, &address, id.as_deref());
                let record = self.records.get_mut(&key)?;
                record.sources.remove(source)?;
                record.indexed(&self.consulted);
                if record.sources.is_empty() {
                    self.records.remove(&key);
                    self.order.retain(|kept| kept != &key);
                    self.answered.remove(&key);
                }
                Some(key)
            }
            Heard::Roster => {
                self.rosters.insert(source.to_owned());
                self.stand(source, Standing::Answering);
                None
            }
            Heard::SignedOut => {
                self.stand(source, Standing::SignedOut);
                None
            }
            Heard::Other => None,
        }
    }

    /// The key a source's line is about, if it's about someone.
    pub fn about(&self, source: &str, heard: &Heard) -> Option<String> {
        match heard {
            Heard::Person { address, entry } => {
                Some(key_of(source, address, id_of(entry).as_deref()))
            }
            Heard::Gone { address, id } => Some(key_of(source, address, id.as_deref())),
            _ => None,
        }
    }

    pub fn stand(&mut self, source: &str, standing: Standing) {
        self.standings.insert(source.to_owned(), standing);
    }

    pub fn standings(&self) -> &BTreeMap<String, Standing> {
        &self.standings
    }

    pub fn record(&self, key: &str) -> Option<&Record> {
        self.records.get(key)
    }

    /// The records held for these keys, as the shell reads them: who an
    /// address is, as a meeting's guest whom nobody lists, asked of no
    /// source, which a step-in would send every source to look up.
    pub fn known(&self, keys: &[String]) -> Vec<&Record> {
        keys.iter().filter_map(|key| self.record(key)).collect()
    }

    /// Every record held, in the order each was first heard of.
    pub fn everyone(&self) -> impl Iterator<Item = &Record> {
        self.order.iter().filter_map(|key| self.records.get(key))
    }

    /// Those what's typed finds, at most `limit`, in the order the shell
    /// shows them (People.qml): those listed at `now` first, then the best
    /// found by Fuzzy.js's score, and among equals in the order heard. So
    /// the shell may show the answer as it comes, merging and ranking no
    /// one: ranked there, everyone is merged again at each answer, 13 ms
    /// of Joel's shell at p90 and 62 at most (his journal, 2026-11-14 to
    /// 17).
    pub fn search(&self, typed: &str, limit: usize, now: f64) -> Vec<&Record> {
        let typed = Typed::of(typed);
        if typed.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(bool, Reverse<i32>, usize, &Record)> = self
            .everyone()
            .enumerate()
            .filter_map(|(at, record)| {
                record
                    .score(&typed)
                    .map(|score| (!record.listed_at(now), Reverse(score), at, record))
            })
            .collect();
        found.sort_unstable_by_key(|&(unlisted, score, at, _)| (unlisted, score, at));
        found
            .into_iter()
            .take(limit)
            .map(|(.., record)| record)
            .collect()
    }

    /// Whether every source that can answer has answered for a key: one
    /// signed out or refused never will, so it isn't waited on.
    pub fn ready(&self, key: &str) -> bool {
        self.waiting(key).next().is_none()
    }

    /// The sources that can answer for a key and haven't yet, so a watch
    /// of those listed can tell which of them put the key off.
    pub fn waiting<'a>(&'a self, key: &str) -> impl Iterator<Item = &'a str> {
        let answered = self.answered.get(key);
        self.standings
            .iter()
            .filter(move |(source, standing)| {
                !(self.rosters.contains(*source)
                    || matches!(standing, Standing::SignedOut | Standing::Refused(_))
                    || answered.is_some_and(|answered| answered.contains(*source)))
            })
            .map(|(source, _)| source.as_str())
    }

    /// Forgets which keys a source answered, as when it starts again, so
    /// what it says after is waited on afresh. Once running it can be
    /// asked, so it's answering, though it says nothing until it is: a
    /// source is only starting until its program has started.
    pub fn restarted(&mut self, source: &str) {
        for answered in self.answered.values_mut() {
            answered.remove(source);
        }
        // It says again whether it's a roster.
        self.rosters.remove(source);
        self.stand(source, Standing::Answering);
    }
}

/// Milliseconds since the epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
