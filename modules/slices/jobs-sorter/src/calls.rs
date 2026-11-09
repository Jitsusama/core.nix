//! Follows whether a call is on, for the door, which keeps more room
//! through one and lets nothing held into it (`src/door.rs`).
//!
//! A call is what modules/calls' crate counts as one: a program recording
//! from a microphone, Meet in a browser as much as Tuple. Tuple's own call
//! state misses a call in a browser, and on 9 November it was a Meet
//! call's microphone that went unread for 59 s. It reads `pw-dump
//! --monitor`, which prints PipeWire's graph as it changes, on a thread of
//! its own, so the sorter's loop never waits on PipeWire; the loop steps a
//! [`Call`] with what it heard, which says a call is on once the
//! recording has gone on two seconds.

use calls::{Call, Recording};
use serde_json::Value;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long a pw-dump that ended waits before it's started again.
const AGAIN: Duration = Duration::from_secs(10);

/// Whether a call is on, from what the thread hears.
pub struct Calls {
    recording: Arc<AtomicBool>,
    call: Call,
}

impl Calls {
    /// Starts following PipeWire's graph with `pw_dump`.
    pub fn follow(pw_dump: &Path) -> Calls {
        let recording = Arc::new(AtomicBool::new(false));
        let heard = Arc::clone(&recording);
        let pw_dump = pw_dump.to_path_buf();
        let followed = thread::Builder::new()
            .name("calls".into())
            .spawn(move || follow(&pw_dump, &heard));
        if let Err(error) = followed {
            eprintln!("jobs-sorter: calls can't be followed: {error}");
        }
        Calls {
            recording,
            call: Call::Off,
        }
    }

    /// Whether a call is on `now`.
    pub fn on(&mut self, now: Instant) -> bool {
        self.call.step(self.recording.load(Ordering::Relaxed), now);
        matches!(self.call, Call::On | Call::Ending(_))
    }
}

/// Reads pw-dump's arrays into `recording`, starting it again when it ends,
/// as it does when PipeWire restarts.
fn follow(pw_dump: &PathBuf, recording: &AtomicBool) {
    loop {
        match Command::new(pw_dump)
            .arg("--monitor")
            .arg("--no-colors")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut dump) => {
                if let Some(out) = dump.stdout.take() {
                    let mut heard = Recording::default();
                    let arrays = serde_json::Deserializer::from_reader(BufReader::new(out))
                        .into_iter::<Value>();
                    for array in arrays.map_while(Result::ok) {
                        heard.take(&array);
                        recording.store(heard.any(), Ordering::Relaxed);
                    }
                }
                let _ = dump.wait();
            }
            Err(error) => eprintln!("jobs-sorter: pw-dump wouldn't start: {error}"),
        }
        recording.store(false, Ordering::Relaxed);
        thread::sleep(AGAIN);
    }
}
