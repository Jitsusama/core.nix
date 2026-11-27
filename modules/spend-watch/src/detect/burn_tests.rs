//! The tests of the burn: the minutes a daemon keeps, read whole only when
//! an hour gone changes, and which hours burn and which don't.

use super::*;
use crate::detect::tests::{NOW, config, turn};
use crate::time::HOUR;

// Eight weeks of the same working days, a dollar every ten minutes
// from nine to five, in a session a day.
fn ordinary() -> History {
    let history = History::memory();
    let today = (NOW / DAY).floor() * DAY;
    for back in 0..56 {
        let day = today - f64::from(back) * DAY;
        for minute in (9 * 60..17 * 60).step_by(10) {
            let at = day + f64::from(minute) * 60.0;
            if at < NOW - 3600.0 {
                turn(&history, &format!("s{back}"), at, 1.0);
            }
        }
    }
    history
}

fn burns_at(history: &History, now: f64) -> Vec<Finding> {
    let config = config();
    let picture = Picture::read(history, &config, now).unwrap();
    burn(&picture, history, &config, &mut Minutes::default()).unwrap()
}

fn burns(history: &History) -> Vec<Finding> {
    burns_at(history, NOW)
}

// A running daemon reads the hour under way on every pass, and the
// hours gone only once one of them gains or loses a turn, so what it
// has is what a fresh read finds, a write rolled back included.
#[test]
fn kept_minutes_are_read_whole_only_when_an_hour_gone_changes() {
    let history = ordinary();
    let from = NOW - 56.0 * DAY;
    let fresh = |now: f64| Minutes::default().read(&history, from, now).unwrap();
    let mut kept = Minutes::default();
    assert_eq!(kept.read(&history, from, NOW).unwrap(), fresh(NOW));
    turn(&history, "hot", NOW - 30.0, 10.0);
    assert_eq!(
        kept.read(&history, from, NOW + 1.0).unwrap(),
        fresh(NOW + 1.0)
    );
    assert_eq!(kept.whole, 1, "a turn in the hour under way");
    turn(&history, "late", NOW - 5.0 * HOUR, 7.0);
    assert_eq!(
        kept.read(&history, from, NOW + 2.0).unwrap(),
        fresh(NOW + 2.0)
    );
    assert_eq!(kept.whole, 2, "a turn arriving late for an hour gone");
    history
        .db
        .execute("DELETE FROM turns WHERE session = 'late'", [])
        .unwrap();
    assert_eq!(
        kept.read(&history, from, NOW + 3.0).unwrap(),
        fresh(NOW + 3.0)
    );
    assert_eq!(kept.whole, 3, "a turn forgotten from an hour gone");
    history.db.execute_batch("BEGIN").unwrap();
    turn(&history, "undone", NOW - 5.0 * HOUR, 3.0);
    kept.read(&history, from, NOW + 4.0).unwrap();
    history.db.execute_batch("ROLLBACK").unwrap();
    turn(&history, "redone", NOW - 5.0 * HOUR, 5.0);
    assert_eq!(
        kept.read(&history, from, NOW + 5.0).unwrap(),
        fresh(NOW + 5.0),
        "a move handed out again after a rollback was missed"
    );
    assert_eq!(kept.whole, 5, "a write not committed was kept as read");
    let next = crate::time::hour_start(crate::time::hour_of(NOW) + 1);
    assert_eq!(kept.read(&history, from, next).unwrap(), fresh(next));
    assert_eq!(kept.whole, 6, "the next hour");
}

#[test]
fn an_ordinary_hour_is_no_burn() {
    let history = ordinary();
    for minute in (0..60).step_by(10) {
        turn(&history, "s0", NOW - 3600.0 + f64::from(minute) * 60.0, 1.0);
    }
    assert_eq!(burns(&history), [] as [crate::findings::Finding; 0]);
}

// Ten dollars a minute for the last hour is one burn, in the session
// that spent it.
#[test]
fn a_burning_hour_names_its_session() {
    let history = ordinary();
    for minute in 0..60 {
        turn(
            &history,
            "hot",
            NOW - 3600.0 + f64::from(minute) * 60.0,
            10.0,
        );
    }
    let found = burns(&history);
    assert_eq!(found.len(), 1, "{found:?}");
    let day = Zone::iso_day(Zone::utc().day(NOW));
    assert_eq!(found[0].key, format!("burn:{day}"));
    assert_eq!(found[0].session.as_deref(), Some("hot"));
    assert_eq!(found[0].note, "a working hour is $6");
}

