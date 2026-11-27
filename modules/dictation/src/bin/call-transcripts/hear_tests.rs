//! The cases of hearing a call, kept beside `hear` so they reach its
//! private parts. Long because it is only cases, each pinning one step of
//! hearing a call: a take's pieces and words, its voices and its names.

use std::path::PathBuf;

use super::*;
use crate::book::Book;
use crate::meet::Seen;
use crate::meeting::Known;

const SECOND: usize = 16_000;

fn line(side: Side, at: usize, voice: Option<usize>) -> Line {
    Line {
        side,
        start: at * SECOND,
        end: (at + 5) * SECOND,
        text: "Said.".to_owned(),
        voice,
        name: None,
    }
}

const ADA: [f32; 2] = [1.0, 0.0];
const BO: [f32; 2] = [0.0, 1.0];

/// Hears each take into `writing`, one piece of 5 s, a line of theirs
/// with a word at its start for each said, each 5 s on by how many lines
/// there are, and never before the last line ends.
fn hear_takes(writing: &mut Writing, takes: &[([f32; 2], bool)]) {
    for &(voice, said) in takes {
        let at = writing.lines.len();
        let line_at = said.then_some(at);
        let after = writing.lines.iter().map(|line| line.end).max().unwrap_or(0);
        let start = ((20 + 5 * at) * SECOND).max(after);
        let words = if said {
            vec![word("Said.", start)]
        } else {
            Vec::new()
        };
        writing.heard(line_at, &words, vec![(voice.to_vec(), start, 5 * SECOND)]);
        if said {
            writing.lines.push(Line {
                start,
                end: start + 5 * SECOND,
                ..line(Side::Theirs, 0, None)
            });
            writing.words.insert(at, words);
        }
    }
}

/// A call heard in one sitting: Ada, Bo, Joel, Ada again, a take of
/// Bo's Parakeet heard no words in, and Bo again.
fn sitting(earlier: Vec<Line>, whole: bool) -> Writing {
    let mut writing = Writing {
        lines: earlier,
        whole,
        ..Writing::default()
    };
    hear_takes(&mut writing, &[(ADA, true), (BO, true)]);
    writing.lines.push(line(Side::Mine, 10, None));
    hear_takes(&mut writing, &[(ADA, true), (BO, false), (BO, true)]);
    writing
}

fn voices(writing: &Writing) -> Vec<Option<usize>> {
    writing.lines.iter().map(|line| line.voice).collect()
}

#[test]
fn each_line_heard_is_given_its_takes_voice() {
    let mut writing = sitting(Vec::new(), true);
    give_voices(&mut writing, None);
    assert_eq!(
        voices(&writing),
        [Some(0), Some(1), None, Some(0), Some(1)],
        "Ada's and Bo's lines weren't given their own voices, or Joel's was given one"
    );
}

/// A take whose embedding is `degrees` round from the first axis.
fn at(degrees: f32) -> [f32; 2] {
    let radians = degrees.to_radians();
    [radians.cos(), radians.sin()]
}

#[test]
fn a_take_with_no_words_helps_tell_the_voices_apart() {
    // Ada's two lines are 0.62 apart, too far to be one voice alone,
    // but a take of hers between them with no words is 0.17 from
    // each, and joined to one it brings the other within 0.6.
    let (first, between, second, bo) = (at(0.0), at(33.8), at(67.67), at(180.0));
    let mut helped = Writing::default();
    hear_takes(
        &mut helped,
        &[(first, true), (between, false), (second, true), (bo, true)],
    );
    give_voices(&mut helped, None);
    let mut unhelped = Writing::default();
    hear_takes(&mut unhelped, &[(first, true), (second, true), (bo, true)]);
    give_voices(&mut unhelped, None);
    assert_eq!(
        voices(&helped),
        [Some(0), Some(0), Some(1)],
        "Ada's lines weren't one voice with her wordless take between them"
    );
    assert_eq!(
        voices(&unhelped),
        [Some(0), Some(1), Some(2)],
        "Ada's lines were one voice without the wordless take, so it proved nothing"
    );
}

fn word(text: &str, at: usize) -> Word {
    Word {
        text: text.to_owned(),
        at,
        level: 0.0,
    }
}

/// A call of Ada's line, Bo's, and a take of 6 s, 40 s in, its first
/// piece Ada's and its second Bo's, heard as the daemon hears them.
fn ada_into_bo() -> Writing {
    let mut writing = Writing {
        whole: true,
        ..Writing::default()
    };
    hear_takes(&mut writing, &[(ADA, true), (BO, true)]);
    hear_take(
        &mut writing,
        40,
        &[ADA, BO],
        &[("Done.", 1), ("Not", 4), ("yet.", 5)],
    );
    writing
}

