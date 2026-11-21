//! Health's end by a signal, as when a `health follow` run by hand is
//! killed or its terminal closes. Health's followers are its own children,
//! and a signal that ends health unhandled ends none of them, each living
//! on until its next line finds no one reading. So the signals that ask
//! health to stop are caught, and a thread waiting on what the catch writes
//! has the followers ended, gives them and health's run `WRITES_WITHIN`
//! from the stop to end and to write the day and return, then ends health
//! by the same signal, so whatever started health sees the stop it asked
//! for. A service manager's stop ends health's whole group anyway, but only
//! this writes what health held unwritten, as on a restart or a reboot.
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
//! A second stop before the first has ended health ends it at once, as
//! one who interrupts twice asks, saying what it cut short: the day's
//! write, or once the run has returned, the followers' end. A hangup during
//! a stop is the same stop, though: a terminal closing sends one from its
//! shell and another from the kernel, and a session's scope stopped sends
//! one just after its stop. The first stop's `WRITES_WITHIN` bounds it all
//! the same, counted from the catch, whether or not the followers end, so
//! a stop can never hang. Health ends no sooner than its followers, but
//! for one stuck past its kill.

use std::io::{self, PipeReader, Read as _};
use std::mem;
use std::os::fd::{AsRawFd as _, IntoRawFd as _, OwnedFd};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

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

/// How `until_stopped`'s run has returned, as `CUT_SHORT`'s index, for a
/// catch, which can't take `RAN`'s lock, to read: not yet, well or with
/// its error said, or with an error kept unsaid.
static RETURNED: AtomicUsize = AtomicUsize::new(0);

/// Whether stderr is the journal's stream, kept before the stops are
/// caught, so a catch marks its line as `source` marks every other.
static JOURNAL: AtomicBool = AtomicBool::new(false);

/// What a second stop says as it ends health, by `RETURNED`, written
/// within its catch, where nothing that formats is safe, so whoever
/// interrupted twice knows what it cost. Each opens with sd-daemon(3)'s
/// mark, `MARKED` long, cut where stderr isn't the journal: a failure for
/// a write or an error given up, a warning for followers not waited for.
const CUT_SHORT: [&[u8]; 3] = [
    b"<3>health: a second stop ended health at once, not waiting for the day's write\n",
    b"<4>health: a second stop ended health at once, its run returned but its followers' end \
      not waited for\n",
    b"<3>health: a second stop ended health at once, not waiting to say the error its run \
      returned\n",
];

/// How long each of `CUT_SHORT`'s marks is.
const MARKED: usize = 3;

const _: () = assert!(
    marked(CUT_SHORT[0], crate::source::ERROR)
        && marked(CUT_SHORT[1], crate::source::WARNING)
        && marked(CUT_SHORT[2], crate::source::ERROR),
    "a second stop's line isn't marked whole as source marks its own"
);

/// Whether `line` opens with sd-daemon(3)'s whole mark for `mark`, as
/// the catch cuts `MARKED` bytes from it where stderr isn't the journal.
const fn marked(line: &[u8], mark: u8) -> bool {
    line.len() > MARKED && line[0] == b'<' && line[1] == b'0' + mark && line[2] == b'>'
}

/// How long a follower that ends waits for health to catch a stop before
/// it says so: a stop to health's whole group, as a service manager's,
/// reaches health and its followers at once, and a follower may end of it
/// before health's catch has run, by the signal or, as journalctl does,
/// cleanly. A follower's end is a fault health waits out for ten minutes
/// anyway, so a quarter second costs nothing, and leaves a busy machine
/// room to run the catch.
pub const CAUGHT_WITHIN: Duration = Duration::from_millis(250);

/// How long a stop waits for the followers to end and `run` to return,
/// so it writes the day, before it ends health all the same: the
/// followers' kill and a write take milliseconds. A stop gives longer by
/// default, systemd 90 seconds and launchd 20, but a reboot gives the
/// account's services 5 in all (`modules/systemd/nixos.nix`), counted
/// from before health hears of it, so this stays well under that, and a
/// write that doesn't make it is said before the kill. It counts from the
/// stop's catch, the followers ended beside the run's write, so one that
/// won't end, stuck on a stalled disk, can't hold the stop.
pub(crate) const WRITES_WITHIN: Duration = Duration::from_secs(3);

/// Whether `run` has returned, and the stop's thread and `rest` woken as
/// it does or as a stop is caught.
static RAN: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

/// Whether a stop's `end` has returned, its followers ended, and the
/// stop's thread woken as it does.
static ENDED: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

/// The error `until_stopped`'s run returned, kept for a stop caught
/// before `main` says it, as health then ends by its signal and `main`
/// never does: the stop says it once the run has returned, as `main` may
/// have too should the stop come just after.
static UNSAID: Mutex<Option<String>> = Mutex::new(None);

