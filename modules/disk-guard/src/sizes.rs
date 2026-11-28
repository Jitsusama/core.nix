//! What surveys sized, kept from one survey to the next, so a survey walks
//! only what changed since. On 27 November the account's guard read 223 GB
//! in three and a half hours, seven whole surveys walking every candidate
//! afresh for 18 to 20 GB each, though few had changed.
//!
//! A size only says what taking an entry would free: whether it may be
//! taken is dated afresh each survey, so a size kept from an earlier one
//! never makes anything look idle or unused. A change deeper than its
//! stamp reads leaves a size behind until a shallower one, and a size
//! known this way adds nothing to the survey's shared blocks, so a block
//! two entries share may count in both; either only misstates what a pass
//! frees.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// How many levels of folders below an entry its stamp reads.
const STAMP_DEPTH: usize = 2;

/// How long before it's read a change must be for its stamp to be kept,
/// in seconds. The kernel stamps a change with a clock that moves a tick
/// at a time, up to 10 ms on Linux, so a change in the tick a stamp was
/// read in can carry the very time the stamp took as newest, and nothing
/// would tell the entry was changed again.
pub const SETTLED: f64 = 0.1;

/// The sizes of the entries surveys sized, each by the stamp it was sized
/// at, kept by a daemon from one survey to the next.
#[derive(Debug, Default)]
pub struct Sizes {
    /// Each entry's size, by the stamp it was sized at.
    known: BTreeMap<PathBuf, Known>,
    /// The entries asked for since the last survey began.
    asked: BTreeSet<PathBuf>,
    /// How many were walked to be sized, rather than known.
    pub walked: usize,
}

/// An entry's size, and the stamp it was sized at.
#[derive(Clone, Copy, Debug)]
struct Known {
    stamp: f64,
    size: u64,
}

impl Sizes {
    /// What `path`, stamped `stamp`, holds: known when an earlier survey
    /// sized it at that stamp, `measure`d otherwise. A stamp that isn't
    /// finite is never known, as the entry may be changing. What a walk a
    /// stop cut short gave, which `measure` answers as none, counted only
    /// part of the entry, so it's kept nowhere and counts as nothing.
    pub(crate) fn of(
        &mut self,
        path: &Path,
        stamp: f64,
        measure: impl FnOnce() -> Option<u64>,
    ) -> u64 {
        self.asked.insert(path.to_owned());
        let known = self
            .known
            .get(path)
            .filter(|known| known.stamp.to_bits() == stamp.to_bits())
            .copied();
        if let Some(known) = known {
            return known.size;
        }
        self.walked += 1;
        match measure() {
            Some(size) if stamp.is_finite() => {
                self.known.insert(path.to_owned(), Known { stamp, size });
                size
            }
            Some(size) => {
                self.known.remove(path);
                size
            }
            None => {
                self.known.remove(path);
                0
            }
        }
    }

    /// Forgets every entry the survey just ended never asked for, gone or
    /// no longer offered, so what's kept is only what the next may ask.
    pub fn ended(&mut self) {
        let asked = std::mem::take(&mut self.asked);
        self.known.retain(|path, _| asked.contains(path));
    }
}

/// When anything near the top of `path` last changed: the newest change
/// time of the entry and of the folders one and two levels below it on
/// its filesystem, or a plain file's own. A file made, renamed or deleted
/// in any of those folders moves it, as does a write to a plain file; a
/// write to a file already in a folder doesn't, nor a change three levels
/// down or deeper. Cargo writes each new file of a target in a profile's
/// `deps`, `build`, `incremental` or `.fingerprint`, two levels down, so a
/// target is stamped in a few dozen reads rather than a walk of 100,000
/// files. Not a number when the entry or a folder in reach can't be read,
/// or when the newest change is too recent to tell from one to come.
pub fn stamp(path: &Path) -> f64 {
    let read = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(f64::NAN, |since| since.as_secs_f64());
    let Ok(meta) = fs::symlink_metadata(path) else {
        return f64::NAN;
    };
    let mut newest = changed(&meta);
    if meta.is_dir() {
        let device = meta.dev();
        let mut level = vec![path.to_owned()];
        for _ in 0..STAMP_DEPTH {
            let mut deeper = Vec::new();
            for dir in level {
                let Ok(entries) = fs::read_dir(&dir) else {
                    return f64::NAN;
                };
                for entry in entries {
                    let Ok(entry) = entry else {
                        return f64::NAN;
                    };
                    if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                        continue;
                    }
                    // A folder gone since it was listed changed the one
                    // listing it, whose own time says so.
                    let Ok(meta) = entry.metadata() else { continue };
                    // A walk never leaves the entry's filesystem, so its
                    // size holds nothing another one changed.
                    if meta.dev() != device {
                        continue;
                    }
                    newest = newest.max(changed(&meta));
                    deeper.push(entry.path());
                }
            }
            level = deeper;
        }
    }
    if newest <= read - SETTLED {
        newest
    } else {
        f64::NAN
    }
}

/// When an entry last changed, in wall seconds.
fn changed(meta: &fs::Metadata) -> f64 {
    crate::units::wall(meta.ctime(), meta.ctime_nsec())
}