/// Hears a take into `writing` `at` seconds into the call, a 3 s piece
/// in each of `voices`, its line spelling `said`, each word that many
/// seconds into the take.
fn hear_take(writing: &mut Writing, at: usize, voices: &[[f32; 2]], said: &[(&str, usize)]) {
    let start = at * SECOND;
    let index = writing.lines.len();
    let words: Vec<Word> = said
        .iter()
        .map(|&(text, from)| word(text, start + from * SECOND))
        .collect();
    let pieces = voices
        .iter()
        .enumerate()
        .map(|(i, voice)| (voice.to_vec(), start + 3 * i * SECOND, 3 * SECOND))
        .collect();
    writing.heard(Some(index), &words, pieces);
    let text: Vec<&str> = said.iter().map(|&(text, _)| text).collect();
    writing.lines.push(Line {
        start,
        end: start + 3 * voices.len() * SECOND,
        text: text.join(" "),
        ..line(Side::Theirs, at, None)
    });
    writing.words.insert(index, words);
}

fn said_by(writing: &Writing) -> Vec<(&str, Option<usize>)> {
    writing
        .lines
        .iter()
        .map(|line| (line.text.as_str(), line.voice))
        .collect()
}

#[test]
fn a_take_that_runs_from_one_voice_into_another_is_split_between_them() {
    let mut writing = ada_into_bo();
    give_voices(&mut writing, None);
    let start = 40 * SECOND;
    let lines = said_by(&writing);
    assert_eq!(
        lines,
        [
            ("Said.", Some(0)),
            ("Said.", Some(1)),
            ("Done.", Some(0)),
            ("Not yet.", Some(1)),
        ],
        "the take running from Ada into Bo wasn't split between their voices"
    );
    assert_eq!(
        writing.lines[3].start,
        start + 3 * SECOND,
        "Bo's part doesn't start where his piece does, so the Meet names it by the wrong time"
    );
}

/// A take of 9 s, 40 s in, Ada's for two pieces and Bo's for the last,
/// heard after one take of each.
fn ada_then_bo() -> Writing {
    let mut writing = Writing {
        whole: true,
        ..Writing::default()
    };
    hear_takes(&mut writing, &[(ADA, true), (BO, true)]);
    let timed = [
        ("We", 0),
        ("could", 1),
        ("ship", 2),
        ("it", 3),
        ("today.", 4),
        ("Not", 6),
        ("yet.", 7),
    ];
    hear_take(&mut writing, 40, &[ADA, ADA, BO], &timed);
    writing
}

/// What of Joel's side is written once the others' echo is left out,
/// built on the calls above.
#[path = "hear_echo_tests.rs"]
mod echoes;

#[test]
fn a_takes_pieces_are_timed_from_the_call_and_a_failing_one_left_out() {
    // A 13 s take 40 s in: pieces of 3, 3, 3 and 4 s, two failing apart.
    let samples = vec![0.0; 13 * SECOND];
    let mut tried = 0;
    let (embedded, failed) = embed_take(40 * SECOND, &samples, |_| {
        tried += 1;
        match tried {
            2 => Err("refused".to_owned()),
            3 => Err("busy".to_owned()),
            _ => Ok(Some(ADA.to_vec())),
        }
    });
    let timed: Vec<(usize, usize)> = embedded
        .iter()
        .map(|&(_, at, samples)| (at / SECOND, samples / SECOND))
        .collect();
    assert_eq!(
        timed,
        [(40, 3), (49, 4)],
        "the pieces kept weren't timed from the call's start, or the failing ones weren't left out"
    );
    assert_eq!(
        failed.as_deref(),
        Some("2 of 4 pieces' voice not heard: refused"),
        "the take's failures weren't told in one line with the first one's reason"
    );
    let (_, none) = embed_take(40 * SECOND, &samples, |_| Ok(None));
    assert_eq!(
        none, None,
        "a take with no failing piece was told as failing"
    );
}

#[test]
fn a_piece_is_worded_only_where_a_word_was_said_in_it() {
    let mut writing = Writing::default();
    hear_take(&mut writing, 40, &[ADA, ADA, ADA], &[("Done.", 1)]);
    let worded: Vec<bool> = writing.heard.iter().map(|heard| heard.worded).collect();
    assert_eq!(
        worded,
        [true, false, false],
        "a piece no word was said in counted as speech"
    );
}

#[test]
fn a_piece_of_a_take_with_no_line_is_never_worded() {
    // A take of only hesitation: its words tidied to none, so no line.
    let mut writing = Writing::default();
    let start = 40 * SECOND;
    writing.heard(
        None,
        &[word("Um.", start)],
        vec![(ADA.to_vec(), start, 3 * SECOND)],
    );
    assert!(
        !writing.heard[0].worded,
        "a take of only hesitation counted as speech"
    );
}

#[test]
fn a_piece_of_only_hesitation_in_a_take_with_a_line_isnt_worded() {
    // Bo's backchannel in Ada's take is never a line of its own, since
    // splitting folds a run of hesitation into its neighbour, so it
    // mustn't count toward Bo's being a voice either.
    let mut writing = Writing::default();
    let start = 40 * SECOND;
    let said = [
        word("Ship", start),
        word("it.", start + SECOND),
        word("Mm-hmm.", start + 3 * SECOND),
    ];
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (BO.to_vec(), start + 3 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(0), &said, pieces);
    let worded: Vec<bool> = writing.heard.iter().map(|heard| heard.worded).collect();
    assert_eq!(
        worded,
        [true, false],
        "a piece holding only hesitation counted as speech"
    );
}

