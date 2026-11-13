//! What the service says of itself beside its state, for the launcher's
//! Dictation row to show in its drawer: when a take last typed something,
//! and how long the model last took to load. The last take outlives the
//! service, kept in one small file under `$XDG_STATE_HOME/dictation`, so
//! a restart or a reboot doesn't make it read as never used.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the last take is kept: `$XDG_STATE_HOME`, else `~/.local/state`,
/// or nowhere when neither is known.
pub fn place(state_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    state_home
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|home| home.join(".local/state")))
        .map(|state| state.join("dictation/last-take"))
}

/// The file this service keeps its last take in.
pub fn here() -> Option<PathBuf> {
    place(
        std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

/// The last take's time, in ms since 1970, as the file says it, or none
/// when there's no file or it says nothing a time reads as.
pub fn read(file: &Path) -> Option<u64> {
    std::fs::read_to_string(file).ok()?.trim().parse().ok()
}

/// Keeps a take's time, written beside the file and moved over it so a
/// read never sees half of it.
pub fn keep(file: &Path, at: u64) -> Result<(), String> {
    let folder = file.parent().ok_or("the last take's file has no folder")?;
    std::fs::create_dir_all(folder).map_err(|e| format!("making {}: {e}", folder.display()))?;
    let new = file.with_extension("new");
    std::fs::write(&new, format!("{at}\n"))
        .map_err(|e| format!("writing {}: {e}", new.display()))?;
    std::fs::rename(&new, file).map_err(|e| format!("keeping {}: {e}", file.display()))
}

/// Now, in ms since 1970.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The line a watcher is told a take typed at `at`.
pub fn last_take(at: u64) -> String {
    format!("last-take {at}")
}

/// The line a watcher is told the model loaded in `ms`.
pub fn ready(ms: u128) -> String {
    format!("ready-ms {ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_take_is_kept_under_the_state_home_else_home() {
        assert_eq!(
            place(Some("/s".into()), Some("/h".into())),
            Some(PathBuf::from("/s/dictation/last-take"))
        );
        // A relative XDG_STATE_HOME is to be ignored, as the spec says.
        assert_eq!(
            place(Some("s".into()), Some("/h".into())),
            Some(PathBuf::from("/h/.local/state/dictation/last-take"))
        );
        assert_eq!(place(None, None), None);
    }

    #[test]
    fn a_kept_take_reads_back_and_a_missing_one_reads_as_none() {
        let folder = std::env::temp_dir().join(format!("dictation-kept-{}", std::process::id()));
        let file = folder.join("dictation/last-take");
        assert_eq!(read(&file), None);
        keep(&file, 1_794_606_000_000).expect("keeping the take");
        assert_eq!(read(&file), Some(1_794_606_000_000));
        keep(&file, 1_794_607_000_000).expect("keeping it again");
        assert_eq!(read(&file), Some(1_794_607_000_000));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn each_line_says_its_word_then_its_figure() {
        assert_eq!(last_take(12), "last-take 12");
        assert_eq!(ready(1460), "ready-ms 1460");
    }
}
