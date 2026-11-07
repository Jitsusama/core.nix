//! What the feed hears from spend-watch on its stdin, as the contract writes
//! it: a pace, a refresh, and lines it passes over.

use claude_plan_feed::{Heard, Pace, heard};

#[test]
fn a_pace_is_heard() {
    assert_eq!(
        heard(r#"{"v": 1, "pace": "active"}"#),
        Some(Heard::Pace(Pace::Active))
    );
    assert_eq!(
        heard(r#"{"v": 1, "pace": "idle"}"#),
        Some(Heard::Pace(Pace::Idle))
    );
}

#[test]
fn a_pace_it_does_not_know_is_idle() {
    assert_eq!(
        heard(r#"{"v": 1, "pace": "asleep"}"#),
        Some(Heard::Pace(Pace::Idle))
    );
}

#[test]
fn a_refresh_is_heard() {
    assert_eq!(heard(r#"{"v": 1, "refresh": true}"#), Some(Heard::Refresh));
}

#[test]
fn what_the_watch_keeps_is_passed_over() {
    let kept = r#"{"v": 1, "kept": {"billed": {}, "caps": {"week": "2026-11-06T15:00:00Z"}}}"#;

    assert_eq!(heard(kept), None);
}

#[test]
fn a_line_of_another_version_or_none_is_passed_over() {
    assert_eq!(heard(r#"{"v": 2, "pace": "active"}"#), None);
    assert_eq!(heard(r#"{"pace": "active"}"#), None);
    assert_eq!(heard("pace: active"), None);
}
