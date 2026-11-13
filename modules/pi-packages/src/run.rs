//! One update of every package, each under its own lock, swapping at
//! most once an hour, and never while a hold stands (hold.rs).

use std::fs::{self, File, TryLockError};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::config::{Config, Package};
use crate::hold::{self, Hold};
use crate::version::{self, Place};
use crate::{jiti, loaded, warm};

/// The least time between two swaps on this machine, across every package.
///
/// Each swap moves `current` under every running pi, and each pi follows
/// it at its next reload, which rewrites its whole prompt cache when the
/// new version declares a tool differently. On 8 November five installs
/// between 18:49 and 20:36 cost about $45 of rewritten cache in an hour,
/// across some 70 sessions, so what merges within the hour is swapped in
/// together.
pub const SPACING: Duration = Duration::from_hours(1);

/// Whether an update waits out [`SPACING`] before swapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// Swap only once an hour has passed since the last swap: the timer's
    /// way, and a hand run's unless it says otherwise.
    Spaced,
    /// Swap at once, for a fix that unblocks work or one somebody waits on.
    Now,
}

/// Why an update builds nothing for a package that has a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wait {
    /// The last swap was within [`SPACING`]; the next may come at this time.
    Spacing(SystemTime),
    /// Somebody holds swaps.
    Hold(Hold),
}

/// What an update did for one package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// `current` already pointed at the branch's head.
    Current { commit: String },
    /// `current` was moved, from the version it pointed at, if any.
    Moved { from: Option<String>, to: String },
    /// Another update holds this package's lock, so this one left it.
    Busy,
    /// The branch has moved, but the last swap was within [`SPACING`], so
    /// `current` stays and nothing is built until `until`.
    Held { commit: String, until: SystemTime },
    /// The branch has moved, but a hold stands, so `current` stays and
    /// nothing is built until it ends.
    OnHold { commit: String, hold: Hold },
    /// The new commit's build failed, so `current` was left where it was.
    Failed { commit: String, why: String },
}

/// What an update did for one package, with what it removed and what it
/// could not fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub done: Done,
    /// Old versions removed, nothing using them.
    pub removed: usize,
    /// Why the mirror could not be fetched, when it could not; the last
    /// commit it saw was used.
    pub unfetched: Option<String>,
    /// Why each pi whose cache could not be warmed before the swap
    /// wasn't (warm.rs); the swap went ahead.
    pub unwarmed: Vec<String>,
}

/// Update each package in turn; one failing never stops the next.
///
/// Whether the hour is up is decided once, before the first package, so
/// every package that moved swaps in the same run, and one swapping
/// doesn't hold the next. A hold stands whatever the pace, `--now`
/// included.
#[must_use]
pub fn update(
    config: &Config,
    now: SystemTime,
    pace: Pace,
) -> Vec<(Package, Result<Outcome, String>)> {
    let wait = match (hold::standing(config, now), pace) {
        (Some(hold), _) => Some(Wait::Hold(hold)),
        (None, Pace::Spaced) => held_until(config, now).map(Wait::Spacing),
        (None, Pace::Now) => None,
    };
    config
        .packages
        .iter()
        .map(|package| {
            let outcome = update_one(config, package, now, wait.as_ref());
            (package.clone(), outcome)
        })
        .collect()
}

/// Where the time of the last swap on this machine is kept, as a file's
/// modification time under the root every package lives in.
#[must_use]
pub fn last_swap(config: &Config) -> PathBuf {
    config.root.join(".last-swap")
}

/// When the next swap may happen, if that is still to come.
fn held_until(config: &Config, now: SystemTime) -> Option<SystemTime> {
    let last = fs::metadata(last_swap(config))
        .and_then(|marker| marker.modified())
        .ok()?;
    let until = last + SPACING;
    (until > now).then_some(until)
}

fn mark_swap(config: &Config, now: SystemTime) -> Result<(), String> {
    let marker = last_swap(config);
    fs::create_dir_all(&config.root)
        .map_err(|error| format!("{}: {error}", config.root.display()))?;
    let file = File::create(&marker).map_err(|error| format!("{}: {error}", marker.display()))?;
    file.set_modified(now)
        .map_err(|error| format!("{}: {error}", marker.display()))
}

