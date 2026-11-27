//! The cases of what the journal is told of Joel's words cut as the
//! others' echo and written back, each writing of the call giving the
//! runs it cuts.

use super::*;

const SECOND: usize = 16_000;

/// A run of Joel's words cut 50 ms after theirs, one heard each second
/// from `from` to `to` seconds into the call.
fn run(from: usize, to: usize) -> Run {
    Run {
        words: (from..=to).map(|at| at * SECOND).collect(),
        by: "50 ms after theirs".to_owned(),
    }
}

/// The journal's line for `words` of Joel's cut from `at`.
fn cut(words: usize, at: &str) -> String {
    format!(
        "call-transcripts: {words} of Joel's words at {at} cut as their echo, 50 ms after theirs"
    )
}

/// The journal's line for `words` of Joel's written back from `at`.
fn back(words: usize, at: &str) -> String {
    format!("call-transcripts: {words} of Joel's words at {at} written back")
}

#[test]
fn only_the_words_of_a_run_written_back_are_told_written_back() {
    let mut told = Told::default();
    assert_eq!(
        told.runs(&[run(45, 47)]),
        [cut(3, "0:45")],
        "the run wasn't told cut, so its words written back show nothing"
    );
    assert_eq!(
        told.runs(&[run(45, 45)]),
        [back(2, "0:46")],
        "two of the run's three words written back weren't told, or were told from its start"
    );
}

#[test]
fn a_run_grown_at_its_front_tells_only_the_words_newly_cut() {
    let mut told = Told::default();
    assert_eq!(
        told.runs(&[run(46, 47)]),
        [cut(2, "0:46")],
        "the run wasn't told cut, so its growing shows nothing"
    );
    assert_eq!(
        told.runs(&[run(44, 47)]),
        [cut(2, "0:44")],
        "a run grown at its front was told written back, or its words told were told again"
    );
}

#[test]
fn a_word_cut_written_back_and_cut_again_is_told_each_time() {
    let mut told = Told::default();
    assert_eq!(
        told.runs(&[run(46, 47)]),
        [cut(2, "0:46")],
        "the run wasn't told cut, so its last word written back shows nothing"
    );
    assert_eq!(
        told.runs(&[run(46, 46)]),
        [back(1, "0:47")],
        "the run's last word written back wasn't told"
    );
    assert_eq!(
        told.runs(&[run(46, 47)]),
        [cut(1, "0:47")],
        "the word written back and cut again wasn't told cut again"
    );
    assert_eq!(
        told.runs(&[]),
        [back(2, "0:46")],
        "the run written back whole wasn't told in one line, with the word cut again in it"
    );
}

#[test]
fn what_is_written_back_is_told_before_what_is_cut_in_one_writing() {
    let mut told = Told::default();
    assert_eq!(
        told.runs(&[run(40, 41), run(44, 45)]),
        [cut(2, "0:40"), cut(2, "0:44")],
        "the runs weren't told cut, so what's told of them next shows nothing"
    );
    assert_eq!(
        told.runs(&[run(45, 46)]),
        [back(2, "0:40"), back(1, "0:44"), cut(1, "0:46")],
        "what was written back wasn't told a line a run before what was newly cut"
    );
}
