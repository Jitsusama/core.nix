//! How well call-transcripts tells the far end's voices apart, measured on
//! the set score.rs hears (meetings.nix lays it out): each meeting's far
//! end cut into takes as a call's is, each take embedded and the takes
//! grouped as hear.rs groups them (`dictation::voices`), with no count of
//! the people on it, as on a call with no invitation, then held to its
//! reference. A take the reference has far-end speech in stands for one
//! Parakeet would hear words in, and one too short or odd to embed is
//! speech under Others, as on a call.
//!
//! Each voice stands for the speaker it holds most of, no two voices for
//! one speaker, taken greedily from the largest share down. Two numbers
//! are kept: of the far end's speech in takes given a voice, how much is
//! its voice's speaker's, so a voice that mixes two people loses; and how
//! much of the far end's speech is handled as it should be, so naming
//! less to be surer loses too. Where one speaker holds all but less than
//! `LEAST_SHARE` of a far end's speech there's nobody to tell them from,
//! so its takes should stay under Others, as voices.rs leaves them: there
//! a take left so is handled right, and one given a voice is wrong in
//! both numbers. Elsewhere a take is handled right when given a voice.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use dictation::transcript::Side;
use dictation::voices::{self, Heard, LEAST_SHARE, Voices};

use crate::score::{self, Meeting, Said, index, meetings, percent, samples, whose};
use crate::takes;

/// A take of a meeting's far end, cut as a call's is: where it starts and
/// how many samples it holds, which of the takes embedded it is, none for
/// one too short or odd to embed, and how many of its samples each
/// speaker of theirs is speaking in, by the reference.
pub struct Cut {
    pub start: usize,
    pub samples: usize,
    pub heard: Option<usize>,
    pub speakers: HashMap<String, usize>,
}

/// A meeting's far end cut into takes as a call's is, each embedded with
/// `speaker` where it can be: the embeddings, and the takes.
pub fn cut(meeting: &Meeting, speaker: &mut Voices) -> Result<(Vec<Heard>, Vec<Cut>), String> {
    let sound = &meeting.sounds[index(Side::Theirs)];
    let mut heard = Vec::new();
    let mut cuts = Vec::new();
    for take in takes(sound) {
        let end = take.start + take.samples.len();
        let speakers = spoken(take.start, end, &meeting.reference, meeting.mine);
        let at = speaker.embed(&take.samples)?.map(|embedding| {
            heard.push(Heard {
                embedding,
                samples: take.samples.len(),
                worded: !speakers.is_empty(),
            });
            heard.len() - 1
        });
        cuts.push(Cut {
            start: take.start,
            samples: take.samples.len(),
            heard: at,
            speakers,
        });
    }
    Ok((heard, cuts))
}

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
    /// In a take given a voice, and its voice's speaker's, where its far
    /// end has more than one.
    right: usize,
    /// In a take given a voice.
    named: usize,
    /// In a take handled as it should be: given a voice, or left under
    /// Others where the far end is one speaker.
    handled: usize,
    /// In any take.
    speech: usize,
    voices: usize,
    speakers: usize,
    /// Whether the far end is one speaker, all but too little to name.
    alone: bool,
}

impl Told {
    fn add(&mut self, other: Self) {
        self.right += other.right;
        self.named += other.named;
        self.handled += other.handled;
        self.speech += other.speech;
        self.voices += other.voices;
        self.speakers += other.speakers;
    }

    /// Of the speech named, the tenths of a percent named right.
    fn right_tenths(&self) -> usize {
        tenths(self.right, self.named)
    }

    /// Of the speech, the tenths of a percent handled as it should be.
    fn handled_tenths(&self) -> usize {
        tenths(self.handled, self.speech)
    }
}

fn tenths(part: usize, whole: usize) -> usize {
    (part * 1000).checked_div(whole).unwrap_or(0)
}

