use disk_guard::anchor::Dir;
use disk_guard::locks::{Hold, hold_cargo};
use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// A target as cargo leaves it: a profile for the host, one for a cross
/// target, and lock files in each.
fn target(root: &Path) -> PathBuf {
    let target = root.join("target");
    for profile in ["debug", "x86_64-unknown-linux-gnu/release"] {
        fs::create_dir_all(target.join(profile).join("deps")).unwrap();
        for lock in [".cargo-lock", ".cargo-build-lock", ".cargo-artifact-lock"] {
            fs::write(target.join(profile).join(lock), "").unwrap();
        }
    }
    target
}

fn held(hold: Hold) -> disk_guard::locks::Held {
    match hold {
        Hold::Held(held) => held,
        Hold::Busy(path) => panic!("busy at {}", path.display()),
    }
}

#[test]
fn an_idle_target_is_held_against_cargo_until_let_go() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let held = held(hold_cargo(&Dir::open(&target).unwrap()).unwrap());
    assert_eq!(held.count(), 6);
    let cargo = File::open(target.join("x86_64-unknown-linux-gnu/release/.cargo-lock")).unwrap();
    assert!(
        cargo.try_lock_shared().is_err(),
        "the guard doesn't hold it"
    );
    drop(held);
    assert!(cargo.try_lock_shared().is_ok(), "the guard didn't let go");
}

#[test]
fn a_target_cargo_holds_is_busy_and_nothing_stays_held() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let cargo = File::open(target.join("x86_64-unknown-linux-gnu/release/.cargo-lock")).unwrap();
    cargo.try_lock_shared().unwrap();
    match hold_cargo(&Dir::open(&target).unwrap()).unwrap() {
        Hold::Busy(path) => {
            assert_eq!(
                path,
                Path::new("x86_64-unknown-linux-gnu/release/.cargo-lock")
            );
        }
        Hold::Held(_) => panic!("held a lock cargo holds"),
    }
    let other = File::open(target.join("debug/.cargo-build-lock")).unwrap();
    assert!(
        other.try_lock().is_ok(),
        "a lock taken before the busy one stayed held"
    );
}

#[test]
fn a_lock_that_is_a_symlink_makes_the_target_busy() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    fs::remove_file(target.join("debug/.cargo-lock")).unwrap();
    fs::write(root.path().join("elsewhere"), "").unwrap();
    symlink(
        root.path().join("elsewhere"),
        target.join("debug/.cargo-lock"),
    )
    .unwrap();
    assert!(matches!(
        hold_cargo(&Dir::open(&target).unwrap()).unwrap(),
        Hold::Busy(_)
    ));
}
