//! The asks measures over the wish list's kinds: Joel's own asks are
//! measured apart from those the list set itself, and asks carried for
//! another owner count in neither.

use super::*;

const NOW: i64 = 1_794_700_000;

fn at(seconds: i64) -> String {
    Timestamp::from_second(seconds)
        .map(|t| t.to_string())
        .unwrap_or_default()
}

/// An ask of `tracking`, asked `ago` seconds before `NOW`, due an hour
/// before it and still open.
fn ask(words: &str, tracking: Option<&str>, ago: i64) -> String {
    serde_json::json!({
        "at": at(NOW - ago), "words": words, "owner": "Y2YBTS",
        "expected": at(NOW - 3600), "landed": null, "first": null,
        "tracking": tracking,
    })
    .to_string()
}

/// One of each kind, and one of his the list closed without landing.
fn list() -> String {
    let mut closed: serde_json::Value =
        serde_json::from_str(&ask("put down", None, 50 * 3600)).expect("an ask the tests made");
    closed["closed"] = "a standing rule".into();
    [
        ask("his own", None, 10 * 3600),
        ask("drawn from his", Some("derived"), 20 * 3600),
        ask("mend a test", Some("quarantine"), 30 * 3600),
        ask("someone else's", Some("owner-tracked"), 40 * 3600),
        closed.to_string(),
    ]
    .join("\n")
}

#[test]
fn his_own_asks_ship_and_slip_apart_from_the_lists() {
    let text = list();
    let slipped = slips(&text, NOW, Tracking::His);
    assert_eq!(slipped.key, "slips", "his keep their key");
    assert_eq!(slipped.behind, ["his own · Y2YBTS"]);
    let first = to_first_slice(&text, NOW, Tracking::His);
    assert_eq!(first.key, "ask-to-first-slice");
    assert_eq!(first.target, Some(4.0));
    assert!(
        (first.now - 10.0).abs() < 0.01,
        "his one open ask, 10 hours, got {}",
        first.now
    );
}

#[test]
fn an_ask_written_again_is_read_from_its_latest_line() {
    let mut remarked: serde_json::Value =
        serde_json::from_str(&ask("his own", None, 10 * 3600)).expect("an ask the tests made");
    remarked["tracking"] = "derived".into();
    let mut put_down: serde_json::Value =
        serde_json::from_str(&ask("twice", None, 12 * 3600)).expect("an ask the tests made");
    put_down["closed"] = "a standing rule".into();
    let text = [
        ask("his own", None, 10 * 3600),
        ask("twice", None, 12 * 3600),
        remarked.to_string(),
        put_down.to_string(),
    ]
    .join("\n");
    assert!(
        slips(&text, NOW, Tracking::His).behind.is_empty(),
        "re-marked and put down"
    );
    assert_eq!(
        slips(&text, NOW, Tracking::Derived).behind,
        ["his own · Y2YBTS"],
        "once, as its latest line marks it"
    );
    assert!(
        to_first_slice(&text, NOW, Tracking::His).behind.is_empty(),
        "neither is his now"
    );
}

/// An ask whose dates, each `(due, given)` in seconds before `NOW`, were
/// moved in place, as the list does, the last its `expected`.
fn moved(words: &str, dates: &[(i64, i64)]) -> String {
    let mut ask: serde_json::Value =
        serde_json::from_str(&ask(words, None, 30 * 3600)).expect("an ask the tests made");
    let promised: Vec<serde_json::Value> = dates
        .iter()
        .map(
            |(due, given)| serde_json::json!({"expected": at(NOW - due), "given": at(NOW - given)}),
        )
        .collect();
    ask["expected"] = promised.last().expect("a date")["expected"].clone();
    ask["promised"] = promised.into();
    ask.to_string()
}

#[test]
fn a_date_moved_in_place_after_it_passed_still_slipped() {
    let text = [
        // Due 3 h ago, moved at 1 h ago to tomorrow: a slip.
        moved("moved late", &[(3 * 3600, 20 * 3600), (-DAY, 3600)]),
        // Due 2 h ago, but moved 5 h ago, before it passed: none.
        moved("moved early", &[(2 * 3600, 20 * 3600), (-DAY, 5 * 3600)]),
        // Due 2 days ago and moved since: past the day read.
        moved("long ago", &[(2 * DAY, 3 * DAY), (-DAY, 3600)]),
        // Moved twice, each after it passed: two slips.
        moved(
            "twice",
            &[
                (10 * 3600, 20 * 3600),
                (6 * 3600, 9 * 3600),
                (-DAY, 5 * 3600),
            ],
        ),
    ]
    .join("\n");
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["twice · Y2YBTS", "twice · Y2YBTS", "moved late · Y2YBTS"],
        "a date moved after it passed is a slip, oldest first; one moved before isn't"
    );
}