/// Tells each meeting of `set` apart with the speaker model in `model`,
/// says how well for each and for the set, and fails when the set names
/// right less than `floors`' first, in tenths of a percent, of what it
/// names, or handles less than its second of the speech as it should.
pub fn apart(model: &Path, set: &Path, floors: Option<(usize, usize)>) -> Result<(), String> {
    let sides = score::read(&set.join("sides"))?;
    let meetings = meetings(set, &sides)?;
    let mut speaker = Voices::load(model, None)?;
    let mut all = Told::default();
    for meeting in &meetings {
        let (heard, cuts) = cut(meeting, &mut speaker)?;
        println!("{}", shares(meeting.name, &heard, &cuts));
        let grouped = voices::group(&heard, None);
        let takes: Vec<Take> = cuts
            .into_iter()
            .map(|cut| Take {
                voice: cut.heard.and_then(|at| grouped[at]),
                speakers: cut.speakers,
            })
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
    let Some((right, handled)) = floors else {
        return Ok(());
    };
    let (got_right, got_handled) = (all.right_tenths(), all.handled_tenths());
    let said = format!(
        "of what it named, {} was named right, and {} of the speech was handled as it should \
         be",
        percent(got_right),
        percent(got_handled)
    );
    if got_right < right || got_handled < handled {
        Err(format!(
            "{said}, short of the {} and {} it must",
            percent(right),
            percent(handled)
        ))
    } else {
        println!(
            "{said}, within the {} and {} it must",
            percent(right),
            percent(handled)
        );
        Ok(())
    }
}

/// How a meeting's takes were grouped, the six largest groups' shares of
/// the takes' samples, beside each speaker's share of the far end's speech
/// by the reference, so a voice not named can be seen to be one too small
/// or one with too little beside it.
fn shares(name: &str, heard: &[Heard], cuts: &[Cut]) -> String {
    let all: usize = heard.iter().map(|h| h.samples).sum();
    let groups: Vec<String> = voices::groups(heard)
        .iter()
        .take(6)
        .map(|g| percent(tenths(g.iter().map(|&i| heard[i].samples).sum(), all)))
        .collect();
    let mut speakers: Vec<(&str, usize)> = Vec::new();
    for cut in cuts {
        for (name, &samples) in &cut.speakers {
            match speakers.iter_mut().find(|(n, _)| n == name) {
                Some((_, sum)) => *sum += samples,
                None => speakers.push((name, samples)),
            }
        }
    }
    speakers.sort_unstable();
    let speech: usize = speakers.iter().map(|(_, s)| s).sum();
    let speakers: Vec<String> = speakers
        .iter()
        .map(|(name, s)| format!("{name} {}", percent(tenths(*s, speech))))
        .collect();
    format!(
        "{name}: its largest groups hold {} of the takes; the reference gives {}",
        groups.join(", "),
        speakers.join(", ")
    )
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
/// speaker, the pairs sharing most speech matched first, unless the far
/// end is one speaker, when no voice is right.
fn tell(takes: &[Take]) -> Told {
    let mut shared: HashMap<(usize, &str), usize> = HashMap::new();
    let mut told = Told::default();
    let mut speakers: Vec<(&str, usize)> = Vec::new();
    for take in takes {
        let speech: usize = take.speakers.values().sum();
        told.speech += speech;
        for (name, &samples) in &take.speakers {
            match speakers.iter_mut().find(|(n, _)| n == name) {
                Some((_, sum)) => *sum += samples,
                None => speakers.push((name, samples)),
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
    told.speakers = speakers.len();
    told.voices = takes
        .iter()
        .filter_map(|take| take.voice)
        .max()
        .map_or(0, |most| most + 1);
    let most = speakers.iter().map(|&(_, s)| s).max().unwrap_or(0);
    told.alone = told.speech > 0 && as_f64(told.speech - most) < LEAST_SHARE * as_f64(told.speech);
    if told.alone {
        told.handled = told.speech - told.named;
        return told;
    }
    told.handled = told.named;
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
    told
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// How well a meeting's voices were told apart, in a line.
fn report(name: &str, told: &Told) -> String {
    let mut out = String::new();
    let alone = if told.alone { ", one in effect" } else { "" };
    let _ = write!(
        out,
        "{name}: {} voices for {} speakers{alone}, of what was named {} named right, {} of the \
         speech handled as it should be",
        told.voices,
        told.speakers,
        percent(told.right_tenths()),
        percent(told.handled_tenths()),
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
        let told = tell(&[
            take(Some(0), &[("B", 60)]),
            take(Some(1), &[("B", 40)]),
            take(None, &[("C", 50)]),
        ]);
        assert_eq!(
            told.right, 60,
            "a speaker split into two voices was counted right for both"
        );
        assert_eq!(
            (told.voices, told.speakers),
            (2, 2),
            "voices or speakers miscounted"
        );
    }

    #[test]
    fn a_take_given_no_voice_is_speech_but_not_named() {
        let told = tell(&[take(Some(0), &[("B", 50)]), take(None, &[("C", 50)])]);
        assert_eq!(
            (told.right_tenths(), told.handled_tenths()),
            (1000, 500),
            "a take under Others counted as handled, or as wrongly named"
        );
    }

    #[test]
    fn a_far_end_of_one_speaker_is_handled_by_naming_nobody() {
        let left = tell(&[take(None, &[("B", 95)]), take(None, &[("C", 5)])]);
        assert_eq!(
            (left.handled_tenths(), left.named),
            (1000, 0),
            "a one-speaker far end left under Others wasn't counted as handled"
        );
        let named = tell(&[take(Some(0), &[("B", 95)]), take(None, &[("C", 5)])]);
        assert_eq!(
            (named.right_tenths(), named.handled_tenths()),
            (0, 50),
            "naming a one-speaker far end's voice scored better than leaving it"
        );
    }

    #[test]
    fn a_take_holds_only_the_far_ends_speech_it_overlaps() {
        let line = |start, end, speaker: &str| Said {
            start,
            end,
            speaker: speaker.to_owned(),
            text: String::new(),
        };
        let reference = [line(1.0, 2.0, "B"), line(0.0, 2.0, "A")];
        let speakers = spoken(0, samples(1.5), &reference, "A");
        assert_eq!(
            speakers.get("B"),
            Some(&samples(0.5)),
            "only the half second the take shares with B's line is B's"
        );
        assert!(
            !speakers.contains_key("A"),
            "Joel's speaker was counted in the far end's speech"
        );
        assert!(
            spoken(samples(3.0), samples(4.0), &reference, "A").is_empty(),
            "a take after every line was given speech"
        );
    }
}
