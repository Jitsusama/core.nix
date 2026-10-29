//! One update of every package, each under its own lock.

use std::fs::{self, File, TryLockError};
use std::time::SystemTime;

use crate::config::{Config, Package};
use crate::version::{self, Place};

/// What an update did for one package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// `current` already pointed at the branch's head.
    Current { commit: String },
    /// `current` was moved, from the version it pointed at, if any.
    Moved { from: Option<String>, to: String },
    /// Another update holds this package's lock, so this one left it.
    Busy,
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
}

/// Update each package in turn; one failing never stops the next.
#[must_use]
pub fn update(config: &Config, now: SystemTime) -> Vec<(Package, Result<Outcome, String>)> {
    config
        .packages
        .iter()
        .map(|package| (package.clone(), update_one(config, package, now)))
        .collect()
}

/// Update one package: fetch, build the branch's head when `current` is
/// elsewhere, swap it in, and remove what nothing uses.
///
/// # Errors
///
/// Says why the package could not be updated at all, which leaves
/// `current` where it was.
pub fn update_one(config: &Config, package: &Package, now: SystemTime) -> Result<Outcome, String> {
    let place = Place::of(config, package);
    let Some(_held) = hold(&place)? else {
        return Ok(Outcome {
            done: Done::Busy,
            removed: 0,
            unfetched: None,
        });
    };
    let env = version::build_env(config);
    let (commit, unfetched) = version::fetch(config, package, &place, &env)?;
    let from = place.current_commit();
    let done = if from.as_deref() == Some(commit.as_str()) {
        Done::Current { commit }
    } else {
        match version::build(config, package, &place, &commit, &env) {
            Ok(_) => {
                version::swap(&place, &commit, now)?;
                Done::Moved { from, to: commit }
            }
            Err(why) => Done::Failed { commit, why },
        }
    };
    let removed = version::prune(&place)?;
    Ok(Outcome {
        done,
        removed,
        unfetched,
    })
}

/// Remove what nothing uses of each package, with no fetch or build: run
/// when a pi starts or ends, so a version goes with the last pi running
/// it. A package an update holds is left to it, since its update removes
/// too: `None`.
#[must_use]
pub fn prune(config: &Config) -> Vec<(Package, Result<Option<usize>, String>)> {
    config
        .packages
        .iter()
        .map(|package| {
            let place = Place::of(config, package);
            let removed = hold(&place).and_then(|held| match held {
                Some(_held) => version::prune(&place).map(Some),
                None => Ok(None),
            });
            (package.clone(), removed)
        })
        .collect()
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
