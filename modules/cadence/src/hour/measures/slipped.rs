//! The slips measure: which dates slipped, and how a date's owner answers
//! one, so a date days past stays until they do.

use serde_json::json;

use super::*;

const NOW: i64 = 1_794_700_000;

/// A moment, `seconds` since 1970, as the asks list writes one; the
/// measures beside this one read their asks' times with it too.
pub(super) fn at(seconds: i64) -> String {
    Timestamp::from_second(seconds)
        .map(|t| t.to_string())
        .unwrap_or_default()
}

/// The asks list `asks` would make, one line each; the measures beside
/// this one write their asks lists with it too.
pub(super) fn list(asks: &[Value]) -> String {
    asks.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_date_passed_without_landing_is_a_slip_and_each_move_counts() {
    let ask = |words: &str, expected: i64, landed: Option<i64>| {
        json!({
            "words": words, "owner": "Y2YBTS", "expected": at(expected),
            "landed": landed.map(at),
        })
        .to_string()
    };
    let text = [
        ask("fix it", NOW - 7200, None),            // moved: the old line
        ask("fix it", NOW - 600, None),             // moved again, also passed
        ask("fix it", NOW + 3600, None),            // the date it has now
        ask("fix it", NOW - 900, Some(NOW - 1000)), // landed in time
        ask("fix it", NOW - 900, Some(NOW - 100)),  // landed late
        ask("fix that", NOW - 2 * DAY, None),       // another, two days past
    ]
    .join("\n");
    let m = slips(&text, NOW, Tracking::His);
    assert!(
        (m.now - 4.0).abs() < 0.5,
        "two moves, a late landing and an older date never answered, got {}",
        m.now
    );
    assert_eq!(m.owner, "Y2YBTS", "the oldest slip's owner");
    assert_eq!(
        m.behind.first().map(String::as_str),
        Some("fix that, due 2 days ago · Y2YBTS"),
        "the oldest slip's line didn't say how long ago it was due"
    );
}

#[test]
fn an_old_slips_line_says_how_long_ago_it_was_due_in_whole_days() {
    let hour = 3600;
    let text = list(&[
        json!({"words": "two days and an hour", "owner": "A",
            "expected": at(NOW - 2 * DAY - hour)}),
        json!({"words": "an hour short of two days", "owner": "B",
            "expected": at(NOW - 2 * DAY + hour)}),
        json!({"words": "a day and an hour", "owner": "C",
            "expected": at(NOW - DAY - hour)}),
        json!({"words": "an hour", "owner": "D", "expected": at(NOW - hour)}),
    ]);
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        [
            "two days and an hour, due 2 days ago · A",
            "an hour short of two days, due a day ago · B",
            "a day and an hour, due a day ago · C",
            "an hour · D"
        ],
        "a part day didn't round down to whole days, or a slip within the day said an age"
    );
}

#[test]
fn a_date_past_a_day_counts_until_its_owner_answers_it() {
    let old = NOW - 3 * DAY;
    let text = list(&[
        json!({"words": "unanswered", "owner": "A", "expected": at(old)}),
        json!({"words": "landed late", "owner": "B", "expected": at(old),
            "landed": at(old + 60)}),
        json!({"words": "landed in time", "owner": "B", "expected": at(old),
            "landed": at(old - 60)}),
        json!({"words": "moved", "owner": "C", "expected": at(NOW + DAY),
            "promised": [{"expected": at(old), "given": at(old - DAY)},
                {"expected": at(NOW + DAY), "given": at(old + 60)}]}),
        json!({"words": "moved past a string", "owner": "F",
            "expected": at(NOW + DAY), "promised": [{"expected": at(old)}, "a",
                {"expected": at(NOW + DAY), "given": at(old + 60)}]}),
        json!({"words": "moved past no date", "owner": "F",
            "expected": at(NOW + DAY), "promised": [{"expected": at(old)}, {"note": "a"},
                {"expected": at(NOW + DAY), "given": at(old + 60)}]}),
        json!({"words": "moved by expected", "owner": "G", "expected": at(NOW + DAY),
            "promised": [{"expected": at(old), "given": at(old - DAY)}]}),
        json!({"words": "moved bare", "owner": "D", "expected": at(old)}),
        json!({"words": "moved bare", "owner": "D", "expected": at(NOW + DAY)}),
        json!({"words": "put down", "owner": "E", "expected": at(old),
            "closed": at(old + 60)}),
    ]);
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["unanswered, due 3 days ago · A"],
        "a date days past was dropped unanswered, or one kept, landed, moved or put down \
         stayed"
    );
}

#[test]
fn an_old_date_isnt_answered_by_what_says_no_date_or_has_yet_to_happen() {
    let old = NOW - 3 * DAY;
    let text = list(&[
        json!({"words": "rewritten undated", "owner": "A", "expected": at(old)}),
        json!({"words": "rewritten undated", "owner": "A", "expected": "soon"}),
        json!({"words": "landing ahead", "owner": "B", "expected": at(old),
            "landed": at(NOW + 60)}),
    ]);
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        [
            "rewritten undated, due 3 days ago · A",
            "landing ahead, due 3 days ago · B"
        ],
        "a rewrite with no date it could read, or a landing after now, answered a slip"
    );
}

#[test]
fn the_day_boundary_and_each_asks_own_moves_decide_an_old_date() {
    let text = list(&[
        // Just inside the day and landed late: a slip as before.
        json!({"words": "inside", "owner": "A", "expected": at(NOW - DAY + 60),
            "landed": at(NOW - 60)}),
        // At the day's edge and unanswered: a slip a day old.
        json!({"words": "edge", "owner": "E", "expected": at(NOW - DAY)}),
        // Just past the day and landed late: answered.
        json!({"words": "past", "owner": "B", "expected": at(NOW - DAY),
            "landed": at(NOW - 60)}),
        // Two asks in the same words, told apart by when each was asked: a
        // rewrite of one moves only its own date.
        json!({"at": at(NOW - 9 * DAY), "words": "twin", "owner": "C",
            "expected": at(NOW - 2 * DAY)}),
        json!({"at": at(NOW - 8 * DAY), "words": "twin", "owner": "D",
            "expected": at(NOW - 2 * DAY)}),
        json!({"at": at(NOW - 8 * DAY), "words": "twin", "owner": "D",
            "expected": at(NOW + DAY)}),
    ]);
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        [
            "twin, due 2 days ago · C",
            "edge, due a day ago · E",
            "inside · A"
        ],
        "the day's edge moved, one ask's rewrite answered another's date, or an old \
         slip's line didn't say how long ago it was due"
    );
}

/// The keeper folds a copy of an ask into it, marking the copy's id with
/// `~`; written after the ask, the copy, with no date of its own, would
/// read as its latest line and answer the date it slipped.
#[test]
fn a_twin_folded_into_an_ask_is_not_read_as_its_latest_line() {
    let asked = at(NOW - 3 * DAY);
    let text = list(&[
        json!({"id": "0a1b2c3d", "at": asked, "words": "fold", "owner": "F",
            "expected": at(NOW - 3600)}),
        json!({"id": "0a1b2c3d~1", "at": asked, "words": "fold", "owner": "F",
            "closed": at(NOW - 1800), "parked": "a ruling"}),
    ]);
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["fold · F"],
        "a twin written after its ask stood for it and answered or parked its slip"
    );
}
