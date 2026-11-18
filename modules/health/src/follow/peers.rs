//! What the follower hears of the person's other machines' calls, as the
//! mycelium nodes here list them: a-call-is-on names each machine whose
//! calls it can't hear, and a node here that won't answer, only to
//! whoever asked, and waits on none, so each stands here as a service's
//! fault until it mends.

use std::path::{Path, PathBuf};

use calls::elsewhere::Heard;

use super::{Follower, date_of};
use crate::day::Kind;

/// What the day calls a machine declaring no call probe.
pub(super) const CALL_UNSEEN: &str = "it declares no call probe, so a call there goes unseen";

/// What the day calls a machine whose call probe said nothing lately.
pub(super) const CALL_UNTOLD: &str =
    "its call probe said nothing lately, so a call there goes unseen";

/// What the day calls a node here that won't answer.
pub(super) const NODE_SILENT: &str =
    "a mycelium node here won't answer, so its peers' calls go unheard";

/// The thing the nodes here are counted under.
pub(super) const NODES: &str = "the mycelium nodes here";

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
            self.see_unheard_calls(unheard(&heard), at);
        }
    }

    /// Keep whose calls go `unheard` at `at`, each a thing and what's said
    /// of it: each counts once as a service's fault, and again only after
    /// it's heard again, a start between them or not.
    pub fn see_unheard_calls(&mut self, unheard: Vec<(String, &str)>, at: i64) {
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        for (thing, said) in &unheard {
            if !self.day.seeing.unseen_calls.contains(thing) {
                self.day.fault(thing, Kind::Services, said, at);
                self.changed = true;
            }
        }
        let things: Vec<String> = unheard.into_iter().map(|(thing, _)| thing).collect();
        if self.day.seeing.unseen_calls != things {
            self.day.seeing.unseen_calls = things;
            self.changed = true;
        }
    }
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
        .collect();
    if !heard.unanswered.is_empty() {
        unheard.push((NODES.to_owned(), NODE_SILENT));
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
