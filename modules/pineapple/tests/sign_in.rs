//! A reader's sign-in: what the program prints, and never a wait for good.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::sign_in::{SignIn, token};

/// The patience a case cuts a sign-in off at.
const SHORT: Duration = Duration::from_millis(300);

/// The patience a case that expects an answer gives: far past any program
/// here that answers, however loaded the machine.
const AMPLE: Duration = Duration::from_secs(20);

/// How far past its patience a case waits before calling it a hang.
const HANG: Duration = Duration::from_secs(5);

/// Taken by each case for its whole run. A program written while another
/// case's thread forks can be held open for writing in that child, and
/// running it then fails with ETXTBSY; one case at a time can't race so.
static TURN: Mutex<()> = Mutex::new(());

/// This case's turn, kept however an earlier case ended.
fn turn() -> MutexGuard<'static, ()> {
    TURN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A folder of the case's own.
fn folder(case: &str) -> PathBuf {
    let at = std::env::temp_dir().join(format!("pineapple-sign-in-{case}-{}", std::process::id()));
    std::fs::create_dir_all(&at).expect("the case's folder is made");
    at
}

/// A sign-in program in `at` that runs `script`.
fn program(at: &Path, script: &str) -> PathBuf {
    let path = at.join("sign-in");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).expect("the program written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the program runs");
    path
}

/// What `token` answers for `path` with `patience`, and how long it took,
/// or a failure naming the hang when it hasn't answered well past it.
fn asked(path: PathBuf, patience: Duration) -> (SignIn, Duration) {
    let (said, heard) = mpsc::channel();
    let began = Instant::now();
    thread::spawn(move || {
        // The case may have given up, and then nobody listens.
        let _ = said.send(token(path.as_os_str(), patience));
    });
    let answer = heard
        .recv_timeout(patience + HANG)
        .expect("the sign-in is never waited on for good");
    (answer, began.elapsed())
}

/// What a sign-in running `script` gives with `patience`, its folder gone.
fn signed(case: &str, script: &str, patience: Duration) -> (SignIn, Duration) {
    let at = folder(case);
    let answer = asked(program(&at, script), patience);
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    answer
}

#[test]
fn sign_in_prints_the_token_trimmed() {
    let _turn = turn();
    let (answer, _) = signed("prints", "echo '  stand-in-token  '", AMPLE);
    assert_eq!(
        answer,
        SignIn::Token("stand-in-token".into()),
        "the printed token"
    );
}

#[test]
fn sign_in_slow_but_in_time_gives_its_token() {
    let _turn = turn();
    let (answer, _) = signed("slow", "sleep 0.3; echo stand-in-token", AMPLE);
    assert_eq!(
        answer,
        SignIn::Token("stand-in-token".into()),
        "a slow token is taken"
    );
}

#[test]
fn sign_in_that_ends_after_its_output_gives_its_token() {
    let _turn = turn();
    let script = "echo stand-in-token; exec >&-; sleep 0.3";
    let (answer, _) = signed("ends-later", script, AMPLE);
    assert_eq!(
        answer,
        SignIn::Token("stand-in-token".into()),
        "waited for to its end"
    );
}

#[test]
fn sign_in_that_fails_gives_no_token() {
    let _turn = turn();
    let (answer, _) = signed("fails", "echo stand-in-token; exit 1", AMPLE);
    assert_eq!(
        answer,
        SignIn::Missing,
        "a failed sign-in's words aren't taken"
    );
}

#[test]
fn sign_in_that_prints_nothing_gives_no_token() {
    let _turn = turn();
    let (answer, _) = signed("nothing", "true", AMPLE);
    assert_eq!(answer, SignIn::Missing, "nothing printed is no token");
}

#[test]
fn sign_in_that_cant_be_run_gives_no_token() {
    let _turn = turn();
    let at = folder("absent");
    let (answer, _) = asked(at.join("no-such-program"), AMPLE);
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, SignIn::Missing, "a missing program is no token");
}

#[test]
fn sign_in_that_hangs_runs_over() {
    let _turn = turn();
    let (answer, took) = signed("hangs", "exec sleep 60", SHORT);
    assert_eq!(answer, SignIn::RanOver, "a hung sign-in runs over");
    assert!(took >= SHORT, "it was given its patience: {took:?}");
}

#[test]
fn sign_in_that_says_its_piece_and_hangs_runs_over() {
    let _turn = turn();
    let (answer, _) = signed(
        "lingers",
        "echo stand-in-token; exec >&-; exec sleep 60",
        SHORT,
    );
    assert_eq!(
        answer,
        SignIn::RanOver,
        "a sign-in that never ends runs over"
    );
}

#[test]
fn sign_in_that_never_stops_printing_runs_over() {
    let _turn = turn();
    let (answer, _) = signed("prints-on", "exec yes stand-in-token", SHORT);
    assert_eq!(
        answer,
        SignIn::RanOver,
        "a sign-in printing for good runs over"
    );
}

#[test]
fn sign_in_run_over_takes_what_it_started_with_it() {
    let _turn = turn();
    let at = folder("leaves");
    let left = at.join("left");
    let script = format!(
        "sleep 60 & echo $! > {}; echo stand-in-token",
        left.display()
    );
    let (answer, _) = asked(program(&at, &script), SHORT);
    let pid = std::fs::read_to_string(&left).expect("the child's id is written");
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(
        answer,
        SignIn::RanOver,
        "a child holding its output runs it over"
    );
    let status = PathBuf::from(format!("/proc/{}/status", pid.trim()));
    let gone = |status: &Path| {
        std::fs::read_to_string(status).map_or(true, |text| text.contains("State:\tZ"))
    };
    let began = Instant::now();
    // Killed, it's gone once whoever took it in reaps it.
    while !gone(&status) && began.elapsed() < HANG {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(gone(&status), "the child it left is killed too");
}
