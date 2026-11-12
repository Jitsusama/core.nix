//! call-transcripts: writes down what's said on Joel's calls, on this
//! machine, with Parakeet (dictation's parakeet.rs) on the NPU taking its
//! turn behind dictation and anything live, since nobody waits on a
//! transcript.
//!
//! Each side is heard apart, Joel's microphone and the call's own sound,
//! so who said what needs no guessing. Each side is cut at its pauses into
//! takes (takes.rs), each take is heard, and the two sides are laid out
//! in the order they spoke (transcript.rs). Parakeet, not dictation's
//! Whisper, hears them: it misheard a fifth fewer words of meetings, and
//! the one thing Whisper did better, writing the names in dictation's
//! words, a call couldn't use, since Whisper told them wrote them back
//! for a take with no speech in it. A take that's only what a speech
//! model writes for a sound is no turn (said.rs).
//!
//! Usage:
//!
//! ```text
//! call-transcripts serve SOCKET MODEL_DIR [SUMMARIZER] CALLS
//!     follow calls as they come and go, writing each one's transcript
//!     and then its summary into a folder of its own in CALLS, and say
//!     them over SOCKET in the shell's daemon interface (book.rs); the
//!     summary's model is asked through SUMMARIZER, a program that
//!     answers as `ask` does (summarizer.rs); with none, a call is only
//!     written down, and a notice says so
//! call-transcripts ask SUMMARIZER_DIR
//!     answer prompts a line each, as JSON, with the language model in
//!     SUMMARIZER_DIR, until the input ends (ask.rs)
//! call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav
//!     print, as Markdown, the transcript of a call's two sides, each a
//!     16 kHz mono WAV starting at the call's start
//! call-transcripts hear MODEL_DIR CLIP.wav...
//!     print what's said in each 16 kHz mono WAV, heard whole as one take,
//!     a line each, which is how a model's mishearings are counted
//! call-transcripts summarize SUMMARIZER_DIR TITLE TRANSCRIPT.md [NAME]
//!     print the summary of the call titled TITLE that a transcript
//!     writes down (summary.rs), the others on it called NAME if given,
//!     or that too little was said to sum up, with the model loaded here,
//!     minding none of serve's manners
//! ```
//!
//! A call is what modules/calls counts as one: a program recording from a
//! microphone for two seconds, over once none has for five (follow.rs).
//! Its sound is kept only in memory until it's heard (hear.rs).
//!
//! It reads two of dictation's settings: `DICTATION_DEVICE`, and
//! `DICTATION_CACHE`, which must be a folder of its own, since a load
//! removes what another build compiled there. The
//! summarizer runs on `SUMMARIZER_DEVICE`, the GPU unless it's set, its
//! compiled model kept in `SUMMARIZER_CACHE`, a folder of its own for
//! the same reason. Its tokenizer needs OpenVINO's tokenizers library,
//! found by `OPENVINO_TOKENIZERS_PATH_GENAI`, as the service sets it.

mod ask;
mod book;
mod follow;
mod hear;
mod meeting;
mod notices;
mod said;
mod summarizer;
mod summary;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

use dictation::openvino::Priority;
use dictation::parakeet::Parakeet;
use dictation::takes::{Cutter, Take};
use dictation::transcript::{Line, Side, markdown};
use dictation::wav;

const USAGE: &str = "usage: call-transcripts serve SOCKET MODEL_DIR [SUMMARIZER] CALLS \
    | ask SUMMARIZER_DIR \
    | transcribe MODEL_DIR MINE.wav THEIRS.wav \
    | hear MODEL_DIR CLIP.wav... \
    | summarize SUMMARIZER_DIR TITLE TRANSCRIPT.md [NAME]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", socket, model, summarizer, calls] => serve(
            Path::new(socket),
            PathBuf::from(model),
            Some(PathBuf::from(summarizer)),
            Path::new(calls),
        ),
        ["serve", socket, model, calls] => serve(
            Path::new(socket),
            PathBuf::from(model),
            None,
            Path::new(calls),
        ),
        ["ask", summarizer] => return ask::serve(Path::new(summarizer)),
        ["summarize", summarizer, title, transcript] => {
            summarize(Path::new(summarizer), title, None, Path::new(transcript))
        }
        ["summarize", summarizer, title, transcript, them] => summarize(
            Path::new(summarizer),
            title,
            Some(them),
            Path::new(transcript),
        ),
        ["transcribe", model, mine, theirs] => {
            transcribe(Path::new(model), Path::new(mine), Path::new(theirs))
        }
        ["hear", model, ref clips @ ..] if !clips.is_empty() => hear_clips(Path::new(model), clips),
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

