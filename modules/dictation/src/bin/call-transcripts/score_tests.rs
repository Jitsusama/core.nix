//! The tests of score.rs, its parent: how a set and its references are
//! read, a side heard by a model that hears what it's told, and a rate
//! held to its ceiling.

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
        let said = score(model, set.path(), [None, None]).unwrap_err();
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
        verdict(199, 199, "the words"),
        Ok("it misheard 19.9% of the words, within the 19.9% it may".to_owned())
    );
    assert_eq!(
        verdict(200, 199, "the words"),
        Err("it misheard 20.0% of the words, above the 19.9% it may".to_owned())
    );
}

#[test]
fn joels_side_is_held_to_its_own_ceiling_however_well_the_whole_is_heard() {
    let scored = Scored {
        written: [
            Errors {
                words: 1000,
                deleted: 130,
                ..Errors::default()
            },
            Errors {
                words: 3000,
                substituted: 300,
                ..Errors::default()
            },
        ],
        ..Scored::default()
    };
    assert_eq!(
        rated(&scored, Some(199), Some(119)),
        [
            Some(Ok(
                "it misheard 10.8% of the words, within the 19.9% it may".to_owned()
            )),
            Some(Err(
                "it misheard 13.0% of Joel's words, above the 11.9% it may".to_owned()
            )),
        ],
        "Joel's side wasn't held to its own ceiling, or the whole was held to it"
    );
    assert_eq!(
        rated(&scored, Some(199), None)[1],
        None,
        "Joel's side was held to a ceiling it wasn't given"
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
        verdict(errors.tenths(), 199, "the words").is_err_and(|said| said.contains("20.0%")),
        "the verdict says the rate the report does"
    );
}

#[test]
fn a_time_before_the_start_is_the_start() {
    assert_eq!(samples(-0.2), 0);
    assert_eq!(samples(1.5), 24_000);
}
