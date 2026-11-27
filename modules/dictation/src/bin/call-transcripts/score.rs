//! How well call-transcripts hears, measured: each meeting of a set
//! (meetings.nix lays them out) written down as a call is, both sides cut
//! into takes, heard, sifted and laid out, then each side's words held to
//! what its speakers said by its reference, as a word error rate
//! (`dictation::wer`).
//!
//! Each of the reference's lines is also heard alone, cut from its side
//! where the reference says it was said, with a little sound either side,
//! and none too long for Parakeet's window: a second reading, with no
//! cutting, sifting or echo filter but no context either. It's no floor
//! for the model alone, since a clip also hears whoever talks over its
//! speaker; the gap between the two readings says what context and the
//! cutting are worth, a side at a time.
//!
//! The far end's words are timed as they're heard, and the times held to
//! their takes and to the reference's speech (times.rs), since cutting
//! their echo from Joel's words goes by them.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use dictation::echo;
use dictation::parakeet::{Parakeet, RATE, WINDOW, Word};
use dictation::takes::Take;
use dictation::transcript::{self, Line, Side, markdown};
use dictation::wav;
use dictation::wer::{Errors, said, words};

use crate::hear::{self, Hears};
use crate::said;
use crate::times::{self, Timed};
use crate::{Settings, takes};

/// How much of the side's sound around a reference line is heard with
/// it, so its first and last words aren't cut.
const AROUND: f64 = 0.2;

/// The longest a reference line may run, in seconds: the longest a take
/// runs, so heard alone, with `AROUND` either side, it fits Parakeet's
/// window.
#[expect(
    clippy::cast_precision_loss,
    reason = "a take is a few hundred thousand samples"
)]
const LONGEST: f64 = dictation::takes::LONGEST as f64 / RATE as f64;

#[expect(
    clippy::cast_precision_loss,
    reason = "the window is a few hundred thousand samples"
)]
const _: () = assert!(
    LONGEST + 2.0 * AROUND < WINDOW as f64 / RATE as f64,
    "a reference line heard alone overruns Parakeet's window"
);

/// One line of a reference: when it was said, by whom, and its words.
pub struct Said {
    pub start: f64,
    pub end: f64,
    pub speaker: String,
    pub text: String,
}

/// A meeting of the set, read: its name, the speaker on Joel's side, its
/// reference, and each side's sound. How well voices are told apart reads
/// the same set (apart.rs).
pub struct Meeting<'a> {
    pub name: &'a str,
    pub mine: &'a str,
    pub reference: Vec<Said>,
    pub sounds: [Vec<f32>; 2],
}

/// A meeting's errors: each side's as written down, and as heard alone,
/// and Joel's side's as it would be written with none of his words taken
/// for the others' echo, and with only his words cut by time and
/// loudness, none of his lines left out whole for repeating theirs. A
/// clean meeting's sides hold no echo (meetings.nix), so where either is
/// lower there, the echo filter left out words he said.
#[derive(Default)]
struct Scored {
    written: [Errors; 2],
    alone: [Errors; 2],
    unfiltered: Errors,
    cut_by_word: Errors,
}

/// A take heard as a line, with its words timed from the start of its
/// sound.
pub type HeardLine = (Line, Vec<Word>);

/// A take of `side`'s heard as a line, with its words timed from the
/// start of its sound, none when it holds no words: heard as the daemon
/// hears it (`hear::transcribe_take`), so what's scored is what it writes.
pub fn hear_take(
    speech: &mut impl Hears,
    side: Side,
    take: &Take,
) -> Result<Option<HeardLine>, String> {
    let (text, words) = hear::transcribe_take(speech, take.start, &take.samples)?;
    Ok(text.map(|text| {
        let line = Line {
            side,
            start: take.start,
            end: take.start + take.samples.len(),
            text,
            voice: None,
            name: None,
        };
        (line, words)
    }))
}

/// `side`'s `takes`, whose sound came from `file`, heard onto `lines`,
/// each line's words kept in `words` by where it falls in them. Unlike
/// the daemon, the first take not heard stops it, so no check passes on
/// a model that fails, naming the file and the second the take starts
/// at, so a builder's failure says where to listen.
pub fn hear_side(
    speech: &mut impl Hears,
    side: Side,
    file: &str,
    takes: &[Take],
    lines: &mut Vec<Line>,
    words: &mut HashMap<usize, Vec<Word>>,
) -> Result<(), String> {
    for take in takes {
        let heard = hear_take(speech, side, take)
            .map_err(|why| format!("hearing {file} at {} s: {why}", take.start / RATE))?;
        if let Some((line, its_words)) = heard {
            words.insert(lines.len(), its_words);
            lines.push(line);
        }
    }
    Ok(())
}

