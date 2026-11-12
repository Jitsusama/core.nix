//! The cleaners a microphone's voice can be taken through, each a 10 ms
//! frame at a time (the glossary's noise cancelled):
//!
//! - `deepfilternet`: `DeepFilterNet` 3, through the graph
//!   modules/noise-cancelled/model.nix exports. The frame and every state
//!   the denoiser keeps go in, the cleaned frame and the next state come
//!   out.
//! - `deepfilternet-gentle`: the same, with the microphone's own sound
//!   mixed back in [`GENTLE_DB`] under it, as `DeepFilterNet`'s attenuation
//!   limit does, so the room is turned down rather than taken out.
//! - `rnnoise`: `RNNoise`, through nnnoiseless, far lighter and needing no
//!   model from MODELS.

use std::collections::VecDeque;
use std::fmt;
use std::path::Path;

use dictation::openvino::{Compiled, Core, Input, Request};
use nnnoiseless::DenoiseState;

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

/// How far behind what it hears `DeepFilterNet` gives it back: its window
/// past its hop, and the two frames it looks ahead, 30 ms. Measured on its
/// repository's voice as the lag where what it heard and what it gave
/// agree best, 0.96 against 0.91 a sample either side.
const LATE: usize = 3 * FRAME;

/// How far under the cleaned voice the gentle cleaner keeps the room.
pub const GENTLE_DB: f32 = 12.0;

/// What nnnoiseless takes a sample as: 16-bit PCM's scale, not 1.
const PCM: f32 = 32_768.0;

/// One way of cleaning a voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cleaner {
    DeepFilterNet,
    DeepFilterNetGentle,
    RNNoise,
}

impl Cleaner {
    /// Every cleaner, in the order they're offered.
    pub const ALL: [Self; 3] = [
        Self::DeepFilterNet,
        Self::DeepFilterNetGentle,
        Self::RNNoise,
    ];

    /// The cleaner a name says, as the command line and Nix name them.
    pub fn named(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|cleaner| cleaner.name() == name)
            .ok_or_else(|| {
                let names: Vec<_> = Self::ALL.iter().map(|cleaner| cleaner.name()).collect();
                format!("no cleaner is named {name}; there are {}", names.join(", "))
            })
    }

    /// Its name, as the command line and Nix name it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DeepFilterNet => "deepfilternet",
            Self::DeepFilterNetGentle => "deepfilternet-gentle",
            Self::RNNoise => "rnnoise",
        }
    }
}

impl fmt::Display for Cleaner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a cleaner needs loaded once, for every microphone's denoiser.
pub struct Model {
    cleaner: Cleaner,
    graph: Option<Graph>,
}

/// `DeepFilterNet`'s graph, compiled.
struct Graph {
    compiled: Compiled,
    _core: Core,
}

// A model is loaded on a thread of its own and handed to the denoiser's,
// then used only there. `OpenVINO`'s core and compiled models are safe to
// use from any thread, so moving one once is too.
unsafe impl Send for Model {}

impl Model {
    /// What `cleaner` needs from `folder`: `DeepFilterNet`'s exported graph,
    /// compiled for the CPU on one thread, or nothing for `RNNoise`.
    pub fn load(folder: &Path, cleaner: Cleaner) -> Result<Self, String> {
        let graph = match cleaner {
            Cleaner::RNNoise => None,
            Cleaner::DeepFilterNet | Cleaner::DeepFilterNetGentle => {
                let core = Core::new(None)?;
                let model = core.read(&folder.join("deepfilternet3-frame.onnx"))?;
                let compiled = core.compile_on_one_thread(&model)?;
                Some(Graph {
                    compiled,
                    _core: core,
                })
            }
        };
        Ok(Self { cleaner, graph })
    }

    /// A denoiser of its own, starting from silence.
    pub fn denoiser(&self) -> Result<Denoiser, String> {
        let deep = |graph: &Graph| -> Result<Deep, String> {
            Ok(Deep {
                request: graph.compiled.request()?,
                state: vec![0.0; STATE],
            })
        };
        let graph = self.graph.as_ref();
        let missing = || format!("{} has no graph loaded", self.cleaner);
        Ok(match self.cleaner {
            Cleaner::DeepFilterNet => Denoiser::Deep(deep(graph.ok_or_else(missing)?)?),
            Cleaner::DeepFilterNetGentle => Denoiser::Gentle(
                deep(graph.ok_or_else(missing)?)?,
                VecDeque::from(vec![0.0; LATE]),
            ),
            Cleaner::RNNoise => Denoiser::RNNoise(DenoiseState::new()),
        })
    }
}

/// One microphone's denoiser, holding what it's heard so far.
pub enum Denoiser {
    Deep(Deep),
    /// `DeepFilterNet`, and what it heard, held back as long as it takes.
    Gentle(Deep, VecDeque<f32>),
    RNNoise(Box<DenoiseState<'static>>),
}

/// `DeepFilterNet`'s request and the state it carries.
pub struct Deep {
    request: Request,
    state: Vec<f32>,
}

impl Denoiser {
    /// One frame cleaned, the denoiser moving on by it.
    pub fn clean(&mut self, frame: &[f32; FRAME]) -> Result<[f32; FRAME], String> {
        match self {
            Self::Deep(deep) => deep.clean(frame),
            Self::Gentle(deep, heard) => {
                let cleaned = deep.clean(frame)?;
                heard.extend(frame);
                let kept = 10_f32.powf(-GENTLE_DB / 20.0);
                let mut mixed = [0.0; FRAME];
                for ((out, cleaned), heard) in
                    mixed.iter_mut().zip(cleaned).zip(heard.drain(..FRAME))
                {
                    *out = cleaned + kept * (heard - cleaned);
                }
                Ok(mixed)
            }
            Self::RNNoise(state) => {
                let scaled = frame.map(|sample| sample * PCM);
                let mut cleaned = [0.0; FRAME];
                state.process_frame(&mut cleaned, &scaled);
                Ok(cleaned.map(|sample| sample / PCM))
            }
        }
    }
}

impl Deep {
    fn clean(&mut self, frame: &[f32; FRAME]) -> Result<[f32; FRAME], String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cleaner_is_found_by_its_name() {
        for cleaner in Cleaner::ALL {
            assert_eq!(Cleaner::named(cleaner.name()), Ok(cleaner), "{cleaner}");
        }
    }

    #[test]
    fn a_cleaner_nobody_has_says_which_there_are() {
        let said = Cleaner::named("krisp").expect_err("no krisp");
        assert!(
            said.contains("deepfilternet, deepfilternet-gentle, rnnoise"),
            "it said {said}"
        );
    }
}
