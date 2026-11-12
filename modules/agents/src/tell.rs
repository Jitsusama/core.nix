//! Whom a line Joel says from anywhere goes to, and sending it there
//! (1JQPPC's AI everywhere, 2026-11-11): the session in the window he's
//! focused on, else one working on the quest he names, else the session
//! orchestrating that quest, else nobody, where the void will take it.
//! The shell asks before he sends, without words, to say who will hear
//! it, and again with them; the line goes as a talk does, through the
//! node's steer, so it reaches a session on any machine.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use crate::acts;
use crate::session::{Lives, Session};
use crate::windows;

/// How far up its quest's parents a line looks for one to hear it, past
/// which a quest's family is taken to loop.
const DEPTH: usize = 8;

/// Who may hear a line, as the sources last said: held between changes,
/// so an ask is answered without waiting on the loop that makes rows.
#[derive(Clone, Default)]
pub struct Routes(Arc<Mutex<Map>>);

#[derive(Default)]
struct Map {
    /// The session each window is, by the windows daemon's key.
    windows: HashMap<String, String>,
    /// Every session running or parked on a machine that can be reached.
    sessions: Vec<Hears>,
    /// Every quest, by its id.
    quests: HashMap<String, Quest>,
}

struct Hears {
    addresses: Vec<String>,
    quest: String,
    name: String,
}

struct Quest {
    address: String,
    parent: String,
    orchestrator: String,
}

/// Whom a line goes to, and why: `window`, `quest` or `orchestrator`.
#[derive(Debug, PartialEq, Eq)]
pub struct Heard {
    pub to: String,
    pub name: String,
    pub why: &'static str,
}

impl Routes {
    /// Holds who may hear a line now: the sessions, the node's quests and
    /// the agent-window records served.
    pub fn hold<'a>(
        &self,
        sessions: impl Iterator<Item = Session<'a>>,
        quests: impl Iterator<Item = &'a Value>,
        served: impl Iterator<Item = &'a Value>,
    ) {
        let text = |value: &Value| value.as_str().unwrap_or("").to_owned();
        let map = Map {
            windows: served
                .filter(|record| record["kind"] == windows::KIND)
                .filter_map(|record| {
                    Some((
                        text(&record["window"]),
                        record["address"].as_str()?.to_owned(),
                    ))
                })
                .filter(|(window, _)| !window.is_empty())
                .collect(),
            sessions: sessions
                .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
                .filter(|one| one.reachable() && !one.address().is_empty())
                .map(|one| Hears {
                    addresses: one
                        .presence()
                        .and_then(|presence| presence["addresses"].as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_uppercase)
                        .collect(),
                    quest: one
                        .presence()
                        .and_then(|presence| presence["questInfo"]["id"].as_str())
                        .unwrap_or_else(|| one.quest())
                        .to_owned(),
                    name: one.named().0,
                })
                .collect(),
            quests: quests
                .filter_map(|quest| {
                    let id = quest["id"].as_str()?.to_owned();
                    let quest = Quest {
                        address: text(&quest["address"]).to_uppercase(),
                        parent: text(&quest["parent"]),
                        orchestrator: text(&quest["orchestrator"]).to_uppercase(),
                    };
                    Some((id, quest))
                })
                .collect(),
        };
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = map;
    }

    /// Whom a line said with `window` focused, naming `quest`, goes to.
    pub fn heard(&self, window: &Value, quest: &str) -> Option<Heard> {
        let map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.in_window(window).or_else(|| map.on_quest(quest))
    }
}

impl Map {
    fn by_address(&self, address: &str, why: &'static str) -> Option<Heard> {
        let address = address.to_uppercase();
        let one = self
            .sessions
            .iter()
            .find(|one| one.addresses.contains(&address))?;
        Some(Heard {
            to: one.addresses[0].clone(),
            name: one.name.clone(),
            why,
        })
    }

    /// The session in a window, named by its key or niri's id for it.
    fn in_window(&self, window: &Value) -> Option<Heard> {
        let key = match window {
            Value::Number(id) => format!("window:{id}"),
            Value::String(key) if key.starts_with("window:") => key.clone(),
            Value::String(id) if !id.is_empty() => format!("window:{id}"),
            _ => return None,
        };
        self.by_address(self.windows.get(&key)?, "window")
    }

    /// A session working on a quest, named by its id or address, or else
    /// the session orchestrating it, or those above.
    fn on_quest(&self, named: &str) -> Option<Heard> {
        let named = named.trim();
        if named.is_empty() {
            return None;
        }
        let mut id = self
            .quests
            .iter()
            .find(|(id, quest)| {
                id.eq_ignore_ascii_case(named) || quest.address == named.to_uppercase()
            })
            .map_or_else(|| named.to_owned(), |(id, _)| id.clone());
        // A quest the node doesn't list is named by its address, which is
        // the address of the session working on it.
        let working = |id: &str| {
            let address = self
                .quests
                .get(id)
                .map_or_else(|| named.to_uppercase(), |quest| quest.address.clone());
            let one = self
                .sessions
                .iter()
                .find(|one| one.quest == id || one.addresses.contains(&address))?;
            Some(Heard {
                to: one.addresses[0].clone(),
                name: one.name.clone(),
                why: "quest",
            })
        };
        if let Some(heard) = working(&id) {
            return Some(heard);
        }
        for _ in 0..DEPTH {
            let quest = self.quests.get(&id)?;
            if let Some(heard) = self.by_address(&quest.orchestrator, "orchestrator") {
                return Some(heard);
            }
            id.clone_from(&quest.parent);
        }
        None
    }
}

