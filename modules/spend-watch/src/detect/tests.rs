//! What the detectors' tests share, a config, a turn and the moment they
//! judge at, and the tests of what's found whole and of the notes the
//! margin shows.

use super::*;
use crate::logs::{Cost, Event, Kind, Tokens, Turn};
use crate::time::{DAY, HOUR};

pub(super) fn config() -> Config {
    serde_json::from_value(serde_json::json!({
        "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
        "state": "/none", "zone": "UTC",
    }))
    .unwrap()
}

pub(super) fn turn(history: &History, session: &str, at: f64, dollars: f64) {
    history
        .add_event(&Event::Turn(Turn {
            session: session.into(),
            at,
            model: "m".into(),
            provider: "p".into(),
            kind: Kind::Agent,
            tokens: Tokens::default(),
            cost: Cost {
                total: dollars,
                ..Cost::default()
            },
            quest: None,
            budget: None,
            gap: None,
            rebuilt: 0.0,
            rebuilt_cost: 0.0,
            cause: None,
        }))
        .unwrap();
}

pub(super) const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z, a Wednesday

// Each note that asks for a look fits the margin's footer at its
// widest: a five-figure cap, every refusal of a day, all of the spend.
#[test]
fn what_asks_for_a_look_fits_the_margin() {
    for note in [
        ending_note(1.0, 15_000.0),
        changed_note(Some(15_000.0), 15_000.0),
        changed_note(None, 15_000.0),
        refused_note(999, "Wed 23:10"),
        NEW_NOTE.into(),
        elsewhere_note(1.0, 1.0),
    ] {
        let shown = note.chars().count();
        assert!(shown <= crate::status::FOOTER, "{note}: {shown}");
    }
}

// A day leaning on one model is a choice already made, not news: over
// 26 weeks each of 32 such notes was a switch to a newer model, for a
// week at a time, which a new model's note had said once.
#[test]
fn a_day_leaning_on_one_model_is_no_finding() {
    let (history, picture) = leaning("y");
    let found = all(&picture, &history, &config()).unwrap();
    assert!(
        !found.iter().any(|f| f.title.starts_with("today leans on")),
        "{:?}",
        found.iter().map(|f| &f.title).collect::<Vec<_>>()
    );
}

// Twenty-eight days of model m from nine to five, with $5 of y each
// morning, and today $900 of `model`.
fn leaning(model: &str) -> (History, Picture) {
    let history = History::memory();
    let today = (NOW / DAY).floor() * DAY;
    for back in 1..28 {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "meter", "tool": "all",
            "hour": crate::time::rfc3339(today - f64::from(back) * DAY + 7.0 * HOUR),
            "model": "y", "dollars": 5.0,
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, NOW).unwrap();
    }
    for back in 0..28 {
        for minute in (9 * 60..17 * 60).step_by(10) {
            let at = today - f64::from(back) * DAY + f64::from(minute) * 60.0;
            if at < NOW - 3600.0 {
                turn(&history, &format!("s{back}"), at, 2.0);
            }
        }
    }
    let line = serde_json::json!({
        "v": 1, "kind": "billed", "source": "meter", "tool": "all",
        "hour": crate::time::rfc3339(NOW - 1800.0), "model": model, "dollars": 900.0,
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(&history, "proxy", &record, NOW).unwrap();
    let picture = Picture::read(&history, &config(), NOW).unwrap();
    (history, picture)
}
