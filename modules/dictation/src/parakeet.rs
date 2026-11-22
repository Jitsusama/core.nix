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
//! The export reads 15 s at a time. Call-transcripts' takes are cut to fit
//! one (takes.rs), since a word at a window's edge can be lost; a longer
//! sound is still heard, as windows of 15 s, the decoder's state carried
//! from one to the next so a window's first word follows the last one's.
//! Decoding is greedy, as TDT's is: at each frame the joint names a token
//! and how many frames to move on, a blank writes nothing, and a frame
//! writes at most `MOST_PER_FRAME`. Each token's frame says when its word
//! was said, so a take can be split where its voice changes.
//!
//! The model knows 25 languages, and now and then heard English as
//! another ("Look at all this stuff man" once came out as "Legal, ne?"),
//! so it's held to English: no token with a letter outside ASCII, and no
//! special token, is ever chosen.
//!
//! Long because over a third of it is its cases, each pinning a step of
//! decoding or spelling against a scripted model or a vocabulary.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::cache;
use crate::openvino::{Core, Input, Priority, Request, Result};

/// The rate it hears at.
pub const RATE: usize = 16_000;
/// The samples the export reads at once: 15 s.
pub const WINDOW: usize = 15 * RATE;
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
/// The tokenizer's mark on a piece that starts a word. Parakeet's
/// exported vocabulary writes a plain space in its place, so `words`
/// takes either.
const SPACE: char = '\u{2581}';
/// The samples an encoder frame stands for, 80 ms: the spectrogram's
/// 10 ms hop, eight of which the encoder makes one.
const FRAME: usize = 8 * RATE / 100;
/// The encoder's frames for a window when it steps by FRAME: three
/// halvings of the spectrogram's 1501 frames, each rounding up.
const STEPPED: usize = 188;

