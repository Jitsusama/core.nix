//! Where the plan's sign-in is kept: the Claude Code config folder pi-claude
//! signs in to (`modules/pi/claude.sh`).

use std::path::{Path, PathBuf};

/// The config folder the feed is given, else the one pi-claude is told of
/// in `PI_CLAUDE_CONFIG_DIR`, else pi-claude's own `~/.claude-personal`
/// where there is one, else Claude Code's own `~/.claude`, on a machine whose
/// only login is the plan's. An empty name is no name.
pub fn config_folder(given: Option<PathBuf>, named: Option<PathBuf>, home: &Path) -> PathBuf {
    let personal = home.join(".claude-personal");
    given
        .into_iter()
        .chain(named)
        .find(|folder| !folder.as_os_str().is_empty())
        .or_else(|| personal.is_dir().then_some(personal))
        .unwrap_or_else(|| home.join(".claude"))
}
