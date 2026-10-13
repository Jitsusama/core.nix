//! Scratch: directories whose entries are throwaway once nothing writes
//! to them, such as a quest workspace's `tmp/`.

use crate::age::last_write;
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use std::fs;
use std::path::{Path, PathBuf};

/// One entry of a scratch directory.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    /// The last write within reach, in wall seconds; infinite when there
    /// was too much to read.
    pub written: f64,
}

/// How far down a last write is looked for, and how many entries may be
/// read on the way before one counts as written now.
const DEPTH: usize = 4;
const BUDGET: usize = 50_000;

/// Every entry of a scratch directory with its last write. A symlink is
/// an entry of its own, dated by itself, and removing it removes only it.
pub fn entries(dir: &Path) -> Vec<Entry> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let paths = listing.filter_map(Result::ok).map(|entry| entry.path());
    paths
        .map(|path| Entry {
            written: last_write(&path, DEPTH, BUDGET),
            path,
        })
        .collect()
}

/// The candidate a scratch entry makes, idle for the awake time since its
/// last write. `healthy` is how long its source keeps it while the disk is
/// healthy, if it lets routine upkeep take it at all.
pub fn candidate(
    entry: &Entry,
    healthy: Option<f64>,
    size: u64,
    awake: &Awake,
    now: f64,
) -> Candidate {
    Candidate {
        path: entry.path.clone(),
        class: Class::Scratch { healthy },
        idle: awake.since(entry.written.min(now), now).max(0.0),
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
