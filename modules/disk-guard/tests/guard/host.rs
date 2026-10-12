use crate::procs::{LOCKS, link, stat, text};
use disk_guard::activity::Use;
use disk_guard::host::{Filesystem, Reclaim, Scan};
use disk_guard::pass::{Disk, Outcome, Reclaimer};
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::Looker;
use disk_guard::survey::{Action, Offer};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

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

fn offer(path: &Path, class: Class, action: Action) -> Offer {
    Offer {
        candidate: Candidate {
            path: path.to_owned(),
            class,
            idle: 0.0,
            size: 0,
            occupied: false,
            spent: false,
            checkout: None,
        },
        action,
        device: 0,
    }
}

/// A scan of a `/proc` with nothing in it, so nothing is in use, that
/// reads the real lock listing as this process, which takes the locks.
fn empty_scan(proc: &Path) -> Scan {
    Scan::new(
        proc,
        Path::new("/proc/locks"),
        std::process::id(),
        Duration::ZERO,
    )
}

#[test]
fn each_offer_is_reclaimed_as_its_action_says() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let target = crate::reclaim::built(&root);
    let crate_dir = root.join("app/target/debug/incremental/app-1a2b3c");
    crate::locks::session(&crate_dir, SystemTime::now() - Duration::from_secs(3600));
    let scratch = root.join("scratch/run-1");
    fs::create_dir_all(&scratch).unwrap();
    fs::write(scratch.join("out"), "x").unwrap();
    let trash = root.join("Trash");
    fs::create_dir_all(trash.join("files")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(trash.join("files/old.txt"), "old").unwrap();
    fs::write(trash.join("info/old.txt.trashinfo"), "[Trash Info]\n").unwrap();
    let offers = [
        offer(
            &crate_dir,
            Class::Variant,
            Action::Variant { root: root.clone() },
        ),
        offer(
            &target,
            Class::Target,
            Action::Target { root: root.clone() },
        ),
        offer(
            &scratch,
            Class::Scratch { healthy: None },
            Action::Entry {
                root: root.join("scratch"),
            },
        ),
        offer(
            &trash.join("files/old.txt"),
            Class::Trash,
            Action::Trash {
                trash: trash.clone(),
            },
        ),
    ];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    for offered in &offers {
        assert_eq!(
            reclaim.reclaim(&offered.candidate),
            Outcome::Reclaimed,
            "{:?}",
            offered.action
        );
    }
    assert!(!crate_dir.exists());
    assert_eq!(
        fs::read_dir(&target).unwrap().count(),
        1,
        "only the benchmarks are left"
    );
    assert!(!scratch.exists());
    assert!(!trash.join("files/old.txt").exists());
    assert!(!trash.join("info/old.txt.trashinfo").exists());
}

#[test]
fn a_workspace_goes_and_its_worktrees_are_pruned() {
    use crate::finished::git;
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let (repo, workspace) = crate::workspaces::linked_fixture(&root);
    let offers = [offer(
        &workspace,
        Class::Workspace,
        Action::Workspace { root: root.clone() },
    )];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    assert_eq!(reclaim.reclaim(&offers[0].candidate), Outcome::Reclaimed);
    assert!(!workspace.exists());
    let listed = git(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert!(reclaim.notes().is_empty());
}

#[test]
fn a_candidate_no_offer_names_fails_untouched() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let stray = root.path().join("stray");
    fs::create_dir_all(&stray).unwrap();
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&[], &mut scan);
    let named = offer(
        &stray,
        Class::Temp,
        Action::Entry {
            root: root.path().to_owned(),
        },
    );
    assert!(matches!(
        reclaim.reclaim(&named.candidate),
        Outcome::Failed(_)
    ));
    assert!(stray.exists());
}

#[test]
fn a_filesystem_is_measured_and_settled_where_it_lies() {
    let root = tempfile::tempdir().unwrap();
    let mut disk = Filesystem::new(root.path());
    let space = disk.space().unwrap();
    assert!(space.total > 0 && space.free <= space.total);
    disk.settle().unwrap();
    let before = disk.now();
    std::thread::sleep(Duration::from_millis(5));
    assert!(disk.now() > before);
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
