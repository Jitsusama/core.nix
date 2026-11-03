//! Parakeet TDT 0.6B v3, NVIDIA's speech model, through OpenVINO, which
//! call-transcripts hears with: on 200 lines of AMI's meetings it misheard
//! 16.5% of words against Whisper small's 21.0%, and took no longer.
//! Dictation keeps Whisper, whose words file spells Joel's names.
//!
//! Four models make it: the spectrogram, the encoder, the decoder (the
//! prediction network) and the joint network. The encoder, the one that
//! costs, runs on the NPU when there is one, at the priority it's given;
//! the rest run on the CPU on a thread each, since the decoder and the
//! joint run once a token or a frame on so little that a hop to an
//! accelerator, or across every core, costs more than the work.
//!
//! The export reads 15 s at a time, so a longer take is heard as windows
//! of 15 s, the decoder's state carried from one to the next so a window's
//! first word follows the last one's. Decoding is greedy, as TDT's is:
//! at each frame the joint names a token and how many frames to move on,
//! a blank writes nothing, and a frame writes at most `MOST_PER_FRAME`.
//!
//! The model knows 25 languages, and now and then heard English as
//! another ("Look at all this stuff man" once came out as "Legal, ne?"),
//! so it's held to English: no token with a letter outside ASCII, and no
//! special token, is ever chosen.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::cache;
use crate::openvino::{Core, Input, Priority, Request, Result};

/// The rate it hears at.
pub const RATE: usize = 16_000;
/// The samples the export reads at once: 15 s.
const WINDOW: usize = 15 * RATE;
/// The spectrogram's mel bands, and its frames for a window.
const MELS: i64 = 128;
const MEL_FRAMES: i64 = 1501;
/// The encoder's width, and the decoder's: its output and each layer of
/// its two layers' state.
const ENCODED: usize = 1024;
const PREDICTED: usize = 640;
const LAYERS: usize = 2;
/// How many frames each of the joint's durations moves on, in its order.
const DURATIONS: [usize; 5] = [0, 1, 2, 3, 4];
/// The most tokens one frame writes before the next is read, so a frame
/// that keeps naming a token and no move can't write forever.
const MOST_PER_FRAME: usize = 10;
/// The piece that starts a word.
const SPACE: char = '\u{2581}';

/// Parakeet, loaded and compiled.
pub struct Parakeet {
    mel: Request,
    encoder: Request,
    decoder: Request,
    joint: Request,
    vocab: Vocab,
    _core: Core,
    /// Where the encoder runs.
    pub device: String,
}

/// How long hearing a take's steps took.
pub struct Timings {
    pub mel: Duration,
    pub encoder: Duration,
    pub decoder: Duration,
    pub tokens: usize,
}

impl Timings {
    /// The timings in a line, with where the encoder ran.
    #[must_use]
    pub fn describe(&self, device: &str) -> String {
        format!(
            "mel {} ms, encoder {} ms on {device}, decoder {} ms for {} tokens",
            self.mel.as_millis(),
            self.encoder.as_millis(),
            self.decoder.as_millis(),
            self.tokens
        )
    }
}

/// The decoder's output and its state, which it's given back each step.
struct Predicted {
    out: Vec<f32>,
    h: Vec<f32>,
    c: Vec<f32>,
}

impl Parakeet {
    /// Parakeet from the export in `dir`, the encoder on `device`, or on
    /// the CPU when that device can't take it, at `priority` on the NPU.
    /// Its compiled models are kept in a folder of `cache` for this model
    /// and build, and every other folder there removed once it loads, so
    /// the program using it needs a cache of its own.
    ///
    /// # Errors
    ///
    /// When a file is missing or neither device takes the encoder.
    pub fn load(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        priority: Priority,
    ) -> Result<Self> {
        // This build's own path changes with OpenVINO or this crate, either
        // of which makes what's compiled differ.
        let build = std::env::current_exe().unwrap_or_default();
        let parts = [
            dir.as_os_str().as_encoded_bytes(),
            device.as_bytes(),
            build.as_os_str().as_encoded_bytes(),
        ];
        let kept = cache.map(|cache| cache::folder(cache, &parts));
        let parakeet = Self::load_in(dir, device, kept.as_deref(), priority)?;
        if let (Some(cache), Some(kept)) = (cache, &kept) {
            cache::forget_others(cache, kept);
        }
        Ok(parakeet)
    }

