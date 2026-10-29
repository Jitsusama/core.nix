//! What Joel keeps: folders the account's guard never offers to clear,
//! each with his reason. A holder the guard would clear can still be
//! something he has a use for that no process shows, and a reason of his
//! own is one the guard can't argue with. The file sits in the guard's
//! state, written by `disk-guard keep` and `unkeep`, and the daemon reads
//! it afresh each time it surveys and each time it writes its status, so
//! a keep holds from the next step without a restart.

use crate::state;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// The file, in a guard's state folder.
pub const FILE: &str = "keeps.json";

/// One keep: why, and since when, in wall seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keep {
    pub reason: String,
    pub at: f64,
}

/// Every keep, by the folder kept.
pub type Keeps = BTreeMap<PathBuf, Keep>;

/// The keeps in the state folder `state`: none when there's no file.
pub fn load(state: &Path) -> io::Result<Keeps> {
    Ok(state::load(&state.join(FILE))?.unwrap_or_default())
}

/// Keeps `path` for `reason`, replacing any reason it had.
pub fn keep(state: &Path, path: &Path, reason: &str, now: f64) -> io::Result<()> {
    let mut keeps = load(state)?;
    keeps.insert(
        path.to_owned(),
        Keep {
            reason: reason.to_owned(),
            at: now,
        },
    );
    state::save(&state.join(FILE), &keeps)
}

/// Stops keeping `path`, saying whether it was kept.
pub fn unkeep(state: &Path, path: &Path) -> io::Result<bool> {
    let mut keeps = load(state)?;
    let was = keeps.remove(path).is_some();
    if was {
        state::save(&state.join(FILE), &keeps)?;
    }
    Ok(was)
}

/// The keep that covers `path`: one on it or on a folder holding it.
pub fn covering<'a>(keeps: &'a Keeps, path: &Path) -> Option<&'a Keep> {
    keeps
        .iter()
        .filter(|(kept, _)| path.starts_with(kept))
        .max_by_key(|(kept, _)| kept.components().count())
        .map(|(_, keep)| keep)
}

/// Whether clearing `path` would take anything kept: it lies in a kept
/// folder, or holds one.
pub fn touches(keeps: &Keeps, path: &Path) -> bool {
    keeps
        .keys()
        .any(|kept| path.starts_with(kept) || kept.starts_with(path))
}
