use crate::reclaim::Script;
use crate::sandbox::Sandbox;
use disk_guard::activity::Use;
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::entry;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn candidate(path: &Path) -> Candidate {
    Candidate {
        path: path.to_owned(),
        class: Class::Scratch { healthy: None },
        idle: 0.0,
        size: 0,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

fn names(dir: &Path) -> Vec<String> {
    let entries = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name());
    let mut names: Vec<String> = entries.map(|n| n.to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// A scratch directory holding a run, with a read-only part as Go's module
/// cache leaves, beside a file and a sibling run.
fn scratch(root: &Path) -> PathBuf {
    let dir = root.join("tmp");
    fs::create_dir_all(dir.join("run-1/locked/inner")).unwrap();
    fs::write(dir.join("run-1/locked/inner/f"), "x").unwrap();
    for path in ["run-1/locked/inner", "run-1/locked"] {
        fs::set_permissions(dir.join(path), fs::Permissions::from_mode(0o555)).unwrap();
    }
    fs::create_dir_all(dir.join("run-2")).unwrap();
    fs::write(dir.join("notes.txt"), "n").unwrap();
    dir
}

fn reclaimed(script: &mut Script, name: &str) -> (Sandbox, PathBuf, Outcome) {
    let root = Sandbox::new();
    let dir = scratch(root.path());
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = entry(&anchor, root.path(), &candidate(&dir.join(name)), script);
    (root, dir, outcome)
}

#[test]
fn an_idle_directory_goes_whole_and_alone() {
    let (_root, dir, outcome) = reclaimed(&mut Script::default(), "run-1");
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&dir), ["notes.txt", "run-2"]);
}

#[test]
fn an_idle_file_goes() {
    let (_root, dir, outcome) = reclaimed(&mut Script::default(), "notes.txt");
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&dir), ["run-1", "run-2"]);
}

#[test]
fn an_entry_directly_in_its_root_goes() {
    // As a temp leftover lies straight in the temp directory.
    let root = Sandbox::new();
    let dir = scratch(root.path());
    let anchor = Dir::open(&dir).unwrap();
    for name in ["run-1", "notes.txt"] {
        let outcome = entry(
            &anchor,
            &dir,
            &candidate(&dir.join(name)),
            &mut Script::default(),
        );
        assert_eq!(outcome, Outcome::Reclaimed, "{name}");
    }
    assert_eq!(names(&dir), ["run-2"]);
}

#[test]
fn a_symlink_goes_and_what_it_names_stays() {
    let root = Sandbox::new();
    let dir = scratch(root.path());
    std::os::unix::fs::symlink(dir.join("run-2"), dir.join("link")).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = entry(
        &anchor,
        root.path(),
        &candidate(&dir.join("link")),
        &mut Script::default(),
    );
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&dir), ["notes.txt", "run-1", "run-2"]);
}

#[test]
fn an_entry_in_use_is_never_touched() {
    for used in [Use::Busy(7), Use::Unknown(7), Use::Occupied(7)] {
        let mut script = Script {
            uses: [used].into(),
            ..Default::default()
        };
        let (_root, dir, outcome) = reclaimed(&mut script, "run-1");
        assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
        assert_eq!(names(&dir), ["notes.txt", "run-1", "run-2"]);
        assert_eq!(names(&dir.join("run-1")), ["locked"]);
    }
}

#[test]
fn a_process_arriving_as_it_moved_gets_it_back_intact() {
    let mut script = Script {
        uses: [Use::Free, Use::Busy(9)].into(),
        ..Default::default()
    };
    let (_root, dir, outcome) = reclaimed(&mut script, "run-1");
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&dir.join("run-1")), ["locked"]);
    assert_eq!(
        fs::read_to_string(dir.join("run-1/locked/inner/f")).unwrap(),
        "x"
    );
}

#[test]
fn an_entry_reached_through_a_symlink_fails_untouched() {
    let root = Sandbox::new();
    let dir = scratch(root.path());
    std::os::unix::fs::symlink(&dir, root.path().join("via")).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let through = root.path().join("via/run-1");
    let outcome = entry(
        &anchor,
        root.path(),
        &candidate(&through),
        &mut Script::default(),
    );
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert_eq!(names(&dir.join("run-1")), ["locked"]);
}

#[test]
fn an_entry_outside_the_root_fails_untouched() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(elsewhere.path().join("keep"), "").unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = entry(
        &anchor,
        root.path(),
        &candidate(&elsewhere.path().join("keep")),
        &mut Script::default(),
    );
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert!(elsewhere.path().join("keep").exists());
}
