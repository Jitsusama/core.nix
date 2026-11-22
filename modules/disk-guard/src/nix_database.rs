//! Whether another process is writing Nix's database, which a delete of
//! Nix's garbage would wait on, keeping that writer waiting in turn.

use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// The byte of a SQLite database's shared memory a writer locks while it
/// writes, in write-ahead mode as Nix keeps its database: the first of the
/// locks that follow the index's header (`WALINDEX_LOCK_OFFSET` and
/// `WAL_WRITE_LOCK` in SQLite's wal.c). On 21 November a sampler saw
/// `nix store delete` and evaluations hold this byte as they wrote.
pub const WRITE_LOCK_BYTE: libc::off_t = 120;

/// Whether the guard has said it can't tell, so it says so once.
static BLIND: AtomicBool = AtomicBool::new(false);

/// Whether another process holds Nix's database in `database` for
/// writing, as the kernel answers for a write lock on its write byte. A
/// delete would wait on it, and a build or an evaluation then waits on the
/// delete, reading "database is busy": on 21 November, of 41 moments a
/// sampler saw a writer, evaluations held it in most. Asked of the file
/// itself, so no lock on another file, and no whole-file `flock`, can
/// answer for it. Where its shared memory isn't there, nothing has the
/// database open to write in write-ahead mode. Where the kernel can't say,
/// nobody is taken to be writing, as before the guard asked, and it says
/// so once, since its deletes then wait for no writer. The file is opened
/// without following a link and without waiting, so nothing put in its
/// place can hold the guard.
pub fn database_written(database: &Path) -> bool {
    let path = database.join("db.sqlite-shm");
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path);
    let shared = match opened {
        Ok(shared) if shared.metadata().is_ok_and(|meta| meta.is_file()) => shared,
        Ok(_) => return blind(&path, "it isn't a file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if database.join("db.sqlite").exists() {
                return blind(&path, "Nix keeps its database without one");
            }
            return false;
        }
        Err(error) => return blind(&path, &error.to_string()),
    };
    let write = write_lock();
    let Ok(whence) = libc::c_short::try_from(libc::SEEK_SET) else {
        return false;
    };
    // SAFETY: a lock of all zeroes is a valid one, filled in below.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = write;
    lock.l_whence = whence;
    lock.l_start = WRITE_LOCK_BYTE;
    lock.l_len = 1;
    let fd = std::os::fd::AsRawFd::as_raw_fd(&shared);
    // SAFETY: a plain system call on a file this holds open, filling in a
    // lock it owns.
    let asked = unsafe { libc::fcntl(fd, libc::F_GETLK, &raw mut lock) };
    if asked != 0 {
        return blind(&path, &std::io::Error::last_os_error().to_string());
    }
    lock.l_type == write
}

/// A write lock's kind as a lock holds it: libc types it as an int on
/// Linux and as a short, which a lock holds, on a Mac.
/// It's 1, which a short holds; were it not, the kernel would refuse a
/// kind no lock has, and the guard would say it can't tell.
#[cfg(target_os = "linux")]
fn write_lock() -> libc::c_short {
    libc::c_short::try_from(libc::F_WRLCK).unwrap_or(libc::c_short::MAX)
}

/// A write lock's kind as a lock holds it: libc types it as an int on
/// Linux and as a short, which a lock holds, on a Mac.
#[cfg(not(target_os = "linux"))]
fn write_lock() -> libc::c_short {
    libc::F_WRLCK
}

/// Says once that the guard can't tell whether `path`'s database is
/// written, and why, and answers that nobody is.
fn blind(path: &Path, why: &str) -> bool {
    if !BLIND.swap(true, Ordering::Relaxed) {
        eprintln!(
            "disk-guard: can't tell whether another process writes Nix's database, \
             so deletes wait for none: {}: {why}",
            path.display()
        );
    }
    false
}
