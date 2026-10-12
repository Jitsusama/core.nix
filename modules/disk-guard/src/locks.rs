//! The locks a producer takes, taken by the guard before it touches.
//!
//! Cargo locks each profile of a target with `flock`: `.cargo-build-lock`
//! in the build directory, `.cargo-lock` in the artifact directory, shared
//! in current cargo and exclusive in older ones, and `.cargo-artifact-lock`
//! (src/compiler/layout.rs). Holding every one exclusively means no cargo is
//! building into the target, and none can start until the guard lets go.
//!
//! rustc locks each incremental session, `s-{timestamp}-{random}-working`
//! and then `s-{timestamp}-{svh}`, with a POSIX record lock on the lock
//! file `s-{timestamp}-{random}.lock` beside it, and its own collection
//! leaves sessions under 10 s old (rustc_incremental's persist/fs.rs). The
//! guard takes an open-file-description lock, which conflicts with rustc's
//! but, unlike a POSIX lock, isn't dropped when any other descriptor the
//! guard has on the file closes.

use crate::anchor::Dir;
use rustix::fs::{FileType, FlockOperation};
use rustix::io::Errno;
use std::ffi::OsStr;
use std::io;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Locks the guard holds, released when this is dropped.
pub struct Held {
    files: Vec<OwnedFd>,
}

impl Held {
    pub fn count(&self) -> usize {
        self.files.len()
    }
}

pub enum Hold {
    Held(Held),
    /// Something holds this lock, relative to the target, or it can't be
    /// judged, as a symlink named like a lock can't.
    Busy(PathBuf),
}

const CARGO_LOCKS: [&str; 3] = [".cargo-lock", ".cargo-build-lock", ".cargo-artifact-lock"];

/// A profile sits in the target, or in a directory named for a target
/// triple inside it, so locks are no deeper than this.
const LOCK_DEPTH: usize = 3;

/// Directories cargo fills with build output and never puts a lock in, so
/// a walk for locks doesn't read through thousands of artifacts.
const NO_LOCKS: [&str; 6] = [
    "deps",
    "build",
    "incremental",
    ".fingerprint",
    "examples",
    "criterion",
];

/// rustc collects no session younger than this, since one being created
/// may not have its lock yet.
const SESSION_SETTLES: Duration = Duration::from_secs(10);

/// Takes the lock on one incremental session in a crate's directory.
/// A session whose name rustc wouldn't give, or that is younger than rustc
/// would collect, is busy. One with no lock file is held with nothing to
/// hold, since rustc itself deletes such a session.
pub fn hold_session(crate_dir: &Dir, session: &OsStr, now: SystemTime) -> io::Result<Hold> {
    let busy = || Ok(Hold::Busy(PathBuf::from(session)));
    let Some((prefix, made)) = session.to_str().and_then(session_parts) else {
        return busy();
    };
    if now
        .duration_since(made)
        .map_or(true, |age| age < SESSION_SETTLES)
    {
        return busy();
    }
    let lock = format!("{prefix}.lock");
    let file = match crate_dir.file(OsStr::new(&lock), true) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Hold::Held(Held { files: Vec::new() }));
        }
        Err(_) => return Ok(Hold::Busy(PathBuf::from(lock))),
    };
    if lock_whole(&file)? {
        Ok(Hold::Held(Held { files: vec![file] }))
    } else {
        Ok(Hold::Busy(PathBuf::from(lock)))
    }
}

/// A session's lock file prefix and when it was made, from its name.
fn session_parts(name: &str) -> Option<(&str, SystemTime)> {
    let dashes: Vec<usize> = name.match_indices('-').map(|(at, _)| at).collect();
    if !name.starts_with("s-") || dashes.len() != 3 {
        return None;
    }
    let micros = u64::from_str_radix(&name[dashes[0] + 1..dashes[1]], 36).ok()?;
    Some((
        &name[..dashes[2]],
        UNIX_EPOCH + Duration::from_micros(micros),
    ))
}

/// An exclusive open-file-description lock on the whole file, without
/// waiting: true if taken.
fn lock_whole(file: &OwnedFd) -> io::Result<bool> {
    // SAFETY: flock is plain data, for which zero is a valid value.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = libc::F_WRLCK as _;
    lock.l_whence = libc::SEEK_SET as _;
    // SAFETY: the descriptor is open for the call, and lock outlives it.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &lock) } == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::EAGAIN | libc::EACCES) => Ok(false),
        _ => Err(error),
    }
}

/// Takes every cargo lock in a target exclusively, without waiting.
pub fn hold_cargo(target: &Dir) -> io::Result<Hold> {
    let mut held = Held { files: Vec::new() };
    match hold_beneath(target, Path::new(""), 0, &mut held)? {
        Some(busy) => Ok(Hold::Busy(busy)),
        None => Ok(Hold::Held(held)),
    }
}

fn hold_beneath(
    dir: &Dir,
    at: &Path,
    depth: usize,
    held: &mut Held,
) -> io::Result<Option<PathBuf>> {
    let mut names = dir.names()?;
    names.sort();
    for name in names {
        let name = OsStr::from_bytes(&name);
        let path = at.join(name);
        let Some(kind) = dir.kind(name) else { continue };
        if CARGO_LOCKS.iter().any(|lock| OsStr::new(lock) == name) {
            let Ok(file) = dir.file(name, false) else {
                return Ok(Some(path));
            };
            match rustix::fs::flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => held.files.push(file),
                Err(Errno::WOULDBLOCK) => return Ok(Some(path)),
                Err(error) => return Err(error.into()),
            }
        } else if kind == FileType::Directory
            && depth + 1 < LOCK_DEPTH
            && !NO_LOCKS.iter().any(|skip| OsStr::new(skip) == name)
            && let Ok(Some(child)) = dir.entry(name)
            && let Some(busy) = hold_beneath(&child, &path, depth + 1, held)?
        {
            return Ok(Some(busy));
        }
    }
    Ok(None)
}
