//! The cases of cutting Joel's words far under the others' sound, held
//! to calls made up word by word over a track of their sound. It builds
//! its calls with the helpers of echo.rs's tests, its parent.

use super::*;

/// A track of their sound at 0 dB of full scale, by its power, from
/// `from_ms` to `to_ms` milliseconds into the call, as a take of theirs.
fn their_sound(from_ms: usize, to_ms: usize) -> Track {
    let (from, to) = (from_ms * RATE / 1000, to_ms * RATE / 1000);
    let mut track = Track::default();
    track.hear(from, &vec![1.0; to - from]);
    track
}

/// What's written of Joel's line, and what each cut from it was cut by,
/// over their `sound`, when he says "Fine by me." at -20 dB a second into
/// the call and "Right." at `level` dB `at_ms` in, and they say "Lunch at
/// noon on Thursday then." at -20 dB from 5 s, their line ending at 6.8 s.
fn right_over(sound: &Track, at_ms: usize, level: f32) -> (Vec<String>, Vec<String>) {
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = at_level(timed("Fine by me.", 1_000), -20.0);
    mine.extend(at_level(timed("Right.", at_ms), level));
    let (lines, words) = lines_heard(&[theirs], &[mine]);
    let (cut, cuts) = cut_echoes(&lines, &words, sound, |left| Some(left.to_owned()));
    (kept(&cut), cuts.iter().map(Cut::by).collect())
}

#[test]
fn joels_word_twenty_db_under_their_sound_after_their_line_is_cut() {
    // Their take's sound runs on past their last word, at 0 dB, and his
    // microphone hears it 20 dB down as a word of its own.
    let (written, cuts) = right_over(&their_sound(5_000, 7_000), 7_500, -20.0);
    assert_eq!(
        written,
        ["Fine by me."],
        "Joel's word 20 dB under their sound was written as his"
    );
    assert_eq!(
        cuts,
        ["20 dB under their sound"],
        "the cut didn't say it was for their sound, or by how far under it"
    );
}

#[test]
fn joels_word_ten_db_under_their_sound_is_his_own() {
    let (written, _) = right_over(&their_sound(5_000, 7_000), 7_500, -10.0);
    assert_eq!(
        written,
        ["Fine by me. Right."],
        "Joel's word only 10 dB under their sound was cut as their echo"
    );
}

#[test]
fn joels_word_under_their_sound_inside_their_line_is_his_own() {
    // Inside their line they and he may both be speaking, and his voice
    // over theirs is no echo, however far under their sound.
    let (written, _) = right_over(&their_sound(5_000, 7_000), 6_000, -25.0);
    assert_eq!(
        written,
        ["Fine by me. Right."],
        "Joel's word inside their line was cut for their sound"
    );
}

#[test]
fn with_none_of_their_sound_known_nothing_is_cut_by_it() {
    let (written, cuts) = right_over(&Track::default(), 7_500, -20.0);
    assert_eq!(
        (written, cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's word was cut by a sound of theirs nobody heard"
    );
}

#[test]
fn with_no_words_of_theirs_nothing_is_cut_by_their_sound() {
    // A cut is timed against a word of theirs, so with none there's
    // nothing to cut his word against.
    let mine = at_level(timed("Fine by me. Right.", 1_000), -20.0);
    let (lines, words) = lines_heard(&[], &[mine]);
    let sound = their_sound(0, 2_000);
    let (cut, cuts) = cut_echoes(&lines, &words, &sound, |left| Some(left.to_owned()));
    assert_eq!(
        (kept(&cut), cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's words were cut by their sound with no word of theirs to cut them against"
    );
}
