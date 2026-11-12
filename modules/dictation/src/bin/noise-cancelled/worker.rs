//! The denoiser's own thread, which every microphone's sound passes
//! through. `OpenVINO`'s requests are used from one thread at a time, and a
//! frame can take far longer than PipeWire's real-time thread may wait, so
//! PipeWire's side only hands samples over through rings (pipe.rs), and
//! this thread cleans them a frame at a time.
//!
//! The model is loaded when a microphone's source is first recorded from
//! and let go once none has been for a while, so nothing is held between
//! calls. It loads on a thread of its own, taking about half a second, and
//! voices pass as heard meanwhile, since a call's first words matter more
//! than their noise. Each microphone cleans as Joel last picked by ear for it
//! (chosen.rs), settled as it starts being recorded and kept until it stops,
//! so a pick changes the next call and never one under way. Each cleaner in
//! use has its model loaded once, however many microphones share it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use rtrb::{Consumer, Producer};

use crate::chosen::Chosen;
use crate::denoise::{Cleaner, Denoiser, FRAME, Model};

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
    /// The microphone's node name, which its pick is kept under.
    pub microphone: String,
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
    /// Starts the thread, which cleans each microphone with the cleaner
    /// `chosen` says for it, and reads what it needs from `models` when it's first
    /// needed.
    pub fn start(models: PathBuf, chosen: Chosen) -> Result<Self, String> {
        let (asks, heard) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("denoiser".into())
            .spawn(move || run(&models, &chosen, &heard))
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
    /// What it cleans with while it's recorded from.
    cleaner: Option<Cleaner>,
    denoiser: Option<Denoiser>,
}

/// The model, loaded only while it's wanted.
enum Loaded {
    No,
    Loading(Receiver<Result<Model, String>>),
    Yes(Model),
    Refused(Instant),
}

fn run(models: &Path, chosen: &Chosen, asks: &Receiver<Ask>) {
    let mut microphones: HashMap<u32, Microphone> = HashMap::new();
    let mut loads: HashMap<Cleaner, Loaded> = HashMap::new();
    let mut last_wanted = Instant::now();
    loop {
        loop {
            match asks.try_recv() {
                Ok(Ask::Add(id, ends)) => {
                    microphones.insert(
                        id,
                        Microphone {
                            ends,
                            cleaner: None,
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
        let wanted = settled(&mut microphones, chosen);
        if !wanted.is_empty() {
            last_wanted = Instant::now();
            for cleaner in &wanted {
                let model = loads.remove(cleaner).unwrap_or(Loaded::No);
                loads.insert(*cleaner, loaded(model, models, *cleaner));
            }
        } else if loads.values().any(|model| matches!(model, Loaded::Yes(_)))
            && last_wanted.elapsed() > UNUSED
        {
            loads.clear();
            eprintln!("noise-cancelled: the model let go");
        }
        for microphone in microphones.values_mut() {
            let model = microphone.cleaner.and_then(|cleaner| loads.get(&cleaner));
            clean(microphone, model);
        }
        thread::park_timeout(if wanted.is_empty() {
            UNUSED
        } else {
            Duration::from_millis(50)
        });
    }
}

/// Settles what each microphone cleans with: as `chosen` says for it when
/// it starts being recorded, kept until it stops, and nothing once it has.
/// Answers every cleaner wanted now, each once.
fn settled(microphones: &mut HashMap<u32, Microphone>, chosen: &Chosen) -> Vec<Cleaner> {
    let mut wanted = Vec::new();
    for microphone in microphones.values_mut() {
        if !microphone.ends.shared.wanted.load(Ordering::Acquire) {
            microphone.cleaner = None;
            continue;
        }
        let cleaner = *microphone.cleaner.get_or_insert_with(|| {
            let cleaner = chosen.cleaner(Some(&microphone.ends.microphone));
            eprintln!(
                "noise-cancelled: {} cleans with {cleaner}",
                microphone.ends.microphone
            );
            cleaner
        });
        if !wanted.contains(&cleaner) {
            wanted.push(cleaner);
        }
    }
    wanted
}

/// The model, loading it unless it's loaded or was refused a moment ago.
fn loaded(model: Loaded, models: &Path, cleaner: Cleaner) -> Loaded {
    match model {
        Loaded::Yes(_) => model,
        Loaded::Refused(when) if when.elapsed() < RETRY => model,
        Loaded::Loading(_) => arrived(model),
        Loaded::No | Loaded::Refused(_) => {
            let models = models.to_path_buf();
            loading(cleaner, move || Model::load(&models, cleaner))
        }
    }
}

/// Starts loading `cleaner`'s model with `load`, on a thread of its own,
/// so this one goes on passing voices as heard while it loads, and wakes
/// this one once it has.
fn loading(
    cleaner: Cleaner,
    load: impl FnOnce() -> Result<Model, String> + Send + 'static,
) -> Loaded {
    let (sends, arrives) = mpsc::channel();
    let waiting = thread::current();
    let started = thread::Builder::new()
        .name("model loader".into())
        .spawn(move || {
            let started = Instant::now();
            let loaded = load();
            if loaded.is_ok() {
                eprintln!(
                    "noise-cancelled: the model loaded in {} ms, cleaning with {cleaner}",
                    started.elapsed().as_millis()
                );
            }
            let _ = sends.send(loaded);
            waiting.unpark();
        });
    match started {
        Ok(_) => Loaded::Loading(arrives),
        Err(why) => refused(&format!("its loader wouldn't start: {why}")),
    }
}

/// The model, once it's arrived from loading.
fn arrived(model: Loaded) -> Loaded {
    let Loaded::Loading(arrives) = model else {
        return model;
    };
    match arrives.try_recv() {
        Ok(Ok(loaded)) => Loaded::Yes(loaded),
        Ok(Err(why)) => refused(&why),
        Err(TryRecvError::Empty) => Loaded::Loading(arrives),
        Err(TryRecvError::Disconnected) => refused("its loader stopped"),
    }
}

/// A model that wouldn't load, tried again after a while.
fn refused(why: &str) -> Loaded {
    eprintln!("noise-cancelled: the model wouldn't load, so voices pass as heard: {why}");
    Loaded::Refused(Instant::now())
}

/// Cleans every whole frame one microphone has heard. Its voice passes as
/// heard when the model won't load or a frame won't clean, since a
/// microphone that goes quiet in a call is worse than one that's noisy.
fn clean(microphone: &mut Microphone, model: Option<&Loaded>) {
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
        && let Some(Loaded::Yes(model)) = model
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

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
