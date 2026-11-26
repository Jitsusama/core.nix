//! The files net keeps on the disk, kept so a minute that can't be written
//! costs only the file. On 24 Nov the runtime folder filled, the write
//! failed, and net stopped; its listeners, which are served from memory,
//! lost it for the minute systemd took to start it again, along with every
//! source's carried state. Now the daemon and the switch's reader keep
//! going, say once that a file can't be kept, again when the cause
//! changes, and again when it is kept. What reads the files judges them by
//! their age, so one left stale through a spell is never taken as now.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A file kept each round, what it's called in what's said of it, what
/// goes on while it can't be kept, and why the last round's couldn't be.
pub struct Record {
    file: PathBuf,
    what: &'static str,
    meanwhile: &'static str,
    failing: Option<io::ErrorKind>,
}

impl Record {
    /// The daemon's record of the network, which the views read.
    pub const fn at(file: PathBuf) -> Self {
        Self::named(file, "the record", "serving on from memory")
    }

    /// A file kept each round, called `what` in what's said of it, with
    /// `meanwhile` saying what goes on while it can't be kept.
    pub const fn named(file: PathBuf, what: &'static str, meanwhile: &'static str) -> Self {
        Self {
            file,
            what,
            meanwhile,
            failing: None,
        }
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Writes `contents` whole, answering with what to say of it, as
    /// `heard` says.
    pub fn keep(&mut self, contents: &str) -> Option<String> {
        let written = write(&self.file, contents);
        self.heard(&written)
    }

    /// What to say of a round's write, however it was made: the first
    /// failure of a spell, a failure of another kind than the last, since
    /// its cause changed, and the first write after one.
    pub fn heard(&mut self, written: &io::Result<()>) -> Option<String> {
        match written {
            Ok(()) => self
                .failing
                .take()
                .map(|_| format!("{} is kept again at {}", self.what, self.file.display())),
            Err(error) if self.failing == Some(error.kind()) => None,
            Err(error) => {
                self.failing = Some(error.kind());
                Some(format!(
                    "can't write {} at {}: {error}; {}",
                    self.what,
                    self.file.display(),
                    self.meanwhile
                ))
            }
        }
    }
}

/// Writes a file whole, by way of `<file>.new`, so a reader never reads
/// half of one. Its folder is made again when it's gone, as a runtime
/// folder cleared under a running daemon is, which a restart used to mend.
/// A failed write takes its partial file with it, since on a full disk
/// that file is what holds the space.
pub fn write(file: &Path, contents: &str) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut fresh = OsString::from(file.as_os_str());
    fresh.push(".new");
    let fresh = PathBuf::from(fresh);
    let written = fs::write(&fresh, contents).and_then(|()| fs::rename(&fresh, file));
    if written.is_err() {
        // Gone already when the folder itself couldn't be written to.
        let _ = fs::remove_file(&fresh);
    }
    written
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
