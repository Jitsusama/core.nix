//! A reader's sign-in: what the program prints, and never a wait for good.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use pineapple::sign_in::token;

/// How long a case lets a sign-in take before it's run over.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a case waits for an answer before calling it a hang: well past
/// PATIENCE, and well short of any program's sleep here.
const HANG: Duration = Duration::from_secs(5);

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

/// What `token` answers for `path`, and how long it took, or a failure
/// naming the hang when it hasn't answered within HANG.
fn asked(path: PathBuf) -> (Option<String>, Duration) {
    let (said, heard) = mpsc::channel();
    let began = Instant::now();
    thread::spawn(move || {
        // The case may have given up, and then nobody listens.
        let _ = said.send(token(path.as_os_str(), PATIENCE));
    });
    let answer = heard
        .recv_timeout(HANG)
        .expect("the sign-in is never waited on for good");
    (answer, began.elapsed())
}

#[test]
fn sign_in_prints_the_token_trimmed() {
    let at = folder("prints");
    let (answer, _) = asked(program(&at, "echo '  stand-in-token  '"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(
        answer.as_deref(),
        Some("stand-in-token"),
        "the printed token"
    );
}

#[test]
fn sign_in_that_fails_gives_no_token() {
    let at = folder("fails");
    let (answer, _) = asked(program(&at, "echo stand-in-token; exit 1"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "a failed sign-in's words aren't taken");
}

#[test]
fn sign_in_that_prints_nothing_gives_no_token() {
    let at = folder("nothing");
    let (answer, _) = asked(program(&at, "true"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "nothing printed is no token");
}

#[test]
fn sign_in_that_cant_be_run_gives_no_token() {
    let at = folder("absent");
    let (answer, _) = asked(at.join("no-such-program"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "a missing program is no token");
}

#[test]
fn sign_in_that_hangs_is_cut_off() {
    let at = folder("hangs");
    let (answer, took) = asked(program(&at, "exec sleep 60"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "a hung sign-in is no token");
    assert!(took >= PATIENCE, "it was given its patience: {took:?}");
}

#[test]
fn sign_in_that_says_its_piece_and_hangs_is_cut_off() {
    let at = folder("lingers");
    let (answer, _) = asked(program(&at, "echo stand-in-token; exec >&-; exec sleep 60"));
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "a sign-in that never ends gives no token");
}

#[test]
fn sign_in_cut_off_takes_what_it_started_with_it() {
    let at = folder("leaves");
    let left = at.join("left");
    let script = format!(
        "sleep 60 & echo $! > {}; echo stand-in-token",
        left.display()
    );
    let (answer, _) = asked(program(&at, &script));
    let pid = std::fs::read_to_string(&left).expect("the child's id is written");
    std::fs::remove_dir_all(&at).expect("the case's folder goes");
    assert_eq!(answer, None, "a child holding its output runs it over");
    let status = PathBuf::from(format!("/proc/{}/status", pid.trim()));
    let began = Instant::now();
    // Killed, it's gone once whoever took it in reaps it.
    while std::fs::read_to_string(&status).is_ok_and(|text| !text.contains("State:\tZ"))
        && began.elapsed() < HANG
    {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        std::fs::read_to_string(&status).map_or(true, |text| text.contains("State:\tZ")),
        "the child it left is killed too"
    );
}