    fn load_in(dir: &Path, device: &str, cache: Option<&Path>, priority: Priority) -> Result<Self> {
        let vocab = std::fs::read_to_string(dir.join("parakeet_v3_vocab.json"))
            .map_err(|e| format!("reading parakeet_v3_vocab.json: {e}"))
            .and_then(|json| Vocab::parse(&json))?;
        let core = Core::new(cache)?;
        let part = |name: &str| core.read(&dir.join(format!("parakeet_{name}.xml")));
        let mel = core.compile(&part("melspectogram")?, "CPU", priority)?;
        let encoder = part("encoder")?;
        let (encoder, device) = match core.compile(&encoder, device, priority) {
            Ok(compiled) => (compiled, device.to_owned()),
            Err(error) if device != "CPU" => {
                eprintln!("parakeet: the encoder runs on the CPU: {error}");
                (core.compile(&encoder, "CPU", priority)?, "CPU".to_owned())
            }
            Err(error) => return Err(error),
        };
        let decoder = core.compile_on_one_thread(&part("decoder")?)?;
        let joint = core.compile_on_one_thread(&part("joint")?)?;
        Ok(Self {
            mel: mel.request()?,
            encoder: encoder.request()?,
            decoder: decoder.request()?,
            joint: joint.request()?,
            vocab,
            _core: core,
            device,
        })
    }

    /// What's said in 16 kHz audio of any length, and how long it took.
    ///
    /// # Errors
    ///
    /// When a run of any of the models fails.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<(String, Timings)> {
        let mut timings = Timings {
            mel: Duration::ZERO,
            encoder: Duration::ZERO,
            decoder: Duration::ZERO,
            tokens: 0,
        };
        let mut tokens = Vec::new();
        // The prediction network starts from blank, once a take.
        let started = Instant::now();
        let mut predicted = self.predict(
            self.vocab.blank(),
            &Predicted {
                out: Vec::new(),
                h: vec![0.0; LAYERS * PREDICTED],
                c: vec![0.0; LAYERS * PREDICTED],
            },
        )?;
        timings.decoder += started.elapsed();
        for window in audio.chunks(WINDOW) {
            let started = Instant::now();
            let (mel, mel_frames) = self.spectrogram(window)?;
            let after_mel = Instant::now();
            let frames = self.encode(&mel, mel_frames)?;
            let after_encoder = Instant::now();
            let mut steps = Steps {
                parakeet: self,
                frames: &frames,
                predicted,
            };
            tokens.extend(greedy(frames.len(), &mut steps)?);
            predicted = steps.predicted;
            timings.mel += after_mel - started;
            timings.encoder += after_encoder - after_mel;
            timings.decoder += after_encoder.elapsed();
        }
        timings.tokens = tokens.len();
        Ok((self.vocab.text(&tokens), timings))
    }

    /// A window's spectrogram, its frames that hold sound counted.
    fn spectrogram(&mut self, window: &[f32]) -> Result<(Vec<f32>, i64)> {
        let mut signal = vec![0.0; WINDOW];
        signal[..window.len()].copy_from_slice(window);
        let length = i64::try_from(window.len()).map_err(|_| "a window too long")?;
        self.mel.infer(&[
            Input::F32("input_signals", &[1, i64_of(WINDOW)], &signal),
            Input::I64("input_length", &[1], &[length]),
        ])?;
        // The spectrogram's output has no name, so it's read by its place.
        let (_, mel) = self.mel.output_at(0)?;
        let frames = first(&self.mel.integers("seq_len")?)?;
        Ok((mel, frames))
    }

    /// The encoder's frames for a spectrogram, each `ENCODED` wide, as
    /// many as hold sound.
    fn encode(&mut self, mel: &[f32], mel_frames: i64) -> Result<Vec<Vec<f32>>> {
        let length = i32::try_from(mel_frames).map_err(|_| "too many frames")?;
        let (shape, encoded) = self.encoder.run(
            &[
                Input::F32("melspectogram", &[1, MELS, MEL_FRAMES], mel),
                Input::I32("melspectogram_length", &[1], &[length]),
            ],
            "encoder_output",
            None,
        )?;
        let count = first(&self.encoder.integers("encoder_output_length")?)?;
        let [_, width, all] = shape[..] else {
            return Err(format!("the encoder's output is shaped {shape:?}"));
        };
        let (width, all) = (usize_of(width)?, usize_of(all)?);
        if width != ENCODED {
            return Err(format!("the encoder's output is {width} wide"));
        }
        // The output is laid out a dimension at a time, every frame's value
        // for each, so each frame is gathered across them.
        Ok((0..usize_of(count)?.min(all))
            .map(|t| (0..width).map(|d| encoded[d * all + t]).collect())
            .collect())
    }

    /// The decoder's output and state once it's read `token` after `was`.
    fn predict(&mut self, token: usize, was: &Predicted) -> Result<Predicted> {
        let state = [i64_of(LAYERS), 1, i64_of(PREDICTED)];
        let (_, out) = self.decoder.run(
            &[
                Input::I64("targets", &[1, 1], &[i64_of(token)]),
                Input::F32("h_in", &state, &was.h),
                Input::F32("c_in", &state, &was.c),
            ],
            "decoder_output",
            None,
        )?;
        let (_, h) = self.decoder.output("h_out", None)?;
        let (_, c) = self.decoder.output("c_out", None)?;
        Ok(Predicted { out, h, c })
    }

    /// The joint's scores for a frame after what the decoder last read.
    fn join(&mut self, frame: &[f32], predicted: &[f32]) -> Result<Vec<f32>> {
        let (_, logits) = self.joint.run(
            &[
                Input::F32("encoder_outputs", &[1, 1, i64_of(ENCODED)], frame),
                Input::F32("decoder_outputs", &[1, 1, i64_of(PREDICTED)], predicted),
            ],
            "logits",
            None,
        )?;
        Ok(logits)
    }
}

