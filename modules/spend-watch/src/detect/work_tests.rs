//! The tests of what the work spends for little: rebuilding against its
//! usual share, and a quest moving in its subquest.

use super::*;
use crate::detect::tests::{NOW, config};
use crate::history::History;
use crate::logs::{Cost, Event, Kind, Tokens, Turn};
use crate::time::DAY;

// A day's turn of a hundred dollars, so much of it rebuilt after a
// compaction, for each of so many days before now.
fn rebuilding(history: &History, days: std::ops::Range<i64>, share: impl Fn(i64) -> f64) {
    for back in days {
        history
            .add_event(&Event::Turn(Turn {
                session: format!("s{back}"),
                at: NOW - crate::time::hour_start(back * 24) - 3600.0,
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: 100.0,
                    ..Cost::default()
                },
                quest: None,
                budget: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 100.0 * share(back),
                cause: Some(crate::logs::Cause::Compaction),
            }))
            .unwrap();
    }
}

fn wasted(history: &History) -> Vec<Finding> {
    let config = config();
    waste(&Picture::read(history, &config, NOW).unwrap(), &config)
}

// A quarter of every week rebuilt is how this account works, not
// news: it stood in the collection every hour for 26 weeks.
#[test]
fn rebuilding_as_usual_is_no_finding() {
    let history = History::memory();
    rebuilding(&history, 0..63, |_| 0.25);
    assert!(wasted(&history).is_empty(), "{:?}", wasted(&history));
}

// A week rebuilding four times its usual share is, as when August's
// rose from a quarter to a half.
#[test]
fn rebuilding_past_the_usual_is_a_note() {
    let history = History::memory();
    rebuilding(&history, 0..63, |back| if back < 7 { 0.4 } else { 0.1 });
    let found = wasted(&history);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].note, "40% of all spent, usually 10%");
}

// Three weeks before can't say what's usual.
#[test]
fn rebuilding_with_too_few_weeks_is_no_finding() {
    let history = History::memory();
    rebuilding(&history, 0..28, |back| if back < 7 { 0.4 } else { 0.1 });
    assert!(wasted(&history).is_empty(), "{:?}", wasted(&history));
}

fn quest(id: &str, parent: Option<&str>, updated: &str) -> crate::quests::Quest {
    crate::quests::Quest {
        id: id.into(),
        title: id.into(),
        kind: if parent.is_some() {
            "subquest"
        } else {
            "quest"
        }
        .into(),
        parent: parent.map(Into::into),
        status: "active".into(),
        started: None,
        updated: Some(updated.into()),
        prs: 0,
        sessions: Vec::new(),
    }
}

fn stalled_keys(updated: &str) -> Vec<String> {
    let history = History::memory();
    let quests = [
        quest("root", None, "2026-09-22"),
        quest("child", Some("root"), updated),
    ];
    crate::quests::store(&history, &quests).unwrap();
    for day in 0..4 {
        history
            .add_event(&Event::Turn(Turn {
                session: "s".into(),
                at: NOW - f64::from(day) * DAY - 3600.0,
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: 50.0,
                    ..Cost::default()
                },
                quest: Some("child".into()),
                budget: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }
    let config = config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    stalled(&picture, &config)
        .into_iter()
        .map(|found| found.key)
        .collect()
}

// A subquest's spend counts toward its root, so its changes do too: on
// 15 October one was called unchanged for 22 days by its root's record
// while its subquest changed that morning.
#[test]
fn a_quest_moving_in_its_subquest_isnt_stalled() {
    assert_eq!(stalled_keys("2026-10-14"), [] as [std::string::String; 0]);
    assert_eq!(stalled_keys("2026-09-30"), ["stalled:root"]);
}
