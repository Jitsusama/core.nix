//! The daemon's own asks answered in turn: off their connection's
//! reading thread, so one that waits holds up no get, watch or other ask
//! sent after it, yet each only once those of them asked before it are
//! done, on one thread a connection. The agents daemon's acts for Joel
//! wait on the user manager to say who the shell is, up to seconds after
//! a restart, which held every get, watch and drawer the shell asked
//! behind them; answered each on a thread of its own instead, a share's
//! end could land before its start (#6918, F99 and F16).

use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::mpsc::{self, SyncSender};

use serde_json::Value;

use super::hang_up::{Peer, Unqueued, queue};
use super::{QUEUED, Shared, owned};

impl Shared {
    /// Answers these of the daemon's own asks in turn, from then on: off
    /// their connection's reading thread, each after those of them asked
    /// before it on that connection; only the first a daemon gives is
    /// kept. One also named [`Shared::slow`] is answered apart, as a slow
    /// ask is.
    pub fn in_turn(&self, asks: &'static [&'static str]) {
        let _ = self.in_turn.set(asks);
    }

    pub(super) fn is_in_turn(&self, said: &Value) -> bool {
        let asked = said["ask"].as_str().unwrap_or("");
        self.in_turn.get().is_some_and(|asks| asks.contains(&asked))
    }
}

/// A connection's asks waiting their turn, and the one thread answering
/// them, started at its first, so a connection that never asks one
/// holds no thread for them. Dropped as its connection ends, its thread
/// answers what's left waiting, then ends.
#[derive(Default)]
pub(super) struct Turns {
    waiting: Option<SyncSender<(Value, Value)>>,
}

impl Turns {
    /// Hands `said`, asked under `id`, to its connection's turn. Past
    /// QUEUED waiting, as only a connection asking far faster than its
    /// asks are done would, it waits for room, as the reading thread did
    /// for each ask before; an unbounded queue would hold whatever such a
    /// connection sent.
    ///
    /// # Errors
    ///
    /// `Gone` once the turn's thread ended, its connection hung up on as
    /// an answer couldn't be queued.
    pub(super) fn take(
        &mut self,
        shared: &Arc<Shared>,
        (id, said): (&Value, &Value),
        told: &SyncSender<String>,
        stream: &Arc<UnixStream>,
        peer: &Arc<Peer>,
    ) -> Result<(), Unqueued> {
        let waiting = self
            .waiting
            .get_or_insert_with(|| answering(shared, told, stream, peer));
        waiting
            .send((id.clone(), said.clone()))
            .map_err(|_| Unqueued::Gone)
    }
}

/// The thread answering a connection's asks in turn, and what hands
/// them to it. One whose answer can't be queued is hung up on, and
/// nothing more is answered, as its reading thread stops at that.
fn answering(
    shared: &Arc<Shared>,
    told: &SyncSender<String>,
    stream: &Arc<UnixStream>,
    peer: &Arc<Peer>,
) -> SyncSender<(Value, Value)> {
    let (waiting, asks) = mpsc::sync_channel::<(Value, Value)>(QUEUED);
    let (shared, told) = (Arc::clone(shared), told.clone());
    let (stream, peer) = (Arc::clone(stream), Arc::clone(peer));
    std::thread::spawn(move || {
        for (id, said) in asks {
            if let Err(why) = queue(&told, &owned(&shared, &id, &said, &stream)) {
                shared.say_once(&peer, why, "an ask in turn was answered");
                let _ = stream.shutdown(Shutdown::Both);
                break;
            }
        }
    });
    waiting
}
