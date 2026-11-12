//! Which window each session here is in, served as an `agent-window`
//! record a session, so Windows folds agents' windows into one row and
//! Programs counts them apart from Joel's own, neither guessing which
//! are agents' (VKCF3Z, 2026-11-11). Each says the session's kitty pane,
//! by its kitty's socket and the pane's id, and its window by the windows
//! daemon's key, found as Sessions.qml finds it: the window opened with
//! the session's app id, or of several, the one its own kitty draws; and
//! for a window opened without one, the one window its kitty draws. One
//! that can't be told apart says no window rather than the wrong one;
//! only Sessions.qml asks kitty which of several a session is in.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use daemon_serve::watch::Held;
use serde_json::{Value, json};

use crate::served::PLACE;
use crate::session::{Lives, Session};

/// The kind each record is served as.
pub const KIND: &str = "agent-window";

/// The kitty pane a session runs in, `(socket, id)`: as the node says a
/// held one's window is, else as its record's terminal says.
fn kitty(one: Session) -> Option<(String, String)> {
    let window = &one.0["window"];
    if window["terminal"] == "kitty" {
        let within = window["within"]
            .as_str()
            .filter(|within| !within.is_empty());
        if let (Some(within), Some(id)) = (within, pane_id(&window["id"])) {
            return Some((within.to_owned(), id));
        }
    }
    let terminal = &one.presence()?["terminal"];
    let scope = terminal["scope"]
        .as_str()
        .filter(|scope| !scope.is_empty())?;
    (terminal["driverId"] == "kitty")
        .then(|| pane_id(&terminal["value"]))
        .flatten()
        .map(|id| (scope.to_owned(), id))
}

/// A kitty pane's id, given as text or a number, when it names one.
fn pane_id(id: &Value) -> Option<String> {
    let id = match id {
        Value::String(id) => id.parse::<u64>().ok()?,
        id => id.as_u64()?,
    };
    (id > 0).then(|| id.to_string())
}

/// The process of the kitty a socket, `unix:@kitty-<pid>`, belongs to.
fn kitty_pid(socket: &str) -> Option<u64> {
    socket.strip_prefix("unix:@kitty-")?.parse().ok()
}

/// The windows daemon's windows, oldest first.
fn listed(windows: &Held) -> Vec<Window<'_>> {
    let mut listed: Vec<Window> = windows
        .records()
        .filter_map(|(key, window)| {
            Some(Window {
                key,
                id: window["id"].as_u64()?,
                pid: window["pid"].as_u64(),
                opened: window["opened"].as_str().unwrap_or(""),
            })
        })
        .collect();
    listed.sort_by_key(|window| window.id);
    listed
}

struct Window<'a> {
    key: &'a str,
    id: u64,
    pid: Option<u64>,
    opened: &'a str,
}

/// The one of `among` a session's window is: the only one, or of several,
/// the only one its kitty draws.
fn one_of<'a>(among: &[&Window<'a>], pid: Option<u64>) -> Option<&'a str> {
    if let [one] = among {
        return Some(one.key);
    }
    let drawn: Vec<_> = among
        .iter()
        .filter(|window| pid.is_some() && window.pid == pid)
        .collect();
    if let [one] = drawn.as_slice() {
        Some(one.key)
    } else {
        None
    }
}

/// The orchestration a session is under, `(address, title)`: the one
/// orchestrating it, or itself where it orchestrates others.
fn over<'a, S: BuildHasher>(
    one: Session<'a>,
    titles: &HashMap<&'a str, String>,
    orchestrating: &HashSet<&str, S>,
) -> Option<(&'a str, String)> {
    let address = if one.under().is_empty() {
        Some(one.address()).filter(|address| orchestrating.contains(address))?
    } else {
        one.under()
    };
    let title = titles
        .get(address)
        .cloned()
        .unwrap_or_else(|| address.to_owned());
    Some((address, title))
}