#[test]
fn a_piece_holding_only_the_end_of_a_note_isnt_worded() {
    // A note Parakeet writes across the cut between two pieces stays
    // whole with the run it opened in when the take is split, so its end
    // is never a line of its own, and mustn't count toward Bo's being a
    // voice either.
    let mut writing = Writing::default();
    let start = 40 * SECOND;
    let said = [
        word("Fine.", start),
        word("(wind", start + 2 * SECOND),
        word("blowing)", start + 3 * SECOND),
    ];
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (BO.to_vec(), start + 3 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(0), &said, pieces);
    let worded: Vec<bool> = writing.heard.iter().map(|heard| heard.worded).collect();
    assert_eq!(
        worded,
        [true, false],
        "a piece holding only the end of a note counted as speech"
    );
}

#[test]
fn words_in_a_piece_left_unembedded_word_the_piece_before() {
    // Splitting gives the words of a piece the speaker model left out to
    // the run before it, so they count toward that piece's voice here
    // too, or the two would disagree about who spoke.
    let mut writing = Writing::default();
    let start = 40 * SECOND;
    let said = [
        word("Mm.", start),
        word("Not", start + 4 * SECOND),
        word("yet.", start + 5 * SECOND),
    ];
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (BO.to_vec(), start + 6 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(0), &said, pieces);
    let worded: Vec<bool> = writing.heard.iter().map(|heard| heard.worded).collect();
    assert_eq!(
        worded,
        [true, false],
        "words in a piece left unembedded didn't word the piece before"
    );
}

#[test]
fn a_piece_in_no_voice_takes_its_takes_voice_and_stays_in_its_line() {
    let mut writing = Writing {
        whole: true,
        ..Writing::default()
    };
    let both = [(ADA, true), (BO, true)];
    hear_takes(&mut writing, &[both, both, both].concat());
    // A sound like no one's, too little of the call to be a voice, in the
    // middle of a take of Ada's.
    let odd = [-0.7, -0.7];
    let timed = [
        ("We", 0),
        ("could", 1),
        ("ship", 4),
        ("it", 5),
        ("today.", 7),
    ];
    hear_take(&mut writing, 60, &[ADA, odd, ADA], &timed);
    give_voices(&mut writing, None);
    let ada = writing.lines[0].voice;
    assert!(ada.is_some(), "Ada's takes weren't given a voice");
    assert_eq!(
        said_by(&writing).last(),
        Some(&("We could ship it today.", ada)),
        "a piece grouped into no voice cut Ada's line, or took it from her"
    );
}

#[test]
fn a_call_taken_up_after_a_restart_splits_only_the_lines_heard_since() {
    // Read back from the file before any was told apart: no pieces, no
    // voices.
    let earlier = vec![
        line(Side::Theirs, 0, None),
        line(Side::Mine, 5, None),
        line(Side::Theirs, 10, None),
    ];
    let mut writing = sitting(earlier.clone(), false);
    hear_take(
        &mut writing,
        80,
        &[ADA, BO],
        &[("Done.", 1), ("Not", 4), ("yet.", 5)],
    );
    give_voices(&mut writing, None);
    assert_eq!(
        writing.lines[..3],
        earlier[..],
        "a line read back was changed"
    );
    let ada = writing.lines[3].voice;
    let bo = writing.lines[4].voice;
    let tail: Vec<(&str, Option<usize>)> = said_by(&writing).split_off(writing.lines.len() - 2);
    assert_eq!(
        tail,
        [("Done.", ada), ("Not yet.", bo)],
        "the take heard since the restart wasn't split between Ada and Bo"
    );
    let of: Vec<Option<usize>> = writing.pieces.iter().map(|piece| piece.line).collect();
    let last = writing.lines.len();
    assert_eq!(
        of[of.len() - 2..],
        [Some(last - 2), Some(last - 1)],
        "the split take's pieces don't say which lines they're of"
    );
}

#[test]
fn a_call_rejoined_after_a_split_numbers_its_new_lines_past_it() {
    let mut writing = ada_into_bo();
    give_voices(&mut writing, None);
    hear_takes(&mut writing, &[(ADA, true)]);
    give_voices(&mut writing, None);
    assert_eq!(
        said_by(&writing),
        [
            ("Said.", Some(0)),
            ("Said.", Some(1)),
            ("Done.", Some(0)),
            ("Not yet.", Some(1)),
            ("Said.", Some(0)),
        ],
        "a take heard after a rejoin wasn't given its own voice past the split"
    );
    let lines_of: Vec<Option<usize>> = writing.pieces.iter().map(|piece| piece.line).collect();
    assert_eq!(
        lines_of,
        [Some(0), Some(1), Some(2), Some(3), Some(4)],
        "the pieces don't say which lines they're of after the rejoin"
    );
    let mut keys: Vec<usize> = writing.words.keys().copied().collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [0, 1, 2, 3, 4],
        "the words aren't kept by their lines"
    );
}

/// A take heard as scripted: these words, given once, and a failure for
/// a take after it.
struct Scripted {
    words: Option<Vec<Word>>,
}

