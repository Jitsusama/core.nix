//! The nested desktop: started in a process group of its own, ended with
//! everything it started, and the signals that end a preview heard.
//!
//! What ends a preview comes to one channel: the desktop's window closing,
//! as its group's leader ends, or a stop signal, heard by one thread with
//! `sigwait` while every other thread has the stops blocked. Whichever
//! comes first, the whole group is ended, so a preview stopped from its
//! terminal leaves no desktop running.

use std::io;
use std::mem::MaybeUninit;
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

/// The signals that end a preview: Ctrl-C, a stop, and the terminal or the
/// caller's shell going away.
pub const STOPS: [i32; 3] = [libc::SIGINT, libc::SIGTERM, libc::SIGHUP];

/// What ended a preview.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// The desktop's leader ended, its window closed, with this status.
    Closed(i32),
    /// The preview was sent this signal.
    Stopped(i32),
}

impl Ended {
    /// Give what the preview exits with: the desktop's status, or what a
    /// shell gives for a program a signal ended.
    pub fn code(&self) -> i32 {
        match self {
            Self::Closed(status) => *status,
            Self::Stopped(signal) => 128 + signal,
        }
    }
}

/// Block the stop signals in this thread and in every thread it starts
/// after, and hear them on a thread of their own, each sent on as
/// `Stopped`. Call it before starting any other thread, or a thread that
/// doesn't block them takes them and dies. A program started after keeps
/// them blocked, unless it's started `unblocked`.
pub fn hear_stops(ended: Sender<Ended>) -> io::Result<()> {
    let stops = signal_set(&STOPS)?;
    // SAFETY: the set is initialised, and the old mask isn't asked for.
    let blocked =
        unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &raw const stops, std::ptr::null_mut()) };
    if blocked != 0 {
        return Err(io::Error::from_raw_os_error(blocked));
    }
    thread::Builder::new()
        .name("stops".to_owned())
        .spawn(move || {
            loop {
                let mut heard = 0;
                // SAFETY: the set is initialised and `heard` is written by sigwait.
                if unsafe { libc::sigwait(&raw const stops, &raw mut heard) } == 0
                    && ended.send(Ended::Stopped(heard)).is_err()
                {
                    return;
                }
            }
        })?;
    Ok(())
}

/// Have the command's program start with the stop signals unblocked, as a
/// program a shell starts does, so the desktop ends when asked and a
/// Ctrl-C at the terminal reaches git and nix as well as the preview.
pub fn unblocked(command: &mut Command) -> &mut Command {
    use std::os::unix::process::CommandExt;
    let unblock = || {
        let stops = signal_set(&STOPS)?;
        // SAFETY: the set is initialised, and the old mask isn't asked for.
        let done = unsafe {
            libc::pthread_sigmask(libc::SIG_UNBLOCK, &raw const stops, std::ptr::null_mut())
        };
        if done == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(done))
        }
    };
    // SAFETY: between fork and exec it calls only sigemptyset, sigaddset and
    // pthread_sigmask, each safe to call there, and allocates nothing.
    unsafe { command.pre_exec(unblock) }
}

