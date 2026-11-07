//! A folder of one test's own, gone once the test ends.
//!
//! The tests run with `$TMPDIR` set to a quest's scratch, which stays until
//! the quest ends, so a folder a test left behind stayed for weeks: on 7
//! November 2,137 of them sat in quests' scratch, each one more for any walk
//! through it to open. Removing the folder on drop removes it even when the
//! test fails.

use std::ops::Deref;
use std::path::{Path, PathBuf};

pub struct Scratch(PathBuf);

impl Scratch {
    /// An empty folder named for this run and the test, not yet made.
    pub fn new(test: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("changed-checks-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        Self(folder)
    }
}

impl Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scratch_folder_goes_when_its_test_ends() {
        let scratch = Scratch::new("goes");
        std::fs::create_dir_all(scratch.join("deep/er")).expect("made");
        let made = scratch.to_path_buf();
        drop(scratch);
        assert!(!made.exists());
    }

    #[test]
    fn a_scratch_folder_goes_when_its_test_fails() {
        let made = Scratch::new("fails").to_path_buf();
        let failed = std::panic::catch_unwind(|| {
            let scratch = Scratch::new("fails");
            std::fs::create_dir_all(scratch.join("deep")).expect("made");
            panic!("the test failed");
        });
        assert!(failed.is_err());
        assert!(!made.exists());
    }
}
