//! Health's end by a signal, as when a `health follow` run by hand is
//! killed or its terminal closes. Health's followers are its own children,
//! and a signal that ends health unhandled ends none of them, each living
//! on until its next line finds no one reading. So the signals that ask
//! health to stop are caught, and a thread waiting on what the catch writes
//! ends the followers, then health by the same signal, so whatever started
//! health sees the stop it asked for. A service manager's stop ends
//! health's whole group anyway; this covers health run anywhere else.
//!
//! The signals are caught rather than blocked and waited for: a blocked
//! signal stays blocked in every child, so a follower would sit out a
//! service manager's stop until it's killed. A caught signal's catch is
//! undone in a child as it starts its program, so each follower still ends
//! on the signals health catches.

use std::fs::File;
use std::io::{self, Read as _};
use std::mem;
use std::os::fd::{AsRawFd as _, FromRawFd as _, IntoRawFd as _, OwnedFd};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread::{self, JoinHandle};

use libc::c_int;

/// The signals that ask health to stop: a service manager's or `kill`'s,
/// an interrupt typed at its terminal, and its terminal closing.
pub const STOPPING: [c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// Where a caught signal is written, as its number in a byte, for the
/// thread waiting on it: one pipe for the whole process, as a signal's
/// catch is the process's.
static CAUGHT: AtomicI32 = AtomicI32::new(-1);

/// Signals caught, and what they're read from as they arrive.
pub struct Caught(File);

impl Caught {
    /// Catch `signals`, each from now on written for `on_one` to read
    /// rather than ending health. A process catches through one pipe, so
    /// a second catch takes the first's place.
    ///
    /// # Errors
    ///
    /// When the pipe can't be made, or a signal isn't one that can be
    /// caught.
    pub fn catch(signals: &[c_int]) -> io::Result<Self> {
        let mut ends = [0; 2];
        // SAFETY: pipe writes the two ends it makes into the array given.
        if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: both ends were just made, and are owned here alone.
        let (read, write) =
            unsafe { (OwnedFd::from_raw_fd(ends[0]), OwnedFd::from_raw_fd(ends[1])) };
        // Neither end goes to a child; and a catch never waits to write,
        // only a pipe full of signals unread would make it.
        for (end, kind, flag) in [
            (&read, libc::F_SETFD, libc::FD_CLOEXEC),
            (&write, libc::F_SETFD, libc::FD_CLOEXEC),
            (&write, libc::F_SETFL, libc::O_NONBLOCK),
        ] {
            // SAFETY: the end is open, and its flag is one fcntl sets.
            if unsafe { libc::fcntl(end.as_raw_fd(), kind, flag) } == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        // Kept open for the process's life, as the catch may write at any
        // time; one replaced by a second catch is closed with it.
        let replaced = CAUGHT.swap(write.into_raw_fd(), Ordering::SeqCst);
        if replaced >= 0 {
            // SAFETY: the end was taken from an `OwnedFd` and is no one's.
            drop(unsafe { OwnedFd::from_raw_fd(replaced) });
        }
        // SAFETY: a zeroed sigaction is an empty one, its mask filled
        // below.
        let mut action: libc::sigaction = unsafe { mem::zeroed() };
        action.sa_sigaction = heard as extern "C" fn(c_int) as libc::sighandler_t;
        // Interrupted calls go on, as they would had nothing caught.
        action.sa_flags = libc::SA_RESTART;
        // SAFETY: the mask is the action's own.
        unsafe { libc::sigemptyset(&raw mut action.sa_mask) };
        for &signal in signals {
            // SAFETY: the action is whole, and the old one isn't asked for.
            if unsafe { libc::sigaction(signal, &raw const action, ptr::null_mut()) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Self(File::from(read)))
    }

    /// Wait, on a thread of its own, for one of the signals caught; then
    /// `end` what must not outlive health, and go on `then` with the
    /// signal, as `die_by` does.
    pub fn on_one(
        self,
        end: impl FnOnce() + Send + 'static,
        then: impl FnOnce(c_int) + Send + 'static,
    ) -> JoinHandle<()> {
        let Self(mut caught) = self;
        thread::spawn(move || {
            let mut signal = [0];
            if caught.read_exact(&mut signal).is_ok() {
                end();
                then(c_int::from(signal[0]));
            }
        })
    }
}

/// The catch: the signal written for the thread waiting on it, and
/// nothing else, as little else is safe while a signal is caught.
extern "C" fn heard(signal: c_int) {
    let number = u8::try_from(signal).unwrap_or(0);
    // SAFETY: write is safe within a catch, and a pipe closed or full
    // only fails it.
    unsafe { libc::write(CAUGHT.load(Ordering::SeqCst), (&raw const number).cast(), 1) };
}

/// End health by `signal`, as it would have ended had nothing caught it,
/// so whatever started health sees the same end.
pub fn die_by(signal: c_int) {
    // SAFETY: the signal's own action is restored, then raised, which ends
    // the process.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
    // Only reached when the signal didn't end health: a shell's exit status
    // for an end by it.
    std::process::exit(128 + signal);
}

#[cfg(test)]
mod tests;
