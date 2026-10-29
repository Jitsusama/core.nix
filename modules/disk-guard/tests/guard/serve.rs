use crate::procs::{link, stat, text};
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places};
use disk_guard::notices::Change;
use disk_guard::serve::{Desk, Unheard, answering, hold, senses, steps, stopping};
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
/// A guard whose every band starts at `pressure`. Its disk is the real
/// one, which other work fills as the test runs, so at 100% the disk is
/// critical and has no band past it for a forecast to start a pass for.
fn guard_at(root: &Path, pressure: &str) -> Guard {
    guard_from(root, pressure, "")
}

/// A guard whose disk is always healthy, with the budget `budget`.
fn guard_over(root: &Path, budget: &str) -> Guard {
    guard_from(root, "0%", &format!(r#""budget": "{budget}","#))
}

fn guard_from(root: &Path, pressure: &str, more: &str) -> Guard {
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true, {more}
             "bands": {{ "pressure": "{pressure}", "urgent": "{pressure}", "critical": "{pressure}", "target": "{pressure}" }},
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{ "trash": {{ "kind": "trash", "path": "{0}" }} }} }}"#,
        root.display()
    ))
    .unwrap();
    fs::create_dir_all(root.join("proc")).unwrap();
    Guard::new(
        crate::unpaced(config),
        &Places {
            state: root.to_owned(),
            status: Some(root.join("status.json")),
            processes: root.join("proc"),
            system: None,
            running: Vec::new(),
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
    // Only that it took the stop, for the journal to time.
    assert_eq!(String::from_utf8_lossy(&err), STOPPING);
}

/// A desktop that saves power as told and keeps what it's told.
#[derive(Default)]
struct Fake {
    saving: bool,
    asked: usize,
    told: Vec<Change>,
    refuse: bool,
    /// How many tells it refuses before it hears, as a notification
    /// server still starting does.
    deaf_for: usize,
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
        if self.deaf_for > 0 {
            self.deaf_for -= 1;
            return Err("not started yet".to_owned());
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
fn the_desktop_is_told_once_when_a_disk_becomes_critically_full() {
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
    assert!(
        summaries[0].ends_with("is still critically full"),
        "{summaries:?}"
    );
}

#[test]
fn a_notice_the_desktop_missed_is_told_again_once_it_hears() {
    let dir = tempfile::tempdir().unwrap();
    let mut guard = guard_at(dir.path(), "100%");
    let mut desk = Fake {
        deaf_for: 1,
        ..Fake::default()
    };
    let said = stepped(&mut guard, 3, &mut desk);
    assert_eq!(desk.told.len(), 1, "{:?} {said}", desk.told);
    assert!(
        matches!(&desk.told[0], Change::Show(notice)
            if notice.summary.ends_with("is still critically full")),
        "{:?}",
        desk.told
    );
    let status: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("status.json")).unwrap()).unwrap();
    assert!(
        status["notice"]["summary"]
            .as_str()
            .is_some_and(|summary| summary.ends_with("is still critically full")),
        "what stands is in the status file too: {status}"
    );
    assert_eq!(status["notice"]["urgency"], "critical", "{status}");
    assert_eq!(status["filesystems"][0]["exhausted"], true, "{status}");
}

#[test]
fn a_disk_below_healthy_has_what_holds_it_measured_on_a_thread_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("kept"), vec![1u8; 64 * 1024]).unwrap();
    let mut guard = guard_at(dir.path(), "100%");
    let mut desk = Fake::default();
    let read = || -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(dir.path().join("status.json")).unwrap()).unwrap()
    };
    // The measure runs beside the steps, so it's looked for over several,
    // with a deadline rather than a count it might outrun.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let status = loop {
        stepped(&mut guard, 2, &mut desk);
        let status = read();
        if status["holders_at"].is_number() || std::time::Instant::now() > deadline {
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let holder = &status["holders"][0];
    assert_eq!(holder["owner"]["kind"], "other", "{status}");
    assert_eq!(holder["owner"]["name"], "trash", "{status}");
    assert!(
        holder["bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes >= 64 * 1024),
        "{status}"
    );
}

/// The status file once its holders are measured, or as it stands when
/// `within` passes first.
fn measured(guard: &mut Guard, dir: &Path, within: Duration) -> serde_json::Value {
    let mut desk = Fake::default();
    let deadline = std::time::Instant::now() + within;
    loop {
        stepped(guard, 2, &mut desk);
        let status: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("status.json")).unwrap()).unwrap();
        if status["holders_at"].is_number() || std::time::Instant::now() > deadline {
            return status;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_healthy_disk_over_its_budget_has_what_holds_it_measured() {
    // A disk can be far from filling and still hold far more than anything
    // there has reason to: 385 GB of a terabyte, against a budget of 250,
    // with no holder named.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("kept"), vec![1u8; 64 * 1024]).unwrap();
    let mut guard = guard_over(dir.path(), "1");
    let status = measured(&mut guard, dir.path(), Duration::from_secs(20));
    assert_eq!(status["filesystems"][0]["band"], "healthy", "{status}");
    assert_eq!(status["holders"][0]["owner"]["name"], "trash", "{status}");
}

#[test]
fn each_measurement_goes_in_the_ledger_and_a_holder_says_what_it_grew_since_one_older() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    fs::write(root.join("kept"), vec![1u8; 64 * 1024]).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let before = format!(
        r#"[{{ "at": {}, "sizes": {{ "{}": 0 }} }}]"#,
        now - 7200.0,
        root.display()
    );
    fs::write(root.join(disk_guard::ledger::FILE), before).unwrap();
    let mut guard = guard_over(&root, "1");
    let status = measured(&mut guard, &root, Duration::from_secs(20));
    let grew = &status["holders"][0]["grew"];
    assert!(grew["bytes"].as_u64().unwrap() >= 64 * 1024, "{status}");
    assert!(grew["over"].as_f64().unwrap() >= 7200.0, "{status}");
    let at: Vec<f64> = disk_guard::ledger::load(&root)
        .iter()
        .map(|entry| entry.at)
        .collect();
    assert_eq!(at.len(), 2, "the older measurement and this one: {at:?}");
}

#[test]
fn a_healthy_disk_within_its_budget_measures_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut guard = guard_over(dir.path(), "100%");
    let status = measured(&mut guard, dir.path(), Duration::from_secs(2));
    assert!(status["holders_at"].is_null(), "{status}");
}

