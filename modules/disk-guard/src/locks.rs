//! The locks a producer takes, taken by the guard before it touches.
//!
//! Cargo locks each profile of a target with `flock`: `.cargo-build-lock`
//! in the build directory, `.cargo-lock` in the artifact directory, shared
//! in current cargo and exclusive in older ones, and `.cargo-artifact-lock`
//! (src/compiler/layout.rs). Holding every one exclusively means no cargo is
//! building into the target, and none can start until the guard lets go.

use crate::anchor::Dir;
use rustix::fs::{FileType, FlockOperation};
use rustix::io::Errno;
use std::ffi::OsStr;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

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
            let Ok(file) = dir.file(name) else {
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
