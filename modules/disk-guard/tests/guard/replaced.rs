use crate::discover::{CARGO_TAG, file};
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim;
use disk_guard::replaced::{Build, Uses, replaced, watched_dirs};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};

fn written(path: &Path, seconds: i64) {
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

/// An artifact in `deps`, written at `seconds`.
fn artifact(deps: &Path, name: &str, seconds: i64) -> PathBuf {
    let path = deps.join(name);
    file(&path, "built");
    written(&path, seconds);
    path
}

const OLD: &str = "1111111111111111";
const NEW: &str = "2222222222222222";

/// A target with two builds of alpha, one of beta and a test binary, in
/// the host's profile and a cross target's.
fn target(root: &Path) -> PathBuf {
    let target = root.join("target");
    for deps in [
        target.join("debug/deps"),
        target.join("x86_64-unknown-linux-gnu/release/deps"),
    ] {
        artifact(&deps, &format!("libalpha-{OLD}.rlib"), 1_000);
        artifact(&deps, &format!("libalpha-{OLD}.rmeta"), 1_000);
        artifact(&deps, &format!("alpha-{OLD}.d"), 1_000);
        artifact(&deps, &format!("libalpha-{NEW}.rlib"), 5_000);
        artifact(&deps, &format!("libalpha-{NEW}.rmeta"), 5_000);
        artifact(&deps, "libbeta-3333333333333333.rlib", 500);
        // A test binary, which no build reads, is no library to replace.
        artifact(&deps, "alpha-4444444444444444", 100);
        artifact(&deps, "alpha-4444444444444444.d", 100);
        fs::create_dir_all(
            deps.parent()
                .unwrap()
                .join(format!(".fingerprint/alpha-{OLD}")),
        )
        .unwrap();
    }
    target
}

#[test]
fn an_older_build_of_a_crate_is_replaced_and_the_newest_stays() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let mut found = replaced(&target);
    found.sort_by(|a, b| a.deps.cmp(&b.deps));
    let deps = |profile: &str| target.join(profile).join("deps");
    let build = |profile: &str| Build {
        deps: deps(profile),
        name: "alpha".to_owned(),
        digest: OLD.to_owned(),
        files: vec![
            deps(profile).join(format!("alpha-{OLD}.d")),
            deps(profile).join(format!("libalpha-{OLD}.rlib")),
            deps(profile).join(format!("libalpha-{OLD}.rmeta")),
        ],
        fingerprint: Some(
            target
                .join(profile)
                .join(format!(".fingerprint/alpha-{OLD}")),
        ),
        written: 1_000.0,
        size: 15,
    };
    assert_eq!(
        found,
        vec![build("debug"), build("x86_64-unknown-linux-gnu/release")]
    );
}

#[test]
fn a_target_in_a_layout_it_doesnt_know_replaces_nothing() {
    // Cargo's per-unit build layout, still behind a nightly flag, where
    // each build has its own directory and its own deps. Nothing there
    // says one replaced another, so two builds of a crate in different
    // directories are never judged against each other.
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    for digest in [OLD, NEW] {
        let deps = target.join(format!("debug/build/alpha/{digest}/deps"));
        artifact(&deps, &format!("libalpha-{digest}.rlib"), 1_000);
        artifact(&deps, &format!("libalpha-{digest}.rmeta"), 1_000);
    }
    assert_eq!(replaced(&target), vec![]);
}

fn build(deps: &Path, written: f64) -> Build {
    Build {
        deps: deps.to_owned(),
        name: "alpha".to_owned(),
        digest: OLD.to_owned(),
        files: vec![
            deps.join(format!("libalpha-{OLD}.rlib")),
            deps.join(format!("libalpha-{OLD}.rmeta")),
        ],
        fingerprint: None,
        written,
        size: 10,
    }
}

#[test]
fn a_build_is_used_when_last_opened_and_never_before_its_deps_was_watched() {
    let deps = Path::new("/t/target/debug/deps");
    let mut uses = Uses::default();
    // Nobody watched it, so nobody knows.
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), None);
    uses.watching(deps, 5_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(5_000.0));
    assert_eq!(uses.last_used(&build(deps, 6_000.0)), Some(6_000.0));
    uses.opened(&deps.join(format!("libalpha-{OLD}.rmeta")), 9_000.0);
    uses.opened(&deps.join("libbeta-3333333333333333.rlib"), 12_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(9_000.0));
    // Watching again later keeps when it began.
    uses.watching(deps, 20_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(9_000.0));
    // Events lost to an overflow could have been opens of anything.
    uses.lost(30_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(30_000.0));
}

#[test]
fn a_target_is_last_opened_when_its_lock_or_a_build_in_its_deps_was() {
    // Every cargo opens its profile's lock, even one that finds all fresh
    // and writes nothing, so the profile is watched beside its deps.
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let profile = target.join("debug");
    let deps = profile.join("deps");
    fs::create_dir_all(&deps).unwrap();
    let kept = target.join("run");
    fs::create_dir_all(kept.join("debug/deps")).unwrap();
    fs::write(kept.join("CACHEDIR.TAG"), CARGO_TAG).unwrap();
    assert_eq!(
        watched_dirs(&target),
        vec![profile.clone(), deps.clone()],
        "a target's watch marks each profile and its deps, and nothing of a target kept in it"
    );
    let mut uses = Uses::default();
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        None,
        "a target nothing opened has no last open"
    );
    uses.opened(&deps.join(format!("libalpha-{OLD}.rlib")), 7_000.0);
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        Some(7_000.0),
        "a library a build read in its deps didn't date the target"
    );
    uses.opened(&profile.join(".cargo-build-lock"), 9_000.0);
    uses.opened(&kept.join("debug/.cargo-build-lock"), 20_000.0);
    uses.opened(
        &profile.join("incremental/alpha-1/s-1/query-cache.bin"),
        30_000.0,
    );
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        Some(9_000.0),
        "something deeper than a target's watched folders dated it"
    );
}

