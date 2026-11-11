//! The cases for a lit screen's settings on the : line, as tests/plate.qml
//! held them for Screens.js's lineSet until 11 November 2026: each
//! completing from its own choices, their words beside, the turns in the
//! vocabulary's typed words, a whole rate taken where one mode rounds to
//! it, and a value it doesn't offer saying what it does.

use super::*;
use crate::screens::tests::words;

/// An HP monitor running 2560 by 1440 at 59.951 Hz, which it prefers,
/// beside 75 Hz at that size and 1920 by 1080, at twice, mirrored and
/// turned left.
fn monitor() -> Value {
    json!({
        "make": "HP",
        "model": "0x3302",
        "current_mode": 0,
        "is_custom_mode": false,
        "vrr_supported": true,
        "vrr_enabled": false,
        "modes": [
            {"width": 2560, "height": 1440, "refresh_rate": 59_951, "is_preferred": true},
            {"width": 2560, "height": 1440, "refresh_rate": 75_000, "is_preferred": false},
            {"width": 1920, "height": 1080, "refresh_rate": 60_000, "is_preferred": false},
        ],
        "logical": {"scale": 2.0, "transform": "Flipped90"},
    })
}

fn outputs(output: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([("DP-2".to_owned(), output)])
}

/// What the line's `option` typed as `typed` asks of DP-2: niri's action,
/// or the words refusing it.
fn set(output: &Value, option: &str, typed: &Value) -> Result<Value, Value> {
    let said = json!({"ask": "screen", "do": "line", "output": "DP-2", "option": option,
        "typed": typed});
    match asked(
        &json!("ask-1"),
        &said,
        &outputs(output.clone()),
        Some(&words()),
    ) {
        Ok(act) => Ok(act.request()["Output"]["action"].clone()),
        Err(answer) => Err(answer["error"]["said"].clone()),
    }
}

/// Each value an option completes from, with its words beside.
fn values(output: &Value, option: &str) -> Vec<String> {
    of(output, &words())[option]["values"]
        .as_array()
        .expect("the option serves no values")
        .iter()
        .map(|each| format!("{} ({})", each["value"], each["note"]).replace('"', ""))
        .collect()
}

#[test]
fn each_setting_completes_from_its_own_choices_their_words_beside() {
    let monitor = monitor();
    let line = of(&monitor, &words());
    let options: Vec<&String> = line.as_object().expect("no options").keys().collect();
    assert_eq!(
        options,
        [
            "refresh",
            "resolution",
            "rotation",
            "scale",
            "variablerefresh"
        ],
        "the line didn't take the five settings a running mode has"
    );
    assert_eq!(
        values(&monitor, "scale")[..3],
        ["1 (1\u{d7})", "1.25 (1.25\u{d7})", "1.5 (1.5\u{d7})"],
        "scale didn't complete as typed, its multiple beside"
    );
    assert_eq!(
        values(&monitor, "refresh"),
        ["59.951 (59.951 Hz \u{b7} preferred)", "75 (75 Hz)"],
        "refresh didn't complete every digit, its words beside"
    );
    assert_eq!(
        values(&monitor, "resolution"),
        [
            "2560x1440 (2560 \u{d7} 1440 \u{b7} preferred)",
            "1920x1080 (1920 \u{d7} 1080)"
        ],
        "resolution didn't complete by x"
    );
    assert_eq!(
        values(&monitor, "rotation")[..2],
        ["upright (upright)", "left (a quarter turn left)"],
        "rotation didn't complete in its typed words"
    );
}

#[test]
fn each_setting_says_where_it_stands_as_its_typed() {
    let line = of(&monitor(), &words());
    let now: Vec<&Value> = [
        "scale",
        "refresh",
        "resolution",
        "rotation",
        "variablerefresh",
    ]
    .iter()
    .map(|option| &line[*option]["now"])
    .collect();
    assert_eq!(
        now,
        [
            &json!("2"),
            &json!("59.951"),
            &json!("2560x1440"),
            &json!("mirrored-left"),
            &json!(false)
        ],
        "the settings didn't say where they stand"
    );
}

#[test]
fn a_value_is_taken_as_the_line_types_it() {
    let monitor = monitor();
    let scaled = Ok(json!({"Scale": {"scale": {"Specific": 1.5}}}));
    assert_eq!(
        set(&monitor, "scale", &json!("1.5")),
        scaled,
        "1.5 wasn't a scale"
    );
    assert_eq!(
        set(&monitor, "scale", &json!("1.5\u{d7}")),
        scaled,
        "1.5× wasn't 1.5"
    );
    assert_eq!(
        set(&monitor, "refresh", &json!("60")),
        Ok(
            json!({"Mode": {"mode": {"Specific": {"width": 2560, "height": 1440,
            "refresh": 59.951}}}})
        ),
        "a whole rate one mode rounds to didn't set it"
    );
    assert_eq!(
        set(&monitor, "refresh", &json!("75 Hz")),
        Ok(
            json!({"Mode": {"mode": {"Specific": {"width": 2560, "height": 1440,
            "refresh": 75.0}}}})
        ),
        "a rate wasn't taken with its unit"
    );
    assert_eq!(
        set(&monitor, "resolution", &json!("1920\u{d7}1080")),
        Ok(
            json!({"Mode": {"mode": {"Specific": {"width": 1920, "height": 1080,
            "refresh": null}}}})
        ),
        "a size by × wasn't taken, niri choosing its rate"
    );
    assert_eq!(
        set(&monitor, "rotation", &json!("Mirrored-Left")),
        Ok(json!({"Transform": {"transform": "Flipped90"}})),
        "a turn wasn't taken by its typed word, in any case"
    );
    assert_eq!(
        set(&monitor, "variablerefresh", &json!(true)),
        Ok(json!({"Vrr": {"vrr": {"vrr": true, "on_demand": false}}})),
        "variable refresh didn't switch on"
    );
}

