//! How loud a clip sounds, so Choose by Ear plays every clip as loud as
//! the others and loud enough to hear. A microphone records far quieter
//! than calls play it back, since call programs raise its level as they
//! go, and the cleaners take some of each clip's loudness away with the
//! noise; Joel heard tonight's clips as too quiet (2026-11-11 23:48).
//!
//! Loudness is measured as EBU R128 does, by ITU-R BS.1770-4: the clip
//! weighted as an ear hears it, in blocks of 400 ms a new one every
//! 100 ms, with silent blocks and those far quieter than the rest left
//! out, so a pause between words doesn't count as a quiet voice.

use crate::denoise::RATE;

/// The loudness every clip is played at, in LUFS: EBU R128's level for
/// speech heard on a laptop or headset.
pub const TARGET: f64 = -16.0;

/// The highest a sample may reach once raised, 1 dB under full scale.
pub const PEAK: f32 = 0.891;

const _: () = assert!(RATE == 48_000, "the weighting's coefficients are 48 kHz's");

/// A block's length and the step between blocks, at 48 kHz.
const BLOCK: usize = 19_200;
const STEP: usize = 4_800;

/// Blocks quieter than this are silence, in LUFS.
const SILENT: f64 = -70.0;

/// Blocks this far under the rest are pauses, in LU.
const PAUSES: f64 = 10.0;

/// One stage of the ear's weighting: a filter's feed-forward and feedback
/// coefficients, the feedback's leading one left out.
struct Stage {
    forward: [f64; 3],
    back: [f64; 2],
}

/// The weighting's two stages at 48 kHz, as BS.1770-4 gives them: a shelf
/// raising what's above about 1.5 kHz, as the head does, then a high-pass
/// leaving out the rumble an ear barely hears.
const SHELF: Stage = Stage {
    forward: [
        1.535_124_859_586_97,
        -2.691_696_189_406_38,
        1.198_392_810_852_85,
    ],
    back: [-1.690_659_293_182_41, 0.732_480_774_215_85],
};
const RUMBLE: Stage = Stage {
    forward: [1.0, -2.0, 1.0],
    back: [-1.990_047_454_833_98, 0.990_072_250_366_21],
};

impl Stage {
    fn filtered(&self, samples: &[f64]) -> Vec<f64> {
        let mut heard = [0.0; 2];
        let mut made = [0.0; 2];
        samples
            .iter()
            .map(|&sample| {
                let out = self.forward[0] * sample
                    + self.forward[1] * heard[0]
                    + self.forward[2] * heard[1]
                    - self.back[0] * made[0]
                    - self.back[1] * made[1];
                heard = [sample, heard[0]];
                made = [out, made[0]];
                out
            })
            .collect()
    }
}

/// How many there are, as a float to divide by.
#[expect(
    clippy::cast_precision_loss,
    reason = "a clip's blocks and samples are far under 2^52"
)]
const fn counted(count: usize) -> f64 {
    count as f64
}

/// A mean square's loudness, in LUFS.
fn loudness_of(power: f64) -> f64 {
    -0.691 + 10.0 * power.log10()
}

fn mean(powers: &[f64]) -> f64 {
    powers.iter().sum::<f64>() / counted(powers.len())
}

/// A clip's integrated loudness in LUFS, or none for one too short or
/// too quiet to measure.
pub fn integrated(samples: &[f32]) -> Option<f64> {
    let widened: Vec<f64> = samples.iter().copied().map(f64::from).collect();
    let weighted = RUMBLE.filtered(&SHELF.filtered(&widened));
    let blocks = weighted.len().checked_sub(BLOCK)? / STEP + 1;
    let powers: Vec<f64> = (0..blocks)
        .map(|block| &weighted[block * STEP..block * STEP + BLOCK])
        .map(|block| block.iter().map(|sample| sample * sample).sum::<f64>() / counted(BLOCK))
        .filter(|power| *power > 0.0 && loudness_of(*power) > SILENT)
        .collect();
    if powers.is_empty() {
        return None;
    }
    let pauses = loudness_of(mean(&powers)) - PAUSES;
    let spoken: Vec<f64> = powers
        .into_iter()
        .filter(|power| loudness_of(*power) > pauses)
        .collect();
    (!spoken.is_empty()).then(|| loudness_of(mean(&spoken)))
}

/// Raises or lowers every clip to the one loudness, `TARGET` where each
/// has the headroom for it, else as loud as the one with the least, so
/// none sounds better for being louder. A clip too quiet to measure is
/// left as it is.
pub fn matched(clips: &mut [&mut Vec<f32>]) {
    let measured: Vec<Option<f64>> = clips.iter().map(|clip| integrated(clip)).collect();
    let level = clips
        .iter()
        .zip(&measured)
        .filter_map(|(clip, loudness)| {
            let loudest = clip
                .iter()
                .fold(0.0_f32, |most, sample| most.max(sample.abs()));
            Some(loudness.as_ref()? + 20.0 * (f64::from(PEAK) / f64::from(loudest)).log10())
        })
        .fold(TARGET, f64::min);
    for (clip, loudness) in clips.iter_mut().zip(measured) {
        let Some(loudness) = loudness else { continue };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a gain between clips is well within an f32"
        )]
        let gain = 10.0_f64.powf((level - loudness) / 20.0) as f32;
        for sample in clip.iter_mut() {
            *sample *= gain;
        }
    }
}

#[cfg(test)]
#[path = "loudness_tests.rs"]
mod tests;
