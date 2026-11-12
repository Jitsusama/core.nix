//! What a meeting's rows inside lend another place, so one thing has one
//! row, made by its own place (ready-rows.md's lends): each guest is
//! People's person, by their address as People matches it, and a document
//! on it Documents' row, where those places hold them.

use serde_json::Value;

use super::meetings;
use super::tests::{MONDAY, planned, words, zone};
use crate::calendar::{Attachment, Guest};
use crate::drive::Kind;

fn of_kind<'a>(rows: &'a [Value], kind: &str) -> Vec<&'a Value> {
    rows.iter().filter(|row| row["kind"] == kind).collect()
}

#[test]
fn a_guest_lends_the_person_and_a_document_on_it_the_document() {
    let mut meeting = planned();
    meeting.guests.push(Guest {
        name: "Sofia Bodnar".into(),
        email: Some(" Sofia.Bodnar@Example.com ".into()),
        answer: crate::calendar::Answer::Yes,
    });
    meeting.attachments = vec![
        Attachment {
            title: "Quarterly Plan".into(),
            link: "https://docs.google.com/document/d/plan/edit".into(),
            id: Some("plan".into()),
            kind: Some(Kind::Document),
        },
        Attachment {
            title: "Agenda.pdf".into(),
            link: "https://drive.google.com/file/d/agenda/view".into(),
            id: None,
            kind: None,
        },
    ];
    let rows = meetings(&[meeting], &[], MONDAY, &zone(), &words());
    let lent =
        |rows: Vec<&Value>| -> Vec<Value> { rows.iter().map(|row| row["lends"].clone()).collect() };
    assert_eq!(
        lent(of_kind(&rows, "guest")),
        [
            "person:ada@example.com",
            "person:grace@example.com",
            "person:sofia.bodnar@example.com"
        ],
        "each guest lends the person by their address trimmed and lowercased"
    );
    let files = of_kind(&rows, "attachment");
    assert_eq!(
        (files[0]["thing"].clone(), lent(files.clone())),
        (
            Value::from("gdoc:plan"),
            vec![Value::from("gdoc:plan"), Value::Null]
        ),
        "a document on it is the document, and a file of no kind lends nothing"
    );
}
