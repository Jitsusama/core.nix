//! The record eero keeps on the disk for net and the shell, kept so a cycle
//! that can't be written costs only the file. On 24 Nov the runtime folder
//! filled, the write failed, and eero stopped; its listeners, which are
//! sent the model from memory, lost it for the minute systemd took to
//! start it again, and the daemon began its schedule afresh. Now it keeps
//! serving, says once that the record can't be kept, again when the
//! cause changes, and again when it is. net judges the record by its
//! `read`, so one left stale through a spell is never taken as now.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The record's file, and why the last cycle's couldn't be kept.
pub struct Record {
    file: PathBuf,
    failing: Option<io::ErrorKind>,
}

impl Record {
    pub const fn at(file: PathBuf) -> Self {
        Self {
            file,
            failing: None,
        }
    }

    /// Writes `contents` whole, answering with what to say of it: the
    /// first failure of a spell, a failure of another kind than the last,
    /// since its cause changed, and the first write after one.
    pub fn keep(&mut self, contents: &str) -> Option<String> {
        match write(&self.file, contents) {
            Ok(()) => self
                .failing
                .take()
                .map(|_| format!("eero: the record is kept again at {}", self.file.display())),
            Err(error) if self.failing == Some(error.kind()) => None,
            Err(error) => {
                self.failing = Some(error.kind());
                Some(format!(
                    "eero: can't write the record at {}: {error}; serving on from memory",
                    self.file.display()
                ))
            }
        }
    }
}

/// Writes a file whole, by way of `<file>.new`, so the shell never reads
/// half of one. Its folder is made again when it's gone, as a runtime
/// folder cleared under a running daemon is, which a restart used to mend.
/// A failed write takes its partial file with it, since on a full disk
/// that file is what holds the space.
fn write(file: &Path, contents: &str) -> io::Result<()> {
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
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
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

    #[test]
    fn a_folder_cleared_away_is_made_again() {
        let folder = folder("cleared");
        let file = folder.join("eero").join("home-network.json");
        let mut record = Record::at(file.clone());
        assert_eq!(record.keep("{}"), None, "the first record was spoken of");
        fs::remove_dir_all(folder.join("eero")).expect("the folder cleared");
        assert_eq!(
            record.keep("{}"),
            None,
            "a record whose folder was cleared away couldn't be kept"
        );
        assert!(file.exists(), "the record wasn't written again");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_failure_that_changes_cause_is_said_again_and_the_last_record_stays_whole() {
        let folder = folder("changed");
        let file = folder.join("home-network.json");
        let mut record = Record::at(file.clone());
        assert_eq!(record.keep("last"), None, "the first record was spoken of");
        // A folder that can't be written to refuses the partial file
        // itself, as a full disk refuses it before a byte is written.
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o500)).expect("the folder shut");
        if fs::write(folder.join("probe"), "").is_ok() {
            // Run as root, which no folder's mode refuses: nothing to show.
            let _ = fs::remove_file(folder.join("probe"));
            fs::set_permissions(&folder, fs::Permissions::from_mode(0o700))
                .expect("the folder open");
            let _ = fs::remove_dir_all(&folder);
            return;
        }
        let refused = record.keep("next");
        assert!(
            refused
                .as_deref()
                .is_some_and(|said| said.contains("can't write the record")),
            "a refused write wasn't said: {refused:?}"
        );
        assert_eq!(
            fs::read_to_string(&file).expect("the last record"),
            "last",
            "a refused write cost the last record"
        );
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).expect("the folder open");
        fs::remove_file(&file).expect("the record taken away");
        fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
        assert!(
            record.keep("next").is_some(),
            "a failure of another cause went unsaid in the same spell"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
