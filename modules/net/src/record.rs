//! The record net keeps on the disk for the views that read it, kept so a
//! minute that can't be written costs only the file. On 24 Nov the runtime
//! folder filled, the write failed, and net stopped; its listeners, which
//! are served from memory, lost it for the minute systemd took to start it
//! again, along with every source's carried state. Now the daemon keeps
//! serving, says once that the record can't be kept, and says again when
//! it is.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The record's file, and whether the last minute's couldn't be kept.
pub struct Record {
    file: PathBuf,
    failing: bool,
}

impl Record {
    pub const fn at(file: PathBuf) -> Self {
        Self {
            file,
            failing: false,
        }
    }

    /// Writes `contents` whole, answering with what to say of it: the
    /// first failure of a spell, and the first write after one. A failed
    /// write takes its partial file with it, since on a full disk that
    /// file is what holds the space.
    pub fn keep(&mut self, contents: &str) -> Option<String> {
        match write(&self.file, contents) {
            Ok(()) if self.failing => {
                self.failing = false;
                Some(format!(
                    "the record is kept again at {}",
                    self.file.display()
                ))
            }
            Ok(()) => None,
            Err(_) if self.failing => None,
            Err(error) => {
                self.failing = true;
                Some(format!(
                    "can't write the record at {}: {error}; serving on from memory",
                    self.file.display()
                ))
            }
        }
    }
}

/// Writes a file whole, so a view never reads half of one.
fn write(file: &Path, contents: &str) -> io::Result<()> {
    let fresh = file.with_extension("json.new");
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
