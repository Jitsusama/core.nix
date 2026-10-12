use disk_guard::age::{last_build, last_write};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::Path;

/// Sets a path's modification time, not following a symlink.
fn touch(path: &Path, seconds: i64) {
    let at = Timespec {
        tv_sec: seconds,
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

fn dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
}

#[test]
fn a_build_is_dated_by_what_only_cargo_writes() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    for profile in ["debug", "release", "x86_64-unknown-linux-gnu/debug"] {
        dir(&target.join(profile).join("deps"));
        dir(&target.join(profile).join(".fingerprint"));
        touch(&target.join(profile).join("deps"), 1_000);
        touch(&target.join(profile).join(".fingerprint"), 1_000);
    }
    touch(
        &target.join("x86_64-unknown-linux-gnu/debug/.fingerprint"),
        5_000,
    );
    touch(&target, 9_000);
    assert_eq!(last_build(&target, None), 5_000.0);
}

#[test]
fn a_commit_in_the_checkout_counts_as_work_in_it() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("app");
    dir(&checkout.join(".git/logs"));
    fs::write(checkout.join(".git/logs/HEAD"), "").unwrap();
    touch(&checkout.join(".git/logs/HEAD"), 7_000);
    let target = checkout.join("target");
    dir(&target.join("debug/deps"));
    touch(&target.join("debug/deps"), 1_000);
    assert_eq!(last_build(&target, Some(&checkout)), 7_000.0);
}

#[test]
fn a_linked_worktrees_reflog_is_found_through_its_git_file() {
    let root = tempfile::tempdir().unwrap();
    let logs = root.path().join("repo/.git/worktrees/feature/logs");
    dir(&logs);
    fs::write(logs.join("HEAD"), "").unwrap();
    touch(&logs.join("HEAD"), 8_000);
    let tree = root.path().join("trees/feature");
    dir(&tree);
    fs::write(
        tree.join(".git"),
        "gitdir: ../../repo/.git/worktrees/feature\n",
    )
    .unwrap();
    let target = tree.join("target");
    dir(&target.join("debug/deps"));
    touch(&target.join("debug/deps"), 1_000);
    assert_eq!(last_build(&target, Some(&tree)), 8_000.0);
}

#[test]
fn a_target_never_built_is_dated_by_itself() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    dir(&target);
    touch(&target, 3_000);
    assert_eq!(last_build(&target, None), 3_000.0);
}

#[test]
fn the_last_write_is_the_newest_within_reach() {
    let root = tempfile::tempdir().unwrap();
    let top = root.path().join("scratch");
    dir(&top.join("a/b/c/d/e"));
    fs::write(top.join("a/b/file"), "").unwrap();
    for (path, when) in [
        ("", 1_000),
        ("a", 1_000),
        ("a/b", 1_000),
        ("a/b/file", 4_000),
    ] {
        touch(&top.join(path), when);
    }
    for (path, when) in [("a/b/c", 1_000), ("a/b/c/d", 1_000), ("a/b/c/d/e", 9_000)] {
        touch(&top.join(path), when);
    }
    assert_eq!(last_write(&top, 4, 1_000), 4_000.0);
    assert_eq!(last_write(&top, 5, 1_000), 9_000.0);
}

#[test]
fn a_tree_too_big_to_read_counts_as_written_now() {
    let root = tempfile::tempdir().unwrap();
    for n in 0..20 {
        fs::write(root.path().join(format!("f{n}")), "").unwrap();
        touch(&root.path().join(format!("f{n}")), 1_000);
    }
    assert_eq!(last_write(root.path(), 4, 10), f64::INFINITY);
}

#[test]
fn a_symlink_is_dated_by_itself_not_what_it_names() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    touch(elsewhere.path(), 9_000);
    let top = root.path().join("scratch");
    dir(&top);
    std::os::unix::fs::symlink(elsewhere.path(), top.join("link")).unwrap();
    touch(&top.join("link"), 2_000);
    touch(&top, 1_000);
    assert_eq!(last_write(&top, 4, 1_000), 2_000.0);
}