/// Joel's lines of `lines` that are written, by when each starts.
fn joels(lines: &[Line]) -> Vec<&Line> {
    let mut own: Vec<&Line> = lines
        .iter()
        .filter(|line| line.side == Side::Mine && !line.text.is_empty())
        .collect();
    own.sort_by_key(|line| line.start);
    own
}

/// Scores every meeting of `set` with the speech model in `model`, says
/// each one's errors and the set's, and fails when the set's, as written
/// down, are above `ceilings`' first in tenths of a percent, or Joel's
/// side's above its second.
pub fn score(model: &Path, set: &Path, ceilings: [Option<usize>; 2]) -> Result<(), String> {
    // Every reference and recording is read before the model loads, so a
    // bad one is refused at once rather than after the meetings before it
    // are heard.
    let sides = read(&set.join("sides"))?;
    let meetings = meetings(set, &sides)?;
    let mut speech = Settings::read(model.to_owned()).load()?;
    let mut all = Scored::default();
    let mut all_times = Timed::default();
    for meeting in meetings {
        let (scored, judged) = score_meeting(&mut speech, &meeting)?;
        all_times += judged;
        for i in 0..2 {
            all.written[i] += scored.written[i];
            all.alone[i] += scored.alone[i];
        }
        all.unfiltered += scored.unfiltered;
        all.cut_by_word += scored.cut_by_word;
    }
    println!("{}", report("all", &all));
    println!("{}", all_times.describe("all"));
    let mut total = all.written[0];
    total += all.written[1];
    if total.words == 0 {
        return Err(format!("{} has no words to hear", set.display()));
    }
    let [most, joels] = ceilings;
    let [rate, joels] = rated(&all, most, joels);
    let (held, failed) = say_each([Some(all_times.verdict(times::INSIDE)), rate, joels]);
    for passed in held {
        println!("{passed}");
    }
    failed
}

/// Whether what `all` wrote down passes `most`, both sides together, and
/// whether Joel's side alone passes `joels`, none for a ceiling not given.
/// A rule that cuts his words with their echo can lower the whole rate
/// while losing his own, so his side is held to a ceiling of its own.
fn rated(
    all: &Scored,
    most: Option<usize>,
    joels: Option<usize>,
) -> [Option<Result<String, String>>; 2] {
    let mut total = all.written[0];
    total += all.written[1];
    let his = all.written[index(Side::Mine)];
    [
        most.map(|most| verdict(total.tenths(), most, "the words")),
        joels.map(|most| verdict(his.tenths(), most, "Joel's words")),
    ]
}

/// Of the verdicts given, each that passed, and every one that failed in
/// one, so a failure hides none after it.
fn say_each<const N: usize>(
    verdicts: [Option<Result<String, String>>; N],
) -> (Vec<String>, Result<(), String>) {
    let (mut passed, mut failed) = (Vec::new(), Vec::new());
    for verdict in verdicts.into_iter().flatten() {
        match verdict {
            Ok(said) => passed.push(said),
            Err(why) => failed.push(why),
        }
    }
    let failed = if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("; "))
    };
    (passed, failed)
}