/// Every session here's `agent-window` record, running or parked, one a
/// session with a kitty pane or a window to say. `orchestrating` holds
/// the addresses laid out as orchestrating others.
pub fn records<S: BuildHasher>(
    sessions: &[Session<'_>],
    windows: &Held,
    orchestrating: &HashSet<&str, S>,
) -> Vec<Value> {
    let windows = listed(windows);
    let titles: HashMap<&str, String> = sessions
        .iter()
        .map(|one| (one.address(), one.named().0))
        .collect();
    sessions
        .iter()
        .filter(|one| one.here() && matches!(one.lives(), Lives::Running | Lives::Parked))
        .filter(|one| !one.address().is_empty())
        .filter_map(|one| {
            let kitty = kitty(*one);
            let pid = kitty.as_ref().and_then(|(socket, _)| kitty_pid(socket));
            let opened = one.opened();
            let among: Vec<&Window> = windows
                .iter()
                .filter(|window| {
                    if opened.is_empty() {
                        pid.is_some() && window.pid == pid
                    } else {
                        window.opened == opened
                    }
                })
                .collect();
            let window = one_of(&among, pid);
            if kitty.is_none() && window.is_none() {
                return None;
            }
            let address = one.address();
            let orchestration = over(*one, &titles, orchestrating).map_or(
                Value::Null,
                |(address, title)| json!({"address": address, "title": title}),
            );
            Some(json!({
                "key": format!("{KIND}:{PLACE}:{address}"), "kind": KIND,
                "address": address,
                "kitty": kitty.map_or(Value::Null, |(within, id)| {
                    json!({"id": id, "within": within})
                }),
                "app_id": opened, "window": window, "title": titles[address],
                "orchestration": orchestration,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(address: &str, presence: &Value, window: &Value) -> Value {
        let mut presence = presence.clone();
        presence["addresses"] = json!([address]);
        json!({"key": format!("session:{address}"), "standing": "running", "here": true,
            "presence": presence, "window": window})
    }

    fn window(id: u64, pid: u64, opened: &str) -> (String, Value) {
        (
            format!("window:{id}"),
            json!({"id": id, "pid": pid, "opened": opened}),
        )
    }

    fn made(sessions: &[Value], windows: &[(String, Value)]) -> Vec<Value> {
        let sessions: Vec<Session> = sessions.iter().map(Session).collect();
        let mut held = Held::default();
        let records: Vec<_> = windows
            .iter()
            .map(|(key, one)| {
                let mut one = one.clone();
                one["key"] = json!(key);
                one
            })
            .collect();
        held.took(&json!({"records": records}));
        super::records(&sessions, &held, &HashSet::from(["KELP"]))
    }

    fn windows_of(made: &[Value]) -> Vec<(String, Value)> {
        made.iter()
            .map(|one| {
                (
                    one["address"].as_str().unwrap_or("").to_owned(),
                    one["window"].clone(),
                )
            })
            .collect()
    }

    #[test]
    fn a_held_session_is_in_the_window_opened_with_its_app_id() {
        let kitty = json!({"terminal": "kitty", "within": "unix:@kitty-7", "id": 3,
            "app_id": "pi-spawned.1"});
        let made = made(
            &[held(
                "SKIFF",
                &json!({"questInfo": {"title": "Skiff"},
                "orchestratedBy": "KELP"}),
                &kitty,
            )],
            &[window(2, 9, "pi-spawned.1"), window(4, 7, "pi-spawned.1")],
        );
        assert_eq!(
            made,
            vec![
                json!({"key": "agent-window:agents:SKIFF", "kind": "agent-window",
                "address": "SKIFF", "kitty": {"id": "3", "within": "unix:@kitty-7"},
                "app_id": "pi-spawned.1", "window": "window:4", "title": "Skiff",
                "orchestration": {"address": "KELP", "title": "KELP"}})
            ],
            "of two windows opened with its app id, its own kitty's"
        );
    }

    #[test]
    fn a_kitty_on_a_socket_of_its_own_is_known_by_the_pid_written_beside_it() {
        let folder = std::env::temp_dir().join(format!("agents-kitty-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket");
        std::fs::write(folder.join("kitty.pid"), "7\n").expect("the pid written");
        let socket = format!("unix:{}", folder.join("kitty.sock").display());
        let kitty = json!({"terminal": "kitty", "within": socket, "id": 3});
        let made = made(
            &[held("SKIFF", &json!({}), &kitty)],
            &[window(2, 9, ""), window(4, 7, "")],
        );
        std::fs::remove_dir_all(&folder).expect("the folder gone");
        assert_eq!(
            windows_of(&made),
            vec![("SKIFF".to_owned(), json!("window:4"))],
            "the sessions kitty's pid is read from beside its socket (home.nix's pi-sessions)"
        );
    }

    #[test]
    fn one_that_cant_be_told_apart_says_no_window() {
        let kitty = json!({"terminal": "kitty", "within": "unix:@kitty-7", "id": 3});
        let made = made(
            &[
                held("SKIFF", &json!({}), &kitty),
                held(
                    "REEF",
                    &json!({"hosted": true, "session": "s-reef"}),
                    &json!(null),
                ),
            ],
            &[
                window(2, 7, ""),
                window(4, 7, ""),
                window(6, 5, "pi-hosted.s-reef"),
            ],
        );
        assert_eq!(
            windows_of(&made),
            vec![
                ("SKIFF".to_owned(), json!(null)),
                ("REEF".to_owned(), json!("window:6")),
            ],
            "a kitty drawing two windows says neither; a hosted one is its viewer"
        );
        assert_eq!(
            made[1]["kitty"],
            json!(null),
            "a hosted session runs in no kitty"
        );
    }

    #[test]
    fn an_orchestrator_leads_its_own_orchestration_and_the_rest_are_under_none() {
        let terminal = |pid: u64| {
            let scope = format!("unix:@kitty-{pid}");
            json!({"driverId": "kitty", "scope": scope, "value": "1"})
        };
        let made = made(
            &[
                held(
                    "KELP",
                    &json!({"questInfo": {"title": "Kelp"},
                    "terminal": terminal(5)}),
                    &json!(null),
                ),
                held("BUOY", &json!({"terminal": terminal(6)}), &json!(null)),
            ],
            &[window(1, 5, ""), window(3, 6, "")],
        );
        assert_eq!(
            made.iter()
                .map(|one| (one["window"].clone(), one["orchestration"].clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    json!("window:1"),
                    json!({"address": "KELP", "title": "Kelp"})
                ),
                (json!("window:3"), json!(null)),
            ],
            "the one window each kitty draws, and only the orchestrator under itself"
        );
    }

    #[test]
    fn a_session_elsewhere_or_closed_has_no_record() {
        let kitty = json!({"terminal": "kitty", "within": "unix:@kitty-7", "id": 3});
        let mut far = held("FAR", &json!({}), &kitty);
        far["here"] = json!(false);
        let mut closed = held("QUAY", &json!({}), &kitty);
        closed["standing"] = json!("closed");
        let alone = held("HERMIT", &json!({}), &json!(null));
        assert!(
            made(&[far, closed, alone], &[window(1, 7, "")]).is_empty(),
            "only a session here, running or parked, with a pane or a window"
        );
    }
}
