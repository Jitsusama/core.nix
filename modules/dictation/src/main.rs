//! dictation: types what's said into the focused window.
//!
//! A chord runs `dictation hold`: the first press starts listening to the
//! microphone, and the next stops, works out what was said with Whisper and
//! types it where the focus is. Held, the chord listens until it's let go,
//! which dictation-chord hears. Listening also stops at 30 s, Whisper's
//! window. The service (service.rs) holds the models between presses,
//! Whisper's encoder on the NPU, so 11 s of speech is worked out in under
//! a second, even while a kernel builds. Nothing is written to disk: the
//! audio is kept in memory until it's typed.
//!
//! Usage:
//!
//! ```text
//! dictation serve MODEL_DIR      the service, on the socket it's given
//! dictation toggle               start listening, or stop and type
//! dictation hold                 the same, and stop when the chord held
//!                                is let go
//! dictation watch                print each change of state as it comes
//! dictation transcribe MODEL_DIR FILE.wav
//!                                print what's said in a 16 kHz mono WAV
//! ```
//!
//! `DICTATION_DEVICE` names where the encoder runs (NPU, falling back to the
//! CPU), `DICTATION_CACHE` where OpenVINO keeps what it compiles, and
//! `DICTATION_WORDS` a file of words Whisper should know, one a line.

mod genai;
mod mel;
mod openvino;
mod service;
mod speech;
mod vocab;
mod wav;
mod whisper;
mod words;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use speech::Speech;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", model] => {
            words().and_then(|words| service::serve(Path::new(model), device(), cache(), words))
        }
        ["toggle"] => service::ask("toggle"),
        ["hold"] => service::ask("hold"),
        ["watch"] => service::ask("watch"),
        ["transcribe", model, file] => transcribe(Path::new(model), Path::new(file)),
        _ => Err(format!("{USAGE} (got {args:?})")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dictation: {error}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str =
    "usage: dictation serve MODEL_DIR | toggle | hold | watch | transcribe MODEL_DIR FILE.wav";

fn device() -> String {
    std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned())
}

fn cache() -> Option<PathBuf> {
    std::env::var_os("DICTATION_CACHE").map(PathBuf::from)
}

/// The words file's text, or nothing when no file is named.
fn words() -> Result<String, String> {
    match std::env::var_os("DICTATION_WORDS") {
        Some(file) => std::fs::read_to_string(&file)
            .map_err(|e| format!("reading {}: {e}", Path::new(&file).display())),
        None => Ok(String::new()),
    }
}

fn transcribe(model: &Path, file: &Path) -> Result<(), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
    let audio = wav::samples(&bytes)?;
    let mut speech = Speech::load(model, &device(), cache().as_deref(), &words()?)?;
    let (text, took) = speech.transcribe(&audio)?;
    println!("{text}");
    eprintln!("dictation: {took}");
    Ok(())
}