/// What's `heard`, with timings no case reads, or a failure if nothing.
fn took<T>(heard: Option<T>) -> Result<(T, Timings), String> {
    let took = Timings {
        mel: Duration::ZERO,
        encoder: Duration::ZERO,
        decoder: Duration::ZERO,
        tokens: 0,
    };
    Ok((heard.ok_or("the encoder failed")?, took))
}

impl Hears for Scripted {
    fn words(&mut self, _: &[f32]) -> Result<(Vec<Word>, Timings), String> {
        took(self.words.take())
    }

    fn device(&self) -> &'static str {
        "CPU"
    }
}

#[test]
fn a_takes_words_find_the_pieces_they_were_said_in() {
    // Parakeet times a take's words from the take's start and its pieces
    // are timed from the call's, so words left unmoved 40 s in would all
    // fall in the take's first piece, and a later voice would get none.
    let start = 40 * SECOND;
    let said = vec![word("Not", 0), word("yet.", 4 * SECOND)];
    let mut scripted = Scripted { words: Some(said) };
    let (text, words) = transcribe_take(&mut scripted, start, &[]).unwrap();
    assert_eq!(
        text.as_deref(),
        Some("Not yet."),
        "the words didn't spell the take"
    );
    assert_eq!(
        words,
        [word("Not", 40 * SECOND), word("yet.", 44 * SECOND)],
        "a word wasn't moved by where its take starts in the call"
    );
    let mut writing = Writing::default();
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (BO.to_vec(), start + 3 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(0), &words, pieces);
    let worded: Vec<bool> = writing.heard.iter().map(|heard| heard.worded).collect();
    assert_eq!(
        worded,
        [true, true],
        "a piece didn't find the word said in it"
    );
}

#[test]
fn a_take_that_isnt_heard_says_why() {
    // Either side's take is heard by its words, so one case holds both;
    // the daemon leaves the take out (the next case).
    let mut failing = Scripted { words: None };
    assert_eq!(
        transcribe_take(&mut failing, 40 * SECOND, &[]),
        Err("the encoder failed".to_owned()),
        "a take that wasn't heard didn't say why"
    );
}

#[test]
fn the_daemon_leaves_out_a_take_that_isnt_heard() {
    // One take not heard mustn't end the call or stop its thread: the
    // daemon goes on with no line and no words, as for a take of silence.
    let failed = || Err("the encoder failed".to_owned());
    assert_eq!(
        heard_or_left_out(failed()),
        ((None, Vec::new()), Some("the encoder failed".to_owned())),
        "a take that wasn't heard wasn't left out, with why"
    );
    let heard = || (Some("Not yet.".to_owned()), vec![word("Not", 40 * SECOND)]);
    assert_eq!(
        heard_or_left_out(Ok(heard())),
        (heard(), None),
        "what was heard of a take wasn't kept, or was taken for a failure"
    );
}

#[test]
fn a_call_is_told_once_whichever_failure_comes_first() {
    // A model failing on every take of theirs would raise a notice a
    // take, so a call's first failure is told and the rest aren't, its
    // model not loading or failing on part of it alike; another call's
    // first still is.
    let mut told = HashSet::new();
    let load = || notices::not_transcribed("Standup", "The speech model didn't load: x");
    let part = || notices::not_all_heard("Standup", "the encoder failed");
    let heading = |notice: Option<Notice>| notice.map(|notice| notice.heading);
    assert_eq!(
        heading(told_once(&mut told, "call", load)),
        Some("Standup Couldn't Be Transcribed".to_owned()),
        "the call's first failure wasn't told"
    );
    assert_eq!(
        heading(told_once(&mut told, "call", part)),
        None,
        "a call told its model didn't load was told again of part not heard"
    );
    assert_eq!(
        heading(told_once(&mut told, "another", part)),
        Some("Part of Standup Wasn't Heard".to_owned()),
        "another call wasn't told, since this one was"
    );
    assert_eq!(
        heading(told_once(&mut told, "another", load)),
        None,
        "a call told part wasn't heard was told again its model didn't load"
    );
}

#[test]
fn a_model_not_loading_once_a_call_has_lines_says_part_wasnt_heard() {
    // A call with lines written was transcribed in part, so its model not
    // loading, as after a lost device, leaves only part of it unheard;
    // a call with no line yet is the one that couldn't be transcribed.
    let why = "The speech model didn't load: the device was lost";
    let none = not_loaded("Standup", why, false);
    assert_eq!(
        none.heading, "Standup Couldn't Be Transcribed",
        "a call with no line yet wasn't said to be not transcribed"
    );
    let some = not_loaded("Standup", why, true);
    assert_eq!(
        some.heading, "Part of Standup Wasn't Heard",
        "a call with lines written was said to be not transcribed at all"
    );
    // The reason already says the speech model didn't load, so it's
    // said once, not after a line that it failed on part of it.
    assert_eq!(
        some.body, why,
        "the notice didn't say once that the model didn't load"
    );
}

