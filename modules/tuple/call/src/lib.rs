//! tuple-call's own logic, apart from its threads, sockets and processes
//! so its tests reach it: what a client asks (ask.rs), what a line of
//! Tuple's stream says (stream.rs), what its log says of a ring (log.rs),
//! the calls had lately (recent.rs), and the one record they make, the
//! call Joel's in ([`Held`]).

pub mod ask;
pub mod log;
pub mod recent;
pub mod stream;

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::log::{Answered, Log, Missed};
use crate::recent::{Past, Recent, Someone};
use crate::stream::{Contact, Live, Person};

/// How many missed calls are kept, the latest.
pub const MISSED: usize = 20;

/// How near to a call first being said what began it must be, in
/// milliseconds, a ring answered or Joel's room opened, for the call to be
/// the one it began: a ring's minute, since Tuple says the call as soon as
/// either happens, and anything further apart may be another call.
pub const BEGINS_WITHIN: i64 = log::RINGS_FOR;

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
    /// The calls ended and missed lately, newest first, kept in a file
    /// so they outlive this (recent.rs).
    pub recent: Recent,
    /// The call under way as Recent will keep it once it ends.
    under_way: Option<UnderWay>,
    /// What began a call no call under way has taken yet, as
    /// [`Held::began`] heard it: its caller and when.
    beginning: Option<(String, i64)>,
}

/// A call under way, as it will be kept: Tuple's stream says only who's
/// in it now, and everyone else may have left before Joel does, so
/// whoever was in it at any point is gathered as it goes.
#[derive(Debug)]
struct UnderWay {
    /// When Tuple's stream first said it, to tell what began it.
    first: i64,
    /// When it connected, once Tuple says.
    since: Option<i64>,
    /// Everyone else who's been in it, in the order they came.
    people: Vec<Someone>,
    /// Who rang whom, as Recent keeps it ([`Past::caller`]).
    caller: Option<String>,
}

