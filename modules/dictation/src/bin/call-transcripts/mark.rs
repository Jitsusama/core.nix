//! How many words a call's own file got wrong, against a reference of
//! what was said on it, counted as the meetings check counts a meeting
//! (score.rs), so a call heard through Meet's codec and the daemon's own
//! recorder reads beside the meetings: the reference call plays one
//! meeting's far end into Meet, and the file it's written down in is
//! marked against that meeting's reference with Joel's speaker's lines
//! taken out, since they were never played.
//!
//! Only the words count, in order, a side at a time, never when each was
//! said, since a call's clock starts when it's joined and a reference's
//! when its meeting did. With no speaker named as Joel's, every line of
//! the reference is the far end's, and every word written on his side is
//! one added: an echo of theirs his microphone heard, or a sound taken
//! for words. That side is told as words added, never as a share of the
//! none he said.
//!
//! Joel's lines are those under the running account's own name, as the
//! daemon writes them, so a file is marked as the account that wrote it.

use std::path::Path;

use dictation::transcript::{self, Side};
use dictation::wer::{Errors, said, words};
use terminal_kit::{Exit, Help};

use crate::filed;
use crate::score::{self, Said};

/// What `call-transcripts mark --help` says, kept beside what it runs.
pub fn help() -> Help {
    Help::new("mark", "how many words of a call's file are wrong")
        .argument("<transcript>", "the call's transcript.md")
        .argument(
            "<reference>",
            "what was said on it, as a meeting's reference",
        )
        .argument("[mine]", "its speaker on Joel's side, where he spoke")
        .example(
            "call-transcripts mark transcript.md far.ref",
            "every line of far.ref is the far end's",
        )
        .example(
            "call-transcripts mark transcript.md ES2004a.ref B",
            "B's lines are Joel's",
        )
        .exit(
            Exit::Failed,
            "a file can't be read, the reference holds no words, or mine never speaks in it",
        )
}

/// Marks the call's file `file` against `reference`, whose speaker `mine`,
/// where one's named, is Joel's side, and prints each side's errors.
pub fn mark(file: &Path, reference: &Path, mine: Option<&str>) -> Result<(), String> {
    let written = score::read(file)?;
    let said = score::reference(&score::read(reference)?)
        .map_err(|e| format!("{}: {e}", reference.display()))?;
    let errors = marked(&written, &said, mine, &crate::own_name())
        .map_err(|e| format!("{}: {e}", reference.display()))?;
    println!("{}", report(&file.display().to_string(), &errors));
    Ok(())
}

/// Each side's errors, Joel's then theirs, in `file`, a call's file with
/// Joel's lines under `me`, against `reference`, whose speaker `mine`,
/// where one's named, is Joel's side; refused when `mine` never speaks,
/// since a mistyped speaker would move his whole side to the far end, and
/// when the reference holds no words, which would read as none wrong.
fn marked(
    file: &str,
    reference: &[Said],
    mine: Option<&str>,
    me: &str,
) -> Result<[Errors; 2], String> {
    if let Some(mine) = mine
        && !reference.iter().any(|line| line.speaker == mine)
    {
        return Err(format!("{mine} never speaks, so can't be Joel's side"));
    }
    if reference.iter().all(|line| said(&line.text).is_empty()) {
        return Err("it holds no words to mark against".to_owned());
    }
    // With none named, every line is theirs, even one whose speaker is
    // left empty, which no stand-in name could promise.
    let whose = |line: &Said| match mine {
        Some(mine) => score::whose(line, mine),
        None => Side::Theirs,
    };
    let lines = transcript::read(filed::transcript(file), me);
    Ok([Side::Mine, Side::Theirs].map(|side| {
        let spoken: Vec<String> = reference
            .iter()
            .filter(|line| whose(line) == side)
            .flat_map(|line| said(&line.text))
            .collect();
        let heard: Vec<String> = lines
            .iter()
            .filter(|line| line.side == side)
            .flat_map(|line| words(&line.text))
            .collect();
        Errors::count(&spoken, &heard)
    }))
}

