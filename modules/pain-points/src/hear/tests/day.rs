//! The keeper's daily count, and the status asks of the day: what the
//! keeper hears in place of each pain heard too late for its owner.

use super::*;

const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

/// 02:00 on the 17th where he is, five hours behind UTC: two hours after
/// the 16th ended, when its count goes.
const NEXT_NIGHT: i64 = AT + 11 * HOUR;

/// One run at `at`: what the keeper was told of whole days, and the hour.
/// Each send of a day's count goes when `counts_go`; every other send goes.
fn run(place: &Place, at: i64, counts_go: bool) -> (Vec<String>, Hour) {
    let mut ears = place.ears();
    ears.at = at;
    run_with(&ears, CELLS, counts_go)
}

fn run_with(ears: &Ears, cells: &str, counts_go: bool) -> (Vec<String>, Hour) {
    let mut counted = Vec::new();
    let hour = hear(ears, cells, &mut |to, text| {
        if to == "ORCH01" && text.starts_with("Joel's day") {
            counted.push(text.to_string());
            return counts_go;
        }
        true
    })
    .expect("a run");
    (counted, hour)
}

/// What the keeper was told of whole days, over runs at each of `ats`.
fn counts(place: &Place, ats: &[i64]) -> Vec<String> {
    ats.iter().flat_map(|at| run(place, *at, true).0).collect()
}

fn hour_says(hour: &Hour, said: &str) -> bool {
    hour.troubles.iter().any(|t| t.contains(said))
}

#[test]
fn the_keeper_hears_each_day_counted_once_after_it_ends() {
    let place = Place::new("day");
    place.routes();
    a_day(&place);
    assert!(
        counts(&place, &[AT]).is_empty(),
        "a day still going, and the first a run hears, isn't counted yet"
    );
    assert!(
        counts(&place, &[AT + 9 * HOUR + 1_800_000]).is_empty(),
        "at 00:30 a day's count waits for what's still on its way"
    );
    let counted = counts(&place, &[NEXT_NIGHT, NEXT_NIGHT + 300_000]);
    assert_eq!(counted.len(), 1, "once a day: {counted:?}");
    let day = &counted[0];
    assert!(
        !day.contains(", from "),
        "a first run reads a day back, so it hears its day whole: {day}"
    );
    for said in [
        "2026-11-16",
        "4 pains heard",
        "1 asking how something stands",
        "3 sent to their owners",
        "1 the ledgers already carried",
        "0 given up",
        "0 sent to no one",
        "0 turns stalled him",
    ] {
        assert!(day.contains(said), "{said:?} in {day}");
    }
}

#[test]
fn a_day_is_his_own_and_holds_what_was_heard_too_late() {
    let place = Place::new("day-zone");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    // 22:00 on the 16th where he is, but the 17th in UTC, said in a session
    // whose cell is live, and first heard four hours later, too late for it.
    place.log(
        "late",
        "s-late",
        Path::new("/home/j/src"),
        &[user("x1", "2026-11-17T03:00:00.000Z", "no, that's wrong")],
    );
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-late","address":"LATE01","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"}]}}"#;
    let mut ears = place.ears();
    ears.at = NEXT_NIGHT;
    let mut counted = Vec::new();
    let mut to_late = 0;
    hear(&ears, cells, &mut |to, text| {
        to_late += usize::from(to == "LATE01");
        if text.starts_with("Joel's day") {
            counted.push(text.to_string());
        }
        true
    })
    .expect("a run");
    assert_eq!(to_late, 0, "its cell has moved on");
    assert_eq!(counted.len(), 1, "{counted:?}");
    let day = &counted[0];
    assert!(
        day.contains("5 pains heard") && day.contains("1 sent to no one"),
        "a late pain is said in his own day's count: {day}"
    );
}

#[test]
fn a_pain_sent_and_then_carried_is_counted_once() {
    let place = Place::new("day-once");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    // The keeper writes the pain it was just told of into its ledger.
    fs::write(
        place.root.join("misses.jsonl"),
        r#"{"at":"x","words":"no, that's broken again","owner":"Y"}"#,
    )
    .expect("a ledger");
    let counted = counts(&place, &[AT + 300_000, NEXT_NIGHT]);
    assert!(
        place
            .heard()
            .iter()
            .any(|h| h.entry == "e1" && h.carried && h.routed_at.is_some()),
        "sent, then carried: {:?}",
        place.heard()
    );
    let day = &counted[0];
    assert!(
        day.contains("3 sent to their owners") && day.contains("1 the ledgers already carried"),
        "each pain in one part of the count: {day}"
    );
}

#[test]
fn a_day_whose_count_did_not_go_is_counted_on_the_next_run() {
    let place = Place::new("day-again");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    run(&place, NEXT_NIGHT, false);
    let counted = counts(&place, &[NEXT_NIGHT + 300_000, NEXT_NIGHT + 600_000]);
    assert_eq!(counted.len(), 1, "{counted:?}");
    assert!(counted[0].contains("2026-11-16"), "{counted:?}");
}

