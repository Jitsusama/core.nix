use crate::discover::file;
use disk_guard::replaced::{Build, replaced};
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
fn a_target_with_no_builds_replaces_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert!(replaced(&root.path().join("target")).is_empty());
    fs::create_dir_all(root.path().join("target/debug/deps")).unwrap();
    assert!(replaced(&root.path().join("target")).is_empty());
}
