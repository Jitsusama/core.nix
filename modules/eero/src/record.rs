//! The record eero keeps on the disk for net and the shell, kept so a cycle
//! that can't be written costs only the file. On 24 Nov the runtime folder
//! filled, the write failed, and eero stopped; its listeners, which are
//! sent the model from memory, lost it for the minute systemd took to
//! start it again, and the daemon began its schedule afresh. Now it keeps
//! serving, says once that the record can't be kept, and says again when
//! it is.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The record's file, and whether the last cycle's couldn't be kept.
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
                    "eero: the record is kept again at {}",
                    self.file.display()
                ))
            }
            Ok(()) => None,
            Err(_) if self.failing => None,
            Err(error) => {
                self.failing = true;
                Some(format!(
                    "eero: can't write the record at {}: {error}; serving on from memory",
                    self.file.display()
                ))
            }
        }
    }
}

/// Writes a file whole, so the shell never reads half of one.
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
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::Record;

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("eero-record-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder for the test");
        folder
    }

    #[test]
    fn a_record_is_written_whole_and_said_nothing_of() {
        let folder = folder("whole");
        let file = folder.join("home-network.json");
        let mut record = Record::at(file.clone());
        assert_eq!(record.keep("{}"), None, "a kept record was spoken of");
        assert_eq!(
            fs::read_to_string(&file).expect("the record"),
            "{}",
            "the record isn't what was kept"
        );
        assert!(
            !folder.join("home-network.json.new").exists(),
            "the partial file outlived a write"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_record_that_cant_be_written_is_said_once_and_leaves_no_partial() {
        let folder = folder("refused");
        // A folder where the record goes can't be renamed over, so the
        // partial file is written and the rename fails, as a full disk
        // fails partway through.
        let file = folder.join("home-network.json");
        fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
        let mut record = Record::at(file);
        let first = record.keep("{}");
        assert!(
            first
                .as_deref()
                .is_some_and(|said| said.contains("can't write the record")),
            "the first failure wasn't said: {first:?}"
        );
        assert_eq!(
            record.keep("{}"),
            None,
            "a failure was said twice in one spell"
        );
        assert!(
            !folder.join("home-network.json.new").exists(),
            "a failed write left its partial file holding the space"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_record_kept_after_a_failure_says_so() {
        let folder = folder("again");
        let file = folder.join("home-network.json");
        fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
        let mut record = Record::at(file.clone());
        assert!(record.keep("{}").is_some(), "the failure wasn't said");
        fs::remove_dir_all(&file).expect("the folder taken away");
        let again = record.keep("{}");
        assert!(
            again
                .as_deref()
                .is_some_and(|said| said.contains("kept again")),
            "the record kept again wasn't said: {again:?}"
        );
        assert_eq!(
            record.keep("{}"),
            None,
            "a kept record was spoken of after the spell"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
