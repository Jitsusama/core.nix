//! The keeper's daily count, and the status asks of the day: what the
//! keeper hears in place of each pain heard too late for its owner.

use super::*;

const HOUR: i64 = 3_600_000;

/// What the keeper was told of whole days, over runs at each of `ats`.
fn counts(place: &Place, ats: &[i64]) -> Vec<String> {
    let mut counted = Vec::new();
    for at in ats {
        let mut ears = place.ears();
        ears.at = *at;
        hear(&ears, CELLS, &mut |to, text| {
            if to == "ORCH01" && text.starts_with("Joel's day") {
                counted.push(text.to_string());
            }
            true
        })
        .expect("a run");
    }
    counted
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
    // 02:00 and 02:05 on the 17th where he is, five hours behind UTC.
    let counted = counts(&place, &[AT + 11 * HOUR, AT + 11 * HOUR + 300_000]);
    assert_eq!(counted.len(), 1, "once a day: {counted:?}");
    let day = &counted[0];
    for said in [
        "2026-11-16",
        "4 pains heard",
        "1 asking how something stands",
        "3 sent to their owners",
        "1 the ledgers already carried",
        "0 turns stalled him",
    ] {
        assert!(day.contains(said), "{said:?} in {day}");
    }
}

#[test]
fn a_day_whose_count_did_not_go_is_counted_on_the_next_run() {
    let place = Place::new("day-again");
    place.routes();
    a_day(&place);
    counts(&place, &[AT]);
    let mut ears = place.ears();
    ears.at = AT + 11 * HOUR;
    hear(&ears, CELLS, &mut |to, text| {
        !(to == "ORCH01" && text.starts_with("Joel's day"))
    })
    .expect("a run");
    let counted = counts(&place, &[AT + 11 * HOUR + 300_000]);
    assert_eq!(counted.len(), 1, "{counted:?}");
}

#[test]
fn the_hour_says_his_status_asks_of_the_day() {
    let place = Place::new("asked");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.status_asks, 1, "{:?}", place.heard());
    let mut ears = place.ears();
    ears.at = AT + 11 * HOUR;
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.status_asks, 0, "a new day starts at none");
}
