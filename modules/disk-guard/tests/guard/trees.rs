use disk_guard::activity::Use;
use disk_guard::clock::{Awake, Tick};
use disk_guard::discover::{Checkout, Found, Kind};
use disk_guard::plan::Class;
use disk_guard::trees::{Seen, candidate, last_used};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};

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

fn found(path: &str, kind: Kind, checkout: Checkout) -> Found {
    Found {
        path: PathBuf::from(path),
        kind,
        checkout,
    }
}

fn seen(used: Use) -> Seen {
    Seen {
        used,
        finished: false,
        size: 4096,
        last_used: 0.0,
    }
}

/// Awake from wall 0, then suspended from 4,000 to 10,000.
fn suspended_once() -> Awake {
    let mut awake = Awake::default();
    let tick = |wall, monotonic, suspended| Tick {
        wall,
        monotonic,
        suspended,
        boot: "b".into(),
    };
    awake.observe(&tick(0.0, 0.0, 0.0));
    awake.observe(&tick(4_000.0, 4_000.0, 0.0));
    awake.observe(&tick(10_000.0, 4_000.0, 6_000.0));
    awake
}

#[test]
fn a_tree_left_before_a_suspend_has_idled_only_while_awake() {
    let target = found(
        "/home/u/src/app/target",
        Kind::Target,
        Checkout::At("/home/u/src/app".into()),
    );
    let made = candidate(&target, &seen(Use::Free), &suspended_once(), 10_000.0).unwrap();
    assert_eq!(made.idle, 4_000.0);
    assert_eq!(made.class, Class::Target);
    assert_eq!(made.size, 4096);
    assert_eq!(made.checkout.as_deref(), Some(Path::new("/home/u/src/app")));
    assert!(!made.occupied && !made.spent);
}

#[test]
fn a_tree_too_big_to_date_has_not_idled() {
    let target = found("/home/u/src/app/target", Kind::Target, Checkout::Unknown);
    let unread = Seen {
        last_used: f64::INFINITY,
        ..seen(Use::Free)
    };
    let made = candidate(&target, &unread, &suspended_once(), 10_000.0).unwrap();
    assert_eq!(made.idle, 0.0);
}

#[test]
fn a_busy_or_unreadable_tree_is_no_candidate() {
    let target = found("/home/u/src/app/target", Kind::Target, Checkout::Unknown);
    for used in [Use::Busy(7), Use::Unknown(7)] {
        assert_eq!(
            candidate(&target, &seen(used), &Awake::default(), 10.0),
            None
        );
    }
}

#[test]
fn a_tree_someone_sits_in_is_occupied() {
    let install = found(
        "/home/u/src/web/node_modules",
        Kind::Install,
        Checkout::At("/home/u/src/web".into()),
    );
    let made = candidate(&install, &seen(Use::Occupied(7)), &Awake::default(), 10.0).unwrap();
    assert!(made.occupied);
    assert_eq!(made.class, Class::Install);
}

#[test]
fn a_finished_or_deleted_tree_is_spent() {
    let target = found(
        "/home/u/src/app/target",
        Kind::Target,
        Checkout::At("/home/u/src/app".into()),
    );
    let finished = Seen {
        finished: true,
        ..seen(Use::Free)
    };
    assert!(
        candidate(&target, &finished, &Awake::default(), 10.0)
            .unwrap()
            .spent
    );
    let orphan = found(
        "/home/u/src/gone/target",
        Kind::Target,
        Checkout::Gone("/home/u/src/gone".into()),
    );
    let made = candidate(&orphan, &seen(Use::Free), &Awake::default(), 10.0).unwrap();
    assert!(made.spent);
    // Nothing is there to judge the tree by.
    assert_eq!(made.checkout, None);
}

#[test]
fn an_unknown_checkout_is_never_spent() {
    let target = found("/home/u/src/app/target", Kind::Target, Checkout::Unknown);
    let made = candidate(&target, &seen(Use::Free), &Awake::default(), 10.0).unwrap();
    assert!(!made.spent);
    assert_eq!(made.checkout, None);
}

#[test]
fn an_npm_install_is_dated_by_the_last_install_or_commit() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("web");
    let modules = checkout.join("node_modules");
    fs::create_dir_all(modules.join("left-pad/lib")).unwrap();
    fs::create_dir_all(checkout.join(".git/logs")).unwrap();
    fs::write(modules.join(".package-lock.json"), "{}").unwrap();
    fs::write(checkout.join(".git/logs/HEAD"), "").unwrap();
    touch(&modules.join("left-pad/lib"), 9_000);
    touch(&modules.join("left-pad"), 1_000);
    touch(&modules.join(".package-lock.json"), 5_000);
    touch(&modules, 1_000);
    touch(&checkout.join(".git/logs/HEAD"), 3_000);
    let install = found(
        modules.to_str().unwrap(),
        Kind::Install,
        Checkout::At(checkout.clone()),
    );
    // A package's own files keep the times it was published with.
    assert_eq!(last_used(&install), 5_000.0);
    touch(&checkout.join(".git/logs/HEAD"), 7_000);
    assert_eq!(last_used(&install), 7_000.0);
}

#[test]
fn a_venv_is_dated_by_the_last_package_installed_into_it() {
    let root = tempfile::tempdir().unwrap();
    let venv = root.path().join("tool/.venv");
    let packages = venv.join("lib/python3.13/site-packages");
    fs::create_dir_all(packages.join("requests")).unwrap();
    fs::write(venv.join("pyvenv.cfg"), "home = /usr/bin\n").unwrap();
    touch(&packages.join("requests"), 1_000);
    for (path, when) in [
        ("lib/python3.13/site-packages", 6_000),
        ("lib/python3.13", 1_000),
    ] {
        touch(&venv.join(path), when);
    }
    for (path, when) in [("lib", 1_000), ("pyvenv.cfg", 1_000), ("", 1_000)] {
        touch(&venv.join(path), when);
    }
    let install = found(
        venv.to_str().unwrap(),
        Kind::Install,
        Checkout::At(root.path().join("tool")),
    );
    assert_eq!(last_used(&install), 6_000.0);
}

#[test]
fn a_target_is_dated_by_its_last_build() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("app/target");
    fs::create_dir_all(target.join("debug/deps")).unwrap();
    touch(&target.join("debug/deps"), 2_000);
    touch(&target.join("debug"), 1_000);
    touch(&target, 1_000);
    let found = found(target.to_str().unwrap(), Kind::Target, Checkout::Unknown);
    assert_eq!(last_used(&found), 2_000.0);
}