#[test]
fn a_transcript_says_what_wasnt_heard() {
    // Once the notice is gone, the file is all that says a call's
    // transcript is missing part of it, so it says so, ahead of the
    // record's note and its lines.
    let mut writing = Writing {
        lines: vec![line(Side::Mine, 10, None)],
        ..Writing::default()
    };
    let said = transcript_of(&writing, "Joel", None, Some("Invited: Ada and Bo."));
    assert_eq!(Unheard::read(said.as_str()), None, "all of it was heard");
    writing.unheard = Some(unheard::and(None, 750 * SECOND));
    writing.unheard = Some(unheard::and(writing.unheard, 80 * SECOND));
    let said = transcript_of(&writing, "Joel", None, Some("Invited: Ada and Bo."));
    let unheard = Unheard {
        stretches: 2,
        first: 80 * SECOND,
    };
    assert_eq!(
        Unheard::read(said.as_str()),
        Some(unheard),
        "the transcript doesn't say what wasn't heard: {said}"
    );
    // The summary carries only what comes before the first line, so the
    // note is said ahead of it.
    let at = |text: &str| said.find(text);
    let (note, invited, first) = (
        at(&unheard.note()),
        at("Invited: Ada and Bo."),
        at("**Joel**, 0:10: Said."),
    );
    assert!(
        note.is_some() && note < invited && invited < first,
        "what wasn't heard isn't said ahead of the note and the lines: {said}"
    );
}

#[test]
fn a_take_not_heard_is_written_down_at_once_and_read_back_after_a_restart() {
    // After a lost device every take may fail, so none is heard again
    // to write the file: each take not heard writes it, and a restart
    // reads its count back from the Transcript section alone, not from
    // a summary that quotes the note.
    let folder = tempfile::tempdir().unwrap();
    let book = Shared::new(Book::default());
    let quoted = Unheard {
        stretches: 9,
        first: SECOND,
    };
    book.lock().put(Record {
        folder: folder.path().to_path_buf(),
        summary: Some(quoted.note()),
        ..record(&[], None)
    });
    let mut writing = Writing {
        lines: vec![line(Side::Mine, 10, None)],
        ..Writing::default()
    };
    let mut told = HashSet::new();
    let part = || notices::not_all_heard("Planning", "the encoder failed");
    let mut lose = |at: usize| {
        let take = at..at + SECOND;
        not_heard(
            &book,
            "call:e1",
            &mut writing,
            "Joel",
            take,
            &mut told,
            part,
        )
    };
    let first = lose(80 * SECOND).map(|notice| notice.heading);
    let read_back = || resumed(&book, "call:e1", "Joel").unheard;
    let unheard = |stretches| {
        Some(Unheard {
            stretches,
            first: 80 * SECOND,
        })
    };
    assert_eq!(
        first.as_deref(),
        Some("Part of Planning Wasn't Heard"),
        "the call's first take not heard wasn't told"
    );
    assert_eq!(
        read_back(),
        unheard(1),
        "a take not heard wasn't written down before the next line was heard"
    );
    assert!(
        lose(90 * SECOND).is_none(),
        "a call was told twice of part not heard"
    );
    assert_eq!(
        read_back(),
        unheard(2),
        "a restart didn't read back how much wasn't heard"
    );
}

#[test]
fn takes_of_both_sides_not_heard_over_one_stretch_count_once() {
    // Both sides are recorded at once, so when the model fails on a take
    // of Joel's and one of theirs said over it, one stretch of the call
    // wasn't heard, not two; a take past them is another.
    let folder = tempfile::tempdir().unwrap();
    let book = Shared::new(Book::default());
    book.lock().put(Record {
        folder: folder.path().to_path_buf(),
        ..record(&[], None)
    });
    let mut writing = Writing::default();
    let mut told = HashSet::new();
    let part = || notices::not_all_heard("Planning", "the encoder failed");
    // What's counted not heard of the call once `take` isn't heard too.
    let mut lose = |take: Range<usize>| {
        let _ = not_heard(
            &book,
            "call:e1",
            &mut writing,
            "Joel",
            take,
            &mut told,
            part,
        );
        writing.unheard
    };
    let one = Some(Unheard {
        stretches: 1,
        first: 40 * SECOND,
    });
    assert_eq!(
        lose(40 * SECOND..50 * SECOND),
        one,
        "a take not heard wasn't counted"
    );
    assert_eq!(
        lose(45 * SECOND..52 * SECOND),
        one,
        "a take of each side over one stretch was counted as two stretches"
    );
    assert_eq!(
        lose(52 * SECOND..60 * SECOND).map(|unheard| unheard.stretches),
        Some(2),
        "a take past the stretch counted wasn't counted"
    );
}

