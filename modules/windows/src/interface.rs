//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps.
pub const KINDS: [&str; 6] = [
    "window",
    "workspace",
    "desktop",
    "cast",
    "layout",
    "screenshot",
];

/// The kinds of which there's one record, keyed by its kind alone.
const SOLE: [&str; 2] = ["desktop", "screenshot"];

/// The kinds an ask gets only by naming them: a window's tile moves at
/// every resize and scroll, which would wake every watch that names no
/// kind, where only the few that draw tiles want it.
const NAMED_ONLY: [&str; 1] = ["layout"];

/// The kind a key names: `window` for `window:12`, and `desktop` for the
/// one record of its kind, keyed by its kind alone.
pub fn kind_of(key: &str) -> Option<&str> {
    if let Some(sole) = SOLE.iter().find(|sole| **sole == key) {
        return Some(sole);
    }
    let (kind, rest) = key.split_once(':')?;
    (KINDS.contains(&kind) && !SOLE.contains(&kind) && !rest.is_empty()).then_some(kind)
}

/// The kinds an ask that names none gets.
fn unnamed() -> Vec<String> {
    KINDS
        .iter()
        .filter(|kind| !NAMED_ONLY.contains(kind))
        .map(|kind| (*kind).to_owned())
        .collect()
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
    /// Records of these kinds as they change, all of them or these keys.
    Watch {
        id: Value,
        kinds: Vec<String>,
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

/// The kinds an ask names, as one or a list, or every kind but those
/// named only when it names none; or the first it doesn't keep.
fn kinds(said: &Value) -> Result<Vec<String>, String> {
    let named: Vec<String> = match said.get("kind") {
        None | Some(Value::Null) => return Ok(unnamed()),
        Some(Value::String(kind)) => vec![kind.clone()],
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(other) => return Err(other.to_string()),
    };
    match named.iter().find(|kind| !KINDS.contains(&kind.as_str())) {
        Some(unknown) => Err(unknown.clone()),
        None => Ok(named),
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
    let named = || kinds(&said);
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys.unwrap_or_default(),
        },
        "list" => match named() {
            Ok(kinds) => Ask::List { id, kinds },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "watch" => match named() {
            Ok(kinds) => Ask::Watch { id, kinds, keys },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
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
    fn a_watch_names_keys_of_its_kinds() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","keys":["window:12","desktop"]}"#),
            Ask::Watch {
                id: json!(1),
                kinds: vec![
                    "window".into(),
                    "workspace".into(),
                    "desktop".into(),
                    "cast".into(),
                    "screenshot".into()
                ],
                keys: Some(vec!["window:12".into(), "desktop".into()])
            }
        );
    }

    #[test]
    fn layouts_are_listed_only_when_named() {
        assert_eq!(
            ask(r#"{"id":4,"ask":"watch","kind":["window","layout"]}"#),
            Ask::Watch {
                id: json!(4),
                kinds: vec!["window".into(), "layout".into()],
                keys: None
            }
        );
        assert!(!unnamed().contains(&"layout".to_owned()));
    }

    #[test]
    fn the_screenshot_is_keyed_by_its_kind_alone() {
        assert_eq!(kind_of("screenshot"), Some("screenshot"));
        assert_eq!(kind_of("screenshot:1"), None);
        assert_eq!(kind_of("cast:3"), Some("cast"));
        assert_eq!(kind_of("layout:12"), Some("layout"));
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused() {
        assert_eq!(
            ask(r#"{"id":2,"ask":"list","kind":"disk"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "disk".into()
            }
        );
    }

    #[test]
    fn a_key_of_another_kind_is_left_out() {
        assert_eq!(
            ask(
                r#"{"id":3,"ask":"get","keys":["workspace:2","disk:nvme0n1","window:","desktop:1"]}"#
            ),
            Ask::Get {
                id: json!(3),
                keys: vec!["workspace:2".into()]
            }
        );
    }
}