/// Give a signal set holding those signals.
fn signal_set(signals: &[i32]) -> io::Result<libc::sigset_t> {
    let mut set = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: sigemptyset initialises the set it's given, and sigaddset
    // adds to an initialised one.
    unsafe {
        if libc::sigemptyset(set.as_mut_ptr()) != 0 {
            return Err(io::Error::last_os_error());
        }
        for &signal in signals {
            if libc::sigaddset(set.as_mut_ptr(), signal) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(set.assume_init())
    }
}

/// A desktop in a process group of its own, which its leader names.
pub struct Desktop {
    group: i32,
}

impl Desktop {
    /// Start the command as a desktop, leading a process group of its own,
    /// and send `Closed` once its leader ends.
    pub fn start(command: &mut Command, ended: Sender<Ended>) -> io::Result<Self> {
        use std::os::unix::process::CommandExt;
        let leader = unblocked(command).process_group(0).spawn()?;
        let group = i32::try_from(leader.id()).map_err(io::Error::other)?;
        reap(leader, ended)?;
        Ok(Self { group })
    }

    /// End every process in the desktop's group: asked, then killed once
    /// `grace` runs out. The whole group, since dbus-run-session passes no
    /// signal on to the niri it runs.
    pub fn end(&self, grace: Duration) {
        for stop in [libc::SIGTERM, libc::SIGKILL] {
            if !present(self.group) {
                return;
            }
            // SAFETY: kill only signals the processes in the group.
            unsafe { libc::kill(-self.group, stop) };
            if emptied(self.group, grace) {
                return;
            }
        }
    }
}

/// Wait for the leader on a thread of its own, so it's reaped as it ends,
/// since a leader ended but not reaped still counts as in its group.
fn reap(mut leader: Child, ended: Sender<Ended>) -> io::Result<()> {
    thread::Builder::new()
        .name("desktop".to_owned())
        .spawn(move || {
            use std::os::unix::process::ExitStatusExt;
            let status = leader.wait().map_or(1, |status| {
                status
                    .code()
                    .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
            });
            let _ = ended.send(Ended::Closed(status));
        })?;
    Ok(())
}

/// Say whether any process is left in a process group.
fn present(group: i32) -> bool {
    // SAFETY: signal 0 only asks whether the group can be signalled.
    let asked = unsafe { libc::kill(-group, 0) };
    // One there that can't be signalled is there all the same.
    asked == 0 || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

/// Wait up to `grace` for a group to empty, and say whether it did. The
/// group gives no word when it empties, so it's looked at.
fn emptied(group: i32, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    while present(group) {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Start a stand-in desktop: a shell, running as the rest of its line
    /// says, and a sleep it started, as dbus-run-session runs niri. It's
    /// given once the shell has said it's ready, its trap set.
    fn stand_in(test: &str, script: &str) -> (Desktop, mpsc::Receiver<Ended>) {
        let ready =
            std::env::temp_dir().join(format!("shell-preview-{}-{test}", std::process::id()));
        let _ = std::fs::remove_file(&ready);
        let (ended, heard) = mpsc::channel();
        let line = format!("{script} sleep 300 & touch {}; wait", ready.display());
        let desktop = Desktop::start(Command::new("sh").args(["-c", &line]), ended)
            .expect("a stand-in desktop");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                Instant::now() < deadline,
                "the stand-in desktop never said it was ready"
            );
            thread::sleep(Duration::from_millis(10));
        }
        (desktop, heard)
    }

    #[test]
    fn ending_asks_the_desktop_and_all_it_started_to_end() {
        let (desktop, heard) = stand_in("asked", "");
        let asked = Instant::now();

        desktop.end(Duration::from_secs(5));

        assert!(!present(desktop.group));
        assert!(
            asked.elapsed() < Duration::from_secs(5),
            "it was killed, not asked"
        );
        assert_eq!(heard.recv(), Ok(Ended::Closed(128 + libc::SIGTERM)));
    }

    #[test]
    fn a_desktop_that_ignores_being_asked_is_killed() {
        let (desktop, heard) = stand_in("ignoring", "trap '' TERM;");

        desktop.end(Duration::from_millis(300));

        assert!(!present(desktop.group));
        assert_eq!(heard.recv(), Ok(Ended::Closed(128 + libc::SIGKILL)));
    }

    #[test]
    fn a_desktop_that_closed_is_closed_with_its_status() {
        let (ended, heard) = mpsc::channel();
        let desktop = Desktop::start(Command::new("sh").args(["-c", "exit 3"]), ended)
            .expect("a stand-in desktop");

        assert_eq!(heard.recv(), Ok(Ended::Closed(3)));
        desktop.end(Duration::from_secs(5));
        assert!(!present(desktop.group));
    }

    #[test]
    fn a_preview_stopped_exits_as_a_shell_says_a_signal_ended_it() {
        assert_eq!(Ended::Stopped(libc::SIGTERM).code(), 143);
        assert_eq!(Ended::Closed(0).code(), 0);
    }
}
