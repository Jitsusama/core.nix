//! Joel's sign-in to a site, as the program a reader is given prints it.
//!
//! A reader runs the program each round, so a sign-in renewed while it
//! runs is read without a restart. It waits on the program only so long:
//! a reader waiting for good on a sign-in that hangs, a keyring asking for
//! an unlock nobody sees or a sign-in tool stuck on the network, reads
//! nothing again while its unit still says it's running, so every stop
//! said meanwhile goes unheard. Run over, the program and everything it
//! started are killed, and the reader is told so, to say it and rest.

use std::ffi::OsStr;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long a sign-in may take: far past any that answers, and well short
/// of the minute between a reader's rounds.
pub const PATIENCE: Duration = Duration::from_secs(30);

/// How often a sign-in that has said its piece is looked at for its end.
const LOOK_EVERY: Duration = Duration::from_millis(20);

/// What a sign-in program gave.
#[derive(Debug, PartialEq, Eq)]
pub enum SignIn {
    /// The sign-in it printed, trimmed.
    Token(String),
    /// None: it couldn't be run, failed, or printed nothing, as a keyring
    /// lacking the item does.
    Missing,
    /// It ran past its patience, and it and all it started were killed.
    RanOver,
}

/// What `program` gives within `patience`. What it says otherwise is let
/// go, so no part of a sign-in reaches a journal.
#[must_use]
pub fn token(program: &OsStr, patience: Duration) -> SignIn {
    let until = Instant::now() + patience;
    let Ok(mut running) = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        // A group of its own, so what it started is killed with it.
        .process_group(0)
        .spawn()
    else {
        return SignIn::Missing;
    };
    let Some(out) = running.stdout.take() else {
        return stop(running);
    };
    // Its output ends only once all it started have let go of it, so a
    // child left holding the pipe runs the program over too. Read here,
    // not on a thread of its own, so nothing is left waiting once it is.
    let Some(text) = read_until(out, until) else {
        return stop(running);
    };
    let ended = loop {
        match running.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < until => thread::sleep(LOOK_EVERY),
            Ok(None) => return stop(running),
            // Not this program's to wait on any more, so nothing to kill.
            Err(_) => return SignIn::Missing,
        }
    };
    match text.trim() {
        token if ended.success() && !token.is_empty() => SignIn::Token(token.to_owned()),
        _ => SignIn::Missing,
    }
}

/// All `out` holds once it ends, or None when it hasn't by `until` or
/// can't be read. Dropped either way, so a writer left behind finds the
/// pipe closed.
fn read_until(mut out: ChildStdout, until: Instant) -> Option<String> {
    let mut held = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let mut ready = libc::pollfd {
            fd: out.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // Rounded up, so a wait under a millisecond still waits.
        let ms = libc::c_int::try_from(left.as_millis() + 1).unwrap_or(libc::c_int::MAX);
        // SAFETY: poll reads one pollfd, kept alive on this stack for the
        // call, naming a descriptor `out` owns.
        let answered = unsafe { libc::poll(&raw mut ready, 1, ms) };
        if answered < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return None;
        }
        // Out of time, whether it went quiet or never stops printing.
        if answered == 0 || Instant::now() >= until {
            return None;
        }
        // Ready, so this read doesn't wait: what's there, or its end.
        match out.read(&mut chunk) {
            Ok(0) => return String::from_utf8(held).ok(),
            Ok(read) => held.extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

/// Kills `running`'s group, and `running` itself should it have left the
/// group, before reaping it, so its id can't have gone to another; then
/// reaps it, killed, so the wait is short.
fn stop(mut running: Child) -> SignIn {
    if let Ok(group) = libc::pid_t::try_from(running.id()) {
        // SAFETY: killpg only sends a signal. The group is the one this
        // program was started in, and its leader isn't reaped yet, so the
        // id names nothing else.
        unsafe { libc::killpg(group, libc::SIGKILL) };
    }
    // An error says it had already ended, which the wait then reaps.
    let _ = running.kill();
    let _ = running.wait();
    SignIn::RanOver
}