/// Hears `meeting` with `speech` and says its errors, its far end's word
/// times and what was cut from Joel's side as their echo; answers its
/// errors and how its far end's words were timed, for the set's.
fn score_meeting(speech: &mut Parakeet, meeting: &Meeting) -> Result<(Scored, Timed), String> {
    let Meeting {
        name,
        mine,
        reference,
        sounds,
    } = meeting;
    let mut scored = Scored::default();
    let mut lines = Vec::new();
    let mut word_times = HashMap::new();
    for side in [Side::Mine, Side::Theirs] {
        let sound = &sounds[index(side)];
        let file = format!("{name}.{}.wav", label(side));
        hear_side(
            &mut *speech,
            side,
            &file,
            &takes(sound),
            &mut lines,
            &mut word_times,
        )?;
        for line in reference.iter().filter(|line| whose(line, mine) == side) {
            let (from, to) = (samples(line.start - AROUND), samples(line.end + AROUND));
            let clip = &sound[from.min(sound.len())..to.min(sound.len())];
            let (text, _) = speech
                .transcribe(clip)
                .map_err(|why| format!("hearing {file} at {} s alone: {why}", line.start))?;
            scored.alone[index(side)] += Errors::count(&said(&line.text), &words(&text));
        }
    }
    let theirs: Vec<(f64, f64)> = reference
        .iter()
        .filter(|line| whose(line, mine) == Side::Theirs)
        .map(|line| (line.start - AROUND, line.end + AROUND))
        .collect();
    let judged = times::judge(&lines, &word_times, &theirs);
    println!("{}", judged.describe(name));
    let spoken = |side: Side| -> Vec<String> {
        reference
            .iter()
            .filter(|line| whose(line, mine) == side)
            .flat_map(|line| said(&line.text))
            .collect()
    };
    // Only the sound of their takes, as the daemon is given it.
    let mut their_takes = vec![0.0; sounds[index(Side::Theirs)].len()];
    for take in takes(&sounds[index(Side::Theirs)]) {
        for (into, &sample) in their_takes[take.start..].iter_mut().zip(&take.samples) {
            *into = sample;
        }
    }
    // Only outside every line of theirs: inside one, they and he may
    // both be speaking.
    let their_lines: Vec<(usize, usize)> = lines
        .iter()
        .filter(|line| line.side == Side::Theirs)
        .map(|line| (line.start, line.end))
        .collect();
    let loud = |word: &Word| {
        let inside = their_lines
            .iter()
            .any(|&(start, end)| (start..end).contains(&word.at));
        let gap = loudest_before(&their_takes, word.at) - word.level;
        (!inside && gap >= SOUND).then_some(gap)
    };
    let (cut, cuts) = echo::cut_echoes_heard(&lines, &word_times, said::spoken, loud);
    println!(
        "{name} lab: words cut by level, sound at {SOUND} dB: {}",
        cuts.iter()
            .filter(|cut| cut.below.is_some())
            .map(|cut| cut.text.split_whitespace().count())
            .sum::<usize>()
    );
    for taken in &cuts {
        println!(
            "{name}: Joel's words at {} taken for an echo, {}: {}",
            transcript::clock(taken.at),
            taken.by(),
            taken.text
        );
    }
    lab_kept(
        name,
        mine,
        reference,
        &lines,
        &word_times,
        &sounds[index(Side::Theirs)],
    );
    let written = transcript::read(&markdown(&cut, "Me", None, None), "Me");
    for side in [Side::Mine, Side::Theirs] {
        let heard: Vec<String> = written
            .iter()
            .filter(|line| line.side == side)
            .flat_map(|line| words(&line.text))
            .collect();
        scored.written[index(side)] = Errors::count(&spoken(side), &heard);
    }
    for line in joels(&cut)
        .into_iter()
        .filter(|line| echo::echoes(line, &cut))
    {
        let at = transcript::clock(line.start);
        println!(
            "{name}: Joel's line at {at} taken for an echo: {}",
            line.text
        );
    }
    let heard: Vec<String> = joels(&lines)
        .iter()
        .flat_map(|line| words(&line.text))
        .collect();
    scored.unfiltered = Errors::count(&spoken(Side::Mine), &heard);
    let cut_by_word: Vec<String> = joels(&cut)
        .iter()
        .flat_map(|line| words(&line.text))
        .collect();
    scored.cut_by_word = Errors::count(&spoken(Side::Mine), &cut_by_word);
    println!("{}", report(name, &scored));
    Ok((scored, judged))
}