/// What greedy decoding asks of the model, so it can be tested without one.
trait Model {
    /// The vocabulary's tokens are those below this; this one is blank.
    fn blank(&self) -> usize;
    /// Whether `token` may be written.
    fn allowed(&self, token: usize) -> bool;
    /// The joint's scores for `frame`: a token's each, then each duration's.
    fn join(&mut self, frame: usize) -> Result<Vec<f32>>;
    /// Gives the decoder `token`, written.
    fn predict(&mut self, token: usize) -> Result<()>;
}

/// A window's frames, decoded with the decoder's state carried in.
struct Steps<'a> {
    parakeet: &'a mut Parakeet,
    frames: &'a [Vec<f32>],
    predicted: Predicted,
}

impl Model for Steps<'_> {
    fn blank(&self) -> usize {
        self.parakeet.vocab.blank()
    }

    fn allowed(&self, token: usize) -> bool {
        self.parakeet.vocab.allowed(token)
    }

    fn join(&mut self, frame: usize) -> Result<Vec<f32>> {
        self.parakeet.join(&self.frames[frame], &self.predicted.out)
    }

    fn predict(&mut self, token: usize) -> Result<()> {
        self.predicted = self.parakeet.predict(token, &self.predicted)?;
        Ok(())
    }
}

/// The tokens written over `frames` frames, greedily: at each frame the
/// likeliest token allowed, or blank, and the likeliest move on. A token
/// that names no move stays on the frame for another, up to
/// `MOST_PER_FRAME`; a blank that names none moves on one.
fn greedy(frames: usize, model: &mut impl Model) -> Result<Vec<usize>> {
    let blank = model.blank();
    let mut tokens = Vec::new();
    let (mut t, mut written) = (0, 0);
    while t < frames {
        let scores = model.join(t)?;
        if scores.len() != blank + 1 + DURATIONS.len() {
            return Err(format!("the joint gave {} scores", scores.len()));
        }
        let token = likeliest(&scores[..=blank], |id| id == blank || model.allowed(id));
        let mut step = DURATIONS[likeliest(&scores[blank + 1..], |_| true)];
        if token != blank {
            tokens.push(token);
            model.predict(token)?;
            written += 1;
        }
        if step == 0 && (token == blank || written >= MOST_PER_FRAME) {
            step = 1;
        }
        if step > 0 {
            t += step;
            written = 0;
        }
    }
    Ok(tokens)
}