#[test]
fn a_call_none_of_which_was_heard_has_a_file_saying_so() {
    // A call whose every take failed from its start, or whose model
    // never loaded, had no file, so a restart before a line was heard
    // left it reading whole from the restart on.
    let folder = tempfile::tempdir().unwrap();
    let book = Shared::new(Book::default());
    book.lock().put(Record {
        folder: folder.path().to_path_buf(),
        ..record(&[], None)
    });
    let mut writing = Writing::default();
    let mut told = HashSet::new();
    let load = || notices::not_transcribed("Planning", "The speech model didn't load: x");
    let told_of = not_heard(
        &book,
        "call:e1",
        &mut writing,
        "Joel",
        0..SECOND,
        &mut told,
        load,
    );
    assert!(
        told_of.is_some(),
        "a call none of which was heard wasn't told"
    );
    let unheard = Some(Unheard {
        stretches: 1,
        first: 0,
    });
    assert_eq!(
        resumed(&book, "call:e1", "Joel").unheard,
        unheard,
        "a call with no line heard had no file saying it wasn't heard"
    );
    let file = std::fs::read_to_string(folder.path().join(TRANSCRIPT)).unwrap();
    assert!(
        file.contains("No words of the call were heard"),
        "a call none of which was heard was said to be heard in part: {file}"
    );
}

#[test]
fn a_call_taken_up_again_is_told_anew_after_a_restart_or_a_rejoin() {
    // Nothing says the notice raised before is still up: it may have
    // been put away, or gone with the daemon before it was shown. So a
    // second notice is raised, rather than none at all.
    let folder = tempfile::tempdir().unwrap();
    let book = Shared::new(Book::default());
    book.lock().put(Record {
        folder: folder.path().to_path_buf(),
        ..record(&[], None)
    });
    let part = || notices::not_all_heard("Planning", "the encoder failed");
    let lose = |key: &str, at: usize, writing: &mut Writing, told: &mut HashSet<String>| {
        not_heard(&book, key, writing, "Joel", at..at + SECOND, told, part)
    };
    let before = lose("call:e1", 0, &mut Writing::default(), &mut HashSet::new());
    assert!(before.is_some(), "the call's first failure wasn't told");
    let mut told = HashSet::new();
    let mut writing = taken_up(&mut Vec::new(), &book, "call:e1", "Joel");
    assert!(
        writing.unheard.is_some(),
        "a call taken up after a restart lost what its file said wasn't heard"
    );
    assert!(
        lose("call:e1", SECOND, &mut writing, &mut told).is_some(),
        "a call its file said wasn't heard in part wasn't told anew after a restart"
    );
    assert!(
        lose("call:e1", 2 * SECOND, &mut writing, &mut told).is_none(),
        "a call was told twice in one sitting"
    );
    let held = Writing {
        unheard: Some(unheard::and(None, 0)),
        ..Writing::default()
    };
    let mut ended = vec![("call:e1".to_owned(), held)];
    let mut told = HashSet::new();
    let mut writing = taken_up(&mut ended, &book, "call:e1", "Joel");
    assert!(
        writing.unheard.is_some(),
        "a call rejoined after it ended lost what this run held wasn't heard"
    );
    assert!(
        lose("call:e1", 2 * SECOND, &mut writing, &mut told).is_some(),
        "a call rejoined after it ended wasn't told anew"
    );
}

#[test]
fn a_call_put_away_is_told_anew_through_the_same_told_calls() {
    // The calls told of a failure are held as long as the daemon runs,
    // so a call written down whole must leave them, or a failure once
    // it's rejoined goes untold.
    let folder = tempfile::tempdir().unwrap();
    let book = Shared::new(Book::default());
    book.lock().put(Record {
        folder: folder.path().to_path_buf(),
        ..record(&[], None)
    });
    let part = || notices::not_all_heard("Planning", "the encoder failed");
    let lose = |writing: &mut Writing, at: usize, told: &mut HashSet<String>| {
        not_heard(
            &book,
            "call:e1",
            writing,
            "Joel",
            at..at + SECOND,
            told,
            part,
        )
    };
    let mut calls: HashMap<String, Writing> = HashMap::new();
    let (mut ended, mut told, mut unsummed) = (Vec::new(), HashSet::new(), Vec::new());
    let writing = calls.entry("call:e1".to_owned()).or_default();
    assert!(
        lose(writing, 0, &mut told).is_some(),
        "the call's first failure wasn't told"
    );
    let key = "call:e1".to_owned();
    put_away(&mut calls, &mut ended, &mut told, &mut unsummed, key);
    assert_eq!(
        unsummed,
        ["call:e1"],
        "a call put away wasn't given to be summed up"
    );
    let mut writing = taken_up(&mut ended, &book, "call:e1", "Joel");
    assert!(
        writing.unheard.is_some(),
        "a call put away wasn't kept as it was heard"
    );
    assert!(
        lose(&mut writing, SECOND, &mut told).is_some(),
        "a call put away and rejoined wasn't told anew of a failure"
    );
}

#[test]
fn a_call_rejoined_in_this_run_is_told_apart_whole() {
    let mut writing = sitting(Vec::new(), true);
    give_voices(&mut writing, None);
    hear_takes(&mut writing, &[(BO, true), (ADA, true)]);
    give_voices(&mut writing, None);
    assert_eq!(
        voices(&writing),
        [Some(0), Some(1), None, Some(0), Some(1), Some(1), Some(0)],
        "the lines after a rejoin weren't given the voices the call already had"
    );
}

