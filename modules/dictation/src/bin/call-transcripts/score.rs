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
/// down, are above `ceiling` tenths of a percent.
pub fn score(model: &Path, set: &Path, ceiling: Option<usize>) -> Result<(), String> {
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
    let rate = ceiling.map(|most| verdict(total.tenths(), most));
    let (held, failed) = say_each([Some(all_times.verdict(times::INSIDE)), rate]);
    for passed in held {
        println!("{passed}");
    }
    failed
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
    let (cut, cuts) = echo::cut_echoes(&lines, &word_times, said::spoken);
    for taken in &cuts {
        let ms = taken.after * 1000 / isize::try_from(RATE).unwrap_or(isize::MAX);
        println!(
            "{name}: Joel's words at {} taken for an echo, {ms} ms after theirs: {}",
            transcript::clock(taken.at),
            taken.text
        );
    }
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

/// Whether `tenths` of a percent wrong passes a ceiling of `most`, said
/// either way, as the report says it.
fn verdict(tenths: usize, most: usize) -> Result<String, String> {
    let (said, may) = (percent(tenths), percent(most));
    if tenths > most {
        Err(format!(
            "it misheard {said} of the words, above the {may} it may"
        ))
    } else {
        Ok(format!(
            "it misheard {said} of the words, within the {may} it may"
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
mod tests {
    use std::time::Duration;

    use dictation::parakeet::Timings;

    use super::*;

    /// A model that hears each take as the next of `said`, or fails on
    /// it where that's none.
    struct Hearing {
        said: Vec<Option<&'static str>>,
    }

    impl Hears for Hearing {
        fn words(&mut self, _: &[f32]) -> dictation::openvino::Result<(Vec<Word>, Timings)> {
            let said = self.said.remove(0).ok_or("the encoder failed")?;
            let word = Word {
                text: said.to_owned(),
                at: 0,
                level: 0.0,
            };
            let took = Timings {
                mel: Duration::ZERO,
                encoder: Duration::ZERO,
                decoder: Duration::ZERO,
                tokens: 0,
            };
            Ok((vec![word], took))
        }

        fn device(&self) -> &'static str {
            "CPU"
        }
    }

    #[test]
    fn a_side_stops_at_a_take_not_heard_and_says_where_it_is() {
        let take = |second: usize| Take {
            start: second * RATE,
            samples: vec![0.0; RATE],
        };
        let takes = [take(10), take(40)];
        let (mut lines, mut words) = (Vec::new(), HashMap::new());
        let mut heard = Hearing {
            said: vec![Some("Yes."), Some("No.")],
        };
        hear_side(
            &mut heard,
            Side::Mine,
            "mine.wav",
            &takes,
            &mut lines,
            &mut words,
        )
        .expect("a side whose every take is heard failed");
        assert_eq!(lines.len(), 2, "a heard take wasn't written as a line");
        // The first take isn't heard and the next would be, so going on
        // past it would write a third line.
        let mut failing = Hearing {
            said: vec![None, Some("Yes.")],
        };
        let failed = hear_side(
            &mut failing,
            Side::Theirs,
            "theirs.wav",
            &takes,
            &mut lines,
            &mut words,
        );
        assert_eq!(
            failed,
            Err("hearing theirs.wav at 10 s: the encoder failed".to_owned()),
            "a take not heard didn't stop its side, or didn't say which file and second it was"
        );
        assert_eq!(lines.len(), 2, "a side went on past a take it didn't hear");
    }

    #[test]
    fn a_reference_reads_its_times_speaker_and_words_past_its_comment() {
        let lines =
            reference("# AMI\n54.95 60.85 A Okay everybody ready\n55.70 56.03 D Okay\n").unwrap();
        assert_eq!(lines.len(), 2);
        assert!(
            (lines[0].start - 54.95).abs() < 1e-9,
            "the first line starts at 54.95, not {}",
            lines[0].start
        );
        assert!(
            (lines[1].end - 56.03).abs() < 1e-9,
            "the second line ends at 56.03, not {}",
            lines[1].end
        );
        assert_eq!(lines[0].speaker, "A");
        assert_eq!(lines[0].text, "Okay everybody ready");
        assert_eq!(whose(&lines[0], "A"), Side::Mine);
        assert_eq!(whose(&lines[1], "A"), Side::Theirs);
    }

    #[test]
    fn a_reference_line_without_its_times_is_refused() {
        assert!(
            reference("A Okay\n").is_err(),
            "a line with no times was read"
        );
    }

    #[test]
    fn a_reference_line_ending_before_it_starts_or_past_a_take_is_refused() {
        assert!(
            reference("60.85 54.95 A Okay\n").is_err(),
            "a line ending before it starts was read"
        );
        assert!(
            reference("0.00 14.50 A Okay\n").is_err(),
            "a line longer than a take was read"
        );
        assert!(
            reference("0.00 14.00 A Okay\n").is_ok(),
            "a line as long as a take was refused"
        );
        // 16.01 - 2.01 is a hair over 14 in f64, so only the hundredths
        // read it as a take's length.
        assert!(
            reference("2.01 16.01 A Okay\n").is_ok(),
            "a line as long as a take, not from the start, was refused"
        );
    }

    #[test]
    fn a_set_is_refused_before_the_model_loads() {
        let set = tempfile::tempdir().unwrap();
        let model = Path::new("/no/model/here");
        let write = |file: &str, text: &str| std::fs::write(set.path().join(file), text).unwrap();
        let refused = |why: &str| {
            let said = score(model, set.path(), None).unwrap_err();
            assert!(said.contains(why), "refused for {said:?}, not {why:?}");
        };
        write("sides", "\n");
        refused("no meetings");
        write("sides", "M\n");
        refused("isn't a meeting and a speaker");
        write("sides", "M A B\n");
        refused("isn't a meeting and a speaker");
        // A whole first meeting, so only reading every reference and
        // recording before the model loads can refuse the second's.
        let silent = |file: &str, samples| {
            std::fs::write(set.path().join(file), silence(samples)).unwrap();
        };
        let blank = |file: &str| silent(file, 32_000);
        write("M.ref", "0.50 1.00 A Okay\n");
        blank("M.mine.wav");
        blank("M.theirs.wav");
        write("sides", "M A \nN A\n");
        write("N.ref", "1.00 0.50 A Okay\n");
        refused("N.ref: a reference line");
        write("N.ref", "0.50 1.00 B Okay\n");
        refused("N.ref: A never speaks");
        write("N.ref", "0.50 1.00 A Okay\n");
        refused(&format!(
            "reading {}",
            set.path().join("N.mine.wav").display()
        ));
        write("N.mine.wav", "");
        refused("N.mine.wav: not a RIFF WAV");
        // N's reference ends at 1 s, its 16,000th sample: a sample short
        // is refused, and a recording ending just there is whole.
        silent("N.mine.wav", 15_999);
        refused("N.mine.wav ends before its reference's last line");
        silent("N.mine.wav", 16_000);
        refused(&format!(
            "reading {}",
            set.path().join("N.theirs.wav").display()
        ));
        silent("N.theirs.wav", 15_999);
        refused("N.theirs.wav ends before its reference's last line");
        blank("N.theirs.wav");
        write("sides", "M A\nN A\nM A\n");
        refused("M is named twice");
    }

    /// A WAV of `samples` of silence, in the one format `wav` reads: PCM,
    /// mono, 16 kHz, 16-bit.
    fn silence(samples: u32) -> Vec<u8> {
        let data = samples * 2;
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(36 + data).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        for word in [1_u16, 1] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for long in [16_000_u32, 32_000] {
            bytes.extend_from_slice(&long.to_le_bytes());
        }
        for word in [2_u16, 16] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data.to_le_bytes());
        bytes.resize(bytes.len() + usize::try_from(data).unwrap(), 0);
        bytes
    }

    #[test]
    fn only_a_percentage_to_one_place_is_a_ceiling() {
        assert_eq!(ceiling("19.9"), Some(199));
        assert_eq!(ceiling("20"), Some(200));
        assert_eq!(ceiling("0"), Some(0));
        assert_eq!(ceiling("99.9"), Some(999));
        let huge = "18446744073709551615";
        for refused in [
            "nan", "inf", "-1", "nineteen", "", "19.96", "19.", ".9", "100", huge,
        ] {
            assert_eq!(ceiling(refused), None, "{refused:?} was taken as a ceiling");
        }
    }

    #[test]
    fn a_rate_is_held_to_its_ceiling_as_it_is_said() {
        assert_eq!(
            verdict(199, 199),
            Ok("it misheard 19.9% of the words, within the 19.9% it may".to_owned())
        );
        assert_eq!(
            verdict(200, 199),
            Err("it misheard 20.0% of the words, above the 19.9% it may".to_owned())
        );
    }

    #[test]
    fn every_failed_verdict_is_said_and_one_failing_fails_them_all() {
        let passed = || Some(Ok("held".to_owned()));
        let failed = |why: &str| Some(Err(why.to_owned()));
        assert_eq!(
            say_each([passed(), None, passed()]),
            (vec!["held".to_owned(), "held".to_owned()], Ok(())),
            "the verdicts given, all passing, weren't each said"
        );
        let (_, both) = say_each([failed("times"), failed("rate")]);
        assert!(
            both.as_ref()
                .is_err_and(|said| said.contains("times") && said.contains("rate")),
            "a failed times verdict hid the rate's: {both:?}"
        );
        assert_eq!(
            say_each([failed("times"), passed()]),
            (vec!["held".to_owned()], Err("times".to_owned())),
            "a failed times verdict hid the rate's that passed, or one failing passed"
        );
    }

    #[test]
    fn the_report_and_the_verdict_say_the_same_rate() {
        let errors = Errors {
            words: 4000,
            substituted: 798,
            ..Errors::default()
        };
        assert_eq!(rate(&errors), "20.0% of 4000 words");
        assert!(
            verdict(errors.tenths(), 199).is_err_and(|said| said.contains("20.0%")),
            "the verdict says the rate the report does"
        );
    }

    #[test]
    fn a_time_before_the_start_is_the_start() {
        assert_eq!(samples(-0.2), 0);
        assert_eq!(samples(1.5), 24_000);
    }
}
