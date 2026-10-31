//! Whisper where it runs best: whole on the NPU through `GenAI`'s pipeline
//! (genai.rs) when the NPU is asked for and takes it, or else the encoder
//! where it runs and the decoder on the CPU (whisper.rs). `GenAI`'s pipeline
//! on the CPU took 1.6 s a take against about 1 s for whisper.rs, so only
//! the NPU goes through it.

use std::path::Path;
use std::time::Instant;

use crate::mel::RATE;
use crate::{cache, genai, whisper, words};

/// Whisper, loaded where it runs best.
pub enum Speech {
    /// Every step on the NPU.
    Whole(genai::Whisper),
    /// The encoder on `device`, the decoder on the CPU.
    Split(whisper::Whisper),
}

impl Speech {
    /// Whisper from the export in `dir`, with `words` read before each take,
    /// its compiled models kept in a folder of `cache` for these models and
    /// words, and every other folder there removed once it loads.
    ///
    /// # Errors
    ///
    /// When neither way loads it.
    pub fn load(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        words: &str,
    ) -> Result<Self, String> {
        // This build's own path changes with OpenVINO, GenAI or dictation,
        // any of which makes what's compiled differ.
        let build = std::env::current_exe().unwrap_or_default();
        let parts = [
            dir.as_os_str().as_encoded_bytes(),
            device.as_bytes(),
            words.as_bytes(),
            build.as_os_str().as_encoded_bytes(),
        ];
        let kept = cache.map(|cache| cache::folder(cache, &parts));
        let speech = Self::load_in(dir, device, kept.as_deref(), words)?;
        if let (Some(cache), Some(kept)) = (cache, &kept) {
            cache::forget_others(cache, kept);
        }
        Ok(speech)
    }

    fn load_in(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        words: &str,
    ) -> Result<Self, String> {
        if device == "NPU" {
            // GenAI's compiled models are kept apart from OpenVINO's own,
            // since its pipeline remakes the models before compiling them.
            let cache = cache.map(|cache| cache.join("genai"));
            let sentence = words::sentence(words);
            // A pipeline the NPU makes can still refuse its takes, so one
            // on silence proves it before it's chosen.
            let whole = genai::Whisper::load(dir, device, cache.as_deref(), &sentence)
                .and_then(|mut whole| whole.transcribe(&vec![0.0; RATE]).map(|_| whole));
            match whole {
                Ok(whole) => return Ok(Self::Whole(whole)),
                Err(error) => eprintln!("dictation: {error}, so the decoder runs on the CPU"),
            }
        }
        whisper::Whisper::load(dir, device, cache, words).map(Self::Split)
    }

    /// Runs a take on a second of silence, so the next one finds every
    /// step ready.
    ///
    /// # Errors
    ///
    /// When the take fails.
    pub fn warm(&mut self) -> Result<(), String> {
        match self {
            Self::Whole(whole) => whole.transcribe(&vec![0.0; RATE]).map(|_| ()),
            Self::Split(split) => split.warm(),
        }
    }

    /// What's said in up to 30 s of 16 kHz audio, and a line saying how
    /// long it took and where.
    ///
    /// # Errors
    ///
    /// When the take fails.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<(String, String), String> {
        match self {
            Self::Whole(whole) => {
                let started = Instant::now();
                let text = whole.transcribe(audio)?;
                Ok((
                    text,
                    format!("{} ms on NPU, whole", started.elapsed().as_millis()),
                ))
            }
            Self::Split(split) => {
                let (text, timings) = split.transcribe(audio)?;
                let line = timings.describe(&split.device);
                Ok((text, line))
            }
        }
    }
}
