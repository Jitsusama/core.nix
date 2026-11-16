//! The cases for the place's own : line, :set screens, brightness,
//! nightlight and warmth, served on Screens' summary from its rows, as
//! Displays.qml worked them out of those rows until 15 November 2026.

use super::*;

/// The summary's line where the panel holds a backlight at half, read,
/// and the Warmth says its Night Light off and Follow the Room on.
fn served(outputs: &BTreeMap<String, Value>) -> Value {
    let light = Light::of(&[
        json!({"key": "backlight:intel_backlight", "kind": "backlight",
            "device": "intel_backlight", "brightness": 48, "max_brightness": 96,
            "connector": "eDP-1"}),
        crate::light::tests::warmed(),
    ]);
    let made = every(outputs, &BTreeMap::new(), None, &light);
    summary(outputs, &made, &words())["line"].clone()
}

#[test]
fn the_place_line_says_each_option_as_its_row_stands_and_what_each_way_asks() {
    let line = served(&desk());
    assert_eq!(
        line["screens"],
        json!({"kind": "switch", "now": true,
            "ways": {"off": {"row": "row:displays:sleep", "act": "displays:sleep"}}}),
        ":set noscreens isn't Turn Every Screen Off's act"
    );
    assert_eq!(
        line["brightness"],
        json!({"kind": "number", "least": 0, "most": 100, "now": 50.0,
            "ask": {"ask": "light", "do": "brightness"}}),
        ":set brightness isn't the share read, asked as a level asks"
    );
    assert_eq!(
        line["nightlight"],
        json!({"kind": "switch", "now": false,
            "ways": {"on": {"ask": {"ask": "light", "do": "night-light", "on": true}}}}),
        ":set nightlight doesn't turn the Night Light on as its Return does, and only on"
    );
    assert_eq!(
        line["warmth"],
        json!({"kind": "switch", "now": true, "ways": {"off": {"ask":
            {"ask": "light", "do": "follow-the-room", "follows": false}}}}),
        ":set nowarmth doesn't stop following the room as its Return does, and only off"
    );
}

#[test]
fn the_place_line_offers_only_what_its_rows_serve() {
    let mut shut = desk();
    shut.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    let docked = served(&shut);
    assert!(
        docked.get("brightness").is_none() && docked.get("warmth").is_none(),
        "the lid shut, the line offers the panel's light: {docked}"
    );
    assert!(
        docked.get("nightlight").is_some(),
        "the lid shut, the Night Light isn't offered"
    );
    let dark: BTreeMap<String, Value> = desk()
        .into_iter()
        .map(|(name, output)| {
            let mut off = output;
            off["logical"] = Value::Null;
            (name, off)
        })
        .collect();
    assert_eq!(
        served(&dark)["screens"]["ways"],
        json!({}),
        "with none lit, :set noscreens still asks Turn Every Screen Off"
    );
}
