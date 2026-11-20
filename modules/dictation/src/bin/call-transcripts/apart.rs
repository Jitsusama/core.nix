//! How well call-transcripts tells the far end's voices apart, measured on
//! the set score.rs hears (meetings.nix lays it out): each meeting's far
//! end cut into takes as a call's is, each take embedded and the takes
//! grouped as hear.rs groups them (`dictation::voices`), with no count of
//! the people on it, as on a call with no invitation, then held to its
//! reference.
//!
//! Each voice stands for the speaker it holds most of, no two voices for
//! one speaker, taken greedily from the largest share down. Two numbers
//! are kept: of the far end's speech in takes given a voice, how much is
//! its voice's speaker's, so a voice that mixes two people loses; and how
//! much of the far end's speech is in takes given a voice at all, so
//! naming less to be surer loses too.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use dictation::transcript::Side;
use dictation::voices::{self, Heard, Voices};

use crate::score::{self, Said, index, meetings, percent, samples, whose};
use crate::takes;

/// A take, as the reference has it: the voice it was given, and how many
/// of its samples each speaker of theirs is speaking in.
struct Take {
    voice: Option<usize>,
    speakers: HashMap<String, usize>,
}

/// A meeting's voices, or a set's, held to its reference, in samples of
/// speech.
#[derive(Default, Clone, Copy)]
struct Told {
    /// In a take given a voice, and its voice's speaker's.
    right: usize,
    /// In a take given a voice.
    named: usize,
    /// In any take.
    speech: usize,
    voices: usize,
    speakers: usize,
}

impl Told {
    fn add(&mut self, other: Self) {
        self.right += other.right;
        self.named += other.named;
        self.speech += other.speech;
        self.voices += other.voices;
        self.speakers += other.speakers;
    }

    /// Of the speech named, the tenths of a percent named right.
    fn right_tenths(&self) -> usize {
        tenths(self.right, self.named)
    }

    /// Of the speech, the tenths of a percent named.
    fn named_tenths(&self) -> usize {
        tenths(self.named, self.speech)
    }
}

fn tenths(part: usize, whole: usize) -> usize {
    (part * 1000).checked_div(whole).unwrap_or(0)
}

/// Tells each meeting of `set` apart with the speaker model in `model`,
/// says how well for each and for the set, and fails when the set names
/// right less than `floors`' first, in tenths of a percent, of what it
/// names, or names less than its second of the speech.
pub fn apart(model: &Path, set: &Path, floors: Option<(usize, usize)>) -> Result<(), String> {
    let sides = score::read(&set.join("sides"))?;
    let meetings = meetings(set, &sides)?;
    let mut speaker = Voices::load(model)?;
    let mut all = Told::default();
    for meeting in &meetings {
        let sound = &meeting.sounds[index(Side::Theirs)];
        let mut heard = Vec::new();
        let mut cut = Vec::new();
        for take in takes(sound) {
            let end = take.start + take.samples.len();
            let speakers = spoken(take.start, end, &meeting.reference, meeting.mine);
            if let Some(embedding) = speaker.embed(&take.samples)? {
                heard.push(Heard {
                    embedding,
                    samples: take.samples.len(),
                });
                cut.push(speakers);
            }
        }
        let takes: Vec<Take> = voices::group(&heard, None)
            .into_iter()
            .zip(cut)
            .map(|(voice, speakers)| Take { voice, speakers })
            .collect();
        let told = tell(&takes);
        println!("{}", report(meeting.name, &told));
        all.add(told);
    }
    println!("{}", report("all", &all));
    if all.speech == 0 {
        return Err(format!(
            "{} has no far-end speech to tell apart",
            set.display()
        ));
    }
    let Some((right, named)) = floors else {
        return Ok(());
    };
    let (got_right, got_named) = (all.right_tenths(), all.named_tenths());
    let said = format!(
        "it named {} of what it named right, and named {} of the speech",
        percent(got_right),
        percent(got_named)
    );
    if got_right < right || got_named < named {
        Err(format!(
            "{said}, short of the {} and {} it must",
            percent(right),
            percent(named)
        ))
    } else {
        println!(
            "{said}, within the {} and {} it must",
            percent(right),
            percent(named)
        );
        Ok(())
    }
}

