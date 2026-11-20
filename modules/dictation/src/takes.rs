//! One side of a call cut into takes Parakeet can hear: each inside its
//! 15 s window, so no word falls across a window's edge, cut where the
//! speaker pauses so no word is split, and only where someone spoke, since
//! a speech model given silence or hum writes words nobody said.
//!
//! Speech is told from the quiet by loudness against the side's own floor,
//! its quietest 20 ms of the last ten seconds, so a fan or a far end's
//! comfort noise counts as quiet wherever it sits. Even unbroken talk
//! falls near the floor at its stops and breaths, so talking long doesn't
//! raise it to the talker's own level, while a louder room raises it
//! within the ten seconds. It's fed sound as it
//! comes, so a live call's takes are ready as each pause ends.
//!
//! Loud isn't always speech: a breath, a cough or a burst of static is
//! loud too, and Parakeet writes "Yeah." for a breath. A voice has a
//! pitch and they have none, so a take is only kept when enough of its
//! loud frames repeat at a voice's pitch. A whisper has none either, and
//! isn't heard: on a call, where nobody whispers to be written down, that
//! costs less than every breath becoming a word. A tone has a pitch and
//! is heard, but Parakeet writes nothing for a ringtone, a chime, a chord
//! or a hum.

use std::collections::VecDeque;

use crate::parakeet::{RATE, WINDOW};

/// The loudness of each 20 ms is judged once.
const FRAME: usize = RATE / 50;
/// A pause this long ends a take: about the gap between sentences.
const PAUSE_FRAMES: usize = 25;
/// What's kept before speech starts and after it ends, so a word's soft
/// start and end stay in.
const EDGE_FRAMES: usize = 10;
/// A take is cut by this length at the latest, a second inside
/// Parakeet's window, so it's heard whole.
const LONGEST_FRAMES: usize = 14 * 50;
const _: () = assert!(LONGEST_FRAMES * FRAME < WINDOW);
/// The longest a take runs, in samples.
pub const LONGEST: usize = LONGEST_FRAMES * FRAME;
/// Where a take that long is cut: at its quietest 20 ms in its last few
/// seconds.
const CUT_WITHIN_FRAMES: usize = 5 * 50;
/// Less speech than this, a cough or a click, isn't a take.
const LEAST_SPEECH_FRAMES: usize = 15;
/// How far back the floor is judged from.
const FLOOR_FRAMES: usize = 10 * 50;
/// How far above the floor speech is.
const ABOVE_FLOOR_DB: f32 = 12.0;
/// Quieter than this is never speech, however quiet the floor.
const QUIETEST_SPEECH_DB: f32 = -55.0;
/// Quieter than this is a line sending nothing, as a call does while
/// its far end is muted, so it doesn't set the floor: the line's hiss,
/// back with the next word, isn't read as loud.
const NOTHING_DB: f32 = -90.0;
/// Fewer voiced frames than this and a take has no voice in it. Each is
/// read with the one before, so three take 80 ms of voice: the vowel of a
/// lone "Yeah." holds several times as much. Five left out 13 of the
/// meetings' words more.
const LEAST_VOICED_FRAMES: usize = 3;
/// A longer take needs a voiced frame for each this many loud ones, so
/// noise voiced now and then, as a breath through the throat is, isn't
/// heard just by lasting. Every one of the meetings' takes that held
/// words had at least one in ten; a word in a long run of loud static
/// may be lost.
const LOUD_PER_VOICED: usize = 20;
/// The shortest and longest lags a repeat is looked for at, in samples: a
/// period of 400 Hz down to 60 Hz. A higher pitch repeats at a multiple
/// of its period too, so it's voiced, while a hum below about 50 Hz
/// repeats only past the longest, so it isn't. Mains hum at 60 Hz is
/// voiced; at 50 Hz it's on the line, voiced or not by where its cycles
/// fall in a frame, and Parakeet writes nothing for either.
const SHORTEST_PERIOD: usize = RATE / 400;
const LONGEST_PERIOD: usize = RATE / 60;
/// Each sample is compared less the one this far before it, 6 samples at
/// 16 kHz. That takes out an offset and a breath's slow drift, and its
/// gain peaks near 1.3 kHz, among a low voice's strong harmonics, so a
/// line's hiss gains about 9 dB on an 85 Hz voice. Less the very next
/// sample, hiss gains about 24 dB, and a voice with hiss 10 dB under it
/// has no voiced frame.
const APART: usize = RATE * 3 / 8000;
/// How alike sound one period apart must be for a frame to be voiced. A
/// voice's vowels are near 1, and white or drifting noise near 0. Noise
/// rung through the throat's resonances, as a breath is, is alike to
/// itself a few periods on, the more so the narrower they ring: at 0.4,
/// one in eight frames of a breath through two 200 Hz wide resonances
/// was voiced, against one in a hundred at 0.5. Every one of the
/// meetings' takes that held words still had three frames past 0.55.
const VOICED_LIKENESS: f32 = 0.5;

