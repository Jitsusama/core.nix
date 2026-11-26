//! The tests of when the tokens run out.

use super::*;

// A reading on Sat 31 Oct 2026 at 05:02:34 in Toronto, and two minutes on.
const AT: f64 = 1_793_437_354.0;
const NOW: f64 = AT + 120.0;

fn toronto() -> Zone {
    Zone::named("America/Toronto").expect("a zone")
}

fn cap(spent: f64, at: f64, state: &str) -> Cap {
    Cap {
        feed: "proxy".into(),
        budget: String::new(),
        name: "weekly".into(),
        label: "Week".into(),
        unit: crate::caps::Unit::Dollars,
        allowance: Some(15_000.0),
        window_hours: 168,
        spent,
        warn: 0.9,
        fallback: None,
        state: state.into(),
        ends: None,
        ends_known: false,
        resets: None,
        used: None,
        at,
    }
}

fn inputs(spent: f64, rate: f64, rolled: f64) -> Inputs {
    Inputs {
        cap: cap(spent, AT, "ok"),
        rate,
        rolling: vec![rolled; AHEAD + 1],
        top: Some(("01a19269".into(), Some("QEST-20261025-5FWIUS".into()))),
        rises: Vec::new(),
        refused: None,
        counter: "the AI proxy".into(),
    }
}

// The Tokens row opens the drawer of its cap's row in the spend
// collection, so it names that row by the key the collection gives it.
#[test]
fn the_outlook_names_its_caps_row() {
    let given = inputs(14_560.0, 300.0, 0.0);
    let tokens = outlook(&given, None, &toronto(), NOW).unwrap();
    assert_eq!(tokens.row, "cap:weekly");
    let unread = unread(&given, &toronto(), NOW);
    assert_eq!(unread.row, "cap:weekly");
}

#[test]
fn a_refusal_is_out_whatever_the_forecast_says() {
    // The forecast says the run at 06:02, but a session was refused at
    // 05:30: out now, and back once what's counted at 06:02 rolls out.
    let mut given = inputs(14_800.0, 300.0, 400.0);
    given.rolling[1] = 0.0;
    given.refused = Some(AT + 1680.0);
    let now = AT + 1800.0;
    let tokens = outlook(&given, Some(Level::Hour), &toronto(), now).unwrap();
    assert_eq!(tokens.level, Level::Out);
    assert_eq!(tokens.title, "Tokens Ran Out");
    assert_eq!(tokens.runs_out, Some(AT + 1680.0));
    assert_eq!(tokens.back, Some(AT + 2.0 * HOUR));
    assert_eq!(tokens.value, "resets 07:02");
    // With nothing forecast to reach it, back at the first run after.
    let mut early = inputs(13_000.0, 300.0, 200.0);
    early.refused = Some(AT + 1680.0);
    let tokens = outlook(&early, None, &toronto(), now).unwrap();
    assert_eq!(tokens.back, Some(AT + HOUR));
}

#[test]
fn under_five_minutes_it_says_under() {
    let tokens = outlook(&inputs(14_800.0, 300.0, 0.0), None, &toronto(), AT + 3420.0);
    assert_eq!(tokens.unwrap().title, "Tokens Run Out in Under 5 Min");
}

#[test]
fn a_count_that_rose_in_a_jump_lately_is_taken_to_again() {
    // $440 left at $300 an hour would be the second run, but the count
    // rose $524 at a run lately, so it could be the first.
    let mut given = inputs(14_560.0, 300.0, 0.0);
    let later = outlook(&given, None, &toronto(), NOW).unwrap();
    assert_eq!(later.runs_out, Some(AT + 2.0 * HOUR));
    given.rises = vec![360.0, -79.0, 524.0, 212.0];
    let sooner = outlook(&given, None, &toronto(), NOW).unwrap();
    assert_eq!(sooner.runs_out, Some(AT + HOUR));
    assert_eq!(sooner.level, Level::Hour);
}

#[test]
fn a_cap_far_off_says_nothing() {
    assert_eq!(
        outlook(&inputs(10_000.0, 300.0, 0.0), None, &toronto(), NOW),
        None
    );
}

