//! The slips measure: which dates slipped, and how a date's owner answers
//! one, so a date days past stays until they do.

use super::*;

const NOW: i64 = 1_794_700_000;

fn at(seconds: i64) -> String {
    Timestamp::from_second(seconds)
        .map(|t| t.to_string())
        .unwrap_or_default()
}

#[test]
fn a_date_passed_without_landing_is_a_slip_and_each_move_counts() {
    let ask = |expected: i64, landed: Option<i64>| {
        serde_json::json!({
            "words": "fix it", "owner": "Y2YBTS", "expected": at(expected),
            "landed": landed.map(at),
        })
        .to_string()
    };
    let text = [
        ask(NOW - 7200, None),            // moved: the old line
        ask(NOW - 600, None),             // moved again, also passed
        ask(NOW + 3600, None),            // the date it has now
        ask(NOW - 900, Some(NOW - 1000)), // landed in time
        ask(NOW - 900, Some(NOW - 100)),  // landed late
        ask(NOW - 2 * DAY, None),         // passed before the day, unanswered
    ]
    .join("\n");
    let m = slips(&text, NOW, Tracking::His);
    assert!(
        (m.now - 4.0).abs() < 0.5,
        "two moves, a late landing and an older date never answered, got {}",
        m.now
    );
    assert_eq!(m.owner, "Y2YBTS", "the oldest slip's owner");
}

#[test]
fn a_date_past_a_day_counts_until_its_owner_answers_it() {
    let old = NOW - 3 * DAY;
    let text = [
        serde_json::json!({"words": "unanswered", "owner": "A", "expected": at(old)}),
        serde_json::json!({"words": "landed late", "owner": "B", "expected": at(old),
            "landed": at(old + 60)}),
        serde_json::json!({"words": "moved", "owner": "C", "expected": at(NOW + DAY),
            "promised": [{"expected": at(old), "given": at(old - DAY)},
                {"expected": at(NOW + DAY), "given": at(old + 60)}]}),
        serde_json::json!({"words": "moved past a string", "owner": "F",
            "expected": at(NOW + DAY), "promised": [{"expected": at(old)}, "a",
                {"expected": at(NOW + DAY), "given": at(old + 60)}]}),
        serde_json::json!({"words": "moved bare", "owner": "D", "expected": at(old)}),
        serde_json::json!({"words": "moved bare", "owner": "D", "expected": at(NOW + DAY)}),
        serde_json::json!({"words": "put down", "owner": "E", "expected": at(old),
            "closed": at(old + 60)}),
    ]
    .iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>()
    .join("\n");
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["unanswered · A"],
        "a date days past was dropped unanswered, or one landed, moved or put down stayed, \
         or a bare string in its dates hid a move"
    );
}
