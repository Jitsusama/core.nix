//! The guard every reader case holds its reader by: a case that fails
//! part way leaves no reader still running, and is shown its journal.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{PATIENCE, home, journal, reader, stand_in};

/// Set for the one run of `failing_while_held` that is meant to fail.
const HELD: &str = "PINEAPPLE_READER_FAILS_WHILE_HELD";

/// What the Slack reader says to its journal of whatever it keeps in
/// shadow (a stop, a reply, Joel's phone), so its own words are what a
/// failing case is shown.
const SAID: &str = ", in shadow: ";

/// How long a killed reader is given to end, which it does at once.
const KILLED: Duration = Duration::from_secs(1);

/// How `failing_while_held` fails, so its failure is told from any other.
const STAGED: &str = "this case fails on purpose, to be shown its journal";

#[test]
fn a_case_failing_part_way_leaves_no_reader_running() {
    let (origin, asked) = stand_in();
    let home = home();
    let running = reader(&home, &origin);
    let pid = running.id();
    asked_once(&asked);
    // Its state follows the parenthesised name; Z is one gone but unreaped.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let state = stat.rsplit_once(')').map(|(_, rest)| rest.trim_start());
    assert!(
        state.is_some_and(|state| !state.starts_with('Z')),
        "the reader runs before the case fails, not {stat:?}"
    );
    // Taken while the reader runs, so a stop reaches it and nothing else
    // that comes to hold its pid once it is reaped.
    let reader_itself = pidfd(pid);
    let (done, dropped) = mpsc::channel();
    thread::spawn(move || {
        let _ = catch_unwind(AssertUnwindSafe(move || {
            let _held = running;
            panic!("this case fails on purpose, to see its reader stopped");
        }));
        // The case gave up waiting on this one and has said so itself.
        let _ = done.send(());
    });
    // A guard that waits without killing waits for ever, one that does
    // neither leaves its reader running, and one that kills without
    // reaping leaves it lingering; the case stops and reaps the reader
    // itself before it says so, rather than leave it.
    if dropped.recv_timeout(PATIENCE).is_err() {
        // The guard is still waiting on it, and reaps it once it ends.
        stop(&reader_itself);
        panic!("the guard never stopped its reader");
    }
    // A kill lands a moment after it is sent, so a reader the guard
    // killed has that moment to end before it is told from one it left.
    ends_within(&reader_itself, KILLED);
    match left_as(&reader_itself) {
        Ok(Left::Reaped) => {}
        Ok(Left::Running) => {
            stop(&reader_itself);
            reap(&reader_itself);
            panic!("the failed case's reader still runs");
        }
        Ok(Left::Unreaped) => {
            reap(&reader_itself);
            panic!("the guard stopped its reader but never reaped it");
        }
        Err(why) => {
            stop(&reader_itself);
            panic!("the reader couldn't be asked how it was left: {why}");
        }
    }
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_case_failing_while_it_holds_its_reader_is_shown_its_journal() {
    let failing = Command::new(std::env::current_exe().expect("this test binary"))
        .args([
            "guard::failing_while_held",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(HELD, "1")
        .output()
        .expect("the failing case runs");
    let said = String::from_utf8_lossy(&failing.stderr);
    let heard = format!("{said}{}", String::from_utf8_lossy(&failing.stdout));
    assert!(
        !failing.status.success() && said.contains(STAGED),
        "the case didn't fail as staged:\n{heard}"
    );
    let mut lines = said.lines();
    let home = lines.by_ref().find_map(|line| {
        line.strip_prefix("what the readers in ")?
            .strip_suffix(" said to their journal:")
    });
    let home = home.unwrap_or_else(|| panic!("the case wasn't shown its journal:\n{heard}"));
    assert!(
        lines.any(|line| line.contains(SAID)),
        "the case was shown a journal without what its reader said:\n{heard}"
    );
    let home = Path::new(home);
    let ours = home.parent() == Some(std::env::temp_dir().as_path())
        && home
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("pineapple-reader-"));
    assert!(
        ours,
        "the case named a home not its own: {}",
        home.display()
    );
    std::fs::remove_dir_all(home).expect("the failed case's home goes");
}

/// What `a_case_failing_while_it_holds_its_reader_is_shown_its_journal`
/// runs in a test binary of its own, failing while it holds its reader.
/// Run any other way, even asked for by name, it does nothing.
#[test]
#[ignore = "run only by the journal case, which sees it fail"]
fn failing_while_held() {
    if std::env::var_os(HELD).is_none() {
        return;
    }
    let (origin, asked) = stand_in();
    let home = home();
    let mut held = reader(&home, &origin);
    asked_once(&asked);
    let began = Instant::now();
    while !journal(&home).contains(SAID) {
        assert!(
            began.elapsed() < PATIENCE,
            "the reader never said what it kept in shadow"
        );
        thread::sleep(Duration::from_millis(50));
    }
    // Stopped here, so only the stop case pins the guard's own stop.
    held.kill().expect("the reader stops");
    held.wait().expect("the stopped reader is reaped");
    panic!("{STAGED}");
}

/// Until the reader has asked the stand-in once, so it has started
/// rather than died on the way up.
fn asked_once(asked: &Mutex<Vec<String>>) {
    let began = Instant::now();
    while asked.lock().expect("what was asked").is_empty() {
        assert!(began.elapsed() < PATIENCE, "the reader never asked Slack");
        thread::sleep(Duration::from_millis(50));
    }
}

/// The process at `pid` itself, held so it can be stopped by what it
/// is rather than by a number the kernel may give out again.
fn pidfd(pid: u32) -> OwnedFd {
    let pid = libc::pid_t::try_from(pid).expect("a pid the kernel gave");
    // SAFETY: pidfd_open reads no memory, and a new fd or -1 comes back.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    let why = std::io::Error::last_os_error();
    let fd = i32::try_from(fd).expect("an fd");
    assert!(fd >= 0, "the reader couldn't be opened by its pid: {why}");
    // SAFETY: the fd was just opened here and nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
}

/// How a guard left its reader.
#[derive(Debug, PartialEq, Eq)]
enum Left {
    /// Still running.
    Running,
    /// Ended, but nobody reaped it, so it lingers as a zombie.
    Unreaped,
    /// Ended and reaped.
    Reaped,
}

/// How the reader was left, asked of the reader itself, and changing
/// nothing: this test binary is its parent, so the kernel answers with
/// its status while it lingers, and with ECHILD once it has been reaped.
fn left_as(reader: &OwnedFd) -> std::io::Result<Left> {
    match waited(reader, libc::WNOHANG | libc::WNOWAIT) {
        Ok(0) => Ok(Left::Running),
        Ok(_) => Ok(Left::Unreaped),
        Err(why) if why.raw_os_error() == Some(libc::ECHILD) => Ok(Left::Reaped),
        Err(why) => Err(why),
    }
}

/// Until the reader has ended, reaped or not, or `patience` is spent:
/// its pidfd reads ready once it has exited.
fn ends_within(reader: &OwnedFd, patience: Duration) {
    let mut asked = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let patience = libc::c_int::try_from(patience.as_millis()).expect("a short patience");
    // SAFETY: poll reads and writes only the one pollfd lent it here.
    // What it answers is left to `left_as`, which asks again.
    let _ = unsafe { libc::poll(&raw mut asked, 1, patience) };
}

/// Reaps a reader that has ended, or that the case has just stopped.
/// Nothing else waits on it by then: the guard has given it up.
fn reap(reader: &OwnedFd) {
    let reaped = waited(reader, 0);
    assert!(
        reaped.is_ok(),
        "the reader the guard left couldn't be reaped: {reaped:?}"
    );
}

/// Waits on the reader for its end with `options` besides WEXITED,
/// answering the pid it reported, 0 when it reported none.
fn waited(reader: &OwnedFd, options: libc::c_int) -> std::io::Result<libc::pid_t> {
    let fd = libc::id_t::try_from(reader.as_raw_fd()).expect("an fd is never negative");
    // SAFETY: an all-zero siginfo_t is a valid one.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // Zeroed first, as waitid(2) asks, so a WNOHANG with nothing to
    // report reads as pid 0.
    // SAFETY: waitid writes only the siginfo lent it here, and the fd is
    // the reader's own, open for as long as `reader`.
    let waited = unsafe { libc::waitid(libc::P_PIDFD, fd, &raw mut info, libc::WEXITED | options) };
    if waited < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: waitid filled the siginfo as a child's, or left it zeroed.
    Ok(unsafe { info.si_pid() })
}

/// Stops the reader its guard didn't; one already gone needs nothing.
fn stop(reader: &OwnedFd) {
    let siginfo = std::ptr::null::<libc::siginfo_t>();
    // SAFETY: a null siginfo asks for an ordinary kill, and no memory is
    // read; the fd is the reader's own, open for as long as `reader`.
    let sent = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            reader.as_raw_fd(),
            libc::SIGKILL,
            siginfo,
            0,
        )
    };
    let why = std::io::Error::last_os_error();
    assert!(
        sent == 0 || why.raw_os_error() == Some(libc::ESRCH),
        "the reader the guard left couldn't be stopped: {why}"
    );
}