/// Lab only: Joel's words left uncut, by why the level rule kept each,
/// counted apart for those said inside only their reference lines (echo
/// left in) and inside only his (his own words), with how far below
/// theirs and below his voice each was, in 5 dB steps.
fn lab_kept(
    name: &str,
    mine: &str,
    reference: &[Said],
    lines: &[Line],
    words: &HashMap<usize, Vec<Word>>,
    their_sound: &[f32],
) {
    let levels: HashMap<usize, f32> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.side == Side::Mine)
        .filter_map(|(index, _)| words.get(&index))
        .flatten()
        .map(|word| (word.at, word.level))
        .collect();
    let inside = |side: Side, t: f64, margin: f64| {
        reference
            .iter()
            .filter(|line| whose(line, mine) == side)
            .any(|line| line.start - margin <= t && t <= line.end + margin)
    };
    // Where the far end's own transcript has them: their lines, and
    // the time of each word in them.
    let their_lines: Vec<(usize, usize)> = lines
        .iter()
        .filter(|line| line.side == Side::Theirs)
        .map(|line| (line.start, line.end))
        .collect();
    let their_words: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.side == Side::Theirs)
        .filter_map(|(index, _)| words.get(&index))
        .flatten()
        .map(|word| word.at)
        .collect();
    let nearest = |at: usize| {
        let apart = their_words
            .iter()
            .map(|&theirs| theirs.abs_diff(at))
            .min()
            .unwrap_or(usize::MAX);
        match apart {
            n if n < RATE / 10 => "under 0.1 s",
            n if n < RATE / 4 => "under 0.25 s",
            n if n < RATE / 2 => "under 0.5 s",
            n if n < RATE => "under 1 s",
            _ => "1 s or more",
        }
    };
    let mut counts: std::collections::BTreeMap<(String, &str), usize> =
        std::collections::BTreeMap::new();
    for (at, why, below, voice) in echo::kept(lines, words) {
        let t = at as f64 / RATE as f64;
        let whose = match (inside(Side::Theirs, t, 0.0), inside(Side::Mine, t, 0.5)) {
            (true, false) => "echo",
            (false, true) => "his",
            (true, true) => "both",
            (false, false) => "neither",
        };
        let step = |db: f32| ((db / 5.0).floor() * 5.0) as i32;
        let voice = voice.map_or("unknown".to_owned(), |v| step(v).to_string());
        let key = format!(
            "{whose} kept, {why}, below theirs {} voice {voice}",
            step(below)
        );
        *counts.entry((key, why)).or_default() += 1;
        *counts
            .entry((format!("{whose} kept, {why}"), ""))
            .or_default() += 1;
        if let Some(&level) = levels.get(&at) {
            let above = step(loudest_before(their_sound, at) - level);
            let key = format!("{whose} kept, sound above {above}");
            *counts.entry((key, "")).or_default() += 1;
            let key = format!("{whose} kept, sound above {above} voice {voice}");
            *counts.entry((key, "")).or_default() += 1;
            if above >= 15 {
                let lined = if their_lines
                    .iter()
                    .any(|&(start, end)| (start..end).contains(&at))
                {
                    "in"
                } else {
                    "out of"
                };
                let key = format!(
                    "{whose} near, sound above {above}, {lined} their lines, their word {}",
                    nearest(at)
                );
                *counts.entry((key, "")).or_default() += 1;
            }
        }
    }
    for ((key, _), count) in counts {
        println!("{name} lab: {key}: {count}");
    }
}

/// Lab only: how far their sound must stand above a word of his for it
/// to be cut as their echo, in dB.
const SOUND: f32 = 20.0;

/// Lab only: the loudest quarter second of `sound` starting from a
/// second before `at` to three tenths after, in 50 ms steps, in dB of
/// full scale, as `parakeet`'s word levels are measured.
fn loudest_before(sound: &[f32], at: usize) -> f32 {
    let quarter = RATE / 4;
    let step = RATE / 20;
    let mut loudest = -120.0_f32;
    let mut from = at.saturating_sub(RATE);
    while from <= at + RATE * 3 / 10 {
        let span = sound
            .get(from..(from + quarter).min(sound.len()))
            .unwrap_or_default();
        let energy: f32 = span.iter().map(|s| s * s).sum();
        let power = energy / span.len().max(1) as f32;
        loudest = loudest.max(10.0 * (power + 1e-12).log10());
        from += step;
    }
    loudest
}

/// The meetings `sides` names in `set`, each refused when its line,
/// reference or either side's recording is wrong.
pub fn meetings<'a>(set: &Path, sides: &'a str) -> Result<Vec<Meeting<'a>>, String> {
    let mut meetings: Vec<Meeting> = Vec::new();
    for entry in sides.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = entry.split_whitespace();
        let (Some(name), Some(mine), None) = (fields.next(), fields.next(), fields.next()) else {
            return Err(format!("sides: {entry:?} isn't a meeting and a speaker"));
        };
        if meetings.iter().any(|meeting| meeting.name == name) {
            return Err(format!("sides: {name} is named twice"));
        }
        let file = format!("{name}.ref");
        let lines = reference(&read(&set.join(&file))?).map_err(|e| format!("{file}: {e}"))?;
        if !lines.iter().any(|line| line.speaker == mine) {
            return Err(format!(
                "{file}: {mine} never speaks, so can't be Joel's side"
            ));
        }
        let last = lines.iter().map(|line| line.end).fold(0.0, f64::max);
        let mut sounds = [Vec::new(), Vec::new()];
        for side in [Side::Mine, Side::Theirs] {
            let file = set.join(format!("{name}.{}.wav", label(side)));
            let sound = sound(&file)?;
            if sound.len() < samples(last) {
                return Err(format!(
                    "{} ends before its reference's last line, at {last} s",
                    file.display()
                ));
            }
            sounds[index(side)] = sound;
        }
        meetings.push(Meeting {
            name,
            mine,
            reference: lines,
            sounds,
        });
    }
    if meetings.is_empty() {
        return Err(format!("{} has no meetings to hear", set.display()));
    }
    Ok(meetings)
}

