//! Whom a line Joel says from anywhere goes to, and sending it there
//! (1JQPPC's AI everywhere, 2026-11-11): one working on the quest he
//! names, else the session orchestrating that quest; and a line naming
//! none, as every Super+S line is, to the session standing at Agents'
//! top, the void, never the focused window's session. One door means one
//! model (Joel, 2026-11-13 04:33): the void routes a line about a window
//! itself, so the answer names the focused window's session beside it
//! (`in`), which the shell carries to the void as one of its Attached
//! rows. The shell asks before he sends, without words, to say who will
//! hear it, and again with them; the line goes as a talk does, through
//! the node's steer, so it reaches a session on any machine.

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
    /// Whether it stands at Agents' top, as the void does.
    top: bool,
}

struct Quest {
    address: String,
    parent: String,
    orchestrator: String,
}

/// Whom a line goes to, and why: `quest`, `orchestrator` or `top`; or
/// the session in a window, `window`.
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
                    top: one.at_top().is_some(),
                })
                // A line goes to an address, so one whose presence names
                // none can't hear it.
                .filter(|one| !one.addresses.is_empty())
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

    /// Whom a line naming `quest` goes to, and one naming none.
    pub fn heard(&self, quest: &str) -> Option<Heard> {
        let map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if quest.trim().is_empty() {
            map.at_top()
        } else {
            map.on_quest(quest)
        }
    }

    /// The session reached at `address`, running or parked, as Joel's
    /// words on what it landed go to it first (feedback.rs).
    pub fn by_address(&self, address: &str) -> Option<Heard> {
        let map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.by_address(address, "session")
    }

    /// The session in `window`, named by its key or niri's id for it.
    pub fn in_window(&self, window: &Value) -> Option<Heard> {
        let map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let key = match window {
            Value::Number(id) => format!("window:{id}"),
            Value::String(key) if key.starts_with("window:") => key.clone(),
            Value::String(id) if !id.is_empty() => format!("window:{id}"),
            _ => return None,
        };
        map.by_address(map.windows.get(&key)?, "window")
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

    /// The session standing at Agents' top, the void.
    fn at_top(&self) -> Option<Heard> {
        let one = self.sessions.iter().find(|one| one.top)?;
        Some(Heard {
            to: one.addresses[0].clone(),
            name: one.name.clone(),
            why: "top",
        })
    }

    /// A session working on a quest, named by its id or address, or else
    /// the session orchestrating it, or those above.
    fn on_quest(&self, named: &str) -> Option<Heard> {
        let named = named.trim();
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
    let Some(heard) = routes.heard(quest) else {
        return json!({"error": {"code": "unheard",
            "message": "no session is on that quest, nor at Agents' top"}});
    };
    let mut answer = json!({"to": heard.to, "name": heard.name, "why": heard.why});
    if let Some(within) = routes.in_window(&said["window"]) {
        answer["in"] = json!({"to": within.to, "name": within.name});
    }
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

    fn the_void() -> Value {
        json!({"standing": "running", "presence": {"addresses": ["VOID"],
            "standsAtTop": {"name": "The Void"}}})
    }

    fn heard(to: &str, why: &'static str) -> Heard {
        Heard {
            to: to.to_owned(),
            name: format!("{to} Quest"),
            why,
        }
    }

    #[test]
    fn a_line_naming_no_quest_goes_to_the_void_never_the_focused_windows_session() {
        let unvoided = routes(
            &[
                session(&["SKIFF"], "QEST-1-SKIFF", "running"),
                json!({"standing": "running", "presence": {"addresses": [],
                    "standsAtTop": {"name": "Nameless"}}}),
            ],
            &[],
            &[],
        );
        let routes = routes(
            &[
                session(&["SKIFF"], "QEST-1-SKIFF", "running"),
                the_void(),
                session(&["BUOY"], "QEST-1-BUOY", "parked"),
            ],
            &[],
            &[json!({"kind": "agent-window", "address": "SKIFF", "window": "window:17"})],
        );
        let void = Some(Heard {
            to: "VOID".to_owned(),
            name: "The Void".to_owned(),
            why: "top",
        });
        assert_eq!(
            answer(&json!({"ask": "tell", "window": 17}), &routes, None, 7),
            json!({"to": "VOID", "name": "The Void", "why": "top",
                "in": {"to": "SKIFF", "name": "SKIFF Quest"}}),
            "the void hears it, never the focused window's session, which it names as `in`"
        );
        assert_eq!(
            answer(
                &json!({"ask": "tell", "window": "window:18"}),
                &routes,
                None,
                7
            ),
            json!({"to": "VOID", "name": "The Void", "why": "top"}),
            "a window that's no session's names none"
        );
        assert_eq!(routes.heard(""), void, "nothing named, the void");
        assert_eq!(
            routes.heard("buoy"),
            Some(heard("BUOY", "quest")),
            "a quest named by its address goes to the session on it"
        );
        assert_eq!(
            unvoided.heard(""),
            None,
            "with no void to hear it, nobody, never a panic on one with no address"
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
            routes.heard("QEST-1-KELP"),
            Some(Heard {
                to: "P9R1KI".to_owned(),
                name: "P9R1KI Quest".to_owned(),
                why: "orchestrator"
            }),
            "past an orchestrator not running, the one above it, by any address it goes by"
        );
        assert_eq!(
            routes.heard("QEST-1-ANCHOR"),
            None,
            "a closed session hears nothing"
        );
        assert_eq!(routes.heard(""), None, "nothing named and no void, nobody");
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
