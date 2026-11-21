//! Which live session each cell is now, served as an `agent-cell` record
//! a name it goes by, so a look that names the cell that left it (the
//! to-do daemon's `cell`) goes to it in one keyed read rather than For
//! You joining it against the sessions (Y2YBTS, 85PKB2, 2026-11-20). A
//! cell running or parked is itself. One ended goes to its quest's own
//! session, the one whose address is the quest id's last part, else to
//! another on its quest, one running before one parked, as Looks.cellOf
//! chose in the launcher; with none, it goes nowhere, and says so. It
//! reads every session the node holds, ended ones too, since a look's
//! cell is most often one that has since ended. Where two sessions go by
//! one name, the one that is it now wins: live and named by it first,
//! then live and once named by it, then ended and named by it, then ended
//! and once named by it. One that only lists a name counts only on the
//! quest of the session named by it, if there is one, so a session that
//! lists another's address on another quest never takes its looks. A name
//! no session goes by first, only lists, goes nowhere where its listers
//! sit on more than one quest, since which of them it was can't be told.

use std::collections::HashMap;

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
/// else one running on its quest, else one parked there. `on` holds the
/// live sessions by their quest, gathered once for every ended one.
fn successor<'a>(on: &HashMap<&str, Vec<Session<'a>>>, quest: &str) -> Option<Session<'a>> {
    if quest.is_empty() {
        return None;
    }
    let own = quest.rsplit('-').next().unwrap_or("").to_uppercase();
    let on = on.get(quest)?;
    on.iter()
        .find(|one| one.address().to_uppercase() == own)
        .or_else(|| on.iter().find(|one| one.lives() == Lives::Running))
        .or_else(|| on.first())
        .copied()
}

/// How strongly a session that goes by `name` is that cell: 0 live and
/// named by it now, 1 live and once named by it, 2 ended and named by it,
/// 3 ended and once named by it, and 4 once named by it on another quest
/// than the session named by it now (`owner`'s), or where that session is
/// on none, which no other shares. The least wins.
fn claim(one: Session<'_>, name: &str, owner: Option<&str>) -> u8 {
    let own = one.address() == name;
    if !own && owner.is_some_and(|quest| quest.is_empty() || quest != quest_of(one)) {
        return 4;
    }
    let own = u8::from(!own);
    if live(one) { own } else { 2 + own }
}