// A Mac gives no folder a handle, so nothing there is ever heard.
#[cfg(target_os = "linux")]
#[test]
fn a_target_is_heard_since_this_guard_marked_its_folders_as_they_are() {
    // An open the watch missed would make a build in use look old, so
    // what it says is trusted only from when this guard's marks last
    // heard everything, on the folders there now.
    let root = tempfile::tempdir().unwrap();
    let dirs = [root.path().join("debug"), root.path().join("debug/deps")];
    fs::create_dir_all(&dirs[1]).unwrap();
    let mut uses = Uses::default();
    uses.watching(&dirs[0], 1.0);
    uses.watching(&dirs[1], 1.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a target an earlier guard watched was heard by this one"
    );
    let mark = |uses: &mut Uses, dir: &Path, at: f64| {
        uses.marked(dir, at, disk_guard::opens::handle(dir).unwrap());
    };
    mark(&mut uses, &dirs[0], 100.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a target with an unmarked deps was heard"
    );
    mark(&mut uses, &dirs[1], 300.0);
    mark(&mut uses, &dirs[1], 400.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(300.0),
        "a target isn't heard from when its last folder was first marked"
    );
    uses.lost(700.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(700.0),
        "a target isn't heard from when the watch last lost events"
    );
    // Cleaned and built again: the mark went with the old folder.
    fs::rename(&dirs[1], root.path().join("old-deps")).unwrap();
    fs::create_dir(&dirs[1]).unwrap();
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deps made anew was heard through the old one's mark"
    );
    // Its handle read before it was made anew, as a mark landing on the
    // new folder after the watch named the old one would be.
    let before = disk_guard::opens::handle(&dirs[1]).unwrap();
    fs::remove_dir(&dirs[1]).unwrap();
    fs::create_dir(&dirs[1]).unwrap();
    uses.marked(&dirs[1], 800.0, before);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deps made anew was heard through a handle read of the one before"
    );
    mark(&mut uses, &dirs[1], 900.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(900.0),
        "a deps made anew isn't heard from when it was marked again"
    );
    fs::remove_dir(&dirs[1]).unwrap();
    assert_eq!(uses.heard_since(&dirs), None, "a gone deps was heard");
    uses.deaf();
    assert!(
        uses.watched.is_empty(),
        "a deaf guard still dated replaced builds by what it watched before"
    );
    fs::create_dir(&dirs[1]).unwrap();
    mark(&mut uses, &dirs[1], 1000.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deaf guard still heard a profile it had marked"
    );
    assert_eq!(
        uses.heard_since(&[]),
        None,
        "a target with no profile was heard"
    );
}

#[test]
fn uses_of_what_is_gone_are_forgotten() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let kept = artifact(&deps, "libalpha-2222222222222222.rlib", 1_000);
    let mut uses = Uses::default();
    uses.watching(&deps, 1.0);
    uses.watching(&root.path().join("gone"), 1.0);
    uses.opened(&kept, 2.0);
    uses.opened(&deps.join("libgone-5555555555555555.rlib"), 3.0);
    uses.forget_gone();
    assert_eq!(uses.watched.keys().collect::<Vec<_>>(), vec![&deps]);
    assert_eq!(uses.opened.keys().collect::<Vec<_>>(), vec![&kept]);
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn older(target: &Path) -> Candidate {
    Candidate {
        path: target.join(format!("debug/deps/alpha-{OLD}")),
        class: Class::Replaced,
        idle: 0.0,
        size: 15,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

#[test]
fn reclaiming_a_build_takes_its_artifacts_and_fingerprint_alone() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = reclaim::build(&anchor, root.path(), &target, &older(&target));
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(
        names(&target.join("debug/deps")),
        [
            "alpha-4444444444444444",
            "alpha-4444444444444444.d",
            &format!("libalpha-{NEW}.rlib"),
            &format!("libalpha-{NEW}.rmeta"),
            "libbeta-3333333333333333.rlib",
        ]
    );
    assert_eq!(
        names(&target.join("debug/.fingerprint")),
        Vec::<String>::new()
    );
    // The cross target's build of it is another candidate.
    assert_eq!(
        names(&target.join("x86_64-unknown-linux-gnu/release/deps")).len(),
        8
    );
}

#[test]
fn a_build_whose_target_cargo_holds_is_kept() {
    use rustix::fs::{FlockOperation, flock};
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    file(&target.join("debug/.cargo-lock"), "");
    let lock = fs::File::open(target.join("debug/.cargo-lock")).unwrap();
    flock(&lock, FlockOperation::LockShared).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = reclaim::build(&anchor, root.path(), &target, &older(&target));
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&target.join("debug/deps")).len(), 8);
}

#[test]
fn a_target_with_no_builds_replaces_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
    fs::create_dir_all(root.path().join("target/debug/deps")).unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
}
