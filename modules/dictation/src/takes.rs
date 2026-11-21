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
const _: () = assert!(
    LONGEST_FRAMES * FRAME < WINDOW,
    "the longest take must fit inside Parakeet's window"
);
/// The longest a take runs, in samples.
pub const LONGEST: usize = LONGEST_FRAMES * FRAME;
/// Where a take that long is cut: at its quietest 20 ms in its last few
/// seconds.
const CUT_WITHIN_FRAMES: usize = 5 * 50;
/// Less speech than this, a cough or a click, isn't a take.
const LEAST_SPEECH_FRAMES: usize = 15;
/// How far back the floor is judged from: ten seconds of the line's
/// sound, so lost packets, which hold nothing, stretch it a little.
const FLOOR_FRAMES: usize = 10 * 50;
/// How far above the floor speech is.
const ABOVE_FLOOR_DB: f32 = 12.0;
/// Quieter than this is never speech, however quiet the floor.
const QUIETEST_SPEECH_DB: f32 = -55.0;
/// Quieter than this is a line sending nothing, as a call does while
/// its far end is muted, between words its far end gates out, or for a
/// packet it lost. The nothing sets no floor, nor does a frame holding
/// this many samples in a row each quieter than it, a millisecond: the
/// frame a mute starts or ends in, or a packet lost part of the way into
/// it, holds only some of the line's sound, and would set a floor under
/// its hiss. A sample is judged by its size, so a mute sent as a faint
/// dither is nothing as exact zeros are.
const NOTHING_DB: f32 = -90.0;
const NOTHING_SAMPLES: usize = RATE / 1000;
/// `NOTHING_DB` as the size of a sample.
const NOTHING_SAMPLE: f32 = 3.16e-5;
/// Nothing this long, 0.4 s, is a mute, across which the line may change,
/// so it clears the floor: the quiet from before it mustn't make the
/// line's hiss, back with the next word, read as loud. Less is packets
/// lost, across which the line holds, so the floor does. It's under the
/// half second `follow` fills a lost link with, which straddling frames
/// may leave only 24 of whole.
const MUTE_FRAMES: usize = 20;
/// A line that has sent nothing within the floor's ten seconds may gate
/// its words, sending nothing between them, so its floor may be its words
/// alone, which would be judged against themselves: on it, any sound that
/// could be speech is. Once the floor holds this long a run of the line's
/// own sound, too quiet to be speech or with no voice in it, as hiss or a
/// room is, the line isn't gated, and its floor is judged as any other's,
/// so a packet lost on a hissy line doesn't make its hiss speech. A fifth
/// of a second is longer than a word's unvoiced sounds.
const OWN_SOUND_FRAMES: usize = 10;
/// A run of the line's own sound loud enough to be speech holds within
/// this, from its quietest frame to its loudest, as a hiss or a fan does
/// and a laugh, a cough or a long "s" doesn't: each rises and falls.
/// A line's hiss wanders a decibel or two over a fifth of a second.
const STEADY_DB: f32 = 3.0;
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
    /// The last frames' loudness, for the floor, each with whether it
    /// ends a run of the line's own sound.
    recent: VecDeque<(f32, bool)>,
    /// The frames just before a take, so its start can be kept.
    before: VecDeque<Vec<f32>>,
    /// The take under way, frame by frame with each one's loudness.
    take: Vec<(Vec<f32>, f32, bool)>,
    /// The frame the take under way starts at.
    take_start: usize,
    /// Frames since the line last sent nothing, if it ever has.
    since_nothing: Option<usize>,
    /// Frames of nothing in a row, up to the one just fed.
    nothing_run: usize,
    /// Frames of the line's own sound in a row, up to the one just fed,
    /// and the quietest and loudest of them.
    own_run: usize,
    own_levels: (f32, f32),
    /// The frame fed before this one, which voicing is read with.
    last: Vec<f32>,
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
        if level <= NOTHING_DB {
            self.nothing_run += 1;
            if self.nothing_run >= MUTE_FRAMES {
                self.recent.clear();
            }
            self.since_nothing = Some(0);
            self.own_run = 0;
        } else {
            self.nothing_run = 0;
            if holds_nothing(&frame) {
                self.own_run = 0;
            } else {
                let own = level <= QUIETEST_SPEECH_DB || !voiced(&self.last, &frame);
                self.own_run = if own { self.own_sound(level) } else { 0 };
                self.recent
                    .push_back((level, self.own_run >= OWN_SOUND_FRAMES));
                if self.recent.len() > FLOOR_FRAMES {
                    self.recent.pop_front();
                }
            }
            self.since_nothing = self.since_nothing.map(|frames| frames + 1);
        }
        self.last.clone_from(&frame);
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
        // Voicing is asked again of a take loud for long enough, of its
        // loud frames, and only until enough are voiced: a few frames'
        // more reckoning, where keeping each frame's answer would carry
        // it through every take.
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

    /// Lengthens the run of the line's own sound by a frame at `level`,
    /// or starts it afresh there when the run is loud and doesn't hold
    /// steady, giving how long it now is.
    fn own_sound(&mut self, level: f32) -> usize {
        let (quietest, loudest) = if self.own_run == 0 {
            (level, level)
        } else {
            (self.own_levels.0.min(level), self.own_levels.1.max(level))
        };
        if loudest <= QUIETEST_SPEECH_DB || loudest - quietest <= STEADY_DB {
            self.own_levels = (quietest, loudest);
            self.own_run + 1
        } else {
            self.own_levels = (level, level);
            1
        }
    }

    fn threshold(&self) -> f32 {
        let gated = self
            .since_nothing
            .is_some_and(|frames| frames < FLOOR_FRAMES)
            && !self.recent.iter().any(|&(_, own)| own);
        if self.recent.is_empty() || gated {
            return QUIETEST_SPEECH_DB;
        }
        let floor = self
            .recent
            .iter()
            .map(|&(level, _)| level)
            .fold(f32::INFINITY, f32::min);
        (floor + ABOVE_FLOOR_DB).max(QUIETEST_SPEECH_DB)
    }
}

/// Whether a frame holds a run of nothing, `NOTHING_SAMPLES` long.
fn holds_nothing(frame: &[f32]) -> bool {
    let mut run = 0;
    frame.iter().any(|&sample| {
        run = if sample.abs() < NOTHING_SAMPLE {
            run + 1
        } else {
            0
        };
        run >= NOTHING_SAMPLES
    })
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
#[path = "takes_tests.rs"]
mod tests;
