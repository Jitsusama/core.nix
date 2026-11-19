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