#[test]
fn it_names_the_run_the_cap_is_reached_at() {
    // $1,200 left at $300 an hour less $0 rolling out: the fourth run.
    let tokens = outlook(&inputs(13_800.0, 300.0, 0.0), None, &toronto(), NOW).unwrap();
    assert_eq!(tokens.level, Level::Hours);
    assert_eq!(tokens.runs_out, Some(AT + 4.0 * HOUR));
    assert_eq!(tokens.title, "Tokens Run Out in About 4 Hours");
    assert_eq!(tokens.hint, "Tokens run out in about 4 hours at this rate");
    assert_eq!(
        tokens.context,
        "$13,800 of $15,000 by the AI proxy's 05:02 count · $300 an hour"
    );
    assert_eq!(tokens.quiet.context, "92% by the AI proxy's 05:02 count");
    assert!(
        !tokens.now,
        "four hours off was raised over what Joel is doing"
    );
}

#[test]
fn what_rolls_out_slows_it() {
    // $1,200 left at $300 an hour less $100 rolling out: the sixth run.
    let tokens = outlook(&inputs(13_800.0, 300.0, 100.0), None, &toronto(), NOW);
    assert_eq!(tokens, None, "six hours off is past the first level");
    let tokens = outlook(&inputs(14_200.0, 300.0, 100.0), None, &toronto(), NOW).unwrap();
    assert_eq!(tokens.runs_out, Some(AT + 4.0 * HOUR));
    // Spending stopped then, $100 rolls out by the first run after.
    assert_eq!(tokens.back, Some(AT + 5.0 * HOUR));
    assert_eq!(tokens.value, "resets 10:02");
    assert_eq!(
        tokens.hint,
        "Tokens run out in about 4 hours at this rate; they reset at 10:02"
    );
}

#[test]
fn the_next_run_is_pressing() {
    let now = AT + HOUR - 600.0;
    let tokens = outlook(&inputs(14_800.0, 300.0, 0.0), None, &toronto(), now).unwrap();
    assert_eq!(tokens.level, Level::Minutes);
    assert!(
        tokens.now,
        "ten minutes off wasn't raised over what Joel is doing"
    );
    assert_eq!(tokens.title, "Tokens Run Out in About 10 Min");
    assert_eq!(tokens.actions[1].name, "Pause 5FWIUS");
    assert_eq!(tokens.actions[1].session.as_deref(), Some("01a19269"));
}

#[test]
fn a_wavering_rate_keeps_the_level_it_reached() {
    // Under an hour left, then a lull puts it at the second run, 70
    // minutes off: still "hour", since it's not yet past 90 minutes.
    let given = inputs(14_850.0, 75.0, 0.0);
    let now = AT + 3000.0;
    let fresh = outlook(&given, None, &toronto(), now).unwrap();
    assert_eq!(fresh.level, Level::Hours);
    let held = outlook(&given, Some(Level::Hour), &toronto(), now).unwrap();
    assert_eq!(held.level, Level::Hour);
    // 110 minutes off is past it, so it's left.
    let left = outlook(&given, Some(Level::Hour), &toronto(), AT + 600.0).unwrap();
    assert_eq!(left.level, Level::Hours);
    // And "hours" holds a little past four hours: the fifth run, 4 h 20
    // min off.
    let far = inputs(13_740.0, 300.0, 0.0);
    assert_eq!(outlook(&far, None, &toronto(), AT + 2400.0), None);
    let kept = outlook(&far, Some(Level::Hours), &toronto(), AT + 2400.0).unwrap();
    assert_eq!(kept.level, Level::Hours);
}

#[test]
fn refused_it_says_when_it_is_back() {
    let mut given = inputs(15_144.0, 300.0, 200.0);
    given.cap.state = "refused".into();
    let tokens = outlook(&given, None, &toronto(), NOW).unwrap();
    assert_eq!(tokens.level, Level::Out);
    assert!(
        tokens.now,
        "tokens run out weren't raised over what Joel is doing"
    );
    assert_eq!(tokens.title, "Tokens Ran Out");
    assert_eq!(tokens.back, Some(AT + HOUR));
    assert_eq!(tokens.hint, "Tokens ran out; they reset at 06:02");
}

