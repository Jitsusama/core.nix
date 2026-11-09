//! The last whole survey a daemon's pass made, kept beside its memory so
//! `disk-guard plan` shows what a pass would take without walking every
//! source again: on 9 November a plan's own walk ran past 200 seconds on
//! a full machine and printed nothing.

use crate::plan::Candidate;
use crate::survey::Offer;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// Where in a daemon's state it's kept.
pub const FILE: &str = "survey.json";

/// What a survey offered, and when, in wall seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surveyed {
    #[serde(default = "crate::state::version")]
    pub version: u32,
    pub at: f64,
    pub offered: Vec<Offered>,
}

/// One candidate, and the filesystem it lies on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Offered {
    pub device: u64,
    pub candidate: Candidate,
}

impl Surveyed {
    pub fn new(at: f64, offers: &[Offer]) -> Self {
        Self {
            version: crate::state::VERSION,
            at,
            offered: offers
                .iter()
                .map(|offer| Offered {
                    device: offer.device,
                    candidate: offer.candidate.clone(),
                })
                .collect(),
        }
    }
}

pub fn path(state: &Path) -> PathBuf {
    state.join(FILE)
}

pub fn save(state: &Path, surveyed: &Surveyed) -> io::Result<()> {
    crate::state::save(&path(state), surveyed)
}

/// The survey kept in `state`, or None when none is, or it can't be read,
/// since a plan then only surveys for itself.
pub fn load(state: &Path) -> Option<Surveyed> {
    crate::state::load(&path(state)).ok().flatten()
}
