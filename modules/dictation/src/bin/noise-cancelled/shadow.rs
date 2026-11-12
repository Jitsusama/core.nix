//! Each call judged in the shadow: what the scorer would have chosen for
//! it, kept beside what Joel chose, so it can be checked against his ear
//! before it ever chooses for him (plan items 16 and 17). Nothing changes
//! what a call hears.
//!
//! While a microphone is recorded from, the first ten seconds of its
//! speech are held in memory: frames 15 dB or more over its room's floor
//! (room.rs), after a second's warm-up. Once a call of at least a minute
//! ends with three seconds or more of it, a thread of its own cleans that
//! speech every way and scores each (score.rs), then drops the sound. A
//! line of numbers and names alone goes in `judged`, beside the picks:
//! when, which microphone, where, what it ran, which way scored best, and
//! every way's marks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::denoise::{Cleaner, FRAME};
use crate::score::Marks;

/// The frames heard before speech counts: a second, while the room's floor
/// is first found.
const WARM: u32 = 100;

/// How far over the room's floor a frame is before it's speech, in dB.
#[expect(dead_code, reason = "a stub")]
const OVER: f32 = 15.0;

/// The most speech held: ten seconds, a little over what the scorer hears.
const MOST: usize = 1_000 * FRAME;

/// The least judged: three seconds.
const LEAST: usize = 300 * FRAME;

/// The speech a call has held so far.
#[derive(Debug, Default)]
pub struct Speech {
    samples: Vec<f32>,
    heard: u32,
}

impl Speech {
    /// Holds `frame` if it's speech over a room whose floor is `floor`,
    /// once warmed up and until ten seconds are held.
    pub fn heard(&mut self, frame: &[f32], floor: Option<i32>) {
        let _ = (frame, floor, &self.heard);
    }

    /// Whether there's enough to judge.
    pub fn enough(&self) -> bool {
        false
    }

    /// The speech held, leaving none.
    pub fn taken(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }
}

/// A call that ended with speech enough to judge.
#[derive(Debug)]
pub struct Call {
    /// The microphone's node, and its name as Sound gives it.
    pub node: String,
    pub microphone: String,
    /// Where it was, by Joel's name for the spot.
    pub spot: String,
    /// What it was cleaned with.
    pub running: Cleaner,
    pub speech: Vec<f32>,
}

/// Judges `call` in a thread of its own, keeping its line in `file`.
pub fn judge(models: PathBuf, file: Option<PathBuf>, call: Call) {
    let _ = (models, file, call);
}

/// The way whose marks are best overall.
pub fn best(marks: &BTreeMap<&'static str, Marks>) -> Option<&'static str> {
    let _ = marks;
    None
}

/// `call`'s line, judged `at`, in milliseconds since the epoch: numbers and
/// names, never its sound.
pub fn said(call: &Call, marks: &BTreeMap<&'static str, Marks>, at: u64) -> Value {
    let _ = (call, marks, at);
    Value::Null
}

/// Adds `line` to `file`, making its folder.
pub fn kept(file: &Path, line: &Value) {
    let _ = (file, line);
}

#[cfg(test)]
#[path = "shadow_tests.rs"]
mod tests;