/// The ceiling given as `text`, in tenths of a percent, when it's a
/// percentage below 100 to one place at most: one such as "nan" would pass
/// every score, one of 100 would pass a model that heard nothing at all,
/// and one with two places would print as another.
pub fn ceiling(text: &str) -> Option<usize> {
    let (whole, tenth) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !digits(tenth) || tenth.len() > 1 {
        return None;
    }
    let tenths = whole
        .parse::<usize>()
        .ok()?
        .checked_mul(10)?
        .checked_add(tenth.parse::<usize>().ok()?)?;
    (tenths < 1000).then_some(tenths)
}

/// Whether `tenths` of a percent of `whose` wrong passes a ceiling of
/// `most`, said either way, as the report says it.
fn verdict(tenths: usize, most: usize, whose: &str) -> Result<String, String> {
    let (said, may) = (percent(tenths), percent(most));
    if tenths > most {
        Err(format!(
            "it misheard {said} of {whose}, above the {may} it may"
        ))
    } else {
        Ok(format!(
            "it misheard {said} of {whose}, within the {may} it may"
        ))
    }
}

/// Tenths of a percent, as a percentage to one place.
pub fn percent(tenths: usize) -> String {
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// A meeting's errors, a line each way it was heard.
fn report(name: &str, scored: &Scored) -> String {
    let mut out = String::new();
    for (how, errors) in [
        ("written down", &scored.written),
        ("heard alone", &scored.alone),
    ] {
        let mut total = errors[0];
        total += errors[1];
        let _ = write!(
            out,
            "{name} {how}: {} ({}), Joel's side {}, the far end {}",
            rate(&total),
            kinds(&total),
            rate(&errors[0]),
            rate(&errors[1]),
        );
        out.push('\n');
    }
    let _ = write!(
        out,
        "{name} with none of Joel's words taken for an echo: Joel's side {} ({})",
        rate(&scored.unfiltered),
        kinds(&scored.unfiltered),
    );
    out.push('\n');
    let _ = write!(
        out,
        "{name} with only Joel's words cut, no line of his left out whole: Joel's side {} ({})",
        rate(&scored.cut_by_word),
        kinds(&scored.cut_by_word),
    );
    out
}

pub fn rate(errors: &Errors) -> String {
    format!("{} of {} words", percent(errors.tenths()), errors.words)
}

pub fn kinds(errors: &Errors) -> String {
    format!(
        "{} substituted, {} dropped, {} added",
        errors.substituted, errors.deleted, errors.inserted
    )
}

/// A reference's lines: a start and end in seconds, a speaker, and what
/// they said, after any comment.
pub fn reference(text: &str) -> Result<Vec<Said>, String> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.splitn(4, ' ');
            let mut seconds = || {
                parts
                    .next()
                    .and_then(|part| part.parse::<f64>().ok())
                    .ok_or_else(|| format!("a reference line without its times: {line:?}"))
            };
            let (start, end) = (seconds()?, seconds()?);
            // Held in hundredths, as the times are written, so a line
            // just 14 s long isn't refused for a float's last bit.
            let fits = start <= end && ((end - start) * 100.0).round() <= LONGEST * 100.0;
            if !fits {
                return Err(format!(
                    "a reference line has to end after it starts, within {LONGEST} s: {line:?}"
                ));
            }
            let speaker = parts.next().unwrap_or_default().to_owned();
            let text = parts.next().unwrap_or_default().to_owned();
            Ok(Said {
                start,
                end,
                speaker,
                text,
            })
        })
        .collect()
}

pub fn whose(line: &Said, mine: &str) -> Side {
    if line.speaker == mine {
        Side::Mine
    } else {
        Side::Theirs
    }
}

fn label(side: Side) -> &'static str {
    match side {
        Side::Mine => "mine",
        Side::Theirs => "theirs",
    }
}

pub fn index(side: Side) -> usize {
    match side {
        Side::Mine => 0,
        Side::Theirs => 1,
    }
}

/// Seconds into a side as samples, none before its start.
pub fn samples(seconds: f64) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a meeting is minutes long, and a time before it is its start"
    )]
    let samples = (seconds.max(0.0) * f64::from(u32::try_from(RATE).unwrap_or(16_000))) as usize;
    samples
}

/// A side's recording, its file named when it can't be read.
fn sound(file: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
    wav::samples(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

pub fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("reading {}: {e}", file.display()))
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
