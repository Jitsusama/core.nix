//! What the follower hears of the person's other machines' calls, as the
//! mycelium nodes here list them: a-call-is-on names each machine whose
//! calls it can't hear, and a node here that won't answer, only to
//! whoever asked, and waits on none, so each stands here as a service's
//! fault until it's heard well. A thing counts once it's gone unheard for
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
    /// as a service's fault when it's gone unheard for [`UNHEARD_FOR`],
    /// and again only after it's heard well, a start between them or not.
    /// A look can say less than is so: a node restarting answers with few
    /// of its peers or none, and one that stays mute says nothing of those
    /// heard through it. So only what a look hears well mends (see
    /// [`mended`]); what it doesn't say keeps its count, and its time while
    /// it was unheard within [`UNHEARD_FOR`], so a restart counts nothing
    /// again and a node that stays mute keeps no time for ever.
    pub fn see_heard(&mut self, heard: &Heard, at: i64) {
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        let unheard = unheard(heard);
        let mended = mended(heard);
        let in_look = |thing: &str| unheard.iter().any(|(unheard, _)| unheard == thing);
        let well = |thing: &str| mended.iter().any(|mended| mended == thing);
        let mut counted: Vec<String> = self
            .day
            .seeing
            .unseen_calls
            .iter()
            .filter(|thing| !well(thing) && !in_look(thing))
            .cloned()
            .collect();
        self.unheard_since.retain(|thing, stretch| {
            !well(thing) && (in_look(thing) || at - stretch.last < UNHEARD_FOR)
        });
        for (thing, said) in unheard.iter().cloned() {
            let stretch = self.unheard_since.entry(thing.clone()).or_insert(Stretch {
                since: at,
                last: at,
            });
            stretch.last = at;
            let since = stretch.since;
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
/// every look that said it without one hearing it well, and when a look
/// last said it.
pub(super) struct Stretch {
    since: i64,
    last: i64,
}

/// What `heard` hears well, each the thing it's counted under: each
/// machine a fresh record says is on a call or on none, whatever another
/// realm's record says, since the strongest wins; and, once every node
/// here answered, the nodes, and the machines past the most named when
/// none goes unnamed.
pub(super) fn mended(heard: &Heard) -> Vec<String> {
    let mut mended: Vec<String> = heard
        .calls
        .iter()
        .chain(&heard.no_call)
        .map(|machine| format!("calls on {machine}"))
        .collect();
    if heard.answered > 0 && heard.unanswered.is_empty() {
        mended.push(NODES.to_owned());
        if heard.unnamed == 0 {
            mended.push(UNNAMED.to_owned());
        }
    }
    mended
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
