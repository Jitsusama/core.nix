//! A session as the node tells of it (mycelium's `session` kind): its
//! record read the way the launcher reads it, how it stands, whether
//! what it wants is truly Joel's, where it waits in the machine's queue,
//! and the order the sessions are listed in. Ported from the shell's
//! Sessions.qml, Waiting.js and Jobs.qml, whose cases these tests carry.

use std::collections::HashSet;

use serde_json::Value;

use crate::standing::{self, Stands};

/// One session's record, as the node holds it.
#[derive(Clone, Copy)]
pub struct Session<'a>(pub &'a Value);

/// How the node says a session stands this boot (`standing`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lives {
    Running,
    Parked,
    Closed,
    PassedOver,
}

impl<'a> Session<'a> {
    /// What the session's harness publishes of itself, or null.
    pub fn presence(self) -> Option<&'a Value> {
        self.0
            .get("presence")
            .filter(|presence| presence.is_object())
    }

    fn said(self, field: &str) -> Option<&'a str> {
        self.presence()?.get(field)?.as_str()
    }

    /// The address it goes by first, or none.
    pub fn address(self) -> &'a str {
        self.presence()
            .and_then(|presence| presence.get("addresses"))
            .and_then(|addresses| addresses.get(0))
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    /// The address of the session orchestrating it, or none.
    pub fn under(self) -> &'a str {
        self.said("orchestratedBy").unwrap_or("")
    }

    /// Pi's own id for it, by which its spend and jobs are kept.
    pub fn id(self) -> &'a str {
        self.said("session").unwrap_or("")
    }

    /// The quest it owns, the first of its names that is one.
    pub fn quest(self) -> &'a str {
        self.presence()
            .and_then(|presence| presence.get("owns"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .find(|owned| owned.starts_with("QEST-"))
            .unwrap_or("")
    }

    /// Its quest's whole title and its folder's name (Sessions.named):
    /// from its record's quest and folder, or, for a record from before
    /// the harness published them, from its window's label; its address
    /// where nothing names it.
    pub fn named(self) -> (String, String) {
        let label = labelled(self.said("title").unwrap_or(""));
        let title = self
            .presence()
            .and_then(|presence| presence.pointer("/questInfo/title"))
            .and_then(Value::as_str)
            .filter(|title| !title.is_empty())
            .map_or(label.0, str::to_owned);
        let folder = self.said("cwd").unwrap_or("").trim_end_matches('/');
        let folder = if folder.is_empty() {
            label.1
        } else {
            folder.rsplit('/').next().unwrap_or(folder).to_owned()
        };
        let title = if title.is_empty() {
            self.address().to_owned()
        } else {
            title
        };
        (title, folder)
    }

    /// How the node says it stands this boot.
    pub fn lives(self) -> Lives {
        match self.0.get("standing").and_then(Value::as_str) {
            Some("running") => Lives::Running,
            Some("parked") => Lives::Parked,
            Some("closed") => Lives::Closed,
            _ => Lives::PassedOver,
        }
    }

    /// Whether it runs on this machine.
    pub fn here(self) -> bool {
        self.0.get("here").and_then(Value::as_bool).unwrap_or(true)
    }

    /// The machine it runs on, as the node names it.
    pub fn machine(self) -> &'a str {
        self.0.get("machine").and_then(Value::as_str).unwrap_or("")
    }

    /// Whether its machine can be reached.
    pub fn reachable(self) -> bool {
        self.0
            .get("reachable")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    /// When the node last heard it change, in milliseconds.
    pub fn changed(self) -> u64 {
        self.0.get("changed").and_then(Value::as_u64).unwrap_or(0)
    }

    /// How it stands among the others, given who has something Joel can
    /// do now and the machine's queue.
    pub fn stands(self, joel_can: &JoelCan, jobs: &[Value]) -> Stands {
        standing::state(
            self.presence(),
            asks_joel(self.presence(), joel_can),
            in_line(jobs, self.id()).is_some(),
        )
    }
}

/// A window's label, "π - ", its quest's name and " - " its folder, split
/// at the last " - ", since a quest's name may hold one and a folder's
/// name hyphens.
fn labelled(title: &str) -> (String, String) {
    let rest = title.strip_prefix("\u{3c0} - ").unwrap_or(title);
    match rest.rfind(" - ") {
        Some(last) => (
            rest[..last].trim().to_owned(),
            rest[last + 3..].trim().to_owned(),
        ),
        None => (rest.trim().to_owned(), String::new()),
    }
}