/// Where Parakeet runs, as dictation's Whisper does.
pub struct Settings {
    pub model: PathBuf,
    pub device: String,
    pub cache: Option<PathBuf>,
}

impl Settings {
    fn read(model: PathBuf) -> Self {
        Self {
            model,
            device: std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned()),
            cache: std::env::var_os("DICTATION_CACHE").map(PathBuf::from),
        }
    }

    /// Whether Parakeet is worth compiling before a call: on the NPU, when
    /// nothing is compiled for this build, so a call's first take waits
    /// seconds rather than a minute.
    pub fn worth_compiling(&self) -> bool {
        Parakeet::worth_compiling(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            dictation::npu(),
        )
    }

    /// Parakeet, at the NPU's background priority.
    pub fn load(&self) -> Result<Parakeet, String> {
        Parakeet::load(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            Priority::Background,
        )
    }
}

fn serve(
    socket: &Path,
    model: PathBuf,
    summarizer: Option<PathBuf>,
    calls: &Path,
) -> Result<(), String> {
    let settings = Settings::read(model);
    let book = Arc::new(book::Shared::new(book::Book::read(calls)));
    let listener = dictation::daemon::listen(socket)?;
    let calling = Arc::new(AtomicBool::new(false));
    let (jobs, heard) = mpsc::channel();
    let (sums, summing) = mpsc::channel();
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || hear::hear(&settings, &book, &heard, &sums, &own_name()));
    }
    {
        let summarizer = summarizer::Summarizer {
            program: summarizer,
            calling: Arc::clone(&calling),
            power: PathBuf::from("/sys/class/power_supply"),
        };
        let (book, again) = (Arc::clone(&book), jobs.clone());
        std::thread::spawn(move || summarizer::run(&summarizer, &book, &summing, &again));
    }
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || dictation::daemon::answer_all(&book, &listener));
    }
    follow::follow(&book, calls, &jobs, &calling)
}

fn summarize(
    summarizer: &Path,
    title: &str,
    them: Option<&str>,
    transcript: &Path,
) -> Result<(), String> {
    let text = std::fs::read_to_string(transcript)
        .map_err(|e| format!("reading {}: {e}", transcript.display()))?;
    if said::too_little(&text) {
        println!("{}", said::TOO_LITTLE);
        return Ok(());
    }
    let mut model = ask::load(summarizer).map_err(|unloaded| match unloaded {
        ask::Unloaded::Short(why) | ask::Unloaded::Failed(why) => why,
    })?;
    let written = summary::summarize(
        &mut |prompt, tokens| {
            let answer = model.answer(prompt, tokens)?;
            // What each pass answered, to check a summary by.
            eprintln!("call-transcripts: asked, answered {answer:?}");
            Ok(answer)
        },
        title,
        them,
        &text,
    )?;
    println!("{written}");
    Ok(())
}

fn transcribe(model: &Path, mine: &Path, theirs: &Path) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned()).load()?;
    let mut lines = Vec::new();
    for (side, file) in [(Side::Mine, mine), (Side::Theirs, theirs)] {
        let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
        for take in takes(&wav::samples(&bytes)?) {
            let (text, took) = speech.transcribe(&take.samples)?;
            eprintln!("call-transcripts: {}", took.describe(&speech.device));
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

fn hear_clips(model: &Path, clips: &[&str]) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned()).load()?;
    for clip in clips {
        let bytes = std::fs::read(clip).map_err(|e| format!("reading {clip}: {e}"))?;
        let (text, took) = speech.transcribe(&wav::samples(&bytes)?)?;
        eprintln!(
            "call-transcripts: {clip}: {}",
            took.describe(&speech.device)
        );
        println!("{}", text.replace('\n', " "));
    }
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
