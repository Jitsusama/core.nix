//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The one kind of record this daemon keeps: where the laptop is.
pub const KINDS: [&str; 1] = ["here"];

/// The one key it keeps, the laptop's.
pub const HERE: &str = "here:laptop";

/// The kind a key names: `here` for `here:laptop`.
pub fn kind_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    (KINDS.contains(&kind) && !rest.is_empty()).then_some(kind)
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
    },
    /// Records as they change, all of them or these keys.
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

/// The first kind an ask names that this daemon doesn't keep, named as one
/// or a list; none names every kind.
fn unkept(said: &Value) -> Option<String> {
    match said.get("kind") {
        None | Some(Value::Null) => None,
        Some(Value::String(kind)) => (!KINDS.contains(&kind.as_str())).then(|| kind.clone()),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|kind| !KINDS.contains(kind))
            .map(str::to_owned),
        Some(other) => Some(other.to_string()),
    }
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
    let asked = said
        .get("ask")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let keys = said.get("keys").and_then(Value::as_array).map(|keys| {
        keys.iter()
            .filter_map(Value::as_str)
            .filter(|key| kind_of(key).is_some())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    let refused = |id: Value| unkept(&said).map(|kind| Ask::UnknownKind { id, kind });
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys.unwrap_or_default(),
        },
        "list" => refused(id.clone()).unwrap_or(Ask::List { id }),
        "watch" => refused(id.clone()).unwrap_or(Ask::Watch { id, keys }),
        "unwatch" => Ask::Unwatch { id },
        "health" => Ask::Health { id },
        _ => Ask::Unknown { id, ask: asked },
    }
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_names_its_kind_or_none_or_its_key() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","kind":"here"}"#),
            Ask::Watch {
                id: json!(1),
                keys: None
            }
        );
        assert_eq!(
            ask(r#"{"id":"all","ask":"watch"}"#),
            Ask::Watch {
                id: json!("all"),
                keys: None
            }
        );
        assert_eq!(
            ask(r#"{"id":2,"ask":"watch","kind":["here"],"keys":["here:laptop"]}"#),
            Ask::Watch {
                id: json!(2),
                keys: Some(vec![HERE.into()])
            }
        );
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused() {
        assert_eq!(
            ask(r#"{"id":2,"ask":"list","kind":"person"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "person".into()
            }
        );
        assert_eq!(
            ask(r#"{"id":3,"ask":"watch","kind":["here","disk"]}"#),
            Ask::UnknownKind {
                id: json!(3),
                kind: "disk".into()
            }
        );
    }

    #[test]
    fn a_key_of_another_kind_is_left_out() {
        assert_eq!(
            ask(r#"{"id":3,"ask":"get","keys":["here:laptop","disk:nvme0n1"]}"#),
            Ask::Get {
                id: json!(3),
                keys: vec![HERE.into()]
            }
        );
    }

    #[test]
    fn a_line_that_isnt_an_ask_is_unknown() {
        assert_eq!(
            ask("not json"),
            Ask::Unknown {
                id: Value::Null,
                ask: String::new()
            }
        );
        assert_eq!(
            ask(r#"{"id":4,"ask":"move"}"#),
            Ask::Unknown {
                id: json!(4),
                ask: "move".into()
            }
        );
    }
}
