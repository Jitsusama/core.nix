//! What the follower hears of the person's other machines' calls, as the
//! mycelium nodes here list them: a-call-is-on names each machine whose
//! calls it can't hear, and a node here that won't answer, only to
//! whoever asked, and waits on none, so each stands here as a service's
//! fault until it mends. A thing counts once it's gone unheard for
//! [`UNHEARD_FOR`], so a node restarting or one probe running late, each
//! mended within a minute, isn't a fault.

use std::path::{Path, PathBuf};

use calls::elsewhere::Heard;

use super::{Follower, date_of};
use crate::day::Kind;

/// What the day calls a machine declaring no call probe.
pub(super) const CALL_UNSEEN: &str = "it declares no call probe, so a call there goes unseen";

/// What the day calls a machine whose call probe said nothing lately.
pub(super) const CALL_UNTOLD: &str =
    "its call probe said nothing lately, so a call there goes unseen";

/// What the day calls a machine whose record's time can't be read or
/// trusted.
pub(super) const CALL_UNDATED: &str =
    "its record is dated ahead of this clock or not at all, so a call there goes unseen";

/// How long a thing goes unheard, in microseconds, before it counts: a
/// record's freshness, the three minutes a late measure or a node's
/// restart is mended within.
const UNHEARD_FOR: i64 = calls::elsewhere::FRESH * 1_000_000;

/// What the day calls a node here that won't answer, or that an account
/// offering too many leaves unasked; a-call-is-on names which, and why.
pub(super) const NODE_SILENT: &str = "a mycelium node here won't answer or isn't asked, so its peers' calls go unheard; \
     a-call-is-on names which and why";

/// The thing the nodes here are counted under.
pub(super) const NODES: &str = "the mycelium nodes here";

/// The thing the machines past the most a-call-is-on names are counted
/// under, as one, so no node can make as many faults as it likes.
pub(super) const UNNAMED: &str = "calls on machines past the most named";

/// What the day calls those machines.
pub(super) const MACHINES_UNNAMED: &str =
    "the nodes here list more machines than a person's realms hold, so the rest go unnamed";

impl Follower {
    /// Ask the mycelium nodes under `runtimes` whose calls go unheard.
    #[must_use]
    pub fn hearing_peers(self, runtimes: PathBuf) -> Self {
        Self {
            peers: Some(runtimes),
            ..self
        }
    }

    /// Hear the nodes' peers at `at`.
    pub fn hear_peers(&mut self, at: i64) {
        if let Some(runtimes) = &self.peers {
            let heard = hear(runtimes);
            self.see_heard(&heard, at);
        }
    }

    /// Keep whose calls `heard` says go unheard at `at`: each counts once
    /// as a service's fault when it's gone unheard on every look for
    /// [`UNHEARD_FOR`], and again only after it's heard again, a start
    /// between them or not. A look on which a node here didn't answer, or
    /// none did, hears less than the last, so what it doesn't hear isn't
    /// heard: each such thing keeps its time and its count while it was
    /// unheard within [`UNHEARD_FOR`], so a node restarting counts nothing
    /// again, and one that stays mute keeps no time for ever.
    pub fn see_heard(&mut self, heard: &Heard, at: i64) {
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        let unheard = unheard(heard);
        let whole = heard.answered > 0 && heard.unanswered.is_empty();
        let in_look = |thing: &str| unheard.iter().any(|(unheard, _)| unheard == thing);
        let lately = |spell: &Spell| !whole && at - spell.last < UNHEARD_FOR;
        // A thing counted before a start has no spell yet, so it stays
        // counted until a whole look or one that hears it unheard.
        let mut counted: Vec<String> = self
            .day
            .seeing
            .unseen_calls
            .iter()
            .filter(|thing| {
                !whole && !in_look(thing) && self.unheard_since.get(*thing).is_none_or(lately)
            })
            .cloned()
            .collect();
        self.unheard_since
            .retain(|thing, spell| in_look(thing) || lately(spell));
        for (thing, said) in unheard.iter().cloned() {
            let spell = self.unheard_since.entry(thing.clone()).or_insert(Spell {
                since: at,
                last: at,
            });
            spell.last = at;
            let since = spell.since;
            if self.day.seeing.unseen_calls.contains(&thing) {
                counted.push(thing);
            } else if at - since >= UNHEARD_FOR {
                self.day.fault(&thing, Kind::Services, said, at);
                self.changed = true;
                counted.push(thing);
            }
        }
        if self.day.seeing.unseen_calls != counted {
            self.day.seeing.unseen_calls = counted;
            self.changed = true;
        }
    }
}

/// How long a thing whose calls go unheard has gone so: since when, on
/// every look that heard it, and when a look last heard it unheard.
pub(super) struct Spell {
    since: i64,
    last: i64,
}

/// Whose calls `heard` says go unheard, each a thing and what's said of
/// it: each machine under `calls on` and its name, apart from any service
/// of the same name, and the nodes here as one.
pub(super) fn unheard(heard: &Heard) -> Vec<(String, &'static str)> {
    let calls_on = |machine: &String| format!("calls on {machine}");
    let mut unheard: Vec<(String, &str)> = heard
        .unseen
        .iter()
        .map(|machine| (calls_on(machine), CALL_UNSEEN))
        .chain(
            heard
                .untold
                .iter()
                .map(|machine| (calls_on(machine), CALL_UNTOLD)),
        )
        .chain(
            heard
                .undated
                .iter()
                .map(|machine| (calls_on(machine), CALL_UNDATED)),
        )
        .collect();
    if !heard.unanswered.is_empty() {
        unheard.push((NODES.to_owned(), NODE_SILENT));
    }
    if heard.unnamed > 0 {
        unheard.push((UNNAMED.to_owned(), MACHINES_UNNAMED));
    }
    unheard
}

/// What the nodes under `runtimes` hear, on Linux, where a-call-is-on
/// hears them too.
#[cfg(target_os = "linux")]
fn hear(runtimes: &Path) -> Heard {
    calls::elsewhere::hear(runtimes)
}

/// Nothing, on a Mac, whose nodes keep their sockets where a-call-is-on,
/// asked as root, can't find them, so it hears no peers to fault.
#[cfg(not(target_os = "linux"))]
fn hear(_runtimes: &Path) -> Heard {
    Heard::default()
}
