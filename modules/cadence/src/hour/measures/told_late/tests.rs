//! The dates told late, read from asks lists in the shapes the list's
//! keeper writes.

use serde_json::{Value, json};

use super::super::slipped::{at, list};
use super::*;

const NOW: i64 = 1_794_700_000;

/// Joel's ask `words` by `owner`, first due at `first` and moved at
/// `moved` to an hour later, with `more` fields.
fn ask_moved(words: &str, owner: &str, (first, moved): (i64, i64), more: &Value) -> Value {
    let mut ask = json!({
        "at": at(NOW - DAY), "words": words, "owner": owner, "expected": at(first + 3600),
        "promised": [
            {"expected": at(first), "given": at(NOW - 2 * DAY)},
            {"expected": at(first + 3600), "given": at(moved)},
        ],
    });
    if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
        ask.extend(more.clone());
    }
    ask
}

/// Joel's ask `words`, due at `due` and never moved, with `more` fields.
fn ask_due(words: &str, due: i64, more: &Value) -> Value {
    let mut ask = json!({
        "at": at(NOW - DAY), "words": words, "owner": "OWNER1", "expected": at(due),
        "promised": [{"expected": at(due), "given": at(NOW - DAY)}],
    });
    if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
        ask.extend(more.clone());
    }
    ask
}

/// On 25 November a look guard's date was moved at 04:24 for 04:30, six
/// minutes before it, with no word to whoever waited on it.
#[test]
fn a_date_moved_six_minutes_before_it_is_told_late_and_named() {
    let text = list(&[ask_moved(
        "the look guard",
        "85PKB2",
        (NOW - 1000, NOW - 1360),
        &json!({}),
    )]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!((m.now - 1.0).abs() < 0.5, "one late move, got {}", m.now);
    assert_eq!(m.owner, "85PKB2", "the late move's owner");
    assert_eq!(
        m.behind,
        ["the look guard, moved 6 min before its date · 85PKB2"],
        "the line names the ask, how late it moved and its owner"
    );
    assert_eq!(m.key, KEY, "Joel's dates go by the bare key");
}

#[test]
fn a_date_moved_after_it_passed_is_told_late() {
    let text = list(&[ask_moved(
        "the cut",
        "A",
        (NOW - 2000, NOW - 1000),
        &json!({}),
    )]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["the cut, moved after its date passed · A"],
        "a move after the date read as in time"
    );
}

#[test]
fn a_date_moved_in_good_time_or_over_a_day_ago_is_not_told_late() {
    let text = list(&[
        ask_moved("in time", "A", (NOW - 1000, NOW - 4600), &json!({})),
        ask_moved(
            "long ago",
            "B",
            (NOW - 2 * DAY + 600, NOW - 2 * DAY + 300),
            &json!({}),
        ),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a move an hour ahead, or a late one older than the day, counted: {:?}",
        m.behind
    );
}

#[test]
fn an_open_date_a_quarter_hour_away_with_no_move_is_told_late() {
    let text = list(&[
        ask_due("near", NOW + 600, &json!({})),
        ask_due("further", NOW + 1200, &json!({})),
        ask_due("landed", NOW + 600, &json!({"landed": at(NOW - 60)})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["near, due in 10 min with no move · OWNER1"],
        "only the open date within a quarter hour counts"
    );
}

#[test]
fn a_parked_ask_or_one_landed_before_its_move_owes_no_word() {
    let text = list(&[
        ask_moved(
            "parked",
            "A",
            (NOW - 1000, NOW - 1360),
            &json!({"parked": "b95ea57b"}),
        ),
        ask_moved(
            "landed",
            "B",
            (NOW - 1000, NOW - 1360),
            &json!({"landed": at(NOW - 1400)}),
        ),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a parked ask, or a move after landing, counted: {:?}",
        m.behind
    );
}

#[test]
fn the_list_s_own_dates_count_apart_from_joel_s() {
    let derived = json!({"tracking": "derived"});
    let text = list(&[ask_moved("a mend", "A", (NOW - 1000, NOW - 1360), &derived)]);
    let his = told_late(&text, NOW, Tracking::His);
    let ours = told_late(&text, NOW, Tracking::Derived);
    assert!(his.behind.is_empty(), "a derived ask counted as Joel's");
    assert_eq!(ours.behind.len(), 1, "the derived move went uncounted");
    assert_eq!(ours.key, "dates-told-late-derived", "the list's own key");
}

#[test]
fn an_ask_written_again_counts_its_move_once() {
    let ask = ask_moved("again", "A", (NOW - 1000, NOW - 1360), &json!({}));
    let text = list(&[ask.clone(), ask]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(m.behind.len(), 1, "one move read twice: {:?}", m.behind);
}
