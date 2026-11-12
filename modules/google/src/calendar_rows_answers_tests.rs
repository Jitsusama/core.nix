//! A meeting's answers as the shell made them (Calendar.qml's
//! `answering`): each said by its value, the one Joel gave noted as his
//! the moment he asks it, before Google takes it, and each saying in the
//! foot how he stands as Google last took it.

use serde_json::{Value, json};

use super::meetings;
use super::tests::{MONDAY, planned, words, zone};
use crate::calendar::Answer;

/// Each answer row's side and state, Going's, Maybe's and Not Going's.
fn answers(rows: &[Value]) -> Vec<(Value, Value)> {
    ["yes", "maybe", "no"]
        .iter()
        .map(|said| {
            let row = rows
                .iter()
                .find(|row| row["thing"] == format!("calendar:meeting:e1:{said}"))
                .expect("an invitation serves each answer");
            (row["side"].clone(), row["state"].clone())
        })
        .collect()
}

#[test]
fn each_answer_says_in_the_foot_how_joel_stands_as_google_took_it() {
    let rows = meetings(&[planned()], &[], MONDAY, &zone(), &words());
    assert_eq!(
        answers(&rows),
        [
            (json!([{"said": "your answer"}]), json!("going")),
            (json!([]), json!("going")),
            (json!([]), json!("going")),
        ],
        "Going is his answer, and each answer's foot says he's going"
    );
}

#[test]
fn an_answer_asked_is_noted_his_at_once_and_the_foot_waits_for_google() {
    let mut meeting = planned();
    meeting.given = Some(Answer::Maybe);
    let rows = meetings(&[meeting], &[], MONDAY, &zone(), &words());
    assert_eq!(
        answers(&rows),
        [
            (json!([]), json!("going")),
            (json!([{"said": "your answer"}]), json!("going")),
            (json!([]), json!("going")),
        ],
        "Maybe, asked, is his answer at once, while Google has him going"
    );
}