/// How many samples of `start` to `end` each speaker on the far end
/// speaks in, by the reference.
fn spoken(start: usize, end: usize, reference: &[Said], mine: &str) -> HashMap<String, usize> {
    let mut speakers = HashMap::new();
    for line in reference
        .iter()
        .filter(|line| whose(line, mine) == Side::Theirs)
    {
        let (from, to) = (samples(line.start).max(start), samples(line.end).min(end));
        if from < to {
            *speakers.entry(line.speaker.clone()).or_default() += to - from;
        }
    }
    speakers
}

/// The takes' voices held to their speakers: each voice standing for one
/// speaker, the pairs sharing most speech matched first.
fn tell(takes: &[Take]) -> Told {
    let mut shared: HashMap<(usize, &str), usize> = HashMap::new();
    let mut told = Told::default();
    let mut speakers: Vec<&str> = Vec::new();
    for take in takes {
        let speech: usize = take.speakers.values().sum();
        told.speech += speech;
        for name in take.speakers.keys() {
            if !speakers.contains(&name.as_str()) {
                speakers.push(name);
            }
        }
        let Some(voice) = take.voice else {
            continue;
        };
        told.named += speech;
        for (name, &samples) in &take.speakers {
            *shared.entry((voice, name)).or_default() += samples;
        }
    }
    let mut pairs: Vec<((usize, &str), usize)> = shared.into_iter().collect();
    // Ties go to the lower voice and the first speaker by name, so a
    // score never changes between runs.
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let (mut voices, mut matched): (Vec<usize>, Vec<&str>) = (Vec::new(), Vec::new());
    for ((voice, name), samples) in pairs {
        if !voices.contains(&voice) && !matched.contains(&name) {
            voices.push(voice);
            matched.push(name);
            told.right += samples;
        }
    }
    told.voices = takes
        .iter()
        .filter_map(|take| take.voice)
        .max()
        .map_or(0, |most| most + 1);
    told.speakers = speakers.len();
    told
}

/// How well a meeting's voices were told apart, in a line.
fn report(name: &str, told: &Told) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{name}: {} voices for {} speakers, {} of what was named named right, {} of the \
         speech named",
        told.voices,
        told.speakers,
        percent(told.right_tenths()),
        percent(told.named_tenths()),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(voice: Option<usize>, speakers: &[(&str, usize)]) -> Take {
        Take {
            voice,
            speakers: speakers.iter().map(|&(n, s)| (n.to_owned(), s)).collect(),
        }
    }

    #[test]
    fn a_voice_mixing_two_speakers_is_right_only_for_the_one_it_holds_most_of() {
        let told = tell(&[
            take(Some(0), &[("B", 80), ("C", 20)]),
            take(Some(1), &[("D", 100)]),
        ]);
        assert_eq!(
            (told.right, told.named, told.speech),
            (180, 200, 200),
            "the speech named right, named, and all of it, are wrong"
        );
    }

    #[test]
    fn two_voices_for_one_speaker_are_right_only_once() {
        let told = tell(&[take(Some(0), &[("B", 60)]), take(Some(1), &[("B", 40)])]);
        assert_eq!(
            told.right, 60,
            "a speaker split into two voices was counted right for both"
        );
        assert_eq!(
            (told.voices, told.speakers),
            (2, 1),
            "voices or speakers miscounted"
        );
    }

    #[test]
    fn a_take_given_no_voice_is_speech_but_not_named() {
        let told = tell(&[take(Some(0), &[("B", 50)]), take(None, &[("C", 50)])]);
        assert_eq!(
            (told.right_tenths(), told.named_tenths()),
            (1000, 500),
            "a take under Others counted as named, or as wrongly named"
        );
    }

    #[test]
    fn a_take_of_nobody_by_the_reference_is_no_speech() {
        let reference = [Said {
            start: 1.0,
            end: 2.0,
            speaker: "B".to_owned(),
            text: String::new(),
        }];
        let speakers = spoken(0, samples(1.5), &reference, "A");
        assert_eq!(
            speakers.get("B"),
            Some(&samples(0.5)),
            "only the half second the take shares with B's line is B's"
        );
        assert!(
            spoken(samples(3.0), samples(4.0), &reference, "A").is_empty(),
            "a take after every line was given speech"
        );
    }
}