/// The place of the highest of `scores` that `allowed` lets through.
fn likeliest(scores: &[f32], allowed: impl Fn(usize) -> bool) -> usize {
    scores
        .iter()
        .enumerate()
        .filter(|&(id, _)| allowed(id))
        .fold((0, f32::NEG_INFINITY), |best, (id, &score)| {
            if score > best.1 { (id, score) } else { best }
        })
        .0
}

/// The model's pieces, by token, and which may be written.
struct Vocab {
    pieces: Vec<String>,
    allowed: Vec<bool>,
}

impl Vocab {
    /// The vocabulary from the export's JSON, an object of pieces by
    /// their tokens' numbers.
    fn parse(json: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("reading the vocabulary: {e}"))?;
        let object = value.as_object().ok_or("the vocabulary isn't an object")?;
        let mut pieces = vec![String::new(); object.len()];
        for (id, piece) in object {
            let slot = id
                .parse::<usize>()
                .ok()
                .and_then(|id| pieces.get_mut(id))
                .ok_or_else(|| format!("the vocabulary's {id} isn't one of its tokens"))?;
            piece
                .as_str()
                .ok_or_else(|| format!("the vocabulary's {id} isn't text"))?
                .clone_into(slot);
        }
        let allowed = pieces.iter().map(|piece| english(piece)).collect();
        Ok(Self { pieces, allowed })
    }

    /// Blank, the token after the vocabulary's last.
    fn blank(&self) -> usize {
        self.pieces.len()
    }

    fn allowed(&self, token: usize) -> bool {
        self.allowed.get(token).copied().unwrap_or(false)
    }

    /// The text `tokens` spell, each word's piece starting it with a space.
    fn text(&self, tokens: &[usize]) -> String {
        let joined: String = tokens
            .iter()
            .filter_map(|&token| self.pieces.get(token))
            .map(String::as_str)
            .collect();
        joined.replace(SPACE, " ").trim().to_owned()
    }
}

/// Whether a piece is English text: no special token such as `<unk>` or
/// `<|nospeech|>`, and nothing outside ASCII but the mark of a word's start.
fn english(piece: &str) -> bool {
    let special = piece.len() > 2 && piece.starts_with('<') && piece.ends_with('>');
    !special && piece.chars().all(|c| c == SPACE || c.is_ascii())
}

fn first(values: &[i64]) -> Result<i64> {
    values
        .first()
        .copied()
        .ok_or_else(|| "a length with no value".to_owned())
}

fn i64_of(n: usize) -> i64 {
    i64::try_from(n).expect("a size Parakeet uses fits in 64 bits")
}

