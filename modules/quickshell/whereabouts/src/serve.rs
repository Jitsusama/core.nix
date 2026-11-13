//! Where the laptop is, held for the shell and clock-follows, and served
//! over the socket systemd holds for it by daemon-serve, in the shell's
//! daemon interface (interface.rs). One record, `here:laptop`, says it
//! whole: the position GeoClue last found or Joel set, what it's called
//! and the zone it keeps, and beside it the zone the clock keeps and its
//! last move. Given the launcher's words, Location's rows are held beside
//! it, made again from what's known as it changes (rows.rs), each place's
//! kept as the ready-rows crate keeps every daemon's, wrapped and placed
//! in their order (ready-rows.md). Each change is handed to daemon-serve
//! whole, which tells each watch only what it changed, and nothing is read
//! for an ask: everything said is already held. Joel's word on where the
//! laptop is, `set` or `again`, is an act that changes the pin (pin.rs),
//! which the daemon then hears as it hears any change of it; it's answered
//! on a thread of its own, since `set` waits on the web.

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use daemon_serve::serve::Shared as Served;
use ready_rows::kept::{Kept, Moved};
use serde_json::{Value, json};

use crate::fix::Fix;
use crate::interface::{DOMAIN, HERE, act_of, error, refused};
use crate::pin::{self, Unpinned};
use crate::rows;
use crate::words::Words;

/// Location's own asks that wait, each answered on a thread of its own so
/// none asked after it waits too: `set` asks the web for the town named.
const SLOW: &[&str] = &["act"];

/// What's known, from which every answer is made.
#[derive(Debug, Default)]
pub struct Known {
    /// Where the laptop is, or none while it isn't known.
    pub here: Option<Fix>,
    /// The zone the clock keeps, or none where it can't be read.
    pub clock_zone: Option<String>,
    /// The clock's last move, as clock.rs reads it.
    pub moved: Option<Value>,
    /// How GeoClue stands, in the interface's words, or none on a machine
    /// that doesn't follow the laptop and never asks it.
    pub geoclue: Option<Value>,
    /// How the last asking of a place's name went, likewise.
    pub names: Option<Value>,
}

struct Held {
    known: Known,
    /// Each record as daemon-serve was last handed it: For You's row with
    /// here beside it, and Location's own rows, a Kept for each place,
    /// since an order is only weighed against its own place's.
    on: Kept,
    location: Kept,
}

impl Held {
    /// Every record as last made.
    fn records(&self) -> Vec<Value> {
        self.on.all().chain(self.location.all()).cloned().collect()
    }

    /// Makes every record again from what's known, answering what moved:
    /// here, and Location's rows where there are words to say them in.
    fn make(&mut self, words: Option<&Words>, now: u64) -> Moved {
        let (on, own): (Vec<Value>, Vec<Value>) = words
            .map(|words| {
                let known = &self.known;
                rows::rows(known.here.as_ref(), known.clock_zone.as_deref(), words, now)
            })
            .unwrap_or_default()
            .into_iter()
            .partition(|row| row["key"] == rows::ON);
        let here = record(&self.known);
        let mut moved = self.on.take(on, vec![here]);
        let Moved { changed, gone } = self.location.take(own, Vec::new());
        moved.changed.extend(changed);
        moved.gone.extend(gone);
        moved
    }
}

