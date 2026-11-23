//! Where the realm's sessions run, machine by machine, from each realm's
//! `mycelium sessions` answer: how many are busy and how many listen on
//! each, with the busy ones on this machine, the one Joel works at,
//! judged. On 23 November he found 30 sessions busy here and none on the
//! cloud machines, the morning after he asked for load to be spread off
//! it ("WE NEED TO FOCUS ON SPREADING LOAD NOW!", 22 November), and
//! nothing had said so.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count};

/// Who moves sessions between machines and sets the order work starts in
/// (WCJQSG, 23 November).
const OWNER: &str = "1OGSE7";

/// The most sessions busy on the machine Joel works at, so his apps and
/// calls keep their room, the rest running elsewhere or waiting (WCJQSG,
/// 23 November).
const BUSY_HERE: f64 = 10.0;

/// One machine's sessions, as the nodes list them.
#[derive(Default)]
struct Carried {
    busy: usize,
    listening: usize,
}

/// The measure, from each realm's `sessions` answer beside its name and
/// each realm's `machines` answer, on `here`, the machine the hour runs
/// on: its busy sessions against `BUSY_HERE`, with each machine's busy and
/// listening behind, busiest first. A machine a realm names but no session
/// runs on is named too, so an idle cloud machine shows. A session two
/// realms list counts once. A realm whose node didn't answer leaves it
/// unjudged, since the sessions it holds would go uncounted.
#[must_use]
pub fn carried(sessions: &[(String, String)], machines: &[String], here: &str) -> Measure {
    let mut each: BTreeMap<String, Carried> = machines
        .iter()
        .filter_map(|answer| crate::elsewhere::machines(answer))
        .flatten()
        .filter_map(|machine| machine["name"].as_str().map(str::to_owned))
        .map(|name| (name, Carried::default()))
        .collect();
    each.entry(here.to_owned()).or_default();
    let mut seen = BTreeSet::new();
    let mut unread = Vec::new();
    for (realm, answer) in sessions {
        let read: Value = serde_json::from_str(answer).unwrap_or_default();
        let Some(listed) = read["Ok"]["Sessions"].as_array() else {
            unread.push(realm.as_str());
            continue;
        };
        for session in listed {
            let (Some(machine), Some(id)) = (
                session["machine"].as_str(),
                session["id"].as_str(),
            ) else {
                continue;
            };
            if !seen.insert(id.to_owned()) {
                continue;
            }
            let one = each.entry(machine.to_owned()).or_default();
            // Idle is the node's word for a session between turns, and
            // only one whose presence listens is woken by a message;
            // parked and ended ones hold no room.
            match (
                session["state"].as_str(),
                session["presence"]["state"].as_str(),
            ) {
                (Some("busy"), _) => one.busy += 1,
                (Some("idle"), Some("listening")) => one.listening += 1,
                _ => {}
            }
        }
    }
    let mut busiest: Vec<(String, Carried)> = each.into_iter().collect();
    busiest.sort_by(|(a, x), (b, y)| {
        y.busy
            .cmp(&x.busy)
            .then(y.listening.cmp(&x.listening))
            .then(a.cmp(b))
    });
    let behind = busiest
        .iter()
        .map(|(name, one)| format!("{name}: {} busy, {} listening", one.busy, one.listening))
        .collect();
    let busy: usize = busiest.iter().map(|(_, one)| one.busy).sum();
    let mine = busiest
        .iter()
        .find(|(name, _)| name == here)
        .map_or(0, |(_, one)| one.busy);
    let mut says = format!(
        "sessions busy on this machine, of {busy} busy on {} machines",
        busiest.len()
    );
    let target = if sessions.is_empty() {
        says.push_str(" (no realm was asked)");
        None
    } else if unread.is_empty() {
        Some(BUSY_HERE)
    } else {
        let silent = unread.join(" and ");
        says.push_str(&format!(" (the {silent} realm's node didn't answer)"));
        None
    };
    Measure {
        key: "sessions-here".to_owned(),
        class: Class::Apps,
        says,
        now: count(mine),
        target,
        owner: OWNER.to_owned(),
        behind,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const MACHINES: &str = concat!(
        r#"{"Ok":{"Machines":"#,
        r#"[{"name":"here"},{"name":"mac"},{"name":"cloud"}]}}"#
    );

    fn session(id: &str, machine: &str, state: &str, presence: &str) -> Value {
        json!({"id": id, "machine": machine, "state": state, "presence": {"state": presence}})
    }

    fn answer(sessions: &[Value]) -> String {
        json!({"Ok": {"Sessions": sessions}}).to_string()
    }

    #[test]
    fn each_machine_says_its_busy_and_listening_and_this_ones_busy_are_judged() {
        let work = answer(&[
            session("a", "here", "busy", "busy"),
            session("b", "here", "busy", "busy"),
            session("c", "here", "busy", "busy"),
            session("d", "here", "idle", "listening"),
            session("e", "here", "idle", "parked"),
            session("f", "here", "ended", "parked"),
            session("g", "mac", "busy", "busy"),
            session("h", "mac", "idle", "listening"),
        ]);
        // One session both realms list, and one ended that still listens.
        let personal = answer(&[
            session("a", "here", "busy", "busy"),
            session("i", "here", "ended", "listening"),
        ]);
        let sessions = [
            ("work".to_owned(), work),
            ("personal".to_owned(), personal),
        ];
        let read = carried(&sessions, &[MACHINES.to_owned()], "here");
        assert_eq!(
            (read.now.to_string(), read.target.map(|t| t.to_string())),
            ("3".to_owned(), Some("10".to_owned())),
            "this machine's busy sessions weren't judged against its most: {}",
            read.says
        );
        assert_eq!(
            read.behind,
            [
                "here: 3 busy, 1 listening",
                "mac: 1 busy, 1 listening",
                "cloud: 0 busy, 0 listening",
            ],
            "a machine's sessions were miscounted, or one carrying none was left out"
        );
        assert!(
            read.says.ends_with("of 4 busy on 3 machines"),
            "the realm's busy sessions were miscounted: {}",
            read.says
        );
    }

    #[test]
    fn a_realm_whose_node_didnt_answer_leaves_it_unjudged() {
        let work = answer(&[session("a", "here", "busy", "busy")]);
        let sessions = [
            ("work".to_owned(), work),
            ("personal".to_owned(), String::new()),
        ];
        let read = carried(&sessions, &[], "here");
        assert!(
            read.target.is_none() && read.says.contains("the personal realm's node didn't answer"),
            "a realm unread was judged without its sessions: {}",
            read.says
        );
        let none = carried(&[], &[], "here");
        assert!(
            none.target.is_none() && none.behind == ["here: 0 busy, 0 listening"],
            "an hour asking no realm was judged: {}",
            none.says
        );
    }
}
