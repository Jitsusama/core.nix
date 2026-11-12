//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers. Lifted from the to-do daemon's
//! interface.rs, the kinds a daemon keeps handed in rather than fixed, and
//! a watch narrowed by place, as every daemon's ready rows are watched
//! (Y2YBTS's ready-rows.md).

use std::time::Duration;

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// A daemon as its clients know it: its domain's name and the kinds of
/// record it keeps, in the order it lists them; of those, the kinds of
/// which there's one record, keyed by its kind alone, as the windows
/// daemon's `desktop`, and those an ask gets only by naming them, as its
/// `focus`, which moves at every opening of the launcher and would wake
/// every watch naming no kind.
#[derive(Clone, Copy, Debug)]
pub struct Domain {
    pub name: &'static str,
    pub kinds: &'static [&'static str],
    pub sole: &'static [&'static str],
    pub named_only: &'static [&'static str],
}

impl Domain {
    /// The kind a key names: `row` for `row:agents:O`, and `desktop` for
    /// the one record of a sole kind, keyed by its kind alone.
    pub fn kind_of<'a>(&self, key: &'a str) -> Option<&'a str> {
        if let Some(sole) = self.sole.iter().find(|sole| **sole == key) {
            return Some(sole);
        }
        let (kind, rest) = key.split_once(':')?;
        (self.kinds.contains(&kind) && !self.sole.contains(&kind) && !rest.is_empty())
            .then_some(kind)
    }

    /// The kinds an ask names, as one or a list, or every kind but those
    /// named only when it names none; or the first it doesn't keep.
    fn kinds(&self, said: &Value) -> Result<Vec<String>, String> {
        let named = match said.get("kind") {
            None | Some(Value::Null) => {
                let unnamed = self
                    .kinds
                    .iter()
                    .filter(|kind| !self.named_only.contains(kind));
                return Ok(unnamed.map(|kind| (*kind).to_owned()).collect());
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
        let fields = said.get("fields").and_then(words);
        let limit = said
            .get("limit")
            .and_then(Value::as_u64)
            .and_then(|limit| usize::try_from(limit).ok());
        let every = said
            .get("every")
            .and_then(Value::as_u64)
            .filter(|ms| *ms > 0)
            .map(Duration::from_millis);
        match asked.as_str() {
            "hello" => Ask::Hello { id },
            "get" => Ask::Get {
                id,
                keys: keys.unwrap_or_default(),
            },
            "list" => match self.kinds(&said) {
                Ok(kinds) => Ask::List {
                    id,
                    kinds,
                    places,
                    limit,
                },
                Err(kind) => Ask::UnknownKind { id, kind },
            },
            "watch" => match self.kinds(&said) {
                Ok(kinds) => Ask::Watch {
                    id,
                    kinds,
                    keys,
                    places,
                    fields,
                    every,
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
    /// Every record of these kinds, or those of them in these places, at
    /// most `limit` of them.
    List {
        id: Value,
        kinds: Vec<String>,
        places: Option<Vec<String>>,
        limit: Option<usize>,
    },
    /// Records of these kinds as they change: all of them, these keys, or
    /// those in these places; whole, or only these fields of each, told
    /// only as one of them moves; told at once, or, paced `every` so
    /// long, at most once a pace with all that moved in it, as a shut
    /// launcher asks.
    Watch {
        id: Value,
        kinds: Vec<String>,
        keys: Option<Vec<String>>,
        places: Option<Vec<String>>,
        fields: Option<Vec<String>>,
        every: Option<Duration>,
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

/// A record as a watch naming these fields is told it: its key and those
/// fields alone, or whole for a watch naming none.
pub fn trimmed(record: &Value, fields: Option<&[String]>) -> Value {
    let Some(fields) = fields else {
        return record.clone();
    };
    let mut kept = json!({"key": record["key"]});
    for field in fields {
        if let Some(value) = record.get(field) {
            kept[field] = value.clone();
        }
    }
    kept
}

/// Whether a watch naming these fields is told a record that was this
/// and is now that: always when it's new or the watch takes it whole, else
/// only when one of its fields changed, so a watch of when windows were
/// used hears nothing of a retitle.
pub fn tells(was: Option<&Value>, now: &Value, fields: Option<&[String]>) -> bool {
    match (was, fields) {
        (Some(was), Some(fields)) => fields.iter().any(|field| was.get(field) != now.get(field)),
        _ => true,
    }
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
        sole: &[],
        named_only: &[],
    };

    const WINDOWS: Domain = Domain {
        name: "windows",
        kinds: &["window", "desktop", "focus", "row"],
        sole: &["desktop", "focus"],
        named_only: &["focus", "row"],
    };

    #[test]
    fn a_sole_kind_is_keyed_by_its_kind_alone() {
        assert_eq!(
            WINDOWS.kind_of("desktop"),
            Some("desktop"),
            "the one desktop"
        );
        assert_eq!(
            WINDOWS.kind_of("desktop:1"),
            None,
            "a desktop keyed as many"
        );
        assert_eq!(WINDOWS.kind_of("window:12"), Some("window"), "a window");
        assert_eq!(WINDOWS.kind_of("window"), None, "a window with no id");
    }

    #[test]
    fn a_kind_named_only_is_left_out_of_an_ask_naming_none() {
        let Ask::Watch { kinds, .. } = WINDOWS.ask(r#"{"id":1,"ask":"watch"}"#) else {
            panic!("a watch of every kind wasn't read as a watch");
        };
        assert_eq!(
            kinds,
            vec!["window".to_owned(), "desktop".to_owned()],
            "named none"
        );
        let Ask::Watch { kinds, .. } = WINDOWS.ask(r#"{"id":1,"ask":"watch","kind":"focus"}"#)
        else {
            panic!("a watch of the focus wasn't read as a watch");
        };
        assert_eq!(kinds, vec!["focus".to_owned()], "named");
    }

    #[test]
    fn a_watch_of_some_fields_is_told_them_only_as_they_change() {
        let Ask::Watch { fields, .. } =
            WINDOWS.ask(r#"{"id":4,"ask":"watch","kind":"window","fields":["order"]}"#)
        else {
            panic!("a watch of some fields wasn't read as a watch");
        };
        assert_eq!(fields, Some(vec!["order".to_owned()]), "its fields");
        let fields = ["order".to_owned()];
        let was = json!({"key": "window:12", "title": "zsh", "order": 100});
        let retitled = json!({"key": "window:12", "title": "vim", "order": 100});
        let used = json!({"key": "window:12", "title": "vim", "order": 200});
        assert!(
            !tells(Some(&was), &retitled, Some(&fields)),
            "a retitle told"
        );
        assert!(
            tells(Some(&retitled), &used, Some(&fields)),
            "a use not told"
        );
        assert!(tells(None, &used, Some(&fields)), "a new window not told");
        assert!(
            tells(Some(&was), &retitled, None),
            "a retitle not told whole"
        );
        assert_eq!(
            trimmed(&used, Some(&fields)),
            json!({"key": "window:12", "order": 200})
        );
        assert_eq!(trimmed(&used, None), used, "whole");
    }

    #[test]
    fn a_list_may_name_its_places() {
        assert_eq!(
            WINDOWS.ask(r#"{"id":5,"ask":"list","kind":"row","place":"displays"}"#),
            Ask::List {
                id: json!(5),
                kinds: vec!["row".into()],
                places: Some(vec!["displays".into()]),
                limit: None,
            },
            "a list of one place's rows"
        );
    }

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
                fields: None,
                every: None,
            },
            "a watch of one place's rows"
        );
        let Ask::Watch { every, .. } =
            ROWS.ask(r#"{"id":1,"ask":"watch","place":"agents","every":10000}"#)
        else {
            panic!("a paced watch wasn't read as a watch");
        };
        assert_eq!(
            every,
            Some(Duration::from_secs(10)),
            "a watch paced ten seconds"
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
