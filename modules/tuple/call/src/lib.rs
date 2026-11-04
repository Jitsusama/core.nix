//! tuple-call's own logic, apart from its threads, sockets and processes
//! so its tests reach it: what a client asks (ask.rs), what a line of
//! Tuple's stream says (stream.rs), what its log says of a ring (log.rs),
//! and the one record they make, the call Joel's in ([`Held`]).

pub mod ask;
pub mod log;
pub mod stream;

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::log::{Log, Missed};
use crate::stream::{Contact, Live, Person};

/// How many missed calls are kept, the latest.
pub const MISSED: usize = 20;

/// Everything the record is made from.
#[derive(Debug)]
pub struct Held {
    /// The call Joel's in, as Tuple's stream last said it.
    pub live: Option<Live>,
    /// Everyone Tuple knows, by id, to name a caller by.
    pub contacts: HashMap<String, Contact>,
    pub log: Log,
    /// The calls missed since this started, the latest last.
    pub missed: Vec<Missed>,
    /// Joel's room's link, once `tuple new` has said it.
    pub room: Option<String>,
    /// The newest of his own rooms, as Tuple's state lists them, until
    /// `tuple new` says which it opens.
    pub personal: Option<String>,
}

impl Held {
    pub fn new(began: i64, room: Option<String>) -> Held {
        Held {
            live: None,
            contacts: HashMap::new(),
            log: Log::new(began),
            missed: Vec::new(),
            room,
            personal: None,
        }
    }

    /// Keeps a missed call, letting the oldest go past [`MISSED`].
    pub fn miss(&mut self, missed: Option<Missed>) {
        let Some(missed) = missed else { return };
        self.missed.push(missed);
        let over = self.missed.len().saturating_sub(MISSED);
        self.missed.drain(..over);
    }

    /// Someone by what fills Tuple's blank: the id it names, and the name
    /// and address Tuple's contacts give that id.
    fn caller(&self, who: &str) -> Value {
        let id = log::number(who);
        let contact = self.contacts.get(id).cloned().unwrap_or_default();
        json!({"who": who, "id": id, "name": contact.name, "email": contact.email})
    }

    /// Someone in the call, named from the contacts when Tuple's call
    /// doesn't say who they are.
    fn person(&self, person: &Person) -> Value {
        let known = self.contacts.get(&person.id);
        let or = |said: &str, known: Option<&str>| {
            if said.is_empty() {
                known.unwrap_or("").to_owned()
            } else {
                said.to_owned()
            }
        };
        json!({
            "id": person.id,
            "name": or(&person.name, known.map(|contact| contact.name.as_str())),
            "email": or(&person.email, known.map(|contact| contact.email.as_str())),
            "connection": person.connection,
            "muted": person.muted,
            "sharing": person.sharing,
            "camera": person.camera,
        })
    }

