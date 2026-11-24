use super::super::measure;
use super::super::measures::Class;
use super::*;

/// 14 November 2026 at noon, UTC.
const NOW: i64 = 1_794_657_600;
const HOUR: i64 = 3600;

#[test]
fn a_run_goes_on_while_out_ends_when_in_and_waits_through_an_unjudged_hour() {
    let mut out = BTreeMap::new();
    let ship = |now: f64, target: Option<f64>| measure("ship", Class::Ship, now, target);
    track(&mut out, &[ship(90.0, Some(18.0))], NOW);
    track(&mut out, &[ship(90.0, None)], NOW + HOUR);
    assert_eq!(
        out.get("ship").map(|run| run.seen),
        Some(NOW),
        "an unjudged hour was taken as seen out"
    );
    track(&mut out, &[ship(60.0, Some(18.0))], NOW + 2 * HOUR);
    assert_eq!(
        out.get("ship"),
        Some(&Out {
            since: NOW,
            seen: NOW + 2 * HOUR,
        }),
        "an unjudged hour broke the run, or a judged one didn't carry it"
    );
    track(&mut out, &[ship(10.0, Some(18.0))], NOW + 3 * HOUR);
    assert!(
        out.is_empty(),
        "a measure back within its target kept its run: {out:?}"
    );
}

#[test]
fn a_run_unjudged_for_a_day_is_let_go() {
    let mut out = BTreeMap::new();
    let gone = |target: Option<f64>| measure("gone", Class::Clean, 9.0, target);
    track(&mut out, &[gone(Some(1.0))], NOW);
    track(&mut out, &[], NOW + HOUR);
    track(&mut out, &[gone(None)], NOW + DAY - HOUR);
    assert!(out.contains_key("gone"), "a run went inside a day");
    track(&mut out, &[gone(None)], NOW + DAY);
    assert!(
        out.is_empty(),
        "a measure unread a day kept its run: {out:?}"
    );
}

#[test]
fn the_measures_out_three_days_are_named_on_the_days_first_card_alone() {
    let ship = [measure("ship", Class::Ship, 90.0, Some(18.0))];
    let mut kept = Kept::default();
    // Out every six hours from three days back, the last early on the
    // day it turns three days: none named, and no day used up.
    for at in (NOW - 3 * DAY..NOW).step_by(6 * 3600) {
        let mut early = String::new();
        daily(&mut early, &mut kept, &ship, at, &TimeZone::UTC);
        assert!(early.is_empty(), "named before three days: {early:?}");
    }
    assert!(
        kept.out_told.is_none(),
        "a card with none to name used up its day"
    );
    let mut first = String::new();
    daily(&mut first, &mut kept, &ship, NOW, &TimeZone::UTC);
    assert!(
        first.contains("ship said"),
        "the day's first card: {first:?}"
    );
    let mut second = String::new();
    daily(&mut second, &mut kept, &ship, NOW + HOUR, &TimeZone::UTC);
    assert!(second.is_empty(), "named twice in a day: {second:?}");
    let mut next_day = String::new();
    daily(&mut next_day, &mut kept, &ship, NOW + DAY, &TimeZone::UTC);
    assert!(
        next_day.contains("ship said"),
        "the next day's first card: {next_day:?}"
    );
}

#[test]
fn a_run_is_named_by_whose_lines_it_holds_and_kept_whole_across_one_key() {
    let mut clean = measure("clean", Class::Clean, 9.0, Some(1.0));
    clean.behind = vec!["a.rs · ZED".to_owned(), "b.rs · AMY".to_owned()];
    let misses = measure("misses", Class::Miss, 2.0, Some(0.0));
    let within = measure("clean", Class::Clean, 0.0, Some(1.0));
    let mut out = BTreeMap::new();
    let measures = [clean.clone(), within, misses.clone()];
    track(&mut out, &measures, NOW - 3 * DAY);
    assert_eq!(out.len(), 2, "a key within beside one out ended it");
    let lines = named(&out, &[clean, misses], NOW, &TimeZone::UTC);
    assert!(
        lines.contains("out since 2026-11-11, AMY, ZED\n")
            && lines.contains("out since 2026-11-11, each miss's owner\n"),
        "a run's owner wasn't who its lines name: {lines}"
    );
}

#[test]
fn a_clock_set_back_leaves_no_run_ahead_of_it() {
    let ship = [measure("ship", Class::Ship, 90.0, Some(18.0))];
    let mut out = BTreeMap::new();
    track(&mut out, &ship, NOW + DAY);
    track(&mut out, &ship, NOW);
    assert_eq!(
        out.get("ship").map(|run| run.since),
        Some(NOW),
        "a run begun ahead of the clock stayed ahead"
    );
    track(&mut out, &[], NOW - HOUR);
    assert!(out.is_empty(), "a run seen ahead of the clock was kept");
}

#[test]
fn the_card_names_each_measure_out_three_days_with_its_owner() {
    let measures = [
        measure("ship", Class::Ship, 90.0, Some(18.0)),
        measure("young", Class::Clean, 9.0, Some(1.0)),
        measure("back", Class::Apps, 1.0, Some(2.0)),
        measure("unjudged", Class::Cells, 9.0, None),
    ];
    let run = |since: i64| Out { since, seen: NOW };
    let out = BTreeMap::from([
        ("ship".to_owned(), run(NOW - 3 * DAY)),
        ("young".to_owned(), run(NOW - 3 * DAY + HOUR)),
        ("back".to_owned(), run(NOW - 5 * DAY)),
        ("unjudged".to_owned(), run(NOW - 5 * DAY)),
    ]);
    assert_eq!(
        named(&out, &measures, NOW, &TimeZone::UTC),
        "Out of spec three days running, each with its owner:\n  \
         - ship said: 90 against at most 18, out since 2026-11-11, OWNER\n",
        "named one out under three days, one within its target or one unjudged"
    );
    assert_eq!(
        named(&out, &measures[1..], NOW, &TimeZone::UTC),
        "",
        "named a line with no measure out three days"
    );
}