/// The answer to a `tell`: whom its line goes to, and, given words, what
/// the node said of sending them; or an error saying nobody hears it.
pub fn answer(said: &Value, routes: &Routes, node: Option<&Path>, now_ms: u128) -> Value {
    let quest = said["quest"].as_str().unwrap_or("");
    let Some(heard) = routes.heard(&said["window"], quest) else {
        return json!({"error": {"code": "unheard",
            "message": "no session is in that window or on that quest"}});
    };
    let mut answer = json!({"to": heard.to, "name": heard.name, "why": heard.why});
    let words = said["words"].as_str().unwrap_or("");
    if words.trim().is_empty() {
        return answer;
    }
    let talk = json!({"ask": "talk", "address": heard.to, "text": words});
    let sent = match (acts::steer_of(&talk, now_ms), node) {
        (Some(steer), Some(node)) => acts::done(steer, node),
        _ => json!({"error": {"code": "unsteered", "message": "no node to send it through"}}),
    };
    answer["sent"] = sent;
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(addresses: &[&str], quest: &str, standing: &str) -> Value {
        json!({"standing": standing, "presence": {"addresses": addresses, "owns": [quest],
            "questInfo": {"title": format!("{} Quest", addresses[0])}}})
    }

    fn routes(sessions: &[Value], quests: &[Value], served: &[Value]) -> Routes {
        let routes = Routes::default();
        routes.hold(sessions.iter().map(Session), quests.iter(), served.iter());
        routes
    }

    fn heard(to: &str, why: &'static str) -> Heard {
        Heard {
            to: to.to_owned(),
            name: format!("{to} Quest"),
            why,
        }
    }

    #[test]
    fn the_focused_windows_session_hears_it_first() {
        let routes = routes(
            &[
                session(&["SKIFF"], "QEST-1-SKIFF", "running"),
                session(&["BUOY"], "QEST-1-BUOY", "parked"),
            ],
            &[],
            &[json!({"kind": "agent-window", "address": "SKIFF", "window": "window:17"})],
        );
        assert_eq!(
            routes.heard(&json!(17), "QEST-1-BUOY"),
            Some(heard("SKIFF", "window")),
            "the window wins over the quest named"
        );
        assert_eq!(
            routes.heard(&json!("window:17"), ""),
            Some(heard("SKIFF", "window")),
            "a window is named by its key too"
        );
        assert_eq!(
            routes.heard(&json!(18), "buoy"),
            Some(heard("BUOY", "quest")),
            "a window that's no session's leaves it to the quest, named by its address"
        );
    }

    #[test]
    fn a_quest_with_nobody_on_it_goes_to_whoever_orchestrates_it() {
        let quests = [
            json!({"id": "QEST-1-REEF", "address": "REEF", "parent": "QEST-1-HARBOUR",
                "orchestrator": "QUAY"}),
            json!({"id": "QEST-1-KELP", "address": "KELP", "parent": "QEST-1-REEF",
                "orchestrator": "REEF"}),
            json!({"id": "QEST-1-HARBOUR", "address": "HARBOUR", "orchestrator": "HARBOUR"}),
        ];
        let sessions = [
            session(&["P9R1KI", "HARBOUR"], "QEST-1-HARBOUR", "running"),
            session(&["ANCHOR"], "QEST-1-ANCHOR", "closed"),
        ];
        let routes = routes(&sessions, &quests, &[]);
        assert_eq!(
            routes.heard(&Value::Null, "QEST-1-KELP"),
            Some(Heard {
                to: "P9R1KI".to_owned(),
                name: "P9R1KI Quest".to_owned(),
                why: "orchestrator"
            }),
            "past an orchestrator not running, the one above it, by any address it goes by"
        );
        assert_eq!(
            routes.heard(&Value::Null, "QEST-1-ANCHOR"),
            None,
            "a closed session hears nothing"
        );
        assert_eq!(
            routes.heard(&Value::Null, ""),
            None,
            "nothing named, nobody"
        );
    }

    #[test]
    fn a_tell_says_who_hears_it_and_sends_only_words() {
        let routes = routes(&[session(&["SKIFF"], "QEST-1-SKIFF", "running")], &[], &[]);
        assert_eq!(
            answer(
                &json!({"ask": "tell", "quest": "QEST-1-SKIFF"}),
                &routes,
                None,
                7
            ),
            json!({"to": "SKIFF", "name": "SKIFF Quest", "why": "quest"}),
            "without words it only says who would hear them"
        );
        assert_eq!(
            answer(
                &json!({"ask": "tell", "quest": "QEST-1-SKIFF", "words": "how goes it?"}),
                &routes,
                None,
                7
            )["sent"]["error"]["code"],
            "unsteered",
            "with no node, words aren't sent"
        );
        assert_eq!(
            answer(
                &json!({"ask": "tell", "quest": "QEST-1-BUOY", "words": "hi"}),
                &routes,
                None,
                7
            )["error"]["code"],
            "unheard",
            "nobody to hear it says so"
        );
    }
}
