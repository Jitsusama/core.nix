//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps.
pub const KINDS: [&str; 8] = [
    "window",
    "workspace",
    "desktop",
    "focus",
    "cast",
    "layout",
    "screenshot",
    "row",
];

/// The kinds of which there's one record, keyed by its kind alone.
const SOLE: [&str; 3] = ["desktop", "focus", "screenshot"];

/// The kinds an ask gets only by naming them: a window's tile moves at
/// every resize and scroll, and the window niri says has the keys moves
/// to none at every opening of the launcher, which would wake every watch
/// that names no kind, where only the few that draw tiles, or follow the
/// keys as niri does, want them. A row is the launcher's alone, ready to
/// show, so only a watch of its place wants it.
const NAMED_ONLY: [&str; 3] = ["focus", "layout", "row"];

/// The place a row's key names: `displays` for
/// `row:displays:display:eDP-1`.
pub fn place_of(key: &str) -> Option<&str> {
    let rest = key.strip_prefix("row:")?;
    rest.split_once(':').map(|(place, _)| place)
}

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
    /// Every record of these kinds, and of rows those of these places.
    List {
        id: Value,
        kinds: Vec<String>,
        places: Option<Vec<String>>,
    },
    /// Records of these kinds as they change, all of them or these keys,
    /// whole or only these fields of them, and of rows those of these
    /// places.
    Watch {
        id: Value,
        kinds: Vec<String>,
        keys: Option<Vec<String>>,
        fields: Option<Vec<String>>,
        places: Option<Vec<String>>,
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
    let fields = said.get("fields").and_then(Value::as_array).map(|fields| {
        fields
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    // A place, as one or a list, as machine's are named.
    let places = match said.get("place") {
        Some(Value::String(place)) => Some(vec![place.clone()]),
        Some(Value::Array(places)) => Some(
            places
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    };
    let named = || kinds(&said);
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys.unwrap_or_default(),
        },
        "list" => match named() {
            Ok(kinds) => Ask::List { id, kinds, places },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "watch" => match named() {
            Ok(kinds) => Ask::Watch {
                id,
                kinds,
                keys,
                fields,
                places,
            },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "unwatch" => Ask::Unwatch { id },
        "health" => Ask::Health { id },
        _ => Ask::Unknown { id, ask: asked },
    }
}

/// Whether an ask of these places takes this record: any record but a
/// row, and a row only of a place it names, or of any where it names
/// none.
pub fn placed(key: &str, places: Option<&[String]>) -> bool {
    match (place_of(key), places) {
        (Some(place), Some(places)) => places.iter().any(|named| named == place),
        _ => true,
    }
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
                keys: Some(vec!["window:12".into(), "desktop".into()]),
                fields: None,
                places: None
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
                keys: None,
                fields: None,
                places: None
            }
        );
        assert!(
            !unnamed().contains(&"layout".to_owned()),
            "a watch naming no kind takes layouts: {:?}",
            unnamed()
        );
        assert!(
            !unnamed().contains(&"focus".to_owned()),
            "a watch naming no kind takes niri's focus: {:?}",
            unnamed()
        );
    }

    #[test]
    fn the_screenshot_is_keyed_by_its_kind_alone() {
        assert_eq!(kind_of("screenshot"), Some("screenshot"));
        assert_eq!(kind_of("screenshot:1"), None);
        assert_eq!(kind_of("focus"), Some("focus"));
        assert_eq!(kind_of("cast:3"), Some("cast"));
        assert_eq!(kind_of("layout:12"), Some("layout"));
    }

    #[test]
    fn a_watch_of_some_fields_is_told_them_only_as_they_change() {
        assert_eq!(
            ask(r#"{"id":4,"ask":"watch","kind":"window","fields":["order"]}"#),
            Ask::Watch {
                id: json!(4),
                kinds: vec!["window".into()],
                keys: None,
                fields: Some(vec!["order".into()]),
                places: None
            }
        );
        let fields = ["order".to_owned()];
        let was = json!({"key": "window:12", "title": "zsh", "order": 100});
        let retitled = json!({"key": "window:12", "title": "vim", "order": 100});
        let used = json!({"key": "window:12", "title": "vim", "order": 200});
        assert!(
            !tells(Some(&was), &retitled, Some(&fields)),
            "a retitle was told to a watch of the order alone"
        );
        assert!(
            tells(Some(&retitled), &used, Some(&fields)),
            "a window used wasn't told to a watch of the order"
        );
        assert!(
            tells(None, &used, Some(&fields)),
            "a window new to a watch of the order wasn't told"
        );
        assert!(
            tells(Some(&was), &retitled, None),
            "a retitle wasn't told to a watch of every field"
        );
        assert_eq!(
            trimmed(&used, Some(&fields)),
            json!({"key": "window:12", "order": 200})
        );
        assert_eq!(trimmed(&used, None), used);
    }

    #[test]
    fn a_watch_of_a_place_takes_its_rows_alone() {
        assert_eq!(
            ask(r#"{"id":5,"ask":"watch","kind":"row","place":"displays"}"#),
            Ask::Watch {
                id: json!(5),
                kinds: vec!["row".into()],
                keys: None,
                fields: None,
                places: Some(vec!["displays".into()])
            }
        );
        assert_eq!(kind_of("row:displays:display:eDP-1"), Some("row"));
        assert_eq!(place_of("row:displays:display:eDP-1"), Some("displays"));
        assert_eq!(place_of("window:12"), None);
        let displays = ["displays".to_owned()];
        assert!(placed("row:displays:sleep", Some(&displays)));
        assert!(!placed("row:windows:12", Some(&displays)));
        assert!(
            placed("window:12", Some(&displays)),
            "a window has no place"
        );
        assert!(
            !unnamed().contains(&"row".to_owned()),
            "a watch naming no kind takes the launcher's rows: {:?}",
            unnamed()
        );
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
