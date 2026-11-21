//! How well call-transcripts names the far end's voices from who was in
//! the Meet as each spoke (presence.rs), measured on the set score.rs
//! hears. Its meetings were held in one room, so nobody joined them: here
//! the far end's speakers join one at a time, in the order they first
//! spoke, the first at the start and each next once a like share of the
//! far end's speech has been said, since a meeting's far end may say
//! nothing for minutes. A take where anyone not yet joined holds
//! `LEAST_SHARE` or more of the speech is left out, as no call would hold
//! it. A take where they only murmur under another, as one does through
//! most of a meeting held in a room, is kept, since the meetings' sound
//! can't be pulled apart by speaker, and leaving out every such take
//! leaves out nearly all; it makes the voices harder to tell apart, never
//! easier. The takes left are grouped as hear.rs groups a call's whose
//! Meet says how many were on it, and each voice is named as presence.rs
//! names it.
//!
//! Two numbers are kept: of the far end's speech in takes given a name,
//! sure or likely, how much is that person's, so a wrong name loses; and
//! of the far end's speech in the takes kept, how much is given a name, so
//! naming nobody to be sure loses too. A voice given the name of anyone
//! but whoever holds most of it fails the check outright, since that is
//! a wrong name rather than another's words under a right one.
//!
//! It measures the rule where it can name: everyone who joins speaks,
//! nobody leaves, the clocks agree and grouping is told how many there
//! are. A silent guest, a voice split in two, two at one seat or a stale
//! leave each name fewer or wrongly, and nothing here measures them.

use std::collections::HashMap;

use dictation::pieces;
use dictation::voices::{self, Heard, LEAST_SHARE, Voices};

use crate::apart::{self, Cut};
use crate::presence::{self, Present, Spoke, Who};
use crate::score::{self, meetings, percent};

/// A meeting's names, or a set's, held to its reference, in samples of
/// speech.
#[derive(Default, Clone, Copy)]
struct Names {
    /// In a take given a name, and that person's.
    right: usize,
    /// In a take given a name.
    named: usize,
    /// In any take.
    speech: usize,
    /// Voices given the name of anyone but whoever holds most of them.
    wrong: usize,
}

impl Names {
    fn add(&mut self, other: Self) {
        self.right += other.right;
        self.named += other.named;
        self.speech += other.speech;
        self.wrong += other.wrong;
    }

    /// Of the speech given a name, the tenths of a percent named right.
    fn right_tenths(&self) -> usize {
        tenths(self.right, self.named)
    }

    /// Of the speech, the tenths of a percent given a name.
    fn named_tenths(&self) -> usize {
        tenths(self.named, self.speech)
    }
}

fn tenths(part: usize, whole: usize) -> usize {
    (part * 1000).checked_div(whole).unwrap_or(0)
}

