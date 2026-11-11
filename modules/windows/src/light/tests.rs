//! The cases for the light a screen's drawer says, from the machine
//! daemon's records as modules/machine's light.rs and steered.rs write
//! them; Displays.qml's cases for the same (tests/plate.qml's backlight
//! and lights, tests/answers.qml's Warmth) moved here with it.

use serde_json::{Value, json};

use super::*;

/// The light's words, from the file the socket's tests hand the daemon.
fn told() -> Words {
    let vocabulary: Value = serde_json::from_str(include_str!("../../tests/screen-words.json"))
        .expect("the test's words are JSON");
    crate::screens::Words::read(&vocabulary).0.light
}

/// The panel's backlight, as light.rs says it.
fn backlight() -> Value {
    json!({"key": "backlight:intel_backlight", "kind": "backlight",
        "device": "intel_backlight", "brightness": 57_600, "actual_brightness": 57_600,
        "max_brightness": 96_000, "type": "raw", "scale": "non-linear", "connector": "eDP-1"})
}

/// The room's sensor, as light.rs says it.
fn sensor() -> Value {
    json!({"key": "light:iio:device0", "kind": "light", "device": "iio:device0",
        "name": "als", "raw": 1234, "scale": 0.1, "offset": 0, "lux": 123.4})
}

/// What the backlight program settled, as steered.rs says it, with its
/// curve.
fn settled(level: f64, lux: &Value, asks: &Value, following: bool) -> Value {
    json!({"key": "brightness:built-in", "kind": "brightness", "level_ratio": level,
        "lux": lux, "asks_ratio": asks, "following": following,
        "curve": [{"lux": 6, "level_ratio": 0.377}, {"lux": 28, "level_ratio": 0.95},
            {"lux": 10_000, "level_ratio": 1}]})
}

/// The warmth following the room, the night light off, as steered.rs
/// says it.
fn warmed() -> Value {
    json!({"key": "warmth:built-in", "kind": "warmth", "kelvin": 6500,
        "follows_the_room": true, "sees_colour": true, "room_white_kelvin": 5100,
        "night_light": false, "night_white_kelvin": 3400, "sunset_ms": 1000,
        "sunrise_ms": 2000})
}

/// The light's facts for a screen, read or not, at the moment 1500, by
/// night.
fn facts_of(records: &[Value], reading: Reading, in_the_room: bool) -> Vec<Value> {
    let numbers = Numbers::default();
    let named = |id: &str| id.to_owned();
    let light = Light::of(records);
    let seen = Seen {
        light: &light,
        reading,
        now: 1500,
    };
    let screen = Screen {
        in_the_room,
        named: &named,
        numbers: &numbers,
    };
    facts(&seen, &screen, &told())
}

