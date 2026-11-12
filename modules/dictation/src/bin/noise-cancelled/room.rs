//! How loud the room around a microphone is, as the floor its sound sits
//! on between words: the level the quietest tenth of its 10 ms frames
//! reach, so a voice over it, however long it talks, leaves it where it
//! is. Frames under [`SILENT`] aren't a room but a microphone gone quiet,
//! as a muted one does, and don't count.
//!
//! A pick by ear keeps the floor of the recording it was made from, in
//! `rooms` beside the picks, a line a microphone and spot: its node name,
//! the spot's key, the floor in whole dB, and `nudged` once Choose by Ear
//! has been offered again for a room far from it. Each call measures its
//! own floor as it's cleaned (worker.rs), and one [`FAR`] from its pick's
//! offers Choose by Ear again (offer.rs), once a pick.

use crate::chosen::Chosen;
use crate::denoise::FRAME;

/// The quietest level a frame counts at, in dB below full scale.
const SILENT: f32 = -90.0;

/// The size of each step of level frames are counted in, in dB.
const STEP: f32 = 0.5;

/// How many steps there are, from [`SILENT`] to full scale.
const STEPS: usize = 180;

/// How many frames a floor needs: a second of sound.
const ENOUGH: u32 = 100;

/// How far a call's floor is from its pick's before Choose by Ear is
/// offered again, in dB: about twice or half as loud to the ear.
pub const FAR: i32 = 10;

/// A room being heard, a frame at a time: how many frames reached each
/// step of level, so its floor is found without keeping any sound.
#[derive(Clone, Debug)]
pub struct Room {
    counts: [u32; STEPS],
    heard: u32,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            counts: [0; STEPS],
            heard: 0,
        }
    }
}

impl Room {
    /// Counts one frame's level.
    pub fn heard(&mut self, frame: &[f32]) {
        let _ = frame;
    }

    /// The room's floor, in whole dB below full scale, once a second of it
    /// has been heard.
    pub fn floor(&self) -> Option<i32> {
        None
    }
}

/// The floor of a whole recording, as the trial hears its own.
pub fn of(samples: &[f32]) -> Option<i32> {
    let _ = samples;
    None
}

/// Keeps `floor` as the room `node`'s pick at the spot `key` was made in,
/// in place of any before, not yet nudged.
pub fn kept(chosen: &Chosen, node: &str, key: &str, floor: i32) -> Result<(), String> {
    let _ = (chosen, node, key, floor);
    Ok(())
}

/// The floor `node`'s pick at the spot `key` was made in, and whether
/// Choose by Ear has been offered again since for a room far from it.
pub fn picked_in(chosen: &Chosen, node: &str, key: &str) -> Option<(i32, bool)> {
    let _ = (chosen, node, key);
    None
}

/// Keeps that Choose by Ear has been offered again for `node` at the spot
/// `key`, so it isn't again until Joel picks again.
pub fn nudged(chosen: &Chosen, node: &str, key: &str) -> Result<(), String> {
    let _ = (chosen, node, key);
    Ok(())
}

#[cfg(test)]
#[path = "room_tests.rs"]
mod tests;