/// Names each meeting's voices in `set`, told apart with the speaker model
/// in `model`, says how well for each and for the set, and fails when the
/// set names right less than `floors`' first, in tenths of a percent, of
/// what it names, or names less than its second of the speech, or gives
/// any voice a name not its own.
pub fn names(
    model: &std::path::Path,
    set: &std::path::Path,
    floors: Option<(usize, usize)>,
) -> Result<(), String> {
    let sides = score::read(&set.join("sides"))?;
    let meetings = meetings(set, &sides)?;
    let mut speaker = Voices::load(model, None)?;
    let mut all = Names::default();
    for meeting in &meetings {
        let (heard, cuts) = apart::cut(meeting, &mut speaker)?;
        let (named, voices) = name(&heard, &cuts);
        println!("{}: {voices}", meeting.name);
        println!("{}", report(meeting.name, &named));
        all.add(named);
    }
    println!("{}", report("all", &all));
    if all.speech == 0 {
        return Err(format!("{} has no far-end speech to name", set.display()));
    }
    let Some((right, named)) = floors else {
        return Ok(());
    };
    if all.wrong > 0 {
        return Err(format!(
            "{} voices were given the name of someone who isn't most of what they hold",
            all.wrong
        ));
    }
    let (got_right, got_named) = (all.right_tenths(), all.named_tenths());
    let said = format!(
        "of what it named, {} was named right, and {} of the speech was given a name",
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

/// A meeting's far end, `heard` and cut into `cuts`,
/// joined one speaker at a time, its voices named and held to the
/// reference; and who each voice was taken to be beside whose speech it
/// holds, so a meeting naming little can be seen to.
fn name(heard: &[Heard], cuts: &[Cut]) -> (Names, String) {
    let joins = joins(cuts);
    let kept: Vec<&Cut> = cuts.iter().filter(|cut| !before(cut, &joins)).collect();
    // The takes kept, as hear.rs has a call's, and which of them each is.
    let mut grouped_heard = Vec::new();
    let mut at = Vec::new();
    for cut in &kept {
        at.push(cut.heard.map(|was| {
            let take = &heard[was];
            grouped_heard.push(Heard {
                embedding: take.embedding.clone(),
                samples: take.samples,
                worded: take.worded,
            });
            grouped_heard.len() - 1
        }));
    }
    let grouped = voices::group(&grouped_heard, Some(joins.len()));
    let as_grouped: Vec<Option<usize>> =
        at.iter().map(|at| at.and_then(|at| grouped[at])).collect();
    let of: Vec<usize> = kept.iter().map(|cut| cut.take).collect();
    let voice_of = pieces::within_takes(&of, &as_grouped);
    let spoke: Vec<Spoke> = kept
        .iter()
        .zip(&voice_of)
        .filter(|(cut, _)| !cut.speakers.is_empty())
        .filter_map(|(cut, voice)| {
            Some(Spoke {
                voice: (*voice)?,
                from: millis(cut.start),
                to: millis(cut.start + cut.samples),
            })
        })
        .collect();
    let mut present: Vec<Present> = joins
        .iter()
        .map(|(name, &joined)| Present {
            name: name.clone(),
            joined: millis(joined),
            left: None,
        })
        .collect();
    present.sort_by(|a, b| a.joined.cmp(&b.joined).then_with(|| a.name.cmp(&b.name)));
    let voices = voice_of.iter().flatten().max().map_or(0, |most| most + 1);
    let who = presence::who(&present, &spoke, voices);
    let mut named = Names::default();
    let mut held: Vec<HashMap<&str, usize>> = vec![HashMap::new(); voices];
    for (cut, voice) in kept.iter().zip(&voice_of) {
        let speech: usize = cut.speakers.values().sum();
        named.speech += speech;
        if let Some(voice) = *voice {
            for (speaker, &samples) in &cut.speakers {
                *held[voice].entry(speaker.as_str()).or_default() += samples;
            }
        }
        if let Some(name) = voice.and_then(|voice| given(&who[voice])) {
            named.named += speech;
            named.right += cut.speakers.get(name).copied().unwrap_or(0);
        }
    }
    named.wrong = wrong(&who, &held);
    let mut first: Vec<Option<usize>> = vec![None; voices];
    for (cut, voice) in kept.iter().zip(&voice_of) {
        if let Some(voice) = voice.filter(|_| !cut.speakers.is_empty()) {
            first[voice].get_or_insert(cut.start);
        }
    }
    let came: Vec<String> = present
        .iter()
        .map(|person| {
            let at = joins[&person.name];
            format!("{} at {}", person.name, dictation::transcript::clock(at))
        })
        .collect();
    let said = format!(
        "joined {}, of {} takes {} kept; {}",
        came.join(", "),
        cuts.len(),
        kept.len(),
        described(&who, &held, &first)
    );
    (named, said)
}

/// How many voices were given the name of anyone but whoever `held` most
/// of them.
fn wrong(who: &[Who], held: &[HashMap<&str, usize>]) -> usize {
    who.iter()
        .zip(held)
        .filter(|(who, held)| {
            given(who).is_some_and(|name| {
                let most = held
                    .iter()
                    .max_by_key(|(speaker, samples)| (**samples, **speaker));
                most.is_none_or(|(speaker, _)| *speaker != name)
            })
        })
        .count()
}

/// The name a voice was given, sure or likely.
fn given(who: &Who) -> Option<&str> {
    match who {
        Who::Is(name) | Who::Likely(name) => Some(name),
        Who::MayBe(_) => None,
    }
}

/// Whether `cut` is said by someone before they joined: `LEAST_SHARE` or
/// more of its speech is theirs.
fn before(cut: &Cut, joins: &HashMap<String, usize>) -> bool {
    let speech: usize = cut.speakers.values().sum();
    let early: usize = cut
        .speakers
        .iter()
        .filter(|(speaker, _)| joins[*speaker] > cut.start)
        .map(|(_, &samples)| samples)
        .sum();
    speech > 0 && as_f64(early) >= LEAST_SHARE * as_f64(speech)
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Who each voice was taken to be, and whose speech it holds, by name.
fn described(who: &[Who], held: &[HashMap<&str, usize>], first: &[Option<usize>]) -> String {
    if who.is_empty() {
        return "no voices".to_owned();
    }
    let voices: Vec<String> = who
        .iter()
        .zip(held)
        .enumerate()
        .map(|(at, (who, held))| {
            let taken = match who {
                Who::Is(name) => format!("is {name}"),
                Who::Likely(name) => format!("is likely {name}"),
                Who::MayBe(names) if names.is_empty() => "may be nobody".to_owned(),
                Who::MayBe(names) => format!("may be {}", names.join(" or ")),
            };
            let all: usize = held.values().sum();
            let mut speakers: Vec<(&str, usize)> = held.iter().map(|(n, &s)| (*n, s)).collect();
            speakers.sort_unstable();
            let holds: Vec<String> = speakers
                .iter()
                .map(|(name, samples)| format!("{name} {}", percent(tenths(*samples, all))))
                .collect();
            let from = first[at].map_or_else(|| "never".to_owned(), dictation::transcript::clock);
            format!(
                "Voice {} {taken}, first at {from}, holding {}",
                at + 1,
                holds.join(", ")
            )
        })
        .collect();
    voices.join("; ")
}

/// When each far-end speaker joins, in samples: in the order they first
/// spoke, the first at the start and each next at the first take once
/// as much more of the far end's speech has been said before it as their
/// count shares it out.
fn joins(cuts: &[Cut]) -> HashMap<String, usize> {
    let mut order: Vec<&str> = Vec::new();
    for cut in cuts {
        let mut speakers: Vec<&str> = cut.speakers.keys().map(String::as_str).collect();
        // Those first heard in one take come in by name, so a run never
        // differs from the last.
        speakers.sort_unstable();
        for speaker in speakers {
            if !order.contains(&speaker) {
                order.push(speaker);
            }
        }
    }
    let speech: usize = cuts.iter().flat_map(|cut| cut.speakers.values()).sum();
    let mut said = 0;
    let mut starts = Vec::new();
    for cut in cuts {
        while starts.len() < order.len() && said * order.len() >= starts.len() * speech {
            starts.push(cut.start);
        }
        said += cut.speakers.values().sum::<usize>();
    }
    // Any not reached join with the last take's start, as the last one may
    // when nothing is said once its share begins.
    let last = cuts.last().map_or(0, |cut| cut.start);
    starts.resize(order.len(), last);
    order
        .into_iter()
        .zip(starts)
        .map(|(speaker, start)| (speaker.to_owned(), start))
        .collect()
}

/// Samples as milliseconds from the start.
fn millis(samples: usize) -> u64 {
    u64::try_from(samples * 1000 / dictation::mel::RATE).unwrap_or(u64::MAX)
}

/// How well a meeting's voices were named, in a line.
fn report(name: &str, named: &Names) -> String {
    if named.named == 0 {
        return format!("{name}: named none of the speech");
    }
    format!(
        "{name}: of what was named {} named right, {} of the speech given a name",
        percent(named.right_tenths()),
        percent(named.named_tenths()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: usize = dictation::mel::RATE;
    const MINUTE: usize = 60 * SECOND;

    /// Three voices that sound nothing alike, one to a speaker.
    fn sound(speaker: &str) -> Vec<f32> {
        match speaker {
            "A" => vec![1.0, 0.0, 0.0],
            "B" => vec![0.0, 1.0, 0.0],
            _ => vec![0.0, 0.0, 1.0],
        }
    }

    /// A meeting's far end: a take of five seconds by each speaker at
    /// each minute given, in the order they're said, far enough apart for
    /// presence.rs's drift to tell who was in.
    fn meeting(said: &[(&str, usize)]) -> (Vec<Heard>, Vec<Cut>) {
        let mut heard = Vec::new();
        let mut cuts = Vec::new();
        for &(speaker, at) in said {
            heard.push(Heard {
                embedding: sound(speaker),
                samples: 5 * SECOND,
                worded: true,
            });
            cuts.push(Cut {
                take: cuts.len(),
                start: at * MINUTE,
                samples: 5 * SECOND,
                heard: Some(heard.len() - 1),
                speakers: HashMap::from([(speaker.to_owned(), 5 * SECOND)]),
            });
        }
        (heard, cuts)
    }

    #[test]
    fn speakers_join_in_the_order_they_first_spoke_a_like_share_apart() {
        let (_, cuts) = meeting(&[("B", 0), ("A", 6), ("B", 12), ("C", 20)]);
        let joins = joins(&cuts);
        assert_eq!(
            (joins["B"], joins["A"], joins["C"]),
            (0, 12 * MINUTE, 20 * MINUTE),
            "the speakers didn't join in the order they first spoke, a third of the speech apart"
        );
    }

    #[test]
    fn a_far_end_joined_one_at_a_time_is_named_whole_and_right() {
        let (heard, cuts) = meeting(&[
            ("A", 0),
            ("B", 5),
            ("A", 6),
            ("B", 11),
            ("C", 21),
            ("B", 23),
            ("A", 25),
            ("C", 27),
        ]);
        let (named, _) = name(&heard, &cuts);
        assert_eq!(
            (named.right, named.named, named.speech),
            (30 * SECOND, 30 * SECOND, 30 * SECOND),
            "what B said before joining wasn't left out, or a voice went unnamed or misnamed"
        );
    }

    #[test]
    fn a_murmur_before_joining_keeps_its_take_and_more_leaves_it_out() {
        let joins = HashMap::from([("A".to_owned(), 0), ("B".to_owned(), 10 * MINUTE)]);
        let (_, mut cuts) = meeting(&[("A", 2), ("A", 4)]);
        cuts[0].speakers.insert("B".to_owned(), SECOND / 4);
        cuts[1].speakers.insert("B".to_owned(), SECOND);
        assert!(
            !before(&cuts[0], &joins),
            "a take of A's with a murmur of B's before B joined was left out"
        );
        assert!(
            before(&cuts[1], &joins),
            "a take a sixth B's before B joined was kept"
        );
    }

    #[test]
    fn a_voice_mixing_two_speakers_is_right_only_for_its_name() {
        let (heard, mut cuts) = meeting(&[("A", 0), ("A", 2), ("B", 12), ("A", 14), ("B", 16)]);
        cuts[3].speakers.insert("B".to_owned(), SECOND);
        let (named, _) = name(&heard, &cuts);
        assert_eq!(
            (named.right, named.named),
            (20 * SECOND, 21 * SECOND),
            "B's second in a take of A's voice was counted as named right"
        );
    }

    #[test]
    fn a_voice_named_for_one_holding_less_of_it_is_wrong() {
        let held: Vec<HashMap<&str, usize>> = vec![
            HashMap::from([("A", 9), ("B", 1)]),
            HashMap::from([("A", 6), ("B", 4)]),
            HashMap::from([("B", 1)]),
        ];
        let who = [
            Who::Is("A".to_owned()),
            Who::Likely("B".to_owned()),
            Who::MayBe(vec!["A".to_owned()]),
        ];
        assert_eq!(
            wrong(&who, &held),
            1,
            "a voice mostly A's taken as likely B wasn't counted wrong, or another was"
        );
    }
}
