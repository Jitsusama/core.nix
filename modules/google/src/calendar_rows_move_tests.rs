//! The later times Move the Meeting offers, served inside Move's row as
//! Calendar.js's `later` and `clashes` and Calendar.qml's `movingTo` made
//! them: each named for how far, its day and times below, the meeting it
//! would overlap at its side, and the one row asking Joel to be sure.

use serde_json::{Value, json};

use super::tests::{HOUR, MINUTE, MONDAY, one, planned, words, zone};
use super::{meetings, typed};
use crate::calendar::{Answer, Meeting};

fn within<'a>(rows: &'a [Value], parent: &str) -> Vec<&'a Value> {
    rows.iter().filter(|row| row["parent"] == parent).collect()
}

/// Planned, 15:00 to 15:30, among one at 15:40 it clashes with, and one
/// Joel turned down at 15:00 tomorrow and one all day tomorrow, neither of
/// which it clashes with.
fn among() -> Vec<Value> {
    let mut declined = one("e3", MONDAY + 25 * HOUR);
    declined.answer = Some(Answer::No);
    let mut all_day = one("e4", MONDAY + 10 * HOUR);
    all_day.end = MONDAY + 34 * HOUR;
    all_day.all_day = true;
    let clashing = one("e2", MONDAY + HOUR + 40 * MINUTE);
    meetings(
        &[planned(), declined, clashing, all_day],
        &[],
        MONDAY,
        &zone(),
        &words(),
    )
}

/// What typing `text` inside `within` answers, among `among`'s meetings.
fn typed_in(within: &str, text: &str) -> Option<Vec<Value>> {
    let clashing = one("e2", MONDAY + 20 * HOUR);
    let listed: [Meeting; 2] = [planned(), clashing];
    typed((&listed, &[]), MONDAY, &zone(), &words(), within, text)
}

#[test]
fn a_time_typed_in_move_is_made_as_a_later_time_is_and_named_by_when() {
    let rows = typed_in("row:calendar:e1:move", "tomorrow 10:00").expect("Move is served");
    let times = within(&rows, "row:calendar:e1:move");
    assert_eq!(times.len(), 1, "one time typed: {times:?}");
    assert_eq!(
        (
            times[0]["key"].clone(),
            times[0]["name"].clone(),
            times[0]["note"].clone(),
            times[0]["side"].clone(),
        ),
        (
            json!("row:calendar:e1:move:2026-10-27T14:00:00Z"),
            json!("Tuesday 27 October \u{b7} 10:00 to 10:30"),
            json!(""),
            json!([{"said": "clashes with Meeting e2"}]),
        ),
        "keyed as a later time, named by its day and times, saying what it clashes with"
    );
    let sure = within(&rows, "row:calendar:e1:move:2026-10-27T14:00:00Z");
    let start = MONDAY + 20 * HOUR;
    assert_eq!(
        (sure.len(), sure[0]["acts"].clone()),
        (
            1,
            json!([{"key": "verb:calendar:move",
                "ask": {"ask": "act", "act": {"move":
                    {"id": "e1", "start": start, "end": start + 30 * MINUTE}}}}])
        ),
        "one row asks Joel to be sure, and moves it: {sure:?}"
    );
    let again = typed_in("row:calendar:e1:move", "tue 10am").expect("Move is served");
    assert_eq!(
        within(&again, "row:calendar:e1:move")[0]["key"],
        times[0]["key"],
        "the same time typed another way is the same row, so none moves under the cursor"
    );
}

#[test]
fn only_a_served_move_answers_and_text_no_clock_reads_makes_no_time() {
    assert_eq!(
        typed_in("row:calendar:e1:move", "someday"),
        Some(Vec::new()),
        "no time to come is an answer with no rows"
    );
    for within in [
        "row:calendar:e1",
        "row:calendar:e1:guests",
        "row:calendar:nobody:move",
    ] {
        assert_eq!(
            typed_in(within, "tomorrow 10:00"),
            None,
            "{within} is no Move the daemon serves"
        );
    }
}

#[test]
fn move_offers_its_later_times_each_saying_when_and_what_it_would_clash_with() {
    let rows = among();
    let times = within(&rows, "row:calendar:e1:move");
    let said: Vec<(&str, &str, Value)> = times
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap(),
                row["note"].as_str().unwrap(),
                row["side"].clone(),
            )
        })
        .collect();
    let clashes = json!([{"said": "clashes with Meeting e2"}]);
    assert_eq!(
        said,
        [
            (
                "Fifteen Minutes Later",
                "Monday 26 October \u{b7} 15:15 to 15:45",
                clashes.clone()
            ),
            (
                "Half an Hour Later",
                "Monday 26 October \u{b7} 15:30 to 16:00",
                clashes.clone()
            ),
            (
                "An Hour Later",
                "Monday 26 October \u{b7} 16:00 to 16:30",
                clashes
            ),
            (
                "Tomorrow, Same Time",
                "Tuesday 27 October \u{b7} 15:00 to 15:30",
                json!([])
            ),
            (
                "Next Week, Same Time",
                "Monday 2 November \u{b7} 15:00 to 15:30",
                json!([])
            ),
        ],
        "neither one turned down nor one all day is a clash"
    );
    assert!(
        times.iter().all(|row| row["depth"] == 2
            && row["acts"] == json!([{"key": "verb:more", "step": "rows"}])
            && row["glyph"] == "moveMeeting"),
        "each steps in to its sure row: {times:?}"
    );
}

#[test]
fn a_later_time_moves_it_once_joel_is_sure_naming_who_google_tells() {
    let rows = among();
    let times = within(&rows, "row:calendar:e1:move");
    let start = MONDAY + HOUR + 15 * MINUTE;
    let key = times[0]["key"].as_str().unwrap();
    assert_eq!(
        key, "row:calendar:e1:move:2026-10-26T19:15:00Z",
        "keyed by when it would start, so it holds still as the meeting's served again"
    );
    let sure = within(&rows, key);
    assert_eq!(sure.len(), 1, "one row asks Joel to be sure: {sure:?}");
    assert_eq!(
        (
            sure[0]["name"].clone(),
            sure[0]["sure"].clone(),
            sure[0]["hue"].clone(),
            sure[0]["acts"].clone(),
        ),
        (
            json!(
                "Move It to Monday 26 October \u{b7} 15:15 to 15:45, \
                 Telling Ada Lovelace and Grace Hopper"
            ),
            json!(true),
            json!("alert"),
            json!([{"key": "verb:calendar:move",
                "ask": {"ask": "act", "act": {"move":
                    {"id": "e1", "start": start, "end": start + 30 * MINUTE}}}}]),
        ),
        "its Return asks the daemon to move it"
    );
}
