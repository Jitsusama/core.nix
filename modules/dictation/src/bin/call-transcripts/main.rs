//! call-transcripts: writes down what's said on Joel's calls, on this
//! machine, with Whisper on the NPU taking its turn behind dictation and
//! anything live, since nobody waits on a transcript.
//!
//! Each side is heard apart, Joel's microphone and the call's own sound,
//! so who said what needs no guessing. Each side is cut at its pauses into
//! takes (takes.rs), each take is heard with Joel's words, and the two
//! sides are laid out in the order they spoke (transcript.rs).
//!
//! Usage:
//!
//! ```text
//! call-transcripts serve SOCKET MODEL_DIR CALLS
//!     follow calls as they come and go, writing each one's transcript
//!     into a folder of its own in CALLS, and say them over SOCKET in the
//!     shell's daemon interface (book.rs)
//! call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav
//!     print, as Markdown, the transcript of a call's two sides, each a
//!     16 kHz mono WAV starting at the call's start
//! ```
//!
//! A call is what modules/calls counts as one: a program recording from a
//! microphone for two seconds, over once none has for five (follow.rs).
//! Its sound is kept only in memory until it's heard (hear.rs).
//!
//! It reads the same three settings dictation does: `DICTATION_DEVICE`,
//! `DICTATION_CACHE`, which must be a folder of its own, since a load
//! removes what another build compiled there, and `DICTATION_WORDS`.

mod book;
mod follow;
mod hear;
mod meeting;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, mpsc};

use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::takes::{Cutter, Take};
use dictation::transcript::{Line, Side, markdown};
use dictation::wav;

const USAGE: &str = "usage: call-transcripts serve SOCKET MODEL_DIR CALLS \
    | transcribe MODEL_DIR MINE.wav THEIRS.wav";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", socket, model, calls] => {
            serve(Path::new(socket), PathBuf::from(model), Path::new(calls))
        }
        ["transcribe", model, mine, theirs] => {
            transcribe(Path::new(model), Path::new(mine), Path::new(theirs))
        }
        _ => Err(format!("{USAGE} (got {args:?})")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("call-transcripts: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Where Whisper runs and what it's told, as dictation's are.
pub struct Settings {
    pub model: PathBuf,
    pub device: String,
    pub cache: Option<PathBuf>,
    pub words: String,
}

impl Settings {
    fn read(model: PathBuf) -> Result<Self, String> {
        let words = match std::env::var_os("DICTATION_WORDS") {
            Some(file) => std::fs::read_to_string(&file)
                .map_err(|e| format!("reading {}: {e}", Path::new(&file).display()))?,
            None => String::new(),
        };
        Ok(Self {
            model,
            device: std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned()),
            cache: std::env::var_os("DICTATION_CACHE").map(PathBuf::from),
            words,
        })
    }

    /// Whisper, at the NPU's background priority.
    pub fn load(&self) -> Result<Speech, String> {
        Speech::load(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            &self.words,
            Priority::Background,
        )
    }
}

fn serve(socket: &Path, model: PathBuf, calls: &Path) -> Result<(), String> {
    let settings = Settings::read(model)?;
    let book = Arc::new(book::Shared::new(book::Book::read(calls)));
    let listener = book::listen(socket)?;
    let (jobs, heard) = mpsc::channel();
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || hear::hear(&settings, &book, &heard, &own_name()));
    }
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || book::answer_all(&book, &listener));
    }
    follow::follow(&book, calls, &jobs)
}

fn transcribe(model: &Path, mine: &Path, theirs: &Path) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned())?.load()?;
    let mut lines = Vec::new();
    for (side, file) in [(Side::Mine, mine), (Side::Theirs, theirs)] {
        let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
        for take in takes(&wav::samples(&bytes)?) {
            let (text, took) = speech.transcribe(&take.samples)?;
            eprintln!("call-transcripts: {took}");
            let text = text.trim();
            if !text.is_empty() {
                lines.push(Line {
                    side,
                    start: take.start,
                    end: take.start + take.samples.len(),
                    text: text.to_owned(),
                });
            }
        }
    }
    print!("{}", markdown(&lines, &own_name()));
    Ok(())
}

/// A side's sound cut into takes.
fn takes(sound: &[f32]) -> Vec<Take> {
    let mut cutter = Cutter::new();
    let mut takes = cutter.feed(sound);
    takes.extend(cutter.finish());
    takes
}

/// The account's own name, its first word as the system knows it, for
/// Joel's lines; or "Me" where it has none.
fn own_name() -> String {
    let user = std::env::var("USER").unwrap_or_default();
    std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|passwd| {
            passwd.lines().find_map(|entry| {
                let fields: Vec<&str> = entry.split(':').collect();
                (fields.first() == Some(&user.as_str()))
                    .then(|| fields.get(4)?.split([' ', ',']).next().map(str::to_owned))
                    .flatten()
            })
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Me".to_owned())
}
