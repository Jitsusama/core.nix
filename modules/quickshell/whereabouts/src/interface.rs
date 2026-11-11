//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps: where the laptop is, and
/// Location's rows, ready for the launcher (rows.rs).
pub const KINDS: [&str; 2] = ["here", "row"];

/// The key where the laptop is, the laptop's.
pub const HERE: &str = "here:laptop";

/// The kind a key names: `here` for `here:laptop`, `row` for
/// `row:location:town`.
pub fn kind_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    (KINDS.contains(&kind) && !rest.is_empty()).then_some(kind)
}

/// The place a row's key names: `location` for `row:location:town`.
pub fn place_of(key: &str) -> Option<&str> {
    key.strip_prefix("row:")?
        .split_once(':')
        .map(|(place, _)| place)
}

/// What narrows a list or a watch: the kinds it names, none naming every
/// kind, and for rows the one place it names, as the launcher watches
/// `{kind: "row", place: "location"}` and hears only Location's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Narrowed {
    pub kinds: Option<Vec<String>>,
    pub place: Option<String>,
}

impl Narrowed {
    /// Whether a record of this key is among those it names.
    pub fn holds(&self, key: &str) -> bool {
        let Some(kind) = kind_of(key) else {
            return false;
        };
        let kept = self
            .kinds
            .as_ref()
            .is_none_or(|kinds| kinds.iter().any(|named| named == kind));
        let placed = self
            .place
            .as_deref()
            .is_none_or(|place| kind != "row" || place_of(key) == Some(place));
        kept && placed
    }
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
        narrowed: Narrowed,
    },
    /// Records as they change, all of them or these keys.
    Watch {
        id: Value,
        keys: Option<Vec<String>>,
        narrowed: Narrowed,
    },
    /// Joel's word on where the laptop is: `set`, to the town `words`
    /// name, or `again`, to have GeoClue find it once more.
    Act {
        id: Value,
        act: String,
        words: String,
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
    let narrowed = Narrowed {
        kinds: match said.get("kind") {
            Some(Value::String(kind)) => Some(vec![kind.clone()]),
            Some(Value::Array(kinds)) => Some(
                kinds
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            ),
            _ => None,
        },
        place: said.get("place").and_then(Value::as_str).map(str::to_owned),
    };
    let text = |field: &str| {
        said.get(field)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys.unwrap_or_default(),
        },
        "list" => refused(id.clone()).unwrap_or(Ask::List { id, narrowed }),
        "watch" => refused(id.clone()).unwrap_or(Ask::Watch { id, keys, narrowed }),
        "act" => Ask::Act {
            id,
            act: text("act"),
            words: text("words"),
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

    /// What narrows to these kinds, and a place.
    fn narrowed(kinds: Option<&[&str]>, place: Option<&str>) -> Narrowed {
        Narrowed {
            kinds: kinds.map(|kinds| kinds.iter().map(|kind| (*kind).to_owned()).collect()),
            place: place.map(str::to_owned),
        }
    }

    #[test]
    fn a_watch_names_its_kind_or_none_or_its_key() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","kind":"here"}"#),
            Ask::Watch {
                id: json!(1),
                keys: None,
                narrowed: narrowed(Some(&["here"]), None),
            }
        );
        assert_eq!(
            ask(r#"{"id":"all","ask":"watch"}"#),
            Ask::Watch {
                id: json!("all"),
                keys: None,
                narrowed: Narrowed::default(),
            }
        );
        assert_eq!(
            ask(r#"{"id":2,"ask":"watch","kind":["here"],"keys":["here:laptop"]}"#),
            Ask::Watch {
                id: json!(2),
                keys: Some(vec![HERE.into()]),
                narrowed: narrowed(Some(&["here"]), None),
            }
        );
    }

    #[test]
    fn a_watch_of_rows_hears_only_the_place_it_names() {
        let location = narrowed(Some(&["row"]), Some("location"));
        assert_eq!(
            ask(r#"{"id":3,"ask":"watch","kind":"row","place":"location"}"#),
            Ask::Watch {
                id: json!(3),
                keys: None,
                narrowed: location.clone(),
            }
        );
        assert!(location.holds("row:location:town"), "its own place's row");
        assert!(!location.holds("row:on:location"), "another place's");
        assert!(!location.holds(HERE), "another kind");
        assert!(
            Narrowed::default().holds("row:on:location"),
            "a watch of all"
        );
    }

    #[test]
    fn an_act_names_what_joel_said() {
        assert_eq!(
            ask(r#"{"id":4,"ask":"act","act":"set","words":"Kelpwater"}"#),
            Ask::Act {
                id: json!(4),
                act: "set".into(),
                words: "Kelpwater".into(),
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
