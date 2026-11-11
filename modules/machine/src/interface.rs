//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps: what it reads, then what it
/// makes of them, a place's ready rows and its headline (`MADE`).
pub const KINDS: [&str; 29] = [
    "processor",
    "memory",
    "disk",
    "boot",
    "using",
    "tree",
    "sensor",
    "zone",
    "cooling",
    "supply",
    "sleep",
    "awake",
    "firmware",
    "backlight",
    "light",
    "brightness",
    "warmth",
    "camera",
    "kept",
    "profile",
    "keyboards",
    "capslock",
    "layer",
    "counts",
    "status",
    "telemetry",
    "link",
    "row",
    "head",
];

/// The kinds made of what's read rather than read themselves, each
/// keyed by its place: `row:heat:processor:0`, `head:heat`.
pub const MADE: [&str; 2] = ["row", "head"];

/// The place a made record's key names: `heat` for `row:heat:fan:1`.
pub fn place_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    MADE.contains(&kind)
        .then(|| rest.split_once(':').map_or(rest, |(place, _)| place))
}

/// The kind a key names: `disk` for `disk:nvme0n1`.
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
        kinds: Vec<String>,
    },
    /// Records of these kinds as they change, all of them or these keys,
    /// and of the rows and headlines these places' alone, or every
    /// place's.
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
    /// Something to do to the machine, named by `act`, with the rest of
    /// what was said as its arguments.
    Act {
        id: Value,
        act: String,
        said: Value,
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

/// The kinds an ask names, as one or a list, or every kind when it names
/// none; or the first it doesn't keep.
fn kinds(said: &Value) -> Result<Vec<String>, String> {
    let named: Vec<String> = match said.get("kind") {
        None | Some(Value::Null) => return Ok(KINDS.iter().map(|k| (*k).to_owned()).collect()),
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
            Ok(kinds) => Ask::List { id, kinds },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "watch" => match named() {
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
        "act" => Ask::Act {
            id,
            act: said
                .get("act")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            said,
        },
        _ => Ask::Unknown { id, ask: asked },
    }
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// A refusal with words for Joel beside its cause, as the ready-rows form
/// has it: `said` is what the shell shows, and `message` goes to the
/// journal. Empty words are left out, so the shell says its own.
pub fn refused(id: &Value, code: &str, message: &str, said: &str) -> Value {
    let mut answer = error(id, code, message);
    if !said.is_empty() {
        answer["error"]["said"] = json!(said);
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_names_one_kind_or_several_or_all() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","kind":"disk"}"#),
            Ask::Watch {
                id: json!(1),
                kinds: vec!["disk".into()],
                keys: None,
                places: None
            }
        );
        let Ask::Watch { kinds, .. } =
            ask(r#"{"id":1,"ask":"watch","kind":["processor","using"]}"#)
        else {
            panic!("not a watch");
        };
        assert_eq!(kinds, ["processor", "using"]);
        let Ask::Watch { kinds, .. } = ask(r#"{"id":1,"ask":"watch"}"#) else {
            panic!("not a watch");
        };
        assert_eq!(kinds.len(), KINDS.len());
    }

    #[test]
    fn a_watch_of_rows_names_its_place() {
        let Ask::Watch { kinds, places, .. } =
            ask(r#"{"id":1,"ask":"watch","kind":["row","head"],"place":"heat"}"#)
        else {
            panic!("not a watch");
        };
        assert_eq!(
            (kinds, places),
            (vec!["row".into(), "head".into()], Some(vec!["heat".into()]))
        );
        assert_eq!(
            ["row:heat:fan:dell_smm:1", "head:heat", "sensor:coretemp"].map(place_of),
            [Some("heat"), Some("heat"), None]
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
    }

    #[test]
    fn a_key_of_another_kind_is_left_out() {
        assert_eq!(
            ask(r#"{"id":3,"ask":"get","keys":["disk:nvme0n1","person:ada@example.com"]}"#),
            Ask::Get {
                id: json!(3),
                keys: vec!["disk:nvme0n1".into()]
            }
        );
    }

    #[test]
    fn an_act_keeps_what_it_was_asked_with() {
        let line = r#"{"id":4,"ask":"act","act":"charge-limit","supply":"BAT0","stop_pct":80}"#;
        let Ask::Act { id, act, said } = ask(line) else {
            panic!("not an act");
        };
        assert_eq!((id, act.as_str()), (json!(4), "charge-limit"));
        assert_eq!(
            (&said["supply"], &said["stop_pct"]),
            (&json!("BAT0"), &json!(80))
        );
    }
}