#[test]
fn a_desktop_refusing_the_same_way_is_told_of_once_until_it_hears_again() {
    let mut unheard = Unheard::default();
    let refused = |why: &str| Err(why.to_owned());
    assert_eq!(
        unheard.said(refused("no one listens")).as_deref(),
        Some("no one listens")
    );
    assert_eq!(unheard.said(refused("no one listens")), None);
    assert_eq!(
        unheard.said(refused("it broke")).as_deref(),
        Some("it broke")
    );
    assert_eq!(unheard.said(Ok(())), None);
    assert_eq!(
        unheard.said(refused("it broke")).as_deref(),
        Some("it broke")
    );
}

#[test]
fn a_healthy_disk_tells_the_desktop_nothing_and_a_deaf_one_is_told_of() {
    let dir = tempfile::tempdir().unwrap();
    let mut desk = Fake::default();
    assert_eq!(stepped(&mut guard(dir.path()), 2, &mut desk), STOPPING);
    assert!(desk.told.is_empty());
    let other = tempfile::tempdir().unwrap();
    let mut deaf = Fake {
        refuse: true,
        ..Fake::default()
    };
    let said = stepped(&mut guard_at(other.path(), "100%"), 2, &mut deaf);
    assert_eq!(
        said,
        format!("disk-guard: telling the desktop: no one listens\n{STOPPING}")
    );
}

/// What the loop says once it takes its stop, last of all.
const STOPPING: &str = "disk-guard: stopping\n";

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
    // Each failed step, then the stop.
    assert_eq!(said.lines().count(), 3, "{said}");
    assert!(said.ends_with(STOPPING), "{said}");
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

#[test]
fn a_clear_asked_for_takes_its_folder_s_share_at_once_on_a_healthy_disk() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("files")).unwrap();
    fs::create_dir_all(root.join("info")).unwrap();
    // Trashed just now, as a date past now reads, so a healthy disk's
    // upkeep would keep both a month.
    for name in ["asked.txt", "other.txt"] {
        fs::write(root.join("files").join(name), vec![0u8; 3000]).unwrap();
        let info = format!("[Trash Info]\nPath=/home/u/{name}\nDeletionDate=2099-01-01T08:00:00\n");
        fs::write(root.join("info").join(format!("{name}.trashinfo")), info).unwrap();
    }
    let asked = fs::canonicalize(root.join("files/asked.txt")).unwrap();
    disk_guard::clears::ask(root, &asked, 1.0).unwrap();
    assert_eq!(
        disk_guard::clears::clearing(root),
        std::slice::from_ref(&asked)
    );
    let mut guard = guard_at(root, "0%");
    stepped(&mut guard, 2, &mut Fake::default());
    assert!(!asked.exists(), "what was asked for goes");
    assert!(root.join("files/other.txt").exists(), "nothing else does");
    assert_eq!(
        disk_guard::clears::clearing(root),
        Vec::<std::path::PathBuf>::new(),
        "the ask ends with its pass"
    );
    let journal = fs::read_to_string(root.join("journal.jsonl")).unwrap();
    assert!(
        journal.contains("asked.txt") && journal.contains("reclaimed"),
        "the journal says what went: {journal}"
    );
}

