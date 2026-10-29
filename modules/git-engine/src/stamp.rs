//! What tells the engine a file it read before has changed: its size, when
//! it was last changed and which inode it is. A rename over it or a write in
//! place changes one of them, so anything read from a file is kept until its
//! stamp moves. But a file's clock ticks coarsely, so a change within a tick
//! of the last moves nothing, and a folder's size may not move at all: so a
//! stamp taken that soon after its file's change is never trusted, and what
//! was read is read again, as git does with its index.

use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stamp {
    size: u64,
    changed: (i64, i64),
    modified: (i64, i64),
    inode: u64,
    /// Whether it was taken long enough after the file's last change to be
    /// trusted (`still`).
    settled: bool,
}

impl Stamp {
    /// The file's stamp, or None when it isn't there.
    pub fn of(path: &Path) -> Option<Stamp> {
        std::fs::metadata(path)
            .ok()
            .map(|meta| Stamp::from_meta(&meta))
    }

    pub fn from_meta(meta: &std::fs::Metadata) -> Stamp {
        let mut stamp = Stamp {
            size: meta.size(),
            changed: (meta.ctime(), meta.ctime_nsec()),
            modified: (meta.mtime(), meta.mtime_nsec()),
            inode: meta.ino(),
            settled: false,
        };
        stamp.settled = !stamp.is_recent();
        stamp
    }

    /// Whether the file changed so recently that another change in the same
    /// tick of the clock wouldn't move its stamp, so nothing read from it can
    /// be trusted to stay current.
    pub fn is_recent(&self) -> bool {
        now() - self.changed.0.max(self.modified.0) < 2
    }
}

/// Now, in seconds since the epoch; a test may move its own clock on
/// ([`tests::ahead`]).
fn now() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    #[cfg(test)]
    let now = now + tests::AHEAD.get();
    now
}

/// The stamps of a set of files, any of them missing, compared as one.
pub type Stamps = Vec<(std::path::PathBuf, Option<Stamp>)>;

/// Whether every file in `stamps` is as it was when they were taken: each
/// stamp the same, and none taken so soon after its file's change that
/// another change may have left it the same.
pub fn still(stamps: &Stamps) -> bool {
    stamps
        .iter()
        .all(|(path, stamp)| Stamp::of(path) == *stamp && stamp.is_none_or(|stamp| stamp.settled))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        /// How far ahead of the real clock this test's clock runs.
        pub static AHEAD: Cell<i64> = const { Cell::new(0) };
    }

    /// Runs this test's clock `seconds` ahead, as if what follows came that
    /// long after the files it reads were last changed.
    pub fn ahead(seconds: i64) {
        AHEAD.set(seconds);
    }

    fn file(name: &str) -> std::path::PathBuf {
        let file =
            std::env::temp_dir().join(format!("git-engine-stamp-{name}-{}", std::process::id()));
        std::fs::write(&file, b"one").unwrap();
        file
    }

    #[test]
    fn a_stamp_taken_within_a_tick_of_a_change_is_not_trusted() {
        let file = file("recent");
        let stamps = vec![(file.clone(), Stamp::of(&file))];
        assert!(
            !still(&stamps),
            "another change in the same tick would leave the stamp the same"
        );
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn a_stamp_taken_after_a_tick_is_trusted_while_nothing_changes() {
        let file = file("settled");
        ahead(3);
        let stamps = vec![(file.clone(), Stamp::of(&file))];
        assert!(still(&stamps), "a settled stamp of an unchanged file holds");
        std::fs::write(&file, b"three").unwrap();
        assert!(!still(&stamps), "a change that moves the stamp is seen");
        let _ = std::fs::remove_file(&file);
    }
}
