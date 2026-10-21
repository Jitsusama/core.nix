//! What the integration tests share: a scratch directory each, under
//! cargo's own target directory rather than the system's temporary one,
//! and gone once its test is, whether the test passed or not, since a
//! failing one unwinds through the drop too.

use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("guide-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        Scratch(directory)
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

impl From<&Scratch> for PathBuf {
    fn from(scratch: &Scratch) -> PathBuf {
        scratch.0.clone()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
