//! Which live session each cell is now, served as an `agent-cell` record
//! a name it goes by, so a look that names the cell that left it (the
//! to-do daemon's `cell`) goes to it in one keyed read rather than For
//! You joining it against the sessions (Y2YBTS, 85PKB2, 2026-11-20). A
//! cell running or parked is itself. One ended goes to its quest's own
//! session, the one whose address is the quest id's last part, else to
//! another on its quest, one running before one parked, as Looks.cellOf
//! chose in the launcher; with none, it goes nowhere, and says so.

use serde_json::{Value, json};

use crate::served::PLACE;
use crate::session::{Lives, Session};

/// The kind each record is served as.
pub const KIND: &str = "agent-cell";

/// Whether it's live: running or parked, so a look can go to it.
fn live(one: Session) -> bool {
    matches!(one.lives(), Lives::Running | Lives::Parked)
}

/// The quest it works on: the one it owns, else the one its presence
/// names, as Recent reads it.
fn quest_of(one: Session<'_>) -> &str {
    let owned = one.quest();
    if !owned.is_empty() {
        return owned;
    }
    one.presence()
        .and_then(|presence| {
            presence
                .pointer("/questInfo/id")
                .and_then(Value::as_str)
                .or_else(|| presence.get("quest")?.as_str())
        })
        .unwrap_or("")
}

/// Every name it goes by: its addresses, the first first.
fn names(one: Session<'_>) -> impl Iterator<Item = &str> {
    one.presence()
        .and_then(|presence| presence.get("addresses"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|name| !name.is_empty())
}

/// The live session an ended one's work goes on in: its quest's own,
/// else one running on its quest, else one parked there.
fn successor<'a>(sessions: &[Session<'a>], quest: &str) -> Option<Session<'a>> {
    if quest.is_empty() {
        return None;
    }
    let own = quest.rsplit('-').next().unwrap_or("").to_uppercase();
    let on: Vec<Session> = sessions
        .iter()
        .copied()
        .filter(|one| live(*one) && quest_of(*one) == quest)
        .collect();
    on.iter()
        .find(|one| one.address().to_uppercase() == own)
        .or_else(|| on.iter().find(|one| one.lives() == Lives::Running))
        .or_else(|| on.first())
        .copied()
}

/// Every cell's `agent-cell` record, one a name it goes by: the address
/// of the live session it is now (`goes`), or null where none is.
pub fn records(sessions: &[Session<'_>]) -> Vec<Value> {
    sessions
        .iter()
        .flat_map(|one| {
            let goes = if live(*one) {
                Some(*one)
            } else {
                successor(sessions, quest_of(*one))
            };
            let goes = goes
                .map(Session::address)
                .filter(|address| !address.is_empty());
            names(*one).map(move |cell| {
                json!({
                    "key": format!("{KIND}:{PLACE}:{cell}"), "kind": KIND,
                    "cell": cell, "goes": goes,
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(address: &str, standing: &str, quest: &str) -> Value {
        json!({"key": format!("session:{address}"), "standing": standing,
            "presence": {"addresses": [address], "owns": [quest]}})
    }

    fn goes(sessions: &[Value]) -> Vec<String> {
        let sessions: Vec<Session> = sessions.iter().map(Session).collect();
        records(&sessions)
            .iter()
            .map(|one| format!("{}>{}", one["cell"].as_str().unwrap_or(""), one["goes"]))
            .collect()
    }

    #[test]
    fn a_live_cell_goes_to_itself() {
        assert_eq!(
            goes(&[
                session("P", "running", "QEST-20261120-AAAAAA"),
                session("Q", "parked", "QEST-20261120-BBBBBB"),
            ]),
            ["P>\"P\"", "Q>\"Q\""],
            "running or parked, a look goes to the cell that left it"
        );
    }

    #[test]
    fn an_ended_cell_goes_to_its_quests_own_session_first() {
        let quest = "QEST-20261120-OWNSIT";
        assert_eq!(
            goes(&[
                session("E", "closed", quest),
                session("R", "running", quest),
                session("OWNSIT", "parked", quest),
            ])[0],
            "E>\"OWNSIT\"",
            "the quest's own session, parked, before another running on it"
        );
    }

    #[test]
    fn an_ended_cell_goes_to_one_running_on_its_quest_before_one_parked() {
        let quest = "QEST-20261120-CCCCCC";
        assert_eq!(
            goes(&[
                session("E", "closed", quest),
                session("S", "parked", quest),
                session("R", "running", quest),
                session("X", "running", "QEST-20261120-DDDDDD"),
            ])[0],
            "E>\"R\"",
            "of those on its quest, the running one; never one on another quest"
        );
    }

    #[test]
    fn an_ended_cell_found_by_its_presences_quest_too() {
        let mut ended = session("E", "passedOver", "");
        ended["presence"]["owns"] = json!([]);
        ended["presence"]["questInfo"] = json!({"id": "QEST-20261120-EEEEEE"});
        let mut on = session("R", "running", "");
        on["presence"]["owns"] = json!([]);
        on["presence"]["quest"] = json!("QEST-20261120-EEEEEE");
        assert_eq!(goes(&[ended, on])[0], "E>\"R\"");
    }

    #[test]
    fn an_ended_cell_with_none_live_on_its_quest_goes_nowhere() {
        assert_eq!(
            goes(&[
                session("E", "closed", "QEST-20261120-FFFFFF"),
                session("N", "closed", ""),
            ]),
            ["E>null", "N>null"],
            "it says it goes nowhere, so the launcher says the cell has ended"
        );
    }

    #[test]
    fn every_name_a_cell_goes_by_is_keyed() {
        let mut two = session("P", "running", "QEST-20261120-GGGGGG");
        two["presence"]["addresses"] = json!(["P", "EAC01C97"]);
        let sessions = [two];
        let sessions: Vec<Session> = sessions.iter().map(Session).collect();
        let keys: Vec<_> = records(&sessions)
            .iter()
            .map(|one| one["key"].as_str().unwrap_or("").to_owned())
            .collect();
        assert_eq!(
            keys,
            [
                format!("{KIND}:{PLACE}:P"),
                format!("{KIND}:{PLACE}:EAC01C97")
            ]
        );
    }
}
