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

/// What the Slack reader says to its journal of each stop it keeps in
/// shadow, so its own words are what a failing case is shown.
const SAID: &str = ", in shadow: ";

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
    // A guard that waits without killing waits for ever, and one that
    // does neither leaves its reader running; either way, the case stops
    // the reader itself before it says so, rather than leave it.
    let stopped = dropped.recv_timeout(PATIENCE).is_ok();
    let gone = !Path::new(&format!("/proc/{pid}")).exists();
    if !gone || !stopped {
        stop(&reader_itself);
    }
    assert!(stopped, "the guard never stopped its reader");
    assert!(gone, "the failed case's reader still runs");
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
        assert!(began.elapsed() < PATIENCE, "the reader never kept a stop");
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
    let fd = i32::try_from(fd).expect("an fd");
    assert!(fd >= 0, "the reader was opened by its pid");
    // SAFETY: the fd was just opened here and nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
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
