use std::fs;

use super::super::super::by_key;
use super::*;

/// An hour ending at 20:00 UTC on 26 November.
const NOW: i64 = 1_795_723_200;

/// A turn `ago` seconds before the hour's end, failed on `error` if given.
fn turn_at(ago: i64, error: Option<&str>) -> Turn {
    Turn {
        at_ms: (NOW - ago) * 1000,
        error: error.map(str::to_owned),
    }
}

/// An assistant's message as pi writes it to a session's log.
fn logged(at_ms: i64, error: Option<&str>) -> String {
    let stop = match error {
        Some(error) => serde_json::json!({"stopReason": "error", "errorMessage": error}),
        None => serde_json::json!({"stopReason": "stop"}),
    };
    let mut message = serde_json::json!({"role": "assistant", "timestamp": at_ms});
    message
        .as_object_mut()
        .expect("an object")
        .extend(stop.as_object().expect("an object").clone());
    serde_json::json!({"type": "message", "message": message}).to_string()
}

#[test]
fn each_failed_call_counts_and_weighs_against_every_call() {
    let mut turns: Vec<Turn> = (1..=48).map(|n| turn_at(n * 60, None)).collect();
    turns.push(turn_at(
        30,
        Some("Connection error.: other side closed (UND_ERR_SOCKET)"),
    ));
    turns.push(turn_at(90, Some("Request timed out.")));
    let measures = calls(Some(&turns), NOW);
    assert_eq!(by_key(&measures, FAILED).now, 2.0, "two failed");
    assert_eq!(by_key(&measures, SHARE).now, 4.0, "2 of 50 is 4 per cent");
    assert!(by_key(&measures, SHARE).past().is_some(), "4% is past 2%");
    assert_eq!(
        by_key(&measures, FAILED).behind,
        [
            "1 on Connection error.: other side closed (UND_ERR_SOCKET)",
            "1 on Request timed out.",
        ]
    );
}

#[test]
fn a_turn_outside_the_hour_counts_for_nothing() {
    let turns = [
        turn_at(3601, Some("Request timed out.")),
        turn_at(-5, Some("Request timed out.")),
        turn_at(10, None),
    ];
    let measures = calls(Some(&turns), NOW);
    assert_eq!(
        by_key(&measures, FAILED).now,
        0.0,
        "neither failure was in the hour"
    );
    assert_eq!(by_key(&measures, SHARE).now, 0.0, "one call, none failed");
}

#[test]
fn each_is_past_its_ceiling_only_above_it() {
    let failing = |failed: i64, of: i64| -> Vec<Turn> {
        (0..of)
            .map(|n| turn_at(1 + n % 3599, (n < failed).then_some("Request timed out.")))
            .collect()
    };
    let at_most = calls(Some(&failing(100, 5000)), NOW);
    assert_eq!(by_key(&at_most, FAILED).past(), None, "100 is the most");
    assert_eq!(by_key(&at_most, SHARE).past(), None, "2% is the most");
    // The share is said to a tenth, so 2.02% would read 2% and be in; 105
    // of 5000 is 2.1%, the first tenth past it.
    let past = calls(Some(&failing(105, 5000)), NOW);
    assert!(by_key(&past, FAILED).past().is_some(), "105 is past 100");
    assert!(by_key(&past, SHARE).past().is_some(), "2.1% is past 2%");
}

#[test]
fn an_hour_no_session_called_a_model_in_leaves_the_share_unjudged() {
    let measures = calls(Some(&[]), NOW);
    assert_eq!(by_key(&measures, SHARE).target, None, "nothing to weigh");
    assert_eq!(
        by_key(&measures, FAILED).target,
        Some(FAILED_AT_MOST),
        "no failure is judged in"
    );
}

#[test]
fn logs_unread_leave_both_unjudged() {
    let measures = calls(None, NOW);
    assert_eq!(measures.len(), 2, "both measures: {measures:?}");
    assert!(
        measures.iter().all(|m| m.target.is_none()),
        "unread is unjudged: {measures:?}"
    );
}

