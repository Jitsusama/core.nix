//! What Joel keeps: folders the account's guard never offers to clear,
//! each with his reason. A holder the guard would clear can still be
//! something he has a use for that no process shows, and a reason of his
//! own is one the guard can't argue with. The file sits in the guard's
//! state, written by `disk-guard keep` and `unkeep`, and the daemon reads
//! it afresh each time it surveys and each time it writes its status, so
//! a keep holds from the next step without a restart.
//!
//! A machine's configuration can declare keeps too, for what is plainly
//! worth its room there, such as a restore image or a swap file sized for
//! hibernation. The daemon writes those into the same file as it starts,
//! marked declared, and takes back the declared ones the configuration no
//! longer names, so every reader sees one list and the machine's own
//! keeps stay in its configuration.

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
    /// Whether the machine's configuration declares it, rather than
    /// `disk-guard keep`: it goes when the configuration stops naming it,
    /// and `unkeep` leaves it to the configuration.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub declared: bool,
}

/// Every keep, by the folder kept.
pub type Keeps = BTreeMap<PathBuf, Keep>;

/// The keeps in the state folder `state`: none when there's no file.
pub fn load(state: &Path) -> io::Result<Keeps> {
    Ok(state::load(&state.join(FILE))?.unwrap_or_default())
}

/// Writes the configuration's keeps, `declared`, into the file in `state`,
/// as `with_declared` gives them. Writes only when something changed.
pub fn declare(state: &Path, declared: &BTreeMap<PathBuf, String>, now: f64) -> io::Result<()> {
    let before = load(state)?;
    let keeps = with_declared(before.clone(), declared, now);
    if keeps == before {
        return Ok(());
    }
    std::fs::create_dir_all(state)?;
    state::save(&state.join(FILE), &keeps)
}

/// `keeps` with the configuration's keeps, `declared`, among them, and
/// any declared keep it no longer names taken back. A keep made by hand
/// on the same folder stands as it is, since its reason is Joel's own. A
/// folder is named with its links resolved when it's there, as `keep`
/// names one.
pub fn with_declared(mut keeps: Keeps, declared: &BTreeMap<PathBuf, String>, now: f64) -> Keeps {
    let named: BTreeMap<PathBuf, &String> = declared
        .iter()
        .map(|(path, reason)| {
            (
                std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()),
                reason,
            )
        })
        .collect();
    keeps.retain(|path, keep| !keep.declared || named.contains_key(path));
    for (path, reason) in named {
        match keeps.get(&path) {
            Some(keep) if !keep.declared => {}
            Some(keep) if keep.reason == *reason => {}
            _ => {
                keeps.insert(
                    path,
                    Keep {
                        reason: reason.clone(),
                        at: now,
                        declared: true,
                    },
                );
            }
        }
    }
    keeps
}

/// Keeps `path` for `reason`, replacing any reason it had.
pub fn keep(state: &Path, path: &Path, reason: &str, now: f64) -> io::Result<()> {
    let mut keeps = load(state)?;
    keeps.insert(
        path.to_owned(),
        Keep {
            reason: reason.to_owned(),
            at: now,
            declared: false,
        },
    );
    state::save(&state.join(FILE), &keeps)
}

/// Stops keeping `path`, saying whether it was kept. A keep the
/// configuration declares is refused, since the daemon would write it
/// back at its next start: it goes by changing the configuration.
pub fn unkeep(state: &Path, path: &Path) -> io::Result<bool> {
    let mut keeps = load(state)?;
    if keeps.get(path).is_some_and(|keep| keep.declared) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the machine's configuration keeps it, so it goes by changing that",
        ));
    }
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