/// Some speech, and where it starts in the side's sound, in samples.
#[derive(Debug, PartialEq)]
pub struct Take {
    pub start: usize,
    pub samples: Vec<f32>,
}

/// Cuts the sound it's fed into takes.
#[derive(Default)]
pub struct Cutter {
    /// Samples fed that don't yet make a whole frame.
    partial: Vec<f32>,
    /// Frames fed so far.
    frames: usize,
    /// The last frames' loudness, for the floor.
    recent: VecDeque<f32>,
    /// The frames just before a take, so its start can be kept.
    before: VecDeque<Vec<f32>>,
    /// The take under way, frame by frame with each one's loudness.
    take: Vec<(Vec<f32>, f32, bool)>,
    /// The frame the take under way starts at.
    take_start: usize,
}

impl Cutter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds sound, and gives each take it ends.
    pub fn feed(&mut self, samples: &[f32]) -> Vec<Take> {
        let mut ended = Vec::new();
        self.partial.extend_from_slice(samples);
        let whole = self.partial.len() / FRAME * FRAME;
        let frames: Vec<Vec<f32>> = self
            .partial
            .drain(..whole)
            .collect::<Vec<_>>()
            .chunks(FRAME)
            .map(<[f32]>::to_vec)
            .collect();
        for frame in frames {
            ended.extend(self.frame(frame));
        }
        ended
    }

    /// Ends the sound, giving the take under way if it holds speech.
    pub fn finish(&mut self) -> Option<Take> {
        let rest = std::mem::take(&mut self.partial);
        if !rest.is_empty() && !self.take.is_empty() {
            let loudness = loudness(&rest);
            self.take.push((rest, loudness, false));
        }
        let frames = self.take.len();
        self.end(frames)
    }

    fn frame(&mut self, frame: Vec<f32>) -> Option<Take> {
        let level = loudness(&frame);
        let speech = level > self.threshold();
        if level > NOTHING_DB {
            self.recent.push_back(level);
            if self.recent.len() > FLOOR_FRAMES {
                self.recent.pop_front();
            }
        }
        let index = self.frames;
        self.frames += 1;
        if self.take.is_empty() {
            if !speech {
                self.before.push_back(frame);
                if self.before.len() > EDGE_FRAMES {
                    self.before.pop_front();
                }
                return None;
            }
            self.take_start = index - self.before.len();
            for earlier in self.before.drain(..) {
                let quiet = loudness(&earlier);
                self.take.push((earlier, quiet, false));
            }
        }
        self.take.push((frame, level, speech));
        let pause = self
            .take
            .iter()
            .rev()
            .take_while(|(_, _, speech)| !speech)
            .count();
        if pause >= PAUSE_FRAMES {
            let keep = self.take.len() - pause + EDGE_FRAMES;
            return self.end(keep);
        }
        if self.take.len() >= LONGEST_FRAMES {
            let from = self.take.len() - CUT_WITHIN_FRAMES;
            let quietest = (from..self.take.len())
                .min_by(|&a, &b| self.take[a].1.total_cmp(&self.take[b].1))
                .unwrap_or(self.take.len());
            return self.end(quietest + 1);
        }
        None
    }

    /// Gives the take's first `frames` as a take if they hold enough
    /// speech, and starts the next take from what's left.
    fn end(&mut self, frames: usize) -> Option<Take> {
        let rest = self.take.split_off(frames.min(self.take.len()));
        let ended = std::mem::replace(&mut self.take, rest);
        let start = self.take_start * FRAME;
        self.take_start += frames;
        if self.take.iter().all(|(_, _, speech)| !speech) {
            // What's left is the pause: it becomes the start of the next.
            self.before = self.take.drain(..).map(|(frame, _, _)| frame).collect();
            while self.before.len() > EDGE_FRAMES {
                self.before.pop_front();
            }
        }
        let spoken = ended.iter().filter(|(_, _, speech)| *speech).count();
        // Voicing is the dearer test, so it's asked only of a take loud for
        // long enough, and only until enough frames are voiced.
        let needed = LEAST_VOICED_FRAMES.max(spoken.div_ceil(LOUD_PER_VOICED));
        let heard = spoken >= LEAST_SPEECH_FRAMES
            && ended
                .windows(2)
                .filter(|pair| pair[1].2 && voiced(&pair[0].0, &pair[1].0))
                .take(needed)
                .count()
                == needed;
        heard.then(|| Take {
            start,
            samples: ended.into_iter().flat_map(|(frame, _, _)| frame).collect(),
        })
    }

    fn threshold(&self) -> f32 {
        if self.recent.is_empty() {
            return QUIETEST_SPEECH_DB;
        }
        let floor = self.recent.iter().copied().fold(f32::INFINITY, f32::min);
        (floor + ABOVE_FLOOR_DB).max(QUIETEST_SPEECH_DB)
    }
}

