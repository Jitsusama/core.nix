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
    let gone = measure("gone", Class::Clean, 9.0, Some(1.0));
    track(&mut out, &[gone], NOW);
    track(&mut out, &[], NOW + DAY - HOUR);
    assert!(out.contains_key("gone"), "a run went before a day unread");
    track(&mut out, &[], NOW + DAY);
    assert!(
        out.is_empty(),
        "a measure unread a day kept its run: {out:?}"
    );
}

#[test]
fn the_measures_out_three_days_are_named_on_the_days_first_card_alone() {
    let ship = [measure("ship", Class::Ship, 90.0, Some(18.0))];
    let mut kept = Kept::default();
    kept.out.insert(
        "ship".to_owned(),
        Out {
            since: NOW - 3 * DAY,
            seen: NOW - HOUR,
        },
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
