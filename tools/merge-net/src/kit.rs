//! The queue for changes to the terminal kit, which most of our commands
//! read. A change to the kit runs every reader's checks, and each reader
//! merged during its run leaves its pass stale, as one to the kit leaves
//! every reader's. On 15 November #4796 ran again three times between
//! 11:40 and 12:30 as readers merged under it, #4749 among them while a
//! hold on the mesh asked readers to wait, since a hold there works only
//! for a session that reads it in time.
//!
//! So `ready` keeps the order itself (`turn.rs`). While a change to the
//! kit gates, runs again or keeps its turn after a pass, a reader waits
//! for it whenever its own gate started, and is told which change it waits
//! for; changes to the kit go first come first served among themselves,
//! as any changes sharing files do. A reader waits for at most one change
//! to the kit: one held behind a change to the kit that has since merged
//! is owed its turn, so no change to the kit holds it again, and the next
//! change to the kit waits for it instead, while its gate goes or its pass
//! keeps its turn. Otherwise a queue of six changes to the kit, back to
//! back, would hold a reader for hours. Either waits at most
//! [`crate::turn::PATIENCE`] from its first hold, then goes first: at 00:30
//! on 16 November a reader that had passed was held about 70 minutes behind
//! a change to the kit still gating.
//!
//! Which reader waited behind which change to the kit is kept in
//! `kit-owed.json` in the net's state, by the reader's patch id, so a
//! rebase keeps it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::turn::Gate;
use crate_reach::Crates;

/// Where the kit's files are, from the top of the repository.
pub const KIT: &str = "modules/terminal-kit/";

/// How long a reader's wait behind a change to the kit is kept: as long as
/// a gate is waited on, past which nothing it waited for still goes.
pub const KEPT: u64 = crate::turn::LONGEST;

/// What a change is to the kit's queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// It changes the kit.
    Kit,
    /// It changes a crate that reads the kit; `owed` once it waited behind
    /// a change to the kit that has since merged.
    Reader { owed: bool },
    /// It neither changes the kit nor reads it.
    Apart,
}

/// What a change changing `files` is to the kit's queue, `crates` saying
/// which read which, and `owed` whether it waited for a change to the kit
/// that has since merged.
#[must_use]
pub fn role(files: &[String], crates: &Crates, owed: bool) -> Role {
    if files.iter().any(|file| file.starts_with(KIT)) {
        return Role::Kit;
    }
    let kit = [format!("{KIT}Cargo.toml")];
    if crates.beside(&kit, files).is_empty() {
        Role::Apart
    } else {
        Role::Reader { owed }
    }
}

/// The gate among `going`, each going now or keeping its turn after a
/// pass, that a change of role `asking` waits for in the kit's queue,
/// `role_of` saying each gate's role: the earliest change to the kit, for a
/// reader not yet owed; the earliest owed reader, for a change to the kit.
/// One that has `waited` [`crate::turn::PATIENCE`] waits for none, as
/// behind any gate sharing its files: a hold lasts 20 minutes at most.
#[must_use]
pub fn behind<'a>(
    asking: Role,
    going: &'a [Gate],
    role_of: &dyn Fn(&Gate) -> Role,
    waited: u64,
) -> Option<&'a Gate> {
    if waited >= crate::turn::PATIENCE {
        return None;
    }
    let waits_for = match asking {
        Role::Reader { owed: false } => Role::Kit,
        Role::Kit => Role::Reader { owed: true },
        Role::Reader { owed: true } | Role::Apart => return None,
    };
    going
        .iter()
        .filter(|gate| role_of(gate) == waits_for)
        .min_by_key(|gate| gate.started)
}

/// Whether a gate of role `theirs` holds back a change of role `ours` by
/// the order gates started in (`turn.rs`): never between the kit and a
/// reader, which the queue orders instead.
#[must_use]
pub fn by_start(ours: Role, theirs: Role) -> bool {
    !matches!(
        (ours, theirs),
        (Role::Kit, Role::Reader { .. }) | (Role::Reader { .. }, Role::Kit)
    )
}

/// Which reader waited for which change to the kit, and when, by the
/// reader's patch id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Owed(BTreeMap<String, (String, u64)>);

impl Owed {
    /// The waits kept at `path`, or none where there is no file or it
    /// doesn't read.
    pub fn read(path: &Path) -> Self {
        let said: BTreeMap<String, (String, u64)> = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self(said)
    }

    /// Keep that the reader `patch` waits for the change to the kit at
    /// `kit` by `now`, unless a change it waited for before has merged, as
    /// `merged` says, since it's owed its turn then.
    pub fn waits(&mut self, patch: &str, kit: &str, now: u64, merged: &dyn Fn(&str) -> bool) {
        if self.owes(patch, now, merged) {
            return;
        }
        self.0.insert(patch.to_owned(), (kit.to_owned(), now));
    }

    /// Whether the reader `patch` is owed its turn by `now`: it waited for
    /// a change to the kit, within [`KEPT`], that `merged` says has merged.
    #[must_use]
    pub fn owes(&self, patch: &str, now: u64, merged: &dyn Fn(&str) -> bool) -> bool {
        self.0
            .get(patch)
            .is_some_and(|(kit, at)| now.saturating_sub(*at) < KEPT && merged(kit))
    }

    /// Give up the waits kept past [`KEPT`] by `now`.
    pub fn tidy(&mut self, now: u64) {
        self.0.retain(|_, (_, at)| now.saturating_sub(*at) < KEPT);
    }

    /// Write them whole to `path`, so a reader never sees half.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.0).map_err(|error| error.to_string())?;
        crate::whole::write(path, &(text + "\n"))
    }
}

/// Where the waits are kept: beside the rest of the net's state.
pub fn file() -> PathBuf {
    crate::state::State::from_env().root.join("kit-owed.json")
}

#[cfg(test)]
mod tests;
