//! What the machine daemon keeps, as the shell's daemon interface names
//! it (docs/daemons.md), which it speaks through daemon-serve: the kinds
//! of record, the places its rows are made for, and its own ask, `act`.

use daemon_serve::asks::Domain;
use serde_json::{Value, json};

/// The kinds of record this daemon keeps: what it reads, then what it
/// makes of them, a place's ready rows and its headline (`MADE`).
pub const KINDS: [&str; 30] = [
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
    "drawer",
];

/// The kinds made of what's read rather than read themselves, each
/// keyed by its place: `row:heat:processor:0`, `head:heat`, and the
/// drawer a row names, `drawer:heat:processor:0`.
pub const MADE: [&str; 3] = ["row", "head", "drawer"];

/// The kind told only to a watch naming its key, never to one of its
/// kind or place: a drawer's kernel counts move on every read, and only
/// the row the launcher rests on shows them (VKCF3Z, 2026-11-12).
pub const APART: &str = "drawer";

/// The machine daemon as its clients know it.
pub const DOMAIN: Domain = Domain {
    name: "machine",
    kinds: &KINDS,
    sole: &[],
    named_only: &[APART],
};

/// The place a made record's key names: `heat` for `row:heat:fan:1`,
/// and none for a record read rather than made.
pub fn place_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    MADE.contains(&kind)
        .then(|| rest.split_once(':').map_or(rest, |(place, _)| place))
}

/// The kind a key names: `disk` for `disk:nvme0n1`.
pub fn kind_of(key: &str) -> Option<&str> {
    DOMAIN.kind_of(key)
}

/// The act an ask of the daemon's own names, with the rest of what was
/// said as its arguments; none for an ask that isn't an act.
pub fn act_of(said: &Value) -> Option<&str> {
    (said["ask"] == "act").then(|| said["act"].as_str().unwrap_or(""))
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
    fn a_made_record_names_its_place_and_one_read_names_none() {
        assert_eq!(
            [
                "row:heat:fan:dell_smm:1",
                "head:heat",
                "drawer:heat:fan:dell_smm:1",
                "sensor:coretemp"
            ]
            .map(place_of),
            [Some("heat"), Some("heat"), Some("heat"), None]
        );
    }

    #[test]
    fn a_key_of_a_kind_it_doesnt_keep_names_none() {
        assert_eq!(
            ["disk:nvme0n1", "person:ada@example.com", "disk:"].map(kind_of),
            [Some("disk"), None, None]
        );
    }

    #[test]
    fn an_act_is_named_by_its_act_and_nothing_else_is_one() {
        let act = json!({"id": 4, "ask": "act", "act": "charge-limit", "supply": "BAT0"});
        assert_eq!(act_of(&act), Some("charge-limit"), "an act names its act");
        let other = json!({"id": 5, "ask": "shout"});
        assert_eq!(act_of(&other), None, "an ask that isn't an act is none");
    }
}
