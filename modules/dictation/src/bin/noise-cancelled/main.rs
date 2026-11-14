//! noise-cancelled: each microphone's voice with the sounds around it taken
//! out on the machine, offered as a microphone of its own named for it,
//! "‹microphone›, Noise Cancelled" (the glossary's noise cancelled).
//!
//! ```text
//! noise-cancelled serve MODELS [CLEANER [CHOSEN [REFUSALS]]]
//! noise-cancelled clean MODELS [CLEANER] < heard.f32 > cleaned.f32
//! noise-cancelled score MODELS < heard.f32
//! noise-cancelled trial SOCKET MODELS RUNNING REFUSALS [CHOSEN]
//! ```
//!
//! CLEANER is deepfilternet, deepfilternet-gentle or rnnoise, and
//! deepfilternet unless named (denoise.rs says what each does). The first
//! two are `DeepFilterNet` 3, run by `OpenVINO` on the processor a 10 ms
//! frame at a time, from the graph in MODELS that
//! modules/noise-cancelled/model.nix exports.
//!
//! `serve` offers a source for each microphone PipeWire has (pipe.rs),
//! which hears its microphone only while something records from it, and
//! loads the model only then (worker.rs). CHOSEN is the file a pick by ear
//! is kept in (chosen.rs): each load cleans as it says, and as CLEANER or
//! RUNNING says until it says a cleaner. After a call on a microphone
//! never picked for where it was made, it offers Choose by Ear beside
//! CHOSEN once, while none of REFUSALS holds (offer.rs).
//!
//! `clean` cleans raw 32-bit float samples, one channel at 48 kHz, from
//! standard input to standard output, as a check hears it.
//!
//! `score` says how good raw samples sound, as above, by the DNSMOS model
//! in MODELS (score.rs): a line of JSON with its voice, background and
//! overall marks out of 5.
//!
//! `trial` answers the launcher's Choose by Ear on SOCKET, or the socket
//! systemd hands it (trial.rs): RUNNING is the cleaner the microphones run,
//! and REFUSALS a JSON file of what refuses a recording while it holds
//! (refusals.rs);
//! a cleaned pick is kept in CHOSEN.

mod chooses;
mod chosen;
mod defaults;
mod denoise;
mod lane;
mod loudness;
mod offer;
mod pipe;
mod refusals;
mod room;
mod score;
mod shadow;
mod spot;
mod trial;
mod ways;
mod worker;

use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::ExitCode;

use chosen::Chosen;
use denoise::{Cleaner, FRAME, Model};
use offer::Offers;
use refusals::Refusal;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let done = match arguments.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", models] => served(models, Cleaner::DeepFilterNet.name(), None, None),
        ["serve", models, cleaner] => served(models, cleaner, None, None),
        ["serve", models, cleaner, chosen] => served(models, cleaner, Some(chosen), None),
        ["serve", models, cleaner, chosen, refusals] => {
            served(models, cleaner, Some(chosen), Some(refusals))
        }
        ["clean", models] => clean(Path::new(models), Cleaner::DeepFilterNet),
        ["clean", models, cleaner] => {
            Cleaner::named(cleaner).and_then(|cleaner| clean(Path::new(models), cleaner))
        }
        ["score", models] => score(Path::new(models)),
        ["trial", socket, models, running, refusals, ref chosen @ ..] if chosen.len() <= 1 => {
            Cleaner::named(running).and_then(|running| {
                trial::serve(
                    Path::new(socket),
                    Path::new(models),
                    Chosen::new(chosen.first().map(|file| (*file).into()), running),
                    Path::new(refusals),
                )
            })
        }
        _ => {
            eprintln!(
                "usage: noise-cancelled serve MODELS [CLEANER [CHOSEN [REFUSALS]]]\n       \
                 noise-cancelled clean MODELS [CLEANER] < heard.f32 > cleaned.f32\n       \
                 noise-cancelled score MODELS < heard.f32\n       \
                 noise-cancelled trial SOCKET MODELS RUNNING REFUSALS [CHOSEN]"
            );
            return ExitCode::from(2);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("noise-cancelled: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Serves each microphone a cleaned twin, as `serve`'s arguments say.
fn served(
    models: &str,
    cleaner: &str,
    chosen: Option<&str>,
    refusals: Option<&str>,
) -> Result<(), String> {
    let chosen = Chosen::new(chosen.map(Into::into), Cleaner::named(cleaner)?);
    let refusals =
        refusals.map_or_else(|| Ok(Vec::new()), |file| Refusal::read(Path::new(file)))?;
    let offers = Offers::new(&chosen, refusals);
    pipe::serve(Path::new(models), chosen, offers)
}

/// The listening socket systemd hands this process, as the first
/// descriptor after the standard three, when it says so (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    let given = unsafe { UnixListener::from_raw_fd(3) };
    // A copy closed on exec, so the recorder and player it runs don't hold
    // the socket open, which the descriptor as handed over isn't.
    given.try_clone().ok()
}

/// The raw samples on standard input.
fn heard() -> Result<Vec<f32>, String> {
    let mut heard = Vec::new();
    std::io::stdin()
        .read_to_end(&mut heard)
        .map_err(|why| format!("reading what was heard: {why}"))?;
    Ok(heard
        .as_chunks::<4>()
        .0
        .iter()
        .map(|sample| f32::from_le_bytes(*sample))
        .collect())
}

/// Says how good standard input sounds.
fn score(models: &Path) -> Result<(), String> {
    let marks = score::Scorer::load(models)?.marks(&heard()?)?;
    println!(
        "{}",
        serde_json::json!({
            "voice": marks.voice,
            "background": marks.background,
            "overall": marks.overall,
        })
    );
    Ok(())
}

/// Cleans standard input to standard output, a frame at a time, the last
/// frame filled out with silence.
fn clean(models: &Path, with: Cleaner) -> Result<(), String> {
    let samples = heard()?;
    // Kept while its denoiser cleans, whose requests run on what it compiled.
    let model = Model::load(models, with)?;
    let mut denoiser = model.denoiser()?;
    let mut cleaned = Vec::with_capacity(samples.len() * 4);
    for chunk in samples.chunks(FRAME) {
        let mut frame = [0.0; FRAME];
        frame[..chunk.len()].copy_from_slice(chunk);
        for sample in denoiser.clean(&frame)? {
            cleaned.extend_from_slice(&sample.to_le_bytes());
        }
    }
    std::io::stdout()
        .write_all(&cleaned)
        .map_err(|why| format!("writing what was cleaned: {why}"))
}
