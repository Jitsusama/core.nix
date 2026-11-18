//! The guard every reader case holds its reader by: a case that fails
//! part way leaves no reader still running, and is shown its journal.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{PATIENCE, home, reader, stand_in};

/// Set for the one run of `failing_while_held` that is meant to fail.
const HELD: &str = "PINEAPPLE_READER_FAILS_WHILE_HELD";

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
    let (done, stopped) = mpsc::channel();
    thread::spawn(move || {
        let failed = catch_unwind(AssertUnwindSafe(move || {
            let _held = running;
            panic!("this case fails on purpose, to see its reader stopped");
        }));
        // The case gave up waiting on this one and has said so itself.
        let _ = done.send(failed.is_err());
    });
    let failed = stopped.recv_timeout(PATIENCE).unwrap_or_else(|_| {
        // A guard that waits without killing waits for ever; stop its
        // reader here rather than leave it running.
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
        panic!("the guard never stopped its reader");
    });
    assert!(failed, "the case failed");
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the failed case's reader still runs"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_case_failing_while_it_holds_its_reader_is_shown_its_journal() {
    let failing = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["guard::failing_while_held", "--exact", "--nocapture"])
        .env(HELD, "1")
        .output()
        .expect("the failing case runs");
    assert!(!failing.status.success(), "the case failed");
    let said = String::from_utf8_lossy(&failing.stderr);
    let home = said.lines().find_map(|line| {
        line.strip_prefix("what the readers in ")?
            .strip_suffix(" said to their journal:")
    });
    let home = home.unwrap_or_else(|| panic!("the case wasn't shown its journal:\n{said}"));
    std::fs::remove_dir_all(home).expect("the failed case's home goes");
}

/// Not a case of its own: what the case above runs in a test binary of
/// its own, failing while it holds its reader. Run any other way, it
/// does nothing.
#[test]
fn failing_while_held() {
    if std::env::var_os(HELD).is_none() {
        return;
    }
    let (origin, asked) = stand_in();
    let home = home();
    let mut held = reader(&home, &origin);
    asked_once(&asked);
    // Stopped here, so only the case above pins the guard's own stop.
    held.kill().expect("the reader stops");
    held.wait().expect("the stopped reader is reaped");
    panic!("this case fails on purpose, to be shown its journal");
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