/// Each fact as `name=said`, in order.
fn said(facts: &[Value]) -> String {
    facts
        .iter()
        .map(|fact| {
            format!(
                "{}={}",
                fact["name"].as_str().unwrap_or(""),
                fact["said"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn the_panel_says_its_light_in_the_order_the_shell_laid_it() {
    let records = [
        backlight(),
        sensor(),
        settled(0.6, &json!(28), &json!(0.6), true),
        warmed(),
    ];
    let facts = facts_of(&records, Reading::Read, true);
    assert_eq!(
        said(&facts),
        "brightness=60% \u{b7} 57,600 of 96,000 steps; stepping=non-linear; \
         driven-by=the graphics chip; room=123 lx; raw=1,234; per-count=0.1 lx; \
         follows=the room; warmth=6,500 K \u{b7} following the room; \
         curve=38% at 6 lx \u{b7} 95% at 28 lx \u{b7} 100% at 10 klx; \
         device=intel_backlight; sensor=iio:device0"
    );
    assert_eq!(
        facts[0]["figure"],
        json!({"shape": "stroke", "values": [0.6], "warn": false}),
        "the Brightness has no meter of its share"
    );
    assert_eq!(
        (
            facts[9]["identifies"].clone(),
            facts[10]["identifies"].clone()
        ),
        (json!(true), json!(true)),
        "the devices aren't said as what identifies them"
    );
}

#[test]
fn a_monitor_following_the_panel_says_only_its_share() {
    let records = [
        backlight(),
        sensor(),
        settled(0.6, &json!(28), &json!(0.6), true),
    ];
    assert_eq!(
        said(&facts_of(&records, Reading::Read, false)),
        "brightness=60%"
    );
}

#[test]
fn a_hundred_steps_are_the_share_again() {
    let mut hundred = backlight();
    hundred["brightness"] = json!(60);
    hundred["actual_brightness"] = json!(60);
    hundred["max_brightness"] = json!(100);
    assert!(
        said(&facts_of(&[hundred], Reading::Read, true)).starts_with("brightness=60%; "),
        "a hundred steps are said again beside their share"
    );
}

#[test]
fn light_not_yet_read_says_nothing_then_that_it_isnt_and_no_count() {
    let records = [
        backlight(),
        sensor(),
        settled(0.6, &json!(28), &json!(0.6), true),
    ];
    let waiting = facts_of(&records, Reading::Waiting, true);
    assert_eq!(waiting[0]["said"], "", "its first second says a level");
    assert_eq!(
        waiting[0]["figure"]["values"],
        json!([0.0]),
        "its meter shows a level"
    );
    let unread = said(&facts_of(&records, Reading::Unread, true));
    assert!(
        unread.starts_with("brightness=not read yet; ") && unread.contains("room=28.0 lx;"),
        "unread, the room isn't said as the program last heard it: {unread}"
    );
    assert!(
        !unread.contains("raw="),
        "a count not yet read is said: {unread}"
    );
}

#[test]
fn what_sets_the_level_follows_the_program() {
    let follows = |level: f64, asks: Value, following: bool| {
        let facts = facts_of(
            &[backlight(), settled(level, &json!(28), &asks, following)],
            Reading::Read,
            true,
        );
        facts
            .iter()
            .find(|fact| fact["name"] == "follows")
            .map_or_else(String::new, |fact| {
                fact["said"].as_str().unwrap_or("").to_owned()
            })
    };
    assert_eq!(follows(0.38, json!(0.38), true), "the room", "there");
    assert_eq!(
        follows(0.6, json!(0.95), true),
        "the room \u{b7} on its way to 95%",
        "still stepping"
    );
    assert_eq!(
        follows(0.38, json!(0.4), true),
        "the room",
        "a gap too slight to move"
    );
    assert_eq!(follows(0.6, json!(0.95), false), "", "the lid shut");
}

#[test]
fn no_light_heard_is_said_where_no_sensor_hears_the_room() {
    let records = [backlight(), settled(0.6, &Value::Null, &Value::Null, false)];
    assert!(
        said(&facts_of(&records, Reading::Read, true)).contains("room=no light heard"),
        "no light heard isn't said"
    );
    assert!(
        !said(&facts_of(&[backlight()], Reading::Read, true)).contains("room="),
        "nothing settled and no sensor says a room"
    );
}

#[test]
fn the_room_is_measured_to_three_digits_and_the_curve_every_digit() {
    let numbers = Numbers::default();
    let readings: Vec<String> = [0.0, 5.2, 28.47, 999.4, 999.6, 1235.0, 40_000.0]
        .iter()
        .map(|lux| reading(*lux, &numbers))
        .collect();
    assert_eq!(
        readings.join("; "),
        "0 lx; 5.20 lx; 28.5 lx; 999 lx; 1.00 klx; 1.24 klx; 40.0 klx"
    );
    let chose: Vec<String> = [0.0, 50.0, 400.0, 1234.0, 1500.0, 2000.0]
        .iter()
        .map(|lux| chosen(*lux, &numbers))
        .collect();
    assert_eq!(
        chose.join("; "),
        "0 lx; 50 lx; 400 lx; 1.234 klx; 1.5 klx; 2 klx"
    );
}

#[test]
fn the_kernel_and_the_sensor_say_only_what_they_know() {
    let mut moved = backlight();
    moved["actual_brightness"] = json!(1200);
    moved["scale"] = Value::Null;
    moved["type"] = json!("platform");
    let mut offset = sensor();
    offset["offset"] = json!(6);
    let facts = said(&facts_of(&[moved, offset], Reading::Read, true));
    assert!(
        facts.contains(
            "brightness=60% \u{b7} 57,600 of 96,000 steps; reads=1,200; \
            driven-by=a platform driver; room=123 lx; raw=1,234; per-count=0.1 lx; offset=6"
        ),
        "the kernel's steps read back, a platform driver or an offset isn't said: {facts}"
    );
}

#[test]
fn no_backlight_saying_its_most_says_no_light() {
    let mut broken = backlight();
    broken["max_brightness"] = json!(0);
    assert!(
        facts_of(&[broken, sensor()], Reading::Read, true).is_empty(),
        "a backlight with no most says a light"
    );
    assert!(
        facts_of(&[], Reading::Read, true).is_empty(),
        "no records say a light"
    );
}

#[test]
fn the_night_light_is_said_until_sunrise_by_night_and_from_sunset_by_day() {
    let warmth_at = |night_light: bool, sunset: Value, now: u64| {
        let mut warmth = warmed();
        warmth["night_light"] = json!(night_light);
        warmth["sunset_ms"] = sunset;
        let light = Light::of(&[warmth]);
        let held = light.warmth.expect("the warmth is read");
        super::warmth(&held, "Warmth", now, &told(), &Numbers::default())
    };
    assert_eq!(
        warmth_at(true, json!(1000), 1500),
        json!({"name": "Warmth", "at": 2000, "as": "moment",
            "said": "6,500 K \u{b7} night light until {time}"}),
        "by night"
    );
    assert_eq!(
        warmth_at(true, json!(1000), 500),
        json!({"name": "Warmth", "at": 1000, "as": "moment",
            "said": "6,500 K \u{b7} night light from {time}"}),
        "by day"
    );
    assert_eq!(
        warmth_at(true, Value::Null, 500)["said"],
        "6,500 K \u{b7} night light waits for where you are",
        "waiting for a place"
    );
    let mut own = warmed();
    own["sees_colour"] = json!(false);
    let held = Light::of(&[own]).warmth.expect("the warmth is read");
    assert_eq!(
        super::warmth(&held, "Warmth", 500, &told(), &Numbers::default())["said"],
        "6,500 K \u{b7} the screen's own white",
        "the screen's own white"
    );
}

#[test]
fn the_machine_daemons_answer_and_changes_are_held_and_read() {
    let mut held = Held::default();
    let whole = json!({"id": "light", "records": [backlight(), sensor()]});
    assert!(
        held.heard(&whole.to_string()),
        "the whole answer isn't the light's"
    );
    assert_eq!(
        held.reading,
        Reading::Read,
        "an answer doesn't read the light"
    );
    let mut dimmer = backlight();
    dimmer["brightness"] = json!(48_000);
    let changed = json!({"id": "light", "changed": [dimmer], "gone": ["light:iio:device0"]});
    assert!(
        held.heard(&changed.to_string()),
        "a change isn't the light's"
    );
    let records: Vec<Value> = held.records.values().cloned().collect();
    assert_eq!(
        said(&facts_of(&records, Reading::Read, true)),
        "brightness=50% \u{b7} 48,000 of 96,000 steps; reads=57,600; stepping=non-linear; \
         driven-by=the graphics chip; device=intel_backlight",
        "a change isn't held, or a gone sensor still is"
    );
    assert!(
        !held.heard(&json!({"id": "hello", "version": 1}).to_string()),
        "another answer is taken for the light's"
    );
    assert!(
        !held.heard("not JSON"),
        "what isn't JSON is taken for the light"
    );
}

#[test]
fn a_look_waits_a_second_unread_and_hangs_up_once_it_ends() {
    let mut held = Held {
        reading: Reading::Read,
        ..Held::default()
    };
    assert!(held.look(true), "a look beginning changes nothing");
    assert_eq!(held.reading, Reading::Waiting, "a look begins read");
    assert!(!held.look(true), "a second look changes something");
    assert_eq!(
        held.wait(Duration::from_millis(300), 0),
        Some(Duration::from_millis(700)),
        "the first second isn't waited"
    );
    assert_eq!(
        held.wait(Duration::from_secs(2), 0),
        Some(Duration::from_millis(1)),
        "a lapsed second waits none, which a socket refuses"
    );
    held.lapsed(false);
    assert_eq!(
        held.reading,
        Reading::Unread,
        "a lapsed second isn't said unread"
    );
    assert_eq!(
        held.wait(Duration::ZERO, 0),
        None,
        "unread, something is waited for"
    );
    held.reading = Reading::Read;
    held.lapsed(false);
    assert_eq!(
        held.reading,
        Reading::Read,
        "a turn of the clock unreads the light"
    );
    held.lapsed(true);
    assert_eq!(
        held.reading,
        Reading::Unread,
        "a daemon gone leaves the light read"
    );
    assert!(held.look(false), "a look ending changes nothing");
    let mut night = warmed();
    night["night_light"] = json!(true);
    held.heard(&json!({"id": "light", "records": [night]}).to_string());
    assert_eq!(
        held.wait(Duration::ZERO, 400),
        Some(Duration::from_millis(600)),
        "the sunset isn't waited for"
    );
}

#[test]
fn the_drawer_is_said_again_at_the_night_lights_next_turn() {
    let mut night = warmed();
    night["night_light"] = json!(true);
    let light = Light::of(&[night]);
    assert_eq!(light.next_turn(500), Some(1000), "by day, the sunset");
    assert_eq!(light.next_turn(1500), Some(2000), "by night, the sunrise");
    assert_eq!(light.next_turn(2500), None, "past both, nothing");
    assert_eq!(
        Light::of(&[warmed()]).next_turn(500),
        None,
        "the night light off"
    );
}