#[test]
fn a_reading_gone_stale_near_the_cap_is_said_in_the_row() {
    let tokens = outlook(
        &inputs(14_000.0, 300.0, 0.0),
        None,
        &toronto(),
        AT + 3.0 * HOUR,
    )
    .unwrap();
    assert_eq!(tokens.level, Level::Unread);
    assert_eq!(tokens.unread.as_deref(), Some("can't be read since 05:02"));
    assert_eq!(tokens.quiet.unread, tokens.unread);
    assert!(
        !tokens.now,
        "a stale reading was raised over what Joel is doing"
    );
    assert_eq!(
        outlook(
            &inputs(9_000.0, 300.0, 0.0),
            None,
            &toronto(),
            AT + 3.0 * HOUR
        ),
        None
    );
}

/// The columns a cap's reading had before budgets.
const CAPS: &str = "(feed, name, at, label, allowance, window_hours, spent, warn, \
    fallback, state, ends, ends_known, resets)";

// The meter's hour, what rolls out and a refusal, each only the cap's
// own budget's: a team's token billed beside it and refused says
// nothing of it.
#[test]
fn it_reads_the_meter_and_what_rolls_out() {
    let history = History::memory();
    history
        .db
        .execute_batch(&format!(
            "INSERT INTO caps {CAPS} VALUES ('proxy', 'weekly', {AT}, 'weekly cap', 15000,
             168, 14000, 0.9, NULL, 'ok', NULL, 0, NULL);
             INSERT INTO caps {CAPS} VALUES ('proxy', 'premium', {AT}, 'premium day', 1000,
             24, 0, 0.5, NULL, 'ok', NULL, 0, NULL);"
        ))
        .unwrap();
    let hour = hour_of(AT);
    let now = hour_start(hour) + 1800.0;
    let bill = |source: &str, hour: i64, dollars: f64, budget: &str| {
        history
            .db
            .execute(
                "INSERT INTO billed VALUES ('proxy', ?1, ?2, 'pi', 'm', 'v', 's', 'a', 't',
                 0, 0, 0, 0, ?3, 0, 0, ?4)",
                rusqlite::params![source, hour, dollars, budget],
            )
            .unwrap();
    };
    bill("meter", hour, 150.0, "");
    bill("meter", hour - 1, 240.0, "");
    bill("warehouse", hour - 168, 90.0, "");
    bill("meter", hour, 1000.0, "delta");
    bill("warehouse", hour - 168, 1000.0, "delta");
    history
        .db
        .execute_batch(&format!(
            "INSERT INTO refusals VALUES ('gone', {r}, 'm', 'over your weekly cap');
             INSERT INTO refusals VALUES ('back', {r}, 'm', 'over your weekly cap');
             INSERT INTO refusals VALUES ('team', {t}, 'm', 'over your weekly cap');",
            r = AT + 600.0,
            t = AT + 1200.0,
        ))
        .unwrap();
    let turn = |session: &str, at: f64, budget: Option<&str>| {
        history
            .db
            .execute(
                "INSERT INTO turns (session, at, hour, model, provider, kind, input, output,
                 cache_read, cache_write, cache_write_long, cost, cost_cache_write,
                 cost_output, rebuilt, rebuilt_cost, budget) VALUES
                 (?1, ?2, 0, 'm', 'p', 'k', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ?3)",
                rusqlite::params![session, at, budget],
            )
            .unwrap_or_else(|error| panic!("a turn: {error}"));
    };
    turn("back", AT + 900.0, None);
    turn("team", AT + 300.0, Some("delta"));
    let read = read(&history, &Budgets::implicit(), now, Rate::Live)
        .unwrap()
        .unwrap();
    // One session got past its refusal and one didn't: out.
    assert_eq!(read.refused, Some(AT + 600.0));
    assert_eq!(read.cap.name, "weekly");
    // The half hour so far and the later half of the hour before.
    assert!(
        (read.rate - 270.0).abs() < 1e-9,
        "the hour's rate read {}, not $270",
        read.rate
    );
    assert!(
        (read.rolling[0] - 90.0).abs() < 1e-9,
        "the hour now rolls out {}, not $90",
        read.rolling[0]
    );
}
