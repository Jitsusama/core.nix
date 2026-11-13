//! dictation: types what's said into the focused window.
//!
//! A chord runs `dictation hold`: the first press starts listening to the
//! microphone, and the next stops, works out what was said with Whisper and
//! types it where the focus is. Held, the chord listens until it's let go,
//! which dictation-chord hears. Listening also stops at 30 s, Whisper's
//! window. The service (service.rs) holds the models between presses,
//! Whisper's encoder on the NPU, so 11 s of speech is worked out in under
//! a second, even while a kernel builds. Nothing is written to disk: the
//! audio is kept in memory until it's typed. What's typed fits the window
//! it lands in (mode.rs): a terminal gets a command, with no capital and no
//! full stop, and anything else the sentence as heard, or, where a tidying
//! model is given, as it tidies it in under 2 s (tidy.rs).
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
//! dictation tidy MODEL_DIR       print each take read, a line each, as
//!                                the tidying model tidies it
//! ```
//!
//! `DICTATION_DEVICE` names where the encoder runs (NPU, falling back to the
//! CPU), `DICTATION_CACHE` where OpenVINO keeps what it compiles, and
//! `DICTATION_WORDS` a file of words Whisper should know, one a line.
//! `DICTATION_TIDY` names the tidying model's folder, which the service
//! tidies prose with only when it's set, `DICTATION_TIDY_DEVICE` where it
//! runs (the GPU unless it's set), and `DICTATION_TIDY_CACHE` where what
//! it compiles is kept, a folder of its own, since each load keeps only
//! its own folder in the cache it's given.

mod kept;
mod mode;
mod service;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::tidy::{self, Tidier};
use dictation::wav;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", model] => words().and_then(|words| {
            service::serve(Path::new(model), device(), cache(), words, tidying())
        }),
        ["toggle"] => service::ask("toggle"),
        ["hold"] => service::ask("hold"),
        ["watch"] => service::ask("watch"),
        ["transcribe", model, file] => transcribe(Path::new(model), Path::new(file)),
        ["tidy", model] => tidy(&service::Tidying {
            model: PathBuf::from(model),
            ..tidying_on()
        }),
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

const USAGE: &str = "usage: dictation serve MODEL_DIR | toggle | hold | watch \
                     | transcribe MODEL_DIR FILE.wav | tidy MODEL_DIR";

fn device() -> String {
    std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned())
}

fn cache() -> Option<PathBuf> {
    std::env::var_os("DICTATION_CACHE").map(PathBuf::from)
}

/// The tidying model the environment names, if it names one.
fn tidying() -> Option<service::Tidying> {
    let model = PathBuf::from(std::env::var_os("DICTATION_TIDY")?);
    Some(service::Tidying {
        model,
        ..tidying_on()
    })
}

/// Where the environment says to tidy, with no model named yet.
fn tidying_on() -> service::Tidying {
    service::Tidying {
        model: PathBuf::new(),
        device: std::env::var("DICTATION_TIDY_DEVICE").unwrap_or_else(|_| "GPU".to_owned()),
        cache: std::env::var_os("DICTATION_TIDY_CACHE").map(PathBuf::from),
    }
}

/// Tidies each line of standard input as the service would, printing the
/// tidy, or the take where the tidy may not be typed, and how long each
/// took. Nobody waits here, so the model has a minute, not the service's
/// 2 s, and a slow device can still be measured.
fn tidy(tidying: &service::Tidying) -> Result<(), String> {
    let started = Instant::now();
    let mut tidier = Tidier::load(
        &tidying.model,
        &tidying.device,
        tidying.cache.as_deref(),
        Priority::Waited,
    )?;
    eprintln!("dictation: loaded in {} ms", started.elapsed().as_millis());
    let started = Instant::now();
    tidier.warm()?;
    eprintln!("dictation: warmed in {} ms", started.elapsed().as_millis());
    for take in std::io::stdin().lock().lines() {
        let take = take.map_err(|e| format!("reading a take: {e}"))?;
        let started = Instant::now();
        let written = tidier.write(&take, started + Duration::from_mins(1));
        let took = started.elapsed().as_millis();
        match written.map(|written| (tidy::tidied(&take, &written), written)) {
            Ok((Some(typed), _)) => {
                eprintln!("dictation: tidied in {took} ms");
                println!("{typed}");
            }
            Ok((None, written)) => {
                eprintln!("dictation: typed as heard after {took} ms; the model wrote: {written}");
                println!("{}", take.trim());
            }
            Err(why) => {
                eprintln!("dictation: typed as heard after {took} ms: {why}");
                println!("{}", take.trim());
            }
        }
    }
    Ok(())
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
    let mut speech = Speech::load(
        model,
        &device(),
        cache().as_deref(),
        &words()?,
        Priority::Waited,
    )?;
    let (text, took) = speech.transcribe(&audio)?;
    println!("{text}");
    eprintln!("dictation: {took}");
    Ok(())
}
