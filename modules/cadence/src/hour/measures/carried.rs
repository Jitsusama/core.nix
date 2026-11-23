//! Where the realm's sessions run, machine by machine, from each realm's
//! `mycelium sessions` answer: how many are busy and how many listen on
//! each, with the busy ones on this machine, the one Joel works at,
//! judged. On 23 November he found 30 sessions busy here and none on the
//! cloud machines, the morning after he asked for load to be spread off
//! it ("WE NEED TO FOCUS ON SPREADING LOAD NOW!", 22 November), and
//! nothing had said so.

use std::collections::BTreeMap;

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

/// How a listing says a session stands, least lively first. Idle is the
/// node's word for a session between turns, and only one whose presence
/// listens is woken by a message; parked and ended ones hold no room.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lively {
    Resting,
    Listening,
    Busy,
}

impl Lively {
    fn of(session: &Value) -> Self {
        match (
            session["state"].as_str(),
            session["presence"]["state"].as_str(),
        ) {
            (Some("busy"), _) => Self::Busy,
            (Some("idle"), Some("listening")) => Self::Listening,
            _ => Self::Resting,
        }
    }
}

/// The measure, from each realm's `sessions` answer beside its name and
/// each realm's `machines` answer, on `here`, the machine the hour runs
/// on: its busy sessions against `BUSY_HERE`, with each machine's busy and
/// listening behind, busiest first. A machine a realm names but no session
/// runs on is named too, so an idle cloud machine shows, and sessions
/// naming no machine are a row of their own.
///
/// It is unjudged while the count here may fall short: a realm's node
/// didn't answer, no node names this machine, or a busy session names no
/// machine. A count already past the most is judged all the same, since
/// what wasn't counted could only add to it.
#[must_use]
pub fn carried(sessions: &[(String, String)], machines: &[String], here: &str) -> Measure {
    let mut each: BTreeMap<String, Carried> = machines
        .iter()
        .filter_map(|answer| crate::elsewhere::machines(answer))
        .flatten()
        .filter_map(|machine| machine["name"].as_str().map(str::to_owned))
        .map(|name| (name, Carried::default()))
        .collect();
    let (listed, unread) = listings(sessions);
    let named_here = each.contains_key(here) || listed.values().any(|(_, m)| m == here);
    each.entry(here.to_owned()).or_default();
    let mut unplaced = Carried::default();
    for (lively, machine) in listed.into_values() {
        let one = if machine.is_empty() {
            &mut unplaced
        } else {
            each.entry(machine).or_default()
        };
        match lively {
            Lively::Busy => one.busy += 1,
            Lively::Listening => one.listening += 1,
            Lively::Resting => {}
        }
    }
    let mut busiest: Vec<(String, Carried)> = each.into_iter().collect();
    busiest.sort_by(|(a, x), (b, y)| {
        y.busy
            .cmp(&x.busy)
            .then(y.listening.cmp(&x.listening))
            .then(a.cmp(b))
    });
    let mut behind: Vec<String> = busiest
        .iter()
        .map(|(name, one)| format!("{name}: {} busy, {} listening", one.busy, one.listening))
        .collect();
    if unplaced.busy + unplaced.listening > 0 {
        behind.push(format!(
            "no machine named: {} busy, {} listening",
            unplaced.busy, unplaced.listening
        ));
    }
    let busy = busiest.iter().map(|(_, one)| one.busy).sum::<usize>() + unplaced.busy;
    let mine = busiest
        .iter()
        .find(|(name, _)| name == here)
        .map_or(0, |(_, one)| one.busy);
    let mut unsure = Vec::new();
    if sessions.is_empty() {
        unsure.push("no realm was asked".to_owned());
    } else {
        unsure.extend(silent(&unread));
        if !named_here {
            unsure.push(format!("no node names this machine, {here}"));
        }
    }
    if unplaced.busy > 0 {
        unsure.push(format!("{} busy name no machine", unplaced.busy));
    }
    let target = (unsure.is_empty() || count(mine) > BUSY_HERE).then_some(BUSY_HERE);
    let unjudged = if unsure.is_empty() {
        String::new()
    } else {
        format!(" ({})", unsure.join("; "))
    };
    let says = format!(
        "sessions busy on this machine, of {busy} busy on {} machines{unjudged}",
        busiest.len()
    );
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

/// Each session the realms list, by id, as its liveliest listing says it
/// stands and where it runs, so an ended copy one realm still lists can't
/// hide the busy one another does, as `cells` prefers a live listing; and
/// the realms whose answer couldn't be read. A session listed with no id
/// can't be matched across realms, so it counts as listed.
fn listings(sessions: &[(String, String)]) -> (BTreeMap<String, (Lively, String)>, Vec<&str>) {
    let mut listed: BTreeMap<String, (Lively, String)> = BTreeMap::new();
    let mut unread = Vec::new();
    for (realm, answer) in sessions {
        let read: Value = serde_json::from_str(answer).unwrap_or_default();
        let Some(all) = read["Ok"]["Sessions"].as_array() else {
            unread.push(realm.as_str());
            continue;
        };
        for (n, session) in all.iter().enumerate() {
            let lively = Lively::of(session);
            let machine = session["machine"].as_str().unwrap_or_default().to_owned();
            let id = session["id"]
                .as_str()
                .map_or_else(|| format!("{realm} #{n}"), str::to_owned);
            if listed.get(&id).is_none_or(|(was, _)| *was < lively) {
                listed.insert(id, (lively, machine));
            }
        }
    }
    (listed, unread)
}

/// Why the realms in `unread` leave the count short, if any are.
fn silent(unread: &[&str]) -> Option<String> {
    match unread {
        [] => None,
        [one] => Some(format!("the {one} realm's node didn't answer")),
        many => Some(format!("the {} realms' nodes didn't answer", many.join(" and "))),
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

    fn judged(read: &Measure) -> (String, Option<String>) {
        (read.now.to_string(), read.target.map(|t| t.to_string()))
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
        let sessions = [("work".to_owned(), work), ("personal".to_owned(), personal)];
        let read = carried(&sessions, &[MACHINES.to_owned()], "here");
        assert_eq!(
            judged(&read),
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
    fn a_session_two_realms_list_counts_as_its_liveliest_listing() {
        let ended = answer(&[session("a", "mac", "ended", "parked")]);
        let busy = answer(&[session("a", "here", "busy", "busy")]);
        for (first, second) in [(&ended, &busy), (&busy, &ended)] {
            let sessions = [
                ("work".to_owned(), first.clone()),
                ("personal".to_owned(), second.clone()),
            ];
            let read = carried(&sessions, &[MACHINES.to_owned()], "here");
            assert_eq!(
                (read.now.to_string(), read.behind[0].as_str()),
                ("1".to_owned(), "here: 1 busy, 0 listening"),
                "an ended copy of a busy session hid it: {:?}",
                read.behind
            );
        }
    }

    #[test]
    fn this_machine_past_its_most_is_judged_though_a_realm_is_silent() {
        let work: Vec<Value> = (0..11)
            .map(|n| session(&format!("s{n}"), "here", "busy", "busy"))
            .collect();
        let sessions = [
            ("work".to_owned(), answer(&work)),
            ("personal".to_owned(), String::new()),
        ];
        let read = carried(&sessions, &[MACHINES.to_owned()], "here");
        assert!(
            judged(&read) == ("11".to_owned(), Some("10".to_owned()))
                && read.says.contains("the personal realm's node didn't answer"),
            "a count past the most went unjudged, or the silent realm unsaid: {}",
            read.says
        );
    }

    #[test]
    fn a_count_that_may_fall_short_is_left_unjudged_and_says_why() {
        let work = answer(&[session("a", "here", "busy", "busy")]);
        let sessions = [
            ("work".to_owned(), work.clone()),
            ("personal".to_owned(), String::new()),
        ];
        let read = carried(&sessions, &[], "here");
        assert!(
            read.target.is_none()
                && read.behind == ["here: 1 busy, 0 listening"]
                && read
                    .says
                    .contains("the personal realm's node didn't answer"),
            "a realm unread was judged without its sessions, or the others miscounted: {}",
            read.says
        );
        let both = carried(
            &[
                ("work".to_owned(), String::new()),
                ("personal".to_owned(), String::new()),
            ],
            &[],
            "here",
        );
        assert!(
            both.says.contains("the work and personal realms' nodes didn't answer"),
            "two silent realms were misworded: {}",
            both.says
        );
        let elsewhere = answer(&[session("a", "mac", "busy", "busy")]);
        let unnamed = carried(
            &[("work".to_owned(), elsewhere)],
            &[MACHINES.replace("\"here\"", "\"there\"")],
            "here",
        );
        assert!(
            unnamed.target.is_none() && unnamed.says.contains("no node names this machine, here"),
            "a machine no node names read as carrying none: {}",
            unnamed.says
        );
        let none = carried(&[], &[], "here");
        assert!(
            none.target.is_none() && none.behind == ["here: 0 busy, 0 listening"],
            "an hour asking no realm was judged: {}",
            none.says
        );
    }

    #[test]
    fn sessions_naming_no_machine_or_no_id_are_counted_and_said() {
        let work = answer(&[
            session("a", "here", "busy", "busy"),
            session("b", "", "busy", "busy"),
            json!({"id": "c", "state": "busy", "presence": {"state": "busy"}}),
            json!({"machine": "here", "state": "idle", "presence": {"state": "listening"}}),
            json!({"machine": "here", "state": "idle", "presence": {"state": "listening"}}),
        ]);
        let read = carried(&[("work".to_owned(), work)], &[MACHINES.to_owned()], "here");
        assert!(
            read.target.is_none()
                && read.behind.first().map(String::as_str) == Some("here: 1 busy, 2 listening")
                && read.behind.last().map(String::as_str)
                    == Some("no machine named: 2 busy, 0 listening")
                && read.says.contains("of 3 busy")
                && read.says.contains("2 busy name no machine"),
            "a session with no machine or no id dropped out unsaid: {:?} {}",
            read.behind,
            read.says
        );
    }
}
