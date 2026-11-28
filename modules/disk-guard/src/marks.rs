//! The targets each walk of a root found, kept between steps, so the
//! hourly marking of builds for fanotify walks a root again only when it
//! changed. On 27 November the account's guard read 223 GB in three hours
//! and 35 minutes, and three of its hourly marking walks read about
//! 12.7 GB each, walking the same roots at the same depths for the same
//! targets as the survey before them.
//!
//! A root is known by its stamp, as `crate::sizes` stamps an entry: the
//! newest change time of the root and of its folders one and two levels
//! down. A checkout made in a root, or a target made at a checkout's top,
//! moves it. A target made deeper than the stamp reads, as a crate's own
//! target three levels below a work tree is, is missed until the next
//! survey walks the root, or until the walk kept is six hours old. That
//! fails safe: a build whose folders aren't marked keeps the hour, as
//! one in a folder that couldn't be marked does.
//!
//! What's kept lives only in memory, so a guard started again walks each
//! root once, as before anything was kept.

use crate::discover::{Found, Kind};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How old a walk may grow, in wall seconds, before the marking walks its
/// root again though its stamp holds, so a target made deeper than the
/// stamp reads is marked within hours however long no survey comes.
pub const WALK_AGAIN_AFTER: f64 = 6.0 * 3_600.0;

/// What a daemon's marks go on from: when they last came, in wall
/// seconds, and the targets each root's last walk found, by a survey or
/// by the marks.
#[derive(Debug, Default)]
pub struct Marks {
    /// When the builds were last marked, or none before the first.
    pub at: Option<f64>,
    /// The targets kept, so the marks walk a root again only once it
    /// changed.
    pub targets: Targets,
}

/// The targets found under each root, by the root as walked, every link
/// resolved, and the depth it was walked to, since a shallower walk finds
/// fewer.
#[derive(Debug, Default)]
pub struct Targets(BTreeMap<(PathBuf, Option<u32>), Walked>);

/// What one walk of a root found, the root's stamp read just before it
/// began, and when it was walked, in wall seconds.
#[derive(Debug)]
struct Walked {
    stamp: f64,
    at: f64,
    targets: Vec<PathBuf>,
}

impl Targets {
    /// Keeps the targets a walk of `root` to `depth` `found`, which began
    /// at `at` with the root stamped `stamp`. Only a walk that finished
    /// is kept, since one a stop cut short found only part of them.
    pub fn keep(
        &mut self,
        (root, depth): (&Path, Option<u32>),
        (stamp, at): (f64, f64),
        found: &[Found],
    ) {
        let targets = found
            .iter()
            .filter(|tree| tree.kind == Kind::Target)
            .map(|tree| tree.path.clone())
            .collect();
        let walked = Walked { stamp, at, targets };
        self.0.insert((root.to_owned(), depth), walked);
    }

    /// The targets the last walk of `root` to `depth` found, while the
    /// root is still stamped `stamp` as it was before that walk, and the
    /// walk is under [`WALK_AGAIN_AFTER`] old at `now`. A stamp that isn't
    /// a number never matches, as the root may be changing, and a walk
    /// dated after `now` is walked again, as the clock that dated it can't
    /// be trusted.
    #[must_use]
    pub fn fresh(
        &self,
        (root, depth): (&Path, Option<u32>),
        stamp: f64,
        now: f64,
    ) -> Option<&[PathBuf]> {
        let walked = self.0.get(&(root.to_owned(), depth))?;
        let age = now - walked.at;
        let same = walked.stamp.to_bits() == stamp.to_bits() && stamp.is_finite();
        (same && (0.0..WALK_AGAIN_AFTER).contains(&age)).then_some(walked.targets.as_slice())
    }
}
