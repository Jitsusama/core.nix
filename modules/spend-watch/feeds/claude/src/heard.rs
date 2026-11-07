//! What spend-watch writes on the feed's stdin, of what this feed reads: the
//! pace, and a refresh. What the watch keeps of a feed's own is passed over,
//! since a read always says how the plan stands now.

use serde_json::Value;

use crate::schedule::Pace;

/// Something spend-watch said that changes when the feed reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heard {
    Pace(Pace),
    Refresh,
}

/// What one line says, or nothing for a line this feed passes over: one of
/// another version, one it doesn't read, or one it has no use for.
pub fn heard(line: &str) -> Option<Heard> {
    let said: Value = serde_json::from_str(line).ok()?;
    if said.get("v").and_then(Value::as_u64) != Some(1) {
        return None;
    }
    if said.get("refresh").and_then(Value::as_bool) == Some(true) {
        return Some(Heard::Refresh);
    }
    let pace = said.get("pace")?.as_str()?;
    Some(Heard::Pace(match pace {
        "active" => Pace::Active,
        _ => Pace::Idle,
    }))
}