/// `run`, with health's stops caught: a stop calls `end`, on a thread of
/// its own, waits up to `WRITES_WITHIN` for it and `run` to return, then
/// ends health by its signal. Once one is caught, health ends by it
/// alone, whatever `run` returns, saying the error it returned (`UNSAID`).
///
/// # Errors
///
/// When the pipe can't be made, the signals caught or the stop's threads
/// started, or when the signals already are caught, or what `run` returns.
pub fn until_stopped<T>(
    end: impl FnOnce() + Send + 'static,
    run: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    ending(end)?;
    let ran = run();
    if let Err(error) = &ran {
        *UNSAID.lock().unwrap_or_else(PoisonError::into_inner) = Some(format!("health: {error}"));
    }
    RETURNED.store(if ran.is_ok() { 1 } else { 2 }, Ordering::SeqCst);
    mark(&RAN);
    await_stop();
    ran
}

/// From now on, a signal that asks health to stop calls `end`, on a
/// thread of its own, wakes a `rest`, waits up to `WRITES_WITHIN` for
/// `end` and `until_stopped`'s run to return, then ends health by that
/// signal. A second while the first's runs ends health at once, unless
/// it's a hangup, the same stop. Both threads start now rather than as a
/// stop comes, so a stop never waits on a thread it may be refused.
fn ending(end: impl FnOnce() + Send + 'static) -> io::Result<()> {
    let journal = terminal_kit::Environment::here().journal;
    JOURNAL.store(journal, Ordering::SeqCst);
    let mut caught = catch(&STOPPING)?;
    let ender = thread::Builder::new().spawn(move || {
        while !stopping() {
            thread::park();
        }
        end();
        mark(&ENDED);
    })?;
    thread::Builder::new().spawn(move || {
        let mut signal = [0];
        if caught.read_exact(&mut signal).is_ok() {
            let by = Instant::now() + WRITES_WITHIN;
            ender.thread().unpark();
            // Under its lock, so a `rest` that hasn't seen the stop yet is
            // waiting by now.
            let (ran, resting) = &RAN;
            let held = ran.lock().unwrap_or_else(PoisonError::into_inner);
            resting.notify_all();
            drop(held);
            // The run first, so its error is said as soon as it's known,
            // before a reboot's kill can land on a follower stuck past its
            // own: both waits end by the same `by` anyway.
            if waited(&RAN, by) {
                let unsaid = UNSAID.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(words) = unsaid {
                    crate::source::fail(&words);
                }
                // Said, so a second stop now costs only the followers' end.
                RETURNED.store(1, Ordering::SeqCst);
            } else {
                crate::source::fail(&format!(
                    "health: a stop ended health before it wrote the day, as its run hadn't \
                     returned {} seconds on",
                    WRITES_WITHIN.as_secs()
                ));
            }
            if !waited(&ENDED, by) {
                crate::source::fail(&format!(
                    "health: a stop left a follower running {} seconds on, as its kill hadn't \
                     ended it",
                    WRITES_WITHIN.as_secs()
                ));
            }
            die_by(c_int::from(signal[0]));
        }
    })?;
    Ok(())
}

/// Set `flag`, waking whatever waits on it.
fn mark(flag: &(Mutex<bool>, Condvar)) {
    let (set, woken) = flag;
    *set.lock().unwrap_or_else(PoisonError::into_inner) = true;
    woken.notify_all();
}

/// Wait for `flag` to be set until `by`, saying whether it was.
fn waited(flag: &(Mutex<bool>, Condvar), by: Instant) -> bool {
    let (set, woken) = flag;
    let set = set.lock().unwrap_or_else(PoisonError::into_inner);
    let left = by.saturating_duration_since(Instant::now());
    let (set, _) = woken
        .wait_timeout_while(set, left, |set| !*set)
        .unwrap_or_else(PoisonError::into_inner);
    *set
}

/// Wait `time`, or less once a stop is caught, so a run resting between
/// its looks writes the day and returns within the stop's
/// `WRITES_WITHIN`.
pub fn rest(time: Duration) {
    let (returned, woken) = &RAN;
    let returned = returned.lock().unwrap_or_else(PoisonError::into_inner);
    drop(woken.wait_timeout_while(returned, time, |_| !stopping()));
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
/// returns, saying so, unless it's a hangup, the same stop.
extern "C" fn heard(signal: c_int) {
    // SAFETY: errno is this thread's, and kept for the code the catch
    // interrupted, which may be about to read it.
    let kept = unsafe { *errno() };
    if STOPPED.swap(true, Ordering::SeqCst) {
        if signal == libc::SIGHUP {
            // SAFETY: errno is this thread's, put back as it was.
            unsafe { *errno() = kept };
            return;
        }
        let said = CUT_SHORT[RETURNED.load(Ordering::SeqCst)];
        // Marked only where the journal reads the mark.
        let said = &said[if JOURNAL.load(Ordering::SeqCst) {
            0
        } else {
            MARKED
        }..];
        // SAFETY: all three are safe within a catch; the signal's own
        // action restored and raised ends the process once the catch
        // returns, as the signal caught waits until then. A stderr that
        // fails only goes unsaid.
        unsafe {
            libc::write(libc::STDERR_FILENO, said.as_ptr().cast(), said.len());
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
    // Only reached when the signal didn't end health: the table's end for
    // an interrupt, the kit's way out of a thread that heard a signal.
    terminal_kit::Exit::Interrupted.end_now();
}

#[cfg(test)]
mod tests;