    /// The record, less when it was heard: how the call stands (none,
    /// ringing, connecting or connected), since when it connected, Joel's
    /// own mute, screen and camera, everyone else in it, who's calling,
    /// the calls missed, and Joel's room. Joel's mute is said only once
    /// connected, since Tuple sends no voice before.
    pub fn record(&self) -> Value {
        let ringing = self.log.ringing();
        let state = match (&self.live, ringing) {
            (Some(live), _) if live.joel.connection == "connected" => "connected",
            (Some(_), _) => "connecting",
            (None, Some(_)) => "ringing",
            (None, None) => "none",
        };
        let joel = self.live.as_ref().map(|live| &live.joel);
        json!({
            "key": ask::KEY,
            "kind": ask::KIND,
            "state": state,
            "since": self.live.as_ref().and_then(|live| live.since),
            "muted": joel.filter(|joel| joel.connection == "connected").map(|joel| joel.muted),
            "sharing": joel.is_some_and(|joel| joel.sharing),
            "camera": joel.is_some_and(|joel| joel.camera),
            "people": self.live.iter().flat_map(|live| &live.people)
                .map(|person| self.person(person)).collect::<Vec<_>>(),
            "ringing": ringing.map(|ring| {
                let mut caller = self.caller(&ring.who);
                caller["since"] = json!(ring.since);
                caller
            }),
            "missed": self.missed.iter().map(|missed| {
                let mut caller = self.caller(&missed.who);
                caller["at"] = json!(missed.at);
                caller
            }).collect::<Vec<_>>(),
            "room": self.room.as_ref().or(self.personal.as_ref()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{Line, read};

    const NOON: i64 = 1_793_707_200_000;

    fn held() -> Held {
        let mut held = Held::new(NOON - 60_000, Some("https://tuple.app/c/room".into()));
        held.contacts.insert(
            "7".into(),
            Contact {
                name: "Ada Brightwater".into(),
                email: "ada@example.com".into(),
            },
        );
        held
    }

    #[test]
    fn no_call_is_a_record_all_the_same() {
        assert_eq!(
            held().record(),
            json!({
                "key": "call:current", "kind": "call", "state": "none", "since": null,
                "muted": null, "sharing": false, "camera": false, "people": [],
                "ringing": null, "missed": [], "room": "https://tuple.app/c/room",
            })
        );
    }

    #[test]
    fn a_call_says_everyone_in_it() {
        let mut held = held();
        let Line::Call(live) = read(&stream::tests::call(
            "connected",
            true,
            [true, false, true],
            [false, true, false],
        )) else {
            panic!("no call read");
        };
        held.live = live;
        let record = held.record();
        assert_eq!(record["state"], "connected");
        assert_eq!(record["since"], 1_792_759_701_457_i64);
        assert_eq!(record["muted"], true);
        assert_eq!(record["camera"], true);
        assert_eq!(
            record["people"],
            json!([{
                "id": "7", "name": "Ada Brightwater", "email": "ada@example.com",
                "connection": "connected", "muted": false, "sharing": true, "camera": false,
            }])
        );
    }

    #[test]
    fn a_call_connecting_says_no_mute() {
        let mut held = held();
        let Line::Call(live) = read(&stream::tests::call(
            "signaling",
            false,
            [true, false, false],
            [false; 3],
        )) else {
            panic!("no call read");
        };
        held.live = live;
        let record = held.record();
        assert_eq!(record["state"], "connecting");
        assert_eq!(record["muted"], Value::Null);
    }

    #[test]
    fn someone_in_a_call_unnamed_is_named_from_the_contacts() {
        let mut held = held();
        held.live = Some(Live {
            since: None,
            joel: Person::default(),
            people: vec![Person {
                id: "7".into(),
                ..Person::default()
            }],
        });
        assert_eq!(held.record()["people"][0]["name"], "Ada Brightwater");
    }

    #[test]
    fn a_ring_and_a_missed_call_are_named_by_their_caller() {
        let mut held = held();
        held.log
            .heard("11:59:50.000|calls|received incoming call: 7", NOON);
        let record = held.record();
        assert_eq!(record["state"], "ringing");
        assert_eq!(
            record["ringing"],
            json!({"who": "7", "id": "7", "name": "Ada Brightwater",
                   "email": "ada@example.com", "since": NOON - 10_000})
        );
        let missed = held.log.heard("11:59:55.000|calls|call ended", NOON).missed;
        held.miss(missed);
        let record = held.record();
        assert_eq!(record["state"], "none");
        assert_eq!(record["ringing"], Value::Null);
        assert_eq!(
            record["missed"],
            json!([{"who": "7", "id": "7", "name": "Ada Brightwater",
                    "email": "ada@example.com", "at": NOON - 5_000}])
        );
    }

    #[test]
    fn the_room_opened_is_said_ahead_of_the_newest_listed() {
        let mut held = Held::new(NOON, None);
        held.personal = Some("https://tuple.app/c/listed".into());
        assert_eq!(held.record()["room"], "https://tuple.app/c/listed");
        held.room = Some("https://tuple.app/c/opened".into());
        assert_eq!(held.record()["room"], "https://tuple.app/c/opened");
    }

    #[test]
    fn only_the_latest_missed_calls_are_kept() {
        let mut held = held();
        for at in 0..30 {
            held.miss(Some(Missed {
                who: "someone".into(),
                at,
            }));
        }
        assert_eq!(held.missed.len(), MISSED);
        assert_eq!(held.missed[0].at, 10);
    }
}
