//! The cases for the Room Light's row, from the machine daemon's sensor
//! record as modules/machine's light.rs writes it; Displays.qml's row for
//! the same (its lightRows, with Power.js's roomLevel) moved here with it
//! on 2026-11-14.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::super::tests::told;
use super::*;

/// The room's sensor hearing `lux`, as light.rs says it.
fn sensing(lux: &Value) -> Value {
    json!({"key": "light:iio:device0", "kind": "light", "device": "iio:device0",
        "name": "als", "raw": 1234, "scale": 0.1, "offset": 0, "lux": lux})
}

/// The Room Light inside eDP-1 from `records`, read as `read` says, its
/// part named by its id.
fn row_of(records: &[Value], read: Reading) -> Option<Value> {
    let numbers = Numbers::default();
    let named = |id: &str| id.to_owned();
    let inside = Inside {
        name: "eDP-1",
        screen: "row:displays:display:eDP-1",
        named: &named,
        numbers: &numbers,
        now: 0,
    };
    row(&Light::of(records), &inside, read, &told())
}

#[test]
fn the_room_light_says_what_the_sensor_hears_and_copies_it() {
    let room = row_of(&[sensing(&json!(123.4))], Reading::Read).expect("no Room Light");
    assert_eq!(
        (
            &room["key"],
            &room["thing"],
            &room["parent"],
            &room["depth"]
        ),
        (
            &json!("row:displays:room:eDP-1"),
            &json!("displays:eDP-1:room"),
            &json!("row:displays:display:eDP-1"),
            &json!(1)
        ),
        "the Room Light isn't inside the screen: {room}"
    );
    assert_eq!(
        room["name"], "room",
        "the Room Light isn't named as its part"
    );
    assert_eq!(room["side"], json!([{"said": "123 lx"}]), "{room}");
    assert_eq!(
        room["acts"],
        json!([{"key": "verb:copy", "copy": "123 lx"}]),
        "Return doesn't copy what it says"
    );
    assert_eq!(
        room["note"], "as the screen's sensor hears it, from dark to daylight",
        "the note doesn't say what the bar runs between"
    );
    let bar = room["graph"]["values"][0].as_f64().unwrap_or(-1.0);
    assert!(
        (bar - 0.5237).abs() < 0.0005,
        "123 lx isn't half way from dark to daylight: {bar}"
    );
}

#[test]
fn the_bar_runs_from_the_dark_to_daylight_as_the_eye_sees_it() {
    assert!(level(0.0).abs() < f64::EPSILON, "the dark fills the bar");
    assert!(
        level(-3.0).abs() < f64::EPSILON,
        "less than dark fills the bar"
    );
    assert!(
        (level(10_000.0) - 1.0).abs() < f64::EPSILON,
        "daylight doesn't fill it"
    );
    assert!(
        (level(80_000.0) - 1.0).abs() < f64::EPSILON,
        "sunlight runs past it"
    );
    assert!(
        level(5.0) > 0.15,
        "a dim room barely moves it: {}",
        level(5.0)
    );
}

#[test]
fn while_its_read_it_says_so_and_copies_nothing() {
    let waiting = row_of(&[sensing(&json!(123.4))], Reading::Waiting).expect("no Room Light");
    assert_eq!(waiting["side"], json!([{"said": "\u{2026}"}]), "{waiting}");
    assert_eq!(waiting["acts"], json!([]), "Return copies a light not read");
    assert_eq!(waiting["graph"]["values"], json!([0.0]), "{waiting}");
    let unread = row_of(&[sensing(&json!(123.4))], Reading::Unread).expect("no Room Light");
    assert_eq!(
        unread["side"],
        json!([{"said": told().said("unread")}]),
        "{unread}"
    );
    assert_eq!(unread["acts"], json!([]), "Return copies a light not read");
    let silent = row_of(&[sensing(&Value::Null)], Reading::Read).expect("no Room Light");
    assert_eq!(
        silent["acts"],
        json!([]),
        "a sensor saying no lux is copied: {silent}"
    );
}

#[test]
fn no_sensor_serves_no_room_light() {
    assert_eq!(
        row_of(&[], Reading::Read),
        None,
        "a Room Light is served where nothing hears the room"
    );
}
