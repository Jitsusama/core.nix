//! The cases for which screen holds the Brightness: the one its row
//! says, read or not, the one whose drawer says the light while it's
//! looked at, and the one Screens' summary says with what its light says.

use super::*;

/// The screens whose drawer leads with the Brightness, by connector.
fn holding(rows: &[Value]) -> Vec<&str> {
    rows.iter()
        .filter(|row| row["drawer"]["facts"][0]["name"] == "Brightness")
        .filter_map(|row| row["held"]["output"].as_str())
        .collect()
}

#[test]
fn only_the_screen_holding_the_brightness_says_the_light_and_only_while_looked_at() {
    let light = crate::light::Light::of(&[json!({"key": "backlight:intel_backlight",
        "kind": "backlight", "device": "intel_backlight", "brightness": 48,
        "max_brightness": 96, "connector": "eDP-1"})]);
    let seen = Seen {
        light: &light,
        reading: crate::light::Reading::Read,
        now: 0,
        looked: true,
    };
    let looked = |outputs: &BTreeMap<String, Value>, desk: &BTreeMap<String, Value>| {
        super::super::rows(outputs, desk, None, true, &seen, &words())
    };
    let none = BTreeMap::new();
    assert_eq!(
        holding(&looked(&desk(), &none)),
        vec!["eDP-1"],
        "the screen the backlight lights doesn't hold it alone"
    );
    let mut away = desk();
    away.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    away.insert("DP-3".to_owned(), output("LG", "27GL850", &lit(1.0)));
    let follows = BTreeMap::from([(
        "DP-3".to_owned(),
        json!([{"name": "Brightness", "value": "95%"}]),
    )]);
    assert_eq!(
        holding(&looked(&away, &follows)),
        vec!["DP-3"],
        "with the built-in screen off, the monitor following the panel doesn't hold it"
    );
    assert!(
        holding(&looked(&away, &none)).is_empty(),
        "a monitor that doesn't follow the panel holds it"
    );
    assert!(
        holding(&rows(&desk(), None, &words())).is_empty(),
        "a drawer says the light while no screen is looked at"
    );
}

/// The screens whose rows say they hold the Brightness, by connector.
fn lighting(rows: &[Value]) -> Vec<&str> {
    rows.iter()
        .filter(|row| row["held"]["lights"] == true)
        .filter_map(|row| row["held"]["output"].as_str())
        .collect()
}

#[test]
fn a_screens_row_says_it_holds_the_brightness_read_or_not() {
    let light = Light::of(&[json!({"key": "backlight:intel_backlight",
        "kind": "backlight", "device": "intel_backlight", "brightness": 48,
        "max_brightness": 96, "connector": "eDP-1"})]);
    let none = BTreeMap::new();
    assert_eq!(
        lighting(&every(&desk(), &none, None, &light)),
        vec!["eDP-1"],
        "the screen the backlight lights doesn't say it holds it alone"
    );
    let mut away = desk();
    away.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    away.insert("DP-3".to_owned(), output("LG", "27GL850", &lit(1.0)));
    let follows = BTreeMap::from([(
        "DP-3".to_owned(),
        json!([{"name": "Brightness", "value": "95%"}]),
    )]);
    assert_eq!(
        lighting(&every(&away, &follows, None, &light)),
        vec!["DP-3"],
        "with the built-in screen off, the monitor following the panel doesn't say it"
    );
    assert!(
        lighting(&every(&away, &none, None, &light)).is_empty(),
        "a monitor that doesn't follow the panel says it holds it"
    );
    assert_eq!(
        lighting(&rows(&desk(), None, &words())),
        vec!["eDP-1"],
        "the built-in screen doesn't say it holds it before the light is read"
    );
}

/// What the row of `key` says, its side and its note, as the summary's
/// light says each.
fn says(rows: &[Value], key: &str) -> Value {
    let row = rows
        .iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no row {key}"));
    let side: Vec<&str> = row["side"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|each| each["said"].as_str())
        .collect();
    json!([side.join(" "), row["note"].as_str().unwrap_or_default()])
}

#[test]
fn the_summary_says_the_screen_holding_the_light_and_what_its_light_rows_say() {
    let light = Light::of(&[
        json!({"key": "backlight:intel_backlight", "kind": "backlight",
            "device": "intel_backlight", "brightness": 48, "max_brightness": 96,
            "connector": "eDP-1"}),
        json!({"key": "light:iio:device0", "kind": "light", "device": "iio:device0",
            "name": "als", "raw": 1234, "scale": 0.1, "offset": 0, "lux": 123.4}),
    ]);
    let none = BTreeMap::new();
    let made = every(&desk(), &none, None, &light);
    let held = &summary(&desk(), &made, &words())["light"];
    assert_eq!(
        held["row"], "row:displays:display:eDP-1",
        "the summary doesn't say the panel's row holds the light: {held}"
    );
    assert_eq!(
        held["said"],
        json!([
            says(&made, "row:displays:setting:eDP-1:brightness"),
            says(&made, "row:displays:room:eDP-1")
        ]),
        "the light says other than its Brightness and its Room Light"
    );
    assert_eq!(
        held["said"][0][0], "50%",
        "the Brightness read isn't said: {held}"
    );
    let mut away = desk();
    away.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    let shut = every(&away, &none, None, &light);
    assert_eq!(
        summary(&away, &shut, &words())["light"],
        Value::Null,
        "with the lid shut and no monitor following the panel, a screen holds the light"
    );
}
