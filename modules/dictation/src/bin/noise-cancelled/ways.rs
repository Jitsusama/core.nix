//! The ways Choose by Ear offers a recording: as heard, or cleaned by each
//! cleaner, each with what it cost to clean and every one played back
//! equally loud (loudness.rs), so a pick is never only the loudest.

use std::path::Path;

use crate::denoise::{Cleaner, FRAME, Model, RATE};
use crate::loudness;
use crate::score::spent;

/// What a clip was cleaned by: one of the cleaners, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Raw,
    Cleaned(Cleaner),
}

impl Way {
    /// Every way, in the order they're cleaned, for the tests to offer.
    #[cfg(test)]
    pub const ALL: [Self; 4] = [
        Self::Raw,
        Self::Cleaned(Cleaner::DeepFilterNet),
        Self::Cleaned(Cleaner::DeepFilterNetGentle),
        Self::Cleaned(Cleaner::RNNoise),
    ];

    /// Its name, as the record says it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Cleaned(cleaner) => cleaner.name(),
        }
    }
}

/// The recording, every way, with each way's share of one core while it
/// cleaned: the processor time this thread spent, over the clip's length.
pub fn every_way(models: &Path, samples: &[f32]) -> Result<Vec<(Way, Vec<f32>, f64)>, String> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a clip is far under 2^52 samples"
    )]
    let length = samples.len() as f64 / f64::from(RATE);
    let mut clips = vec![(Way::Raw, samples.to_vec(), 0.0)];
    for cleaner in Cleaner::ALL {
        // Loaded outside the time it's charged, since the microphones load
        // it once and keep it while a call goes on, and kept while its
        // denoiser cleans, whose requests run on what it compiled.
        let model = Model::load(models, cleaner)?;
        let mut denoiser = model.denoiser()?;
        let started = spent();
        let mut cleaned = Vec::with_capacity(samples.len());
        for chunk in samples.chunks(FRAME) {
            let mut frame = [0.0; FRAME];
            frame[..chunk.len()].copy_from_slice(chunk);
            cleaned.extend(denoiser.clean(&frame)?);
        }
        cleaned.truncate(samples.len());
        let cost = spent().saturating_sub(started).as_secs_f64() / length;
        clips.push((Way::Cleaned(cleaner), cleaned, cost));
    }
    // Matched after the costs are taken, since calls never pay for it.
    let mut heard: Vec<&mut Vec<f32>> = clips.iter_mut().map(|(_, samples, _)| samples).collect();
    loudness::matched(&mut heard);
    Ok(clips)
}
