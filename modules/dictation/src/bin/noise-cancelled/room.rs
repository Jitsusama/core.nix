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

use std::fmt::Write as _;

use crate::chosen::{Chosen, written};
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
        if frame.is_empty() {
            return;
        }
        let energy: f32 = frame.iter().map(|sample| sample * sample).sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a frame is far under 2^23 samples"
        )]
        let level = 10.0 * (energy / frame.len() as f32).log10();
        // Not a number, or under the silent one, as nothing at all gives.
        if level.is_nan() || level < SILENT {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the level is at or over SILENT, so the step is a small whole number"
        )]
        let step = (((level - SILENT) / STEP) as usize).min(STEPS - 1);
        self.counts[step] += 1;
        self.heard += 1;
    }

    /// The room's floor, in whole dB below full scale, once a second of it
    /// has been heard.
    pub fn floor(&self) -> Option<i32> {
        if self.heard < ENOUGH {
            return None;
        }
        let wanted = self.heard.div_ceil(10);
        let mut seen = 0;
        let step = self.counts.iter().position(|count| {
            seen += count;
            seen >= wanted
        })?;
        #[expect(clippy::cast_precision_loss, reason = "there are 180 steps")]
        let floor = SILENT + (step as f32 + 0.5) * STEP;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the floor is between SILENT and full scale"
        )]
        Some(floor.round() as i32)
    }
}

/// The floor of a whole recording, as the trial hears its own.
pub fn of(samples: &[f32]) -> Option<i32> {
    let mut room = Room::default();
    for frame in samples.chunks_exact(FRAME) {
        room.heard(frame);
    }
    room.floor()
}

/// One line of `rooms`.
struct Kept {
    node: String,
    key: String,
    floor: i32,
    nudged: bool,
}

/// Keeps `floor` as the room `node`'s pick at the spot `key` was made in,
/// in place of any before, not yet nudged.
pub fn kept(chosen: &Chosen, node: &str, key: &str, floor: i32) -> Result<(), String> {
    let mut rooms = read(chosen);
    rooms.retain(|room| room.node != node || room.key != key);
    rooms.push(Kept {
        node: node.to_owned(),
        key: key.to_owned(),
        floor,
        nudged: false,
    });
    write(chosen, &rooms)
}

/// The floor `node`'s pick at the spot `key` was made in, and whether
/// Choose by Ear has been offered again since for a room far from it.
pub fn picked_in(chosen: &Chosen, node: &str, key: &str) -> Option<(i32, bool)> {
    read(chosen)
        .into_iter()
        .find(|room| room.node == node && room.key == key)
        .map(|room| (room.floor, room.nudged))
}

/// Keeps that Choose by Ear has been offered again for `node` at the spot
/// `key`, so it isn't again until Joel picks again.
pub fn nudged(chosen: &Chosen, node: &str, key: &str) -> Result<(), String> {
    let mut rooms = read(chosen);
    for room in &mut rooms {
        if room.node == node && room.key == key {
            room.nudged = true;
        }
    }
    write(chosen, &rooms)
}

/// Every room `rooms` keeps, past any line that doesn't read.
fn read(chosen: &Chosen) -> Vec<Kept> {
    let said = chosen
        .beside("rooms")
        .and_then(|file| std::fs::read_to_string(file).ok())
        .unwrap_or_default();
    said.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some(Kept {
                node: fields.next()?.to_owned(),
                key: fields.next()?.to_owned(),
                floor: fields.next()?.parse().ok()?,
                nudged: fields.next() == Some("nudged"),
            })
        })
        .collect()
}

/// Writes `rooms` whole, or nothing where no pick is kept.
fn write(chosen: &Chosen, rooms: &[Kept]) -> Result<(), String> {
    let Some(file) = chosen.beside("rooms") else {
        return Ok(());
    };
    let mut said = String::new();
    for room in rooms {
        let nudged = if room.nudged { " nudged" } else { "" };
        let _ = writeln!(said, "{} {} {}{nudged}", room.node, room.key, room.floor);
    }
    written(&file, &said)
}

#[cfg(test)]
#[path = "room_tests.rs"]
mod tests;
