//! A guard whose step never asks after a stop, told to stop as systemd
//! tells one, ends by that signal within its patience. It's a program of
//! its own with no test harness, since a harness's threads leave the
//! signal open where the guard's watch isn't, and one of them would take
//! it instead: the guard blocks its stops before any thread starts.

use std::io::BufRead;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Set in the child, which stands in for the guard.
const HELD: &str = "DISK_GUARD_TEST_HELD_STEP";

fn main() {
    if std::env::var_os(HELD).is_some() {
        held_step();
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .env(HELD, "1")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut said = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    assert!(
        said.read_line(&mut line).unwrap() > 0,
        "the guard never stepped"
    );
    let told = Instant::now();
    let pid = libc::pid_t::try_from(child.id()).unwrap();
    // SAFETY: kill takes a pid and a signal and touches no memory.
    assert_eq!(
        unsafe { libc::kill(pid, libc::SIGTERM) },
        0,
        "it couldn't be told"
    );
    let ended = loop {
        if let Some(ended) = child.try_wait().unwrap() {
            break ended;
        }
        if told.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            panic!("the step held the stop past 10 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let took = told.elapsed();
    assert_eq!(
        ended.signal(),
        Some(libc::SIGTERM),
        "it ended otherwise: {ended:?}"
    );
    assert!(took < Duration::from_secs(2), "it took {took:?} to stop");
}

/// Blocks its stops as the daemon does, before any thread, starts the
/// watch, then steps for longer than any stop would wait.
fn held_step() {
    let _wait = disk_guard::serve::stopping(&[libc::SIGTERM]).unwrap();
    disk_guard::serve::leave_when_held_up(&[libc::SIGTERM], std::io::sink()).unwrap();
    println!("stepping");
    // The step that never asks: longer than the parent waits, so only the
    // watch letting the stop through ends it in time.
    std::thread::sleep(Duration::from_secs(30));
}
