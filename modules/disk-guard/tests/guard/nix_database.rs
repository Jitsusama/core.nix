use disk_guard::nix_database::database_written;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// A lock on `len` bytes of `path` from `start`, of `kind`, held by its
/// own open file until the file drops, so it counts against any other
/// lock this process asks about, as a POSIX lock of its own wouldn't.
pub(crate) fn locked(path: &Path, kind: libc::c_int, start: i64, len: i64) -> fs::File {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    // SAFETY: a lock of all zeroes is a valid one, filled in before use.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = libc::c_short::try_from(kind).unwrap();
    lock.l_whence = libc::c_short::try_from(libc::SEEK_SET).unwrap();
    lock.l_start = start;
    lock.l_len = len;
    let fd = std::os::fd::AsRawFd::as_raw_fd(&file);
    // SAFETY: the descriptor is open for both and the lock is filled in.
    let taken = unsafe { libc::fcntl(fd, libc::F_OFD_SETLK, &raw const lock) };
    assert_eq!(taken, 0, "couldn't lock {}", path.display());
    file
}

/// A database folder holding SQLite's shared memory, as Nix's does.
fn database() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("db.sqlite-shm");
    fs::write(&shared, vec![0; 4096]).unwrap();
    (dir, shared)
}

#[test]
fn only_a_write_lock_on_sqlite_s_write_byte_says_the_database_is_written() {
    // SQLite's writer locks byte 120 of the shared memory alone; its
    // readers lock 123 to 127 and its checkpointer 121, all the time.
    let (dir, shared) = database();
    assert!(!database_written(dir.path()), "written with no lock held");
    for (kind, start, len) in [
        (libc::F_RDLCK, 120, 1),
        (libc::F_WRLCK, 119, 1),
        (libc::F_WRLCK, 121, 1),
        (libc::F_WRLCK, 128, 1),
        (libc::F_RDLCK, 0, 0),
    ] {
        let held = locked(&shared, kind, start, len);
        assert!(
            !database_written(dir.path()),
            "written by a lock of kind {kind} on {len} bytes from {start}"
        );
        drop(held);
    }
    for (start, len) in [(120, 1), (0, 0)] {
        let writing = locked(&shared, libc::F_WRLCK, start, len);
        assert!(
            database_written(dir.path()),
            "a write lock on {len} bytes from {start} went unseen"
        );
        drop(writing);
    }
    assert!(
        !database_written(dir.path()),
        "still written once it let go"
    );
}

#[test]
fn a_lock_on_another_file_or_a_whole_file_flock_isnt_the_database_s() {
    let (dir, shared) = database();
    let other = dir.path().join("other");
    fs::write(&other, vec![0; 4096]).unwrap();
    let elsewhere = locked(&other, libc::F_WRLCK, 0, 0);
    let whole = fs::File::open(&shared).unwrap();
    rustix::fs::flock(&whole, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();
    assert!(
        !database_written(dir.path()),
        "another file's lock or a flock counted"
    );
    drop((elsewhere, whole));
    let gone = tempfile::tempdir().unwrap();
    assert!(
        !database_written(gone.path()),
        "written with no shared memory"
    );
}

#[test]
fn a_link_or_a_pipe_in_the_shared_memory_s_place_is_neither_followed_nor_waited_on() {
    let (dir, shared) = database();
    let real = dir.path().join("real");
    fs::rename(&shared, &real).unwrap();
    let writing = locked(&real, libc::F_WRLCK, 120, 1);
    std::os::unix::fs::symlink(&real, &shared).unwrap();
    assert!(
        !database_written(dir.path()),
        "a link was followed to a written file"
    );
    drop(writing);
    fs::remove_file(&shared).unwrap();
    let name = CString::new(shared.as_os_str().as_bytes()).unwrap();
    // SAFETY: a plain system call on a name this test owns.
    assert_eq!(
        unsafe { libc::mkfifo(name.as_ptr(), 0o600) },
        0,
        "no pipe made"
    );
    let began = std::time::Instant::now();
    assert!(!database_written(dir.path()), "a pipe read as written");
    assert!(
        began.elapsed() < std::time::Duration::from_secs(5),
        "a pipe held the guard {:?}",
        began.elapsed()
    );
}
