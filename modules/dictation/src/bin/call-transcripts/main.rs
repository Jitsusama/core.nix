//! call-transcripts: writes down what's said on Joel's calls, on this
//! machine, with Whisper on the NPU taking its turn behind dictation and
//! anything live, since nobody waits on a transcript.
//!
//! Each side is heard apart, Joel's microphone and the call's own sound,
//! so who said what needs no guessing. Each side is cut at its pauses into
//! takes (takes.rs), each take is heard, and the two sides are laid out
//! in the order they spoke (transcript.rs). Whisper isn't told the words
//! dictation is: given a take with no speech in it, it writes back what
//! it was told, so a call came out as those words over and over, and its
//! summary as a call about them. A take that's only what Whisper writes
//! for a sound is no turn (said.rs).
//!
//! Usage:
//!
//! ```text
//! call-transcripts serve SOCKET MODEL_DIR SUMMARIZER_DIR CALLS
//!     follow calls as they come and go, writing each one's transcript
//!     and then its summary into a folder of its own in CALLS, and say
//!     them over SOCKET in the shell's daemon interface (book.rs)
//! call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav
//!     print, as Markdown, the transcript of a call's two sides, each a
//!     16 kHz mono WAV starting at the call's start
//! call-transcripts summarize SUMMARIZER_DIR TITLE TRANSCRIPT.md
//!     print the summary of the call titled TITLE that a transcript
//!     writes down (summary.rs), or that too little was said to sum up,
//!     failing on one that names what the call never did (said.rs)
//! ```
//!
//! A call is what modules/calls counts as one: a program recording from a
//! microphone for two seconds, over once none has for five (follow.rs).
//! Its sound is kept only in memory until it's heard (hear.rs).
//!
//! It reads two of dictation's settings: `DICTATION_DEVICE`, and
//! `DICTATION_CACHE`, which must be a folder of its own, since a load
//! removes what another build compiled there. The
//! summarizer runs on the same device, its compiled model kept in
//! `SUMMARIZER_CACHE`, a folder of its own for the same reason. Its
//! tokenizer needs OpenVINO's tokenizers library, found by
//! `OPENVINO_TOKENIZERS_PATH_GENAI`, as the service sets it.

mod book;
mod follow;
mod hear;
mod meeting;
mod notices;
mod said;
mod summary;

use summary::Summed;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, mpsc};

use dictation::language::LanguageModel;
use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::takes::{Cutter, Take};
use dictation::transcript::{Line, Side, markdown};
use dictation::wav;

const USAGE: &str = "usage: call-transcripts serve SOCKET MODEL_DIR SUMMARIZER_DIR CALLS \
    | transcribe MODEL_DIR MINE.wav THEIRS.wav \
    | summarize SUMMARIZER_DIR TITLE TRANSCRIPT.md";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", socket, model, summarizer, calls] => serve(
            Path::new(socket),
            PathBuf::from(model),
            PathBuf::from(summarizer),
            Path::new(calls),
        ),
        ["summarize", summarizer, title, transcript] => {
            summarize(PathBuf::from(summarizer), title, Path::new(transcript))
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

/// Where Whisper runs, as dictation's does, and where the summarizer is.
pub struct Settings {
    pub model: PathBuf,
    pub device: String,
    pub cache: Option<PathBuf>,
    pub summarizer: PathBuf,
    pub summarizer_cache: Option<PathBuf>,
}

impl Settings {
    fn read(model: PathBuf, summarizer: PathBuf) -> Self {
        Self {
            model,
            device: std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned()),
            cache: std::env::var_os("DICTATION_CACHE").map(PathBuf::from),
            summarizer,
            summarizer_cache: std::env::var_os("SUMMARIZER_CACHE").map(PathBuf::from),
        }
    }

    /// The summarizer, at the NPU's background priority.
    pub fn load_summarizer(&self) -> Result<LanguageModel, String> {
        LanguageModel::load(
            &self.summarizer,
            &self.device,
            self.summarizer_cache.as_deref(),
            Priority::Background,
        )
    }

    /// Whisper, at the NPU's background priority, told no words.
    pub fn load(&self) -> Result<Speech, String> {
        Speech::load(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            "",
            Priority::Background,
        )
    }
}

fn serve(socket: &Path, model: PathBuf, summarizer: PathBuf, calls: &Path) -> Result<(), String> {
    let settings = Settings::read(model, summarizer);
    let book = Arc::new(book::Shared::new(book::Book::read(calls)));
    let listener = dictation::daemon::listen(socket)?;
    let (jobs, heard) = mpsc::channel();
    {
        let (book, again) = (Arc::clone(&book), jobs.clone());
        std::thread::spawn(move || hear::hear(&settings, &book, &heard, &again, &own_name()));
    }
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || dictation::daemon::answer_all(&book, &listener));
    }
    follow::follow(&book, calls, &jobs)
}

fn summarize(summarizer: PathBuf, title: &str, transcript: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(transcript)
        .map_err(|e| format!("reading {}: {e}", transcript.display()))?;
    if said::too_little(&text) {
        println!("{}", said::TOO_LITTLE);
        return Ok(());
    }
    let mut model = Settings::read(PathBuf::new(), summarizer).load_summarizer()?;
    match summary::grounded(&mut model, title, &text)? {
        Summed::Kept(written) => {
            println!("{written}");
            Ok(())
        }
        Summed::Strayed(strays) => Err(format!(
            "each summary named what the call never did: {}",
            strays.join(", ")
        )),
    }
}

fn transcribe(model: &Path, mine: &Path, theirs: &Path) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned(), PathBuf::new()).load()?;
    let mut lines = Vec::new();
    for (side, file) in [(Side::Mine, mine), (Side::Theirs, theirs)] {
        let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
        for take in takes(&wav::samples(&bytes)?) {
            let (text, took) = speech.transcribe(&take.samples)?;
            eprintln!("call-transcripts: {took}");
            if let Some(text) = said::spoken(&text) {
                lines.push(Line {
                    side,
                    start: take.start,
                    end: take.start + take.samples.len(),
                    text,
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
