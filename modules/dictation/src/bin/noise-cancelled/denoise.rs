//! `DeepFilterNet` 3, a 10 ms frame at a time, through the graph
//! modules/noise-cancelled/model.nix exports: the frame and every state
//! the denoiser keeps go in, the cleaned frame and the next state come out.

use std::path::Path;

use dictation::openvino::{Compiled, Core, Input, Request};

/// The samples in a frame: 10 ms at 48 kHz.
pub const FRAME: usize = 480;

/// The rate it hears and speaks at.
pub const RATE: u32 = 48_000;

/// How many floats of state the denoiser carries from frame to frame.
const STATE: usize = 45_304;

/// The graph's own names for what goes in and out.
const FRAME_IN: &str = "input_frame";
const STATE_IN: &str = "states";
const FRAME_OUT: &str = "enhanced_audio_frame";
const STATE_OUT: &str = "out_states";

/// The model compiled once, for every microphone's denoiser.
pub struct Model {
    compiled: Compiled,
    _core: Core,
}

impl Model {
    /// The exported graph in `folder`, compiled for the CPU on one thread.
    pub fn load(folder: &Path) -> Result<Self, String> {
        let core = Core::new(None)?;
        let model = core.read(&folder.join("deepfilternet3-frame.onnx"))?;
        let compiled = core.compile_on_one_thread(&model)?;
        Ok(Self {
            compiled,
            _core: core,
        })
    }

    /// A denoiser of its own, starting from silence.
    pub fn denoiser(&self) -> Result<Denoiser, String> {
        Ok(Denoiser {
            request: self.compiled.request()?,
            state: vec![0.0; STATE],
        })
    }
}

/// One microphone's denoiser, holding what it's heard so far.
pub struct Denoiser {
    request: Request,
    state: Vec<f32>,
}

impl Denoiser {
    /// One frame cleaned, the denoiser moving on by it.
    pub fn clean(&mut self, frame: &[f32; FRAME]) -> Result<[f32; FRAME], String> {
        let frame_dims = [i64::try_from(FRAME).unwrap_or(i64::MAX)];
        let state_dims = [i64::try_from(STATE).unwrap_or(i64::MAX)];
        let (_, cleaned) = self.request.run(
            &[
                Input::F32(FRAME_IN, &frame_dims, frame),
                Input::F32(STATE_IN, &state_dims, &self.state),
            ],
            FRAME_OUT,
            None,
        )?;
        let (_, state) = self.request.output(STATE_OUT, None)?;
        if state.len() != STATE {
            return Err(format!(
                "the state came back {} long, not {STATE}",
                state.len()
            ));
        }
        self.state = state;
        cleaned
            .try_into()
            .map_err(|cleaned: Vec<f32>| format!("a frame came back {} long", cleaned.len()))
    }
}
