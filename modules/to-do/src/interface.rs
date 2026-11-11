//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers. Machine's daemon speaks the same
//! (modules/machine/src/interface.rs); only the kinds differ.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps: each to-do, each look the
/// agents left, and what waits on Joel counted whole.
pub const KINDS: [&str; 3] = ["to-do", "look", "waiting"];

/// The key of the one record that counts what waits.
pub const SUMMARY: &str = "waiting:summary";

/// The kind a key names: `to-do` for `to-do:QEST-1:sign-in`.
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
    /// Joel's word on a to-do, by its identity: `done`, or `notNow`, put
    /// off until it says something new.
    Act {
        id: Value,
        act: String,
        item: String,
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
        "act" => {
            let field = |name: &str| said.get(name).and_then(Value::as_str).unwrap_or("");
            Ask::Act {
                act: field("act").to_owned(),
                item: field("item").to_owned(),
                id,
            }
        }
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
    fn a_to_dos_key_names_its_kind_however_many_colons_it_holds() {
        assert_eq!(
            kind_of("to-do:QEST-1:sign-in"),
            Some("to-do"),
            "a to-do's key"
        );
        assert_eq!(kind_of("look:01b53bb1"), Some("look"), "a look's key");
        assert_eq!(kind_of(SUMMARY), Some("waiting"), "the count's key");
        assert_eq!(kind_of("to-do:"), None, "a kind with nothing after it");
        assert_eq!(kind_of("disk:nvme0n1"), None, "another daemon's kind");
    }

    #[test]
    fn a_watch_names_one_kind_or_several_or_all() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","kind":"waiting"}"#),
            Ask::Watch {
                id: json!(1),
                kinds: vec!["waiting".into()],
                keys: None
            },
            "a watch of one kind"
        );
        let Ask::Watch { kinds, .. } = ask(r#"{"id":1,"ask":"watch","kind":["to-do","look"]}"#)
        else {
            panic!("a watch of two kinds wasn't read as a watch");
        };
        assert_eq!(kinds, ["to-do", "look"], "a watch of two kinds");
        let Ask::Watch { kinds, .. } = ask(r#"{"id":1,"ask":"watch"}"#) else {
            panic!("a watch of every kind wasn't read as a watch");
        };
        assert_eq!(kinds.len(), KINDS.len(), "a watch naming none watches all");
    }

    #[test]
    fn an_act_names_what_joel_said_and_of_which_to_do() {
        assert_eq!(
            ask(r#"{"id":4,"ask":"act","act":"done","item":"QEST-1:sign-in"}"#),
            Ask::Act {
                id: json!(4),
                act: "done".into(),
                item: "QEST-1:sign-in".into()
            },
            "an act on a to-do"
        );
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused_and_its_keys_left_out() {
        assert_eq!(
            ask(r#"{"id":2,"ask":"list","kind":"person"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "person".into()
            },
            "a list of a kind kept elsewhere"
        );
        assert_eq!(
            ask(r#"{"id":3,"ask":"get","keys":["look:a1","person:someone"]}"#),
            Ask::Get {
                id: json!(3),
                keys: vec!["look:a1".into()]
            },
            "a get naming another daemon's key"
        );
    }
}
