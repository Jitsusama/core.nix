//! The cleaners a microphone's voice can be taken through, each a 10 ms
//! frame at a time (the glossary's noise cancelled):
//!
//! - `deepfilternet`: `DeepFilterNet` 3, through the graph
//!   modules/noise-cancelled/model.nix exports, as rewritten to hold in
//!   half precision, the same maths in single, on the NPU with the
//!   processor behind it for a frame the NPU is late with (lane.rs says
//!   how). The frame and every state the denoiser keeps go in, the
//!   cleaned frame and the next state come out.
//! - `deepfilternet-gentle`: the same, with the microphone's own sound
//!   mixed back in [`GENTLE_DB`] under it, as `DeepFilterNet`'s attenuation
//!   limit does, so the room is turned down rather than taken out.
//! - `rnnoise`: `RNNoise`, through nnnoiseless, far lighter and needing no
//!   model from MODELS.

use std::collections::VecDeque;
use std::fmt;
use std::path::Path;
use std::thread;
use std::time::Duration;

use dictation::openvino::{Compiled, Core, Input, Priority, Request, Started};
use nnnoiseless::DenoiseState;

use crate::lane::{Cleaned, Lane, Run};

/// The samples in a frame: 10 ms at 48 kHz.
pub const FRAME: usize = 480;

/// The rate it hears and speaks at.
pub const RATE: u32 = 48_000;

/// How many floats of state the denoiser carries from frame to frame.
const STATE: usize = 45_304;

/// `DeepFilterNet`'s graph in MODELS, as model.nix writes it.
const GRAPH: &str = "deepfilternet3-frame-f16.xml";

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

/// `DeepFilterNet`'s graph, compiled for the processor, and for the NPU
/// when there's one.
struct Graph {
    compiled: Compiled,
    ahead: Option<Ahead>,
    _core: Core,
}

/// The graph compiled for the device frames go to first.
struct Ahead {
    compiled: Compiled,
    _core: Core,
}

// A model is loaded on a thread of its own and handed to the denoiser's,
// then used only there, and the graph ahead is compiled on one more beside
// it. `OpenVINO`'s core and compiled models are safe to use from any
// thread, so moving one once is too.
unsafe impl Send for Model {}
unsafe impl Send for Ahead {}

/// The device frames go to first: the NPU, unless `NOISE_CANCELLED_DEVICE`
/// names another, as a check names the processor where there's no NPU.
fn ahead_on() -> String {
    std::env::var("NOISE_CANCELLED_DEVICE").unwrap_or_else(|_| "NPU".to_owned())
}

impl Ahead {
    /// The graph in `file` compiled for `device`, ahead of everything the
    /// NPU runs that waits less.
    fn load(file: &Path, device: &str) -> Result<Self, String> {
        let core = Core::new(None)?;
        let compiled = core.compile_file(file, device, Priority::Live)?;
        Ok(Self {
            compiled,
            _core: core,
        })
    }
}

