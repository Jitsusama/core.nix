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
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::denoise::{Cleaner, FRAME};
use crate::room;
use crate::score::{self, Marks};
use crate::ways;

/// The frames heard before speech counts: a second, while the room's floor
/// is first found.
const WARM: u32 = 100;

/// How far over the room's floor a frame is before it's speech, in dB.
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
        self.heard = self.heard.saturating_add(1);
        if self.heard <= WARM || self.samples.len() >= MOST {
            return;
        }
        let (Some(floor), Some(level)) = (floor, room::level(frame)) else {
            return;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a floor is a small whole number of dB"
        )]
        let over = level - floor as f32;
        if over >= OVER {
            self.samples.extend_from_slice(frame);
        }
    }

    /// Whether there's enough to judge.
    pub fn enough(&self) -> bool {
        self.samples.len() >= LEAST
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
    std::thread::spawn(move || {
        let Some(line) = judged(&models, &call) else {
            return;
        };
        eprintln!(
            "noise-cancelled: {}'s call at {} would have cleaned with {}, not {}",
            call.node, call.spot, line["best"], call.running
        );
        if let Some(file) = file {
            kept(&file, &line);
        }
    });
}

/// `call`'s line, once its speech is cleaned every way and scored, or none
/// when it can't be.
fn judged(models: &Path, call: &Call) -> Option<Value> {
    let cleaned = match ways::every_way(models, &call.speech) {
        Ok(cleaned) => cleaned,
        Err(why) => {
            eprintln!("noise-cancelled: judging a call: {why}");
            return None;
        }
    };
    let heard: Vec<_> = cleaned
        .into_iter()
        .map(|(way, samples, _)| (way.name(), std::sync::Arc::new(samples)))
        .collect();
    let marks = score::every_way(models, &heard);
    if marks.is_empty() {
        return None;
    }
    Some(said(call, &marks, now()))
}

/// The way whose marks are best overall.
pub fn best(marks: &BTreeMap<&'static str, Marks>) -> Option<&'static str> {
    marks
        .iter()
        .max_by(|one, other| one.1.overall.total_cmp(&other.1.overall))
        .map(|(way, _)| *way)
}

/// `call`'s line, judged `at`, in milliseconds since the epoch: numbers and
/// names, never its sound.
pub fn said(call: &Call, marks: &BTreeMap<&'static str, Marks>, at: u64) -> Value {
    let each: serde_json::Map<String, Value> = marks
        .iter()
        .map(|(way, marks)| ((*way).to_owned(), marks.said()))
        .collect();
    json!({
        "at": at,
        "microphone": call.microphone,
        "node": call.node,
        "spot": call.spot,
        "running": call.running.to_string(),
        "best": best(marks),
        "marks": each,
    })
}

/// Adds `line` to `file`, making its folder.
pub fn kept(file: &Path, line: &Value) {
    let appended = file
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| OpenOptions::new().create(true).append(true).open(file))
        .and_then(|mut kept| writeln!(kept, "{line}"));
    if let Err(why) = appended {
        eprintln!(
            "noise-cancelled: keeping a call's marks in {}: {why}",
            file.display()
        );
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |at| u64::try_from(at.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[path = "shadow_tests.rs"]
mod tests;
