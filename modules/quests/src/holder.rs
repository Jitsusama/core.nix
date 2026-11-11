//! The session that has each quest loaded, from the node's sessions
//! (mycelium's `session` kind), as the launcher's `Quests.qml` found it in
//! `Sessions.qml`'s running and parked: the first of a session's names
//! that is a quest is the quest it has loaded; one running stands before
//! one parked, and of those alike, this machine's before another's, then
//! the one that changed last.

use std::collections::BTreeMap;

use serde_json::Value;

/// A session holding a quest: its address, and whether it's parked, its
/// memory given back until it's woken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    pub address: String,
    pub parked: bool,
}

/// A session's record's quest, address and how it stands, where it runs
/// or is parked and has a quest loaded.
fn held(record: &Value) -> Option<(String, Holder, bool, i64)> {
    let parked = match record.get("standing").and_then(Value::as_str) {
        Some("running") => false,
        Some("parked") => true,
        _ => return None,
    };
    let presence = record.get("presence")?;
    let quest = presence
        .get("owns")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .find(|owned| owned.starts_with("QEST-"))?;
    let address = presence.get("addresses")?.get(0)?.as_str()?;
    let here = record.get("here").and_then(Value::as_bool).unwrap_or(true);
    let changed = record.get("changed").and_then(Value::as_i64).unwrap_or(0);
    let holder = Holder {
        address: address.to_owned(),
        parked,
    };
    Some((quest.to_owned(), holder, here, changed))
}

/// Each quest's holder, by its id, from every session the node holds.
pub fn holders<'a>(sessions: impl Iterator<Item = &'a Value>) -> BTreeMap<String, Holder> {
    let mut all: Vec<_> = sessions.filter_map(held).collect();
    all.sort_by(|(_, a, a_here, a_changed), (_, b, b_here, b_changed)| {
        a.parked
            .cmp(&b.parked)
            .then_with(|| b_here.cmp(a_here))
            .then_with(|| b_changed.cmp(a_changed))
            .then_with(|| a.address.cmp(&b.address))
    });
    let mut by = BTreeMap::new();
    for (quest, holder, _, _) in all {
        by.entry(quest).or_insert(holder);
    }
    by
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn session(address: &str, standing: &str, owns: &[&str], changed: i64) -> Value {
        json!({"key": format!("session:{address}"), "standing": standing, "changed": changed,
            "presence": {"addresses": [address], "owns": owns}})
    }

    #[test]
    fn one_running_holds_its_quest_before_one_parked() {
        let sessions = [
            session("P", "parked", &["QEST-A"], 9),
            session("R", "running", &["mesh", "QEST-A"], 1),
            session("S", "parked", &["QEST-B"], 1),
            session("C", "closed", &["QEST-C"], 1),
            session("N", "running", &["mesh"], 1),
        ];
        let by = holders(sessions.iter());
        assert_eq!(by.len(), 2, "only running and parked sessions hold");
        assert_eq!(
            by["QEST-A"],
            Holder {
                address: "R".into(),
                parked: false
            }
        );
        assert!(by["QEST-B"].parked, "one parked still holds its quest");
    }

    #[test]
    fn of_two_running_the_one_changed_last_holds_it() {
        let sessions = [
            session("OLD", "running", &["QEST-A"], 1),
            session("NEW", "running", &["QEST-A"], 5),
        ];
        assert_eq!(holders(sessions.iter())["QEST-A"].address, "NEW");
    }
}
