//! Scratch: directories whose entries are throwaway once nothing writes
//! to them, such as a quest workspace's `tmp/` and what lies beside it.

use crate::age::last_write;
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
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
pub const LIVE_BUDGET: usize = 100_000;

/// How many entries one survey reads of all live quests' `tmp/` entries,
/// so the many a long-lived session leaves cost a pass a few seconds at
/// most; those past it are kept for a later survey to read.
pub const LIVE_SURVEY_BUDGET: usize = 1_000_000;

/// What a whole walk of a live quest's `tmp/` entry found.
#[derive(Clone, Debug, PartialEq)]
pub enum Live {
    /// Its newest change anywhere in it, in wall seconds.
    Written(f64),
    /// Why it stays whatever its age.
    Holds(String),
    /// Nothing is there now: its session took it away itself.
    Gone,
}

/// Reads a live quest's `tmp/` entry whole: its newest change at any
/// depth, since a session may edit a file in place far below where the
/// shallow dating looks, unless it holds what may be found nowhere else.
/// That is a handover a leaving machine packed, a repository, whose
/// commits may be on no remote and which the guard can't ask git about,
/// or a link into the store, whose build is the session's to drop. What
/// can't be judged is kept: a folder or entry that can't be read, another
/// filesystem mounted within, which may hang or be anything, or more than
/// `budget` entries, which counts down as it reads. A link is dated by
/// itself and never followed, as removing it removes only it.
pub fn live_reading(path: &Path, budget: &mut usize) -> Live {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == ErrorKind::NotFound => return Live::Gone,
        Err(error) => return Live::Holds(format!("it can't be read: {error}")),
    };
    if let Some(why) = path.file_name().and_then(named_work) {
        return Live::Holds(why.to_owned());
    }
    if !meta.is_dir() {
        return Live::Written(changed(&meta));
    }
    let mounted = |meta: &fs::Metadata, beside: u64| meta.dev() != beside;
    if path
        .parent()
        .and_then(|parent| fs::metadata(parent).ok())
        .is_none_or(|up| mounted(&meta, up.dev()))
    {
        return Live::Holds("it is another filesystem".to_owned());
    }
    let mut newest = changed(&meta);
    let mut level = vec![path.to_owned()];
    while !level.is_empty() {
        let mut deeper = Vec::new();
        for dir in level {
            match within(&dir, meta.dev(), budget, &mut newest) {
                Ok(more) => deeper.extend(more),
                Err(why) => return Live::Holds(why),
            }
        }
        level = deeper;
    }
    Live::Written(newest)
}

/// Reads one folder of a whole walk: the folders in it to read next, after
/// raising `newest` to its newest change, or why the entry holds.
fn within(
    dir: &Path,
    device: u64,
    budget: &mut usize,
    newest: &mut f64,
) -> Result<Vec<PathBuf>, String> {
    let unread =
        |path: &Path, error: std::io::Error| format!("{} can't be read: {error}", path.display());
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        // Gone since it was listed: nothing in it to keep.
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(unread(dir, error)),
    };
    let (mut deeper, mut bare) = (Vec::new(), [false; 3]);
    for entry in listing {
        let entry = entry.map_err(|error| unread(dir, error))?;
        *budget = budget
            .checked_sub(1)
            .ok_or("it holds more than can be read")?;
        let name = entry.file_name();
        if let Some(why) = named_work(&name) {
            return Err(why.to_owned());
        }
        // Not followed: a link's own metadata. A folder listed but not
        // searchable fails here, and so holds rather than reading as empty.
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => return Err(unread(&entry.path(), error)),
        };
        *newest = newest.max(changed(&meta));
        if meta.file_type().is_symlink() {
            if fs::read_link(entry.path()).is_ok_and(|to| to.starts_with("/nix/store")) {
                return Err("it holds a link into the store".to_owned());
            }
            continue;
        }
        match name.to_str() {
            Some("HEAD") => bare[0] |= meta.is_file(),
            Some("objects") => bare[1] |= meta.is_dir(),
            Some("refs") => bare[2] |= meta.is_dir(),
            _ => {}
        }
        if meta.is_dir() && meta.dev() != device {
            return Err(format!(
                "another filesystem is mounted at {}",
                entry.path().display()
            ));
        }
        if meta.is_dir() {
            deeper.push(entry.path());
        }
    }
    if bare == [true; 3] {
        return Err("it holds a bare repository".to_owned());
    }
    Ok(deeper)
}

/// Dates a live quest's `tmp/` entry by [`live_reading`], as the shallow
/// date can miss an edit far down, reading no more than [`LIVE_BUDGET`]
/// of what's `left` of the survey's [`LIVE_SURVEY_BUDGET`]: false, leaving
/// it as it was, when it holds work and so stays whatever its age, or is
/// gone.
pub fn dated_whole(entry: &mut Entry, left: &mut usize) -> bool {
    let mut each = (*left).min(LIVE_BUDGET);
    let before = each;
    let read = live_reading(&entry.path, &mut each);
    *left -= before - each;
    match read {
        Live::Written(written) => {
            entry.written = written;
            true
        }
        Live::Holds(_) | Live::Gone => false,
    }
}

/// Why an entry by this name is work, when it is: a handover, or a
/// repository of any of the version control systems a session may use.
fn named_work(name: &std::ffi::OsStr) -> Option<&'static str> {
    match name.to_str()? {
        "handover" => Some("it holds a handover"),
        ".git" | ".jj" | ".hg" | ".sl" | ".pijul" | ".svn" | ".fslckout" | "_FOSSIL_" => {
            Some("it holds a repository")
        }
        _ => None,
    }
}

/// When an entry last changed, in wall seconds: the later of when it was
/// written and when its inode changed. An archive unpacked or a tree
/// copied keeping its times carries old modification times, but its
/// inodes are new, and nothing can set a change time back.
fn changed(meta: &fs::Metadata) -> f64 {
    crate::units::wall(meta.mtime(), meta.mtime_nsec())
        .max(crate::units::wall(meta.ctime(), meta.ctime_nsec()))
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