/// Whether a frame, read with the one before it so a low voice's period
/// fits twice, repeats at a voice's pitch. Each sample less the one
/// `APART` before it is what's compared, so a voice riding on an offset,
/// or on the drift of a breath's low noise, still shows its own repeat
/// rather than the drift's likeness at every lag. A repeat only counts past
/// where the sound is first unlike itself, so a hum below a voice's pitch,
/// alike to itself at every short lag, isn't one either.
fn voiced(before: &[f32], frame: &[f32]) -> bool {
    let sound: Vec<f32> = before
        .iter()
        .chain(frame)
        .copied()
        .collect::<Vec<_>>()
        .windows(APART + 1)
        .map(|run| run[APART] - run[0])
        .collect();
    let mut energy = vec![0.0_f32; sound.len() + 1];
    for (i, sample) in sound.iter().enumerate() {
        energy[i + 1] = energy[i] + sample * sample;
    }
    let likeness = |period: usize| {
        let overlap = sound.len() - period;
        let together: f32 = sound[..overlap]
            .iter()
            .zip(&sound[period..])
            .map(|(a, b)| a * b)
            .sum();
        let apart = energy[overlap] * (energy[sound.len()] - energy[period]);
        if apart > 0.0 {
            together / apart.sqrt()
        } else {
            0.0
        }
    };
    let longest = LONGEST_PERIOD.min(sound.len() / 2);
    let Some(unlike) = (1..=longest).find(|&period| likeness(period) < 0.0) else {
        return false;
    };
    (unlike.max(SHORTEST_PERIOD)..=longest).any(|period| likeness(period) > VOICED_LIKENESS)
}