#[test]
fn a_value_it_doesnt_offer_says_what_it_does() {
    let monitor = monitor();
    assert_eq!(
        set(&monitor, "scale", &json!("1.3")),
        Err(json!(
            "1.3\u{d7} not set \u{b7} HP offers 1\u{d7} to 3\u{d7}"
        )),
        "a scale it doesn't offer didn't say what it does"
    );
    assert_eq!(
        set(&monitor, "refresh", &json!("120")),
        Err(json!("120 Hz not set \u{b7} HP offers 59.951 Hz to 75 Hz")),
        "a rate no one mode rounds to wasn't refused in hertz"
    );
    let turns = set(&monitor, "rotation", &json!("sideways"));
    assert!(
        turns
            .as_ref()
            .err()
            .and_then(Value::as_str)
            .is_some_and(|said| said.starts_with(
                "sideways not set \u{b7} HP offers upright, left, upside-down, right, mirrored, "
            )),
        "a turn it doesn't know didn't say each: {turns:?}"
    );
    assert_eq!(
        set(&monitor, "scale", &json!(" ")),
        Err(json!("no scale given \u{b7} HP offers 1\u{d7} to 3\u{d7}")),
        "nothing after = didn't say none was given"
    );
}

#[test]
fn a_figures_ends_are_its_least_and_its_most_and_two_modes_alike_complete_once() {
    let mut monitor = monitor();
    // niri lists the preferred mode first, so the ends are found by size,
    // never by place.
    monitor["modes"] = json!([
        {"width": 1920, "height": 1080, "refresh_rate": 60_000, "is_preferred": true},
        {"width": 1920, "height": 1080, "refresh_rate": 144_000, "is_preferred": false},
        {"width": 1920, "height": 1080, "refresh_rate": 144_000, "is_preferred": false},
        {"width": 1920, "height": 1080, "refresh_rate": 59_940, "is_preferred": false},
        {"width": 2560, "height": 1440, "refresh_rate": 60_000, "is_preferred": false},
        {"width": 1280, "height": 720, "refresh_rate": 60_000, "is_preferred": false},
    ]);
    assert_eq!(
        set(&monitor, "refresh", &json!("100")),
        Err(json!("100 Hz not set \u{b7} HP offers 59.94 Hz to 144 Hz")),
        "a rate's ends weren't its least and its most"
    );
    assert_eq!(
        set(&monitor, "resolution", &json!("800x600")),
        Err(json!(
            "800 \u{d7} 600 not set \u{b7} HP offers 1280 \u{d7} 720 to 2560 \u{d7} 1440"
        )),
        "a size's ends weren't its least and its most"
    );
    assert_eq!(
        values(&monitor, "refresh"),
        [
            "60 (60 Hz \u{b7} preferred)",
            "144 (144 Hz)",
            "59.94 (59.94 Hz)"
        ],
        "two modes alike didn't complete once"
    );
}

#[test]
fn a_screen_running_no_mode_takes_only_its_scale_and_turn() {
    let mut monitor = monitor();
    monitor["current_mode"] = Value::Null;
    let line = of(&monitor, &words());
    let options: Vec<&String> = line.as_object().expect("no options").keys().collect();
    assert_eq!(
        options,
        ["rotation", "scale"],
        "it took more than its scale and turn"
    );
}

#[test]
fn a_screen_gone_says_its_unplugged_and_any_other_ask_is_its_rows_act() {
    let said = json!({"ask": "screen", "do": "line", "output": "HDMI-1", "option": "scale",
        "typed": "1"});
    let answer = asked(&json!("ask-1"), &said, &outputs(monitor()), Some(&words()));
    assert_eq!(
        answer.err().map(|answer| answer["error"]["said"].clone()),
        Some(json!("HDMI-1 is unplugged")),
        "a screen gone didn't say it's unplugged"
    );
    let sleep = asked(
        &json!("ask-2"),
        &Act::Sleep.ask(),
        &outputs(monitor()),
        Some(&words()),
    );
    assert_eq!(
        sleep.ok(),
        Some(Act::Sleep),
        "a row's act wasn't asked as it is"
    );
}
