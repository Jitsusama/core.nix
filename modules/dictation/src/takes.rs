//! One side of a call cut into takes Whisper can hear: each under its 30 s
//! window, cut where the speaker pauses so no word is split, and only where
//! someone spoke, since Whisper given silence or hum writes words nobody
//! said.
//!
//! Speech is told from the quiet by loudness against the side's own floor,
//! its quietest 20 ms of the last ten seconds, so a fan or a far end's
//! comfort noise counts as quiet wherever it sits. Even unbroken talk
//! falls near the floor at its stops and breaths, so talking long doesn't
//! raise it to the talker's own level, while a louder room raises it
//! within the ten seconds. It's fed sound as it
//! comes, so a live call's takes are ready as each pause ends.

use std::collections::VecDeque;

use crate::mel::RATE;

/// The loudness of each 20 ms is judged once.
const FRAME: usize = RATE / 50;
/// A pause this long ends a take: about the gap between sentences.
const PAUSE_FRAMES: usize = 25;
/// What's kept before speech starts and after it ends, so a word's soft
/// start and end stay in.
const EDGE_FRAMES: usize = 10;
/// A take is cut by this length at the latest, under Whisper's 30 s.
const LONGEST_FRAMES: usize = 28 * 50;
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
        self.recent.push_back(level);
        if self.recent.len() > FLOOR_FRAMES {
            self.recent.pop_front();
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
        (spoken >= LEAST_SPEECH_FRAMES).then(|| Take {
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
    fn talk_with_no_pause_is_cut_under_whispers_window_and_nothing_is_lost() {
        let sound = [quiet(1.0), spoken(70.0)].concat();
        let takes = cut(&sound);
        assert!(takes.len() >= 3, "{}", takes.len());
        assert!(takes.iter().all(|take| seconds(take.samples.len()) <= 28.0));
        let heard: usize = takes.iter().map(|take| take.samples.len()).sum();
        assert!(seconds(heard) >= 70.0, "{}", seconds(heard));
        for pair in takes.windows(2) {
            assert_eq!(pair[0].start + pair[0].samples.len(), pair[1].start);
        }
    }

    #[test]
    fn quiet_alone_and_a_click_make_no_take() {
        let sound = [quiet(5.0), spoken(0.1), quiet(5.0)].concat();
        assert_eq!(cut(&sound), Vec::new());
    }
}
