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
/// with a word at its start for each said.
fn hear_takes(writing: &mut Writing, takes: &[([f32; 2], bool)]) {
    for &(voice, said) in takes {
        let at = writing.lines.len();
        let line_at = said.then_some(at);
        let start = (20 + 5 * at) * SECOND;
        let words = if said {
            vec![word("Said.", start)]
        } else {
            Vec::new()
        };
        writing.heard(line_at, &words, vec![(voice.to_vec(), start, 5 * SECOND)]);
        if said {
            writing.lines.push(line(Side::Theirs, 20 + 5 * at, None));
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
    let start = 40 * SECOND;
    let index = writing.lines.len();
    let said = [("Done.", 1), ("Not", 4), ("yet.", 5)];
    let words = said.map(|(text, at)| word(text, start + at * SECOND));
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (BO.to_vec(), start + 3 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(index), &words, pieces);
    writing.lines.push(Line {
        start,
        end: start + 6 * SECOND,
        text: "Done. Not yet.".to_owned(),
        ..line(Side::Theirs, 40, None)
    });
    writing.words.insert(index, words.into());
    writing
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

#[test]
fn joels_answer_to_one_part_of_a_split_take_is_written() {
    let mut writing = Writing {
        whole: true,
        ..Writing::default()
    };
    hear_takes(&mut writing, &[(ADA, true), (BO, true)]);
    // A take of 9 s, 40 s in, Ada's for two pieces and Bo's for the last,
    // and Joel saying Bo's words back as his own answer.
    let start = 40 * SECOND;
    let index = writing.lines.len();
    let timed = [
        ("We", 0),
        ("could", 1),
        ("ship", 2),
        ("it", 3),
        ("today.", 4),
        ("Not", 6),
        ("yet.", 7),
    ];
    let words = timed.map(|(text, at)| word(text, start + at * SECOND));
    let pieces = vec![
        (ADA.to_vec(), start, 3 * SECOND),
        (ADA.to_vec(), start + 3 * SECOND, 3 * SECOND),
        (BO.to_vec(), start + 6 * SECOND, 3 * SECOND),
    ];
    writing.heard(Some(index), &words, pieces);
    writing.lines.push(Line {
        start,
        end: start + 9 * SECOND,
        text: "We could ship it today. Not yet.".to_owned(),
        ..line(Side::Theirs, 40, None)
    });
    writing.words.insert(index, words.into());
    writing.lines.push(Line {
        start: 50 * SECOND,
        end: 51 * SECOND,
        text: "Not yet.".to_owned(),
        ..line(Side::Mine, 50, None)
    });
    give_voices(&mut writing, None);
    let written = transcript::read(&transcript_of(&writing, "Joel", None, None), "Joel");
    let joels: Vec<&str> = written
        .iter()
        .filter(|line| line.side == Side::Mine)
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(
        joels,
        ["Not yet."],
        "Joel's answer was taken for an echo of Bo's part, not judged against the whole take"
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

#[test]
fn a_takes_words_are_timed_from_the_calls_start() {
    let words = from_call_start(40 * SECOND, vec![word("Done.", SECOND)]);
    assert_eq!(
        words,
        [word("Done.", 41 * SECOND)],
        "a word wasn't moved by where its take starts in the call"
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
