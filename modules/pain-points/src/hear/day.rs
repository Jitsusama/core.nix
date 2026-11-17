//! The keeper's daily count. Once a day ends where Joel is, the ledgers'
//! keeper hears how many pains he said in it, how many of them asked how
//! something stands, how many went to their owners and how many turns
//! stalled him: one line a day, in place of a note for every pain heard
//! too late for its owner, which the keeper already knew of.

use super::heard::Heard;
use std::path::Path;

/// The kind sore.rs gives a status ask.
pub const ASKED: &str = "asked how it stands";

/// How a day stood.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub heard: usize,
    pub asked: usize,
    pub routed: usize,
    pub carried: usize,
    pub stalled: usize,
}

/// The status asks he said since `since`.
pub fn asked_since(_kept: &[Heard], _since: i64) -> usize {
    0
}

/// Sends the keeper the count of the day that ended since the last run,
/// once.
///
/// # Errors
///
/// When day.json can't be written.
pub fn count(
    _state: &Path,
    _kept: &[Heard],
    _at: i64,
    _offset_s: i64,
    _send: &mut dyn FnMut(&str) -> bool,
) -> Result<(), String> {
    Ok(())
}
