//! What the to-do daemon serves in version 1 of the shell's daemon
//! interface (RSCH-20261029-0E8050, "Daemons: One Interface"), which
//! daemon-serve speaks for every daemon: its kinds, and its one ask of its
//! own, Joel's word on a to-do (acts.rs answers it).

use daemon_serve::asks::Domain;
use serde_json::Value;

/// The kinds of record this daemon keeps: each to-do, each look the
/// agents left, and what waits on Joel counted whole.
pub const DOMAIN: Domain = Domain {
    name: "to-do",
    kinds: &["to-do", "look", "waiting"],
    sole: &[],
    named_only: &[],
};

/// The key of the one record that counts what waits.
pub const SUMMARY: &str = "waiting:summary";

/// Joel's word on a to-do, by its identity: `done`, or `notNow`, put off
/// until it says something new.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub act: String,
    pub item: String,
}

/// The act an ask says, if it's one: `{"ask": "act", "act", "item"}`.
pub fn act_of(said: &Value) -> Option<Act> {
    (said["ask"] == "act").then(|| {
        let field = |name: &str| said[name].as_str().unwrap_or("").to_owned();
        Act {
            act: field("act"),
            item: field("item"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_to_dos_key_names_its_kind_however_many_colons_it_holds() {
        assert_eq!(
            DOMAIN.kind_of("to-do:QEST-1:sign-in"),
            Some("to-do"),
            "a to-do's key"
        );
        assert_eq!(
            DOMAIN.kind_of("look:01b53bb1"),
            Some("look"),
            "a look's key"
        );
        assert_eq!(DOMAIN.kind_of(SUMMARY), Some("waiting"), "the count's key");
        assert_eq!(
            DOMAIN.kind_of("to-do:"),
            None,
            "a kind with nothing after it"
        );
        assert_eq!(
            DOMAIN.kind_of("disk:nvme0n1"),
            None,
            "another daemon's kind"
        );
    }

    #[test]
    fn an_act_names_what_joel_said_and_of_which_to_do() {
        assert_eq!(
            act_of(&json!({"id": 4, "ask": "act", "act": "done", "item": "QEST-1:sign-in"})),
            Some(Act {
                act: "done".into(),
                item: "QEST-1:sign-in".into()
            }),
            "an act on a to-do"
        );
        assert_eq!(
            act_of(&json!({"id": 5, "ask": "settle"})),
            None,
            "an ask that isn't an act"
        );
    }
}