/// What the daemon's threads share: what's known, and what serves it.
pub struct Shared {
    held: Mutex<Held>,
    /// What answers every connection, from the records handed it.
    served: Arc<Served>,
    /// The launcher's words, which Location's rows are worded from; none
    /// serves no rows.
    words: Option<Words>,
    /// Where Joel's own word on where the laptop is kept; none where the
    /// laptop isn't followed, which takes no word.
    pin: Option<PathBuf>,
    /// Woken each time the records change, for `turn` to look again at
    /// when they next say something another way.
    turned: Condvar,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared.held.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The one record, made from what's known. Each figure GeoClue doesn't
/// know, as Wi-Fi never knows the altitude, is null.
pub fn record(known: &Known) -> Value {
    let here = known.here.as_ref();
    json!({
        "key": HERE,
        "kind": "here",
        "latitude": here.map(|fix| fix.latitude),
        "longitude": here.map(|fix| fix.longitude),
        "accuracy_m": here.map(|fix| fix.accuracy),
        "at_s": here.map(|fix| fix.when),
        "altitude_m": here.and_then(|fix| fix.altitude),
        "speed_m_s": here.and_then(|fix| fix.speed),
        "heading_deg": here.and_then(|fix| fix.heading),
        "source": here.and_then(|fix| fix.source.as_deref()),
        "place": here.and_then(|fix| fix.place.as_deref()),
        "country": here.and_then(|fix| fix.country.as_deref()),
        "zone": here.and_then(|fix| fix.zone.as_deref()),
        "zone_holds": here.and_then(|fix| fix.zone_holds),
        "hand": here.map(|fix| fix.hand),
        "clock_zone": known.clock_zone,
        "moved": known.moved,
    })
}

/// Each of its sources by name, with its word for how it stands and, for
/// one refused, why, which daemon-serve says on health alone.
fn sources(known: &Known) -> Value {
    let mut sources = serde_json::Map::new();
    sources.insert("clock".into(), json!({"state": "answering"}));
    if let Some(geoclue) = &known.geoclue {
        sources.insert("geoclue".into(), geoclue.clone());
    }
    if let Some(names) = &known.names {
        sources.insert("names".into(), names.clone());
    }
    Value::Object(sources)
}

impl Shared {
    pub fn new(known: Known, words: Option<Words>, pin: Option<PathBuf>) -> Arc<Shared> {
        let mut held = Held {
            known,
            on: Kept::default(),
            location: Kept::default(),
        };
        held.make(words.as_ref(), now_ms());
        let shared = Arc::new_cyclic(|this: &Weak<Shared>| {
            let this = this.clone();
            let own = Box::new(move |said: &Value| this.upgrade()?.own(said));
            Shared {
                served: Served::new(DOMAIN, own),
                held: Mutex::new(held),
                words,
                pin,
                turned: Condvar::new(),
            }
        });
        shared.served.slow(SLOW);
        let held = lock(&shared);
        shared.served.change(held.records(), sources(&held.known));
        drop(held);
        shared
    }

    /// What answers every connection, for the socket to hand them to.
    pub fn served(&self) -> Arc<Served> {
        Arc::clone(&self.served)
    }

    /// Changes what's known, and hands daemon-serve the records and the
    /// sources again, which tells each watch only what moved of those it
    /// wants, and a source can move with no record moving. Handed while
    /// what's known is held, so two changes reach it in the order made.
    pub fn change(&self, how: impl FnOnce(&mut Known)) {
        let mut held = lock(self);
        how(&mut held.known);
        let Moved { changed, gone } = held.make(self.words.as_ref(), now_ms());
        self.served.change(held.records(), sources(&held.known));
        if !(changed.is_empty() && gone.is_empty()) {
            self.turned.notify_all();
        }
    }

    /// The answer to an ask of Location's own, without its id, which
    /// daemon-serve gives; none for an ask it doesn't know.
    fn own(&self, said: &Value) -> Option<Value> {
        let act = act_of(said)?;
        Some(self.act(&act.act, &act.words))
    }

    /// Joel's word on where the laptop is, answered once it's kept. What
    /// it changes is told as the pin is heard changing, as when it's
    /// changed by hand.
    fn act(&self, act: &str, words: &str) -> Value {
        let Some(pin) = self.pin.as_deref() else {
            return error("not-following", "the laptop isn't followed here");
        };
        let done = match act {
            "again" => pin::again(pin).map_err(|why| error("refused", &why.to_string())),
            "set" if words.trim().is_empty() => Err(error("bad-act", "set names no place")),
            "set" => {
                pin::set(pin, words.trim()).map_err(|unpinned| self.unpinned(&unpinned, words))
            }
            _ => Err(error("unknown-act", &format!("no act {act:?} here"))),
        };
        match done {
            Ok(()) => json!({"done": true}),
            Err(refusal) => refusal,
        }
    }

