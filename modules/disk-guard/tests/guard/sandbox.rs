//! A temp directory that can always be removed, whatever a test made
//! read-only in it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub(crate) struct Sandbox(tempfile::TempDir);

impl Sandbox {
    pub(crate) fn new() -> Self {
        Sandbox(tempfile::tempdir().unwrap())
    }

    pub(crate) fn path(&self) -> &Path {
        self.0.path()
    }
}

/// Gives the owner everything on each directory, never through a symlink.
fn open_up(dir: &Path) {
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            open_up(&entry.path());
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        open_up(self.0.path());
    }
}
