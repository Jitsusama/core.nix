//! The closed orchestrators, which stand dimmed at the end of Agents'
//! tree: each the harness lists (orchestrators.json) with no session
//! running or parked by its address, the one last live first (VKCF3Z,
//! 2026-11-07 [9a4b1dd0]: the one Joel last used is the one he's
//! likeliest to want). Each is a whole record, keyed
//! `closed:agents:<address>`, saying what reopening it needs: the
//! session its quest names to resume and the folder that ran in, the
//! folder a new session on its quest starts in, and which of those are
//! gone, as the node's `quest` records say them. The shell once read
//! the harness's 321 KB quest index itself for this, written again about
//! twice a minute, and ordered them by what an opening had read, so the
//! first opening after the shell started kept the file's order
//! (vm-desktop-looks, 2026-11-11: Beacon then Lamplight).

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use serde_json::{Value, json};

use crate::clock;
use crate::served::{PLACE, SPACING};
use crate::session::{Lives, Session};

/// What a quest says of reopening it, from the node's `quest` record.
#[derive(Debug, Default, PartialEq)]
pub struct Resume<'a> {
    /// The session to resume it by, or "" with none yet.
    pub id: &'a str,
    /// The folder that session ran in, or "".
    pub cwd: &'a str,
    /// When it was last live, in milliseconds, or 0 unknown.
    pub at: i64,
    /// Where a new session on it starts, or "".
    pub start: &'a str,
}

impl<'a> Resume<'a> {
    /// What the node's record of a quest says.
    pub fn of(quest: &'a Value) -> Self {
        let resume = &quest["resume"];
        let text = |value: &'a Value| value.as_str().unwrap_or("");
        Self {
            id: text(&resume["id"]),
            cwd: text(&resume["cwd"]),
            at: live_at(resume),
            start: text(&quest["start"]),
        }
    }
}

/// When a session to resume was last live: the time the harness wrote,
/// or else the stamp its id carries, as a UUID v7 does in its first 48
/// bits; 0 when neither says (SessionRow.js's liveAt).
fn live_at(resume: &Value) -> i64 {
    if let Some(at) = clock::ms_of(&resume["at"]) {
        return at;
    }
    let hex: String = resume["id"]
        .as_str()
        .unwrap_or("")
        .chars()
        .filter(|one| *one != '-')
        .collect();
    let v7 = hex.len() == 32
        && hex.chars().all(|one| one.is_ascii_hexdigit())
        && hex.as_bytes()[12] == b'7';
    if !v7 {
        return 0;
    }
    i64::from_str_radix(&hex[..12], 16).unwrap_or(0)
}

/// The addresses of every session running or parked, any machine's,
/// each by every address its presence answers to.
pub fn live<'a>(sessions: impl Iterator<Item = Session<'a>>) -> HashSet<&'a str> {
    sessions
        .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
        .flat_map(|one| {
            one.presence()
                .and_then(|presence| presence["addresses"].as_array())
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect()
}

/// What the daemon asks of the world beside its sources: what a quest's
/// orchestration spent today, and whether a folder is there.
pub struct Known<'a> {
    pub family: &'a dyn Fn(&str) -> Option<String>,
    pub there: &'a dyn Fn(&str) -> bool,
}

/// Every closed orchestrator's record, the one last live first, those
/// equally live in the order the harness lists them: of those `listed`
/// (orchestrators.json's `orchestrators`), each whose address no session
/// `live` answers to, with what its quest says (`quests`, the node's
/// `quest` records, by quest id).
pub fn closed<S: BuildHasher, T: BuildHasher>(
    listed: &[Value],
    quests: &HashMap<&str, &Value, S>,
    live: &HashSet<&str, T>,
    known: &Known,
) -> Vec<Value> {
    let mut shut: Vec<(usize, &Value, Resume)> = listed
        .iter()
        .enumerate()
        .filter(|(_, one)| {
            let address = one["address"].as_str().unwrap_or("");
            !address.is_empty() && !live.contains(address)
        })
        .map(|(given, one)| {
            let quest = one["quest"].as_str().unwrap_or("");
            let resume = quests
                .get(quest)
                .map(|said| Resume::of(said))
                .unwrap_or_default();
            (given, one, resume)
        })
        .collect();
    shut.sort_by(|a, b| b.2.at.cmp(&a.2.at).then(a.0.cmp(&b.0)));
    (1..)
        .zip(shut)
        .map(|(place, (_, one, resume))| record(place * SPACING, one, &resume, known))
        .collect()
}

