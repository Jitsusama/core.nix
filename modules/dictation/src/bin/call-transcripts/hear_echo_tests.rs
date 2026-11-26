//! What of Joel's side a call being heard writes once the others' echo
//! is left out: judged against each take of theirs whole where it's split
//! by voice, cut from his lines by word, and kept across a restart. It
//! builds its calls with the helpers of hear.rs's tests, its parent.

use super::*;

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
        echo::echoes(&answer, &writing.lines),
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
        !echo::echoes(&across, &writing.lines),
        "the echo across the cut is one of a part's, so leaving it out shows nothing"
    );
    assert_eq!(
        joels_written(&writing),
        ["Right."],
        "an echo of the split take, or of Bo's take after it, was written as Joel's"
    );
}

#[test]
fn their_echo_in_a_take_of_joels_is_cut_and_his_own_words_written() {
    let mut writing = ada_then_bo();
    // Joel says "Right.", and his microphone hears Bo's "Not yet." 50 ms
    // after Bo says it, in the same take. Judged whole, the take shares
    // two of the seven words of the one it echoes, so it's no echo.
    let late = SECOND / 20;
    let words = vec![
        word("Right.", 44 * SECOND),
        word("Not", 46 * SECOND + late),
        word("yet.", 47 * SECOND + late),
    ];
    writing.words.insert(writing.lines.len(), words);
    writing.lines.push(joels(44, 48, "Right. Not yet."));
    give_voices(&mut writing, None);
    assert_eq!(
        joels_written(&writing),
        ["Right."],
        "Bo's echo was written as Joel's, or Joel's own word went with it"
    );
}

#[test]
fn each_echo_cut_from_joels_words_is_told_once_and_without_them() {
    let mut writing = ada_then_bo();
    let late = SECOND / 20;
    let words = vec![
        word("Right.", 44 * SECOND),
        word("Not", 46 * SECOND + late),
        word("yet.", 47 * SECOND + late),
    ];
    writing.words.insert(writing.lines.len(), words);
    writing.lines.push(joels(44, 48, "Right. Not yet."));
    assert_eq!(
        writing.untold_cuts(),
        ["call-transcripts: 2 of Joel's words at 0:46 cut as their echo, 50 ms after theirs"],
        "the cut wasn't told, or was told with the call's words"
    );
    hear_take(&mut writing, 60, &[ADA], &[("Right.", 0)]);
    assert!(
        writing.untold_cuts().is_empty(),
        "a cut told when its line was written was told again with the next take"
    );
}

#[test]
fn an_echo_cut_for_its_level_is_told_by_how_far_below_theirs_it_was() {
    // His microphone hears Bo's "Not yet." 20 dB down, its own way, in a
    // take of his that has his voice in it alone first.
    let mut writing = ada_then_bo();
    let late = SECOND / 20;
    let mut words: Vec<Word> = ["Fine", "by", "me,", "thanks", "all."]
        .iter()
        .zip(10..)
        .map(|(text, at)| word(text, at * SECOND))
        .collect();
    for (text, at) in [("Knot", 46), ("yep.", 47)] {
        words.push(Word {
            level: -20.0,
            ..word(text, at * SECOND + late)
        });
    }
    writing.words.insert(writing.lines.len(), words);
    writing
        .lines
        .push(joels(10, 48, "Fine by me, thanks all. Knot yep."));
    assert_eq!(
        writing.untold_cuts(),
        ["call-transcripts: 2 of Joel's words at 0:46 cut as their echo, 20 dB below theirs"],
        "the level cut was told by how late it was, or with the call's words"
    );
}

#[test]
fn each_line_of_joels_left_out_whole_as_their_echo_is_told_once_and_without_it() {
    let mut writing = ada_then_bo();
    hear_take(&mut writing, 49, &[BO], &[("Sounds", 0), ("good.", 1)]);
    writing.lines.push(joels(49, 51, "Sounds good."));
    assert_eq!(
        writing.untold_cuts(),
        ["call-transcripts: a line of Joel's, 2 words at 0:49, left out whole as their echo"],
        "the line left out wasn't told, or was told with the call's words"
    );
    assert!(
        joels_written(&writing).is_empty(),
        "the line told as left out was written, so the journal and the file disagree"
    );
    hear_take(&mut writing, 60, &[ADA], &[("Right.", 0)]);
    assert!(
        writing.untold_cuts().is_empty(),
        "a line told when it was left out was told again with the next take"
    );
}

#[test]
fn what_was_told_left_out_stays_told_once_the_lines_are_split_by_voice() {
    let mut writing = ada_then_bo();
    let late = SECOND / 20;
    let words = vec![
        word("Right.", 44 * SECOND),
        word("Not", 46 * SECOND + late),
        word("yet.", 47 * SECOND + late),
    ];
    writing.words.insert(writing.lines.len(), words);
    writing.lines.push(joels(44, 48, "Right. Not yet."));
    hear_take(&mut writing, 49, &[BO], &[("Sounds", 0), ("good.", 1)]);
    writing.lines.push(joels(49, 51, "Sounds good."));
    assert_eq!(
        writing.untold_cuts().len(),
        2,
        "the cut from Joel's words and his line left out whole weren't each told"
    );
    // The call ends and its takes are split by voice, moving each line
    // of Joel's after the split take on by one; then it's rejoined.
    let before = writing.lines.len();
    give_voices(&mut writing, None);
    assert!(
        writing.lines.len() > before,
        "no take was split, so Joel's lines never moved and this shows nothing"
    );
    hear_take(&mut writing, 60, &[ADA], &[("Right.", 0)]);
    assert_eq!(
        writing.untold_cuts(),
        Vec::<String>::new(),
        "what was told before the split was told again once Joel's lines moved"
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
        echo::echoes(answer, read_back),
        "Joel's answer, read back, isn't an echo of Bo's part, so keeping it shows nothing"
    );
    // Joel's side hears Bo's part again, the restart having cut his
    // recording there, so it's an echo of a line read back. A line read
    // back ends where it starts, 46 s in, so it's heard within the echo's
    // slack of that.
    let again = joels(48, 49, "Not yet.");
    assert!(
        echo::echoes(&again, &writing.lines[..writing.read]),
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
