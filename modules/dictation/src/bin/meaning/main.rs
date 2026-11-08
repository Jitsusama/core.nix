//! meaning: what the launcher's things and what's typed into it mean, on
//! the NPU, so the launcher can list what Joel meant beside what he typed
//! (the glossary's search by meaning). It only adds: the shell lists what
//! it answers below everything typing found, never above.
//!
//! ```text
//! meaning serve SOCKET MODELS MEANINGS
//! meaning near MODELS WORDS [THING...]
//! ```
//!
//! `serve` answers the shell at SOCKET (known.rs), keeping each thing's
//! meaning in MEANINGS (store.rs), so a restart embeds only what's new.
//! One worker holds the model and does one thing at a time, a query
//! before any thing waiting, so a key waits behind at most one thing's
//! embedding; things wait while anything records from a microphone
//! (quiet.rs). The model is loaded once there's something to do and let
//! go once nothing has come for a while, so it holds no memory at rest.
//!
//! `near` prints how near each THING, or each line read when none is
//! given, means WORDS, nearest first, with what each step took, for
//! measuring the model and setting the floor. MODELS is the folder
//! modules/meaning/models.nix makes. `MEANING_DEVICE` names the device,
//! the NPU unless it says otherwise, falling back to the CPU, and
//! `MEANING_CACHE` where `OpenVINO` keeps what it compiles.

mod known;
mod quiet;
mod store;
mod text;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use dictation::daemon::{self, Shared};
use dictation::openvino::Core;

use crate::known::{Job, Known};
use crate::quiet::Quiet;
use crate::store::Store;
use crate::text::{Texts, near};

const USAGE: &str = "usage: meaning serve SOCKET MODELS MEANINGS
       meaning near MODELS WORDS [THING...]";

/// How long the model is kept once nothing's been asked, so a launcher
/// opened again soon finds it loaded.
const UNUSED: Duration = Duration::from_mins(5);

