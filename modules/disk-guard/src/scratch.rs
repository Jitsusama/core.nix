//! Scratch: directories whose entries are throwaway once nothing writes
//! to them, such as a quest workspace's `tmp/` and what lies beside it.

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
    dated(dir, |_| true)
}

/// The entries a quest's workspace holds beside its `tmp/`, with their last
/// writes, leaving out those it keeps whatever their age before any is
/// dated: a working quest's `tmp/` alone can hold tens of thousands of
/// entries, read to the budget on every pass for nothing.
pub fn beside_tmp(workspace: &Path) -> Vec<Entry> {
    dated(workspace, |path| !kept_beside_tmp(path))
}

fn dated(dir: &Path, wanted: impl Fn(&Path) -> bool) -> Vec<Entry> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let paths = listing.filter_map(Result::ok).map(|entry| entry.path());
    paths
        .filter(|path| wanted(path))
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
    left(
        entry,
        Class::Scratch { healthy },
        size,
        awake.since(entry.written.min(now), now),
    )
}

/// How long an entry has gone unwritten, in awake seconds.
pub fn idle(entry: &Entry, awake: &Awake, now: f64) -> f64 {
    awake.since(entry.written.min(now), now)
}

/// How long a quest at work's `tmp/` entry must go unwritten, in awake
/// seconds, before it's offered: the plan's shortest window for one, so
/// a younger one, which no band would take, is never sized.
pub const QUEST_UNWRITTEN: f64 = 6.0 * 3_600.0;

/// The candidate an entry of a quest at work's `tmp/` makes, once it's
/// gone `QUEST_UNWRITTEN` awake with nothing written; none before.
/// `size` is asked only then, as sizing is a walk of all of it.
pub fn quest_candidate(
    entry: &Entry,
    size: impl FnOnce() -> u64,
    awake: &Awake,
    now: f64,
) -> Option<Candidate> {
    let idle = idle(entry, awake, now);
    (idle >= QUEST_UNWRITTEN).then(|| left(entry, Class::QuestTemp, size(), idle))
}

/// How long what a quest at work left beside its `tmp/` must go unwritten,
/// in awake seconds, before it's offered: the plan's shortest window for
/// one.
pub const QUEST_SCRATCH_UNWRITTEN: f64 = 12.0 * 3_600.0;

/// Whether an entry of a quest's workspace stays whatever its age: `tmp/`,
/// whose entries go by their own rule; the handover a leaving machine
/// packs for the share to take home; and a folder of GC roots, whose links
/// keep what running programs read.
fn kept_beside_tmp(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| name == "tmp" || name == "handover" || name.starts_with("gcroots"))
}

/// The candidate what a quest at work left beside its `tmp/` makes, once
/// it's gone `QUEST_SCRATCH_UNWRITTEN` awake with nothing written; none
/// before, so `size`, a walk of all of it, is asked only then.
pub fn quest_scratch_candidate(
    entry: &Entry,
    size: impl FnOnce() -> u64,
    awake: &Awake,
    now: f64,
) -> Option<Candidate> {
    let idle = idle(entry, awake, now);
    (idle >= QUEST_SCRATCH_UNWRITTEN).then(|| left(entry, Class::QuestScratch, size(), idle))
}

fn left(entry: &Entry, class: Class, size: u64, idle: f64) -> Candidate {
    Candidate {
        path: entry.path.clone(),
        class,
        idle: idle.max(0.0),
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
