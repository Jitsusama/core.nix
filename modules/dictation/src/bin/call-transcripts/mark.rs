//! How many words a call's own file got wrong, against a reference of
//! what was said on it, counted as the meetings check counts a meeting
//! (score.rs), so a call heard through Meet's codec and the daemon's own
//! recorder reads beside the meetings: the reference call plays one
//! meeting's far end into Meet, and the file it's written down in is
//! marked against that meeting's reference.
//!
//! Only the words count, in order, a side at a time, never when each was
//! said, since a call's clock starts when it's joined and a reference's
//! when its meeting did. Where the reference holds none of Joel's lines,
//! as it doesn't for a call that plays only the far end, every word
//! written on his side is one added: an echo of theirs his microphone
//! heard, or a sound taken for words.

use std::path::Path;

use dictation::transcript::{self, Side};
use dictation::wer::{Errors, said, words};

use crate::filed;
use crate::score::{self, Said};

/// Marks the call's file `file` against `reference`, whose speaker `mine`
/// is Joel's side, and prints each side's errors.
pub fn mark(file: &Path, reference: &Path, mine: &str) -> Result<(), String> {
    let written = score::read(file)?;
    let said = score::reference(&score::read(reference)?)
        .map_err(|e| format!("{}: {e}", reference.display()))?;
    let errors = marked(&written, &said, mine, &crate::own_name());
    println!("{}", report(&file.display().to_string(), &errors));
    Ok(())
}

/// Each side's errors, Joel's then theirs, in `file`, a call's file with
/// Joel's lines under `me`, against `reference`, whose speaker `mine` is
/// Joel's side.
fn marked(file: &str, reference: &[Said], mine: &str, me: &str) -> [Errors; 2] {
    let lines = transcript::read(filed::transcript(file), me);
    [Side::Mine, Side::Theirs].map(|side| {
        let spoken: Vec<String> = reference
            .iter()
            .filter(|line| score::whose(line, mine) == side)
            .flat_map(|line| said(&line.text))
            .collect();
        let heard: Vec<String> = lines
            .iter()
            .filter(|line| line.side == side)
            .flat_map(|line| words(&line.text))
            .collect();
        Errors::count(&spoken, &heard)
    })
}

/// A call's errors, the whole of it and then each side.
fn report(name: &str, errors: &[Errors; 2]) -> String {
    let mut total = errors[score::index(Side::Mine)];
    total += errors[score::index(Side::Theirs)];
    let side = |side: Side| {
        let errors = &errors[score::index(side)];
        format!("{} ({})", score::rate(errors), score::kinds(errors))
    };
    format!(
        "{name}: {} ({}), Joel's side {}, the far end {}",
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
        let [mine, theirs] = marked(&written, &reference, "B", "Joel");
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
            marked(&written, &reference, "B", "Joel"),
            [
                Errors::default(),
                Errors {
                    words: 2,
                    ..Errors::default()
                }
            ],
            "words outside the transcript's lines were marked"
        );
    }

    #[test]
    fn words_on_joels_side_where_he_said_none_are_added() {
        // The reference call plays only the far end, so Joel's side of
        // its reference is empty, and what's written there is an echo.
        let reference = [line("A", "Thursday works for me.")];
        let written = file(
            "**Others**, 0:00: Thursday works for me.\n\n\
             **Joel**, 0:01: Thursday works.\n",
        );
        let [mine, theirs] = marked(&written, &reference, "B", "Joel");
        assert_eq!(
            (mine.words, mine.inserted, theirs.total()),
            (0, 2, 0),
            "an echo on Joel's side wasn't counted as words added"
        );
    }
}
