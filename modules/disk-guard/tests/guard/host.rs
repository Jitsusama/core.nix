use crate::procs::{LOCKS, link, stat, text};
use disk_guard::activity::Use;
use disk_guard::host::Scan;
use disk_guard::reclaim::Looker;
use std::path::Path;
use std::time::Duration;

/// A readable process sitting in `cwd`, in a fake `/proc`.
fn sitting(proc: &Path, pid: u32, cwd: &str) {
    let at = |file: &str| format!("{pid}/{file}");
    text(proc, &at("stat"), &stat(pid, "sh", 0, 100));
    text(
        proc,
        &at("status"),
        "Name:\tsh\nUid:\t1000\t1000\t1000\t1000\n",
    );
    text(proc, &at("cmdline"), "sh\0");
    text(proc, &at("maps"), "");
    link(proc, &at("cwd"), cwd);
    link(proc, &at("exe"), "/bin/sh");
    std::fs::create_dir_all(proc.join(at("fd"))).unwrap();
}

const TARGET: &str = "/home/u/src/app/target";

#[test]
fn a_scan_is_reused_until_something_is_moved() {
    let proc = tempfile::tempdir().unwrap();
    let mut scan = Scan::new(
        proc.path(),
        &proc.path().join("locks"),
        42,
        Duration::from_secs(3600),
    );
    let target = Path::new(TARGET);
    assert_eq!(scan.judge(target, None).unwrap(), Use::Free);
    sitting(proc.path(), 100, "/home/u/src/app/target/debug");
    assert_eq!(
        scan.judge(target, None).unwrap(),
        Use::Free,
        "the scan was reused"
    );
    scan.moved();
    assert_eq!(scan.judge(target, None).unwrap(), Use::Busy(100));
}

#[test]
fn a_scan_older_than_its_freshness_is_taken_again() {
    let proc = tempfile::tempdir().unwrap();
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    let target = Path::new(TARGET);
    assert_eq!(scan.judge(target, None).unwrap(), Use::Free);
    sitting(proc.path(), 100, "/home/u/src/app/target/debug");
    assert_eq!(scan.judge(target, None).unwrap(), Use::Busy(100));
}

#[test]
fn the_guard_itself_judges_nothing() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 42, "/home/u/src/app/target");
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Free);
}

#[test]
fn waiters_are_read_from_the_lock_listing() {
    let proc = tempfile::tempdir().unwrap();
    text(proc.path(), "locks", LOCKS);
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    assert_eq!(scan.waiters(&[7]).unwrap(), [500]);
}

#[test]
fn the_processes_a_survey_judges_by_come_from_the_same_scan() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 100, "/home/u");
    let mut scan = Scan::new(
        proc.path(),
        &proc.path().join("locks"),
        42,
        Duration::from_secs(3600),
    );
    assert_eq!(scan.processes().unwrap().len(), 1);
    sitting(proc.path(), 101, "/home/u");
    assert_eq!(scan.processes().unwrap().len(), 1);
    scan.moved();
    assert_eq!(scan.processes().unwrap().len(), 2);
}
