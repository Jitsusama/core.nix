//! A reader's sign-in: what the program prints, and never a wait for good.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use sign_in_program::{MOST, SignIn, token};

/// The patience a case cuts a sign-in off at.
const SHORT: Duration = Duration::from_millis(300);

/// The patience a case that expects an answer gives: far past any program
/// here that answers, however loaded the machine.
const AMPLE: Duration = Duration::from_secs(20);

/// How far past its patience a case waits before calling it a hang.
const HANG: Duration = Duration::from_secs(5);

/// The patience a case gives a program that must start a child first,
/// so a loaded builder can't cut it off before the child is running.
const SETTLED: Duration = Duration::from_secs(2);

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
    let at = std::env::temp_dir().join(format!("sign-in-program-{case}-{}", std::process::id()));
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
fn sign_in_from_a_locked_keyring_is_told_apart() {
    let _turn = turn();
    let (answer, _) = signed("locked", "exit 75", AMPLE);
    assert_eq!(
        answer,
        SignIn::Locked,
        "a locked keyring isn't one lacking it"
    );
    let (printed, _) = signed("locked-printing", "echo stand-in-token; exit 75", AMPLE);
    assert_eq!(printed, SignIn::Locked, "nor are its words taken");
}

#[test]
fn sign_in_that_prints_nothing_gives_no_token() {
    let _turn = turn();
    let (answer, _) = signed("nothing", "true", AMPLE);
    assert_eq!(answer, SignIn::Missing, "nothing printed is no token");
}

#[test]
fn sign_in_still_held_for_writing_is_run_once_let_go() {
    let _turn = turn();
    let at = folder("busy");
    let path = program(&at, "echo stand-in-token");
    // Held as a child forked mid-write holds it: running it fails with
    // ETXTBSY until the hold is let go.
    let held = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the program held for writing");
    let letting_go = thread::spawn(move || {
        thread::sleep(SHORT);
        drop(held);
    });
    let (answer, _) = asked(path, AMPLE);
    letting_go.join().expect("the hold is let go");
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(
        answer,
        SignIn::Token("stand-in-token".into()),
        "a program busy being written is run once it can be"
    );
}

#[test]
fn sign_in_that_cant_be_run_gives_no_token() {
    let _turn = turn();
    let at = folder("absent");
    let (answer, took) = asked(at.join("no-such-program"), AMPLE);
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, SignIn::Missing, "a missing program is no token");
    assert!(
        took < SETTLED,
        "and is told at once, not tried again as a busy one is: {took:?}"
    );
}

#[test]
fn sign_in_held_for_writing_past_its_patience_gives_no_token() {
    let _turn = turn();
    let at = folder("held");
    let path = program(&at, "echo stand-in-token");
    let held = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the program held for writing");
    let (answer, took) = asked(path, SHORT);
    drop(held);
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(
        answer,
        SignIn::Missing,
        "a program never run is no token, not one that ran over"
    );
    assert!(
        took >= SHORT,
        "it was tried for its whole patience: {took:?}"
    );
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
    let script = "while :; do echo stand-in-token; sleep 0.01; done";
    let (answer, _) = signed("prints-on", script, SHORT);
    assert_eq!(
        answer,
        SignIn::RanOver,
        "a sign-in printing for good runs over"
    );
}

#[test]
fn sign_in_printing_past_the_most_is_no_token_at_once() {
    let _turn = turn();
    let (answer, took) = signed("floods", "exec yes stand-in-token", AMPLE);
    assert_eq!(
        answer,
        SignIn::Missing,
        "a flood past {MOST} bytes is no token"
    );
    assert!(
        took < HANG,
        "cut off at its most, not its patience: {took:?}"
    );
}

#[test]
fn sign_in_that_isnt_text_is_no_token_and_not_run_over() {
    let _turn = turn();
    let (answer, _) = signed("not-text", r"printf '\377\n'", AMPLE);
    assert_eq!(answer, SignIn::Missing, "what can't be a token is none");
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
    let (answer, _) = asked(program(&at, &script), SETTLED);
    let pid = std::fs::read_to_string(&left).expect("the child's id is written");
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(
        answer,
        SignIn::RanOver,
        "a child holding its output runs it over"
    );
    let pid: u32 = pid.trim().parse().expect("the child's id is whole");
    let status = PathBuf::from(format!("/proc/{pid}/status"));
    let state = |status: &Path| std::fs::read_to_string(status).ok();
    let gone = |status: &Path| state(status).is_none_or(|text| text.contains("State:\tZ"));
    let began = Instant::now();
    // Killed, it's gone once whoever took it in reaps it.
    while !gone(&status) && began.elapsed() < AMPLE {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        gone(&status),
        "the child it left is killed too: {:?}",
        state(&status)
    );
}
