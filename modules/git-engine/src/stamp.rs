//! What tells the engine a file it read before has changed: its size, when
//! it was last changed and which inode it is. A rename over it or a write in
//! place changes one of them, so anything read from a file is kept until its
//! stamp moves.

use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stamp {
    size: u64,
    changed: (i64, i64),
    modified: (i64, i64),
    inode: u64,
}

impl Stamp {
    /// The file's stamp, or None when it isn't there.
    pub fn of(path: &Path) -> Option<Stamp> {
        std::fs::metadata(path)
            .ok()
            .map(|meta| Stamp::from_meta(&meta))
    }

    pub fn from_meta(meta: &std::fs::Metadata) -> Stamp {
        Stamp {
            size: meta.size(),
            changed: (meta.ctime(), meta.ctime_nsec()),
            modified: (meta.mtime(), meta.mtime_nsec()),
            inode: meta.ino(),
        }
    }

    /// Whether the file changed so recently that another change in the same
    /// tick of the clock wouldn't move its stamp, so nothing read from it can
    /// be trusted to stay current.
    pub fn is_recent(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
            });
        now - self.changed.0.max(self.modified.0) < 2
    }
}

/// The stamps of a set of files, any of them missing, compared as one.
pub type Stamps = Vec<(std::path::PathBuf, Option<Stamp>)>;

pub fn still(stamps: &Stamps) -> bool {
    stamps.iter().all(|(path, stamp)| Stamp::of(path) == *stamp)
}
