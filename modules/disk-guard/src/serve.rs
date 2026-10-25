//! Running the daemons: one per scope, stepping until told to stop, and
//! the system daemon answering over its socket for the processes a user
//! daemon can't read, and hearing user daemons that drive its passes.

use crate::daemon::{Guard, Hearing, Senses};
use crate::moments::Power;
use crate::notices::Change;
use crate::socket::{Reply, Request, Server};
use crate::vouch;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

/// Takes the lock that keeps one daemon to a state directory, or None
/// while another daemon holds it. It's held as long as the file is open,
/// and the kernel lets it go when the daemon dies however it dies.
pub fn hold(state: &Path) -> io::Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(state.join("lock"))?;
    match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(Some(file)),
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Answers on `socket`, from a thread of its own, each account asking each
/// kind of request at most once `every`: which processes read from `proc`
/// touch what the asker owns; which of the filesystems a user daemon
/// watches hold the store, on device `store`, noting that it spoke for
/// them; and a collect there, handed to the guard through what it hears.
pub fn answering(
    socket: &Path,
    proc: &Path,
    every: Duration,
    store: Option<u64>,
) -> io::Result<Hearing> {
    let mut server = Server::bind(socket, every)?;
    let proc: PathBuf = proc.to_owned();
    let spoken = Arc::new(Mutex::new(HashMap::new()));
    let (passing, collects) = mpsc::channel();
    let hearing = Hearing {
        spoken: Arc::clone(&spoken),
        collects,
    };
    let device = |path: &Path| crate::pool::pool(path).ok();
    std::thread::Builder::new()
        .name("answering".to_owned())
        .spawn(move || {
            loop {
                let answered = server.serve(&mut |uid, request, responder| {
                    let reply = match request {
                        Request::Processes { roots } => match crate::sys::processes(&proc) {
                            Ok(processes) => {
                                Reply::Answer(vouch::answer(&processes, &roots, |path| {
                                    vouch::reached_by(path, uid)
                                }))
                            }
                            Err(error) => Reply::Error(error.to_string()),
                        },
                        Request::Watching { paths } => {
                            let held: Vec<PathBuf> = paths
                                .into_iter()
                                .filter(|path| store.is_some() && device(path) == store)
                                .collect();
                            if let (Some(store), false) = (store, held.is_empty()) {
                                let mut spoken =
                                    spoken.lock().unwrap_or_else(PoisonError::into_inner);
                                spoken.insert(store, std::time::Instant::now());
                            }
                            Reply::Collects(held)
                        }
                        Request::Collect { path, band } => {
                            if store.is_none() || device(&path) != store {
                                Reply::Error(format!(
                                    "no garbage is collected on {}",
                                    path.display()
                                ))
                            } else {
                                // Answered by the guard once its pass ends.
                                if let Err(mpsc::SendError((_, _, responder))) =
                                    passing.send((path, band, responder))
                                {
                                    responder
                                        .send(&Reply::Error("the guard has stopped".to_owned()));
                                }
                                return;
                            }
                        }
                    };
                    responder.send(&reply);
                });
                if let Err(error) = answered {
                    // Out of descriptors, most likely; a moment may free one.
                    eprintln!("disk-guard: answering: {error}");
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        })?;
    Ok(hearing)
}

/// Why the desktop last refused a notice, so a refusal said once isn't
/// said again on every pass: a session with no notification server would
/// otherwise fill its journal with it.
#[derive(Debug, Default)]
pub struct Unheard {
    last: Option<String>,
}

impl Unheard {
    /// What to say of how the desktop took a notice, if anything.
    pub fn said(&mut self, told: Result<(), String>) -> Option<String> {
        let why = told.err();
        let changed = why != self.last;
        self.last.clone_from(&why);
        why.filter(|_| changed)
    }
}

/// Steps `guard` until `wait`, given how long to wait, says to stop. A
/// step that fails is told of on `err` and the next goes on, since what
/// failed (a filesystem gone, a full disk refusing the status file) may
/// not fail next time, and a guard that stopped would guard nothing.
pub fn steps(
    guard: &mut Guard,
    every: Duration,
    mut wait: impl FnMut(Duration) -> bool,
    desk: &mut dyn Desk,
    err: &mut dyn Write,
) {
    let mut unheard = Unheard::default();
    loop {
        let stepped = crate::sys::tick().and_then(|tick| {
            let before = guard.status(tick.wall);
            guard.step(&tick, &senses(desk.saving()))?;
            let change = crate::notices::change(&before, &guard.status(tick.wall));
            if let Some(change) = change
                && let Some(why) = unheard.said(desk.tell(&change))
            {
                let _ = writeln!(err, "disk-guard: telling the desktop: {why}");
            }
            Ok(())
        });
        if let Err(error) = stepped {
            let _ = writeln!(err, "disk-guard: {error}");
        }
        // What the step built and dropped isn't kept while it waits.
        crate::sys::give_back();
        if wait(every) {
            // The journal shows how long after systemd's stop signal the
            // guard took it, which a step can hold up.
            let _ = writeln!(err, "disk-guard: stopping");
            return;
        }
    }
}

/// Blocks `signals` in this thread, and in every thread it starts after,
/// and gives a wait that ends early, saying so, once one of them comes.
/// A signal during a step waits for the step to finish, so a pass is
/// never cut off halfway by a stop. Call it before starting any thread,
/// or a thread that doesn't block them takes them and dies.
#[cfg(target_os = "linux")]
pub fn stopping(signals: &[i32]) -> io::Result<impl FnMut(Duration) -> bool + use<>> {
    let set = block(signals)?;
    Ok(move |wait: Duration| {
        let deadline = std::time::Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let timeout = timespec(left);
            // SAFETY: set and timeout are valid for the call; no siginfo
            // is asked for.
            let taken = unsafe {
                libc::sigtimedwait(&raw const set, std::ptr::null_mut(), &raw const timeout)
            };
            if taken > 0 {
                return true;
            }
            // Woken by another signal, wait out the rest; timed out, done.
            if io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) || left.is_zero() {
                return false;
            }
        }
    })
}