#[test]
fn a_cause_is_cut_short_before_it_reaches_the_card() {
    let long = "x".repeat(500);
    let measures = calls(Some(&[turn_at(10, Some(&long))]), NOW);
    let behind = &by_key(&measures, FAILED).behind;
    assert_eq!(
        behind[0].len(),
        "1 on ".len() + CAUSE_AT_MOST,
        "cut: {behind:?}"
    );
}

#[test]
fn a_log_is_read_back_only_to_the_hours_start() {
    let sessions = tempfile::tempdir().expect("a folder");
    let folder = sessions.path().join("--home-me--");
    fs::create_dir(&folder).expect("its folder");
    // A tool's output wider than a step, so a line runs across steps.
    let wide = format!(
        "{{\"type\":\"message\",\"message\":{{\"role\":\"toolResult\",\"text\":\"{}\"}}}}",
        "y".repeat(usize::try_from(STEP).expect("a step fits") * 2)
    );
    let lines = [
        logged((NOW - 7200) * 1000, Some("Request timed out.")),
        logged((NOW - 3000) * 1000, None),
        wide,
        logged((NOW - 60) * 1000, Some("Connection error.\nat line 2")),
    ];
    fs::write(folder.join("a.jsonl"), lines.join("\n") + "\n").expect("a log");
    fs::write(folder.join("notes.txt"), logged(NOW * 1000, None)).expect("not a log");
    let mut turns = read(sessions.path(), NOW).expect("read");
    turns.sort_by_key(|t| t.at_ms);
    assert_eq!(
        turns,
        [turn_at(3000, None), turn_at(60, Some("Connection error.")),],
        "only the hour's turns, each error its first line"
    );
}

#[test]
fn a_turn_wider_than_several_steps_is_read_whole() {
    let sessions = tempfile::tempdir().expect("a folder");
    let folder = sessions.path().join("--home-me--");
    fs::create_dir(&folder).expect("its folder");
    let step = usize::try_from(STEP).expect("a step fits");
    // Its error spans three steps and a part, and the narrow turns either
    // side of it sit across a step's edge, so every piece is joined.
    let wide = "z".repeat(step * 3 + step / 2);
    let lines = [
        logged((NOW - 3000) * 1000, None),
        logged((NOW - 2000) * 1000, Some(&wide)),
        logged((NOW - 60) * 1000, None),
    ];
    fs::write(folder.join("a.jsonl"), lines.join("\n") + "\n").expect("a log");
    let mut turns = read(sessions.path(), NOW).expect("read");
    turns.sort_by_key(|t| t.at_ms);
    assert_eq!(
        turns,
        [
            turn_at(3000, None),
            turn_at(2000, Some(&wide)),
            turn_at(60, None)
        ],
        "each turn once, the wide one whole"
    );
}

#[test]
fn a_log_is_read_back_no_further_than_the_hours_first_turn() {
    let sessions = tempfile::tempdir().expect("a folder");
    let folder = sessions.path().join("--home-me--");
    fs::create_dir(&folder).expect("its folder");
    // back_to keeps what it reads unfiltered, so a turn of an earlier hour
    // in its answer means it read on past the first one before the hour.
    let step = usize::try_from(STEP).expect("a step fits");
    let lines = [
        logged((NOW - 9000) * 1000, Some("Request timed out.")),
        logged((NOW - 7200) * 1000, Some("Request timed out.")),
        "x".repeat(step * 2),
        logged((NOW - 60) * 1000, None),
    ];
    fs::write(folder.join("a.jsonl"), lines.join("\n") + "\n").expect("a log");
    let path = folder.join("a.jsonl");
    let turns = back_to(&path, (NOW - 3600) * 1000).expect("read");
    assert_eq!(turns, [turn_at(60, None)], "only the hour's turn");
}

#[test]
fn a_missing_folder_is_unread() {
    assert!(
        read(Path::new("/nonexistent/sessions"), NOW).is_err(),
        "a folder that isn't there reads as unread"
    );
}
