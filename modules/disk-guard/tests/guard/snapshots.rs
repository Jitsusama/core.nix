use disk_guard::activity::Process;
use disk_guard::clock::Awake;
use disk_guard::plan::Class;
use disk_guard::snapshots::{Boot, Snapshot, candidate, entries, orphaned};
use regex_lite::Regex;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

const PATTERN: &str = r"^[a-z]+_\d+_(?P<pid>\d+)$";

/// Booted at wall 1,000, counting a hundred ticks a second.
const BOOT: Boot = Boot {
    wall: 1_000.0,
    ticks: 100.0,
};

fn snapshot(pid: u32, born: f64) -> Snapshot {
    Snapshot {
        path: PathBuf::from(format!("/home/u/snapshots/repo_1_{pid}")),
        pid,
        born,
    }
}

/// A process started at a wall time, running a program.
fn running(pid: u32, started: f64, exe: &str) -> Process {
    Process {
        pid,
        start_ticks: ((started - BOOT.wall) * BOOT.ticks) as u64,
        exe: Some(PathBuf::from(exe)),
        cmdline: vec![OsString::from(exe)],
        readable: true,
        ..Default::default()
    }
}

#[test]
fn snapshots_are_named_for_the_process_that_made_them() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "repo_1_4242",
        "repo_2_77",
        "notes",
        "repo_x_1",
        "repo_1_4242.tmp",
    ] {
        fs::create_dir_all(root.path().join(name)).unwrap();
    }
    let mut found = entries(root.path(), &Regex::new(PATTERN).unwrap());
    found.sort_by_key(|snapshot| snapshot.pid);
    let pids: Vec<u32> = found.iter().map(|snapshot| snapshot.pid).collect();
    assert_eq!(pids, [77, 4242]);
    assert_eq!(found[1].path, root.path().join("repo_1_4242"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    assert!((found[1].born - now).abs() < 60.0);
}

#[test]
fn a_snapshot_whose_process_runs_is_kept() {
    let processes = [running(42, 2_000.0, "/nix/store/abc-monobus/bin/monobus")];
    assert!(!orphaned(
        &snapshot(42, 3_000.0),
        &processes,
        &["monobus"],
        BOOT
    ));
}

#[test]
fn a_snapshot_whose_process_is_gone_is_orphaned() {
    let processes = [running(7, 2_000.0, "/nix/store/abc-monobus/bin/monobus")];
    assert!(orphaned(
        &snapshot(42, 3_000.0),
        &processes,
        &["monobus"],
        BOOT
    ));
}

#[test]
fn a_reused_pid_keeps_nothing_alive() {
    // Another program now has the pid, or the same program started after
    // the snapshot was made, so it can't be the one that made it.
    let other = [running(42, 2_000.0, "/usr/bin/vim")];
    assert!(orphaned(&snapshot(42, 3_000.0), &other, &["monobus"], BOOT));
    let later = [running(42, 4_000.0, "/nix/store/abc-monobus/bin/monobus")];
    assert!(orphaned(&snapshot(42, 3_000.0), &later, &["monobus"], BOOT));
}

#[test]
fn a_process_that_cant_be_read_keeps_its_snapshot() {
    let hidden = Process {
        pid: 42,
        readable: false,
        ..Default::default()
    };
    assert!(!orphaned(
        &snapshot(42, 3_000.0),
        &[hidden],
        &["monobus"],
        BOOT
    ));
}

#[test]
fn a_wrapped_program_is_known_by_its_own_name() {
    let wrapped = [running(
        42,
        2_000.0,
        "/nix/store/abc-monobus/bin/.monobus-wrapped",
    )];
    assert!(!orphaned(
        &snapshot(42, 3_000.0),
        &wrapped,
        &["monobus"],
        BOOT
    ));
}

#[test]
fn an_orphaned_snapshot_is_waste() {
    let made = candidate(&snapshot(42, 3_000.0), 5, &Awake::default(), 4_000.0);
    assert_eq!(made.class, Class::Snapshot);
    assert_eq!(made.idle, 1_000.0);
    assert_eq!(made.size, 5);
}
