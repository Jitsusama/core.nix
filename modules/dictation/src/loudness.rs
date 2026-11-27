//! How loud the far end's side of a call is, 50 ms at a time, so a word
//! of Joel's far under their sound just before it can be cut as their
//! echo (echo.rs), whatever words, if any, Parakeet heard in that sound.
//!
//! It's filled from their takes, as the daemon is given them: what no
//! take covers is silence, as their side is between takes. Only each
//! step's energy is kept, never the sound, about 300 KB an hour of call.

use crate::mel::RATE;

/// How much sound each step of the track holds: 50 ms, the step between
/// the quarter seconds the lab measured their sound by.
const STEP: usize = RATE / 20;

/// How many steps their sound is measured over at once: a quarter
/// second, as a word's level is (parakeet.rs), so the two compare.
const QUARTER: usize = 5;

const _: () = assert!(
    STEP * QUARTER == RATE / 4,
    "their sound must be measured over a quarter second, as a word's level is"
);

/// How long before a word the quarter seconds of their sound it may be
/// the echo of start from, in samples: a second, as their echo reaches
/// Joel's microphone within one of their word (`LAG` in echo.rs).
const BEFORE: usize = RATE;

/// How long after a word they may start, in samples: three tenths of a
/// second, as each side's words are timed by its own frames, so an echo
/// can be timed before what it echoes (`LEAD` in echo.rs).
const AFTER: usize = RATE * 3 / 10;

/// Their side's sound as how much energy each `STEP` of it holds, from
/// the call's start.
#[derive(Clone, Debug, Default)]
pub struct Track {
    energy: Vec<f32>,
}

impl Track {
    /// Adds a take's `samples`, starting `start` samples into the call.
    /// Each take is given once and takes never overlap, so a step's
    /// energy is its take's; were two to overlap, theirs would add.
    pub fn hear(&mut self, start: usize, samples: &[f32]) {
        let steps = (start + samples.len()).div_ceil(STEP);
        if self.energy.len() < steps {
            self.energy.resize(steps, 0.0);
        }
        for (at, sample) in (start..).zip(samples) {
            self.energy[at / STEP] += sample * sample;
        }
    }

    /// The loudest quarter second of their sound starting from `BEFORE`
    /// `at` to `AFTER` it, a `STEP` apart, as the mean power's dB of full
    /// scale, which is how a word's level is measured (`parakeet::Word`):
    /// silence, or past the last take, is -120 dB. It reads what the lab
    /// read from the sound itself, except that each quarter second starts
    /// on a step: past the call's first second, where `at` falls inside a
    /// step, each starts as much earlier than the lab's as `at` is into it.
    #[must_use]
    pub fn loudest_before(&self, at: usize) -> f32 {
        let first = at.saturating_sub(BEFORE) / STEP;
        let last = (at + AFTER) / STEP;
        let loudest = (first..=last)
            .map(|from| self.energy.iter().skip(from).take(QUARTER).sum::<f32>())
            .fold(0.0, f32::max);
        #[expect(clippy::cast_precision_loss, reason = "a quarter second's samples")]
        let power = loudest / (STEP * QUARTER) as f32;
        10.0 * (power + 1e-12).log10()
    }
}

#[cfg(test)]
#[path = "loudness_tests.rs"]
mod tests;