fn usize_of(n: i64) -> Result<usize> {
    usize::try_from(n).map_err(|_| format!("a negative size, {n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOCAB: &str = r#"{"0": "<unk>", "1": "<|nospeech|>", "2": "▁hello", "3": "▁wor",
        "4": "ld", "5": "▁é", "6": "<", "7": "▁ne"}"#;

    fn vocab() -> Vocab {
        Vocab::parse(VOCAB).unwrap()
    }

    /// A model whose joint gives each frame's scores from a script, a
    /// token and a move a call, and that notes what the decoder was given.
    struct Scripted {
        vocab: Vocab,
        // For each call to join: the frame it's expected at, the token
        // scored highest, and the duration's place scored highest.
        script: Vec<(usize, usize, usize)>,
        // A token scored second, to show a refused one's place is taken.
        runner_up: usize,
        joined: usize,
        predicted: Vec<usize>,
    }

    impl Scripted {
        fn new(script: &[(usize, usize, usize)]) -> Self {
            Self {
                vocab: vocab(),
                script: script.to_vec(),
                runner_up: 3,
                joined: 0,
                predicted: Vec::new(),
            }
        }
    }

    impl Model for Scripted {
        fn blank(&self) -> usize {
            self.vocab.blank()
        }

        fn allowed(&self, token: usize) -> bool {
            self.vocab.allowed(token)
        }

        fn join(&mut self, frame: usize) -> Result<Vec<f32>> {
            let (at, token, step) = self.script[self.joined];
            assert_eq!(frame, at, "call {} read the wrong frame", self.joined);
            self.joined += 1;
            let mut scores = vec![0.0; self.blank() + 1 + DURATIONS.len()];
            scores[self.runner_up] = 1.0;
            scores[token] = 2.0;
            scores[self.blank() + 1 + step] = 1.0;
            Ok(scores)
        }

        fn predict(&mut self, token: usize) -> Result<()> {
            self.predicted.push(token);
            Ok(())
        }
    }

    #[test]
    fn words_are_spelled_from_their_pieces() {
        assert_eq!(vocab().text(&[2, 3, 4]), "hello world");
    }

    #[test]
    fn blank_follows_the_vocabulary() {
        assert_eq!(vocab().blank(), 8);
    }

    #[test]
    fn only_english_text_may_be_written() {
        let vocab = vocab();
        let allowed: Vec<usize> = (0..vocab.blank()).filter(|&t| vocab.allowed(t)).collect();
        assert_eq!(allowed, [2, 3, 4, 6, 7]);
    }

    #[test]
    fn a_vocabulary_with_a_gap_is_refused() {
        assert!(Vocab::parse(r#"{"0": "a", "2": "b"}"#).is_err());
    }

    #[test]
    fn a_token_and_its_move_are_followed() {
        // Frame 0 writes "hello" and moves two on; frame 2 writes "wor"
        // and stays; then "ld" and moves on one, to the last frame's blank.
        let mut model = Scripted::new(&[(0, 2, 2), (2, 3, 0), (2, 4, 1), (3, 8, 1)]);
        let tokens = greedy(4, &mut model).unwrap();
        assert_eq!(tokens, [2, 3, 4]);
        assert_eq!(model.predicted, [2, 3, 4]);
    }

    #[test]
    fn a_blank_naming_no_move_moves_on_one() {
        let mut model = Scripted::new(&[(0, 8, 0), (1, 8, 0), (2, 8, 0)]);
        assert!(greedy(3, &mut model).unwrap().is_empty());
        assert!(model.predicted.is_empty());
    }

    #[test]
    fn a_frame_writes_at_most_its_share() {
        let mut script = vec![(0, 2, 0); MOST_PER_FRAME];
        script.push((1, 8, 1));
        let mut model = Scripted::new(&script);
        assert_eq!(greedy(2, &mut model).unwrap(), [2; MOST_PER_FRAME]);
    }

    #[test]
    fn a_token_outside_english_gives_way_to_the_next() {
        // "é" and "<|nospeech|>" score highest, so the runner up is written.
        let mut model = Scripted::new(&[(0, 5, 1), (1, 1, 1)]);
        assert_eq!(greedy(2, &mut model).unwrap(), [3, 3]);
    }

    #[test]
    fn a_joint_of_the_wrong_size_is_refused() {
        struct Short;
        impl Model for Short {
            fn blank(&self) -> usize {
                8
            }
            fn allowed(&self, _: usize) -> bool {
                true
            }
            fn join(&mut self, _: usize) -> Result<Vec<f32>> {
                Ok(vec![0.0; 9])
            }
            fn predict(&mut self, _: usize) -> Result<()> {
                Ok(())
            }
        }
        assert!(greedy(1, &mut Short).is_err());
    }
}
