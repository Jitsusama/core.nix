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
    /// Every record held, in the order heard, at most `limit`.
    List {
        id: Value,
        limit: Option<usize>,
    },
    /// Records as they change, these keys or all of them.
    Watch {
        id: Value,
        keys: Option<Vec<String>>,
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
    if matches!(asked, "list" | "watch") && kind != KIND {
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
        },
        "watch" => Ask::Watch { id, keys: keys() },
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

/// What a source said, one line of its protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    /// What it knows of someone, its line less the address.
    Person { address: String, entry: Value },
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
    let Some(Value::String(address)) = said.remove("email") else {
        return Heard::Other;
    };
    let address = address.trim().to_lowercase();
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
}

impl Record {
    pub fn json(&self) -> Value {
        json!({"key": self.key, "kind": KIND, "heard": self.heard, "sources": self.sources})
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
                let key = key(&address);
                self.answered
                    .entry(key.clone())
                    .or_default()
                    .insert(source.to_owned());
                let record = self.records.entry(key.clone()).or_insert_with(|| Record {
                    key: key.clone(),
                    heard: now,
                    sources: BTreeMap::new(),
                });
                if record.sources.get(source) == Some(&entry) {
                    return None;
                }
                if record.sources.is_empty() {
                    self.order.push(key.clone());
                }
                record.sources.insert(source.to_owned(), entry);
                record.heard = now;
                Some(key)
            }
            Heard::Later { .. } => {
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

    /// Whether every source that can answer has answered for a key: one
    /// signed out or refused never will, so it isn't waited on.
    pub fn ready(&self, key: &str) -> bool {
        let answered = self.answered.get(key);
        self.standings.iter().all(|(source, standing)| {
            matches!(standing, Standing::SignedOut | Standing::Refused(_))
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
                limit: Some(200)
            }
        );
        assert_eq!(
            ask(r#"{"id": 4, "ask": "watch", "kind": "person"}"#),
            Ask::Watch {
                id: json!(4),
                keys: None
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
}
