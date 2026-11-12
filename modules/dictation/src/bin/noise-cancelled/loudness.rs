//! How loud a clip sounds, so Choose by Ear plays every clip as loud as
//! the others and loud enough to hear. A microphone records far quieter
//! than calls play it back, since call programs raise its level as they
//! go, and the cleaners take some of each clip's loudness away with the
//! noise; Joel heard tonight's clips as too quiet (2026-11-11 23:48).

/// The loudness every clip is played at, in LUFS: EBU R128's level for
/// speech heard on a laptop or headset.
pub const TARGET: f64 = -16.0;

/// The highest a sample may reach once raised, 1 dB under full scale.
pub const PEAK: f32 = 0.891;

/// A clip's integrated loudness in LUFS, or none for one too short or
/// too quiet to measure.
pub const fn integrated(_samples: &[f32]) -> Option<f64> {
    None
}

/// Raises or lowers every clip to the one loudness, `TARGET` where each
/// has the headroom for it, else as loud as the one with the least.
pub const fn matched(_clips: &mut [&mut Vec<f32>]) {}

#[cfg(test)]
#[path = "loudness_tests.rs"]
mod tests;