/// Who has something Joel can do now, by every name a session's presence
/// might give them: a quest, or a session's own address (Waiting.standing).
#[derive(Debug, Default)]
pub struct JoelCan(pub HashSet<String>);

impl JoelCan {
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

/// Whether a session's want is truly on Joel (Waiting.asksJoel). A yes it
/// waits for, or a run stuck without him, only his keys answer. An action
/// stands only while its quest, or the session itself, has something he
/// can do now (`joel_can`), since a presence mirrors every item it keeps,
/// ready or not. A question from a session another orchestrates is put
/// to its orchestrator, who brings Joel what's his.
pub fn asks_joel(presence: Option<&Value>, joel_can: &JoelCan) -> bool {
    let Some(presence) = presence else {
        return false;
    };
    let Some(wants) = presence.get("wants").filter(|wants| is_set(wants)) else {
        return false;
    };
    let names = |field| {
        presence
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
    };
    match wants.get("kind").and_then(Value::as_str) {
        Some("action") => names("owns")
            .chain(names("addresses"))
            .any(|name| joel_can.contains(name)),
        Some("question") => {
            let orchestrator = presence
                .get("orchestratedBy")
                .and_then(Value::as_str)
                .unwrap_or("");
            orchestrator.is_empty() || names("addresses").any(|address| address == orchestrator)
        }
        _ => true,
    }
}

/// Whether a want is there at all, as JavaScript's truth reads it.
fn is_set(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(set) => *set,
        Value::String(said) => !said.is_empty(),
        Value::Number(number) => number.as_f64().is_some_and(|n| n != 0.0),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Where a session's first job waiting its turn stands in the machine's
/// queue (Jobs.of), by pi's id for the session.
pub fn in_line(jobs: &[Value], session: &str) -> Option<u64> {
    if session.is_empty() {
        return None;
    }
    jobs.iter()
        .filter(|job| job.pointer("/owner/session").and_then(Value::as_str) == Some(session))
        .find(|job| job.get("state").and_then(Value::as_str) == Some("waiting"))
        .map(|job| job.get("place").and_then(Value::as_u64).unwrap_or(0))
}

/// A place in a line as it's said: 1 as "1st", 12 as "12th", 22 as "22nd".
pub fn ordinal(place: u64) -> String {
    let tens = place % 100;
    let suffix = if (11..=13).contains(&tens) {
        "th"
    } else {
        match place % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{place}{suffix}")
}

/// The sessions listed in Sessions' order: those running, this machine's
/// first, the one whose window was used last first (`used`, by address),
/// then those elsewhere, the one that changed last first; then those
/// parked, as the node holds them. Closed ones and those passed over
/// aren't listed, as Recent reads the closed.
pub fn in_order<'a>(
    sessions: impl IntoIterator<Item = Session<'a>>,
    used: &dyn Fn(&str) -> u64,
) -> Vec<Session<'a>> {
    let (mut running, parked): (Vec<_>, Vec<_>) = sessions
        .into_iter()
        .filter(|session| matches!(session.lives(), Lives::Running | Lives::Parked))
        .partition(|session| session.lives() == Lives::Running);
    running.sort_by(|a, b| {
        b.here().cmp(&a.here()).then_with(|| {
            if a.here() {
                used(b.address()).cmp(&used(a.address()))
            } else {
                b.changed().cmp(&a.changed())
            }
        })
    });
    running.extend(parked);
    running
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn can(names: &[&str]) -> JoelCan {
        JoelCan(names.iter().map(|name| (*name).to_owned()).collect())
    }

    #[test]
    fn a_record_says_its_address_quest_and_orchestrator() {
        let record = json!({"presence": {"addresses": ["P", "Q"], "orchestratedBy": "O",
            "owns": ["8IDU4E", "QEST-20261106-AAAAAA"], "session": "s-1"}});
        let session = Session(&record);
        assert_eq!(session.address(), "P");
        assert_eq!(session.under(), "O");
        assert_eq!(session.quest(), "QEST-20261106-AAAAAA");
        assert_eq!(session.id(), "s-1");
        assert_eq!(
            Session(&json!({})).address(),
            "",
            "no presence names nothing"
        );
    }

    #[test]
    fn a_session_is_named_by_its_quest_and_folder() {
        let record = json!({"presence": {"addresses": ["P"], "cwd": "/src/reef/",
            "questInfo": {"title": "Kelp Water"}, "title": "\u{3c0} - old - x"}});
        assert_eq!(
            Session(&record).named(),
            ("Kelp Water".into(), "reef".into())
        );
        let old =
            json!({"presence": {"addresses": ["P"], "title": "\u{3c0} - Kelp - Water - reef-a"}});
        assert_eq!(
            Session(&old).named(),
            ("Kelp - Water".into(), "reef-a".into()),
            "from its label, split at the last dash"
        );
        let bare = json!({"presence": {"addresses": ["P"]}});
        assert_eq!(
            Session(&bare).named(),
            ("P".into(), String::new()),
            "else its address"
        );
    }

    #[test]
    fn the_node_says_how_each_lives() {
        let lives = |standing: Value| Session(&json!({"standing": standing})).lives();
        assert_eq!(lives(json!("running")), Lives::Running);
        assert_eq!(lives(json!("parked")), Lives::Parked);
        assert_eq!(lives(json!("closed")), Lives::Closed);
        assert_eq!(lives(Value::Null), Lives::PassedOver);
    }

    #[test]
    fn an_action_is_joels_only_while_he_can_do_it_now() {
        let presence = json!({"wants": {"kind": "action"}, "owns": ["QEST-A"], "addresses": ["P"]});
        assert!(
            asks_joel(Some(&presence), &can(&["QEST-A"])),
            "its quest has his item"
        );
        assert!(
            asks_joel(Some(&presence), &can(&["P"])),
            "the session itself has"
        );
        assert!(
            !asks_joel(Some(&presence), &can(&["QEST-B"])),
            "another's isn't his"
        );
    }

    #[test]
    fn a_question_is_joels_only_from_one_none_orchestrates() {
        let asked = |orchestrator: Value| {
            let presence = json!({"wants": {"kind": "question"}, "addresses": ["P"],
                "orchestratedBy": orchestrator});
            asks_joel(Some(&presence), &can(&[]))
        };
        assert!(asked(Value::Null), "an orchestrator's own");
        assert!(asked(json!("P")), "one orchestrating itself");
        assert!(!asked(json!("O")), "put to its orchestrator");
    }

    #[test]
    fn a_yes_or_a_stuck_run_is_always_joels_and_no_want_never() {
        let yes = json!({"wants": {"kind": "approval"}, "orchestratedBy": "O"});
        assert!(asks_joel(Some(&yes), &can(&[])), "a yes is always Joel's");
        assert!(
            !asks_joel(Some(&json!({"wants": null})), &can(&[])),
            "no want asks nothing"
        );
        assert!(!asks_joel(None, &can(&["P"])), "no record asks nothing");
    }

    #[test]
    fn a_session_waits_where_its_first_waiting_job_stands() {
        let jobs = [
            json!({"owner": {"session": "s-1"}, "state": "running", "place": 0}),
            json!({"owner": {"session": "s-1"}, "state": "waiting", "place": 3}),
            json!({"owner": {"session": "s-2"}, "state": "waiting", "place": 1}),
        ];
        assert_eq!(in_line(&jobs, "s-1"), Some(3));
        assert_eq!(in_line(&jobs, "s-3"), None);
        assert_eq!(
            in_line(&jobs, ""),
            None,
            "a session pi hasn't named waits for nothing"
        );
        let record = json!({"presence": {"session": "s-1", "addresses": ["P"]}});
        assert_eq!(Session(&record).stands(&can(&[]), &jobs), Stands::InLine);
    }

    #[test]
    fn places_are_said_as_ordinals() {
        let said: Vec<_> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 111, 112]
            .map(ordinal)
            .into();
        assert_eq!(
            said.join(" "),
            "1st 2nd 3rd 4th 11th 12th 13th 21st 22nd 111th 112th"
        );
    }

    #[test]
    fn sessions_are_listed_here_first_then_by_change_then_parked() {
        let records = [
            json!({"presence": {"addresses": ["A"]}, "standing": "parked"}),
            json!({"presence": {"addresses": ["B"]}, "standing": "running", "here": false,
                "changed": 5}),
            json!({"presence": {"addresses": ["O"]}, "standing": "running", "here": true}),
            json!({"presence": {"addresses": ["P"]}, "standing": "running", "here": false,
                "changed": 9}),
            json!({"presence": {"addresses": ["Q"]}, "standing": "running", "here": true}),
            json!({"presence": {"addresses": ["X"]}, "standing": "closed"}),
        ];
        let used = |address: &str| u64::from(address == "Q");
        let order: Vec<_> = in_order(records.iter().map(Session), &used)
            .iter()
            .map(|session| session.address())
            .collect();
        assert_eq!(order, ["Q", "O", "P", "B", "A"]);
    }
}
