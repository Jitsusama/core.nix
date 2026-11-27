//! The cases of cutting Joel's words far under the others' sound, held
//! to calls made up word by word over a track of their sound. It builds
//! its calls with the helpers of echo.rs's tests, its parent.

use super::*;

/// A track of their sound at 0 dB of full scale, by its power, from
/// `from_ms` to `to_ms` milliseconds into the call, as a take of theirs.
fn their_sound(from_ms: usize, to_ms: usize) -> Track {
    their_sound_at(from_ms, to_ms, 0.0)
}

/// A track of their sound at `level` dB of full scale, by its power,
/// from `from_ms` to `to_ms` milliseconds into the call.
fn their_sound_at(from_ms: usize, to_ms: usize, level: f32) -> Track {
    let (from, to) = (from_ms * RATE / 1000, to_ms * RATE / 1000);
    let mut track = Track::default();
    track.hear(from, &vec![10f32.powf(level / 20.0); to - from]);
    track
}

/// What's written of Joel's line, and what each cut from it was cut by,
/// over their `sound`, when he says "Fine by me." at -20 dB a second into
/// the call and then each of `said`, its text at its millisecond at its
/// dB, and they say "Lunch at noon on Thursday then." at -20 dB from 5 s,
/// their line spanning their take, 5 s to 7 s, as the daemon's and the
/// scorer's do.
fn said_over(sound: &Track, said: &[(&str, usize, f32)]) -> (Vec<String>, Vec<String>) {
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = at_level(timed("Fine by me.", 1_000), -20.0);
    for &(text, at_ms, level) in said {
        mine.extend(at_level(timed(text, at_ms), level));
    }
    let (mut lines, words) = lines_heard(&[theirs], &[mine]);
    lines[0].end = 7_000 * RATE / 1000;
    let (cut, cuts) = cut_echoes(&lines, &words, sound, |left| Some(left.to_owned()));
    (kept(&cut), cuts.iter().map(Cut::by).collect())
}

