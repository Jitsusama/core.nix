//! When each change was first held back for an earlier gate (`turn.rs`),
//! which its [`crate::turn::PATIENCE`] counts from. A change is known by its
//! patch id, the same across a rebase that met no conflict, so a change
//! that main moves under keeps its wait rather than starting it again.
//!
//! It's known by its tree too, since a patch id changes with an edit, a
//! rebase that met a conflict, or a stale origin's main in the asking tree:
//! on 16 November #5026 was held under one patch id at 09:38 and another
//! at 10:13, and heard "15 more minutes" anew, waiting 70 minutes in all.
//! A tree keeps its first hold as long as it keeps its place
//! (`places.rs`), so a new change in a tree a change merged from inherits
//! its wait, as it inherits its place.
//!
//! Counting from when its pass was kept let a change rebased onto an old
//! pass go ahead at once: on 15 November #4721, rebased 19 seconds before,
//! went ahead of an earlier gate reading the kit. Counting from its commit
//! instead, as #4740 did, started the wait again at each rebase: #4738,
//! behind a gate started 95 minutes before its own, heard "15 more
//! minutes" after each of the rebases main's moves forced, and #4731 the
//! same, so neither reached its turn while main moved faster than the wait.
//!
//! Kept in `waited.json` in the net's state, read, changed and written
//! under its lock (`whole.rs`), so two asking at once each keep their
//! hold. Where the lock can't be had, the one written last stands, and a
//! wait lost so starts again at the next hold, as it did before it was kept.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How long a change's first hold is kept: as long as a tree keeps its
/// place, past which no gate it waited on still holds it.
pub const KEPT: u64 = crate::places::KEPT;

/// When each change was first held, by its patch id and by its tree, in
/// seconds since the epoch.
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

    /// Keep `now` as when the change `patch` in `tree` was first held,
    /// unless an earlier hold is kept.
    pub fn held(&mut self, patch: Option<&str>, tree: &Path, now: u64) {
        for key in patch.map(str::to_owned).into_iter().chain([tree_key(tree)]) {
            let first = self.0.entry(key).or_insert(now);
            *first = (*first).min(now);
        }
    }

    /// When the change `patch` in `tree`, whose pass was `kept`, waits from
    /// by `now`: the earlier of its first hold and its tree's kept within
    /// [`KEPT`], or now, as it is held for the first time. A change whose
    /// patch couldn't be read and whose tree was never held waits from its
    /// pass, as before holds were kept.
    pub fn since(&self, patch: Option<&str>, tree: &Path, kept: Option<u64>, now: u64) -> u64 {
        let first = patch
            .map(str::to_owned)
            .into_iter()
            .chain([tree_key(tree)])
            .filter_map(|key| self.0.get(&key).copied())
            .filter(|first| now.saturating_sub(*first) < KEPT)
            .min();
        match (first, patch) {
            (Some(first), _) => first.min(now),
            (None, Some(_)) => now,
            (None, None) => kept.unwrap_or(now),
        }
    }

    /// Give up the holds kept past [`KEPT`] by `now`.
    pub fn tidy(&mut self, now: u64) {
        self.0.retain(|_, first| now.saturating_sub(*first) < KEPT);
    }

    /// Write them whole to `path`, so a reader never sees half.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.0).map_err(|error| error.to_string())?;
        crate::whole::write(path, &(text + "\n"))
    }
}

/// How a tree's first hold is kept, beside the patch ids.
fn tree_key(tree: &Path) -> String {
    format!("tree:{}", tree.display())
}

/// The patch id of the change at `commit`, from where it leaves origin's
/// main, as git says in the asking tree.
pub fn patch_of(commit: &str) -> Option<String> {
    let base = Command::new("git")
        .args(["merge-base", "refs/remotes/origin/main", commit])
        .output()
        .ok()
        .filter(|ran| ran.status.success())?;
    let base = String::from_utf8_lossy(&base.stdout).trim().to_owned();
    crate::patch_id(Path::new("."), &base, commit)
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

    fn tree(name: &str) -> PathBuf {
        PathBuf::from(format!("/trees/{name}"))
    }

    #[test]
    fn a_change_rebased_onto_an_old_pass_waits_from_its_first_hold() {
        // #4721: its pass kept two hours before, it was held for the first
        // time now, so it waits its whole patience.
        let waited = Waited::default();
        let since = waited.since(Some("4721"), &tree("a"), Some(NOW - 2 * 60 * 60), NOW);
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
        waited.held(Some("4738"), &tree("a"), NOW - PATIENCE - 60);
        waited.held(Some("4738"), &tree("a"), NOW - 120);
        let since = waited.since(Some("4738"), &tree("a"), Some(NOW - 60), NOW);
        assert_eq!(
            since,
            NOW - PATIENCE - 60,
            "a change rebased since it was first held started its wait again"
        );
        assert_eq!(
            waited.since(None, &tree("b"), Some(NOW - 60), NOW),
            NOW - 60,
            "a change whose patch couldn't be read didn't wait from its pass"
        );
    }

    #[test]
    fn a_change_whose_patch_changed_keeps_waiting_from_its_tree_s_first_hold() {
        // #5026 on 16 November: first held under one patch id at 09:38,
        // then under another once it was rebased with a new diff at 10:13,
        // so its wait started again, and it had waited since 09:20.
        let mut waited = Waited::default();
        waited.held(
            Some("5026-first"),
            &tree("machine-share"),
            NOW - PATIENCE - 60,
        );
        let since = waited.since(Some("5026-edited"), &tree("machine-share"), Some(NOW), NOW);
        assert_eq!(
            since,
            NOW - PATIENCE - 60,
            "a change whose patch changed in its tree started its wait again"
        );
        assert_eq!(
            waited.since(Some("other"), &tree("elsewhere"), None, NOW),
            NOW,
            "a change in another tree waited from another change's hold"
        );
    }

    #[test]
    fn holds_past_keeping_are_given_up_and_the_rest_written_whole() {
        let mut waited = Waited::default();
        waited.held(Some("old"), &tree("old"), NOW - KEPT);
        waited.held(Some("new"), &tree("new"), NOW - 60);
        waited.tidy(NOW);
        let folder = std::env::temp_dir().join(format!("waited-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("waited.json");
        waited.write(&path).unwrap();
        let read = Waited::read(&path);
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            read.since(Some("old"), &tree("old"), None, NOW),
            NOW,
            "a hold past keeping stood"
        );
        assert_eq!(
            read.since(Some("new"), &tree("new"), None, NOW),
            NOW - 60,
            "a hold within keeping was lost in writing"
        );
    }
}
