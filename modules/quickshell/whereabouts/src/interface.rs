//! What Location serves in the shell's daemon interface, which
//! daemon-serve speaks for it (docs/daemons.md): the kinds of record it
//! keeps, and Joel's word on where the laptop is, the one ask of its own.

use daemon_serve::asks::Domain;
use serde_json::{Value, json};

/// Where the laptop is, and Location's rows, ready for the launcher
/// (rows.rs), as daemon-serve answers for them.
pub const DOMAIN: Domain = Domain {
    name: "location",
    kinds: &["here", "row"],
    sole: &[],
    named_only: &[],
};

/// The key where the laptop is, the laptop's.
pub const HERE: &str = "here:laptop";

/// Joel's word on where the laptop is: `set`, to the town `words` name,
/// or `again`, to have GeoClue find it once more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub act: String,
    pub words: String,
}

/// The act an ask of Location's own says, or none for any other ask,
/// which daemon-serve refuses as one it doesn't know.
pub fn act_of(said: &Value) -> Option<Act> {
    if said.get("ask").and_then(Value::as_str) != Some("act") {
        return None;
    }
    let text = |field: &str| {
        said.get(field)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    Some(Act {
        act: text("act"),
        words: text("words"),
    })
}

/// An error answer, by the interface's codes, its id given by
/// daemon-serve.
pub fn error(code: &str, message: &str) -> Value {
    json!({"error": {"code": code, "message": message}})
}

/// A refusal Joel reads: beside the code and what went wrong, `said`,
/// his words for it, which the launcher says as they are (ready-rows.md's
/// acts), so the shell words no refusal of its own.
pub fn refused(code: &str, message: &str, said: &str) -> Value {
    json!({"error": {"code": code, "message": message, "said": said}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_names_its_kind() {
        assert_eq!(DOMAIN.kind_of(HERE), Some("here"));
        assert_eq!(DOMAIN.kind_of("row:location:town"), Some("row"));
        assert_eq!(
            DOMAIN.kind_of("disk:nvme0n1"),
            None,
            "a kind it doesn't keep"
        );
        assert_eq!(DOMAIN.kind_of("here:"), None, "a kind naming nothing");
    }

    #[test]
    fn an_act_names_what_joel_said() {
        assert_eq!(
            act_of(&json!({"id": 4, "ask": "act", "act": "set", "words": "Kelpwater"})),
            Some(Act {
                act: "set".into(),
                words: "Kelpwater".into(),
            })
        );
        assert_eq!(
            act_of(&json!({"id": 5, "ask": "act", "act": "again"})),
            Some(Act {
                act: "again".into(),
                words: String::new(),
            }),
            "again names no words"
        );
        assert_eq!(
            act_of(&json!({"id": 6, "ask": "move"})),
            None,
            "another ask is daemon-serve's to refuse"
        );
    }
}