/// An ask due 3 h ago and given 20 h ago, its first date marked `met`
/// `met` seconds before `NOW`, then moved to tomorrow an hour ago when
/// `then` is set, or left its last date when not.
fn met(words: &str, met: i64, then: bool) -> String {
    let dates: &[(i64, i64)] = if then {
        &[(3 * 3600, 20 * 3600), (-DAY, 3600)]
    } else {
        &[(3 * 3600, 20 * 3600)]
    };
    let mut ask: serde_json::Value =
        serde_json::from_str(&moved(words, dates)).expect("an ask the tests made");
    ask["promised"][0]["met"] = at(NOW - met).into();
    ask.to_string()
}

/// On 18 November Joel's 16:58 ask carried 21:00 for a step, its account,
/// which went to the orchestrator at 19:59; re-dated for the fix, the
/// 21:00 read as a slip.
#[test]
fn a_date_met_by_its_time_isnt_a_slip_once_the_ask_moves_on() {
    let text = [
        // Met an hour before it was due, then moved: none.
        met("met early", 4 * 3600, true),
        // Met at the very time it was due: none.
        met("met on time", 3 * 3600, true),
        // Met an hour after it was due: still a slip.
        met("met late", 2 * 3600, true),
        // Met before it was ever given: no step kept, a slip.
        met("met before given", 21 * 3600, true),
        // Its last date, met but not landed: landing alone keeps it.
        met("met, never moved", 4 * 3600, false),
    ]
    .join("\n");
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        [
            "met late · Y2YBTS",
            "met before given · Y2YBTS",
            "met, never moved · Y2YBTS",
        ],
        "only a date met by its time, after it was given, and moved on from, is kept"
    );
}

#[test]
fn a_met_dropped_by_a_later_line_is_a_slip_again() {
    let dropped: serde_json::Value =
        serde_json::from_str(&moved("dropped", &[(3 * 3600, 20 * 3600), (-DAY, 3600)]))
            .expect("an ask the tests made");
    let kept = met("kept", 4 * 3600, true);
    let text = [
        met("dropped", 4 * 3600, true),
        dropped.to_string(),
        moved("kept", &[(3 * 3600, 20 * 3600), (-DAY, 3600)]),
        kept,
    ]
    .join("\n");
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["dropped · Y2YBTS"],
        "each date is read from its ask's latest line, met or not"
    );
}

/// The keeper's own line for the 18 November ask, words shortened: its
/// 18:00 was moved at 18:28, a slip; its 21:00 was met at 19:59, none.
#[test]
fn the_keepers_met_line_reads_as_written() {
    let line = r#"{"at":"2026-11-18T16:58:00-05:00","words":"slack and chrome",
        "owner":"1DZYLJ","expected":"2026-11-21T12:00:00-05:00","landed":null,
        "first":null,"tracking":null,"promised":[
        {"expected":"2026-11-19T18:00:00-05:00","given":"2026-11-19T14:25:00-05:00"},
        {"expected":"2026-11-19T21:00:00-05:00","given":"2026-11-19T18:28:31-05:00",
        "met":"2026-11-19T19:59:00-05:00"},
        {"expected":"2026-11-21T12:00:00-05:00","given":"2026-11-19T21:14:45-05:00"}]}"#;
    let line: serde_json::Value = serde_json::from_str(line).expect("the keeper's line");
    let now = "2026-11-19T23:00:00-05:00"
        .parse::<Timestamp>()
        .expect("a time")
        .as_second();
    assert_eq!(
        slips(&line.to_string(), now, Tracking::His).behind,
        ["slack and chrome · 1DZYLJ"],
        "its 18:00 alone slipped"
    );
}

#[test]
fn a_date_promised_has_not_caught_up_with_still_counts() {
    // Its list moved `expected` to 2 h ago without adding it to `promised`,
    // and another's `promised` is empty: each passed date still counts.
    let mut lagging: serde_json::Value =
        serde_json::from_str(&moved("lagging", &[(-DAY, 20 * 3600)])).expect("an ask made");
    lagging["expected"] = at(NOW - 2 * 3600).into();
    let mut empty: serde_json::Value =
        serde_json::from_str(&moved("empty", &[(3600, 20 * 3600)])).expect("an ask made");
    empty["promised"] = serde_json::json!([]);
    let text = [lagging.to_string(), empty.to_string()].join("\n");
    assert_eq!(
        slips(&text, NOW, Tracking::His).behind,
        ["lagging · Y2YBTS", "empty · Y2YBTS"],
        "an ask's current date was dropped where its promised dates lag it"
    );
}

#[test]
fn the_lists_own_asks_and_mends_have_lines_of_their_own() {
    let text = list();
    let slipped = slips(&text, NOW, Tracking::Derived);
    assert_eq!(slipped.key, "slips-derived");
    assert_eq!(
        slipped.target,
        Some(0.0),
        "a date the list set is still a date"
    );
    assert_eq!(
        slipped.behind,
        ["drawn from his · Y2YBTS", "mend a test · Y2YBTS"]
    );
    let first = to_first_slice(&text, NOW, Tracking::Derived);
    assert_eq!(first.key, "ask-to-first-slice-derived");
    assert_eq!(first.target, None, "a mend takes days by design");
    assert!(
        (first.now - 25.0).abs() < 0.01,
        "the median of 20 and 30 hours, got {}",
        first.now
    );
}
