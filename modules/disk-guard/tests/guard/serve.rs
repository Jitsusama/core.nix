use crate::procs::{link, stat, text};
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places};
use disk_guard::notices::Change;
use disk_guard::serve::{Desk, answering, hold, senses, steps, stopping};
use disk_guard::socket::{Reply, Request, ask};
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
    // A child another test forked meanwhile shares the lock until it execs.
    let freed = (0..100).any(|_| {
        let held = hold(dir.path()).unwrap().is_some();
        if !held {
            std::thread::sleep(Duration::from_millis(10));
        }
        held
    });
    assert!(freed, "free once it's gone");
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
    answering(&socket, &proc, Duration::ZERO, None).unwrap();
    let request = Request::Processes {
        roots: vec![root.join("src"), "/proc".into()],
    };
    let answer = |request: &Request| match ask(&socket, request, Duration::from_secs(5)) {
        Ok(Reply::Answer(answer)) => answer,
        other => panic!("{other:?}"),
    };
    let told = answer(&request);
    assert!(told.vouched.contains(&(700, 500)) && told.vouched.contains(&(701, 500)));
    assert_eq!(told.touching.len(), 1, "{told:?}");
    assert_eq!(told.touching[0].cwd.as_deref(), Some(mine.as_path()));
    let again = answer(&request);
    assert_eq!(again.vouched.len(), 2, "it goes on answering");
}

#[test]
fn the_system_daemon_hears_who_watches_its_store_and_passes_on_collects() {
    use disk_guard::band::Band;
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let socket = root.join("socket");
    let device = disk_guard::pool::pool(&root).unwrap();
    let hearing = answering(&socket, &root, Duration::ZERO, Some(device)).unwrap();
    let watching = Request::Watching {
        paths: vec![root.clone(), "/proc".into()],
    };
    assert_eq!(
        ask(&socket, &watching, Duration::from_secs(5)).unwrap(),
        Reply::Collects(vec![root.clone()]),
        "/proc holds no store"
    );
    assert!(hearing.spoken.lock().unwrap().contains_key(&device));
    assert!(
        !hearing
            .spoken
            .lock()
            .unwrap()
            .contains_key(&disk_guard::pool::pool(Path::new("/proc")).unwrap())
    );
    let asking = socket.clone();
    let asked = root.clone();
    let collected = std::thread::spawn(move || {
        let collect = Request::Collect {
            path: asked,
            band: Band::Urgent,
        };
        ask(&asking, &collect, Duration::from_secs(10))
    });
    let (path, band, responder) = hearing
        .collects
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    assert_eq!((path, band), (root.clone(), Band::Urgent));
    responder.send(&Reply::Collected("reached".to_owned()));
    assert_eq!(
        collected.join().unwrap().unwrap(),
        Reply::Collected("reached".to_owned())
    );
    let elsewhere = Request::Collect {
        path: "/proc".into(),
        band: Band::Urgent,
    };
    let refused = ask(&socket, &elsewhere, Duration::from_secs(5)).unwrap_err();
    assert!(refused.to_string().contains("no garbage"), "{refused}");
}

fn guard(root: &Path) -> Guard {
    guard_at(root, "0%")
}

/// A guard whose disk is under pressure below `pressure` free.
fn guard_at(root: &Path, pressure: &str) -> Guard {
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true,
             "bands": {{ "pressure": "{pressure}", "urgent": "0%", "critical": "0%", "target": "{pressure}" }},
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
        &mut Fake::default(),
        &mut err,
    );
    assert_eq!(waits, [Duration::from_secs(5); 3]);
    assert!(dir.path().join("status.json").exists(), "it stepped");
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
}

/// A desktop that saves power as told and keeps what it's told.
#[derive(Default)]
struct Fake {
    saving: bool,
    asked: usize,
    told: Vec<Change>,
    refuse: bool,
}

impl Desk for Fake {
    fn saving(&mut self) -> bool {
        self.asked += 1;
        self.saving
    }

    fn tell(&mut self, change: &Change) -> Result<(), String> {
        if self.refuse {
            return Err("no one listens".to_owned());
        }
        self.told.push(change.clone());
        Ok(())
    }
}

/// Steps `guard` `count` times with `desk`, giving what was said.
fn stepped(guard: &mut Guard, count: usize, desk: &mut Fake) -> String {
    let mut waits = 0;
    let mut err = Vec::new();
    steps(
        guard,
        Duration::from_secs(5),
        |_| {
            waits += 1;
            waits == count
        },
        desk,
        &mut err,
    );
    String::from_utf8(err).unwrap()
}

#[test]
fn the_desktop_is_told_once_when_a_disk_comes_under_pressure() {
    let dir = tempfile::tempdir().unwrap();
    let mut guard = guard_at(dir.path(), "100%");
    let mut desk = Fake::default();
    let said = stepped(&mut guard, 3, &mut desk);
    assert_eq!(desk.asked, 3, "the power profile is read each step");
    let summaries: Vec<_> = desk
        .told
        .iter()
        .map(|change| match change {
            Change::Show(notice) => notice.summary.clone(),
            Change::Withdraw => "withdrawn".to_owned(),
        })
        .collect();
    // The first pass finds nothing in the empty trash, so it ends short.
    assert_eq!(summaries.len(), 1, "{summaries:?} {said}");
    assert!(summaries[0].ends_with("short of room"), "{summaries:?}");
}

#[test]
fn a_healthy_disk_tells_the_desktop_nothing_and_a_deaf_one_is_told_of() {
    let dir = tempfile::tempdir().unwrap();
    let mut desk = Fake::default();
    assert_eq!(stepped(&mut guard(dir.path()), 2, &mut desk), "");
    assert!(desk.told.is_empty());
    let other = tempfile::tempdir().unwrap();
    let mut deaf = Fake {
        refuse: true,
        ..Fake::default()
    };
    let said = stepped(&mut guard_at(other.path(), "100%"), 2, &mut deaf);
    assert_eq!(said, "disk-guard: telling the desktop: no one listens\n");
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
        &mut Fake::default(),
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
    let senses = senses(true);
    assert!(senses.saver);
    assert!(senses.pressure.io.some >= 0.0);
}
