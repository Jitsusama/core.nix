//! How loud the far end's side of a call is, 50 ms at a time, so a word
//! of Joel's far under their sound just before it can be cut as their
//! echo (echo.rs), whatever words, if any, Parakeet heard in that sound.
//!
//! It's filled from their takes, as the daemon is given them: what no
//! take covers is silence, as their side is between takes. Only each
//! step's energy is kept, never the sound, about 300 KB an hour of call.

use crate::mel::RATE;

/// Their side's sound as how much energy each 50 ms step of it holds,
/// from the call's start.
#[derive(Clone, Debug, Default)]
pub struct Track {
    energy: Vec<f32>,
}

impl Track {
    /// Adds a take's `samples`, starting `start` samples into the call.
    pub fn hear(&mut self, start: usize, samples: &[f32]) {
        // Red: nothing is kept yet.
        let _ = (self, start, samples);
    }

    /// The loudest quarter second of their sound around `at`, in dB of
    /// full scale.
    #[must_use]
    pub fn loudest_before(&self, at: usize) -> f32 {
        let _ = (self, at);
        f32::NAN
    }
}

#[cfg(test)]
#[path = "loudness_tests.rs"]
mod tests;