impl Model {
    /// What `cleaner` needs from `folder`: `DeepFilterNet`'s graph,
    /// compiled for the CPU on one thread and for the NPU at once, so it's
    /// no later to load than the processor's alone; or nothing for
    /// `RNNoise`. Where the NPU won't take it, the processor cleans alone.
    pub fn load(folder: &Path, cleaner: Cleaner) -> Result<Self, String> {
        let graph = match cleaner {
            Cleaner::RNNoise => None,
            Cleaner::DeepFilterNet | Cleaner::DeepFilterNetGentle => {
                let file = folder.join(GRAPH);
                let device = ahead_on();
                let loading = {
                    let (file, device) = (file.clone(), device.clone());
                    thread::Builder::new()
                        .name("model loader ahead".into())
                        .spawn(move || Ahead::load(&file, &device))
                };
                let core = Core::new(None)?;
                let compiled = core.compile_on_one_thread(&core.read(&file)?)?;
                let ahead = loading
                    .map_err(|why| format!("its loader wouldn't start: {why}"))
                    .and_then(|loader| {
                        loader.join().map_err(|_| "its loader stopped".to_owned())?
                    });
                Some(Graph {
                    compiled,
                    ahead: ahead
                        .inspect_err(|why| {
                            eprintln!(
                                "noise-cancelled: the processor cleans every frame, \
                                 as {device} won't: {why}"
                            );
                        })
                        .ok(),
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
                lane: Lane::on(graph.ahead.as_ref().and_then(Npu::on)),
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

/// `DeepFilterNet`'s request on the processor, the state it carries, and
/// its lane on the NPU.
pub struct Deep {
    request: Request,
    state: Vec<f32>,
    lane: Lane,
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
        let request = &mut self.request;
        self.lane.clean(frame, &mut self.state, |frame, state| {
            behind(request, frame, state)
        })
    }
}

/// The shapes of a frame and a state, as the graph takes them.
fn dims() -> ([i64; 1], [i64; 1]) {
    (
        [i64::try_from(FRAME).unwrap_or(i64::MAX)],
        [i64::try_from(STATE).unwrap_or(i64::MAX)],
    )
}

/// `frame` cleaned by the processor from `state`.
fn behind(request: &mut Request, frame: &[f32; FRAME], state: &[f32]) -> Result<Cleaned, String> {
    let (frame_dims, state_dims) = dims();
    let (_, cleaned) = request.run(
        &[
            Input::F32(FRAME_IN, &frame_dims, frame),
            Input::F32(STATE_IN, &state_dims, state),
        ],
        FRAME_OUT,
        None,
    )?;
    let (_, next) = request.output(STATE_OUT, None)?;
    as_cleaned(cleaned, next)
}

/// A cleaned frame and the state it left, as the graph gave them.
fn as_cleaned(cleaned: Vec<f32>, state: Vec<f32>) -> Result<Cleaned, String> {
    if state.len() != STATE {
        return Err(format!(
            "the state came back {} long, not {STATE}",
            state.len()
        ));
    }
    let cleaned = cleaned
        .try_into()
        .map_err(|cleaned: Vec<f32>| format!("a frame came back {} long", cleaned.len()))?;
    Ok((cleaned, state))
}

/// A denoiser's request on the NPU, with the frame and state it reads in
/// place while it runs, held here until it's done.
struct Npu {
    // Declared first, so a run still going is waited out before what it
    // reads is freed.
    started: Started,
    frame: Vec<f32>,
    state: Vec<f32>,
}

impl Npu {
    /// A request on `ahead`, or none, saying why, when it won't make one.
    fn on(ahead: &Ahead) -> Option<Box<dyn Run>> {
        match ahead.compiled.request().and_then(Started::new) {
            Ok(started) => Some(Box::new(Self {
                started,
                frame: vec![0.0; FRAME],
                state: vec![0.0; STATE],
            })),
            Err(why) => {
                eprintln!("noise-cancelled: the processor cleans every frame: {why}");
                None
            }
        }
    }
}

impl Run for Npu {
    fn start(&mut self, frame: &[f32; FRAME], state: &[f32]) -> Result<(), String> {
        if state.len() != STATE {
            return Err(format!("a state {} long, not {STATE}", state.len()));
        }
        self.frame.copy_from_slice(frame);
        self.state.copy_from_slice(state);
        let (frame_dims, state_dims) = dims();
        self.started.start(&[
            Input::F32(FRAME_IN, &frame_dims, &self.frame),
            Input::F32(STATE_IN, &state_dims, &self.state),
        ])
    }

    fn done(&mut self, within: Duration) -> Result<bool, String> {
        self.started.done(within)
    }

    fn taken(&mut self) -> Result<Cleaned, String> {
        let (_, cleaned) = self.started.output(FRAME_OUT, None)?;
        let (_, state) = self.started.output(STATE_OUT, None)?;
        as_cleaned(cleaned, state)
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
