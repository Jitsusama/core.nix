//! The cases for the Warmth's rows, from the backlight program's record
//! as modules/machine's steered.rs writes it; Displays.qml's cases for the
//! same (tests/answers/clock.js's Warmth.note and Warmth.figures) moved
//! here with it on 2026-11-14.

use serde_json::{Value, json};

use super::super::tests::{told, warmed};
use super::*;

/// The Warmth's rows inside eDP-1 at the moment `now`, from the warmth
/// record `warmth` changed by `changes`, each part named by its id.
fn rows_at(changes: &Value, now: u64) -> Vec<Value> {
    let mut record = warmed();
    for (field, value) in changes.as_object().into_iter().flatten() {
        record[field] = value.clone();
    }
    let numbers = Numbers::default();
    let named = |id: &str| id.to_owned();
    let inside = Inside {
        name: "eDP-1",
        screen: "row:displays:display:eDP-1",
        named: &named,
        numbers: &numbers,
        now,
    };
    rows(&Light::of(&[record]), &inside, &told())
}

fn keys(rows: &[Value]) -> Vec<&str> {
    rows.iter().filter_map(|row| row["key"].as_str()).collect()
}

/// The row of `key`'s among `rows`.
fn row<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no row {key} among {:?}", keys(rows)))
}

const WARMTH: &str = "row:displays:warmth:eDP-1";

#[test]
fn by_day_the_warmth_steps_into_its_switches_then_its_figures() {
    let made = rows_at(&json!({}), 500);
    assert_eq!(
        keys(&made),
        [
            WARMTH,
            "row:displays:switch:eDP-1:night-light",
            "row:displays:switch:eDP-1:follow-room",
            "row:displays:fact:eDP-1:warmth-room",
            "row:displays:fact:eDP-1:warmth-sunset",
            "row:displays:fact:eDP-1:warmth-sunrise",
            "row:displays:fact:eDP-1:warmth-night",
        ],
        "the Warmth's rows aren't served in the order a step into it lists them"
    );
    let warmth = row(&made, WARMTH);
    assert_eq!(
        (
            &warmth["parent"],
            &warmth["depth"],
            &warmth["thing"],
            &warmth["kind"]
        ),
        (
            &json!("row:displays:display:eDP-1"),
            &json!(1),
            &json!("displays:eDP-1:warmth"),
            &json!("warmth")
        ),
        "the Warmth isn't inside its screen as the shell made it"
    );
    assert_eq!(
        (&warmth["side"], &warmth["note"]),
        (&json!([{"said": "6,500 K"}]), &json!("following the room")),
        "the Warmth doesn't say its kelvin and what holds it"
    );
    assert_eq!(
        warmth["acts"],
        json!([{"key": "verb:more", "step": "rows"}]),
        "the Warmth's Return doesn't step into its rows"
    );
    assert!(
        made[1..]
            .iter()
            .all(|inside| inside["parent"] == WARMTH && inside["depth"] == 2),
        "what a step into the Warmth holds isn't inside it"
    );
}

#[test]
fn each_switch_says_how_it_stands_and_asks_the_other_way() {
    let made = rows_at(&json!({}), 500);
    let night = row(&made, "row:displays:switch:eDP-1:night-light");
    assert_eq!(
        (
            &night["name"],
            &night["glyph"],
            &night["side"],
            &night["held"]
        ),
        (
            &json!("night-light"),
            &json!("nightLight"),
            &json!([{"said": "off"}]),
            &json!({"on": false})
        ),
        "the Night Light doesn't stand off"
    );
    assert_eq!(
        night["acts"],
        json!([{"key": "displays:night-light",
            "ask": {"ask": "light", "do": "night-light", "on": true}}]),
        "the Night Light's Return doesn't ask it on"
    );
    let follows = row(&made, "row:displays:switch:eDP-1:follow-room");
    assert_eq!(
        (&follows["side"], &follows["thing"]),
        (
            &json!([{"said": "on"}]),
            &json!("displays:eDP-1:follow-room")
        ),
        "Follow the Room doesn't stand on"
    );
    assert_eq!(
        follows["acts"],
        json!([{"key": "displays:follow-room",
            "ask": {"ask": "light", "do": "follow-the-room", "follows": false}}]),
        "Follow the Room's Return doesn't ask it off"
    );
}

