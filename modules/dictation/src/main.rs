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
mod tidying;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::tidy::{self, Tidier};
use dictation::wav;
use terminal_kit::{Exit, Failure, Help};

fn help() -> Help {
    Help::new("dictation", "types what's said into the focused window")
        .example("dictation toggle", "starts listening, or stops and types")
        .example("dictation watch", "each change of state, as it comes")
        .subcommand(asks(
            "toggle",
            "starts listening, or stops and types what was said",
            "listens, or types what it heard",
        ))
        .subcommand(asks(
            "hold",
            "the same, stopping when the held chord is let go",
            "listens until the chord is let go",
        ))
        .subcommand(asks(
            "watch",
            "says each change of state as it comes",
            "a line a change, until stopped",
        ))
        .subcommand(
            Help::new("transcribe", "says what's said in a 16 kHz mono WAV")
                .argument("<model-dir>", "the speech model's folder")
                .argument("<file>", "the WAV to read")
                .example("dictation transcribe ~/m said.wav", "what said.wav says")
                .example(
                    "dictation transcribe ~/m said.wav --json",
                    "the same, as dictation.transcript/1",
                )
                .exit(Exit::Failed, "the model failed")
                .exit(Exit::BadInput, "the file isn't a WAV it can read")
                .exit(Exit::NotFound, "there's no such file"),
        )
        .subcommand(
            Help::new("tidy", "tidies each line read, as the service would")
                .argument("<model-dir>", "the tidying model's folder")
                .example("echo um so yes | dictation tidy ~/t", "so yes, tidied")
                .exit(Exit::Failed, "the model failed"),
        )
        .subcommand(
            Help::new("serve", "the service, on the socket systemd gives it")
                .argument("<model-dir>", "the speech model's folder")
                .example("dictation serve ~/m", "what its unit runs")
                .exit(Exit::Failed, "the models or the microphone failed"),
        )
        .exit(Exit::Failed, "the models or the service failed")
        .exit(Exit::BadInput, "the file isn't a WAV it can read")
        .exit(Exit::NotFound, "there's no such file")
        .exit(Exit::Unavailable, "dictation's service isn't answering")
}

/// A subcommand that asks the running service, copying its answer.
fn asks(name: &str, about: &str, does: &str) -> Help {
    Help::new(name, about)
        .example(&format!("dictation {name}"), does)
        .exit(Exit::Failed, "the service's answer broke off")
        .exit(Exit::Unavailable, "dictation's service isn't answering")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let failed = |error: String| Failure::new(Exit::Failed, error);
        match context.args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
            ["serve", model] => words()
                .and_then(|words| {
                    service::serve(Path::new(model), device(), cache(), words, tidying())
                })
                .map_err(failed),
            // The chord's own path: the service's answer is copied as it
            // comes, and nothing else is said.
            ["toggle"] => service::ask("toggle"),
            ["hold"] => service::ask("hold"),
            ["watch"] => service::ask("watch"),
            ["transcribe", model, file] => transcribe(context, Path::new(model), Path::new(file)),
            ["tidy", model] => tidy(&tidying::Tidying {
                model: PathBuf::from(model),
                ..tidying_on()
            })
            .map_err(failed),
            // The kit has refused a word no subcommand answers to, so only
            // a subcommand given too few or too many words is left.
            _ => Err(Failure::new(Exit::Usage, "those aren't the words it takes")
                .try_this(format!(
                    "dictation {} --help",
                    context.args.first().map_or("", String::as_str)
                ))),
        }
    })
}

fn device() -> String {
    std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned())
}

fn cache() -> Option<PathBuf> {
    std::env::var_os("DICTATION_CACHE").map(PathBuf::from)
}

/// The tidying model the environment names, if it names one.
fn tidying() -> Option<tidying::Tidying> {
    let model = PathBuf::from(std::env::var_os("DICTATION_TIDY")?);
    Some(tidying::Tidying {
        model,
        ..tidying_on()
    })
}

/// Where the environment says to tidy, with no model named yet.
fn tidying_on() -> tidying::Tidying {
    tidying::Tidying {
        model: PathBuf::new(),
        device: std::env::var("DICTATION_TIDY_DEVICE").unwrap_or_else(|_| "GPU".to_owned()),
        cache: std::env::var_os("DICTATION_TIDY_CACHE").map(PathBuf::from),
    }
}

/// Tidies each line of standard input as the service would, printing the
/// tidy, or the take where the tidy may not be typed, and how long each
/// took. Nobody waits here, so the model has a minute, not the service's
/// 2 s, and a slow device can still be measured.
fn tidy(tidying: &tidying::Tidying) -> Result<(), String> {
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

/// Says what's said in `file`, or answers `dictation.transcript/1` with
/// `--json`; how long it took goes to standard error either way.
fn transcribe(context: &terminal_kit::Context, model: &Path, file: &Path) -> Result<(), Failure> {
    let bytes = std::fs::read(file).map_err(|error| {
        let exit = if error.kind() == std::io::ErrorKind::NotFound {
            Exit::NotFound
        } else {
            Exit::Failed
        };
        Failure::new(exit, format!("can't read {}", file.display())).because(error.to_string())
    })?;
    let audio = wav::samples(&bytes).map_err(|why| {
        Failure::new(Exit::BadInput, format!("{} isn't a WAV it can read", file.display()))
            .because(why)
    })?;
    let failed = |error: String| Failure::new(Exit::Failed, error);
    let mut speech = Speech::load(
        model,
        &device(),
        cache().as_deref(),
        &words().map_err(failed)?,
        Priority::Waited,
    )
    .map_err(failed)?;
    let (text, took) = speech.transcribe(&audio).map_err(failed)?;
    let said = if context.format().is_json() {
        context.answer(&serde_json::json!({"schema": "dictation.transcript/1", "text": text}))
    } else {
        context.say(&format!("{text}\n"))
    };
    said.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })?;
    eprintln!("dictation: {took}");
    Ok(())
}
