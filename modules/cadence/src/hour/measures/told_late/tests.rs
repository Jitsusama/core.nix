//! The dates told late, read from asks lists in the shapes the list's
//! keeper writes.

use serde_json::{Value, json};

use super::super::slipped::{at, list};
use super::*;

const NOW: i64 = 1_794_700_000;

/// Joel's ask `words` by `owner`, first due at `first` and moved at
/// `moved` to an hour later, with `more` fields.
fn ask_moved(words: &str, owner: &str, (first, moved): (i64, i64), more: &Value) -> Value {
    ask_moved_to(words, owner, (first, moved, first + 3600), more)
}

/// Joel's ask `words` by `owner`, first due at `first` and moved at
/// `moved` to `due`, with `more` fields.
fn ask_moved_to(
    words: &str,
    owner: &str,
    (first, moved, due): (i64, i64, i64),
    more: &Value,
) -> Value {
    let mut ask = json!({
        "at": at(NOW - DAY), "words": words, "owner": owner, "expected": at(due),
        "promised": [
            {"expected": at(first), "given": at(NOW - 2 * DAY)},
            {"expected": at(due), "given": at(moved)},
        ],
    });
    if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
        ask.extend(more.clone());
    }
    ask
}

/// Joel's ask `words`, due at `due` and never moved.
fn ask_due(words: &str, due: i64) -> Value {
    json!({
        "at": at(NOW - DAY), "words": words, "owner": "OWNER1", "expected": at(due),
        "promised": [{"expected": at(due), "given": at(NOW - DAY)}],
    })
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
fn a_move_in_its_last_minute_says_so() {
    let text = list(&[ask_moved(
        "the cut",
        "A",
        (NOW - 1000, NOW - 1030),
        &json!({}),
    )]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["the cut, moved in its last minute · A"],
        "a move 30 s before its date read otherwise"
    );
}

/// A date that passes before it moves is a slip, so the slips measure
/// counts it and this one doesn't count it again.
#[test]
fn a_date_moved_at_or_after_it_passed_is_left_to_the_slips() {
    let text = list(&[
        ask_moved("at its date", "A", (NOW - 2000, NOW - 2000), &json!({})),
        ask_moved("after it", "B", (NOW - 2000, NOW - 1000), &json!({})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a move once the date had come counted beside its slip: {:?}",
        m.behind
    );
}

/// A date given again, or brought forward, keeps nobody waiting longer.
#[test]
fn a_date_given_again_or_brought_forward_is_no_late_move() {
    let text = list(&[
        ask_moved_to(
            "again",
            "A",
            (NOW - 1000, NOW - 1360, NOW - 1000),
            &json!({}),
        ),
        ask_moved_to(
            "sooner",
            "B",
            (NOW - 1000, NOW - 1360, NOW - 1200),
            &json!({}),
        ),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a date kept or pulled in counted as moved late: {:?}",
        m.behind
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

/// The day is read to the second: a move a day old has left it, one a
/// second younger hasn't, and one not yet made isn't read.
#[test]
fn a_late_move_counts_for_a_day_and_none_yet_to_come() {
    let text = list(&[
        ask_moved("a day old", "A", (NOW - DAY + 300, NOW - DAY), &json!({})),
        ask_moved(
            "in the day",
            "B",
            (NOW - DAY + 301, NOW - DAY + 1),
            &json!({}),
        ),
        ask_moved("to come", "C", (NOW + 300, NOW + 1), &json!({})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["in the day, moved 5 min before its date · B"],
        "the day's edge, or a move still to come, read wrong"
    );
}

#[test]
fn a_move_with_a_quarter_hour_left_is_late_and_a_second_more_is_not() {
    let text = list(&[
        ask_moved("at the edge", "A", (NOW - 1000, NOW - 1900), &json!({})),
        ask_moved("just in time", "B", (NOW - 1000, NOW - 1901), &json!({})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["at the edge, moved 15 min before its date · A"],
        "a move with 15 minutes left is late, one with a second more isn't"
    );
}

/// A date that passes with no move is the slips measure's to count.
#[test]
fn a_near_or_passed_date_with_no_move_is_left_to_the_slips() {
    let text = list(&[ask_due("near", NOW + 600), ask_due("passed", NOW - 600)]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a date never moved counted as told late: {:?}",
        m.behind
    );
}

#[test]
fn a_parked_or_put_down_ask_or_one_kept_before_its_move_owes_no_word() {
    let mut met = ask_moved("met", "C", (NOW - 1000, NOW - 1360), &json!({}));
    met["promised"][0]["met"] = json!(at(NOW - 1400));
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
        met,
        ask_moved(
            "put down",
            "D",
            (NOW - 1000, NOW - 1360),
            &json!({"closed": "put down", "landed": null}),
        ),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert!(
        m.behind.is_empty(),
        "a parked or put-down ask, or a move after landing or meeting, counted: {:?}",
        m.behind
    );
}

/// Parking or putting an ask down after its late move doesn't undo the
/// move, as landing after it doesn't: the word still came too late.
#[test]
fn an_ask_parked_or_put_down_after_its_late_move_keeps_it() {
    let moved = (NOW - 1000, NOW - 1360);
    let text = list(&[
        ask_moved("parked after", "A", moved, &json!({})),
        ask_moved("parked after", "A", moved, &json!({"parked": "b95ea57b"})),
        ask_moved("put down after", "B", moved, &json!({})),
        ask_moved(
            "put down after",
            "B",
            moved,
            &json!({"closed": "put down", "landed": null}),
        ),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        [
            "parked after, moved 6 min before its date · A",
            "put down after, moved 6 min before its date · B",
        ],
        "a move told late was wiped by what came after it"
    );
}

/// Landing or meeting a date after its late move doesn't undo the move.
#[test]
fn a_landing_or_met_after_the_move_leaves_it_late() {
    let mut met = ask_moved("met after", "B", (NOW - 1000, NOW - 1360), &json!({}));
    met["promised"][0]["met"] = json!(at(NOW - 1300));
    let text = list(&[
        ask_moved(
            "landed after",
            "A",
            (NOW - 1000, NOW - 1360),
            &json!({"landed": at(NOW - 1300)}),
        ),
        met,
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind.len(),
        2,
        "a landing or met after a late move undid it: {:?}",
        m.behind
    );
}

#[test]
fn the_earliest_late_move_owns_it_and_leads_the_lines() {
    let text = list(&[
        ask_moved("later", "B", (NOW - 200, NOW - 500), &json!({})),
        ask_moved("earlier", "A", (NOW - 1000, NOW - 1360), &json!({})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(m.owner, "A", "the earliest move's owner owns the measure");
    assert_eq!(
        m.behind,
        [
            "earlier, moved 6 min before its date · A",
            "later, moved 5 min before its date · B",
        ],
        "the lines aren't in the order the moves were made"
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

/// The keeper writes an ask whole each time, so its latest line holds the
/// move its first line hadn't yet.
#[test]
fn an_ask_is_read_from_its_latest_line() {
    let text = list(&[
        ask_due("again", NOW - 1000),
        ask_moved("again", "A", (NOW - 1000, NOW - 1360), &json!({})),
    ]);
    let m = told_late(&text, NOW, Tracking::His);
    assert_eq!(
        m.behind,
        ["again, moved 6 min before its date · A"],
        "the ask was read from its first line, not its latest"
    );
}
