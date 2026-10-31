//! The people daemon's own logic, apart from its sockets and processes so
//! its tests reach it: what a client asks ([`Ask`]), what a source says
//! ([`Heard`]), and the book of everyone heard of ([`Book`]), each person
//! a record keyed by their lowercased address, holding what each source
//! said of them as it said it, unmerged: the shell's Person.js merges.
//!
//! The interface is version 1 of the shell's daemons
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The one kind of record this daemon keeps.
pub const KIND: &str = "person";

/// A person's key, by an address in any case.
pub fn key(address: &str) -> String {
    format!("person:{}", address.trim().to_lowercase())
}

/// The address a person's key names, or none for a key of another kind.
pub fn address(key: &str) -> Option<&str> {
    key.strip_prefix("person:")
        .filter(|address| !address.is_empty())
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

/// A person, as every source said of them last.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub key: String,
    /// When a source last said something new of them, in milliseconds
    /// since the epoch.
    pub heard: u64,
    pub sources: BTreeMap<String, Value>,
    /// Each name a source gives them and their address, made ready to
    /// search as they're heard, since a search reads everyone.
    names: Vec<Text>,
    mailed: Option<Text>,
    listed: bool,
}

impl Record {
    fn of(key: String, now: u64) -> Record {
        let mailed = address(&key).map(Text::of);
        Record {
            key,
            heard: now,
            sources: BTreeMap::new(),
            names: Vec::new(),
            mailed,
            listed: false,
        }
    }

    /// Reads again what a search and a list read, as what's said changes.
    fn indexed(&mut self) {
        self.names = self
            .sources
            .values()
            .filter_map(|entry| entry.get("name").and_then(Value::as_str))
            .map(Text::of)
            .collect();
        self.listed = self.sources.values().any(|entry| {
            ["favourite", "recent"]
                .iter()
                .any(|word| entry.get(word).and_then(Value::as_bool) == Some(true))
        });
    }

    pub fn json(&self) -> Value {
        json!({"key": self.key, "kind": KIND, "heard": self.heard, "sources": self.sources})
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

    /// How well the words find them: by a name, by their address, or not.
    fn found(&self, words: &[Vec<char>]) -> Option<Found> {
        if self.names.iter().any(|name| name.finds(words)) {
            return Some(Found::ByName);
        }
        self.mailed
            .as_ref()
            .filter(|address| address.finds(words))
            .map(|_| Found::ByAddress)
    }
}

/// A text made ready to search: its characters in lower case, and where
/// each of its words starts.
#[derive(Clone, Debug, PartialEq)]
struct Text {
    lower: Vec<char>,
    starts: Vec<usize>,
    /// The letters its words start with, a bit each, as Fuzzy.js's
    /// `starts`, so someone no word typed could start is passed over at
    /// once.
    begins: u32,
}

/// A character's bit: one for each of a to z, one for any digit, and one
/// shared by everything else.
fn bit(c: char) -> u32 {
    match c {
        'a'..='z' => 1 << (c as u32 - 'a' as u32),
        '0'..='9' => 1 << 26,
        _ => 1 << 27,
    }
}

impl Text {
    fn of(text: &str) -> Text {
        let chars: Vec<char> = text.chars().collect();
        let starts: Vec<usize> = (0..chars.len())
            .filter(|&at| starts_a_word(at.checked_sub(1).map(|before| chars[before]), chars[at]))
            .collect();
        let lower: Vec<char> = chars
            .iter()
            .map(|c| c.to_lowercase().next().unwrap_or(*c))
            .collect();
        let begins = starts.iter().fold(0, |mask, &at| mask | bit(lower[at]));
        Text {
            lower,
            starts,
            begins,
        }
    }

    fn finds(&self, words: &[Vec<char>]) -> bool {
        !words.is_empty()
            && words
                .iter()
                .all(|word| word.first().is_some_and(|&c| self.begins & bit(c) != 0))
            && words.iter().all(|word| {
                self.starts
                    .iter()
                    .any(|&at| self.lower.get(at..at + word.len()) == Some(word.as_slice()))
            })
    }
}

/// How a search found someone, the better first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Found {
    ByName,
    ByAddress,
}