/// `said_over` their `sound`, Joel saying "Right." at `level` dB `at_ms`
/// milliseconds into the call.
fn right_over(sound: &Track, at_ms: usize, level: f32) -> (Vec<String>, Vec<String>) {
    said_over(sound, &[("Right.", at_ms, level)])
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
fn joels_word_nineteen_db_under_their_sound_is_his_own() {
    let (written, cuts) = right_over(&their_sound(5_000, 7_000), 7_500, -19.0);
    assert_eq!(
        (written, cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's word 19 dB under their sound, short of the 20 dB it takes, was cut"
    );
}

#[test]
fn joels_words_in_a_row_under_their_sound_by_different_gaps_are_one_cut() {
    // "Right." is 20 dB under their sound and "Okay.", with only 200 ms
    // of it in the quarter second before, about 29 dB: one rule cuts
    // both, so they're told as one cut, by the first's gap.
    let said = [("Right.", 7_500, -20.0), ("Okay.", 7_800, -30.0)];
    let (written, cuts) = said_over(&their_sound(5_000, 7_000), &said);
    assert_eq!(
        (written, cuts),
        (
            vec!["Fine by me.".to_owned()],
            vec!["20 dB under their sound".to_owned()]
        ),
        "two words in a row cut for their sound weren't told as one cut"
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
    // Inside their line, beside a word of theirs, they and he may both be
    // speaking, and his voice over theirs is no echo, however far under
    // their sound.
    let (written, _) = right_over(&their_sound(5_000, 7_000), 6_000, -25.0);
    assert_eq!(
        written,
        ["Fine by me. Right."],
        "Joel's word inside their line was cut for their sound"
    );
}

/// What's written of Joel's line, and what each cut from it was cut by,
/// when he says "Fine by me." at -20 dB a second into the call and
/// "Right." at `level` dB `at_ms` in, inside a line of theirs at 0 dB
/// from 5 s to 9.5 s whose model heard "Lunch at noon" from 5 s and "on
/// Thursday then." from 8.6 s, at -20 dB, and nothing between: from
/// 5.6 s to 8.6 s no word of theirs is heard.
fn right_inside(at_ms: usize, level: f32) -> (Vec<String>, Vec<String>) {
    let mut theirs = at_level(timed("Lunch at noon", 5_000), -20.0);
    theirs.extend(at_level(timed("on Thursday then.", 8_600), -20.0));
    let mut mine = at_level(timed("Fine by me.", 1_000), -20.0);
    mine.extend(at_level(timed("Right.", at_ms), level));
    let (mut lines, words) = lines_heard(&[theirs], &[mine]);
    lines[0].end = 9_500 * RATE / 1000;
    let sound = their_sound(5_000, 9_500);
    let (cut, cuts) = cut_echoes(&lines, &words, &sound, |left| Some(left.to_owned()));
    (kept(&cut), cuts.iter().map(Cut::by).collect())
}

#[test]
fn joels_word_under_their_sound_far_from_their_words_in_their_line_is_cut() {
    // Their take's sound goes on between the words their model heard,
    // and his microphone hears it 22 dB down, 1.5 s from either.
    let (written, cuts) = right_inside(7_100, -22.0);
    assert_eq!(
        (written, cuts),
        (
            vec!["Fine by me.".to_owned()],
            vec!["22 dB under their sound".to_owned()]
        ),
        "Joel's word inside their line, 22 dB under their sound and 1.5 s from \
         their words, wasn't cut for their sound"
    );
}

#[test]
fn joels_word_under_their_sound_half_a_second_after_their_word_is_his_own() {
    let (written, cuts) = right_inside(6_100, -22.0);
    assert_eq!(
        (written, cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's word inside their line 0.5 s after their word was cut for their sound"
    );
}

#[test]
fn joels_word_under_their_sound_half_a_second_before_their_word_is_his_own() {
    let (written, cuts) = right_inside(8_100, -22.0);
    assert_eq!(
        (written, cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's word inside their line 0.5 s before their word was cut for their sound"
    );
}

#[test]
fn joels_word_nineteen_db_under_their_sound_in_their_line_is_his_own() {
    let (written, cuts) = right_inside(7_100, -19.0);
    assert_eq!(
        (written, cuts),
        (vec!["Fine by me. Right.".to_owned()], Vec::new()),
        "Joel's word inside their line 19 dB under their sound, short of 20 dB, was cut"
    );
}

#[test]
fn joels_word_at_his_own_voice_far_from_their_words_in_their_line_is_cut() {
    // Seven words of his alone at -40 dB, a quiet laptop microphone, make
    // his voice -40 dB. His word between their words is at his voice,
    // and 25 dB under their take at -15 dB: inside their line his voice
    // isn't asked, since there the echo is no quieter than his voice
    // is, so his word is cut. That's the risk this rule takes.
    let mut theirs = at_level(timed("Lunch at noon", 5_000), -15.0);
    theirs.extend(at_level(timed("on Thursday then.", 8_600), -15.0));
    let mut mine = at_level(timed("Fine by me, I can do that.", 1_000), -40.0);
    mine.extend(at_level(timed("Right.", 7_100), -40.0));
    let (mut lines, words) = lines_heard(&[theirs], &[mine]);
    lines[0].end = 9_500 * RATE / 1000;
    let sound = their_sound_at(5_000, 9_500, -15.0);
    let (cut, cuts) = cut_echoes(&lines, &words, &sound, |left| Some(left.to_owned()));
    assert_eq!(
        (kept(&cut), cuts.iter().map(Cut::by).collect::<Vec<_>>()),
        (
            vec!["Fine by me, I can do that.".to_owned()],
            vec!["25 dB under their sound".to_owned()]
        ),
        "Joel's word inside their line, 25 dB under their sound and far from their \
         words, was kept for being at his own voice"
    );
}

#[test]
fn joels_answer_at_his_own_voice_after_their_take_is_his_own() {
    // Seven words of his alone at -40 dB, a quiet laptop microphone, make
    // his voice -40 dB. Their take at -15 dB is 25 dB over his answer
    // just after it, but his answer is at his voice, not 12 dB under it.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -15.0);
    let mut mine = at_level(timed("Fine by me, I can do that.", 1_000), -40.0);
    mine.extend(at_level(timed("Right.", 7_500), -40.0));
    let (mut lines, words) = lines_heard(&[theirs], &[mine]);
    // Their line spans their whole take, as the daemon's does.
    lines[0].end = 7_000 * RATE / 1000;
    let sound = their_sound_at(5_000, 7_000, -15.0);
    let (cut, cuts) = cut_echoes(&lines, &words, &sound, |left| Some(left.to_owned()));
    assert_eq!(
        (kept(&cut), cuts.iter().map(Cut::by).collect::<Vec<_>>()),
        (
            vec!["Fine by me, I can do that. Right.".to_owned()],
            Vec::new()
        ),
        "Joel's answer at his own voice was cut for their sound just before it"
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
