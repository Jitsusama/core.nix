//! The day's file, as a review reads it.

use serde_json::Value;

use super::*;
use crate::probe::Reading;

/// 2026-11-12 at 23:30 in a zone five hours behind UTC, which is already
/// the 13th in UTC.
const LATE: i64 = 1_794_544_200;

fn zone() -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(-5))
}

fn looked(history: &mut History, at: i64) {
    let reading = Reading {
        home: true,
        gateway_ms: Some(4.0),
        internet_ms: Some(20.0),
        dns_ms: Some(30.0),
        reachable: true,
    };
    history
        .record_probe(&reading, at)
        .expect("a look is recorded");
}

#[test]
fn the_day_is_written_by_its_own_date_in_the_machines_zone() {
    let folder = std::env::temp_dir().join(format!("eero-day-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    let mut history = History::in_memory();
    looked(&mut history, LATE - 23 * 60 * 60);
    looked(&mut history, LATE);
    looked(&mut history, LATE + 60 * 60);

    write(&history, LATE, &zone(), &folder).expect("the day is written");

    let text = fs::read_to_string(folder.join("2026-11-12.json")).expect("the day's file");
    let day: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(day["date"], "2026-11-12");
    assert_eq!(day["probes"], 2);
    assert_eq!(day["internet_p95_ms"], 20.0);
    assert_eq!(day["outage_minutes"], 0.0);
    let names: Vec<String> = fs::read_dir(&folder)
        .expect("the folder")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["2026-11-12.json"], "nothing half-written is left");
    fs::remove_dir_all(&folder).expect("tidied");
}

#[test]
fn a_day_older_than_the_history_keeps_is_dropped() {
    let folder = std::env::temp_dir().join(format!("eero-days-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder");
    for kept in ["2026-10-01.json", "2026-10-15.json", "notes.txt"] {
        fs::write(folder.join(kept), "{}").expect("a file");
    }

    write(&History::in_memory(), LATE, &zone(), &folder).expect("the day is written");

    let mut names: Vec<String> = fs::read_dir(&folder)
        .expect("the folder")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["2026-10-15.json", "2026-11-12.json", "notes.txt"]);
    fs::remove_dir_all(&folder).expect("tidied");
}