/// Adds someone to those a call has had, or fills in what was blank of
/// them, as Tuple names someone only once it knows them.
fn gather(people: &mut Vec<Someone>, someone: Someone) {
    let same = |kept: &Someone| {
        if someone.id.is_empty() {
            !someone.email.is_empty() && kept.email == someone.email
        } else {
            kept.id == someone.id
        }
    };
    match people.iter_mut().find(|kept| same(kept)) {
        Some(kept) => {
            if kept.name.is_empty() {
                kept.name = someone.name;
            }
            if kept.email.is_empty() {
                kept.email = someone.email;
            }
        }
        None => people.push(someone),
    }
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
            recent: Recent::default(),
            under_way: None,
            beginning: None,
        }
    }

    /// Takes a ring Joel answered as what began the call that follows it,
    /// its caller by the number Tuple knows them by. One the log doesn't
    /// number can't be told from Joel, nor found in the call, so it says
    /// nothing.
    pub fn answer(&mut self, answered: Option<Answered>) {
        let Some(answered) = answered else { return };
        let id = log::number(&answered.who);
        if !id.is_empty() {
            self.began(id.to_owned(), answered.at);
        }
    }

    /// Takes what began a call at `at`: "" for Joel opening his room, which
    /// starts a call in it, or the id of someone whose ring he answered.
    /// Tuple's stream may say the call before tuple-call hears either, or
    /// after, so it goes to the call under way if Tuple first said it
    /// near then, or else waits for the next one.
    ///
    /// This is all tuple-call sees of who rang whom: Tuple's stream says
    /// only who's in a call, and Joel calling someone runs Tuple's command
    /// through niri (TuplePeople.qml), which tuple-call never hears, so a
    /// call it saw neither begin is kept with no caller rather than
    /// guessed at.
    pub fn began(&mut self, caller: String, at: i64) {
        if let Some(way) = &mut self.under_way
            && way.caller.is_none()
            && (at - way.first).abs() <= BEGINS_WITHIN
        {
            way.caller = Some(caller);
            return;
        }
        self.beginning = Some((caller, at));
    }

    /// Keeps a missed call, letting the oldest go past [`MISSED`], and
    /// keeps it in Recent too, by its caller.
    pub fn miss(&mut self, missed: Option<Missed>) {
        let Some(missed) = missed else { return };
        let caller = self.someone(log::number(&missed.who), "", "");
        self.recent.keep(Past {
            people: if caller.is_nobody() {
                Vec::new()
            } else {
                vec![caller]
            },
            since: missed.since,
            ended: missed.at,
            missed: true,
            caller: None,
        });
        self.missed.push(missed);
        let over = self.missed.len().saturating_sub(MISSED);
        self.missed.drain(..over);
    }

    /// Takes the call as Tuple's stream says it now, `now` being when it
    /// did, keeping one that's over in Recent, with who began it where
    /// [`Held::began`] heard. Only a call that connected and had someone
    /// else in it is kept: one that never connected has no length, and one
    /// Joel had alone has nobody to call again.
    pub fn follow(&mut self, call: Option<Live>, now: i64) {
        match &call {
            Some(live) => {
                let mut way = self.under_way.take().unwrap_or_else(|| {
                    let caller = self
                        .beginning
                        .take()
                        .filter(|(_, at)| (now - at).abs() <= BEGINS_WITHIN)
                        .map(|(caller, _)| caller);
                    UnderWay {
                        first: now,
                        since: None,
                        people: Vec::new(),
                        caller,
                    }
                });
                if way.since.is_none()
                    && let Some(since) = live.since
                {
                    way.since = Some(since);
                    if let Some(past) = self.recent.reopen(since) {
                        let came = std::mem::replace(&mut way.people, past.people);
                        for someone in came {
                            gather(&mut way.people, someone);
                        }
                        way.caller = way.caller.or(past.caller);
                    }
                }
                for person in &live.people {
                    let someone = self.someone(&person.id, &person.name, &person.email);
                    gather(&mut way.people, someone);
                }
                self.under_way = Some(way);
            }
            None => {
                if let Some(UnderWay {
                    since: Some(since),
                    people,
                    caller,
                    ..
                }) = self.under_way.take()
                    && !people.is_empty()
                {
                    // A ring answered began the call only if its caller
                    // was in it; otherwise it was some other call's.
                    let had = |id: &String| people.iter().any(|one| one.id == *id);
                    let caller = caller.filter(|id| id.is_empty() || had(id));
                    self.recent.keep(Past {
                        people,
                        since,
                        ended: now.max(since),
                        missed: false,
                        caller,
                    });
                }
            }
        }
        self.live = call;
    }

    /// Someone by Tuple's id for them, with the name and address said,
    /// or the contacts' where they're blank.
    fn someone(&self, id: &str, name: &str, email: &str) -> Someone {
        let known = self.contacts.get(id);
        let or = |said: &str, known: Option<&str>| {
            if said.is_empty() {
                known.unwrap_or("").to_owned()
            } else {
                said.to_owned()
            }
        };
        Someone {
            id: id.to_owned(),
            name: or(name, known.map(|contact| contact.name.as_str())),
            email: or(email, known.map(|contact| contact.email.as_str())),
        }
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
        let someone = self.someone(&person.id, &person.name, &person.email);
        json!({
            "id": someone.id,
            "name": someone.name,
            "email": someone.email,
            "connection": person.connection,
            "muted": person.muted,
            "sharing": person.sharing,
            "camera": person.camera,
        })
    }

    /// The record, less when it was heard: how the call stands (none,
    /// ringing, connecting or connected), since when it connected, Joel's
    /// own mute, screen and camera, everyone else in it, who's calling,
    /// the calls missed, Joel's room, and Recent, the calls ended and
    /// missed lately, each named from the contacts where it was kept
    /// without. Joel's mute is said only once connected, since Tuple
    /// sends no voice before.
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
            "recent": self.recent.calls.iter().map(|past| {
                let people = past.people.iter()
                    .map(|one| self.someone(&one.id, &one.name, &one.email))
                    .collect();
                Past { people, ..past.clone() }.json()
            }).collect::<Vec<_>>(),
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
                "recent": [],
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
        assert_eq!(
            record["recent"],
            json!([{"people": [{"id": "7", "name": "Ada Brightwater",
                                "email": "ada@example.com"}],
                    "since": NOON - 10_000, "ended": NOON - 5_000, "missed": true,
                    "caller": null}])
        );
    }

    /// Tuple's stream saying a call connected at `since`, with these
    /// others in it, each by id and name.
    fn live(since: Option<i64>, people: &[(&str, &str)]) -> Live {
        Live {
            since,
            joel: Person::default(),
            people: people
                .iter()
                .map(|(id, name)| Person {
                    id: (*id).into(),
                    name: (*name).into(),
                    ..Person::default()
                })
                .collect(),
        }
    }

    #[test]
    fn an_ended_call_is_kept_with_everyone_who_was_in_it_and_its_length() {
        let mut held = held();
        held.follow(Some(live(None, &[("7", "")])), NOON);
        held.follow(Some(live(Some(NOON), &[("7", "")])), NOON + 1_000);
        held.follow(
            Some(live(Some(NOON), &[("7", ""), ("8", "Eli Hart")])),
            NOON + 60_000,
        );
        // Everyone else leaves before Joel does, as a call ends.
        held.follow(Some(live(Some(NOON), &[])), NOON + 2_400_000);
        assert!(held.recent.calls.is_empty(), "kept while it went on");
        held.follow(None, NOON + 2_520_000);
        assert_eq!(
            held.record()["recent"],
            json!([{"people": [
                        {"id": "7", "name": "Ada Brightwater", "email": "ada@example.com"},
                        {"id": "8", "name": "Eli Hart", "email": ""}],
                    "since": NOON, "ended": NOON + 2_520_000, "missed": false,
                    "caller": null}])
        );
        assert_eq!(held.record()["state"], "none");
    }

    /// Who rang whom, as Recent kept the one call `held` had.
    fn caller(held: &Held) -> Option<&str> {
        held.recent.calls[0].caller.as_deref()
    }

    #[test]
    fn a_call_after_a_ring_answered_was_the_callers() {
        let mut held = held();
        held.log
            .heard("11:59:50.000|calls|received incoming call: 7", NOON);
        let heard = held.log.heard("11:59:55.000|calls|accepting call", NOON);
        held.answer(heard.answered);
        held.follow(Some(live(None, &[("7", "")])), NOON);
        held.follow(Some(live(Some(NOON), &[("7", "")])), NOON + 1_000);
        held.follow(None, NOON + 600_000);
        assert_eq!(caller(&held), Some("7"));
        assert_eq!(held.record()["recent"][0]["caller"], "7");
    }

    #[test]
    fn a_ring_answered_after_tuple_said_the_call_is_the_callers_all_the_same() {
        let mut held = held();
        held.follow(Some(live(None, &[("7", "")])), NOON);
        held.answer(Some(Answered {
            who: "7".into(),
            at: NOON + 500,
        }));
        held.follow(Some(live(Some(NOON), &[("7", "")])), NOON + 1_000);
        held.follow(None, NOON + 600_000);
        assert_eq!(caller(&held), Some("7"));
    }

    #[test]
    fn joels_room_opened_began_the_call_in_it() {
        let mut held = held();
        held.began(String::new(), NOON);
        held.follow(Some(live(Some(NOON + 2_000), &[])), NOON + 2_000);
        held.follow(Some(live(Some(NOON + 2_000), &[("7", "")])), NOON + 90_000);
        held.follow(None, NOON + 600_000);
        assert_eq!(caller(&held), Some(""));
    }

    #[test]
    fn a_call_tuple_call_saw_nothing_begin_has_no_caller() {
        let answered = |who: &str, at: i64| {
            Some(Answered {
                who: who.into(),
                at,
            })
        };
        let cases = [
            ("nothing heard", None),
            (
                "a ring long before",
                answered("7", NOON - 2 * BEGINS_WITHIN),
            ),
            ("a ring from someone not in it", answered("8", NOON)),
            ("a ring the log doesn't number", answered("a caller", NOON)),
        ];
        for (what, heard) in cases {
            let mut held = held();
            held.answer(heard);
            held.follow(Some(live(Some(NOON), &[("7", "")])), NOON);
            held.follow(None, NOON + 600_000);
            assert_eq!(caller(&held), None, "{what}");
        }
    }

    #[test]
    fn a_call_said_again_keeps_who_began_it() {
        let mut held = held();
        held.began(String::new(), NOON);
        held.follow(Some(live(Some(NOON), &[("7", "")])), NOON);
        held.follow(None, NOON + 60_000);
        held.follow(
            Some(live(Some(NOON), &[("7", "")])),
            NOON + 3 * BEGINS_WITHIN,
        );
        held.follow(None, NOON + 600_000);
        assert_eq!(held.recent.calls.len(), 1);
        assert_eq!(caller(&held), Some(""));
    }

    #[test]
    fn a_call_never_connected_or_had_alone_is_not_kept() {
        let mut held = held();
        held.follow(Some(live(None, &[("7", "")])), NOON);
        held.follow(None, NOON + 30_000);
        held.follow(Some(live(Some(NOON + 60_000), &[])), NOON + 60_000);
        held.follow(None, NOON + 600_000);
        assert_eq!(held.recent.calls, []);
    }

    #[test]
    fn a_call_said_again_after_its_stream_ended_is_still_one_call() {
        let mut held = held();
        held.follow(Some(live(Some(NOON), &[("7", "")])), NOON);
        held.follow(None, NOON + 60_000);
        held.follow(Some(live(Some(NOON), &[("8", "Eli Hart")])), NOON + 65_000);
        held.follow(None, NOON + 600_000);
        let ids: Vec<_> = held.recent.calls[0]
            .people
            .iter()
            .map(|one| one.id.as_str())
            .collect();
        assert_eq!(held.recent.calls.len(), 1);
        assert_eq!(ids, ["7", "8"]);
        assert_eq!(held.recent.calls[0].ended, NOON + 600_000);
    }

    #[test]
    fn missed_calls_blend_in_newest_first() {
        let mut held = held();
        held.follow(Some(live(Some(NOON), &[("8", "Eli Hart")])), NOON);
        // Ada rings while Joel's in a call with Eli, and isn't answered.
        held.miss(Some(Missed {
            who: "7".into(),
            since: NOON + 60_000,
            at: NOON + 90_000,
        }));
        held.follow(None, NOON + 600_000);
        held.miss(Some(Missed {
            who: "an unknown caller".into(),
            since: NOON + 700_000,
            at: NOON + 760_000,
        }));
        let said = held.record()["recent"].clone();
        let order: Vec<_> = said
            .as_array()
            .unwrap()
            .iter()
            .map(|past| (past["since"].as_i64().unwrap(), past["missed"] == true))
            .collect();
        assert_eq!(
            order,
            [(NOON + 700_000, true), (NOON + 60_000, true), (NOON, false)]
        );
        assert_eq!(said[0]["people"], json!([]));
        assert_eq!(said[1]["people"][0]["name"], "Ada Brightwater");
    }

    #[test]
    fn recent_holds_only_the_newest_calls() {
        let mut held = held();
        for n in 0..30 {
            let since = NOON + n * 600_000;
            if n % 2 == 0 {
                held.follow(Some(live(Some(since), &[("7", "")])), since);
                held.follow(None, since + 300_000);
            } else {
                held.miss(Some(Missed {
                    who: "7".into(),
                    since,
                    at: since + 60_000,
                }));
            }
        }
        assert_eq!(held.recent.calls.len(), recent::KEPT);
        assert_eq!(held.recent.calls[0].since, NOON + 29 * 600_000);
    }

    #[test]
    fn recent_kept_without_names_is_named_from_the_contacts() {
        let mut held = held();
        held.recent = Recent::read(
            "{\"people\":[{\"id\":\"7\",\"name\":\"\",\"email\":\"\"}],\"since\":1,\
             \"ended\":2,\"missed\":true}\n",
        );
        assert_eq!(
            held.record()["recent"][0]["people"][0]["email"],
            "ada@example.com"
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
                since: at,
                at,
            }));
        }
        assert_eq!(held.missed.len(), MISSED);
        assert_eq!(held.missed[0].at, 10);
    }
}