#[test]
fn a_clear_leaves_what_the_other_holders_could_clear_as_it_was() {
    // A clear surveys only the sources around its folder, so the rest
    // must keep what the last whole survey said of them.
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    for (trash, name) in [("asked", "asked.txt"), ("spare", "spare.txt")] {
        let trash = root.join(trash);
        fs::create_dir_all(trash.join("files")).unwrap();
        fs::create_dir_all(trash.join("info")).unwrap();
        fs::write(trash.join("files").join(name), vec![0u8; 3000]).unwrap();
        let info = format!("[Trash Info]\nPath=/home/u/{name}\nDeletionDate=2099-01-01T08:00:00\n");
        fs::write(trash.join("info").join(format!("{name}.trashinfo")), info).unwrap();
    }
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true, "budget": "1",
             "bands": {{ "pressure": "0%", "urgent": "0%", "critical": "0%", "target": "0%" }},
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{ "asked": {{ "kind": "trash", "path": "{0}/asked" }},
                           "spare": {{ "kind": "trash", "path": "{0}/spare" }} }} }}"#,
        root.display()
    ))
    .unwrap();
    fs::create_dir_all(root.join("proc")).unwrap();
    let mut guard = Guard::new(
        crate::unpaced(config),
        &Places {
            state: root.clone(),
            status: Some(root.join("status.json")),
            processes: root.join("proc"),
            system: None,
            running: Vec::new(),
        },
    )
    .unwrap();
    let clears = |status: &serde_json::Value| {
        status["holders"]
            .as_array()
            .unwrap()
            .iter()
            .find(|holder| holder["owner"]["name"] == "spare")
            .map(|holder| holder["clears"].clone())
    };
    let status = measured(&mut guard, &root, Duration::from_secs(20));
    let before = clears(&status);
    assert!(
        before.as_ref().and_then(serde_json::Value::as_u64) > Some(0),
        "the spare trash can be cleared: {}",
        status["holders"]
    );
    disk_guard::clears::ask(&root, &root.join("asked/files/asked.txt"), 1.0).unwrap();
    stepped(&mut guard, 2, &mut Fake::default());
    assert!(
        !root.join("asked/files/asked.txt").exists(),
        "what was asked for goes"
    );
    assert!(
        root.join("spare/files/spare.txt").exists(),
        "nothing else does"
    );
    let status: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("status.json")).unwrap()).unwrap();
    assert_eq!(
        clears(&status),
        before,
        "the spare trash's Clear is as it was"
    );
}

#[test]
fn what_is_kept_is_never_cleared_even_on_a_critically_full_disk() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("files")).unwrap();
    fs::create_dir_all(root.join("info")).unwrap();
    for name in ["kept.txt", "spare.txt"] {
        fs::write(root.join("files").join(name), vec![0u8; 3000]).unwrap();
        let info = format!("[Trash Info]\nPath=/home/u/{name}\nDeletionDate=2026-01-01T08:00:00\n");
        fs::write(root.join("info").join(format!("{name}.trashinfo")), info).unwrap();
    }
    let kept = fs::canonicalize(root.join("files/kept.txt")).unwrap();
    disk_guard::keeps::keep(root, &kept, "a copy of the old photos", 1.0).unwrap();
    let mut guard = guard_at(root, "100%");
    stepped(&mut guard, 3, &mut Fake::default());
    assert!(
        !root.join("files/spare.txt").exists(),
        "what isn't kept goes"
    );
    assert!(kept.exists(), "what is kept stays");
}

#[test]
fn a_footprint_is_only_of_what_the_asker_reaches() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let mine = fs::canonicalize(dir.path()).unwrap();
    let uid = fs::metadata(&mine).unwrap().uid();
    fs::write(mine.join("file"), b"bytes").unwrap();
    // A link of the asker's own to what isn't theirs is judged by where it
    // leads, never by who made the link.
    std::os::unix::fs::symlink("/", mine.join("root")).unwrap();
    let asked = [mine.join("file"), mine.join("root")];
    assert_eq!(
        disk_guard::serve::footprints(&asked, uid.wrapping_add(1)),
        [None, None],
        "another account reaches neither"
    );
    assert_eq!(
        disk_guard::serve::footprints(&asked[1..], uid),
        [None],
        "nor does the link's maker reach what it leads to"
    );
}
