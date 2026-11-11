//! Where each tree stands in the lease's order (`turn.rs`): the start of
//! the earliest gate seen in it. A change whose pass a merge leaves stale
//! runs again, and that run starts after every gate already going on its
//! files, so ordering by each run's own start put it behind all of them
//! each time: on 11 November #3027 lost nine gates from 22:40 on to merges
//! of gates started before its rerun, until merges into its files were
//! held by hand. Kept in the tree's place, its rerun goes ahead of those
//! that started after its first gate, and behind only those before it.
//!
//! Each `ready` keeps the start of every gate going and of its own pass,
//! in `places.json` in the net's state. Two asking at once may each write
//! a place the other read without, and the one written last stands; a
//! place lost so is kept again by the next `ready` that sees its gate.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// How long a tree keeps its place: twice the longest a gate is waited
/// on, so a change that loses two long gates in a row keeps it, while a
/// tree left standing gives it up within the night.
pub const KEPT: u64 = 2 * crate::turn::LONGEST;

/// The start each tree keeps, by its folder, in seconds since the epoch.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Places(BTreeMap<PathBuf, u64>);

impl Places {
    /// The places kept at `path`, or none where there is no file or it
    /// doesn't read.
    pub fn read(path: &Path) -> Self {
        let said: BTreeMap<PathBuf, u64> = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self(said)
    }

    /// Keep `started` as the place of the tree at `folder`, unless it
    /// already keeps an earlier one.
    pub fn keep(&mut self, folder: &Path, started: u64) {
        let place = self.0.entry(folder.to_path_buf()).or_insert(started);
        *place = (*place).min(started);
    }

    /// When a gate in `folder` that started at `started` counts as having
    /// started, by `now`: the earlier of its start and its tree's place.
    pub fn start(&self, folder: &Path, started: u64, now: u64) -> u64 {
        self.0
            .get(folder)
            .filter(|place| now.saturating_sub(**place) < KEPT)
            .map_or(started, |place| (*place).min(started))
    }

    /// Give up the places kept past [`KEPT`] by `now`, and those of trees
    /// that, as `stands` says, are gone.
    pub fn tidy(&mut self, now: u64, stands: &dyn Fn(&Path) -> bool) {
        self.0
            .retain(|folder, place| now.saturating_sub(*place) < KEPT && stands(folder));
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

/// Where the places are kept: beside the rest of the net's state.
pub fn file() -> PathBuf {
    crate::state::State::from_env().root.join("places.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 100_000;

    #[test]
    fn a_tree_keeps_the_start_of_its_earliest_gate() {
        let mut places = Places::default();
        places.keep(Path::new("/a"), NOW - 300);
        places.keep(Path::new("/a"), NOW - 500);
        places.keep(Path::new("/a"), NOW - 400);
        assert_eq!(
            places.start(Path::new("/a"), NOW - 10, NOW),
            NOW - 500,
            "a rerun didn't keep its tree's first start"
        );
        assert_eq!(
            places.start(Path::new("/b"), NOW - 10, NOW),
            NOW - 10,
            "a tree with no place took another's"
        );
        assert_eq!(
            places.start(Path::new("/a"), NOW - 600, NOW),
            NOW - 600,
            "a gate earlier than its tree's place was taken as later"
        );
    }

    #[test]
    fn a_place_past_its_keeping_or_of_a_tree_gone_counts_for_nothing() {
        let mut places = Places::default();
        places.keep(Path::new("/old"), NOW - KEPT);
        places.keep(Path::new("/gone"), NOW - 10);
        places.keep(Path::new("/here"), NOW - 20);
        assert_eq!(
            places.start(Path::new("/old"), NOW - 5, NOW),
            NOW - 5,
            "a place kept past its time still counted"
        );
        places.tidy(NOW, &|folder| folder != Path::new("/gone"));
        let mut left = Places::default();
        left.keep(Path::new("/here"), NOW - 20);
        assert_eq!(places, left, "tidying kept the wrong places");
    }

    #[test]
    fn places_read_back_as_they_were_written() {
        let folder = std::env::temp_dir().join(format!("merge-net-places-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("places.json");
        assert_eq!(
            Places::read(&path),
            Places::default(),
            "no file read as places"
        );
        let mut places = Places::default();
        places.keep(Path::new("/a"), 7);
        places.write(&path).unwrap();
        let read = Places::read(&path);
        fs::write(&path, "not yet whole").unwrap();
        let unreadable = Places::read(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(read, places, "places didn't read back as written");
        assert_eq!(
            unreadable,
            Places::default(),
            "a file that doesn't read gave places"
        );
    }
}
