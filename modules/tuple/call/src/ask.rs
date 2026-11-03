//! What a client asks, one JSON line of it, in the shell's daemon
//! interface (RSCH-20261029-0E8050, "Daemons: One Interface"), version
//! 1, each ask with an `id` echoed in its answers: the interface's own
//! asks, and the call's acts, each done through Tuple's own command.

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The one kind of record this daemon keeps.
pub const KIND: &str = "call";

/// The key of its one record, the call Joel's in or none.
pub const KEY: &str = "call:current";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Hello,
    Health,
    /// The record, when its key is among these.
    Get {
        keys: Vec<String>,
    },
    /// Every record of its kind, which is the one.
    List,
    /// The record now and each time it changes.
    Watch,
    Unwatch,
    /// Joins the call a link opens.
    Join {
        link: String,
    },
    Leave,
    /// Keeps Joel's voice from the call, or sends it again.
    Mute {
        muted: bool,
    },
    /// Shares a screen, which the desktop's portal asks Joel to pick.
    Share,
    /// Answers the call ringing.
    Accept,
    /// Opens Joel's room, answering with its link.
    Open,
    /// A kind this daemon doesn't keep.
    UnknownKind {
        kind: String,
    },
    /// An ask it doesn't know, or a line that isn't one.
    Unknown {
        ask: String,
    },
}

/// What a line asks, and the id to answer it by.
pub fn read(line: &str) -> (Value, Ask) {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return (Value::Null, Ask::Unknown { ask: String::new() });
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
    let kind = said.get("kind").and_then(Value::as_str).unwrap_or(KIND);
    if matches!(asked, "list" | "watch") && kind != KIND {
        return (
            id,
            Ask::UnknownKind {
                kind: kind.to_owned(),
            },
        );
    }
    let ask = match asked {
        "hello" => Ask::Hello,
        "health" => Ask::Health,
        "get" => Ask::Get {
            keys: said
                .get("keys")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        },
        "list" => Ask::List,
        "watch" => Ask::Watch,
        "unwatch" => Ask::Unwatch,
        "join" => Ask::Join {
            link: link(said.get("link").and_then(Value::as_str).unwrap_or("")).to_owned(),
        },
        "leave" => Ask::Leave,
        "mute" => Ask::Mute {
            muted: said.get("muted").and_then(Value::as_bool) != Some(false),
        },
        "share" => Ask::Share,
        "accept" => Ask::Accept,
        "open" => Ask::Open,
        other => Ask::Unknown {
            ask: other.to_owned(),
        },
    };
    (id, ask)
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// The first call link in some text, as `tuple call join` takes one, or
/// "": `https://tuple.app/c/` or `https://production.tuple.app/c/` and
/// the room's letters, digits, dashes and underscores.
pub fn link(text: &str) -> &str {
    let mut from = 0;
    while let Some(found) = text[from..].find("https://") {
        let start = from + found;
        from = start + 1;
        let rest = &text[start + "https://".len()..];
        let rest = rest.strip_prefix("production.").unwrap_or(rest);
        let Some(room) = rest.strip_prefix("tuple.app/c/") else {
            continue;
        };
        let length = room
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(room.len());
        if length > 0 {
            let end = text.len() - room.len() + length;
            return &text[start..end];
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(line: &str) -> Ask {
        read(line).1
    }

    #[test]
    fn each_ask_is_read_with_its_id() {
        assert_eq!(
            read(r#"{"id": "a", "ask": "hello", "version": 1}"#),
            (json!("a"), Ask::Hello)
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "get", "keys": ["call:current", 7]}"#),
            Ask::Get {
                keys: vec!["call:current".into()]
            }
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "watch", "kind": "call"}"#),
            Ask::Watch
        );
        assert_eq!(asked(r#"{"id": 1, "ask": "list"}"#), Ask::List);
        assert_eq!(
            asked(r#"{"id": 1, "ask": "mute", "muted": false}"#),
            Ask::Mute { muted: false }
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "mute"}"#),
            Ask::Mute { muted: true }
        );
        for (word, ask) in [
            ("leave", Ask::Leave),
            ("share", Ask::Share),
            ("accept", Ask::Accept),
            ("open", Ask::Open),
            ("unwatch", Ask::Unwatch),
            ("health", Ask::Health),
        ] {
            assert_eq!(asked(&format!(r#"{{"id": 1, "ask": "{word}"}}"#)), ask);
        }
    }

    #[test]
    fn a_join_takes_only_a_call_link() {
        assert_eq!(
            asked(r#"{"id": 1, "ask": "join", "link": "see https://tuple.app/c/ab_C-9 now"}"#),
            Ask::Join {
                link: "https://tuple.app/c/ab_C-9".into()
            }
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "join", "link": "https://example.com/c/x"}"#),
            Ask::Join {
                link: String::new()
            }
        );
    }

    #[test]
    fn another_kind_or_ask_is_refused() {
        assert_eq!(
            asked(r#"{"id": 1, "ask": "watch", "kind": "person"}"#),
            Ask::UnknownKind {
                kind: "person".into()
            }
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "dance"}"#),
            Ask::Unknown {
                ask: "dance".into()
            }
        );
        assert_eq!(
            read("not json"),
            (Value::Null, Ask::Unknown { ask: String::new() })
        );
    }

    #[test]
    fn a_link_is_found_in_text_as_tuple_call_join_takes_it() {
        assert_eq!(
            link("https://production.tuple.app/c/xyz?x=1"),
            "https://production.tuple.app/c/xyz"
        );
        assert_eq!(link("https://tuple.app/c/"), "");
        assert_eq!(
            link("https://tuple.app/x https://tuple.app/c/ok"),
            "https://tuple.app/c/ok"
        );
        assert_eq!(link("nothing"), "");
    }
}