#[test]
fn by_night_the_night_light_holds_until_sunrise_which_leads_the_figures() {
    let made = rows_at(&json!({"night_light": true, "kelvin": 3400}), 1500);
    let warmth = row(&made, WARMTH);
    assert_eq!(
        warmth["note"],
        json!({"at": 2000, "as": "moment", "said": "night light until {time}"}),
        "by night the note doesn't say until sunrise"
    );
    assert_eq!(
        keys(&made)[3..5],
        [
            "row:displays:fact:eDP-1:warmth-room",
            "row:displays:fact:eDP-1:warmth-sunrise"
        ],
        "by night the sunrise doesn't come first"
    );
    let sunrise = &made[4];
    assert_eq!(
        (&sunrise["side"], &sunrise["acts"]),
        (
            &json!([{"at": 2000, "as": "moment"}]),
            &json!([{"key": "verb:copy", "copy": {"at": 2000, "as": "moment"}}])
        ),
        "a time isn't served for the shell to say and copy by its clock"
    );
    assert_eq!(
        warmth["drawer"],
        json!({
            "eyebrow": "via wl-gammarelay-rs \u{b7} built-in screen",
            "eyebrowItems": [{"said": "via wl-gammarelay-rs", "keep": 2},
                {"said": "built-in screen", "keep": 1}],
            "eyebrowNames": true,
            "title": "warmth",
            "facts": [{"name": "warmth:room", "said": "5,100 K"},
                {"name": "warmth:sunrise", "at": 2000, "as": "moment"},
                {"name": "warmth:sunset", "at": 1000, "as": "moment"},
                {"name": "warmth:night", "said": "3,400 K"}],
        }),
        "the Warmth's plate doesn't show its figures under where it's set"
    );
}

#[test]
fn the_note_says_what_holds_the_warmth_at_each_hour() {
    let note = |changes: Value, now: u64| row(&rows_at(&changes, now), WARMTH)["note"].clone();
    assert_eq!(
        note(json!({"night_light": true}), 500),
        json!({"at": 1000, "as": "moment", "said": "night light from {time}"}),
        "by day the night light doesn't say it's from sunset"
    );
    assert_eq!(
        note(json!({"night_light": true, "sunset_ms": null}), 1500),
        json!("night light waits for where you are"),
        "a night light with no sunset doesn't wait for a place"
    );
    assert_eq!(
        note(json!({"follows_the_room": false}), 500),
        json!("the screen's own white"),
        "a warmth not following the room isn't the screen's own"
    );
}

#[test]
fn where_no_sensor_sees_colour_the_room_neither_switches_nor_figures() {
    let made = rows_at(&json!({"sees_colour": false}), 500);
    assert_eq!(
        keys(&made)[1..],
        [
            "row:displays:switch:eDP-1:night-light",
            "row:displays:fact:eDP-1:warmth-sunset",
            "row:displays:fact:eDP-1:warmth-sunrise",
            "row:displays:fact:eDP-1:warmth-night",
        ],
        "Follow the Room or the room's white is served where no sensor sees colour"
    );
    assert_eq!(
        row(&made, WARMTH)["note"],
        json!("the screen's own white"),
        "the room is said to hold a warmth no sensor sees"
    );
}

#[test]
fn nothing_warming_the_screen_serves_no_warmth() {
    let numbers = Numbers::default();
    let named = |id: &str| id.to_owned();
    let inside = Inside {
        name: "eDP-1",
        screen: "row:displays:display:eDP-1",
        named: &named,
        numbers: &numbers,
        now: 0,
    };
    assert!(
        rows(&Light::default(), &inside, &told()).is_empty(),
        "a Warmth is served with no record of it"
    );
}
