//! Calls that skip noise cancelling, recording from a microphone itself
//! rather than its twin, as Joel's calls on the MV7 do until he picks a
//! cleaner by ear. Each is heard so it can be judged in the shadow
//! (shadow.rs) and offer Choose by Ear (offer.rs) as a twin's call does,
//! so the scorer learns from his real calls before he's picked at all.
//!
//! PipeWire's graph is followed as `pw-dump --monitor` prints it, as
//! call-transcripts follows it, and a call counts as modules/calls counts
//! one. While a call is on, every microphone running is told to pipe.rs,
//! which wakes its capture so the worker hears it (worker.rs). Nothing is
//! cleaned while it's heard, and nothing it hears leaves memory but the
//! scorer's numbers. A microphone whose twin is recorded from is the
//! twin's call, and judged as that.

use std::collections::BTreeSet;
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use calls::Call;
use dictation::graph::Graph;
use serde_json::Value;

/// How long to wait before following the graph again once pw-dump ends.
const AGAIN: Duration = Duration::from_mins(1);

/// The graph and the call, and the microphones told to be heard.
pub struct Follower {
    graph: Graph,
    call: Call,
    heard: BTreeSet<String>,
}

impl Default for Follower {
    fn default() -> Self {
        Self {
            graph: Graph::default(),
            call: Call::Off,
            heard: BTreeSet::new(),
        }
    }
}

impl Follower {
    /// Takes in one of pw-dump's arrays.
    pub fn take(&mut self, array: &Value) {
        self.graph.take(array);
    }

    /// Steps the call on at `now`, answering the microphones to hear when
    /// they've changed: those running while a call is on, until it's over.
    pub fn stepped(&mut self, now: Instant) -> Option<BTreeSet<String>> {
        self.call.step(self.graph.call().is_some(), now);
        // Still heard while a call is ending, so an app reconnecting is
        // the one call, as modules/calls counts it.
        let heard = match self.call {
            Call::On | Call::Ending(_) => self.graph.microphones_running(),
            Call::Off | Call::Starting(_) => BTreeSet::new(),
        };
        (heard != self.heard).then(|| {
            self.heard.clone_from(&heard);
            heard
        })
    }

    /// How long until the call starts or ends if nothing changes.
    pub fn due(&self, now: Instant) -> Option<Duration> {
        self.call.due(now)
    }
}

/// Follows calls in a thread of its own for as long as this runs, telling
/// `told` the microphones to hear each time they change. A pw-dump that
/// won't start or ends is said, and followed again a minute later.
pub fn follow(told: impl Fn(BTreeSet<String>) + Send + 'static) {
    let started = thread::Builder::new()
        .name("call follower".into())
        .spawn(move || {
            loop {
                if let Err(why) = followed(&told) {
                    eprintln!("noise-cancelled: calls that skip it won't be heard: {why}");
                }
                told(BTreeSet::new());
                thread::sleep(AGAIN);
            }
        });
    if let Err(why) = started {
        eprintln!("noise-cancelled: the call follower wouldn't start: {why}");
    }
}

/// Follows the graph until pw-dump ends.
fn followed(told: &impl Fn(BTreeSet<String>)) -> Result<(), String> {
    let mut dump = Command::new("pw-dump")
        .args(["--monitor", "--no-colors"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|why| format!("pw-dump wouldn't start: {why}"))?;
    let out = dump.stdout.take().ok_or("pw-dump has no output")?;
    let (arrays, heard) = mpsc::channel();
    thread::spawn(move || {
        let read = serde_json::Deserializer::from_reader(BufReader::new(out)).into_iter::<Value>();
        for array in read {
            let Ok(array) = array else { break };
            if arrays.send(array).is_err() {
                break;
            }
        }
    });
    let mut follower = Follower::default();
    loop {
        let wait = follower
            .due(Instant::now())
            .unwrap_or(Duration::from_secs(3600));
        match heard.recv_timeout(wait) {
            Ok(array) => follower.take(&array),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if let Some(microphones) = follower.stepped(Instant::now()) {
            told(microphones);
        }
    }
    let _ = dump.wait();
    Err("pw-dump ended".to_owned())
}

#[cfg(test)]
#[path = "plain_tests.rs"]
mod tests;
