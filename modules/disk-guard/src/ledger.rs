//! The disk guard's ledger: each measurement of what holds the disks, kept
//! for two days, so a holder can say how much it grew in a day. A leak
//! nobody configured then shows by name before it's big, rather than as a
//! disk filling for no reason anyone can give.

use crate::holders::Place;
use crate::state;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// The file, in a guard's state folder.
pub const FILE: &str = "ledger.json";

/// How long a measurement is kept, in seconds.
const KEPT: f64 = 2.0 * DAY;

/// How far back growth is read from, in seconds.
const DAY: f64 = 24.0 * 3600.0;

/// How old a measurement must be before growth is read from it, in
/// seconds, so the walk's own noise between two measurements close
/// together isn't said as growth.
const HOUR: f64 = 3600.0;

/// One measurement: when, in wall seconds, and each place's bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub at: f64,
    pub sizes: BTreeMap<PathBuf, u64>,
}

/// How much a place grew, and over how long, in seconds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Growth {
    pub bytes: u64,
    pub over: f64,
}

/// Each place that grew, by its path.
pub type Grown = BTreeMap<PathBuf, Growth>;

/// The measurements in the state folder `state`, oldest first: none when
/// there's no file, or one that won't read, since the ledger is a signal
/// and starting it afresh loses only a day of it.
pub fn load(state: &Path) -> Vec<Entry> {
    state::load(&state.join(FILE))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Keeps the measurement `measured`, taken at `at`, letting go of those
/// past two days, and says how much each place grew.
pub fn record(state: &Path, at: f64, measured: &[(Place, u64)]) -> io::Result<Grown> {
    let mut entries = load(state);
    let grew = grown(&entries, at, measured);
    entries.retain(|entry| at - entry.at <= KEPT);
    entries.push(Entry {
        at,
        sizes: measured
            .iter()
            .map(|(place, bytes)| (place.path.clone(), *bytes))
            .collect(),
    });
    state::save(&state.join(FILE), &entries)?;
    Ok(grew)
}

/// How much each place in `measured` grew since the earliest measurement
/// within the last day that's at least an hour old. A place that
/// measurement didn't size has nothing to grow from, so a place newly
/// named isn't said to have grown from nothing.
pub fn grown(entries: &[Entry], at: f64, measured: &[(Place, u64)]) -> Grown {
    let Some(then) = entries
        .iter()
        .filter(|entry| at - entry.at <= DAY && at - entry.at >= HOUR)
        .min_by(|one, other| one.at.total_cmp(&other.at))
    else {
        return Grown::new();
    };
    measured
        .iter()
        .filter_map(|(place, bytes)| {
            let before = then.sizes.get(&place.path)?;
            let bytes = bytes.checked_sub(*before).filter(|grew| *grew > 0)?;
            let over = at - then.at;
            Some((place.path.clone(), Growth { bytes, over }))
        })
        .collect()
}
