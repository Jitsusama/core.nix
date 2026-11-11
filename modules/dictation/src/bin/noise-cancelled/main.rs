//! noise-cancelled: each microphone's voice with the sounds around it taken
//! out on the machine, offered as a microphone of its own named for it,
//! "‹microphone›, Noise Cancelled" (the glossary's noise cancelled).
//!
//! ```text
//! noise-cancelled serve MODELS [CLEANER]
//! noise-cancelled clean MODELS [CLEANER] < heard.f32 > cleaned.f32
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
//! loads the model only then (worker.rs).
//!
//! `clean` cleans raw 32-bit float samples, one channel at 48 kHz, from
//! standard input to standard output, as a check hears it.

mod denoise;
mod pipe;
mod worker;

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use denoise::{Cleaner, FRAME, Model};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let done = match arguments.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", models] => pipe::serve(Path::new(models), Cleaner::DeepFilterNet),
        ["serve", models, cleaner] => {
            Cleaner::named(cleaner).and_then(|cleaner| pipe::serve(Path::new(models), cleaner))
        }
        ["clean", models] => clean(Path::new(models), Cleaner::DeepFilterNet),
        ["clean", models, cleaner] => {
            Cleaner::named(cleaner).and_then(|cleaner| clean(Path::new(models), cleaner))
        }
        _ => {
            eprintln!(
                "usage: noise-cancelled serve MODELS [CLEANER]\n       \
                 noise-cancelled clean MODELS [CLEANER] < heard.f32 > cleaned.f32"
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

/// Cleans standard input to standard output, a frame at a time, the last
/// frame filled out with silence.
fn clean(models: &Path, with: Cleaner) -> Result<(), String> {
    let mut heard = Vec::new();
    std::io::stdin()
        .read_to_end(&mut heard)
        .map_err(|why| format!("reading what was heard: {why}"))?;
    let samples: Vec<f32> = heard
        .as_chunks::<4>()
        .0
        .iter()
        .map(|sample| f32::from_le_bytes(*sample))
        .collect();
    let mut denoiser = Model::load(models, with)?.denoiser()?;
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
