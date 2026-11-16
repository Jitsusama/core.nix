//! Where repos keeps what its timers learn between readings, GitHub's
//! changes (github.rs) and how far each head is from its branch
//! (upstreams.rs), and how each file is written: whole or not at all,
//! and readable by its owner alone, since both name private work.

use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The file `name` in repos's state folder: `$XDG_STATE_HOME/repos`, else
/// under `home`'s ~/.local/state; none when neither is set.
pub fn file(home: Option<&Path>, name: &str) -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .filter(|state| !state.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".local/state")))
        .map(|state| state.join("repos").join(name))
}

/// `bytes` written at `file` through a file beside it, renamed over it, so
/// a reading never finds half of one.
pub fn write(file: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let beside = file.with_extension(format!("json.{}", std::process::id()));
    let mut written = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&beside)?;
    written.write_all(bytes)?;
    written.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&beside, file)
}
