//! What surveys sized, kept from one survey to the next, so a survey walks
//! only what changed since. On 27 November the account's guard read 223 GB
//! in three and a half hours, seven whole surveys walking every candidate
//! afresh for 18 to 20 GB each, though few had changed.
//!
//! A size only says what taking an entry would free: whether it may be
//! taken is dated afresh each survey, so a size kept from an earlier one
//! never makes anything look idle or unused. A change deeper than its
//! stamp reads leaves a size behind until a shallower one or until the
//! size is old enough to be walked again, and a size known this way adds
//! nothing to the survey's shared blocks, so a block two entries share
//! may count in both; either only misstates what a pass frees.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How many levels of folders below an entry its stamp reads.
const STAMP_DEPTH: usize = 2;

/// How long before it's read a change must be for its stamp to be kept,
/// in seconds. The kernel stamps a change with a clock that moves a tick
/// at a time, up to 10 ms on Linux, so a change in the tick a stamp was
/// read in can carry the very time the stamp took as newest, and nothing
/// would tell the entry was changed again.
pub const SETTLED: f64 = 0.1;

/// Where in a daemon's state the record is kept across restarts, so the
/// first survey after one walks only what changed while it was down.
pub const FILE: &str = "sizes.json";

/// The shape of the record's file, its own rather than the state's: a
/// file of any other is read as no record at all.
pub const VERSION: u32 = 1;

/// How old a size may grow, in wall seconds, before a survey walks its
/// entry again though its stamp holds, so a change deeper than the stamp
/// reads, as a new incremental session of a cargo target is, is counted
/// within hours rather than never.
pub const REMEASURE_AFTER: f64 = 6.0 * 3_600.0;

/// The share of the record, one in this many rounded up, a survey walks
/// again for age at most, so walking every old size at once never comes
/// back as the whole walk the record was kept to spare.
pub const REMEASURE_SHARE: usize = 10;

/// How long a survey may spend walking again for age, after which it
/// takes the rest of the old sizes as known, since a few huge entries
/// could take as long as walking everything.
pub const REMEASURE_FOR: Duration = Duration::from_secs(60);

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
    /// When the survey under way began, in wall seconds, which each size
    /// it walks is kept as walked at.
    now: f64,
    /// The entries the survey under way walks again for the age of their
    /// sizes, while it has time left for it.
    due: BTreeSet<PathBuf>,
    /// How long the survey under way may still spend walking again.
    left: Duration,
}

/// An entry's size, the stamp it was sized at, and when it was walked,
/// in wall seconds.
#[derive(Clone, Copy, Debug)]
struct Known {
    stamp: f64,
    size: u64,
    walked: f64,
}

/// The record as its file holds it.
#[derive(Serialize, Deserialize)]
struct Record {
    version: u32,
    sizes: Vec<Kept>,
}

/// One entry's size as the file holds it.
#[derive(Serialize, Deserialize)]
struct Kept {
    path: String,
    stamp: f64,
    size: u64,
    walked: f64,
}

impl Sizes {
    /// The record kept in the state folder `state`, or none when there is
    /// none, it can't be read, or its shape is another's. A failed read is
    /// taken as nothing rather than refusing to start, since a size the
    /// record lacks is only walked again, as every survey did before one
    /// was kept, and a guard that won't start guards no disk.
    pub fn load(state: &Path) -> Sizes {
        let record: Option<Record> = crate::state::load(&state.join(FILE)).ok().flatten();
        let known = record
            .filter(|record| record.version == VERSION)
            .map(|record| record.sizes)
            .unwrap_or_default()
            .into_iter()
            .filter(|kept| kept.stamp.is_finite() && kept.walked.is_finite())
            .map(|kept| {
                let known = Known {
                    stamp: kept.stamp,
                    size: kept.size,
                    walked: kept.walked,
                };
                (PathBuf::from(kept.path), known)
            })
            .collect();
        Sizes {
            known,
            ..Sizes::default()
        }
    }

    /// Keeps the record in the state folder `state`, whole, for the guard
    /// after a restart. A path that isn't UTF-8 has no place in the file,
    /// so its entry is only walked again then.
    pub fn save(&self, state: &Path) -> io::Result<()> {
        let sizes = self
            .known
            .iter()
            .filter_map(|(path, known)| {
                Some(Kept {
                    path: path.to_str()?.to_owned(),
                    stamp: known.stamp,
                    size: known.size,
                    walked: known.walked,
                })
            })
            .collect();
        let record = Record {
            version: VERSION,
            sizes,
        };
        crate::state::save(&state.join(FILE), &record)
    }

    /// Starts a survey at `now`, in wall seconds, choosing which sizes it
    /// walks again for their age: those walked more than six hours ago,
    /// oldest first, a tenth of the record at most, with a minute to do it.
    pub fn began(&mut self, now: f64) {
        self.now = now;
        self.left = REMEASURE_FOR;
        let mut old: Vec<_> = self
            .known
            .iter()
            .filter(|(_, known)| now - known.walked > REMEASURE_AFTER)
            .collect();
        old.sort_by(|(_, one), (_, other)| one.walked.total_cmp(&other.walked));
        let most = self.known.len().div_ceil(REMEASURE_SHARE);
        self.due = old
            .into_iter()
            .take(most)
            .map(|(path, _)| path.clone())
            .collect();
    }

    /// What `path`, stamped `stamp`, holds: known when an earlier survey
    /// sized it at that stamp, `measure`d otherwise, as is one whose size
    /// is due again for its age while the survey has time for it. A stamp
    /// that isn't finite is never known, as the entry may be changing.
    /// What a walk a stop cut short gave, which `measure` answers as none,
    /// counted only part of the entry, so it's kept nowhere and counts as
    /// nothing.
    pub(crate) fn of(
        &mut self,
        path: &Path,
        stamp: f64,
        measure: impl FnOnce() -> Option<u64>,
    ) -> u64 {
        self.asked.insert(path.to_owned());
        let again = self.due.remove(path) && !self.left.is_zero();
        let known = self
            .known
            .get(path)
            .filter(|known| known.stamp.to_bits() == stamp.to_bits())
            .copied();
        if let Some(known) = known
            && !again
        {
            return known.size;
        }
        self.walked += 1;
        let began = Instant::now();
        let measured = measure();
        // Only a walk again for age spends the time for it: a changed
        // entry is walked however long it takes, as before any was known.
        if known.is_some() {
            self.left = self.left.saturating_sub(began.elapsed());
        }
        match measured {
            Some(size) if stamp.is_finite() => {
                let walked = self.now;
                self.known.insert(
                    path.to_owned(),
                    Known {
                        stamp,
                        size,
                        walked,
                    },
                );
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