/// A call's errors, the whole of it and then each side.
fn report(name: &str, errors: &[Errors; 2]) -> String {
    let mut total = errors[score::index(Side::Mine)];
    total += errors[score::index(Side::Theirs)];
    let side = |side: Side| {
        let errors = &errors[score::index(side)];
        if errors.words == 0 {
            format!("nothing said, {} words added", errors.inserted)
        } else {
            format!("{} ({})", score::rate(errors), score::kinds(errors))
        }
    };
    format!(
        "{name}: {} ({}), Joel's side: {}, the far end: {}",
        score::rate(&total),
        score::kinds(&total),
        side(Side::Mine),
        side(Side::Theirs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reference's line of `speaker`'s, saying `text`.
    fn line(speaker: &str, text: &str) -> Said {
        Said {
            start: 0.0,
            end: 1.0,
            speaker: speaker.to_owned(),
            text: text.to_owned(),
        }
    }

    /// A call's file, summed up, holding `transcript`.
    fn file(transcript: &str) -> String {
        filed::whole(
            "Planning",
            Some("Ada asked to move the review, and Thursday works."),
            transcript,
        )
    }

    #[test]
    fn each_side_is_marked_against_its_own_speakers_lines() {
        let reference = [
            line("B", "Can we move the review?"),
            line("A", "Thursday works for me."),
            line("C", "And for me."),
        ];
        let written = file(
            "Invited: Ada and Bo.\n\n\
             **Joel**, 0:00: Can we move review?\n\n\
             **Others**, 0:04: Thursday works for you. And for me.\n",
        );
        let [mine, theirs] = marked(&written, &reference, Some("B"), "Joel")
            .expect("B speaks, so can be Joel's side");
        assert_eq!(
            mine,
            Errors {
                words: 5,
                substituted: 0,
                deleted: 1,
                inserted: 0
            },
            "Joel's side wasn't marked against B's lines alone"
        );
        assert_eq!(
            theirs,
            Errors {
                words: 7,
                substituted: 1,
                deleted: 0,
                inserted: 0
            },
            "the far end wasn't marked against the others' lines alone"
        );
    }

    #[test]
    fn the_summary_and_the_opening_are_not_words_said() {
        let reference = [line("A", "Thursday works.")];
        let written = file("Invited: Ada and Bo.\n\n**Others**, 0:04: Thursday works.\n");
        assert_eq!(
            marked(&written, &reference, None, "Joel"),
            Ok([
                Errors::default(),
                Errors {
                    words: 2,
                    ..Errors::default()
                }
            ]),
            "words outside the transcript's lines were marked"
        );
    }

    #[test]
    fn words_on_joels_side_where_he_said_none_are_added() {
        // The reference call plays only the far end, so no speaker is
        // Joel's, and what's written on his side is an echo.
        let reference = [line("A", "Thursday works for me.")];
        let written = file(
            "**Others**, 0:00: Thursday works for me.\n\n\
             **Joel**, 0:01: Thursday works.\n",
        );
        let [mine, theirs] =
            marked(&written, &reference, None, "Joel").expect("no speaker is named");
        assert_eq!(
            (mine.words, mine.inserted, theirs.total()),
            (0, 2, 0),
            "an echo on Joel's side wasn't counted as words added"
        );
    }

    #[test]
    fn with_no_speaker_named_every_line_is_the_far_ends() {
        // The meeting's whole reference, B's lines and all, with none
        // named as Joel's: what Joel's side would hold is the far end's.
        let reference = [line("B", "Can we move it?"), line("A", "Yes.")];
        let written = file("**Others**, 0:00: Can we move it? Yes.\n");
        assert_eq!(
            marked(&written, &reference, None, "Joel"),
            Ok([
                Errors::default(),
                Errors {
                    words: 5,
                    ..Errors::default()
                }
            ]),
            "a line was put on Joel's side with no speaker named as his"
        );
    }

    #[test]
    fn with_no_speaker_named_a_line_without_one_is_the_far_ends_too() {
        // A doubled space leaves a reference line's speaker empty, and
        // with none named as Joel's it's still not his.
        let reference = [line("", "A yes.")];
        let written = file("**Others**, 0:00: A yes.\n");
        assert_eq!(
            marked(&written, &reference, None, "Joel"),
            Ok([
                Errors::default(),
                Errors {
                    words: 2,
                    ..Errors::default()
                }
            ]),
            "a line with no speaker was put on Joel's side with none named"
        );
    }

    #[test]
    fn a_reference_with_no_words_cant_be_marked_against() {
        // An empty or wrong file would otherwise read as 0.0% wrong.
        let reference = [line("A", "")];
        let written = file("**Others**, 0:00: Thursday works.\n");
        assert_eq!(
            marked(&written, &reference, None, "Joel"),
            Err("it holds no words to mark against".to_owned()),
            "a reference with no words was marked against"
        );
    }

    #[test]
    fn a_speaker_who_never_speaks_cant_be_joels_side() {
        // A mistyped speaker would otherwise move all of Joel's lines to
        // the far end and still give a number.
        let reference = [line("B", "Can we move it?"), line("A", "Yes.")];
        let written = file("**Joel**, 0:00: Can we move it?\n");
        assert_eq!(
            marked(&written, &reference, Some("b"), "Joel"),
            Err("b never speaks, so can't be Joel's side".to_owned()),
            "a speaker the reference never names was taken as Joel's"
        );
    }

    #[test]
    fn a_side_where_nothing_was_said_is_told_as_words_added() {
        // A share of no words said is no share at all: 0.0% would read as
        // a side heard perfectly, however many words were added to it.
        let echo = Errors {
            inserted: 3,
            ..Errors::default()
        };
        let heard = Errors {
            words: 10,
            substituted: 1,
            ..Errors::default()
        };
        let told = report("call", &[echo, heard]);
        assert!(
            told.contains("Joel's side: nothing said, 3 words added"),
            "Joel's empty side wasn't told as words added: {told}"
        );
        assert!(
            told.contains("the far end: 10.0% of 10 words"),
            "the far end wasn't told as a share of its words: {told}"
        );
    }
}
