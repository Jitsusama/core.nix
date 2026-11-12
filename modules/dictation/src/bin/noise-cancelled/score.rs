//! How good a clip of a voice sounds, with nothing to compare it to:
//! Microsoft's DNSMOS P.835, a small model trained on many thousands of
//! people's ratings of cleaned calls, gives three marks out of 5. They are
//! how natural the voice sounds, how quiet the background is, and the two
//! together, as ITU-T P.835 asks listeners to rate them. Choose by Ear
//! scores each clip with it beside Joel's ear, so the marks can be checked
//! against what he picks before anything chooses a cleaner by them.
//!
//! The model hears 9.01 s of 16 kHz sound at a time. A clip is brought
//! down from 48 kHz, repeated until it's that long, and heard in windows a
//! second apart, each window's marks fitted to people's with the
//! polynomials DNSMOS publishes and then averaged, as its own
//! dnsmos_local.py does.

use std::f64::consts::{PI, TAU};
use std::path::Path;

use dictation::openvino::{Compiled, Core, Input};
use serde_json::{Value, json};

/// The rate the model hears at.
pub const HEARD: usize = 16_000;

/// The samples it hears at once: 9.01 s at 16 kHz.
pub const WINDOW: usize = 144_160;

/// The graph's own names for what goes in, and the file it's kept in.
const INPUT: &str = "input_1";
const FILE: &str = "dnsmos-sig-bak-ovr.onnx";

/// A clip's marks out of 5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marks {
    /// How natural the voice sounds (P.835's SIG).
    pub voice: f64,
    /// How quiet what's around it is (BAK).
    pub background: f64,
    /// Both together (OVRL).
    pub overall: f64,
}

impl Marks {
    /// The marks as a record says them.
    pub fn said(&self) -> Value {
        json!({
            "voice": self.voice,
            "background": self.background,
            "overall": self.overall,
        })
    }
}

/// The model, compiled for the CPU on one thread.
pub struct Scorer {
    compiled: Compiled,
    _core: Core,
}

impl Scorer {
    /// The model in `folder`.
    pub fn load(folder: &Path) -> Result<Self, String> {
        let core = Core::new(None)?;
        let mut model = core.read(&folder.join(FILE))?;
        model.fix_input(&[1, i64::try_from(WINDOW).unwrap_or(i64::MAX)])?;
        let compiled = core.compile_on_one_thread(&model)?;
        Ok(Self {
            compiled,
            _core: core,
        })
    }

    /// The marks of `clip`, at 48 kHz: each window's, fitted, averaged.
    pub fn marks(&self, clip: &[f32]) -> Result<Marks, String> {
        let heard = windows(&decimated(clip));
        if heard.is_empty() {
            return Err("there's nothing to score".to_owned());
        }
        let mut request = self.compiled.request()?;
        let dims = [1, i64::try_from(WINDOW).unwrap_or(i64::MAX)];
        let mut sum = Marks {
            voice: 0.0,
            background: 0.0,
            overall: 0.0,
        };
        for window in &heard {
            request.infer(&[Input::F32(INPUT, &dims, window)])?;
            let (_, said) = request.output_at(0)?;
            let said: [f32; 3] = said
                .get(..3)
                .and_then(|said| said.try_into().ok())
                .ok_or_else(|| "the scorer gave fewer than three marks".to_owned())?;
            let marks = fitted(said);
            sum.voice += marks.voice;
            sum.background += marks.background;
            sum.overall += marks.overall;
        }
        #[expect(clippy::cast_precision_loss, reason = "a handful of windows")]
        let count = heard.len() as f64;
        Ok(Marks {
            voice: sum.voice / count,
            background: sum.background / count,
            overall: sum.overall / count,
        })
    }
}

/// How many taps the low-pass filter has before every third sample is
/// kept, and where it stops passing sound, in cycles a sample: 7.5 kHz of
/// 48, under the 8 kHz that 16 kHz can hold, so nothing above folds into
/// the voice. A Blackman window keeps what's past it 70 dB down.
const TAPS: usize = 129;
const CUTOFF: f64 = 7_500.0 / 48_000.0;

/// The low-pass filter's taps, summing to 1 so a voice keeps its level.
fn taps() -> Vec<f64> {
    let half = TAPS / 2;
    #[expect(clippy::cast_precision_loss, reason = "a hundred or so taps")]
    let mut taps: Vec<f64> = (0..TAPS)
        .map(|tap| {
            let at = tap as f64 - half as f64;
            let sinc = if tap == half {
                2.0 * CUTOFF
            } else {
                (TAU * CUTOFF * at).sin() / (PI * at)
            };
            let phase = TAU * tap as f64 / (TAPS - 1) as f64;
            let blackman = 0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos();
            sinc * blackman
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    for tap in &mut taps {
        *tap /= sum;
    }
    taps
}

/// `samples` at 48 kHz brought down to 16 kHz: filtered below 8 kHz, then
/// every third sample kept.
pub fn decimated(samples: &[f32]) -> Vec<f32> {
    let taps = taps();
    let half = TAPS / 2;
    (0..samples.len() / 3)
        .map(|kept| {
            let centre = kept * 3 + half;
            let sum: f64 = taps
                .iter()
                .enumerate()
                .filter_map(|(tap, weight)| {
                    let sample = samples.get(centre.checked_sub(tap)?)?;
                    Some(weight * f64::from(*sample))
                })
                .sum();
            narrowed(sum)
        })
        .collect()
}

/// A filtered sample, back to the width the model takes.
#[expect(clippy::cast_possible_truncation, reason = "a sample, within ±1")]
const fn narrowed(sample: f64) -> f32 {
    sample as f32
}

/// The windows the model hears of `samples` at 16 kHz: the clip repeated
/// until it fills one, then each window a second on from the last, as
/// many as fit, as dnsmos_local.py takes them.
pub fn windows(samples: &[f32]) -> Vec<Vec<f32>> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mut tiled = samples.to_vec();
    while tiled.len() < WINDOW {
        tiled.extend_from_within(..);
    }
    let hops = (tiled.len() / HEARD).saturating_sub(9).max(1);
    (0..hops)
        .filter_map(|hop| tiled.get(hop * HEARD..hop * HEARD + WINDOW))
        .map(<[f32]>::to_vec)
        .collect()
}

/// The polynomials DNSMOS publishes to bring each of its marks to what
/// people say, highest power first.
const VOICE: [f64; 3] = [-0.083_972_78, 1.220_839_53, 0.005_243_9];
const BACKGROUND: [f64; 3] = [-0.131_668_88, 1.609_155_14, -0.396_045_46];
const OVERALL: [f64; 3] = [-0.067_662_83, 1.115_464_68, 0.046_025_35];

/// What people would say, from the model's own three marks.
pub fn fitted(said: [f32; 3]) -> Marks {
    let fit = |[square, linear, constant]: [f64; 3], mark: f32| {
        let mark = f64::from(mark);
        square * mark * mark + linear * mark + constant
    };
    Marks {
        voice: fit(VOICE, said[0]),
        background: fit(BACKGROUND, said[1]),
        overall: fit(OVERALL, said[2]),
    }
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
