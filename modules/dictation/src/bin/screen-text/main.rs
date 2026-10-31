//! screen-text: reads the words in Joel's screenshots on the NPU, so they
//! can be found by them and copied out (the glossary's screenshot text).
//!
//! `PaddleOCR`'s `PP-OCRv5` does it in two models: one finds where each line
//! of text is (find.rs), the other reads each line (read.rs). Both are
//! compiled at fixed shapes, as the NPU needs, at its background priority,
//! since nobody waits on a screenshot's words.
//!
//! ```text
//! screen-text serve SOCKET MODELS FOLDER RECORDS
//! screen-text read MODELS PICTURE...
//! ```
//!
//! `serve` reads each picture saved into FOLDER, the screenshots', once as
//! it lands, and the ones there before it started that it hasn't read, the
//! newest first; keeps what it read in RECORDS; and answers the shell at
//! SOCKET (shelf.rs). The models are loaded for what waits to be read and
//! let go once nothing has for a while, so it holds no memory between
//! screenshots.
//!
//! `read` prints each picture's lines in reading order. MODELS is a folder
//! holding the detector (det.onnx), the recognizer (rec.onnx) and the
//! characters the recognizer knows, one a line (characters.txt), as
//! modules/screen-text/models.nix makes it. `SCREEN_TEXT_DEVICE` names the
//! device, the NPU unless it says otherwise, falling back to the CPU, and `SCREEN_TEXT_CACHE` where
//! `OpenVINO` keeps what it compiles, so the next start only reads it.

mod find;
mod picture;
mod read;
mod shelf;
mod watch;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use dictation::daemon::{self, Shared};
use dictation::openvino::Core;

use crate::find::Finder;
use crate::picture::Picture;
use crate::read::Reader;
use crate::shelf::{Shelf, Shot, is_picture, stamp};
use crate::watch::Change;

const USAGE: &str = "usage: screen-text serve SOCKET MODELS FOLDER RECORDS
       screen-text read MODELS PICTURE...";

/// How long the models are kept once nothing waits to be read, so a run
/// of screenshots taken together loads them once.
const UNUSED: Duration = Duration::from_mins(2);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.split_first() {
        Some((ask, rest)) if ask == "read" && rest.len() >= 2 => {
            read_all(Path::new(&rest[0]), &rest[1..])
        }
        Some((ask, [socket, models, folder, records])) if ask == "serve" => serve(
            Path::new(socket),
            Path::new(models),
            Path::new(folder),
            Path::new(records),
        ),
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("screen-text: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Both models, compiled for the device named.
pub struct Models {
    finder: Finder,
    reader: Reader,
    // Last, so it's let go after what it compiled.
    _core: Core,
}

impl Models {
    /// Both models, on the device named, or on the CPU when that one
    /// can't take them, as on a machine with no NPU.
    pub fn load(folder: &Path) -> Result<Self, String> {
        let device = std::env::var("SCREEN_TEXT_DEVICE").unwrap_or_else(|_| "NPU".to_owned());
        match Self::load_on(folder, &device) {
            Err(why) if device != "CPU" => {
                eprintln!("screen-text: reading on the CPU, since the {device} wouldn't: {why}");
                Self::load_on(folder, "CPU")
            }
            loaded => loaded,
        }
    }

    fn load_on(folder: &Path, device: &str) -> Result<Self, String> {
        let cache = std::env::var_os("SCREEN_TEXT_CACHE").map(PathBuf::from);
        let core = Core::new(cache.as_deref())?;
        let finder = Finder::load(&core, &folder.join("det.onnx"), device)?;
        let reader = Reader::load(
            &core,
            &folder.join("rec.onnx"),
            &folder.join("characters.txt"),
            device,
        )?;
        Ok(Self {
            finder,
            reader,
            _core: core,
        })
    }

    /// The lines of text in a picture, in reading order.
    pub fn lines(&mut self, picture: &Picture) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        for place in read::in_order(self.finder.find(picture)?) {
            if let Some(line) = self.reader.read(picture, place)? {
                lines.push(line.text.trim().to_owned());
            }
        }
        Ok(lines)
    }
}

/// Reads every screenshot as it lands, and answers the shell, until the
/// folder's watch ends, for systemd to start it again.
fn serve(socket: &Path, models: &Path, folder: &Path, records: &Path) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|why| format!("making {}: {why}", folder.display()))?;
    // Watched before it's looked through, so nothing saved between is missed.
    let (changes, heard) = mpsc::channel();
    watch::watch(folder, changes)?;
    let shelf = Arc::new(Shared::new(Shelf::read(records)));
    let mut waiting: VecDeque<PathBuf> = shelf.lock().unread(folder).into();
    let listener = daemon::listen(socket)?;
    {
        let shelf = Arc::clone(&shelf);
        std::thread::spawn(move || daemon::answer_all(&shelf, &listener));
    }
    eprintln!("screen-text: answering, {} to read", waiting.len());
    let ended = || "the folder's watch ended".to_owned();
    let mut loaded: Option<Models> = None;
    loop {
        shelf.lock().waiting = waiting.len();
        // What changed comes first, so a screenshot just taken is read
        // before older ones still waiting.
        let change = if !waiting.is_empty() {
            heard.try_recv().ok()
        } else if loaded.is_some() {
            match heard.recv_timeout(UNUSED) {
                Ok(change) => Some(change),
                Err(RecvTimeoutError::Timeout) => {
                    loaded = None;
                    eprintln!("screen-text: the models let go");
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return Err(ended()),
            }
        } else {
            Some(heard.recv().map_err(|_| ended())?)
        };
        match change {
            Some(Change::Came(path)) => {
                if is_picture(&path) {
                    waiting.retain(|held| *held != path);
                    waiting.push_front(path);
                }
                continue;
            }
            Some(Change::Went(path)) => {
                waiting.retain(|held| *held != path);
                shelf.lock().forget(&path);
                eprintln!("screen-text: {} gone", path.display());
                continue;
            }
            None => {}
        }
        let Some(path) = waiting.pop_front() else {
            continue;
        };
        let Some((taken, size)) = stamp(&path).filter(|_| !shelf.lock().holds(&path)) else {
            continue;
        };
        if loaded.is_none() {
            let began = Instant::now();
            loaded = Some(Models::load(models)?);
            eprintln!("screen-text: loaded in {} ms", began.elapsed().as_millis());
        }
        let Some(reading) = loaded.as_mut() else {
            continue;
        };
        let began = Instant::now();
        match Picture::read(&path).and_then(|picture| reading.lines(&picture)) {
            Ok(lines) => {
                eprintln!(
                    "screen-text: {} read, {} lines in {} ms",
                    path.display(),
                    lines.len(),
                    began.elapsed().as_millis()
                );
                shelf.lock().put(Shot {
                    path,
                    taken,
                    size,
                    lines,
                });
            }
            Err(why) => eprintln!("screen-text: {} wasn't read: {why}", path.display()),
        }
    }
}

fn read_all(models: &Path, pictures: &[String]) -> Result<(), String> {
    let began = Instant::now();
    let mut models = Models::load(models)?;
    eprintln!("screen-text: loaded in {} ms", began.elapsed().as_millis());
    for path in pictures {
        let began = Instant::now();
        let picture = Picture::read(Path::new(path))?;
        let lines = models.lines(&picture)?;
        eprintln!(
            "screen-text: {path}, {} lines in {} ms",
            lines.len(),
            began.elapsed().as_millis()
        );
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}
