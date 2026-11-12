//! A day's figures, from looks recorded as the daemon records them.

use super::*;
use crate::probe::Reading;

/// A moment in the day the tests read, 2026-11-12 in UTC.
const FROM: i64 = 1_794_441_600;
const TO: i64 = FROM + 24 * 60 * 60;

fn looked(history: &mut History, at: i64, home: bool, internet_ms: Option<f64>) {
    let reading = Reading {
        home,
        gateway_ms: home.then_some(internet_ms.map_or(2.0, |ms| ms / 4.0)),
        internet_ms,
        dns_ms: internet_ms.map(|ms| ms * 2.0),
        reachable: internet_ms.is_some(),
    };
    history
        .record_probe(&reading, at)
        .expect("a look is recorded");
}

#[test]
fn a_day_says_each_times_95th_percentile_of_the_looks_from_home() {
    let mut history = History::in_memory();
    for n in 1_u8..=20 {
        looked(
            &mut history,
            FROM + i64::from(n) * 300,
            true,
            Some(f64::from(n) * 10.0),
        );
    }
    looked(&mut history, FROM + 30_000, false, Some(900.0));
    looked(&mut history, FROM - 60, true, Some(900.0));
    looked(&mut history, TO, true, Some(900.0));

    let day = history.day("2026-11-12", FROM, TO).expect("a day");

    assert_eq!(
        day,
        Day {
            date: "2026-11-12".to_owned(),
            probes: 20,
            away_probes: 1,
            gateway_p95_ms: Some(47.5),
            internet_p95_ms: Some(190.0),
            dns_p95_ms: Some(380.0),
            unreachable: 0,
            outage_minutes: 0.0,
        }
    );
}

#[test]
fn a_day_counts_the_looks_that_found_the_internet_out_and_the_minutes_out() {
    let mut history = History::in_memory();
    looked(&mut history, FROM - 120, true, None);
    looked(&mut history, FROM + 60, true, Some(20.0));
    for at in [FROM + 600, FROM + 660, FROM + 720] {
        looked(&mut history, at, true, None);
    }
    looked(&mut history, FROM + 870, true, Some(20.0));

    let day = history.day("2026-11-12", FROM, TO).expect("a day");

    assert_eq!(day.unreachable, 3);
    assert_eq!(day.outage_minutes, 1.0 + 4.5);
    assert_eq!(day.probes, 5);
}

#[test]
fn a_day_with_no_looks_from_home_has_no_times() {
    let mut history = History::in_memory();
    looked(&mut history, FROM + 60, false, Some(20.0));

    let day = history.day("2026-11-12", FROM, TO).expect("a day");

    assert_eq!((day.probes, day.away_probes), (0, 1));
    assert_eq!(
        (day.gateway_p95_ms, day.internet_p95_ms, day.dns_p95_ms),
        (None, None, None)
    );
}
