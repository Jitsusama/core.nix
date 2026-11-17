//! Joel's sign-in to a site, as the program a reader is given prints it.
//!
//! A reader runs the program each round, so a sign-in renewed while it
//! runs is read without a restart. It waits on the program only so long:
//! a reader waiting for good on a sign-in that hangs, a keyring asking for
//! an unlock nobody sees or a sign-in tool stuck on the network, reads
//! nothing again while its unit still says it's running, so every stop
//! said meanwhile goes unheard. Run over, the program and everything it
//! started are killed, and that round reads nothing; the next one asks
//! again.

use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// How long a sign-in may take: far past any that answers, and well short
/// of the minute between a reader's rounds.
pub const PATIENCE: Duration = Duration::from_secs(30);

/// How often a sign-in that has said its piece is looked at for its end.
const LOOK_EVERY: Duration = Duration::from_millis(20);

/// The sign-in `program` prints, trimmed, if it ends well within
/// `patience`, saying something. None when it can't be run, fails, prints
/// nothing or runs over; what it says otherwise is let go, so no part of a
/// sign-in reaches a journal.
#[must_use]
pub fn token(program: &OsStr, patience: Duration) -> Option<String> {
    let until = Instant::now() + patience;
    let mut running = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        // A group of its own, so what it started is killed with it.
        .process_group(0)
        .spawn()
        .ok()?;
    let mut out = running.stdout.take()?;
    let (said, heard) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let read = out.read_to_string(&mut text).map(|_| text);
        // An asker that ran out of patience has stopped listening, and
        // then what was read is let go with it.
        let _ = said.send(read);
    });
    // Its output ends only once all it started have let go of it, so a
    // child left holding the pipe runs the program over too.
    let Ok(Ok(text)) = heard.recv_timeout(until.saturating_duration_since(Instant::now())) else {
        return stop(running);
    };
    let ended = loop {
        match running.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < until => thread::sleep(LOOK_EVERY),
            _ => return stop(running),
        }
    };
    let token = text.trim();
    (ended.success() && !token.is_empty()).then(|| token.to_owned())
}

/// Kills `running`'s group, before reaping it, so its id can't have gone
/// to another, and waits for it; the round reads nothing.
fn stop(mut running: Child) -> Option<String> {
    if let Ok(group) = libc::pid_t::try_from(running.id()) {
        // SAFETY: killpg only sends a signal. The group is the one this
        // program was started in, and its leader isn't reaped yet, so the
        // id names nothing else.
        unsafe { libc::killpg(group, libc::SIGKILL) };
    }
    // Killed, it ends at once; an error says it already had.
    let _ = running.wait();
    None
}
