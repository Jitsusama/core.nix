//! The cases for the Brightness's rows, from the machine daemon's
//! backlight record as modules/machine's light.rs writes it; Displays.qml's
//! rows for the same (its lightRows) moved here with them on 2026-11-14.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

// The panel's backlight is at 57,600 of its 96,000 steps, 60%.
use super::super::tests::{backlight, told};
use super::*;

/// The Brightness's rows inside eDP-1 from `records`, read as `read`
/// says, in the room or not, its part named by its id.
fn rows_of(records: &[Value], in_the_room: bool, read: Reading) -> Vec<Value> {
    let numbers = Numbers::default();
    let named = |id: &str| id.to_owned();
    let inside = Inside {
        name: "eDP-1",
        screen: "row:displays:display:eDP-1",
        named: &named,
        numbers: &numbers,
        now: 0,
    };
    rows(&Light::of(records), &inside, in_the_room, read, &told())
}

/// Each level's name and what it says beside it.
fn levels(rows: &[Value]) -> Vec<(&str, &Value)> {
    rows.iter()
        .skip(1)
        .map(|row| (row["name"].as_str().unwrap_or(""), &row["side"]))
        .collect()
}

#[test]
fn the_brightness_says_its_share_and_steps_and_offers_its_levels() {
    let made = rows_of(&[backlight()], true, Reading::Read);
    let Some(brightness) = made.first() else {
        panic!("no Brightness is served");
    };
    assert_eq!(
        (
            &brightness["key"],
            &brightness["thing"],
            &brightness["kind"],
            &brightness["parent"],
            &brightness["depth"]
        ),
        (
            &json!("row:displays:setting:eDP-1:brightness"),
            &json!("displays:eDP-1:brightness"),
            &json!("setting"),
            &json!("row:displays:display:eDP-1"),
            &json!(1)
        ),
        "the Brightness isn't a setting inside the screen: {brightness}"
    );
    assert_eq!(
        brightness["name"], "brightness",
        "it isn't named as its part"
    );
    assert_eq!(brightness["side"], json!([{"said": "60%"}]), "{brightness}");
    assert_eq!(
        brightness["note"], "57,600 of 96,000 steps; it follows the room, and the keys set it",
        "the note doesn't say its steps and what sets it"
    );
    let bar = brightness["graph"]["values"][0].as_f64().unwrap_or(-1.0);
    assert!((bar - 0.6).abs() < 1e-9, "its bar isn't at 60%: {bar}");
    assert_eq!(
        brightness["acts"],
        json!([{"key": "verb:more", "step": "rows"}]),
        "Return doesn't step into its levels"
    );
    let chosen = json!([{"said": told().chosen}]);
    let none = json!([]);
    let expected: Vec<(String, &Value)> = (1..=10)
        .rev()
        .map(|tenth| {
            let side = if tenth == 6 { &chosen } else { &none };
            (format!("{}%", tenth * 10), side)
        })
        .collect();
    let expected: Vec<(&str, &Value)> = expected.iter().map(|(n, s)| (n.as_str(), *s)).collect();
    assert_eq!(
        levels(&made),
        expected,
        "the levels aren't each tenth, brightest first, 60% chosen"
    );
}

#[test]
fn a_levels_return_asks_the_machine_daemon_to_set_it_as_the_line_types_it() {
    let made = rows_of(&[backlight()], true, Reading::Read);
    let Some(forty) = made.iter().find(|row| row["name"] == "40%") else {
        panic!("no 40% is offered: {made:?}");
    };
    assert_eq!(
        (
            &forty["key"],
            &forty["thing"],
            &forty["kind"],
            &forty["depth"]
        ),
        (
            &json!("row:displays:choice:eDP-1:brightness-40"),
            &json!("verb:display-brightness-40"),
            &json!("choice"),
            &json!(2)
        ),
        "40% isn't a choice keyed as the shell keyed it: {forty}"
    );
    assert_eq!(
        forty["parent"], "row:displays:setting:eDP-1:brightness",
        "40% isn't inside the Brightness"
    );
    assert_eq!(
        forty["acts"],
        json!([{"key": LEVEL,
            "ask": {"ask": "light", "do": "brightness", "share": 40}}]),
        "Return doesn't ask the light to be set to 40%"
    );
    assert_eq!(
        forty["held"],
        json!({"keywords": "brightness", "fill": {"n": 40}}),
        "it isn't found by its setting's name, or :set brightness=40 isn't filled in"
    );
}

#[test]
fn while_its_read_it_says_only_its_steps_and_no_level_is_chosen() {
    let waiting = rows_of(&[backlight()], true, Reading::Waiting);
    let Some(brightness) = waiting.first() else {
        panic!("no Brightness is served while it's read");
    };
    assert_eq!(brightness["side"], json!([]), "{brightness}");
    assert_eq!(
        brightness["note"], "96,000 steps \u{b7} follows the room",
        "the note says a level not read"
    );
    assert_eq!(brightness["graph"]["values"], json!([0.0]), "{brightness}");
    assert!(
        levels(&waiting).iter().all(|(_, side)| *side == &json!([])),
        "a level is chosen while it's read: {waiting:?}"
    );
    let unread = rows_of(&[backlight()], true, Reading::Unread);
    assert_eq!(
        unread[0]["side"],
        json!([{"said": told().said("unread")}]),
        "after its first second it doesn't say it isn't read"
    );
}

#[test]
fn a_monitor_following_the_panel_says_only_what_sets_it() {
    let made = rows_of(&[backlight()], false, Reading::Read);
    assert_eq!(
        made[0]["note"], "every screen follows it, and the keys set it",
        "out of the room, it says the panel's steps"
    );
    assert_eq!(made.len(), 11, "out of the room, its levels aren't offered");
}

#[test]
fn no_backlight_serves_no_brightness() {
    assert!(
        rows_of(&[], true, Reading::Read).is_empty(),
        "a Brightness is served where nothing lights the screen"
    );
}