/// The words of a search, lowercased, each its characters.
pub fn words(typed: &str) -> Vec<Vec<char>> {
    typed
        .split_whitespace()
        .map(|word| word.chars().flat_map(char::to_lowercase).collect())
        .collect()
}

/// Whether each word starts a word of the text, its letters unbroken, in
/// any order and any case. A word starts where the shell's Fuzzy.js says
/// one does, as fzf does: at the text's start, after anything that isn't
/// a letter or a digit, at a capital after a small letter or a digit, and
/// at a digit after a letter. So what's found here is never fewer than
/// the shell would find among everyone, and the shell ranks it.
pub fn finds(text: &str, words: &[Vec<char>]) -> bool {
    Text::of(text).finds(words)
}

fn starts_a_word(before: Option<char>, c: char) -> bool {
    let Some(before) = before else {
        return true;
    };
    if !c.is_alphanumeric() {
        // Punctuation is its own start, as Fuzzy.js gives it a bonus.
        return !c.is_whitespace();
    }
    !before.is_alphanumeric()
        || (before.is_lowercase() && c.is_uppercase())
        || (before.is_ascii_digit() && c.is_uppercase())
        || (!before.is_ascii_digit() && c.is_ascii_digit())
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
            ..Book::default()
        }
    }

    /// Takes what a source said, and gives the key of a person whose
    /// record changed.
    pub fn take(&mut self, source: &str, heard: Heard, now: u64) -> Option<String> {
        match heard {
            Heard::Person { address, entry } => {
                self.stand(source, Standing::Answering);
                let key = key_of(source, &address, id_of(&entry).as_deref());
                self.answered
                    .entry(key.clone())
                    .or_default()
                    .insert(source.to_owned());
                let record = self
                    .records
                    .entry(key.clone())
                    .or_insert_with(|| Record::of(key.clone(), now));
                if record.sources.get(source) == Some(&entry) {
                    return None;
                }
                if record.sources.is_empty() {
                    self.order.push(key.clone());
                }
                record.sources.insert(source.to_owned(), entry);
                record.indexed();
                record.heard = now;
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
                record.indexed();
                record.heard = now;
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

    /// Every record held, in the order each was first heard of.
    pub fn everyone(&self) -> impl Iterator<Item = &Record> {
        self.order.iter().filter_map(|key| self.records.get(key))
    }

    /// Those the words find, at most `limit`: those a source lists first,
    /// then those found by name before those found only by address, and
    /// among equals in the order heard.
    pub fn search(&self, typed: &str, limit: usize) -> Vec<&Record> {
        let words = words(typed);
        let mut found: Vec<(bool, Found, usize, &Record)> = self
            .everyone()
            .enumerate()
            .filter_map(|(at, record)| {
                record
                    .found(&words)
                    .map(|how| (!record.listed(), how, at, record))
            })
            .collect();
        found.sort_unstable_by_key(|&(unlisted, how, at, _)| (unlisted, how, at));
        found
            .into_iter()
            .take(limit)
            .map(|(.., record)| record)
            .collect()
    }

    /// Whether every source that can answer has answered for a key: one
    /// signed out or refused never will, so it isn't waited on.
    pub fn ready(&self, key: &str) -> bool {
        let answered = self.answered.get(key);
        self.standings.iter().all(|(source, standing)| {
            self.rosters.contains(source)
                || matches!(standing, Standing::SignedOut | Standing::Refused(_))
                || answered.is_some_and(|answered| answered.contains(source))
        })
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
mod tests {
    use super::*;

    #[test]
    fn one_person_is_one_key_whatever_case_their_address_comes_in() {
        assert_eq!(key(" Ada@Example.com"), "person:ada@example.com");
        assert_eq!(address("person:ada@example.com"), Some("ada@example.com"));
        assert_eq!(address("file:/x"), None);
        assert_eq!(
            ask(r#"{"id": 2, "ask": "get", "keys": ["person:Ada@Example.com", "file:x"]}"#),
            Ask::Get {
                id: json!(2),
                keys: vec!["person:ada@example.com".to_owned()],
                patience: 0
            }
        );
    }

    #[test]
    fn each_ask_is_read_with_its_id_and_a_kind_not_kept_refused() {
        assert_eq!(
            ask(r#"{"id": 1, "ask": "hello", "version": 1}"#),
            Ask::Hello { id: json!(1) }
        );
        assert_eq!(
            ask(r#"{"id": 3, "ask": "list", "kind": "person", "limit": 200}"#),
            Ask::List {
                id: json!(3),
                limit: Some(200),
                listed: false
            }
        );
        assert_eq!(
            ask(r#"{"id": 4, "ask": "watch", "kind": "person"}"#),
            Ask::Watch {
                id: json!(4),
                keys: None,
                listed: false
            }
        );
        assert_eq!(
            ask(r#"{"id": 7, "ask": "watch", "listed": true}"#),
            Ask::Watch {
                id: json!(7),
                keys: None,
                listed: true
            }
        );
        assert_eq!(
            ask(r#"{"id": 8, "ask": "search", "words": "ada b"}"#),
            Ask::Search {
                id: json!(8),
                words: "ada b".to_owned(),
                limit: SEARCHED
            }
        );
        assert_eq!(
            ask(r#"{"id": 5, "ask": "list", "kind": "file"}"#),
            Ask::UnknownKind {
                id: json!(5),
                kind: "file".to_owned()
            }
        );
        assert_eq!(
            ask(r#"{"id": 6, "ask": "shout"}"#),
            Ask::Unknown {
                id: json!(6),
                ask: "shout".to_owned()
            }
        );
        assert!(matches!(ask("not json"), Ask::Unknown { .. }));
    }

    #[test]
    fn a_sources_lines_are_someone_a_later_or_its_sign_in() {
        assert_eq!(
            heard(r#"{"email": "Ada@example.com", "id": "U1", "name": "Ada"}"#),
            Heard::Person {
                address: "ada@example.com".to_owned(),
                entry: json!({"id": "U1", "name": "Ada"})
            }
        );
        assert_eq!(
            heard(r#"{"email": "ada@example.com", "later": true}"#),
            Heard::Later {
                address: "ada@example.com".to_owned()
            }
        );
        assert_eq!(
            heard(r#"{"email": "", "id": 7, "gone": true}"#),
            Heard::Gone {
                address: String::new(),
                id: Some("7".to_owned())
            }
        );
        assert_eq!(heard(r#"{"email": "", "name": "Nobody"}"#), Heard::Other);
        assert_eq!(heard(r#"{"roster": true}"#), Heard::Roster);
        assert_eq!(heard(r#"{"signedOut": true}"#), Heard::SignedOut);
        assert_eq!(heard(r#"{"signedIn": false}"#), Heard::SignedOut);
        assert_eq!(heard("hello"), Heard::Other);
    }

    #[test]
    fn what_a_source_says_is_held_whole_under_its_name_changing_only_when_it_does() {
        let mut book = Book::of(&["slack".to_owned(), "vault".to_owned()]);
        let ada = |name: &str| Heard::Person {
            address: "ada@example.com".to_owned(),
            entry: json!({"name": name}),
        };
        assert_eq!(
            book.take("slack", ada("Ada"), 5),
            Some("person:ada@example.com".to_owned())
        );
        assert_eq!(book.take("slack", ada("Ada"), 6), None);
        assert!(!book.ready("person:ada@example.com"));
        assert_eq!(
            book.take("vault", ada("Ada L"), 7),
            Some("person:ada@example.com".to_owned())
        );
        assert!(book.ready("person:ada@example.com"));
        let record = book.record("person:ada@example.com").unwrap();
        assert_eq!(
            record.json(),
            json!({"key": "person:ada@example.com", "kind": "person", "heard": 7,
                   "sources": {"slack": {"name": "Ada"}, "vault": {"name": "Ada L"}}})
        );
        assert_eq!(book.everyone().count(), 1);
    }

    #[test]
    fn a_source_signed_out_is_never_waited_on_and_one_started_again_is() {
        let mut book = Book::of(&["slack".to_owned(), "vault".to_owned()]);
        book.take(
            "slack",
            Heard::Person {
                address: "ada@example.com".to_owned(),
                entry: json!({}),
            },
            1,
        );
        book.take("vault", Heard::SignedOut, 2);
        assert!(book.ready("person:ada@example.com"));
        assert_eq!(book.standings()["vault"].word(), "signed-out");
        assert_eq!(
            Book::of(&["tuple".to_owned()]).standings()["tuple"].word(),
            "starting"
        );
        book.restarted("slack");
        assert!(!book.ready("person:ada@example.com"));
        assert_eq!(book.standings()["slack"].word(), "answering");
    }

    fn tuple(address: &str, id: u32, name: &str, favourite: bool) -> Heard {
        Heard::Person {
            address: address.to_owned(),
            entry: json!({"id": id, "name": name, "status": "unknown",
                          "favourite": favourite, "recent": false}),
        }
    }

    #[test]
    fn a_roster_is_never_waited_on_and_knows_some_by_its_own_id() {
        let mut book = Book::of(&["slack".to_owned(), "tuple".to_owned()]);
        book.take("tuple", Heard::Roster, 1);
        book.take("tuple", tuple("", 7, "Gus", false), 1);
        assert!(book.record("person:tuple:7").is_some());
        // Only Slack, which is asked, is waited on.
        assert!(!book.ready("person:ada@example.com"));
        book.take(
            "slack",
            Heard::Later {
                address: "ada@example.com".to_owned(),
            },
            2,
        );
        book.take(
            "slack",
            Heard::Person {
                address: "ada@example.com".to_owned(),
                entry: json!({}),
            },
            2,
        );
        assert!(book.ready("person:ada@example.com"));
        book.restarted("tuple");
        assert!(!book.ready("person:ada@example.com"));
    }

    #[test]
    fn someone_a_source_forgets_keeps_what_the_others_say_or_goes() {
        let mut book = Book::of(&["slack".to_owned(), "tuple".to_owned()]);
        book.take("tuple", tuple("ada@example.com", 1, "Ada", false), 1);
        book.take("tuple", tuple("eli@example.com", 2, "Eli", false), 1);
        book.take(
            "slack",
            Heard::Person {
                address: "ada@example.com".to_owned(),
                entry: json!({}),
            },
            1,
        );
        let gone = |address: &str| Heard::Gone {
            address: address.to_owned(),
            id: None,
        };
        assert_eq!(
            book.take("tuple", gone("ada@example.com"), 2),
            Some("person:ada@example.com".to_owned())
        );
        assert_eq!(
            book.record("person:ada@example.com")
                .unwrap()
                .sources
                .keys()
                .collect::<Vec<_>>(),
            ["slack"]
        );
        assert_eq!(
            book.take("tuple", gone("eli@example.com"), 3),
            Some("person:eli@example.com".to_owned())
        );
        assert!(book.record("person:eli@example.com").is_none());
        assert_eq!(book.everyone().count(), 1);
        assert_eq!(book.take("tuple", gone("eli@example.com"), 4), None);
    }

    #[test]
    fn words_start_words_as_the_shells_fuzzy_says() {
        let find = |text: &str, typed: &str| finds(text, &words(typed));
        assert!(find("Ada Brightwater", "bri ad"));
        assert!(find("Ada Brightwater", "ADA"));
        assert!(!find("Ada Brightwater", "water"));
        assert!(find("ada.brightwater@example.com", "bright"));
        assert!(find("ada@example.com", "exam"));
        assert!(find("McKenzie", "ken"));
        assert!(find("agent007", "007"));
        assert!(find("Zoë Ångström", "ång"));
        assert!(!find("Ada", ""));
        assert!(!find("Ada", "adam"));
    }

    #[test]
    fn a_search_lists_the_listed_first_then_names_then_addresses() {
        let mut book = Book::of(&["tuple".to_owned()]);
        book.take(
            "tuple",
            tuple("dale@example.com", 1, "Ada's Friend", false),
            1,
        );
        book.take("tuple", tuple("ada.k@example.com", 2, "Kim", false), 1);
        book.take("tuple", tuple("eli@example.com", 3, "Ada Eli", false), 1);
        book.take("tuple", tuple("x@example.com", 4, "Adams Ray", true), 1);
        book.take("tuple", tuple("gus@example.com", 5, "Gus", false), 1);
        let keys = |limit| {
            book.search("ada", limit)
                .iter()
                .map(|record| record.key.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            keys(10),
            [
                "person:x@example.com",
                "person:dale@example.com",
                "person:eli@example.com",
                "person:ada.k@example.com"
            ]
        );
        assert_eq!(keys(2).len(), 2);
        assert!(book.record("person:x@example.com").unwrap().listed());
        assert!(!book.record("person:gus@example.com").unwrap().listed());
    }
}
