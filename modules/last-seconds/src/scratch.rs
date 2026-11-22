//! A folder of its own for each test's stand-ins and rings.
//!
//! The tests run in a build's sandbox, whose temporary folder is its own,
//! and by `cargo test` while a crate is edited, whose temporary folder
//! every account shares. There a name another account can guess, such as
//! one by the process's id alone, could already be taken: by a folder a
//! killed run left, whose records would read as the new run's, or by
//! another account's folder or link, which a test would read stand-ins
//! from or write its ring through.

use std::fs::{self, DirBuilder};
use std::io;
use std::ops::Deref;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A new, empty folder in the temporary folder that only the running
/// account can open, cleared away with all it holds when dropped, so a
/// test that fails leaves nothing behind either.
#[derive(Debug)]
pub struct Folder(PathBuf);

impl Folder {
    /// Makes one named for `name`. A name already there, whatever it is,
    /// is passed over for another rather than used, so nothing is in the
    /// folder but what the test puts there.
    ///
    /// # Errors
    ///
    /// When no folder can be made there at all.
    pub fn make(name: &str) -> io::Result<Self> {
        static MADE: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir();
        let mut maker = DirBuilder::new();
        maker.mode(0o700);
        loop {
            // What keeps the folder the test's own is that a name already
            // there, a link included, is never used: making it fails, and
            // another name is tried. The process's id and a count keep this
            // run's names apart, and the clock's nanoseconds make one hard
            // to guess, so a name taken first is rare.
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.subsec_nanos());
            let made = MADE.fetch_add(1, Ordering::Relaxed);
            let folder = base.join(format!(
                "last-seconds-{name}-{}-{made}-{nanos}",
                std::process::id()
            ));
            match maker.create(&folder) {
                Ok(()) => return Ok(Self(folder)),
                Err(taken) if taken.kind() == io::ErrorKind::AlreadyExists => {}
                Err(failed) => return Err(failed),
            }
        }
    }
}

impl Deref for Folder {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Folder {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
