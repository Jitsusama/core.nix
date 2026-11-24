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

/// A line of Joel's from `from` to `to` seconds into the call.
fn joels(from: usize, to: usize, text: &str) -> Line {
    Line {
        start: from * SECOND,
        end: to * SECOND,
        text: text.to_owned(),
        ..line(Side::Mine, from, None)
    }
}

/// What of Joel's `writing`'s transcript holds, as its file is read.
fn joels_written(writing: &Writing) -> Vec<String> {
    transcript::read(&transcript_of(writing, "Joel", None, None), "Joel")
        .into_iter()
        .filter(|line| line.side == Side::Mine)
        .map(|line| line.text)
        .collect()
}

/// `ada_then_bo` told apart, with Joel saying Bo's part back as his own
/// answer, 47 s in, where he'd be taken for its echo if judged against
/// the part alone.
fn answered() -> Writing {
    let mut writing = ada_then_bo();
    let answer = joels(47, 48, "Not yet.");
    writing.lines.push(answer.clone());
    give_voices(&mut writing, None);
    let bo = writing.lines[1].voice;
    assert!(
        said_by(&writing).contains(&("Not yet.", bo)),
        "the take wasn't split, so the answer is never judged near Bo's part alone"
    );
    assert!(
        transcript::echoes(&answer, &writing.lines),
        "the answer isn't an echo of Bo's part alone, so keeping it shows nothing"
    );
    writing
}

#[test]
fn joels_answer_to_one_part_of_a_split_take_is_written() {
    assert_eq!(
        joels_written(&answered()),
        ["Not yet."],
        "Joel's answer was taken for an echo of Bo's part, not judged against the whole take"
    );
}

#[test]
fn joels_echoes_of_their_takes_are_left_out_of_what_is_written() {
    let mut writing = ada_then_bo();
    // Bo again, straight after, in a take of his own.
    hear_take(&mut writing, 49, &[BO], &[("Sounds", 0), ("good.", 1)]);
    // Joel's microphone hearing the split take across its cut, and Bo's
    // short take, then Joel saying something of his own.
    let across = joels(43, 48, "It today, not yet.");
    writing.lines.push(across.clone());
    writing.lines.push(joels(49, 51, "Sounds good."));
    writing.lines.push(joels(60, 61, "Right."));
    give_voices(&mut writing, None);
    let bo = writing.lines[1].voice;
    assert!(
        said_by(&writing).contains(&("Not yet.", bo)),
        "the take wasn't split, so its echo across the cut shows nothing"
    );
    assert!(
        !transcript::echoes(&across, &writing.lines),
        "the echo across the cut is one of a part's, so leaving it out shows nothing"
    );
    assert_eq!(
        joels_written(&writing),
        ["Right."],
        "an echo of the split take, or of Bo's take after it, was written as Joel's"
    );
}

#[test]
fn joels_answer_to_a_split_take_stays_written_after_a_restart() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("transcript.md");
    std::fs::write(&file, transcript_of(&answered(), "Joel", None, None)).unwrap();
    let mut call = in_meet(&["Ada", "Bo"]);
    call.folder = folder.path().join("call:e1");
    call.transcript = Some(file);
    let book = Shared::new(Book::default());
    book.lock().put(call);
    // Taken up from the file, Bo's part is a line of its own with no
    // pieces to join it to Ada's, and with voices read back nothing since
    // is grouped. Ada says more, heard on Joel's side as an echo.
    let mut writing = resumed(&book, "call:e1", "Joel");
    let read_back = &writing.lines[..writing.read];
    let answer = read_back
        .iter()
        .find(|line| line.side == Side::Mine)
        .unwrap();
    assert!(
        transcript::echoes(answer, read_back),
        "Joel's answer, read back, isn't an echo of Bo's part, so keeping it shows nothing"
    );
    // Joel's side hears Bo's part again, the restart having cut his
    // recording there, so it's an echo of a line read back. A line read
    // back ends where it starts, 46 s in, so it's heard within the echo's
    // slack of that.
    let again = joels(48, 49, "Not yet.");
    assert!(
        transcript::echoes(&again, &writing.lines[..writing.read]),
        "Bo's part heard again isn't an echo of it, so leaving it out shows nothing"
    );
    writing.lines.push(again);
    hear_take(&mut writing, 80, &[ADA], &[("Right.", 0)]);
    writing.lines.push(joels(80, 81, "Right."));
    give_voices(&mut writing, None);
    assert_eq!(
        joels_written(&writing),
        ["Not yet."],
        "Joel's answer, written before the restart, was judged again against Bo's part alone, \
         or an echo heard since, of a line read back or of Ada's, wasn't"
    );
}

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

/// A take heard as scripted: these words for theirs and this text for
/// Joel's, each given once, and a failure for what isn't scripted.
struct Scripted {
    words: Option<Vec<Word>>,
    text: Option<String>,
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

    fn text(&mut self, _: &[f32]) -> Result<(String, Timings), String> {
        took(self.text.take())
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
    let mut scripted = Scripted {
        words: Some(vec![word("Not", 0), word("yet.", 4 * SECOND)]),
        text: None,
    };
    let (text, words) = transcribe_take(&mut scripted, Side::Theirs, start, &[]).unwrap();
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
fn joels_take_is_kept_as_spelled_with_no_words() {
    let mut scripted = Scripted {
        words: None,
        text: Some(" Done. ".to_owned()),
    };
    assert_eq!(
        transcribe_take(&mut scripted, Side::Mine, 40 * SECOND, &[]),
        Ok((Some("Done.".to_owned()), Vec::new())),
        "Joel's text wasn't kept as Parakeet spelled it, without words"
    );
}

#[test]
fn a_take_that_isnt_heard_says_why() {
    // The `transcribe` subcommand stops on why, so a check can't pass on
    // a failing model; the daemon leaves the take out (the next case).
    let mut failing = Scripted {
        words: None,
        text: None,
    };
    let why = Err("the encoder failed".to_owned());
    assert_eq!(
        transcribe_take(&mut failing, Side::Theirs, 40 * SECOND, &[]),
        why,
        "a take of theirs that wasn't heard didn't say why"
    );
    assert_eq!(
        transcribe_take(&mut failing, Side::Mine, 40 * SECOND, &[]),
        why,
        "a take of Joel's that wasn't heard didn't say why"
    );
}

#[test]
fn the_daemon_leaves_out_a_take_that_isnt_heard() {
    // One take not heard mustn't end the call or stop its thread: the
    // daemon goes on with no line and no words, as for a take of silence.
    assert_eq!(
        heard_or_left_out(Err("the encoder failed".to_owned())),
        (None, Vec::new()),
        "a take that wasn't heard wasn't left out"
    );
    let heard = || (Some("Not yet.".to_owned()), vec![word("Not", 40 * SECOND)]);
    assert_eq!(
        heard_or_left_out(Ok(heard())),
        heard(),
        "what was heard of a take wasn't kept"
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