#[test]
fn a_call_taken_up_again_keeps_the_voices_it_had_and_its_new_lines_stay_others() {
    let earlier = vec![
        line(Side::Theirs, 0, Some(0)),
        line(Side::Theirs, 5, Some(1)),
    ];
    let mut writing = sitting(earlier, false);
    give_voices(&mut writing, None);
    let voices: Vec<Option<usize>> = writing.lines.iter().map(|line| line.voice).collect();
    assert_eq!(
        &voices[..2],
        [Some(0), Some(1)],
        "the voices told apart before the rejoin were wiped"
    );
    assert!(
        voices[2..].iter().all(Option::is_none),
        "lines after a restart were numbered anew beside the old voices: {voices:?}"
    );
}

const STARTED: u64 = 1_800_000_000_000;
const HOUR: u64 = 3_600_000;

fn present(name: &str, joined_s: u64) -> Present {
    Present {
        name: name.to_owned(),
        joined: STARTED + joined_s * 1000,
        left: None,
    }
}

/// A Meet call with `people` on it, as its record has them.
fn in_meet(people: &[&str]) -> Record {
    Record {
        started: STARTED,
        ..record(people, Some(Known::Meet))
    }
}

/// Who was in, as a Meet last looked at an hour in says.
fn seen(present: &[Present]) -> Attendance {
    Attendance {
        present: present.to_vec(),
        looked: STARTED + HOUR,
    }
}

/// A line of theirs in `voice`, said from `from` seconds in to `to`.
fn take_of(voice: usize, from: usize, to: usize) -> Line {
    Line {
        start: from * SECOND,
        end: to * SECOND,
        ..line(Side::Theirs, from, Some(voice))
    }
}

fn names(lines: &[Line]) -> Vec<Option<&str>> {
    lines.iter().map(|line| line.name.as_deref()).collect()
}

#[test]
fn a_voice_heard_only_while_one_other_was_in_takes_that_name_and_the_rest_are_guessed() {
    let mut lines = vec![
        line(Side::Theirs, 10, Some(0)),
        line(Side::Mine, 30, None),
        line(Side::Theirs, 60, Some(0)),
        line(Side::Theirs, 600, Some(1)),
        line(Side::Theirs, 620, None),
    ];
    let seen = seen(&[present("Ada", 0), present("Bo", 300)]);
    let guesses = give_names(&mut lines, &in_meet(&["Ada", "Bo"]), &seen);
    assert_eq!(
        names(&lines),
        [Some("Ada"), None, Some("Ada"), None, None],
        "Ada's voice wasn't named for her, or Joel's line or Bo's or Others' was named"
    );
    assert_eq!(
        guesses.as_deref(),
        Some("Voice 2 is likely Bo."),
        "the voice heard while both were in wasn't said only likely Bo"
    );
}

#[test]
fn with_nobody_seen_in_the_meet_no_voice_is_named_or_guessed() {
    let mut lines = vec![line(Side::Theirs, 10, Some(0))];
    let guesses = give_names(&mut lines, &in_meet(&["Ada", "Bo"]), &seen(&[]));
    assert_eq!(
        guesses, None,
        "a voice was guessed with nobody seen in the Meet"
    );
    assert_eq!(
        names(&lines),
        [None],
        "a voice was named for who the record has, with nobody seen in the Meet"
    );
}

#[test]
fn someone_on_the_call_meet_gave_no_time_for_keeps_a_voice_from_another_name() {
    let mut lines = vec![line(Side::Theirs, 10, Some(0))];
    let seen = seen(&[present("Ada", 0)]);
    let guesses = give_names(&mut lines, &in_meet(&["Ada", "Bo"]), &seen);
    assert_eq!(
        names(&lines),
        [None],
        "a voice was named Ada while Bo, with no time in the Meet, may have been in"
    );
    assert_eq!(
        guesses.as_deref(),
        Some("Voice 1 may be Ada or Bo."),
        "Bo, with no time in the Meet, wasn't someone the voice may be"
    );
}

#[test]
fn what_was_said_after_the_meet_was_last_looked_at_names_no_voice() {
    let mut lines = vec![line(Side::Theirs, 600, Some(0))];
    let seen = Attendance {
        looked: STARTED + 100_000,
        ..seen(&[present("Ada", 0)])
    };
    let guesses = give_names(&mut lines, &in_meet(&["Ada"]), &seen);
    assert_eq!(
        (names(&lines), guesses),
        (vec![None], None),
        "a voice heard after the last look was named or guessed, though someone may have joined"
    );
}

#[test]
fn a_take_running_past_someones_leaving_isnt_theirs() {
    let mut lines = vec![take_of(0, 250, 400)];
    let ada = Present {
        left: Some(STARTED + 300_000),
        ..present("Ada", 0)
    };
    let seen = seen(&[ada, present("Bo", 0)]);
    give_names(&mut lines, &in_meet(&["Ada", "Bo"]), &seen);
    assert_eq!(
        names(&lines),
        [Some("Bo")],
        "a take that ran on after Ada left wasn't named for Bo, the only one in throughout"
    );
}

