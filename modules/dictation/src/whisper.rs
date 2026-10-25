//! Whisper through OpenVINO: the encoder where it runs fastest, the NPU
//! when there is one, and greedy decoding on the CPU, in English, without
//! timestamps.
//!
//! The decoder keeps what it has read as state of its own, so each step
//! gives it only the token before, and a fresh request starts afresh. It
//! won't compile for the NPU, whose shapes are fixed while the decoder's
//! grow a token at a time; on the GPU it was no faster than the CPU.
//!
//! Given words, the decoder reads them first as what came before
//! (words.rs), so it writes them as they're spelled there.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::mel::{FRAMES, MELS, Spectrogram};
use crate::openvino::{Compiled, Core, Input, Request, Result};
use crate::vocab::Vocab;
use crate::words::{self, Encoder};

const END: usize = 50_257;
const PROMPT: [i64; 4] = [
    50_258, // <|startoftranscript|>
    50_259, // <|en|>
    50_359, // <|transcribe|>
    50_363, // <|notimestamps|>
];
// As many tokens as Whisper writes for its 30 s window.
const MOST: usize = 224;
// As many tokens as the decoder holds, what it's given and what it writes.
const CONTEXT: usize = 448;
const VOCAB: usize = 51_865;

/// Whisper, loaded and compiled.
pub struct Whisper {
    spectrogram: Spectrogram,
    vocab: Vocab,
    encoder: Request,
    decoder: Compiled,
    start: Vec<i64>,
    _core: Core,
    /// Where the encoder runs.
    pub device: String,
}

/// How long a transcription's steps took.
pub struct Timings {
    pub features: Duration,
    pub encoder: Duration,
    pub decoder: Duration,
    pub tokens: usize,
}

impl Timings {
    /// The timings in a line, with where the encoder ran.
    #[must_use]
    pub fn describe(&self, device: &str) -> String {
        format!(
            "features {} ms, encoder {} ms on {device}, decoder {} ms for {} tokens",
            self.features.as_millis(),
            self.encoder.as_millis(),
            self.decoder.as_millis(),
            self.tokens
        )
    }
}

impl Whisper {
    /// Whisper from an OpenVINO export in `dir`, the encoder on `device`,
    /// or on the CPU when that device can't take it, leaning toward the
    /// words file's words, one a line, when there are any.
    ///
    /// # Errors
    ///
    /// When a file is missing or neither device takes a model.
    pub fn load(dir: &Path, device: &str, cache: Option<&Path>, words: &str) -> Result<Self> {
        let vocab = std::fs::read_to_string(dir.join("vocab.json"))
            .map_err(|e| format!("reading vocab.json: {e}"))
            .and_then(|json| Vocab::parse(&json))?;
        if vocab.count() <= END {
            return Err(format!(
                "vocab.json has {} tokens, too few for Whisper",
                vocab.count()
            ));
        }
        let sentence = words::sentence(words);
        let before = if sentence.is_empty() {
            Vec::new()
        } else {
            let merges = std::fs::read_to_string(dir.join("merges.txt"))
                .map_err(|e| format!("reading merges.txt: {e}"))?;
            Encoder::new(&vocab, &merges)?.encode(&sentence)?
        };
        let task: Vec<usize> = PROMPT.iter().map(|&id| usize_of(id)).collect();
        let start = words::start(&before, &task)
            .into_iter()
            .map(i64_of)
            .collect();
        let core = Core::new(cache)?;
        let mut encoder = core.read(&dir.join("openvino_encoder_model.xml"))?;
        encoder.fix_input(&[1, i64_of(MELS), i64_of(FRAMES)])?;
        let (encoder, device) = match core.compile(&encoder, device) {
            Ok(compiled) => (compiled, device.to_owned()),
            Err(error) if device != "CPU" => {
                eprintln!("dictation: the encoder runs on the CPU: {error}");
                (core.compile(&encoder, "CPU")?, "CPU".to_owned())
            }
            Err(error) => return Err(error),
        };
        let decoder = core.compile(&core.read(&dir.join("openvino_decoder_model.xml"))?, "CPU")?;
        Ok(Self {
            spectrogram: Spectrogram::new(),
            vocab,
            encoder: encoder.request()?,
            decoder,
            start,
            _core: core,
            device,
        })
    }

    /// What's said in up to 30 s of 16 kHz audio, and how long it took.
    ///
    /// # Errors
    ///
    /// When a run of either model fails.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<(String, Timings)> {
        let start = Instant::now();
        let features = self.spectrogram.features(audio);
        let after_features = Instant::now();
        let (shape, hidden) = self.encoder.run(
            &[Input::F32(
                "input_features",
                &[1, i64_of(MELS), i64_of(FRAMES)],
                &features,
            )],
            "last_hidden_state",
            None,
        )?;
        let after_encoder = Instant::now();

        let mut decoder = self.decoder.request()?;
        let mut tokens = Vec::new();
        let mut ids = self.start.clone();
        for _ in 0..MOST.min(CONTEXT - self.start.len()) {
            let (_, logits) = decoder.run(
                &[
                    Input::I64("input_ids", &[1, i64_of(ids.len())], &ids),
                    Input::F32("encoder_hidden_states", &shape, &hidden),
                    Input::I32("beam_idx", &[1], &[0]),
                ],
                "logits",
                Some(VOCAB),
            )?;
            let next = most_likely(&logits);
            if next == END {
                break;
            }
            tokens.push(next);
            ids = vec![i64_of(next)];
        }
        let after_decoder = Instant::now();
        Ok((
            self.vocab.text(&tokens, END),
            Timings {
                features: after_features - start,
                encoder: after_encoder - after_features,
                decoder: after_decoder - after_encoder,
                tokens: tokens.len(),
            },
        ))
    }
}

fn i64_of(n: usize) -> i64 {
    i64::try_from(n).expect("a size Whisper uses fits in 64 bits")
}

fn usize_of(n: i64) -> usize {
    usize::try_from(n).expect("Whisper's tokens are positive")
}

/// The likeliest token that's text or the end, leaving out the special
/// tokens after the end, which a transcription without timestamps never
/// wants.
fn most_likely(logits: &[f32]) -> usize {
    logits
        .iter()
        .take(END + 1)
        .enumerate()
        .fold((0, f32::NEG_INFINITY), |best, (id, &score)| {
            if score > best.1 { (id, score) } else { best }
        })
        .0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_likeliest_text_wins() {
        let mut logits = vec![0.0; VOCAB];
        logits[7] = 3.0;
        logits[9] = 2.0;
        assert_eq!(most_likely(&logits), 7);
    }

    #[test]
    fn a_special_after_the_end_never_wins() {
        let mut logits = vec![0.0; VOCAB];
        logits[50_300] = 9.0;
        logits[END] = 1.0;
        assert_eq!(most_likely(&logits), END);
    }
}
