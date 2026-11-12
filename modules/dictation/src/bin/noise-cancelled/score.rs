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

use std::path::Path;

use dictation::openvino::{Compiled, Core, Input};

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

    /// The marks of `clip`, at 48 kHz.
    pub fn marks(&self, clip: &[f32]) -> Result<Marks, String> {
        let _ = (&self.compiled, clip, INPUT);
        Err("not yet".to_owned())
    }
}

/// `samples` at 48 kHz brought down to 16 kHz.
pub fn decimated(samples: &[f32]) -> Vec<f32> {
    let _ = samples;
    Vec::new()
}

/// The windows the model hears of `samples` at 16 kHz.
pub fn windows(samples: &[f32]) -> Vec<Vec<f32>> {
    let _ = samples;
    Vec::new()
}

/// What people would say, from the model's own three marks.
pub fn fitted(said: [f32; 3]) -> Marks {
    let _ = said;
    Marks {
        voice: 0.0,
        background: 0.0,
        overall: 0.0,
    }
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