    /// Why the town Joel named wasn't pinned: what went wrong, and as the
    /// launcher's foot says it, in Location's words (the phrase's id where
    /// none were handed over). A pin that couldn't be kept says why on the
    /// journal too, which isn't his to read.
    fn unpinned(&self, unpinned: &Unpinned, words: &str) -> Value {
        let words = words.trim();
        let (code, message, phrase) = match unpinned {
            Unpinned::Nowhere => ("nowhere", format!("nothing is called {words:?}"), "nowhere"),
            Unpinned::Unasked => (
                "unasked",
                format!("{words:?} couldn't be looked up"),
                "unasked",
            ),
            Unpinned::Unkept(why) => {
                let message = format!("the place found for {words:?} wasn't kept: {why}");
                eprintln!("<4>{message}");
                ("refused", message, "unkept")
            }
        };
        let said = self.words.as_ref().map_or_else(
            || phrase.to_owned(),
            |said| said.said(phrase, &[("words", words)]),
        );
        refused(code, &message, &said)
    }
}

/// Says each record again as it next says something another way though
/// nothing else changed, as when it was found turns from how long ago to
/// its day a day on, and otherwise sleeps until the records change: so it
/// never wakes but to change something.
pub fn turn(shared: &Shared) {
    if shared.words.is_none() {
        return;
    }
    let mut held = lock(shared);
    loop {
        let now = now_ms();
        held = match rows::turns(held.known.here.as_ref(), now) {
            Some(at) if at <= now => {
                drop(held);
                shared.change(|_| ());
                lock(shared)
            }
            Some(at) => {
                let until = Duration::from_millis(at - now);
                shared
                    .turned
                    .wait_timeout(held, until)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
            None => shared
                .turned
                .wait(held)
                .unwrap_or_else(PoisonError::into_inner),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nowhere_is_null_and_the_clock_still_said() {
        let known = Known {
            clock_zone: Some("America/Halifax".into()),
            ..Known::default()
        };
        let said = record(&known);
        assert_eq!(said["key"], HERE);
        assert_eq!(said["kind"], "here");
        assert_eq!(said["latitude"], Value::Null);
        assert_eq!(said["hand"], Value::Null);
        assert_eq!(said["clock_zone"], "America/Halifax");
        assert_eq!(said["moved"], Value::Null);
    }

    fn kelpwater() -> Fix {
        Fix::new(12.3457, -98.7654, 26_000.0, 1_790_000_000)
            .unwrap()
            .named(
                Some("Kelpwater, Westmark".into()),
                Some("America/Toronto".into()),
            )
    }

    #[test]
    fn a_fix_is_said_in_its_units() {
        let fix = kelpwater().countried(Some("Canada".into()));
        let said = record(&Known {
            here: Some(fix),
            ..Known::default()
        });
        assert_eq!(said["latitude"], 12.3457);
        assert_eq!(said["accuracy_m"], 26_000.0);
        assert_eq!(said["at_s"], 1_790_000_000_u64);
        assert_eq!(said["place"], "Kelpwater, Westmark");
        assert_eq!(said["country"], "Canada");
        assert_eq!(said["zone"], "America/Toronto");
        assert_eq!(said["hand"], false);
        assert!(said["altitude_m"].is_null(), "Wi-Fi finds no altitude");
        assert!(said["source"].is_null(), "a fix GeoClue named no source of");
    }

    /// What's held from nothing known, made once.
    fn held(words: Option<&Words>) -> Held {
        let mut held = Held {
            known: Known::default(),
            on: Kept::default(),
            location: Kept::default(),
        };
        held.make(words, 0);
        held
    }

    fn keys(records: &[Value]) -> Vec<&str> {
        records
            .iter()
            .filter_map(|record| record["key"].as_str())
            .collect()
    }

    #[test]
    fn making_what_changed_nothing_moves_nothing() {
        let mut held = held(None);
        let moved = held.make(None, 0);
        assert!(
            moved.changed.is_empty() && moved.gone.is_empty(),
            "made again from the same, nothing moves: {:?}",
            keys(&moved.changed)
        );
        held.known.clock_zone = Some("UTC".into());
        let moved = held.make(None, 0);
        assert_eq!(keys(&moved.changed), [HERE], "only here moves");
        assert_eq!(moved.changed[0]["clock_zone"], "UTC");
    }

    #[test]
    fn a_new_here_moves_only_the_rows_it_changes() {
        let words = crate::words::fixture();
        let mut held = held(Some(&words));
        held.known.here = Some(kelpwater());
        let moved = held.make(Some(&words), 0);
        let changed = keys(&moved.changed);
        assert!(changed.contains(&HERE), "here moves: {changed:?}");
        assert!(
            changed.contains(&"row:location:town"),
            "the town is made: {changed:?}"
        );
        assert!(
            changed.contains(&"row:location:copy"),
            "Copy comes: {changed:?}"
        );
        assert!(
            !changed.contains(&"row:location:set"),
            "Set It by Hand, as it stood, doesn't move: {changed:?}"
        );

        held.known.here = None;
        let moved = held.make(Some(&words), 0);
        assert!(
            moved.gone.iter().any(|key| key == "row:location:copy"),
            "Copy goes once nowhere is known: {:?}",
            moved.gone
        );
    }

    #[test]
    fn a_source_is_handed_over_with_its_reason() {
        let known = Known {
            geoclue: Some(json!({"state": "refused", "reason": "no bus"})),
            ..Known::default()
        };
        assert_eq!(
            sources(&known),
            json!({
                "clock": {"state": "answering"},
                "geoclue": {"state": "refused", "reason": "no bus"},
            }),
            "daemon-serve says the word, and the reason on health alone"
        );
    }

    #[test]
    fn joels_word_is_refused_where_the_laptop_isnt_followed_or_says_nothing() {
        let unfollowed = Shared::new(Known::default(), None, None);
        let said = |shared: &Shared, ask: Value| shared.own(&ask).unwrap();
        assert_eq!(
            said(&unfollowed, json!({"ask": "act", "act": "again"}))["error"]["code"],
            "not-following"
        );
        assert!(
            unfollowed.own(&json!({"ask": "move"})).is_none(),
            "another ask is left to daemon-serve to refuse"
        );
        let folder = std::env::temp_dir().join(format!("whereabouts-act-{}", std::process::id()));
        let pin = folder.join("pin.json");
        let followed = Shared::new(Known::default(), None, Some(pin.clone()));
        let set = json!({"ask": "act", "act": "set", "words": "  "});
        assert_eq!(said(&followed, set)["error"]["code"], "bad-act");
        let wander = json!({"ask": "act", "act": "wander"});
        assert_eq!(said(&followed, wander)["error"]["code"], "unknown-act");
        std::fs::create_dir_all(&folder).unwrap();
        kelpwater().by_hand().write(&pin).unwrap();
        let again = json!({"ask": "act", "act": "again"});
        assert_eq!(said(&followed, again.clone()), json!({"done": true}));
        assert!(!pin.exists(), "finding it again takes the pin away");
        assert_eq!(
            said(&followed, again),
            json!({"done": true}),
            "no pin to take away is done too"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_town_not_pinned_is_refused_in_locations_words() {
        let worded = Shared::new(Known::default(), Some(crate::words::fixture()), None);
        let refusal = |code: &str, message: &str, said: &str| {
            let error = json!({"code": code, "message": message, "said": said});
            json!({"error": error})
        };
        assert_eq!(
            worded.unpinned(&Unpinned::Nowhere, " Kelpwater "),
            refusal(
                "nowhere",
                "nothing is called \"Kelpwater\"",
                "no place is called Kelpwater"
            )
        );
        assert_eq!(
            worded.unpinned(&Unpinned::Unasked, "Kelpwater"),
            refusal(
                "unasked",
                "\"Kelpwater\" couldn't be looked up",
                "couldn't ask where Kelpwater is"
            )
        );
        assert_eq!(
            worded.unpinned(&Unpinned::Unkept("read-only".into()), "Kelpwater"),
            refusal(
                "refused",
                "the place found for \"Kelpwater\" wasn't kept: read-only",
                "Kelpwater couldn't be set"
            )
        );
        let unworded = Shared::new(Known::default(), None, None);
        assert_eq!(
            unworded.unpinned(&Unpinned::Nowhere, "Kelpwater")["error"]["said"],
            "nowhere",
            "with no words handed over, the phrase's id"
        );
    }
}