/// A frame's loudness in decibels below full scale.
fn loudness(frame: &[f32]) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "a frame is 320 samples")]
    let mean = frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32;
    10.0 * mean.max(1e-12).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `seconds` of a sine at `level`, stepping `step` radians a sample.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a test's few seconds of samples"
    )]
    fn tone(seconds: f32, level: f32, step: f32) -> Vec<f32> {
        let count = (seconds * RATE as f32) as usize;
        (0..count)
            .map(|i| level * (i as f32 * step).sin())
            .collect()
    }

    /// Speech stood in for by syllables of a tone at about -20 dBFS, each
    /// 200 ms, with 50 ms near the room's quiet between them, as a stop
    /// or a breath falls.
    fn spoken(seconds: f32) -> Vec<f32> {
        let syllable = [tone(0.2, 0.14, 0.07), quiet(0.05)].concat();
        syllable
            .iter()
            .copied()
            .cycle()
            .take(quiet(seconds).len())
            .collect()
    }

    /// `seconds` of noise at about `level`, white or, `brown`, each sample
    /// a step from the last, as a breath or a rumble is.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a test's few seconds of samples"
    )]
    fn noise(seconds: f32, level: f32, brown: bool) -> Vec<f32> {
        let count = (seconds * RATE as f32) as usize;
        let mut seed: u32 = 1;
        let mut walk = 0.0;
        let raw: Vec<f32> = (0..count)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let white = (seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
                walk += white;
                if brown { walk } else { white }
            })
            .collect();
        let rms = (raw.iter().map(|s| s * s).sum::<f32>() / count as f32).sqrt();
        raw.iter().map(|s| s * level / rms).collect()
    }

    /// `seconds` of a breath at about `level`: white noise softened as a
    /// breath's is, its high hiss falling away.
    fn breath(seconds: f32, level: f32) -> Vec<f32> {
        let mut soft = 0.0;
        let raw: Vec<f32> = noise(seconds, 1.0, false)
            .into_iter()
            .map(|white| {
                soft = 0.98 * soft + white;
                soft
            })
            .collect();
        let loudest = raw.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
        raw.iter().map(|s| s * level / loudest).collect()
    }

    /// `seconds` of a breath through the throat at about `level`: white
    /// noise rung through two resonances, at 600 and 1200 Hz and each
    /// 200 Hz wide, as a vowel's first two shape the breath before it.
    /// Rung that narrowly, noise is alike to itself a few periods on.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a test's few seconds of samples"
    )]
    fn throat(seconds: f32, level: f32) -> Vec<f32> {
        let ring = |sound: &[f32], pitch: f32| -> Vec<f32> {
            let rate = RATE as f32;
            let fade = (-std::f32::consts::PI * 200.0 / rate).exp();
            let turn = 2.0 * fade * (std::f32::consts::TAU * pitch / rate).cos();
            let (mut last, mut before) = (0.0_f32, 0.0_f32);
            sound
                .iter()
                .map(|white| {
                    let next = white + turn * last - fade * fade * before;
                    (before, last) = (last, next);
                    next
                })
                .collect()
        };
        let white = noise(seconds, 1.0, false);
        let raw: Vec<f32> = ring(&white, 600.0)
            .iter()
            .zip(ring(&white, 1200.0))
            .map(|(low, high)| low + high)
            .collect();
        let loudest = raw.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
        raw.iter().map(|s| s * level / loudest).collect()
    }

    /// `seconds` of a voice at about -27 dBFS: a pitch and its first twelve
    /// harmonics, each fainter, the pitch rising a tenth as a question's
    /// does.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a test's few seconds of samples"
    )]
    fn voice(seconds: f32, pitch: f32) -> Vec<f32> {
        let count = (seconds * RATE as f32) as usize;
        let mut phase = 0.0_f32;
        (0..count)
            .map(|i| {
                let rising = 1.0 + 0.1 * i as f32 / count as f32;
                phase += std::f32::consts::TAU * pitch * rising / RATE as f32;
                (1..=12)
                    .map(|k| (k as f32 * phase).sin() / k as f32)
                    .sum::<f32>()
                    * 0.05
            })
            .collect()
    }

    /// A room's quiet, about -70 dBFS.
    fn quiet(seconds: f32) -> Vec<f32> {
        tone(seconds, 0.0004, 1.3)
    }

    fn cut(sound: &[f32]) -> Vec<Take> {
        let mut cutter = Cutter::new();
        let mut takes = Vec::new();
        for piece in sound.chunks(1000) {
            takes.extend(cutter.feed(piece));
        }
        takes.extend(cutter.finish());
        takes
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "a test's sample counts stay small"
    )]
    fn seconds(samples: usize) -> f32 {
        samples as f32 / RATE as f32
    }

    #[test]
    fn a_pause_between_sentences_makes_two_takes_each_starting_just_before_its_speech() {
        let sound = [quiet(2.0), spoken(3.0), quiet(1.5), spoken(2.0), quiet(2.0)].concat();
        let takes = cut(&sound);
        assert_eq!(
            takes.len(),
            2,
            "{:?}",
            takes.iter().map(|t| t.start).collect::<Vec<_>>()
        );
        assert!(
            (seconds(takes[0].start) - 1.8).abs() < 0.05,
            "{}",
            seconds(takes[0].start)
        );
        assert!(
            (seconds(takes[1].start) - 6.3).abs() < 0.05,
            "{}",
            seconds(takes[1].start)
        );
    }

    #[test]
    fn a_breath_between_words_stays_in_one_take() {
        let sound = [quiet(1.0), spoken(2.0), quiet(0.2), spoken(2.0), quiet(1.0)].concat();
        assert_eq!(cut(&sound).len(), 1);
    }

    #[test]
    fn talk_with_no_pause_is_cut_inside_one_window_and_nothing_is_lost() {
        let sound = [quiet(1.0), spoken(70.0)].concat();
        let takes = cut(&sound);
        assert!(takes.len() >= 5, "{}", takes.len());
        assert!(takes.iter().all(|take| take.samples.len() < WINDOW));
        assert!(takes.iter().all(|take| seconds(take.samples.len()) <= 14.0));
        let heard: usize = takes.iter().map(|take| take.samples.len()).sum();
        assert!(seconds(heard) >= 70.0, "{}", seconds(heard));
        for pair in takes.windows(2) {
            assert_eq!(pair[0].start + pair[0].samples.len(), pair[1].start);
        }
    }

    #[test]
    fn loud_noise_with_no_voice_in_it_makes_no_take() {
        let sounds = [
            ("white", noise(1.0, 0.1, false)),
            ("brown", noise(1.0, 0.1, true)),
            ("a breath", breath(1.0, 0.1)),
            ("a breath through the throat", throat(1.0, 0.1)),
            // Its frames are voiced now and then, so a long one must not
            // be heard just by lasting.
            ("a long breath through the throat", throat(3.0, 0.1)),
        ];
        for (name, loud) in sounds {
            let sound = [quiet(2.0), loud, quiet(2.0)].concat();
            assert_eq!(cut(&sound), Vec::new(), "{name}");
        }
    }

    #[test]
    fn a_low_voice_and_a_high_one_are_each_a_take() {
        for pitch in [65.0, 380.0] {
            let sound = [quiet(2.0), voice(0.4, pitch), quiet(2.0)].concat();
            assert_eq!(cut(&sound).len(), 1, "pitch: {pitch}");
        }
    }

    #[test]
    fn a_low_voice_on_a_hissy_line_is_a_take() {
        // The hiss is 10 dB under the voice, as a far end's noisy
        // microphone or a laptop's fan puts it.
        let hissy: Vec<f32> = voice(0.4, 85.0)
            .iter()
            .zip(noise(0.4, 0.014, false))
            .map(|(voice, hiss)| voice + hiss)
            .collect();
        let sound = [quiet(2.0), hissy, quiet(2.0)].concat();
        assert_eq!(cut(&sound).len(), 1);
    }

    #[test]
    fn a_voice_riding_on_a_drift_is_a_take() {
        // The drift is three times the voice, as a breath into the
        // microphone or a draught carries it: read as it comes, it is
        // alike to itself at every lag and hides the voice's repeat.
        let drifting: Vec<f32> = voice(0.4, 85.0)
            .iter()
            .zip(noise(0.4, 0.13, true))
            .map(|(voice, drift)| voice + drift)
            .collect();
        let sound = [quiet(2.0), drifting, quiet(2.0)].concat();
        assert_eq!(cut(&sound).len(), 1);
    }

    #[test]
    fn a_rumble_below_a_voice_makes_no_take() {
        let step = std::f32::consts::TAU * 45.0 / 16_000.0;
        let sound = [quiet(2.0), tone(1.0, 0.1, step), quiet(2.0)].concat();
        assert_eq!(cut(&sound), Vec::new());
    }

    #[test]
    fn noise_takes_three_frames_voiced_with_the_one_before_to_be_heard() {
        // Each frame is read with the one before it, so 60 ms of voice
        // after noise holds two voiced frames and 80 ms holds three. The
        // take is short enough that one voiced frame in twenty loud ones
        // asks for only two, so the three are what's tested.
        for (heard, seconds) in [(false, 0.06), (true, 0.08)] {
            let sound = [
                quiet(2.0),
                noise(0.2, 0.05, false),
                voice(seconds, 150.0),
                noise(0.2, 0.05, false),
                quiet(2.0),
            ]
            .concat();
            assert_eq!(
                cut(&sound).len(),
                usize::from(heard),
                "{seconds} s of voice"
            );
        }
    }

    #[test]
    fn a_short_answer_in_a_long_loud_take_is_a_take() {
        // 0.3 s of voice in 4 s of loud noise is one frame in sixteen.
        let sound = [
            quiet(2.0),
            noise(2.0, 0.05, false),
            voice(0.3, 150.0),
            noise(2.0, 0.05, false),
            quiet(2.0),
        ]
        .concat();
        assert_eq!(cut(&sound).len(), 1, "the answer in the noise");
    }

    #[test]
    fn a_short_answer_after_a_line_sent_nothing_is_a_take() {
        // A call sends nothing while its far end is muted, so the line's
        // hiss comes back only with the answer, and the answer must not
        // be lost in it.
        let nothing = vec![0.0; RATE * 3];
        let sound = [
            nothing,
            noise(3.5, 0.005, false),
            voice(0.3, 150.0),
            noise(3.5, 0.005, false),
            quiet(2.0),
        ]
        .concat();
        assert_eq!(cut(&sound).len(), 1, "the answer in the hiss");
    }

    #[test]
    fn quiet_alone_and_a_click_make_no_take() {
        let sound = [quiet(5.0), spoken(0.1), quiet(5.0)].concat();
        assert_eq!(cut(&sound), Vec::new());
    }
}
