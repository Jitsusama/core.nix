//! The tests of the spend changing: a moved price noted and taken up, and
//! something new weighed by what it spent, at its moment.

use super::*;
use crate::detect::tests::{NOW, config, turn};

// A model billed at a new share of its list price for three settled
// days is a note, and the estimates take the new price up at once.
#[test]
fn a_moved_price_is_noted_and_taken_up() {
    let history = History::memory();
    let today = crate::time::hour_of(NOW) / 24;
    for back in 1..=23 {
        let day = today - back;
        let hour = day * 24 + 12;
        turn(&history, "s", crate::time::hour_start(hour), 100.0);
        let ratio = if back <= 3 { 0.55 } else { 0.72 };
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "warehouse", "tool": "pi",
            "hour": crate::time::rfc3339(crate::time::hour_start(hour)), "model": "m",
            "dollars": 100.0 * ratio,
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, NOW).unwrap();
    }
    let settled = serde_json::json!({
        "v": 1, "kind": "settled", "source": "warehouse",
        "through": crate::time::rfc3339(crate::time::hour_start(today * 24)),
    });
    let record = crate::feed::parse(&settled.to_string()).unwrap();
    crate::feed::store(&history, "proxy", &record, NOW).unwrap();
    let picture = Picture::read(&history, &config(), NOW).unwrap();
    let found = prices(&picture);
    assert_eq!(found.len(), 1, "{:?}", picture.shifts);
    assert_eq!(
        (found[0].key.as_str(), found[0].badge.as_str()),
        ("price:m", "55%")
    );
    // A title opens on its own words, since what begins it is written
    // with a capital and a model's name is written as it's named.
    assert_eq!(
        (found[0].title.as_str(), found[0].note.as_str()),
        ("a new price for m", "of list, from 72% before")
    );
    assert_eq!(picture.ratios.of("m"), Some(0.55));
}

fn billed(history: &History, hour: &str, model: &str, dollars: f64) {
    let line = serde_json::json!({
        "v": 1, "kind": "billed", "source": "warehouse", "hour": hour, "tool": "pi",
        "model": model, "vendor": "v", "dollars": dollars,
        "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(history, "proxy", &record, NOW).unwrap();
}

// Something new is a warning past the floor; below it, it's too
// little to mention. One billed the month before is nothing new.
#[test]
fn something_new_is_weighed_by_what_it_spent() {
    let history = History::memory();
    billed(&history, "2026-10-04T12:00:00Z", "old", 50.0);
    billed(&history, "2026-10-14T12:00:00Z", "old", 50.0);
    billed(&history, "2026-10-14T12:00:00Z", "big", 30.0);
    billed(&history, "2026-10-14T13:00:00Z", "small", 3.0);
    let config = config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = novelty(&picture, &history, &config).unwrap();
    let severity = |key: &str| found.iter().find(|f| f.key == key).map(|f| f.severity);
    assert_eq!(severity("new:model:big"), Some(Severity::Note));
    assert_eq!(severity("new:model:small"), None);
    assert_eq!(severity("new:model:old"), None);
}

// A new model or tool is one someone chose, so it's listed; a new
// credential spending under the account's name may be a leaked key, so
// it's told.
#[test]
fn a_new_credential_is_told_and_a_new_model_listed() {
    let history = History::memory();
    let by = |hour: &str, model: &str, token: &str, dollars: f64| {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "warehouse", "hour": hour, "tool": "pi",
            "model": model, "vendor": "v", "token": token, "dollars": dollars,
            "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, NOW).unwrap();
    };
    by("2026-10-04T12:00:00Z", "old", "mine", 50.0);
    by("2026-10-14T12:00:00Z", "fresh", "mine", 30.0);
    by("2026-10-14T12:00:00Z", "old", "stranger", 30.0);
    let config = config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = novelty(&picture, &history, &config).unwrap();
    let severity = |prefix: &str| {
        found
            .iter()
            .find(|f| f.key.starts_with(prefix))
            .map(|f| f.severity)
    };
    assert_eq!(severity("new:model:fresh"), Some(Severity::Note));
    assert_eq!(severity("new:token:"), Some(Severity::Warning));
}

// What's new is what the last day billed, up to the moment judged: a
// replay of a past day mustn't count the months after it, as it once
// counted $42,642 of one model as a single day's.
#[test]
fn something_new_is_judged_at_its_moment() {
    let history = History::memory();
    billed(&history, "2026-10-04T12:00:00Z", "old", 50.0);
    billed(&history, "2026-10-14T12:00:00Z", "big", 30.0);
    billed(&history, "2026-10-20T12:00:00Z", "later", 900.0);
    let config = config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = novelty(&picture, &history, &config).unwrap();
    let keys: Vec<&str> = found.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, ["new:model:big"]);
}
