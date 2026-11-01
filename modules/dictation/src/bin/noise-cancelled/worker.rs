//! The denoiser's own thread, which every microphone's sound passes
//! through. `OpenVINO`'s objects stay on the thread that made them, and a
//! frame can take far longer than PipeWire's real-time thread may wait, so
//! PipeWire's side only hands samples over through rings (pipe.rs), and
//! this thread cleans them a frame at a time.
//!
//! The model is loaded when a microphone's source is first recorded from
//! and let go once none has been for a while, so nothing is held between
//! calls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use rtrb::{Consumer, Producer};

use crate::denoise::{Denoiser, FRAME, Model};

/// How long the model stays loaded once nothing records from a source.
const UNUSED: Duration = Duration::from_mins(2);

/// How long to wait before loading a model that wouldn't load again.
const RETRY: Duration = Duration::from_mins(1);

/// What one microphone's two streams and this thread share.
#[derive(Default)]
pub struct Shared {
    /// Whether something records from the source, so its frames are wanted.
    pub wanted: AtomicBool,
    /// Counts each time recording starts, so the source drops what was
    /// cleaned before it.
    pub started: AtomicU32,
    /// Times the source had nothing cleaned to give, since it last started.
    pub late: AtomicU64,
    /// Frames with nowhere to go because a ring was full, since it last
    /// started.
    pub lost: AtomicU64,
}

/// This thread's ends of one microphone's rings.
pub struct Ends {
    /// What the microphone heard, from its capture stream.
    pub heard: Consumer<f32>,
    /// What's been cleaned, for its source stream.
    pub cleaned: Producer<f32>,
    pub shared: Arc<Shared>,
}

enum Ask {
    Add(u32, Ends),
    Remove(u32),
}

/// The handle PipeWire's side keeps on the denoiser's thread.
pub struct Worker {
    asks: Sender<Ask>,
    thread: Thread,
}

impl Worker {
    /// Starts the thread, which reads the model from `models` when it's
    /// first needed.
    pub fn start(models: PathBuf) -> Result<Self, String> {
        let (asks, heard) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("denoiser".into())
            .spawn(move || run(&models, &heard))
            .map_err(|why| format!("the denoiser's thread wouldn't start: {why}"))?;
        Ok(Self {
            asks,
            thread: handle.thread().clone(),
        })
    }

    /// Wakes the thread, for a stream that's handed it sound.
    pub fn waker(&self) -> Thread {
        self.thread.clone()
    }

    /// Hands it a microphone's ends of its rings.
    pub fn add(&self, id: u32, ends: Ends) {
        let _ = self.asks.send(Ask::Add(id, ends));
        self.thread.unpark();
    }

    /// Forgets a microphone that's gone.
    pub fn remove(&self, id: u32) {
        let _ = self.asks.send(Ask::Remove(id));
        self.thread.unpark();
    }
}

/// One microphone as this thread holds it.
struct Microphone {
    ends: Ends,
    denoiser: Option<Denoiser>,
}

/// The model, loaded only while it's wanted.
enum Loaded {
    No,
    Yes(Model),
    Refused(Instant),
}

fn run(models: &std::path::Path, asks: &Receiver<Ask>) {
    let mut microphones: HashMap<u32, Microphone> = HashMap::new();
    let mut model = Loaded::No;
    let mut last_wanted = Instant::now();
    loop {
        loop {
            match asks.try_recv() {
                Ok(Ask::Add(id, ends)) => {
                    microphones.insert(
                        id,
                        Microphone {
                            ends,
                            denoiser: None,
                        },
                    );
                }
                Ok(Ask::Remove(id)) => {
                    microphones.remove(&id);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let wanted = microphones
            .values()
            .any(|microphone| microphone.ends.shared.wanted.load(Ordering::Acquire));
        if wanted {
            last_wanted = Instant::now();
            model = loaded(model, models);
        } else if matches!(model, Loaded::Yes(_)) && last_wanted.elapsed() > UNUSED {
            model = Loaded::No;
            eprintln!("noise-cancelled: the model let go");
        }
        for microphone in microphones.values_mut() {
            clean(microphone, &model);
        }
        thread::park_timeout(if wanted {
            Duration::from_millis(50)
        } else {
            UNUSED
        });
    }
}

/// The model, loading it unless it's loaded or was refused a moment ago.
fn loaded(model: Loaded, models: &std::path::Path) -> Loaded {
    match model {
        Loaded::Yes(_) => model,
        Loaded::Refused(when) if when.elapsed() < RETRY => model,
        Loaded::No | Loaded::Refused(_) => {
            let started = Instant::now();
            match Model::load(models) {
                Ok(loaded) => {
                    eprintln!(
                        "noise-cancelled: the model loaded in {} ms",
                        started.elapsed().as_millis()
                    );
                    Loaded::Yes(loaded)
                }
                Err(why) => {
                    eprintln!(
                        "noise-cancelled: the model wouldn't load, so voices pass as heard: {why}"
                    );
                    Loaded::Refused(Instant::now())
                }
            }
        }
    }
}

/// Cleans every whole frame one microphone has heard. Its voice passes as
/// heard when the model won't load or a frame won't clean, since a
/// microphone that goes quiet in a call is worse than one that's noisy.
fn clean(microphone: &mut Microphone, model: &Loaded) {
    let ends = &mut microphone.ends;
    if !ends.shared.wanted.load(Ordering::Acquire) {
        microphone.denoiser = None;
        let stale = ends.heard.slots();
        if let Ok(chunk) = ends.heard.read_chunk(stale) {
            chunk.commit_all();
        }
        return;
    }
    if microphone.denoiser.is_none()
        && let Loaded::Yes(model) = model
    {
        match model.denoiser() {
            Ok(denoiser) => microphone.denoiser = Some(denoiser),
            Err(why) => eprintln!("noise-cancelled: a denoiser wouldn't start: {why}"),
        }
    }
    let mut frame = [0.0; FRAME];
    while ends.heard.pop_entire_slice(&mut frame).is_ok() {
        let cleaned = match microphone
            .denoiser
            .as_mut()
            .map(|denoiser| denoiser.clean(&frame))
        {
            Some(Ok(cleaned)) => cleaned,
            Some(Err(why)) => {
                eprintln!("noise-cancelled: a frame wouldn't clean: {why}");
                frame
            }
            None => frame,
        };
        if ends.cleaned.push_entire_slice(&cleaned).is_err() {
            ends.shared.lost.fetch_add(1, Ordering::Relaxed);
        }
    }
}