/// How many things are embedded between saves while many wait, so a
/// stop loses little.
const SAVED_EVERY: usize = 64;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.split_first() {
        Some((ask, [socket, models, meanings])) if ask == "serve" => {
            serve(Path::new(socket), Path::new(models), Path::new(meanings))
        }
        Some((ask, [models, words, things @ ..])) if ask == "near" => {
            print_near(Path::new(models), words, things)
        }
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("meaning: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The model, compiled for its device, with what it was compiled for.
struct Model {
    texts: Texts,
    device: String,
    // Last, so it's let go after what it compiled.
    _core: Core,
}

impl Model {
    /// The model on the device named, or on the CPU when that one can't
    /// take it, as on a machine with no NPU.
    fn load(folder: &Path) -> Result<Self, String> {
        let device = std::env::var("MEANING_DEVICE").unwrap_or_else(|_| "NPU".to_owned());
        match Self::load_on(folder, &device) {
            Err(why) if device != "CPU" => {
                eprintln!("meaning: on the CPU, since the {device} wouldn't: {why}");
                Self::load_on(folder, "CPU")
            }
            loaded => loaded,
        }
    }

    fn load_on(folder: &Path, device: &str) -> Result<Self, String> {
        let cache = std::env::var_os("MEANING_CACHE").map(PathBuf::from);
        let core = Core::new(cache.as_deref())?;
        let texts = Texts::load(&core, folder, device)?;
        Ok(Self {
            texts,
            device: device.to_owned(),
            _core: core,
        })
    }
}

/// Answers the shell, embedding what it's told and asked, until the
/// socket can't be listened on.
fn serve(socket: &Path, models: &Path, meanings: &Path) -> Result<(), String> {
    let (wake, woken) = mpsc::channel();
    let store = Store::read(meanings, &models.to_string_lossy());
    let known = Arc::new(Shared::new(Known::new(store, wake)));
    let listener = daemon::listen(socket)?;
    {
        let known = Arc::clone(&known);
        std::thread::spawn(move || daemon::answer_all(&known, &listener));
    }
    eprintln!(
        "meaning: answering, {} meanings kept",
        known.lock().store.len()
    );
    work(&known, &woken, models)
}

/// The worker: one job at a time, each from what's shared, waiting for a
/// wake when there's none.
fn work(known: &Shared<Known>, woken: &Receiver<()>, models: &Path) -> Result<(), String> {
    let ended = || "the socket's listener ended".to_owned();
    let mut loaded: Option<Model> = None;
    let mut quiet = Quiet::default();
    let mut embedded = 0;
    // Things embedded since the worker last had nothing to do, said once
    // it has, so the journal (and the check) sees each pass end.
    let mut this_pass = 0;
    loop {
        let job = known.lock().next();
        match job {
            Some(Job::Query(query)) => {
                let model = loaded_or_load(&mut loaded, known, models)?;
                let began = Instant::now();
                match model.texts.embed(&query.words, true) {
                    Ok(meaning) => {
                        let took = began.elapsed().as_millis();
                        let mut held = known.lock();
                        held.last_ms = Some(took);
                        held.answer_query(&query, &meaning);
                    }
                    Err(why) => {
                        eprintln!("meaning: {:?} wasn't embedded: {why}", query.words);
                        let line = daemon::error(&query.id, "failed", &why);
                        let _ = query.told.send(line.to_string());
                    }
                }
            }
            Some(Job::Embed(text)) => {
                let recording = quiet.recording();
                known.lock().held_back = recording;
                if recording {
                    // A query still wakes it; things wait for the quiet.
                    match woken.recv_timeout(quiet::KEPT) {
                        Ok(()) | Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => return Err(ended()),
                    }
                }
                let model = loaded_or_load(&mut loaded, known, models)?;
                match model.texts.embed(&text, false) {
                    Ok(meaning) => {
                        let mut held = known.lock();
                        held.store.put(&text, meaning);
                        embedded += 1;
                        this_pass += 1;
                        if embedded % SAVED_EVERY == 0 {
                            held.store.save();
                        }
                    }
                    Err(why) => {
                        eprintln!("meaning: {text:?} wasn't embedded: {why}");
                        // Kept as meaning nothing, so it isn't tried on
                        // every pass; a new model or build tries again.
                        known.lock().store.put(&text, vec![0.0; text::WIDTH]);
                    }
                }
            }
            None => {
                let kept = {
                    let mut held = known.lock();
                    held.store.save();
                    held.store.len()
                };
                if this_pass > 0 {
                    eprintln!("meaning: {this_pass} things meant, {kept} kept");
                    this_pass = 0;
                }
                if loaded.is_some() {
                    match woken.recv_timeout(UNUSED) {
                        Ok(()) => {}
                        Err(RecvTimeoutError::Timeout) => {
                            loaded = None;
                            eprintln!("meaning: the model let go");
                        }
                        Err(RecvTimeoutError::Disconnected) => return Err(ended()),
                    }
                } else {
                    woken.recv().map_err(|_| ended())?;
                }
            }
        }
    }
}

/// The model, loaded now if it isn't.
fn loaded_or_load<'a>(
    loaded: &'a mut Option<Model>,
    known: &Shared<Known>,
    models: &Path,
) -> Result<&'a mut Model, String> {
    if loaded.is_none() {
        let began = Instant::now();
        let model = Model::load(models)?;
        eprintln!(
            "meaning: loaded on the {} in {} ms",
            model.device,
            began.elapsed().as_millis()
        );
        known.lock().device.clone_from(&model.device);
        *loaded = Some(model);
    }
    loaded
        .as_mut()
        .ok_or_else(|| "the model wasn't loaded".to_owned())
}

/// Prints how near each thing means the words, nearest first, saying
/// what loading and each embedding took.
fn print_near(models: &Path, words: &str, things: &[String]) -> Result<(), String> {
    let things: Vec<String> = if things.is_empty() {
        std::io::stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .collect()
    } else {
        things.to_vec()
    };
    let began = Instant::now();
    let mut model = Model::load(models)?;
    eprintln!(
        "meaning: loaded on the {} in {} ms",
        model.device,
        began.elapsed().as_millis()
    );
    let began = Instant::now();
    let asked = model.texts.embed(words, true)?;
    eprintln!(
        "meaning: the words in {:.2} ms",
        began.elapsed().as_secs_f64() * 1000.0
    );
    let began = Instant::now();
    let mut found = Vec::with_capacity(things.len());
    for thing in &things {
        found.push((near(&asked, &model.texts.embed(thing, false)?), thing));
    }
    let took = began.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "meaning: {} things in {took:.1} ms, {:.2} ms each",
        things.len(),
        took / f64::from(u32::try_from(things.len().max(1)).unwrap_or(u32::MAX))
    );
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (near, thing) in found {
        println!("{near:.3}\t{thing}");
    }
    Ok(())
}