/// The same on macOS, which has no sigtimedwait: a kqueue records each
/// signal sent, blocked or not, and waits with a timeout, and sigwait then
/// takes the signal, which is pending by then. One that came before the
/// queue could hear it is pending already.
#[cfg(target_os = "macos")]
pub fn stopping(signals: &[i32]) -> io::Result<impl FnMut(Duration) -> bool + use<>> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let set = block(signals)?;
    // SAFETY: kqueue takes nothing and returns a new descriptor.
    let queue = unsafe { libc::kqueue() };
    if queue < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: queue is a new descriptor nothing else owns.
    let queue = unsafe { OwnedFd::from_raw_fd(queue) };
    let heard: Vec<libc::kevent> = signals
        .iter()
        .map(|&signal| {
            Ok(libc::kevent {
                ident: libc::uintptr_t::try_from(signal).map_err(io::Error::other)?,
                filter: libc::EVFILT_SIGNAL,
                flags: libc::EV_ADD,
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            })
        })
        .collect::<io::Result<_>>()?;
    let count = libc::c_int::try_from(heard.len()).map_err(io::Error::other)?;
    // SAFETY: heard is a list of that many kevents, and no events are
    // asked for back.
    let added = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            heard.as_ptr(),
            count,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    };
    if added < 0 {
        return Err(io::Error::last_os_error());
    }
    let signals = signals.to_vec();
    Ok(move |wait: Duration| {
        if take_pending(set, &signals) {
            return true;
        }
        let deadline = std::time::Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let timeout = timespec(left);
            let mut event = std::mem::MaybeUninit::<libc::kevent>::zeroed();
            // SAFETY: event has room for the one event asked for, and
            // timeout is valid for the call.
            let got = unsafe {
                libc::kevent(
                    queue.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    event.as_mut_ptr(),
                    1,
                    &raw const timeout,
                )
            };
            if got > 0 {
                // Sent, so it's pending now: taken, so the next wait
                // doesn't hear it again.
                take_pending(set, &signals);
                return true;
            }
            if got == 0 || left.is_zero() {
                return false;
            }
            if io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return false;
            }
        }
    })
}

/// Takes one of `signals` if one is pending here, saying whether it did.
#[cfg(target_os = "macos")]
fn take_pending(set: libc::sigset_t, signals: &[i32]) -> bool {
    // SAFETY: waiting is a sigset_t sigpending fills in; both sets are
    // then only read, and sigwait returns at once for a pending signal.
    unsafe {
        let mut waiting: libc::sigset_t = std::mem::zeroed();
        if libc::sigpending(&raw mut waiting) != 0 {
            return false;
        }
        if !signals
            .iter()
            .any(|&signal| libc::sigismember(&raw const waiting, signal) == 1)
        {
            return false;
        }
        let mut taken = 0;
        libc::sigwait(&raw const set, &raw mut taken) == 0
    }
}

/// A span as the kernel takes one, the longest it holds standing in for
/// any longer.
fn timespec(span: Duration) -> libc::timespec {
    libc::timespec {
        tv_sec: libc::time_t::try_from(span.as_secs()).unwrap_or(libc::time_t::MAX),
        tv_nsec: libc::c_long::from(span.subsec_nanos()),
    }
}

/// Blocks `signals` in this thread and every thread it starts after, so
/// that only the wait takes them.
fn block(signals: &[i32]) -> io::Result<libc::sigset_t> {
    // SAFETY: set is a sigset_t the calls fill in and then only read.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&raw mut set);
        for &signal in signals {
            libc::sigaddset(&raw mut set, signal);
        }
        let blocked = libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
        if blocked != 0 {
            return Err(io::Error::from_raw_os_error(blocked));
        }
        Ok(set)
    }
}

/// The desktop a daemon answers to: whether its person asked to save
/// power, and where its notice goes.
pub trait Desk {
    fn saving(&mut self) -> bool;
    fn tell(&mut self, change: &Change) -> Result<(), String>;
}

/// What the machine is doing, with `saver` saying whether its person
/// asked to save power. What can't be read is taken as least in the way:
/// a machine with no supplies to read runs on mains, and no stall to read
/// is no stall.
pub fn senses(saver: bool) -> Senses {
    Senses {
        power: crate::sys::power(Path::new("/sys/class/power_supply")).unwrap_or(Power {
            mains: true,
            battery: None,
        }),
        saver,
        pressure: crate::sys::pressure(Path::new("/proc/pressure")).unwrap_or_default(),
    }
}
