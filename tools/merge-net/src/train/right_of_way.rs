//! The train's right of way over changes that would merge alone. On 16
//! November two riders ran five times from 20:27, 22 to 30 minutes each,
//! and every time main moved beside them before the run ended, as changes
//! merged alone about every ten minutes: nothing landed off the train for
//! two hours. Once main moves beside a candidate that passed, the train
//! holds the right of way, `train/right-of-way` saying since when, and a
//! change `ready` would merge alone boards it instead, so main stays where
//! the train's next run is cut. It ends when the train lands, goes red,
//! can't reach main or has no one left, and after `RIGHT_OF_WAY` whatever
//! happens, so a train that can't land never holds main back for long.

use std::fs;
use std::path::{Path, PathBuf};

/// How long the train holds the right of way it was given: longer than
/// the 22 to 38 minutes its runs took on 16 November, short enough that
/// one that never lands keeps changes from merging alone for less than
/// an hour.
pub const RIGHT_OF_WAY: u64 = 45 * 60;

fn file(root: &Path) -> PathBuf {
    super::folder(root).join("right-of-way")
}

/// Give the train under `root` the right of way at `now`, keeping when it
/// began while it still holds one.
pub fn give(root: &Path, now: u64) {
    if holds(root, now).is_none() {
        let _ = fs::write(file(root), format!("{now}\n"));
    }
}

/// Take the right of way back from the train under `root`.
pub fn take_back(root: &Path) {
    let _ = fs::remove_file(file(root));
}

/// Since when the train under `root` has held the right of way, at `now`,
/// if it still does.
#[must_use]
pub fn holds(root: &Path, now: u64) -> Option<u64> {
    let since: u64 = fs::read_to_string(file(root)).ok()?.trim().parse().ok()?;
    (now.saturating_sub(since) < RIGHT_OF_WAY).then_some(since)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("merge-net-way-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(super::super::folder(&root)).unwrap();
        root
    }

    #[test]
    fn a_train_given_the_right_of_way_holds_it_from_then() {
        let root = root("given");
        assert_eq!(
            holds(&root, 100),
            None,
            "a train given nothing held the right of way"
        );
        give(&root, 100);
        assert_eq!(
            holds(&root, 160),
            Some(100),
            "the train given the right of way didn't hold it"
        );
        give(&root, 200);
        assert_eq!(
            holds(&root, 260),
            Some(100),
            "giving it again moved when the right of way began"
        );
    }

    #[test]
    fn the_right_of_way_ends_when_taken_back_or_when_its_time_is_up() {
        let root = root("ends");
        give(&root, 100);
        take_back(&root);
        assert_eq!(
            holds(&root, 160),
            None,
            "a right of way taken back still held"
        );
        give(&root, 100);
        assert_eq!(
            holds(&root, 100 + RIGHT_OF_WAY - 1),
            Some(100),
            "the right of way ended before its time"
        );
        assert_eq!(
            holds(&root, 100 + RIGHT_OF_WAY),
            None,
            "the right of way outlasted its time"
        );
        give(&root, 100 + RIGHT_OF_WAY);
        assert_eq!(
            holds(&root, 100 + RIGHT_OF_WAY),
            Some(100 + RIGHT_OF_WAY),
            "a train given it again after its time didn't hold a fresh one"
        );
    }
}
