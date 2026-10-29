use disk_guard::holders::{Kind, Owner, Place};
use disk_guard::ledger::{Entry, Growth, grown, load, record};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const GIB: u64 = 1 << 30;
const HOUR: f64 = 3600.0;
const DAY: f64 = 24.0 * HOUR;

fn sized(sizes: &[(&str, u64)]) -> Vec<(Place, u64)> {
    sizes
        .iter()
        .map(|(path, bytes)| {
            let place = Place {
                path: PathBuf::from(path),
                owner: Owner {
                    kind: Kind::Other,
                    name: (*path).to_owned(),
                },
            };
            (place, *bytes)
        })
        .collect()
}

fn entry(at: f64, sizes: &[(&str, u64)]) -> Entry {
    Entry {
        at,
        sizes: sizes
            .iter()
            .map(|(path, bytes)| (PathBuf::from(path), *bytes))
            .collect(),
    }
}

#[test]
fn growth_is_said_against_the_earliest_measurement_within_a_day_at_least_an_hour_old() {
    let now = 10.0 * DAY;
    let entries = [
        entry(now - 30.0 * HOUR, &[("/a", GIB)]),
        entry(now - 20.0 * HOUR, &[("/a", 2 * GIB), ("/b", 5 * GIB)]),
        entry(now - 0.5 * HOUR, &[("/a", 9 * GIB), ("/b", 5 * GIB)]),
    ];
    let measured = sized(&[("/a", 10 * GIB), ("/b", 4 * GIB), ("/c", 3 * GIB)]);
    let mut expected = BTreeMap::new();
    expected.insert(
        PathBuf::from("/a"),
        Growth {
            bytes: 8 * GIB,
            over: 20.0 * HOUR,
        },
    );
    assert_eq!(
        grown(&entries, now, &measured),
        expected,
        "one that shrank grew nothing, and one not measured then has nothing to grow from"
    );
}

#[test]
fn nothing_has_grown_until_a_measurement_is_an_hour_old() {
    let now = DAY;
    let entries = [entry(now - 0.5 * HOUR, &[("/a", GIB)])];
    let measured = sized(&[("/a", 10 * GIB)]);
    assert_eq!(grown(&entries, now, &measured), BTreeMap::new());
}

#[test]
fn the_ledger_keeps_two_days_of_measurements() {
    let dir = tempfile::tempdir().unwrap();
    let measured = sized(&[("/a", GIB)]);
    record(dir.path(), 0.0, &measured).unwrap();
    record(dir.path(), 1.5 * DAY, &measured).unwrap();
    let grew = record(dir.path(), 3.0 * DAY, &sized(&[("/a", 3 * GIB)])).unwrap();
    let at: Vec<f64> = load(dir.path()).iter().map(|entry| entry.at).collect();
    assert_eq!(at, [1.5 * DAY, 3.0 * DAY]);
    assert_eq!(
        grew,
        BTreeMap::new(),
        "the measurement a day and a half old is past a day, so it says no growth"
    );
}

#[test]
fn a_ledger_that_will_not_read_starts_afresh() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(disk_guard::ledger::FILE), "not json").unwrap();
    record(dir.path(), DAY, &sized(&[("/a", GIB)])).unwrap();
    let at: Vec<f64> = load(dir.path()).iter().map(|entry| entry.at).collect();
    assert_eq!(at, [DAY]);
}