/// Update one package: fetch, build the branch's head when `current` is
/// elsewhere, swap it in, and remove what nothing uses. Given a `wait`, a
/// package that already has a version keeps it and builds nothing; one
/// with none is installed anyway, since pi can't load it at all until it
/// is.
///
/// # Errors
///
/// Says why the package could not be updated at all, which leaves
/// `current` where it was.
pub fn update_one(
    config: &Config,
    package: &Package,
    now: SystemTime,
    wait: Option<&Wait>,
) -> Result<Outcome, String> {
    let place = Place::of(config, package);
    let Some(_held) = hold(&place)? else {
        return Ok(Outcome {
            done: Done::Busy,
            removed: 0,
            unfetched: None,
            unwarmed: Vec::new(),
        });
    };
    let env = version::build_env(config);
    let (commit, unfetched) = version::fetch(config, package, &place, &env)?;
    let from = place.current_commit();
    let mut unwarmed = Vec::new();
    let done = if from.as_deref() == Some(commit.as_str()) {
        Done::Current { commit }
    } else if let (Some(_), Some(wait)) = (&from, wait) {
        match wait {
            Wait::Spacing(until) => Done::Held {
                commit,
                until: *until,
            },
            Wait::Hold(hold) => Done::OnHold {
                commit,
                hold: hold.clone(),
            },
        }
    } else {
        match version::build(config, package, &place, &commit, &env) {
            Ok(built) => {
                unwarmed = warm::warm(config, package, &built, &warm::pis(config), &env);
                // A hold put while the warm ran stands too; a package with
                // no version is installed whatever holds, as before.
                if let Some(hold) = hold::standing(config, now).filter(|_| from.is_some()) {
                    Done::OnHold { commit, hold }
                } else {
                    version::swap(&place, &commit, now)?;
                    mark_swap(config, now)?;
                    Done::Moved { from, to: commit }
                }
            }
            Err(why) => Done::Failed { commit, why },
        }
    };
    let removed = version::sweep(&place, first_pi_started(config), &caches(config))?;
    Ok(Outcome {
        done,
        removed,
        unfetched,
        unwarmed,
    })
}

/// Remove what nothing uses of each package, with no fetch or build: run
/// when a pi starts or ends, so a version goes with the last pi running
/// it. A package an update holds is left to it, since its update removes
/// too: `None`.
#[must_use]
pub fn sweep(config: &Config) -> Vec<(Package, Result<Option<usize>, String>)> {
    let first_pi = first_pi_started(config);
    config
        .packages
        .iter()
        .map(|package| {
            let place = Place::of(config, package);
            let removed = hold(&place).and_then(|held| match held {
                Some(_held) => version::sweep(&place, first_pi, &caches(config)).map(Some),
                None => Ok(None),
            });
            (package.clone(), removed)
        })
        .collect()
}

/// When the longest-running pi started, of those that said what they
/// loaded of any package: a pi that loads one package resolves through
/// another's `current` too, though only some packages are said.
fn first_pi_started(config: &Config) -> Option<SystemTime> {
    config
        .packages
        .iter()
        .filter_map(|package| loaded::read(&Place::of(config, package)).earliest())
        .min()
}

/// How long jiti's entries that no kept version wrote are left: a week,
/// so a tree in use now and then keeps its entries warm.
pub const JITI_KEEPS: Duration = Duration::from_hours(24 * 7);

/// The least time between two age-outs. Entries age by the week, so a
/// day loses nothing, and each 15-minute run is spared listing every
/// kept version's files (191,099 on 12 November, 0.2 s).
pub const JITI_AGES_EVERY: Duration = Duration::from_hours(24);

/// Age out of every jiti cache the entries no version kept here wrote,
/// once they are older than [`JITI_KEEPS`] (jiti.rs), at most once each
/// [`JITI_AGES_EVERY`]: `None` when it isn't due.
#[must_use]
pub fn age_jiti(config: &Config, now: SystemTime) -> Option<jiti::Aged> {
    let marker = config.root.join(".last-jiti-age");
    let last = fs::metadata(&marker).and_then(|marker| marker.modified());
    if last.is_ok_and(|last| last + JITI_AGES_EVERY > now) {
        return None;
    }
    let kept: Vec<PathBuf> = config
        .packages
        .iter()
        .flat_map(|package| {
            let place = Place::of(config, package);
            [place.versions(), place.former_versions()]
        })
        .filter_map(|folder| fs::read_dir(folder).ok())
        .flat_map(|entries| entries.flatten().map(|entry| entry.path()))
        .collect();
    let aged = jiti::age_out(&kept, &caches(config), now - JITI_KEEPS);
    // Failing, the next run ages out again, which costs only the listing.
    let _ = File::create(&marker).and_then(|file| file.set_modified(now));
    Some(aged)
}

/// Each jiti cache a removed version's entries may be in: every live pi's,
/// and this process's own temporary folder's, the one a pi that set no
/// temporary folder used.
fn caches(config: &Config) -> Vec<PathBuf> {
    let mut caches: Vec<PathBuf> = warm::pis(config)
        .iter()
        .map(|pi| jiti::cache_of(&pi.tmp))
        .chain([jiti::cache_of(&std::env::temp_dir())])
        .collect();
    caches.sort();
    caches.dedup();
    caches
}

/// The package's lock, or nothing when another run holds it.
fn hold(place: &Place) -> Result<Option<Held>, String> {
    fs::create_dir_all(&place.dir).map_err(|error| format!("{}: {error}", place.dir.display()))?;
    let lock_path = place.dir.join(".lock");
    let lock =
        File::create(&lock_path).map_err(|error| format!("{}: {error}", lock_path.display()))?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(Held(lock))),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(format!("{}: {error}", lock_path.display())),
    }
}

/// A package's lock, given back the moment the update is done with it.
///
/// Closing the file isn't enough: a child another thread forked keeps a
/// copy of it until it runs its program, and the lock lasts while any copy
/// is open, so the next update would find it still held. Unlocking frees
/// it for every copy at once.
struct Held(File);

impl Drop for Held {
    fn drop(&mut self) {
        // Failing, the lock goes when the last copy closes, as it did before.
        let _ = self.0.unlock();
    }
}
