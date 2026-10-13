use crate::locks::{posix_lock, session};
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::variant;
use disk_guard::trees::{Variant, variants};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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

fn an_hour_ago() -> SystemTime {
    SystemTime::now() - Duration::from_secs(3600)
}

fn candidate(path: &Path) -> Candidate {
    Candidate {
        path: path.to_owned(),
        class: Class::Variant,
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

#[test]
fn each_crate_variant_is_dated_by_its_newest_session() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let app = target.join("debug/incremental/app-1a2b3c");
    let lib = target.join("x86_64-unknown-linux-gnu/release/incremental/lib-4d5e6f");
    let older = session(&app, an_hour_ago());
    let newer = session(&app, an_hour_ago() + Duration::from_secs(60));
    session(&lib, an_hour_ago());
    touch(&app.join(older), 2_000);
    touch(&app.join(newer), 3_000);
    touch(&app, 1_000);
    let mut found = variants(&target);
    found.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0],
        Variant {
            path: app,
            last_used: 3_000.0
        }
    );
    assert_eq!(found[1].path, lib);
}

#[test]
fn a_variant_mid_compile_isnt_offered() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let app = target.join("debug/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    fs::create_dir_all(app.join("s-abc-def-working")).unwrap();
    assert_eq!(variants(&target), []);
}

#[test]
fn an_idle_variant_goes_with_every_session_and_lock() {
    let root = tempfile::tempdir().unwrap();
    let incremental = root.path().join("target/debug/incremental");
    let app = incremental.join("app-1a2b3c");
    session(&app, an_hour_ago());
    session(&app, an_hour_ago() + Duration::from_secs(60));
    fs::write(
        app.join(session(&app, an_hour_ago() + Duration::from_secs(120)))
            .join("dep-graph.bin"),
        [0; 64],
    )
    .unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(&anchor, root.path(), &candidate(&app), SystemTime::now());
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&incremental), Vec::<String>::new());
}

#[test]
fn a_variant_with_a_session_rustc_holds_is_kept_whole() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("target/debug/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    let held = session(&app, an_hour_ago() + Duration::from_secs(60));
    let lock = app.join(format!("{}.lock", held.rsplit_once('-').unwrap().0));
    let _rustc = posix_lock(&lock).unwrap();
    let before = names(&app);
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(&anchor, root.path(), &candidate(&app), SystemTime::now());
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&app), before);
}

#[test]
fn a_variant_with_a_young_session_is_kept_whole() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("target/debug/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    session(&app, SystemTime::now());
    let before = names(&app);
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(&anchor, root.path(), &candidate(&app), SystemTime::now());
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&app), before);
}

#[test]
fn a_variant_mid_compile_is_kept_whole_even_with_no_lock_to_see() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("target/debug/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    fs::create_dir_all(app.join("s-abc-def-working")).unwrap();
    let before = names(&app);
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(&anchor, root.path(), &candidate(&app), SystemTime::now());
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&app), before);
}

#[test]
fn what_isnt_a_session_keeps_the_variants_directory() {
    // rustc may start a session as the old ones go, and that one must
    // survive, so the directory is removed only if nothing is left in it.
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("target/debug/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    fs::create_dir_all(app.join("newcomer/inside")).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(&anchor, root.path(), &candidate(&app), SystemTime::now());
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&app), ["newcomer"]);
    assert!(app.join("newcomer/inside").is_dir());
}

#[test]
fn a_variant_reached_through_a_symlink_fails_untouched() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("real/incremental/app-1a2b3c");
    session(&app, an_hour_ago());
    std::os::unix::fs::symlink(root.path().join("real"), root.path().join("link")).unwrap();
    let before = names(&app);
    let through: PathBuf = root.path().join("link/incremental/app-1a2b3c");
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = variant(
        &anchor,
        root.path(),
        &candidate(&through),
        SystemTime::now(),
    );
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert_eq!(names(&app), before);
}