/// Every cell's `agent-cell` record, one a name it goes by: the address
/// of the live session it is now (`goes`), or null where none is. Each
/// name is served once, by the session with the strongest claim to it.
pub fn records(sessions: &[Session<'_>]) -> Vec<Value> {
    let mut on: HashMap<&str, Vec<Session>> = HashMap::new();
    for one in sessions.iter().copied().filter(|one| live(*one)) {
        on.entry(quest_of(one)).or_default().push(one);
    }
    // The quest of the session each name is now, a live one's first.
    let mut owners: HashMap<&str, &str> = HashMap::new();
    for one in sessions.iter().copied() {
        let name = one.address();
        if !name.is_empty() && (live(one) || !owners.contains_key(name)) {
            owners.insert(name, quest_of(one));
        }
    }
    // Each name only listed: the one quest its listers share, or none,
    // which listers on no quest never share.
    let mut listed: HashMap<&str, Option<&str>> = HashMap::new();
    for one in sessions.iter().copied() {
        let quest = quest_of(one);
        for name in names(one).filter(|name| !owners.contains_key(name)) {
            listed
                .entry(name)
                .and_modify(|shared| {
                    if *shared != Some(quest) || quest.is_empty() {
                        *shared = None;
                    }
                })
                .or_insert(Some(quest));
        }
    }
    let mut order: Vec<&str> = Vec::new();
    let mut best: HashMap<&str, (u8, Option<&str>)> = HashMap::new();
    for one in sessions.iter().copied() {
        let goes = if live(one) {
            Some(one)
        } else {
            successor(&on, quest_of(one))
        };
        let goes = goes
            .map(Session::address)
            .filter(|address| !address.is_empty());
        for cell in names(one) {
            let rank = claim(one, cell, owners.get(cell).copied());
            match best.get(cell) {
                Some((held, _)) if *held <= rank => {}
                Some(_) => {
                    best.insert(cell, (rank, goes));
                }
                None => {
                    order.push(cell);
                    best.insert(cell, (rank, goes));
                }
            }
        }
    }
    order
        .into_iter()
        .map(|cell| {
            let contested = listed.get(cell).is_some_and(Option::is_none);
            let goes = best
                .get(cell)
                .and_then(|(_, goes)| *goes)
                .filter(|_| !contested);
            json!({
                "key": format!("{KIND}:{PLACE}:{cell}"), "kind": KIND,
                "cell": cell, "goes": goes,
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
            "it says it goes nowhere, so the launcher sends its looks to no stranger"
        );
    }

    #[test]
    fn a_name_two_sessions_go_by_is_the_one_that_is_it_now() {
        let quest = "QEST-20261120-HHHHHH";
        let mut thief = session("T", "running", "QEST-20261120-IIIIII");
        thief["presence"]["addresses"] = json!(["T", "V"]);
        let mut heir = session("W", "running", quest);
        heir["presence"]["addresses"] = json!(["W", "V2"]);
        let mut moved = session("V", "closed", quest);
        moved["presence"]["addresses"] = json!(["V", "V2"]);
        // The quest's own session, which the ended V2 would go to, so the
        // heir's claim is what puts W there.
        let own = session("HHHHHH", "parked", quest);
        let served = goes(&[
            thief.clone(),
            session("V", "parked", quest),
            heir,
            moved,
            own,
        ]);
        assert!(
            served.contains(&"V>\"V\"".to_owned()),
            "a live session named V now keeps V's looks over one listing V: {served:?}"
        );
        assert!(
            served.contains(&"V2>\"W\"".to_owned()),
            "a live session once named V2 holds it over an ended one: {served:?}"
        );
        let ended = goes(&[thief, session("V", "closed", quest)]);
        assert!(
            ended.contains(&"V>null".to_owned()),
            "once V ends, one listing V on another quest still can't take it: {ended:?}"
        );
        assert_eq!(
            served.iter().filter(|one| one.starts_with("V>")).count(),
            1,
            "each name is served once: {served:?}"
        );
    }

    #[test]
    fn a_live_session_once_named_e_on_its_quest_takes_its_looks_over_the_ended_e() {
        let quest = "QEST-20261120-JJJJJJ";
        let mut ended = session("E", "closed", quest);
        ended["presence"]["addresses"] = json!(["E"]);
        let mut other = session("O", "running", quest);
        other["presence"]["addresses"] = json!(["O", "E"]);
        let mut owner = session("JJJJJJ", "parked", quest);
        owner["presence"]["addresses"] = json!(["JJJJJJ"]);
        assert!(
            goes(&[ended, owner, other]).contains(&"E>\"O\"".to_owned()),
            "O is E now, over the ended E, whose successor would be the quest's own"
        );
    }

    #[test]
    fn an_ended_session_with_no_quest_is_given_no_stranger() {
        let mut lister = session("L", "running", "QEST-20261120-LLLLLL");
        lister["presence"]["addresses"] = json!(["L", "N"]);
        assert_eq!(
            goes(&[
                session("N", "closed", ""),
                lister,
                session("R", "running", "QEST-20261120-LLLLLL"),
                session("Z", "running", ""),
            ])[0],
            "N>null",
            "N had no quest, so neither the live L listing it nor Z, on no quest, is N now"
        );
        let mut lister = session("Z", "running", "");
        lister["presence"]["addresses"] = json!(["Z", "N"]);
        assert_eq!(
            goes(&[session("N", "closed", ""), lister])[0],
            "N>null",
            "two sessions on no quest share none, so Z listing N isn't N now"
        );
    }

    #[test]
    fn a_name_only_listed_by_two_on_no_quest_goes_nowhere() {
        let mut one = session("Y", "running", "");
        one["presence"]["addresses"] = json!(["Y", "OLD"]);
        let mut other = session("Z", "running", "");
        other["presence"]["addresses"] = json!(["Z", "OLD"]);
        assert!(
            goes(&[one.clone(), other]).contains(&"OLD>null".to_owned()),
            "on no quest they share none, so neither takes OLD's looks"
        );
        assert!(
            goes(&[one]).contains(&"OLD>\"Y\"".to_owned()),
            "listed by one alone, it is OLD now"
        );
    }

    #[test]
    fn a_name_only_listed_on_two_quests_goes_nowhere() {
        let mut ended = session("E", "closed", "QEST-20261120-AAAAAA");
        ended["presence"]["addresses"] = json!(["E", "OLD"]);
        let mut stranger = session("T", "running", "QEST-20261120-BBBBBB");
        stranger["presence"]["addresses"] = json!(["T", "OLD"]);
        assert!(
            goes(&[ended.clone(), stranger]).contains(&"OLD>null".to_owned()),
            "which of them OLD was can't be told, so neither takes its looks"
        );
        let mut heir = session("H", "running", "QEST-20261120-AAAAAA");
        heir["presence"]["addresses"] = json!(["H", "OLD"]);
        assert!(
            goes(&[ended, heir]).contains(&"OLD>\"H\"".to_owned()),
            "listed on one quest alone, the live one there is OLD now"
        );
    }

    #[test]
    fn a_successor_comes_from_the_ended_cells_own_quest() {
        assert_eq!(
            goes(&[
                session("E", "closed", "QEST-20261120-MMMMMM"),
                session("X", "running", "QEST-20261120-OTHERS"),
                session("S", "parked", "QEST-20261120-MMMMMM"),
            ])[0],
            "E>\"S\"",
            "the parked one on E's quest, never the running one on another"
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
            ["agent-cell:agents:P", "agent-cell:agents:EAC01C97"],
            "the keys the launcher's Looks.cellFor reads"
        );
    }
}
