use crate::procs::{link, stat, text};
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places};
use disk_guard::serve::{answering, hold, senses, steps, stopping};
use disk_guard::socket::{Request, ask};
use std::fs;
use std::path::Path;
use std::time::Duration;

#[test]
fn one_daemon_holds_its_state_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let first = hold(dir.path()).unwrap();
    assert!(first.is_some());
    assert!(hold(dir.path()).unwrap().is_none(), "held by the first");
    drop(first);
    assert!(hold(dir.path()).unwrap().is_some(), "free once it's gone");
}

/// A readable process sitting in `cwd`, in a fake `/proc`.
fn sitting(proc: &Path, pid: u32, cwd: &Path) {
    let at = |file: &str| format!("{pid}/{file}");
    text(proc, &at("stat"), &stat(pid, "cargo", 0, 500));
    text(proc, &at("status"), "Name:\tcargo\nUid:\t0\t0\t0\t0\n");
    text(proc, &at("cmdline"), "cargo\0build\0");
    text(proc, &at("maps"), "");
    link(proc, &at("cwd"), cwd.to_str().unwrap());
    link(proc, &at("exe"), "/bin/cargo");
    fs::create_dir_all(proc.join(at("fd"))).unwrap();
}

#[test]
fn the_system_daemon_answers_for_what_the_asker_owns() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let mine = root.join("src/app");
    fs::create_dir_all(&mine).unwrap();
    let proc = root.join("proc");
    sitting(&proc, 700, &mine);
    // Beneath what's asked about, but root's.
    sitting(&proc, 701, Path::new("/proc"));
    let socket = root.join("socket");
    answering(&socket, &proc, Duration::ZERO).unwrap();
    let request = Request::Processes {
        roots: vec![root.join("src"), "/proc".into()],
    };
    let told = ask(&socket, &request, Duration::from_secs(5)).unwrap();
    assert!(told.vouched.contains(&(700, 500)) && told.vouched.contains(&(701, 500)));
    assert_eq!(told.touching.len(), 1, "{told:?}");
    assert_eq!(told.touching[0].cwd.as_deref(), Some(mine.as_path()));
    let again = ask(&socket, &request, Duration::from_secs(5)).unwrap();
    assert_eq!(again.vouched.len(), 2, "it goes on answering");
}

fn guard(root: &Path) -> Guard {
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true,
             "bands": {{ "pressure": "0%", "urgent": "0%", "critical": "0%", "target": "0%" }},
             "sources": {{ "trash": {{ "kind": "trash", "path": "{}" }} }} }}"#,
        root.display()
    ))
    .unwrap();
    fs::create_dir_all(root.join("proc")).unwrap();
    Guard::new(
        config,
        &Places {
            state: root.to_owned(),
            status: Some(root.join("status.json")),
            processes: root.join("proc"),
            system: None,
        },
    )
    .unwrap()
}

#[test]
fn the_loop_steps_until_told_to_stop_waiting_between() {
    let dir = tempfile::tempdir().unwrap();
    let mut guard = guard(dir.path());
    let mut waits = Vec::new();
    let mut err = Vec::new();
    steps(
        &mut guard,
        Duration::from_secs(5),
        |wait| {
            waits.push(wait);
            waits.len() == 3
        },
        &mut err,
    );
    assert_eq!(waits, [Duration::from_secs(5); 3]);
    assert!(dir.path().join("status.json").exists(), "it stepped");
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
}

#[test]
fn a_step_that_fails_is_told_of_and_the_loop_goes_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut guard = guard(dir.path());
    // A status file that can't be written, since its place is a directory.
    fs::create_dir_all(dir.path().join("status.json/x")).unwrap();
    let mut waits = 0;
    let mut err = Vec::new();
    steps(
        &mut guard,
        Duration::from_secs(5),
        |_| {
            waits += 1;
            waits == 2
        },
        &mut err,
    );
    assert_eq!(waits, 2);
    let said = String::from_utf8(err).unwrap();
    assert_eq!(said.lines().count(), 2, "{said}");
}

#[test]
fn waiting_ends_early_for_a_signal_to_stop() {
    let mut wait = stopping(&[libc::SIGUSR2]).unwrap();
    let started = std::time::Instant::now();
    assert!(!wait(Duration::from_millis(50)), "no signal, so it waited");
    assert!(started.elapsed() >= Duration::from_millis(50));
    // Raised at this thread, which blocks it, so it waits to be taken.
    assert_eq!(unsafe { libc::raise(libc::SIGUSR2) }, 0);
    let started = std::time::Instant::now();
    assert!(wait(Duration::from_secs(5)), "told to stop");
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn this_machine_can_be_sensed() {
    let senses = senses();
    assert!(senses.pressure.io.some >= 0.0);
}
