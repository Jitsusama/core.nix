//! The NPU's lane for `DeepFilterNet`'s frames, with the processor behind
//! it. The NPU cleans a frame for a tenth of what the processor spends, but
//! it's shared, and while something else keeps it busy, as dictation's
//! Whisper can, a frame may come back far later than the 10 ms a frame
//! lasts. So each frame is started there and given at most [`BOUND`]; one
//! not back by then is cleaned by the processor from the same state, and
//! the processor goes on cleaning until the NPU is free again. Both run the
//! same graph, so the state either leaves is the other's to carry on from.

use std::time::Duration;

use crate::denoise::FRAME;

/// How long a frame waits on the NPU before the processor cleans it: with
/// the processor's 2 ms after, well inside the 10 ms a frame lasts.
pub const BOUND: Duration = Duration::from_millis(5);

/// A frame cleaned, with the state it leaves.
pub type Cleaned = ([f32; FRAME], Vec<f32>);

/// A device that cleans a frame without waiting for it to.
pub trait Run {
    /// Starts cleaning `frame` from `state`.
    ///
    /// # Errors
    ///
    /// When the device refuses it.
    fn start(&mut self, frame: &[f32; FRAME], state: &[f32]) -> Result<(), String>;

    /// Whether what was started is done, waiting at most `within`.
    ///
    /// # Errors
    ///
    /// When the run failed.
    fn done(&mut self, within: Duration) -> Result<bool, String>;

    /// What was started and is done.
    ///
    /// # Errors
    ///
    /// When it can't be read.
    fn taken(&mut self) -> Result<Cleaned, String>;
}

/// One microphone's lane on the NPU, when there's one.
pub struct Lane {
    run: Option<Box<dyn Run>>,
    /// Whether a frame the processor took over is still running there.
    busy: bool,
    /// How many frames the processor took over.
    late: u64,
}

impl Lane {
    /// A lane on `run`, or none, so the processor cleans every frame.
    pub fn on(run: Option<Box<dyn Run>>) -> Self {
        Self {
            run,
            busy: false,
            late: 0,
        }
    }

    /// How many frames the processor took over.
    pub fn late(&self) -> u64 {
        self.late
    }

    /// `frame` cleaned from `state`, which moves on by it: on the NPU when
    /// it's back within [`BOUND`], else by `behind`. A lane whose NPU fails
    /// is closed, and the processor cleans from then on.
    ///
    /// # Errors
    ///
    /// When `behind` fails.
    pub fn clean(
        &mut self,
        frame: &[f32; FRAME],
        state: &mut Vec<f32>,
        behind: impl FnOnce(&[f32; FRAME], &[f32]) -> Result<Cleaned, String>,
    ) -> Result<[f32; FRAME], String> {
        let (cleaned, next) = match self.ahead(frame, state) {
            Ok(Some(cleaned)) => cleaned,
            Ok(None) => behind(frame, state)?,
            Err(why) => {
                eprintln!("noise-cancelled: the NPU failed, so the processor cleans: {why}");
                self.run = None;
                behind(frame, state)?
            }
        };
        *state = next;
        Ok(cleaned)
    }

    /// `frame` cleaned on the NPU, or nothing when it's busy or late.
    fn ahead(&mut self, frame: &[f32; FRAME], state: &[f32]) -> Result<Option<Cleaned>, String> {
        let Some(run) = self.run.as_mut() else {
            return Ok(None);
        };
        if self.busy {
            if !run.done(Duration::ZERO)? {
                return Ok(None);
            }
            self.busy = false;
        }
        run.start(frame, state)?;
        if run.done(BOUND)? {
            return run.taken().map(Some);
        }
        self.busy = true;
        self.late += 1;
        Ok(None)
    }
}

impl Drop for Lane {
    /// Says how often the processor stood in, so a call's journal shows
    /// whether the NPU kept up.
    fn drop(&mut self) {
        if self.late() > 0 {
            eprintln!(
                "noise-cancelled: the processor cleaned {} frames the NPU was late with",
                self.late()
            );
        }
    }
}

#[cfg(test)]
#[path = "lane_tests.rs"]
mod tests;
