//! Choose by Ear offered once for a microphone at a spot Joel has never
//! picked for it at, after a call there, as a row in Now he can take or
//! put off. A new microphone, or one at a new spot, cleans as the best
//! guess says (chosen.rs); the offer is how his ear gets a say.
//!
//! It's made only once a call has used the microphone there, a recording
//! of at least [`CALL`], since the shell's level meter and a program
//! testing a device record for a moment. It stands in `offer`, beside the
//! picks, only while nothing records from a microphone and no refusal
//! holds, such as a meeting being near, so it never asks during a call or
//! before one; it's withdrawn while one does and stands again once none
//! do. Taken or put off, its microphone and spot are kept in `offered`
//! and never offered again, and a pick by ear there withdraws it too.
//!
//! `offer` is one JSON object: the microphone's name and node, the spot's
//! name and key, and whether the microphone or the spot is the new one.

use std::path::PathBuf;
use std::time::Duration;

use crate::chosen::Chosen;
use crate::refusals::Refusal;
use crate::spot::Spot;

/// How long a microphone is recorded from before it counts as a call.
pub const CALL: Duration = Duration::from_mins(1);

/// Choose by Ear, offered for one microphone at one spot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The microphone's node name, which its picks are kept under.
    pub node: String,
    /// The microphone's name, as Sound says it.
    pub microphone: String,
    pub spot: Spot,
    /// Whether the microphone has never been picked for anywhere, rather
    /// than only here.
    pub new_microphone: bool,
}

/// The offer waiting to stand, and what keeps it from standing.
pub struct Offers {
    refusals: Vec<Refusal>,
    waiting: Option<Offer>,
    standing: bool,
}

impl Offers {
    pub fn new(_chosen: &Chosen, refusals: Vec<Refusal>) -> Self {
        Self {
            refusals,
            waiting: None,
            standing: false,
        }
    }

    pub fn heard(
        &mut self,
        _chosen: &Chosen,
        _node: &str,
        _microphone: &str,
        _spot: &Spot,
        _lasted: Duration,
    ) {
    }

    pub fn withheld(&mut self, _chosen: &Chosen) {}

    pub fn stand(&mut self, _chosen: &Chosen) {}
}

pub fn taken(_chosen: &Chosen) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "offer_tests.rs"]
mod tests;
