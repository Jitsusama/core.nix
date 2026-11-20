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
//! on the signals health catches. One health was started ignoring, as
//! under `nohup`, is left ignored.
//!
//! A stop under way is `stopping`, from the moment it's caught: a follower
//! the same stop ended, as a terminal's interrupt ends its whole group,
//! says nothing of its end, and health's own end waits for the signal's.
//! A second stop while the first ends the followers ends health at once,
//! leaving any follower not yet ended, so a stop can never hang.

use std::io::{self, PipeReader, Read as _};
use std::mem;
use std::os::fd::{AsRawFd as _, IntoRawFd as _, OwnedFd};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

use libc::c_int;

/// The signals that ask health to stop: a service manager's or `kill`'s,
/// an interrupt typed at its terminal, and its terminal closing.
const STOPPING: [c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// Where a caught signal is written, as its number in a byte, for the
/// thread waiting on it: one pipe for the whole process, as a signal's
/// catch is the process's.
static CAUGHT: AtomicI32 = AtomicI32::new(-1);

/// Whether a stop has been caught.
static STOPPED: AtomicBool = AtomicBool::new(false);

/// How long a follower that ends waits for health to catch a stop before
/// it says so: a stop to health's whole group, as a service manager's,
/// reaches health and its followers at once, and a follower may end of it
/// before health's catch has run, by the signal or, as journalctl does,
/// cleanly.
pub const CAUGHT_WITHIN: Duration = Duration::from_millis(100);

/// `run`, with health's stops caught: a stop calls `end`, on a thread of
/// its own, then ends health by its signal, and once one is caught,
/// health ends by it alone, whatever `run` returns.
///
/// # Errors
///
/// When the pipe can't be made or the signals caught, or when they
/// already are, or what `run` returns.
pub fn until_stopped<T>(
    end: impl FnOnce() + Send + 'static,
    run: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    ending(end)?;
    let ran = run();
    await_stop();
    ran
}

/// From now on, a signal that asks health to stop calls `end`, on a
/// thread of its own, then ends health by that signal. A second while
/// the first's `end` runs ends health at once, so a stop can't hang.
fn ending(end: impl FnOnce() + Send + 'static) -> io::Result<()> {
    let mut caught = catch(&STOPPING)?;
    thread::spawn(move || {
        let mut signal = [0];
        if caught.read_exact(&mut signal).is_ok() {
            end();
            die_by(c_int::from(signal[0]));
        }
    });
    Ok(())
}

/// Whether a stop has been caught, so health is ending by it.
pub fn stopping() -> bool {
    STOPPED.load(Ordering::SeqCst)
}

/// Wait for good once a stop has been caught, for the thread it's
/// caught on to end health by its signal, rather than end health some
/// other way first.
fn await_stop() {
    while stopping() {
        thread::park();
    }
}

/// Catch `signals`, but those health was started ignoring, each written
/// to the pipe read from what this answers.
fn catch(signals: &[c_int]) -> io::Result<PipeReader> {
    // Both ends are made closed to every child.
    let (read, write) = io::pipe()?;
    let write = OwnedFd::from(write);
    // The catch never waits to write; only a pipe full of signals unread
    // would make it.
    // SAFETY: the end is open, and O_NONBLOCK is a flag fcntl sets.
    if unsafe { libc::fcntl(write.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // Kept open for the process's life, as the catch may write at any
    // time; so a process catches once.
    let write = write.into_raw_fd();
    if CAUGHT
        .compare_exchange(-1, write, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // SAFETY: the end is this call's own, and no catch writes to it.
        unsafe { libc::close(write) };
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the stops are already caught",
        ));
    }
    // SAFETY: a zeroed sigaction is an empty one, its mask filled below.
    let mut action: libc::sigaction = unsafe { mem::zeroed() };
    action.sa_sigaction = heard as extern "C" fn(c_int) as libc::sighandler_t;
    // Interrupted calls go on, as they would had nothing caught.
    action.sa_flags = libc::SA_RESTART;
    // SAFETY: the mask is the action's own.
    unsafe { libc::sigemptyset(&raw mut action.sa_mask) };
    for &signal in signals {
        // SAFETY: a zeroed sigaction is whole, for the old action to fill.
        let mut was: libc::sigaction = unsafe { mem::zeroed() };
        // SAFETY: the old action is only asked for.
        if unsafe { libc::sigaction(signal, ptr::null(), &raw mut was) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if was.sa_sigaction == libc::SIG_IGN {
            continue;
        }
        // SAFETY: the action is whole, and the old one isn't asked for.
        if unsafe { libc::sigaction(signal, &raw const action, ptr::null_mut()) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(read)
}

/// The catch: the stop marked and its signal written for the thread
/// waiting on it, as little else is safe within a catch; or, for a stop
/// caught while one is under way, health ended by its signal as the catch
/// returns.
extern "C" fn heard(signal: c_int) {
    // SAFETY: errno is this thread's, and kept for the code the catch
    // interrupted, which may be about to read it.
    let kept = unsafe { *errno() };
    if STOPPED.swap(true, Ordering::SeqCst) {
        // SAFETY: both are safe within a catch; the signal's own action
        // restored and raised ends the process once the catch returns, as
        // the signal caught waits until then.
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    } else {
        let number = u8::try_from(signal).unwrap_or(0);
        // SAFETY: write is safe within a catch, and a pipe full only fails it.
        unsafe { libc::write(CAUGHT.load(Ordering::SeqCst), (&raw const number).cast(), 1) };
    }
    // SAFETY: as above.
    unsafe { *errno() = kept };
}

/// This thread's errno.
#[cfg(target_os = "linux")]
unsafe fn errno() -> *mut c_int {
    // SAFETY: always this thread's own.
    unsafe { libc::__errno_location() }
}

/// This thread's errno.
#[cfg(target_os = "macos")]
unsafe fn errno() -> *mut c_int {
    // SAFETY: always this thread's own.
    unsafe { libc::__error() }
}

/// End health by `signal`, as it would have ended had nothing caught it,
/// so whatever started health sees the same end.
fn die_by(signal: c_int) {
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
