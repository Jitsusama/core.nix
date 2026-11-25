//! Whether Parakeet's word times hold on real sound, which cutting the
//! far end's echo from Joel's words rests on (`dictation::echo`): each of
//! the far end's words the scorer hears (score.rs) is timed inside the
//! take it was heard in, no earlier than the word before it, and mostly
//! while the reference says one of theirs is speaking.

use std::collections::HashMap;
use std::ops::AddAssign;

use dictation::parakeet::{RATE, Word};
use dictation::transcript::{Line, Side};

use crate::score::percent;

/// The least share of the far end's words, in tenths of a percent, that
/// may be timed while the reference says one of theirs is speaking.
/// The clean meetings time 98.1% of 2,218 so (99.4%, 92.6% and 100.0%,
/// meeting by meeting), and none outside its take or before the word
/// ahead; this floor sits a tenth under, so a model or a split that
/// moves the far end's word times off their speech fails the check.
pub const INSIDE: usize = 980;

/// How the far end's words sat in time: how many were heard, how many
/// were timed outside their take, how many before the word ahead of them,
/// and how many while one of theirs was speaking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timed {
    pub words: usize,
    pub outside: usize,
    pub backward: usize,
    pub inside: usize,
}

impl AddAssign for Timed {
    fn add_assign(&mut self, other: Self) {
        self.words += other.words;
        self.outside += other.outside;
        self.backward += other.backward;
        self.inside += other.inside;
    }
}

impl Timed {
    /// The share of the words timed while one of theirs was speaking, in
    /// tenths of a percent, all of them when none were heard.
    pub fn tenths_inside(&self) -> usize {
        (self.inside * 1000).checked_div(self.words).unwrap_or(1000)
    }

    /// The words' times, said as the scorer prints them for `name`.
    pub fn describe(&self, name: &str) -> String {
        format!(
            "{name} the far end's word times: {} words, {} outside their take, \
             {} before the word ahead of them, {} while one of theirs speaks",
            self.words,
            self.outside,
            self.backward,
            percent(self.tenths_inside()),
        )
    }

    /// Whether the times hold: some timed, none outside its take or out of
    /// order, and at least `least` tenths of a percent while one of theirs
    /// speaks.
    pub fn verdict(&self, least: usize) -> Result<String, String> {
        let inside = percent(self.tenths_inside());
        let may = percent(least);
        if self.words == 0 {
            Err(String::from("none of the far end's words was timed"))
        } else if self.outside > 0 || self.backward > 0 {
            Err(format!(
                "{} of the far end's words were timed outside their take, {} out of order",
                self.outside, self.backward
            ))
        } else if self.tenths_inside() < least {
            Err(format!(
                "{inside} of the far end's words were timed while they spoke, below the {may} \
                 they must"
            ))
        } else {
            Ok(format!(
                "{inside} of the far end's words were timed while they spoke, at least the \
                 {may} they must"
            ))
        }
    }
}

/// How the far end's words in `timed`, kept by where their line falls in
/// `lines`, sit against their takes and `speech`, the spans in seconds in
/// which the reference says one of theirs speaks.
pub fn judge(lines: &[Line], timed: &HashMap<usize, Vec<Word>>, speech: &[(f64, f64)]) -> Timed {
    let mut judged = Timed::default();
    let theirs = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.side == Side::Theirs);
    for (i, line) in theirs {
        let mut last = None;
        for word in timed.get(&i).into_iter().flatten() {
            judged.words += 1;
            if !(line.start..line.end).contains(&word.at) {
                judged.outside += 1;
            }
            if last.is_some_and(|before| word.at < before) {
                judged.backward += 1;
            }
            last = last.max(Some(word.at));
            let at = seconds(word.at);
            if speech.iter().any(|&(from, to)| from <= at && at <= to) {
                judged.inside += 1;
            }
        }
    }
    judged
}

/// Samples into a side as seconds.
fn seconds(samples: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a meeting is minutes long, far under a float's 2^52 samples"
    )]
    let seconds = samples as f64 / RATE as f64;
    seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(side: Side, start: usize, end: usize) -> Line {
        Line {
            side,
            start,
            end,
            text: String::from("Said."),
            voice: None,
            name: None,
        }
    }

    fn at(seconds: f64) -> Word {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a case's times are a few seconds"
        )]
        let at = (seconds * 16_000.0) as usize;
        Word {
            text: String::from("said"),
            at,
            level: 0.0,
        }
    }

    #[test]
    fn the_far_ends_words_are_held_to_their_take_their_order_and_their_speech() {
        let lines = [
            line(Side::Theirs, 16_000, 64_000),
            line(Side::Mine, 0, 160_000),
            line(Side::Theirs, 80_000, 112_000),
        ];
        let timed = HashMap::from([
            (0, vec![at(1.0), at(2.5), at(2.0), at(4.5)]),
            (1, vec![at(0.5), at(9.0)]),
            (2, vec![at(5.0), at(6.5)]),
        ]);
        let speech = [(0.8, 2.6), (4.9, 7.0)];
        let judged = judge(&lines, &timed, &speech);
        assert_eq!(
            judged,
            Timed {
                words: 6,
                outside: 1,
                backward: 1,
                inside: 5,
            },
            "Joel's words are left out, the one at 4.5 s is past its take, the one at \
             2 s comes before the word ahead of it, and only the one at 4.5 s is \
             outside their speech"
        );
    }

    #[test]
    fn times_hold_only_in_their_take_in_order_and_mostly_while_they_speak() {
        let held = Timed {
            words: 10,
            outside: 0,
            backward: 0,
            inside: 9,
        };
        assert!(
            held.verdict(900).is_ok(),
            "nine in ten holds a floor of 90%"
        );
        assert!(
            held.verdict(901).is_err(),
            "nine in ten fails a floor of 90.1%"
        );
        let outside = Timed { outside: 1, ..held };
        assert!(outside.verdict(0).is_err(), "a word outside its take fails");
        let backward = Timed {
            backward: 1,
            ..held
        };
        assert!(backward.verdict(0).is_err(), "a word out of order fails");
        assert!(
            Timed::default().verdict(0).is_err(),
            "no word timed fails, since a model that times none would otherwise pass"
        );
    }
}