// A burn is something to look at, never to be told: the caps say when
// spending fast is about to cost access.
#[test]
fn a_burn_is_a_note() {
    let history = ordinary();
    for minute in 0..60 {
        turn(
            &history,
            "hot",
            NOW - 3600.0 + f64::from(minute) * 60.0,
            10.0,
        );
    }
    let found = burns(&history);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Note);
}

// A burn in a session here offers to go to it; one in another
// machine's has no terminal here to go to, so it offers nothing.
#[test]
fn a_burn_elsewhere_offers_no_session_to_go_to() {
    let history = ordinary();
    for minute in 0..60 {
        turn(
            &history,
            "hot",
            NOW - 3600.0 + f64::from(minute) * 60.0,
            10.0,
        );
    }
    let action = |history: &History| burns(history)[0].action.clone().map(|a| a.run);
    assert_eq!(
        action(&history),
        Some(
            ["spend-watch", "sessions", "hot", "go"]
                .map(String::from)
                .to_vec()
        ),
        "a burn here didn't offer its session"
    );
    history
        .db
        .execute(
            "INSERT INTO sessions VALUES ('hot', '/x', '', 0, NULL, NULL, 'teal')",
            [],
        )
        .unwrap();
    assert_eq!(
        action(&history),
        None,
        "a burn on another machine offered to go to it"
    );
}

// Agents left working overnight at a working hour's pace aren't a burn,
// though the night usually spends nothing: on 15 October they were, at
// $151 an hour against a usual $3 for 4 a.m.
#[test]
fn working_through_the_night_is_no_burn() {
    let history = ordinary();
    let night = (NOW / DAY).floor() * DAY + 5.0 * 3600.0;
    for minute in (0..60).step_by(10) {
        turn(
            &history,
            "agent",
            night - 3600.0 + f64::from(minute) * 60.0,
            1.5,
        );
    }
    assert_eq!(
        burns_at(&history, night),
        [] as [crate::findings::Finding; 0]
    );
}

// A burn is the account's spend running hot, so it's one burn however
// many sessions take the lead in it: on 28 July one asked four times
// as its busiest session changed, and marking one expected left the
// next. It still names the session spending most now.
#[test]
fn a_burn_is_one_however_its_sessions_change() {
    let history = ordinary();
    for minute in 0..60 {
        turn(
            &history,
            "first",
            NOW - 7200.0 + f64::from(minute) * 60.0,
            10.0,
        );
        turn(
            &history,
            "second",
            NOW - 3600.0 + f64::from(minute) * 60.0,
            10.0,
        );
    }
    let earlier = burns_at(&history, NOW - 3600.0);
    let now = burns(&history);
    assert_eq!((earlier.len(), now.len()), (1, 1), "{earlier:?} {now:?}");
    assert_eq!(earlier[0].key, now[0].key);
    assert_eq!(now[0].session.as_deref(), Some("second"));
}

// A day of logs at a steady three dollars an hour knows no usual hour
// yet, so it's no burn at any hour of the next day.
#[test]
fn a_day_of_logs_is_no_burn() {
    let history = History::memory();
    let start = NOW - 2.0 * DAY;
    for half in 0..96 {
        turn(&history, "s", start + f64::from(half) * 1800.0, 1.5);
    }
    for hour in 0..24 {
        let now = start + DAY + f64::from(hour) * 3600.0;
        assert!(burns_at(&history, now).is_empty(), "a burn at {hour}");
    }
}

// A burn from six to seven in the evening, when a usual hour allows a
// dollar, still scores high at half past eight; but nothing is burning.
#[test]
fn a_burn_that_stopped_clears() {
    let history = ordinary();
    let evening = (NOW / DAY).floor() * DAY + 20.5 * 3600.0;
    for minute in 0..60 {
        turn(
            &history,
            "hot",
            evening - 9000.0 + f64::from(minute) * 60.0,
            10.0,
        );
    }
    assert_eq!(
        burns_at(&history, evening),
        [] as [crate::findings::Finding; 0]
    );
}