#[test]
fn a_count_the_node_never_takes_is_sent_three_times_then_given_up() {
    let place = Place::new("day-tries");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    let mut sent = 0;
    let mut last = Hour::default();
    for n in 0..5 {
        let (counted, hour) = run(&place, NEXT_NIGHT + n * 300_000, false);
        sent += counted.len();
        last = hour;
        if n == 2 {
            assert!(
                hour_says(&last, "gave up on 2026-11-16's count"),
                "{last:?}"
            );
        }
    }
    assert_eq!(sent, 3, "a count read as failed isn't sent every five minutes");
    assert!(!hour_says(&last, "count"), "given up once: {last:?}");
}

#[test]
fn a_count_still_tried_when_the_listener_stops_is_said_as_given_up() {
    let place = Place::new("day-tried-off");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    run(&place, NEXT_NIGHT, false);
    let (counted, hour) = run(&place, AT + 3 * DAY, true);
    assert!(counted.is_empty(), "{counted:?}");
    assert!(
        hour_says(&hour, "gave up on 2026-11-16's count"),
        "{hour:?}"
    );
}

#[test]
fn days_the_listener_did_not_hear_whole_are_not_counted() {
    let place = Place::new("day-off");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    assert!(
        counts(&place, &[AT + 3 * DAY]).is_empty(),
        "three days off: none of them heard whole"
    );
    let counted = counts(&place, &[AT + 3 * DAY + 11 * HOUR]);
    assert_eq!(counted.len(), 1, "{counted:?}");
    assert!(
        counted[0].contains("2026-11-19") && !counted[0].contains(", from "),
        "the day it came back is heard whole, a day back: {counted:?}"
    );
}

#[test]
fn a_day_heard_in_part_says_from_when() {
    let place = Place::new("day-part");
    place.routes();
    // 00:10 on the 16th where he is, then nothing until 02:30 on the 17th:
    // what he said before 02:30 on the 16th was a day back too far to read.
    counts(&place, &[AT - 14 * HOUR - 50 * 60_000]);
    a_day(&place);
    let counted = counts(&place, &[AT + 11 * HOUR + 1_800_000]);
    assert_eq!(counted.len(), 1, "{counted:?}");
    assert!(
        counted[0].contains("2026-11-16, from 02:30"),
        "{counted:?}"
    );
}

#[test]
fn a_day_counted_is_never_counted_again() {
    let place = Place::new("day-back");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    assert_eq!(counts(&place, &[NEXT_NIGHT]).len(), 1);
    // Flown eight hours west, it's the 16th again where he is.
    let mut ears = place.ears();
    ears.at = NEXT_NIGHT + HOUR;
    ears.offset_s = -13 * 3600;
    let (counted, _) = run_with(&ears, CELLS, true);
    assert!(counted.is_empty(), "{counted:?}");
    assert!(
        counts(&place, &[AT - 2 * DAY, AT - 2 * DAY + 11 * HOUR]).is_empty(),
        "a clock set back counts no day before the last one counted"
    );
    let counted = counts(&place, &[NEXT_NIGHT + DAY]);
    assert_eq!(counted.len(), 1, "{counted:?}");
    assert!(counted[0].contains("2026-11-17"), "{counted:?}");
}

#[test]
fn a_day_file_that_cant_be_read_is_a_trouble() {
    let place = Place::new("day-garbled");
    place.routes();
    fs::write(place.root.join("state/day.json"), "{").expect("written");
    let (_, hour) = run(&place, AT, true);
    assert!(hour_says(&hour, "day.json"), "{hour:?}");
    let (_, hour) = run(&place, AT + 300_000, true);
    assert!(!hour_says(&hour, "day.json"), "begun again: {hour:?}");
}

#[test]
fn a_run_that_cant_tell_his_time_zone_counts_no_day() {
    let place = Place::new("day-zoneless");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    let mut ears = place.ears();
    ears.at = NEXT_NIGHT;
    ears.zone_known = false;
    let (counted, hour) = run_with(&ears, CELLS, true);
    assert!(counted.is_empty(), "{counted:?}");
    assert!(hour_says(&hour, "time zone"), "{hour:?}");
    assert_eq!(counts(&place, &[NEXT_NIGHT + 300_000]).len(), 1);
}

#[test]
fn the_hour_says_his_status_asks_of_his_own_day() {
    let place = Place::new("asked");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.status_asks, 1, "{:?}", place.heard());
    // 23:30 on the 16th where he is, but the 17th in UTC.
    place.log(
        "night",
        "s-night",
        Path::new("/home/j/src"),
        &[user("n1", "2026-11-17T04:30:00.000Z", "status update")],
    );
    let mut ears = place.ears();
    ears.at = NEXT_NIGHT;
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.status_asks, 0, "his new day starts at none");
}
