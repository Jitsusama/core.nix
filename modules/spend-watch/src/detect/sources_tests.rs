//! The tests of what the sources say: a meter's words told once, and a
//! source unreadable for a moment told nothing.

use super::*;
use crate::detect::tests::{NOW, config};
use crate::history::History;

// A meter saying the same thing with a newer total each time is one
// finding, the newest, and a thing of another shape stands beside it.
#[test]
fn a_thing_said_again_with_new_figures_is_said_once() {
    let history = History::memory();
    let said = |text: &str, ago: f64| {
        let line =
            serde_json::json!({"v": 1, "kind": "said", "severity": "warning", "text": text});
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, NOW - ago).unwrap();
    };
    said(
        "7d AI spend has exceeded the $4,000.00 budget ($26,601.81 of $4,000.00)",
        600.0,
    );
    said(
        "7d AI spend has exceeded the $4,000.00 budget ($26,685.44 of $4,000.00)",
        60.0,
    );
    said(
        "7d AI spend has exceeded the $4,000.00 budget ($26,635.59 of $4,000.00)",
        300.0,
    );
    said("The token was rotated", 120.0);
    let picture = Picture::read(&history, &config(), NOW).unwrap();
    let found = said_findings(&picture);
    assert_eq!(
        found,
        [
            "the provider's meter says: The token was rotated",
            "the provider's meter says: 7d AI spend has exceeded the $4,000.00 budget \
             ($26,685.44 of $4,000.00)",
        ]
    );
}

fn said_findings(picture: &Picture) -> Vec<String> {
    said(picture).into_iter().map(|found| found.title).collect()
}

fn health_at(history: &History, source: &str, state: &str, message: &str, at: f64) {
    let line = serde_json::json!({
        "v": 1, "kind": "health", "source": source, "state": state, "message": message,
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(history, "proxy", &record, at).unwrap();
}

// A machine booting before its network finds every source unreadable
// for a moment, which is nothing to say. One unreadable a quarter of
// an hour is a note, saying since when, and never a notice.
#[test]
fn a_source_down_a_moment_is_nothing() {
    let history = History::memory();
    let failed = "the meter couldn't be reached: io: failed to lookup address information";
    health_at(&history, "meter", "error", failed, NOW - 60.0);
    health_at(&history, "meter", "error", failed, NOW);
    let config = config();
    assert_eq!(
        health(&Picture::read(&history, &config, NOW).unwrap()),
        [] as [crate::findings::Finding; 0]
    );
    let later = NOW + 15.0 * 60.0;
    health_at(&history, "meter", "error", failed, later);
    let found = health(&Picture::read(&history, &config, later).unwrap());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Note);
    assert_eq!(found[0].title, "the meter can't be read");
    assert_eq!(found[0].badge, "since 14:29");
    assert_eq!(found[0].note, "the meter couldn't be reached");
    // Fine for a moment between starts the count again.
    health_at(&history, "meter", "fine", "live", later + 30.0);
    health_at(&history, "meter", "error", failed, later + 60.0);
    assert_eq!(
        health(&Picture::read(&history, &config, later + 60.0).unwrap()),
        [] as [crate::findings::Finding; 0]
    );
}