/// One closed orchestrator's record: its names, what it spent today,
/// and what reopening it needs. `gone` is null while the folder its
/// session ran in is there, or none is known; otherwise it names that
/// folder, and `start` is "" where a new session would have nowhere to
/// start either.
fn record(order: i64, one: &Value, resume: &Resume, known: &Known) -> Value {
    let text = |field: &str| one[field].as_str().unwrap_or("").to_owned();
    let address = text("address");
    let quest = text("quest");
    let short = text("short");
    let title = text("title");
    let name = [&short, &title, &address]
        .into_iter()
        .find(|one| !one.is_empty())
        .cloned()
        .unwrap_or_default();
    let spent = if quest.is_empty() {
        None
    } else {
        (known.family)(&quest)
    };
    let gone = (!resume.cwd.is_empty() && !(known.there)(resume.cwd)).then(|| {
        let starts = !resume.start.is_empty() && (known.there)(resume.start);
        json!({"cwd": resume.cwd, "start": if starts { resume.start } else { "" }})
    });
    let key = format!("closed:{PLACE}:{address}");
    json!({
        "key": key,
        "kind": "closed",
        "order": order,
        "closed": {
            "address": address,
            "quest": quest,
            "name": name,
            "short": short,
            "title": title,
            "spent": spent,
            "resume": resume.id,
            "cwd": resume.cwd,
            "start": resume.start,
            "at": resume.at,
            "gone": gone,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed() -> Vec<Value> {
        vec![
            json!({"quest": "QEST-BOSSES", "address": "BOSSES", "title": "Orchestrating"}),
            json!({"quest": "QEST-BEACON", "address": "BEACON", "title": "Light the beacon",
                "short": "Beacon"}),
            json!({"quest": "QEST-LAMPLT", "address": "LAMPLT", "title": "Keep the lamp lit",
                "short": "Lamplight"}),
            json!({"quest": "QEST-UNSAID", "address": "UNSAID", "title": "Never resumed"}),
        ]
    }

    fn quests() -> Vec<Value> {
        vec![
            json!({"id": "QEST-BEACON", "start": "/home/skiff/harbour", "resume":
                {"id": "01b4-beacon", "cwd": "/home/skiff/harbour",
                "at": "2026-11-06T22:00:00Z"}}),
            json!({"id": "QEST-LAMPLT", "start": "/home/skiff/gone-too", "resume":
                {"id": "01b4-lamplt", "cwd": "/home/skiff/gone", "at": "2026-11-07T04:00:00Z"}}),
        ]
    }

    fn by_id(quests: &[Value]) -> HashMap<&str, &Value> {
        quests
            .iter()
            .map(|one| (one["id"].as_str().unwrap_or(""), one))
            .collect()
    }

    fn made(live: &HashSet<&str>) -> Vec<Value> {
        let family = |quest: &str| (quest == "QEST-LAMPLT").then(|| "$14.20".to_owned());
        let there = |folder: &str| folder == "/home/skiff/harbour";
        let known = Known {
            family: &family,
            there: &there,
        };
        let quests = quests();
        closed(&listed(), &by_id(&quests), live, &known)
    }

    fn names(records: &[Value]) -> Vec<&str> {
        records
            .iter()
            .map(|one| one["closed"]["name"].as_str().unwrap_or(""))
            .collect()
    }

    #[test]
    fn the_closed_are_those_no_live_session_answers_to_the_last_live_first() {
        let records = made(&HashSet::from(["BOSSES"]));
        assert_eq!(
            names(&records),
            ["Lamplight", "Beacon", "Never resumed"],
            "the closed, the one last live first, though the harness lists Beacon first"
        );
        let orders: Vec<i64> = records
            .iter()
            .map(|one| one["order"].as_i64().unwrap_or(0))
            .collect();
        assert_eq!(
            orders,
            [SPACING, 2 * SPACING, 3 * SPACING],
            "spaced in that order"
        );
        assert_eq!(
            records[0]["key"], "closed:agents:LAMPLT",
            "keyed in Agents' place by its address"
        );
        assert_eq!(
            names(&made(&HashSet::from(["BOSSES", "LAMPLT"]))),
            ["Beacon", "Never resumed"],
            "one a live session answers to isn't closed"
        );
    }

    #[test]
    fn each_says_what_reopening_it_needs_and_which_folders_are_gone() {
        let records = made(&HashSet::from(["BOSSES"]));
        let lamp = &records[0]["closed"];
        assert_eq!(
            (&lamp["resume"], &lamp["cwd"], &lamp["spent"]),
            (
                &json!("01b4-lamplt"),
                &json!("/home/skiff/gone"),
                &json!("$14.20")
            ),
            "Lamplight resumes by its session, where it ran, and says what it spent"
        );
        assert_eq!(
            lamp["gone"],
            json!({"cwd": "/home/skiff/gone", "start": ""}),
            "its folder is gone, and so is where a new one would start"
        );
        let beacon = &records[1]["closed"];
        assert_eq!(
            (&beacon["gone"], &beacon["spent"]),
            (&Value::Null, &Value::Null),
            "Beacon's folder is there, and it spent nothing"
        );
        let unsaid = &records[2]["closed"];
        assert_eq!(
            (&unsaid["resume"], &unsaid["at"], &unsaid["gone"]),
            (&json!(""), &json!(0), &Value::Null),
            "one its quest says nothing of starts afresh, last"
        );
    }

    #[test]
    fn one_whose_start_is_there_starts_afresh_in_it() {
        let family = |_: &str| None;
        let there = |folder: &str| folder == "/home/skiff/gone-too";
        let known = Known {
            family: &family,
            there: &there,
        };
        let quests = quests();
        let records = closed(&listed(), &by_id(&quests), &HashSet::new(), &known);
        let lamp = records
            .iter()
            .find(|one| one["closed"]["address"] == "LAMPLT")
            .expect("Lamplight is closed");
        assert_eq!(
            lamp["closed"]["gone"],
            json!({"cwd": "/home/skiff/gone", "start": "/home/skiff/gone-too"}),
            "its folder is gone, and a new one starts where its quest says"
        );
    }

    #[test]
    fn a_session_with_no_time_is_as_live_as_its_ids_stamp() {
        let stamped = json!({"id": "01a1c3a6-059e-7c35-8413-aabddbbdcee1"});
        assert_eq!(
            live_at(&stamped),
            0x01a1_c3a6_059e,
            "a v7 id's first 48 bits"
        );
        assert_eq!(
            live_at(&json!({"id": "01b4-beacon"})),
            0,
            "none from any other id"
        );
        let v4 = json!({"id": "01a1c3a6-059e-4c35-8413-aabddbbdcee1", "at": "whenever"});
        assert_eq!(
            live_at(&v4),
            0,
            "none from a v4 id, nor a time that isn't one"
        );
        assert_eq!(
            live_at(&json!({"id": "01b4-beacon", "at": "2026-11-07T04:00:00Z"})),
            1_794_024_000_000,
            "the time written wins"
        );
    }

    #[test]
    fn live_takes_every_address_of_those_running_or_parked() {
        let sessions = [
            json!({"standing": "running", "presence": {"addresses": ["BOSSES", "b0sses"]}}),
            json!({"standing": "parked", "presence": {"addresses": ["NAPPER"]}}),
            json!({"standing": "closed", "presence": {"addresses": ["LAMPLT"]}}),
        ];
        let live = live(sessions.iter().map(Session));
        assert_eq!(
            live,
            HashSet::from(["BOSSES", "b0sses", "NAPPER"]),
            "the running and parked, by every address"
        );
    }
}