#[test]
fn a_call_rejoined_in_this_run_is_named_anew() {
    let mut writing = sitting(Vec::new(), true);
    writing.lines[0].name = Some("Ada".to_owned());
    assert!(
        give_voices(&mut writing, None),
        "a call rejoined in this run wasn't grouped anew"
    );
    assert_eq!(
        writing.lines[0].name, None,
        "a name from before the rejoin outlived the voices it was given by"
    );
}

#[test]
fn a_call_taken_up_again_keeps_the_names_it_had() {
    let named = Line {
        name: Some("Ada".to_owned()),
        ..line(Side::Theirs, 0, None)
    };
    let mut writing = sitting(vec![named], false);
    assert!(
        !give_voices(&mut writing, None),
        "a call taken up again was grouped anew beside the voices it had"
    );
    assert_eq!(
        writing.lines[0].name.as_deref(),
        Some("Ada"),
        "the name given before a restart was lost"
    );
}

fn record(people: &[&str], known: Option<Known>) -> Record {
    Record {
        key: "call:e1".to_owned(),
        title: "Planning".to_owned(),
        started: 0,
        ended: None,
        event: Some("e1".to_owned()),
        person: None,
        account: None,
        people: people.iter().map(|&someone| someone.to_owned()).collect(),
        known,
        folder: PathBuf::new(),
        transcript: None,
        opening: None,
        summary: None,
        transcribing: false,
    }
}

#[test]
fn a_voice_is_named_from_what_a_look_at_the_meet_saw() {
    let folder = tempfile::tempdir().unwrap();
    let mut ada = in_meet(&["Ada", "Bo"]);
    ada.folder = folder.path().join("call:e1");
    let book = Shared::new(Book::default());
    book.lock().put(ada);
    let look = Seen {
        people: vec!["Ada".to_owned(), "Bo".to_owned()],
        known: Some(Known::Meet),
        // Ada's takes are 20 s and 25 s in, and Bo's 30 s and 35 s; Bo
        // joining 58 s in is after the first two by more than DRIFT.
        present: vec![present("Ada", 0), present("Bo", 58)],
    };
    book.lock().saw("call:e1", look, STARTED + HOUR);
    let mut writing = Writing {
        whole: true,
        ..Writing::default()
    };
    hear_takes(
        &mut writing,
        &[(ADA, true), (ADA, true), (BO, true), (BO, true)],
    );
    tell_apart(&book, "call:e1", &mut writing, "Joel");
    assert_eq!(
        names(&writing.lines),
        [Some("Ada"), Some("Ada"), None, None],
        "a voice heard while only Ada was in, by the Meet's last look, wasn't named for her"
    );
    assert_eq!(
        writing.guesses.as_deref(),
        Some("Voice 2 is likely Bo."),
        "the voice heard once Bo was in too wasn't said likely Bo"
    );
}

#[test]
fn a_name_read_back_stays_only_while_the_record_names_no_side() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("transcript.md");
    let named = Line {
        name: Some("Ada".to_owned()),
        ..line(Side::Theirs, 0, Some(0))
    };
    let note = "Voice 2 may be Bo.";
    let text = transcript::markdown_naming(&[named], "Joel", None, None, Some(note));
    std::fs::write(&file, text).unwrap();
    let read_back = |people: &[&str]| {
        let mut call = in_meet(people);
        call.folder = folder.path().join("call:e1");
        call.transcript = Some(file.clone());
        let book = Shared::new(Book::default());
        book.lock().put(call);
        let writing = resumed(&book, "call:e1", "Joel");
        writing.lines.first().and_then(|line| line.name.clone())
    };
    assert_eq!(
        read_back(&["Ada", "Bo"]).as_deref(),
        Some("Ada"),
        "a voice named before a restart lost its name"
    );
    assert_eq!(
        read_back(&["Ada"]),
        None,
        "a name read back was kept as a voice's where it is their side's"
    );
}

#[test]
fn voices_are_told_apart_only_where_their_side_is_unnamed_and_no_more_than_were_on() {
    assert_eq!(
        their_side(&record(&["Ada"], Some(Known::Invitation)), "Joel"),
        TheirSide::Named,
        "voices were told apart on a call whose one other person is named"
    );
    assert_eq!(
        their_side(&record(&["Ada", "Bo"], Some(Known::Tuple)), "Joel"),
        TheirSide::Unnamed { people: Some(2) },
        "with two others known, the voices weren't held to two"
    );
    assert_eq!(
        their_side(&record(&[], None), "Joel"),
        TheirSide::Unnamed { people: None },
        "with nobody known, the voices were held to a count"
    );
}

#[test]
fn a_speaker_model_that_wont_load_is_tried_once_until_let_go() {
    let mut speakers = Speakers::default();
    let mut tries = 0;
    for _ in 0..3 {
        let model = speakers.get(|| {
            tries += 1;
            Err("no model".to_owned())
        });
        assert!(model.is_none(), "a model that won't load was given");
    }
    assert_eq!(
        tries, 1,
        "a model that won't load was read again for each take"
    );
    speakers = Speakers::default();
    let _ = speakers.get(|| {
        tries += 1;
        Err("no model".to_owned())
    });
    assert_eq!(tries, 2, "a model let go wasn't tried again");
}
