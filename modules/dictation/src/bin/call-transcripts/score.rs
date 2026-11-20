//! How well call-transcripts hears, measured: each meeting of a set
//! (meetings.nix lays them out) written down as a call is, both sides cut
//! into takes, heard, sifted and laid out, then each side's words held to
//! what its speakers said by its reference, as a word error rate
//! (`dictation::wer`).
//!
//! Each of the reference's lines is also heard alone, cut from its side
//! where the reference says it was said, with a little sound either side,
//! and none too long for Parakeet's window: a second reading, with no cutting, sifting or echo filter but no context
//! either. It's no floor for the model alone, since a clip also hears
//! whoever talks over its speaker; the gap between the two readings says
//! what context and the cutting are worth, a side at a time.

use std::fmt::Write as _;
use std::path::Path;

use dictation::parakeet::{RATE, WINDOW};
use dictation::transcript::{self, Line, Side, markdown};
use dictation::wav;
use dictation::wer::{Errors, said, words};

use crate::said;
use crate::{Settings, takes};

/// How much of the side's sound around a reference line is heard with
/// it, so its first and last words aren't cut.
const AROUND: f64 = 0.2;

/// The longest a reference line may run, in seconds: the longest a take
/// runs (`dictation::takes`), so heard alone, with `AROUND` either side, it
/// fits Parakeet's window.
const LONGEST: f64 = 14.0;

#[expect(
    clippy::cast_precision_loss,
    reason = "the window is a few hundred thousand samples"
)]
const _: () = assert!(LONGEST + 2.0 * AROUND < WINDOW as f64 / RATE as f64);

/// One line of a reference: when it was said, by whom, and its words.
struct Said {
    start: f64,
    end: f64,
    speaker: String,
    text: String,
}

/// A meeting's errors: each side's as written down, and as heard alone.
#[derive(Default)]
struct Scored {
    written: [Errors; 2],
    alone: [Errors; 2],
}

/// Scores every meeting of `set` with the speech model in `model`, says
/// each one's errors and the set's, and fails when the set's, as written
/// down, are above `ceiling` tenths of a percent.
pub fn score(model: &Path, set: &Path, ceiling: Option<usize>) -> Result<(), String> {
    // Every reference is read before the model loads, so a bad one is
    // refused at once rather than after the meetings before it are heard.
    let sides = read(&set.join("sides"))?;
    let mut meetings = Vec::new();
    for entry in sides.lines().filter(|line| !line.trim().is_empty()) {
        let (name, mine) = entry
            .split_once(' ')
            .ok_or_else(|| format!("sides: {entry:?} isn't a meeting and a speaker"))?;
        let file = format!("{name}.ref");
        let lines = reference(&read(&set.join(&file))?).map_err(|e| format!("{file}: {e}"))?;
        meetings.push((name, mine, lines));
    }
    let mut speech = Settings::read(model.to_owned()).load()?;
    let mut all = Scored::default();
    for (name, mine, reference) in meetings {
        let mut scored = Scored::default();
        let mut lines = Vec::new();
        for side in [Side::Mine, Side::Theirs] {
            let file = set.join(format!("{name}.{}.wav", label(side)));
            let sound = wav::samples(
                &std::fs::read(&file).map_err(|e| format!("reading {}: {e}", file.display()))?,
            )?;
            for take in takes(&sound) {
                let (text, _) = speech.transcribe(&take.samples)?;
                if let Some(text) = said::spoken(&text) {
                    lines.push(Line {
                        side,
                        start: take.start,
                        end: take.start + take.samples.len(),
                        text,
                    });
                }
            }
            for line in reference.iter().filter(|line| whose(line, mine) == side) {
                let (from, to) = (samples(line.start - AROUND), samples(line.end + AROUND));
                let clip = &sound[from.min(sound.len())..to.min(sound.len())];
                let (text, _) = speech.transcribe(clip)?;
                scored.alone[index(side)] += Errors::count(&said(&line.text), &words(&text));
            }
        }
        let written = transcript::read(&markdown(&lines, "Me"));
        for side in [Side::Mine, Side::Theirs] {
            let spoken: Vec<String> = reference
                .iter()
                .filter(|line| whose(line, mine) == side)
                .flat_map(|line| said(&line.text))
                .collect();
            let heard: Vec<String> = written
                .iter()
                .filter(|line| line.side == side)
                .flat_map(|line| words(&line.text))
                .collect();
            scored.written[index(side)] = Errors::count(&spoken, &heard);
        }
        println!("{}", report(name, &scored));
        for i in 0..2 {
            all.written[i] += scored.written[i];
            all.alone[i] += scored.alone[i];
        }
    }
    println!("{}", report("all", &all));
    let mut total = all.written[0];
    total += all.written[1];
    if total.words == 0 {
        return Err(format!("{} has no words to hear", set.display()));
    }
    match ceiling {
        Some(most) => verdict(total.tenths(), most).map(|passed| println!("{passed}")),
        None => Ok(()),
    }
}

/// The ceiling given as `text`, in tenths of a percent, when it's a
/// percentage to one place at most: one such as "nan" would pass every
/// score, and one with two places would print as another.
pub fn ceiling(text: &str) -> Option<usize> {
    let (whole, tenth) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !digits(tenth) || tenth.len() > 1 {
        return None;
    }
    Some(whole.parse::<usize>().ok()? * 10 + tenth.parse::<usize>().ok()?)
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
fn percent(tenths: usize) -> String {
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
    out.trim_end().to_owned()
}

fn rate(errors: &Errors) -> String {
    format!("{} of {} words", percent(errors.tenths()), errors.words)
}

fn kinds(errors: &Errors) -> String {
    format!(
        "{} substituted, {} dropped, {} added",
        errors.substituted, errors.deleted, errors.inserted
    )
}

/// A reference's lines: a start and end in seconds, a speaker, and what
/// they said, after any comment.
fn reference(text: &str) -> Result<Vec<Said>, String> {
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

fn whose(line: &Said, mine: &str) -> Side {
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

fn index(side: Side) -> usize {
    match side {
        Side::Mine => 0,
        Side::Theirs => 1,
    }
}

/// Seconds into a side as samples, none before its start.
fn samples(seconds: f64) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a meeting is minutes long, and a time before it is its start"
    )]
    let samples = (seconds.max(0.0) * f64::from(u32::try_from(RATE).unwrap_or(16_000))) as usize;
    samples
}

fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("reading {}: {e}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            reference("80.49 94.49 A Okay\n").is_ok(),
            "a line as long as a take, not from the start, was refused"
        );
    }

    #[test]
    fn only_a_percentage_to_one_place_is_a_ceiling() {
        assert_eq!(ceiling("19.9"), Some(199));
        assert_eq!(ceiling("20"), Some(200));
        assert_eq!(ceiling("0"), Some(0));
        for refused in ["nan", "inf", "-1", "nineteen", "", "19.96", "19.", ".9"] {
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
