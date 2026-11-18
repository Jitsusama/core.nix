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
/// `size` is asked only then, as sizing is a walk of all of it. A link
/// into the store is a root of a build, and goes by its own windows.
pub fn quest_candidate(
    entry: &Entry,
    size: impl FnOnce() -> u64,
    awake: &Awake,
    now: f64,
) -> Option<Candidate> {
    let idle = idle(entry, awake, now);
    (idle >= QUEST_UNWRITTEN).then(|| {
        let class = if std::fs::read_link(&entry.path)
            .is_ok_and(|target| target.starts_with("/nix/store"))
        {
            Class::QuestRoot
        } else {
            Class::QuestTemp
        };
        left(entry, class, size(), idle)
    })
}

/// How long an entry of a quest a live session works on must go unwritten
/// in its `tmp/`, in awake seconds, before it's offered: long enough that
/// a session pausing over a weekend's work, or a script it wrote that
/// evening, is never taken from under it.
pub const LIVE_QUEST_UNWRITTEN: f64 = 2.0 * 86_400.0;

/// How many entries a live quest's `tmp/` entry may hold before it's kept
/// as more than can be read: its walk goes as deep as the tree does, where
/// the shallow dating stops at `DEPTH`.
const LIVE_BUDGET: usize = 100_000;

/// What a whole walk of a live quest's `tmp/` entry found.
#[derive(Clone, Debug, PartialEq)]
pub enum Live {
    /// Its newest write anywhere in it, in wall seconds.
    Written(f64),
    /// Why it stays whatever its age.
    Holds(String),
}

/// Reads a live quest's `tmp/` entry whole: its newest write at any depth,
/// since a session may edit a file in place far below where the shallow
/// dating looks, unless it holds what may be found nowhere else. That is a
/// handover a leaving machine packed, a repository, whose commits may be on
/// no remote and which the guard can't ask git about, or a link into the
/// store, whose build is the session's to drop. A folder that can't be
/// listed, or more than `LIVE_BUDGET` entries, holds it too, so what can't
/// be judged is kept. A link is dated by itself and never followed, as
/// removing it removes only it.
pub fn live_reading(path: &Path) -> Live {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Live::Holds("it can't be read".to_owned());
    };
    let mut newest = wall(&meta);
    if let Some(why) = path.file_name().and_then(named_work) {
        return Live::Holds(why.to_owned());
    }
    if !meta.is_dir() {
        return Live::Written(newest);
    }
    let mut level = vec![path.to_owned()];
    let mut seen = 0;
    while !level.is_empty() {
        let mut deeper = Vec::new();
        for dir in level {
            let listing = match fs::read_dir(&dir) {
                Ok(listing) => listing,
                // Gone since it was listed: nothing in it to keep.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Live::Holds(format!("{} can't be read: {error}", dir.display()));
                }
            };
            let mut bare = [false; 3];
            for entry in listing.filter_map(Result::ok) {
                seen += 1;
                if seen > LIVE_BUDGET {
                    return Live::Holds("it holds more than can be read".to_owned());
                }
                let name = entry.file_name();
                if let Some(why) = named_work(&name) {
                    return Live::Holds(why.to_owned());
                }
                // Not followed: a link's own metadata.
                let Ok(meta) = entry.metadata() else { continue };
                newest = newest.max(wall(&meta));
                if meta.file_type().is_symlink() {
                    if fs::read_link(entry.path()).is_ok_and(|to| to.starts_with("/nix/store")) {
                        return Live::Holds("it holds a link into the store".to_owned());
                    }
                    continue;
                }
                match name.to_str() {
                    Some("HEAD") => bare[0] |= meta.is_file(),
                    Some("objects") => bare[1] |= meta.is_dir(),
                    Some("refs") => bare[2] |= meta.is_dir(),
                    _ => {}
                }
                if meta.is_dir() {
                    deeper.push(entry.path());
                }
            }
            if bare == [true; 3] {
                return Live::Holds("it holds a bare repository".to_owned());
            }
        }
        level = deeper;
    }
    Live::Written(newest)
}

/// Dates a live quest's `tmp/` entry by [`live_reading`], as the shallow
/// date can miss an edit far down: false, leaving it as it was, when it
/// holds work and so stays whatever its age.
pub fn dated_whole(entry: &mut Entry) -> bool {
    match live_reading(&entry.path) {
        Live::Written(written) => {
            entry.written = written;
            true
        }
        Live::Holds(_) => false,
    }
}

/// Why an entry by this name is work, when it is.
fn named_work(name: &std::ffi::OsStr) -> Option<&'static str> {
    match name.to_str()? {
        "handover" => Some("it holds a handover"),
        ".git" | ".jj" => Some("it holds a repository"),
        _ => None,
    }
}

fn wall(meta: &fs::Metadata) -> f64 {
    use std::os::unix::fs::MetadataExt;
    crate::units::wall(meta.mtime(), meta.mtime_nsec())
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