/// A word heard, and the sample of the sound it starts at, by the frame
/// its first piece was written on.
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub at: usize,
}

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
        let kept = cache.map(|cache| cache::kept(cache, &Self::parts(dir, device)));
        let parakeet = Self::load_in(dir, device, kept.as_deref(), priority)?;
        if let (Some(cache), Some(kept)) = (cache, &kept) {
            cache::forget_others(cache, kept);
        }
        Ok(parakeet)
    }

    /// Whether the models for `dir` on `device` are compiled in `cache`
    /// for this build, so a load needn't compile them: the folder `load`
    /// keeps them in holds something. OpenVINO writes each model there once
    /// it's compiled, so a compile cut short still counts, and the next
    /// load compiles only what's missing.
    pub fn compiled(dir: &Path, device: &str, cache: Option<&Path>) -> bool {
        cache.is_some_and(|cache| {
            let kept = cache::kept(cache, &Self::parts(dir, device));
            std::fs::read_dir(kept).is_ok_and(|mut held| held.next().is_some())
        })
    }

    /// Whether to compile the models for `dir` on `device` ahead of need:
    /// only on an NPU the machine has, which `npu` says, and only while
    /// nothing is compiled for this build. The NPU's compile takes a
    /// minute; without one the encoder compiles for the CPU as it loads,
    /// and compiling it early only held 2.4 GB, which filled a desktop
    /// VM's memory on 12 November.
    pub fn worth_compiling(dir: &Path, device: &str, cache: Option<&Path>, npu: bool) -> bool {
        device != "CPU" && npu && !Self::compiled(dir, device, cache)
    }

    /// What the compiled models' folder is keyed on besides the build.
    fn parts<'a>(dir: &'a Path, device: &'a str) -> [&'a [u8]; 2] {
        [dir.as_os_str().as_encoded_bytes(), device.as_bytes()]
    }

    fn load_in(dir: &Path, device: &str, cache: Option<&Path>, priority: Priority) -> Result<Self> {
        let vocab = std::fs::read_to_string(dir.join("parakeet_v3_vocab.json"))
            .map_err(|e| format!("reading parakeet_v3_vocab.json: {e}"))
            .and_then(|json| Vocab::parse(&json))?;
        let core = Core::new(cache)?;
        let file = |name: &str| dir.join(format!("parakeet_{name}.xml"));
        let part = |name: &str| core.read(&file(name));
        let mel = core.compile(&part("melspectogram")?, "CPU", priority)?;
        // The encoder, a gigabyte, is compiled from its file, so what's
        // cached is found without first hashing all of it.
        let encoder = file("encoder");
        let (encoder, device) = match core.compile_file(&encoder, device, priority) {
            Ok(compiled) => (compiled, device.to_owned()),
            Err(error) if device != "CPU" => {
                eprintln!("parakeet: the encoder runs on the CPU: {error}");
                (
                    core.compile_file(&encoder, "CPU", priority)?,
                    "CPU".to_owned(),
                )
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
        let (tokens, timings) = self.decode(audio)?;
        let tokens: Vec<usize> = tokens.iter().map(|&(token, _)| token).collect();
        Ok((self.vocab.text(&tokens), timings))
    }

    /// The words said in 16 kHz audio of any length, each with where it
    /// starts, so a take can be split where its voice changes; they spell
    /// what `transcribe` hears, a space between each.
    ///
    /// # Errors
    ///
    /// When a run of any of the models fails.
    pub fn transcribe_words(&mut self, audio: &[f32]) -> Result<(Vec<Word>, Timings)> {
        let (tokens, timings) = self.decode(audio)?;
        Ok((self.vocab.words(&tokens), timings))
    }

    /// The tokens written over `audio`, each with the sample it was
    /// written at, and how long it took.
    fn decode(&mut self, audio: &[f32]) -> Result<(Vec<(usize, usize)>, Timings)> {
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
        for (number, window) in audio.chunks(WINDOW).enumerate() {
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
            tokens.extend(
                greedy(frames.len(), &mut steps)?
                    .into_iter()
                    .map(|(token, frame)| (token, sample_of(number, frame))),
            );
            predicted = steps.predicted;
            timings.mel += after_mel - started;
            timings.encoder += after_encoder - after_mel;
            timings.decoder += after_encoder.elapsed();
        }
        timings.tokens = tokens.len();
        Ok((tokens, timings))
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
        // It's checked here, on every window, rather than only where words
        // are timed, so each check that runs the real model proves it.
        steps_by_frame(all)?;
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

/// The tokens written over `frames` frames, greedily, each with the frame
/// it was written on: at each frame the likeliest token allowed, or
/// blank, and the likeliest move on. A token that names no move stays on
/// the frame for another, up to `MOST_PER_FRAME`; a blank that names none
/// moves on one.
fn greedy(frames: usize, model: &mut impl Model) -> Result<Vec<(usize, usize)>> {
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
            tokens.push((token, t));
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

    /// The words `tokens` spell, each at the sample its first piece was
    /// written at; a piece that starts a word starts the next.
    fn words(&self, tokens: &[(usize, usize)]) -> Vec<Word> {
        let mut words: Vec<Word> = Vec::new();
        for &(token, at) in tokens {
            let Some(piece) = self.pieces.get(token) else {
                continue;
            };
            let text = piece.trim_start_matches([SPACE, ' ']);
            match words.last_mut() {
                Some(word) if text.len() == piece.len() || word.text.is_empty() => {
                    word.text.push_str(text);
                }
                _ => words.push(Word {
                    text: text.to_owned(),
                    at,
                }),
            }
        }
        words.retain(|word| !word.text.is_empty());
        words
    }
}

/// Whether a piece is English text: no special token such as `<unk>` or
/// `<|nospeech|>`, and nothing outside ASCII but the mark of a word's start.
fn english(piece: &str) -> bool {
    let special = piece.len() > 2 && piece.starts_with('<') && piece.ends_with('>');
    !special && piece.chars().all(|c| c == SPACE || c.is_ascii())
}

/// Whether an encoder that gives `all` frames a window steps by FRAME.
/// Word times rest on it, so one that steps otherwise is refused rather
/// than misplacing every word.
fn steps_by_frame(all: usize) -> Result<()> {
    if all == STEPPED {
        Ok(())
    } else {
        Err(format!(
            "the encoder gave {all} frames a window, not {STEPPED}, so it doesn't step by 80 ms"
        ))
    }
}

/// The sample of a take that window `number`'s encoder frame `frame`
/// starts at.
fn sample_of(number: usize, frame: usize) -> usize {
    number * WINDOW + frame * FRAME
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

    // Spaced as Parakeet's exported vocabulary is: a piece that starts a
    // word leads with a plain space, never the tokenizer's own mark.
    const VOCAB: &str = r#"{"0": "<unk>", "1": "<|nospeech|>", "2": " hello", "3": " wor",
        "4": "ld", "5": " é", "6": "<", "7": " ne"}"#;

    fn vocab() -> Vocab {
        Vocab::parse(VOCAB).unwrap()
    }

    #[test]
    fn parakeet_is_compiled_once_its_own_folder_holds_models() {
        let cache = std::env::temp_dir().join(format!("parakeet-compiled-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);
        let dir = Path::new("/models/parakeet");
        assert!(
            !Parakeet::compiled(dir, "NPU", Some(&cache)),
            "an empty cache was taken for compiled"
        );
        let other = cache::kept(&cache, &[dir.as_os_str().as_encoded_bytes(), b"GPU"]);
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("encoder.blob"), b"compiled").unwrap();
        assert!(
            !Parakeet::compiled(dir, "NPU", Some(&cache)),
            "models compiled for another device were taken for this one's"
        );
        let kept = cache::kept(&cache, &[dir.as_os_str().as_encoded_bytes(), b"NPU"]);
        std::fs::create_dir_all(&kept).unwrap();
        std::fs::write(kept.join("encoder.blob"), b"compiled").unwrap();
        assert!(
            Parakeet::compiled(dir, "NPU", Some(&cache)),
            "its own folder's models weren't found"
        );
        assert!(
            !Parakeet::compiled(dir, "NPU", None),
            "with no cache nothing is kept, so nothing is compiled ahead"
        );
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn parakeet_compiles_ahead_only_on_an_npu_the_machine_has() {
        let cache = std::env::temp_dir().join(format!("parakeet-ahead-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);
        let dir = Path::new("/models/parakeet");
        assert!(
            Parakeet::worth_compiling(dir, "NPU", Some(&cache), true),
            "an NPU with nothing compiled wasn't compiled for ahead"
        );
        assert!(
            !Parakeet::worth_compiling(dir, "NPU", Some(&cache), false),
            "with no NPU, Parakeet would be compiled ahead on the CPU"
        );
        assert!(
            !Parakeet::worth_compiling(dir, "CPU", Some(&cache), true),
            "set to the CPU, Parakeet would be compiled ahead"
        );
        let kept = cache::kept(&cache, &[dir.as_os_str().as_encoded_bytes(), b"NPU"]);
        std::fs::create_dir_all(&kept).unwrap();
        std::fs::write(kept.join("encoder.blob"), b"compiled").unwrap();
        assert!(
            !Parakeet::worth_compiling(dir, "NPU", Some(&cache), true),
            "compiled already, it would be compiled again"
        );
        let _ = std::fs::remove_dir_all(&cache);
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
        assert!(english("\u{2581}hello"), "the tokenizer's mark refused");
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
        assert_eq!(
            tokens,
            [(2, 0), (3, 2), (4, 2)],
            "the tokens, or the frames they were written on, were wrong"
        );
        assert_eq!(model.predicted, [2, 3, 4]);
    }

    #[test]
    fn a_frame_is_timed_from_the_takes_start_through_its_window() {
        assert_eq!(sample_of(0, 3), 3 * 1280, "a frame isn't 80 ms");
        assert_eq!(
            sample_of(2, 1),
            2 * 15 * RATE + 1280,
            "a later window's frame isn't timed from the take's start"
        );
    }

    #[test]
    fn an_encoder_stepping_otherwise_than_by_a_frame_is_refused() {
        let halved = (0..3).fold(usize::try_from(MEL_FRAMES).unwrap(), |n, _| n.div_ceil(2));
        assert_eq!(
            STEPPED, halved,
            "STEPPED isn't the spectrogram's frames halved thrice"
        );
        assert_eq!(
            steps_by_frame(STEPPED),
            Ok(()),
            "the shipped encoder's step was refused"
        );
        for all in [94, 187, 189, 376] {
            assert!(
                steps_by_frame(all).is_err(),
                "an encoder giving {all} frames a window wasn't refused"
            );
        }
    }

    #[test]
    fn words_start_where_their_first_piece_was_written_and_spell_the_text() {
        let vocab = vocab();
        let tokens = [(2, 0), (3, 2 * FRAME), (4, 3 * FRAME)];
        let words = vocab.words(&tokens);
        assert_eq!(
            words,
            [
                Word {
                    text: "hello".to_owned(),
                    at: 0
                },
                Word {
                    text: "world".to_owned(),
                    at: 2 * FRAME
                }
            ],
            "a word didn't start at its first piece"
        );
        let spelled: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(
            spelled.join(" "),
            vocab.text(&[2, 3, 4]),
            "the words spell other than the text"
        );
    }

    #[test]
    fn a_piece_that_doesnt_start_a_word_opens_one_when_nothing_came_before() {
        let words = vocab().words(&[(4, FRAME), (7, 2 * FRAME)]);
        let spelled: Vec<(&str, usize)> = words.iter().map(|w| (w.text.as_str(), w.at)).collect();
        assert_eq!(
            spelled,
            [("ld", FRAME), ("ne", 2 * FRAME)],
            "a take opening mid-word lost its first piece"
        );
    }

    #[test]
    fn a_bare_word_start_opens_the_word_its_next_piece_spells() {
        // Parakeet's export has one bare piece, a plain space; the
        // tokenizer's own vocabularies write its mark, which `words` takes too.
        for mark in [" ", "\u{2581}"] {
            let pieces = format!(r#"{{"0": "{mark}", "1": "ld", "2": "{mark}hello"}}"#);
            let vocab = Vocab::parse(&pieces).unwrap();
            let tokens = [(0, 0), (1, FRAME), (2, 2 * FRAME)];
            let words = vocab.words(&tokens);
            let spelled: Vec<(&str, usize)> =
                words.iter().map(|w| (w.text.as_str(), w.at)).collect();
            assert_eq!(
                spelled,
                [("ld", 0), ("hello", 2 * FRAME)],
                "a bare {mark:?} didn't open the word after it"
            );
            let joined: Vec<&str> = spelled.iter().map(|(text, _)| *text).collect();
            assert_eq!(
                joined.join(" "),
                vocab.text(&[0, 1, 2]),
                "{mark:?} spelled otherwise"
            );
        }
    }

    #[test]
    fn the_shipped_vocabulary_gives_a_word_for_each_its_text_has() {
        // The build names the vocabulary Parakeet ships with (package.nix),
        // so one spaced another way fails here rather than leaving every
        // call's takes whole; `nix develop` on the package names it too.
        // A plain `cargo test`, or one in the repository's shell, has none
        // to read. A shell sets NIX_BUILD_TOP as a build does, but sets
        // IN_NIX_SHELL beside it.
        let Some(path) = std::env::var_os("PARAKEET_VOCAB") else {
            let building = std::env::var_os("NIX_BUILD_TOP").is_some()
                && std::env::var_os("IN_NIX_SHELL").is_none();
            assert!(
                !building,
                "the build named no vocabulary in PARAKEET_VOCAB, as package.nix does"
            );
            eprintln!("the shipped vocabulary wasn't read: PARAKEET_VOCAB names none");
            return;
        };
        let path = Path::new(&path);
        let read = std::fs::read_to_string(path)
            .unwrap_or_else(|why| panic!("PARAKEET_VOCAB's {} didn't read: {why}", path.display()));
        let vocab = Vocab::parse(&read)
            .unwrap_or_else(|why| panic!("PARAKEET_VOCAB's {} isn't one: {why}", path.display()));
        let tokens: Vec<usize> = (0..vocab.blank()).filter(|&t| vocab.allowed(t)).collect();
        let timed: Vec<(usize, usize)> = tokens.iter().map(|&t| (t, t * FRAME)).collect();
        let words = vocab.words(&timed);
        let spelled: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        let text = vocab.text(&tokens);
        let said: Vec<&str> = text.split_whitespace().collect();
        assert!(
            said.len() > 1000,
            "the vocabulary spelled only {} words",
            said.len()
        );
        assert_eq!(spelled, said, "its words aren't the ones its text has");
    }

    #[test]
    fn a_blank_naming_no_move_moves_on_one() {
        let mut model = Scripted::new(&[(0, 8, 0), (1, 8, 0), (2, 8, 0)]);
        assert_eq!(
            greedy(3, &mut model).unwrap(),
            [] as [(usize, usize); 0],
            "a blank wrote a token"
        );
        assert_eq!(model.predicted, [] as [usize; 0]);
    }

    #[test]
    fn a_frame_writes_at_most_its_share() {
        let mut script = vec![(0, 2, 0); MOST_PER_FRAME];
        script.push((1, 8, 1));
        let mut model = Scripted::new(&script);
        assert_eq!(
            greedy(2, &mut model).unwrap(),
            [(2, 0); MOST_PER_FRAME],
            "a frame wrote more than its share, or on another frame"
        );
    }

    #[test]
    fn a_token_outside_english_gives_way_to_the_next() {
        // "é" and "<|nospeech|>" score highest, so the runner up is written.
        let mut model = Scripted::new(&[(0, 5, 1), (1, 1, 1)]);
        assert_eq!(
            greedy(2, &mut model).unwrap(),
            [(3, 0), (3, 1)],
            "a token outside English was written, or not on its frame"
        );
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
