//! When each change was first held back for an earlier gate (`turn.rs`),
//! which its [`crate::turn::PATIENCE`] counts from. A change is known by its
//! patch id, the same across a rebase that met no conflict, so a change
//! that main moves under keeps its wait rather than starting it again.
//!
//! Counting from when its pass was kept let a change rebased onto an old
//! pass go ahead at once: on 15 November #4721, rebased 19 seconds before,
//! went ahead of an earlier gate reading the kit. Counting from its commit
//! instead, as #4740 did, started the wait again at each rebase: #4738,
//! behind a gate started 95 minutes before its own, heard "15 more
//! minutes" after each of the rebases main's moves forced, and #4731 the
//! same, so neither reached its turn while main moved faster than the wait.
//!
//! Kept in `waited.json` in the net's state. Two asking at once may each
//! write without the other's, and the one written last stands; a wait lost
//! so starts again at the next hold, as it did before it was kept.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// How long a change's first hold is kept: as long as a tree keeps its
/// place, past which no gate it waited on still holds it.
pub const KEPT: u64 = crate::places::KEPT;

/// When each change was first held, by its patch id, in seconds since the
/// epoch.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Waited(BTreeMap<String, u64>);

impl Waited {
    /// The holds kept at `path`, or none where there is no file or it
    /// doesn't read.
    pub fn read(path: &Path) -> Self {
        let said: BTreeMap<String, u64> = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self(said)
    }

    /// Keep `now` as when the change `patch` was first held, unless an
    /// earlier hold is kept.
    pub fn held(&mut self, patch: &str, now: u64) {
        let first = self.0.entry(patch.to_owned()).or_insert(now);
        *first = (*first).min(now);
    }

    /// When the change `patch`, whose pass was `kept`, waits from by `now`.
    pub fn since(&self, patch: Option<&str>, kept: Option<u64>, now: u64) -> u64 {
        let _ = patch;
        kept.unwrap_or(now)
    }

    /// Give up the holds kept past [`KEPT`] by `now`.
    pub fn tidy(&mut self, now: u64) {
        self.0.retain(|_, first| now.saturating_sub(*first) < KEPT);
    }

    /// Write them whole to `path`, by a rename, so a reader never sees half.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let partial = path.with_extension("partial");
        let text = serde_json::to_string_pretty(&self.0).map_err(|error| error.to_string())?;
        fs::write(&partial, text + "\n")
            .and_then(|()| fs::rename(&partial, path))
            .map_err(|error| format!("writing {}: {error}", path.display()))
    }
}

/// Where the holds are kept: beside the rest of the net's state.
pub fn file() -> PathBuf {
    crate::state::State::from_env().root.join("waited.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn::PATIENCE;

    const NOW: u64 = 1_794_750_000;

    #[test]
    fn a_change_rebased_onto_an_old_pass_waits_from_its_first_hold() {
        // #4721: its pass kept two hours before, it was held for the first
        // time now, so it waits its whole patience.
        let waited = Waited::default();
        let since = waited.since(Some("4721"), Some(NOW - 2 * 60 * 60), NOW);
        assert_eq!(
            since, NOW,
            "a change held for the first time waited from its old pass, not from now"
        );
    }

    #[test]
    fn a_change_main_moves_under_keeps_waiting_from_its_first_hold() {
        // #4738: first held, then rebased again and again, each rebase a new
        // pass kept a minute before it asked again.
        let mut waited = Waited::default();
        waited.held("4738", NOW - PATIENCE - 60);
        waited.held("4738", NOW - 120);
        let since = waited.since(Some("4738"), Some(NOW - 60), NOW);
        assert_eq!(
            since,
            NOW - PATIENCE - 60,
            "a change rebased since it was first held started its wait again"
        );
        assert_eq!(
            waited.since(None, Some(NOW - 60), NOW),
            NOW - 60,
            "a change whose patch couldn't be read didn't wait from its pass"
        );
    }

    #[test]
    fn holds_past_keeping_are_given_up_and_the_rest_written_whole() {
        let mut waited = Waited::default();
        waited.held("old", NOW - KEPT);
        waited.held("new", NOW - 60);
        waited.tidy(NOW);
        let folder = std::env::temp_dir().join(format!("waited-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("waited.json");
        waited.write(&path).unwrap();
        let read = Waited::read(&path);
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(read.since(Some("old"), None, NOW), NOW, "a hold past keeping stood");
        assert_eq!(
            read.since(Some("new"), None, NOW),
            NOW - 60,
            "a hold within keeping was lost in writing"
        );
    }
}
