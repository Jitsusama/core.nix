use disk_guard::anchor::Dir;
use disk_guard::locks::{Hold, hold_cargo, hold_session};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// Whether `free` comes true within a second. A lock belongs to its open
/// file description, which a child another test forks shares until it
/// execs and closes it, so a lock the guard let go can look held for a
/// moment: 34 rounds in 5,000 with a thread spawning, every one free
/// 50 ms later.
fn soon(mut free: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        if free() {
            return true;
        }
        if std::time::Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn held(hold: Hold) -> disk_guard::locks::Held {
    match hold {
        Hold::Held(locks) => locks,
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
    assert!(
        soon(|| cargo.try_lock_shared().is_ok()),
        "the guard didn't let go"
    );
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
        soon(|| other.try_lock().is_ok()),
        "a lock taken before the busy one stayed held"
    );
}

/// rustc's session timestamp: microseconds since the epoch in base 36.
fn stamp(at: SystemTime) -> String {
    let mut micros = u64::try_from(at.duration_since(UNIX_EPOCH).unwrap().as_micros()).unwrap();
    let mut digits = Vec::new();
    while micros > 0 {
        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[usize::try_from(micros % 36).unwrap()]);
        micros /= 36;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

/// A crate's incremental directory with one finished session made at
/// `made`, and its lock file; answers the session's name.
pub(crate) fn session(crate_dir: &Path, made: SystemTime) -> String {
    let prefix = format!("s-{}-1f2e3d", stamp(made));
    fs::create_dir_all(crate_dir.join(format!("{prefix}-0a1b2c3d"))).unwrap();
    fs::write(crate_dir.join(format!("{prefix}.lock")), "").unwrap();
    format!("{prefix}-0a1b2c3d")
}

/// A POSIX record lock, as rustc takes on its session, or None if held.
pub(crate) fn posix_lock(path: &Path) -> Option<File> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = short(libc::F_WRLCK);
    lock.l_whence = short(libc::SEEK_SET);
    let taken = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &lock) } == 0;
    taken.then_some(file)
}

/// A constant as a `flock`'s short fields hold it, whether the platform
/// names it as an int, as Linux does, or as a short already.
fn short<T>(constant: T) -> libc::c_short
where
    T: TryInto<libc::c_short>,
    T::Error: std::fmt::Debug,
{
    constant.try_into().unwrap()
}

#[test]
fn an_old_session_nobody_holds_is_held_against_rustc() {
    let root = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let name = session(root.path(), now - Duration::from_secs(60));
    let dir = Dir::open(root.path()).unwrap();
    let held = held(hold_session(&dir, OsStr::new(&name), now).unwrap());
    assert_eq!(held.count(), 1);
    let lock = root
        .path()
        .join(format!("{}.lock", name.rsplit_once('-').unwrap().0));
    assert!(
        posix_lock(&lock).is_none(),
        "rustc could lock a session the guard holds"
    );
    drop(held);
    assert!(
        soon(|| posix_lock(&lock).is_some()),
        "the guard didn't let go"
    );
}

#[test]
fn a_session_rustc_holds_is_busy() {
    let root = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let name = session(root.path(), now - Duration::from_secs(60));
    let lock = root
        .path()
        .join(format!("{}.lock", name.rsplit_once('-').unwrap().0));
    let _rustc = posix_lock(&lock).unwrap();
    let dir = Dir::open(root.path()).unwrap();
    assert!(matches!(
        hold_session(&dir, OsStr::new(&name), now).unwrap(),
        Hold::Busy(_)
    ));
}

#[test]
fn a_session_younger_than_ten_seconds_is_busy() {
    let root = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let name = session(root.path(), now - Duration::from_secs(5));
    let dir = Dir::open(root.path()).unwrap();
    assert!(matches!(
        hold_session(&dir, OsStr::new(&name), now).unwrap(),
        Hold::Busy(_)
    ));
}

#[test]
fn a_name_rustc_wouldnt_give_is_busy() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("s-zz-x")).unwrap();
    let dir = Dir::open(root.path()).unwrap();
    let now = SystemTime::now();
    assert!(matches!(
        hold_session(&dir, OsStr::new("s-zz-x"), now).unwrap(),
        Hold::Busy(_)
    ));
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
