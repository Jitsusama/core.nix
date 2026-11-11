//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers. Lifted from the to-do daemon's
//! interface.rs, the kinds a daemon keeps handed in rather than fixed, and
//! a watch narrowed by place, as every daemon's ready rows are watched
//! (Y2YBTS's ready-rows.md).

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// A daemon as its clients know it: its domain's name and the kinds of
/// record it keeps, in the order it lists them.
#[derive(Clone, Copy, Debug)]
pub struct Domain {
    pub name: &'static str,
    pub kinds: &'static [&'static str],
}

impl Domain {
    /// The kind a key names: `row` for `row:agents:O`.
    pub fn kind_of<'a>(&self, key: &'a str) -> Option<&'a str> {
        let (kind, rest) = key.split_once(':')?;
        (self.kinds.contains(&kind) && !rest.is_empty()).then_some(kind)
    }

    /// The kinds an ask names, as one or a list, or every kind when it
    /// names none; or the first it doesn't keep.
    fn kinds(&self, said: &Value) -> Result<Vec<String>, String> {
        let named = match said.get("kind") {
            None | Some(Value::Null) => {
                return Ok(self.kinds.iter().map(|kind| (*kind).to_owned()).collect());
            }
            Some(other) => words(other).ok_or_else(|| other.to_string())?,
        };
        match named
            .iter()
            .find(|kind| !self.kinds.contains(&kind.as_str()))
        {
            Some(unknown) => Err(unknown.clone()),
            None => Ok(named),
        }
    }

    /// What a line from a client asks.
    pub fn ask(&self, line: &str) -> Ask {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return Ask::Unknown {
                id: Value::Null,
                ask: String::new(),
            };
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said
            .get("ask")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let keys = said.get("keys").and_then(Value::as_array).map(|keys| {
            keys.iter()
                .filter_map(Value::as_str)
                .filter(|key| self.kind_of(key).is_some())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });
        let places = said.get("place").and_then(words);
        match asked.as_str() {
            "hello" => Ask::Hello { id },
            "get" => Ask::Get {
                id,
                keys: keys.unwrap_or_default(),
            },
            "list" => match self.kinds(&said) {
                Ok(kinds) => Ask::List { id, kinds },
                Err(kind) => Ask::UnknownKind { id, kind },
            },
            "watch" => match self.kinds(&said) {
                Ok(kinds) => Ask::Watch {
                    id,
                    kinds,
                    keys,
                    places,
                },
                Err(kind) => Ask::UnknownKind { id, kind },
            },
            "unwatch" => Ask::Unwatch { id },
            "health" => Ask::Health { id },
            "" => Ask::Unknown { id, ask: asked },
            _ => Ask::Own { id, said },
        }
    }
}

/// One word or a list of them, as a kind or a place is named.
fn words(said: &Value) -> Option<Vec<String>> {
    match said {
        Value::String(one) => Some(vec![one.clone()]),
        Value::Array(many) => Some(
            many.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    }
}

/// The place a key is in, as `row:<place>:<id>` names it.
pub fn place_of(key: &str) -> Option<&str> {
    key.split(':').nth(1)
}

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Records by key, as they stand now.
    Get {
        id: Value,
        keys: Vec<String>,
    },
    /// Every record of these kinds.
    List {
        id: Value,
        kinds: Vec<String>,
    },
    /// Records of these kinds as they change: all of them, these keys, or
    /// those in these places.
    Watch {
        id: Value,
        kinds: Vec<String>,
        keys: Option<Vec<String>>,
        places: Option<Vec<String>>,
    },
    Unwatch {
        id: Value,
    },
    Health {
        id: Value,
    },
    /// An ask of the daemon's own, as the agents daemon is told whether
    /// the screen is shared, answered by the daemon rather than here.
    Own {
        id: Value,
        said: Value,
    },
    /// A kind this daemon doesn't keep.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// A line that isn't an ask.
    Unknown {
        id: Value,
        ask: String,
    },
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: Domain = Domain {
        name: "agents",
        kinds: &["row"],
    };

    #[test]
    fn a_key_names_its_kind_and_its_place() {
        assert_eq!(
            ROWS.kind_of("row:agents:fold:O"),
            Some("row"),
            "a fold's key"
        );
        assert_eq!(ROWS.kind_of("row:"), None, "a kind with nothing after it");
        assert_eq!(
            ROWS.kind_of("to-do:QEST-1:sign-in"),
            None,
            "another daemon's kind"
        );
        assert_eq!(place_of("row:agents:fold:O"), Some("agents"), "its place");
    }

    #[test]
    fn a_watch_names_its_kinds_and_may_name_its_places() {
        assert_eq!(
            ROWS.ask(r#"{"id":1,"ask":"watch","kind":"row","place":"agents"}"#),
            Ask::Watch {
                id: json!(1),
                kinds: vec!["row".into()],
                keys: None,
                places: Some(vec!["agents".into()]),
            },
            "a watch of one place's rows"
        );
        let Ask::Watch { kinds, places, .. } = ROWS.ask(r#"{"id":1,"ask":"watch"}"#) else {
            panic!("a watch of every kind wasn't read as a watch");
        };
        assert_eq!(
            (kinds, places),
            (vec!["row".to_owned()], None),
            "naming none watches all"
        );
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused_and_an_ask_it_doesnt_know_is_its_own() {
        assert_eq!(
            ROWS.ask(r#"{"id":2,"ask":"list","kind":"person"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "person".into()
            },
            "a list of a kind kept elsewhere"
        );
        let said = json!({"id": 3, "ask": "sharing", "sharing": true});
        assert_eq!(
            ROWS.ask(&said.to_string()),
            Ask::Own {
                id: json!(3),
                said: said.clone()
            },
            "the daemon's own ask, handed it whole"
        );
        assert_eq!(
            ROWS.ask("not json"),
            Ask::Unknown {
                id: Value::Null,
                ask: String::new()
            },
            "a line that isn't an ask"
        );
    }
}
