//! The call's acts, as a client asks them in the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"), each done through
//! Tuple's own command. The interface's own asks, and the socket, are the
//! serving side every daemon shares (../../daemon-serve).

use serde_json::{Value, json};

/// The one kind of record this daemon keeps.
pub const KIND: &str = "call";

/// The key of its one record, the call Joel's in or none.
pub const KEY: &str = "call:current";

/// The asks that are acts, each answered on a thread of its own, since
/// each waits on Tuple's command, and a share on Joel picking a screen.
pub const ACTS: &[&str] = &["join", "leave", "mute", "share", "accept", "open"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
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
}

/// The act an ask is, or none for an ask that isn't one.
pub fn act(said: &Value) -> Option<Act> {
    let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
    Some(match asked {
        "join" => Act::Join {
            link: link(said.get("link").and_then(Value::as_str).unwrap_or("")).to_owned(),
        },
        "leave" => Act::Leave,
        "mute" => Act::Mute {
            muted: said.get("muted").and_then(Value::as_bool) != Some(false),
        },
        "share" => Act::Share,
        "accept" => Act::Accept,
        "open" => Act::Open,
        _ => return None,
    })
}

/// An act refused, by the interface's code for it, and why.
pub fn refused(message: &str) -> Value {
    json!({"error": {"code": "refused", "message": message}})
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

    fn asked(line: &str) -> Option<Act> {
        act(&serde_json::from_str(line).expect("the ask is JSON"))
    }

    #[test]
    fn each_act_is_read() {
        assert_eq!(
            asked(r#"{"id": 1, "ask": "mute", "muted": false}"#),
            Some(Act::Mute { muted: false })
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "mute"}"#),
            Some(Act::Mute { muted: true })
        );
        for (word, act) in [
            ("leave", Act::Leave),
            ("share", Act::Share),
            ("accept", Act::Accept),
            ("open", Act::Open),
        ] {
            assert_eq!(
                asked(&format!(r#"{{"id": 1, "ask": "{word}"}}"#)),
                Some(act)
            );
        }
    }

    #[test]
    fn each_act_is_answered_apart() {
        for word in ACTS {
            assert!(
                asked(&format!(r#"{{"id": 1, "ask": "{word}"}}"#)).is_some(),
                "{word} is an act"
            );
        }
    }

    #[test]
    fn a_join_takes_only_a_call_link() {
        assert_eq!(
            asked(r#"{"id": 1, "ask": "join", "link": "see https://tuple.app/c/ab_C-9 now"}"#),
            Some(Act::Join {
                link: "https://tuple.app/c/ab_C-9".into()
            })
        );
        assert_eq!(
            asked(r#"{"id": 1, "ask": "join", "link": "https://example.com/c/x"}"#),
            Some(Act::Join {
                link: String::new()
            })
        );
    }

    #[test]
    fn an_ask_that_is_no_act_is_none() {
        assert_eq!(asked(r#"{"id": 1, "ask": "dance"}"#), None);
        assert_eq!(asked(r#"{"id": 1, "ask": "hello"}"#), None);
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
