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
    fs::create_dir_all(&place.dir).map_err(|error| format!("{}: {error}", place.dir.display()))?;
    let lock_path = place.dir.join(".lock");
    let lock =
        File::create(&lock_path).map_err(|error| format!("{}: {error}", lock_path.display()))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Ok(Outcome {
                done: Done::Busy,
                removed: 0,
                unfetched: None,
            });
        }
        Err(TryLockError::Error(error)) => return Err(format!("{}: {error}", lock_path.display())),
    }
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
    let removed = version::prune(config, &place, now)?;
    Ok(Outcome {
        done,
        removed,
        unfetched,
    })
}
