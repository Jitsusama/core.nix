//! The cases for a lit screen's settings and their choices, as a step
//! into its row offers them, as Displays.qml's setUp made them of
//! Screens.js until 11 November 2026.

use super::*;
use crate::screens::tests::words;

/// A monitor running its 2560 by 1440 at 59.951 Hz, beside the same size
/// at 60 Hz, which its EDID prefers, and 1920 by 1080, variable refresh
/// off, turned upright at 1.5×.
fn monitor() -> Value {
    json!({
        "current_mode": 0,
        "is_custom_mode": false,
        "vrr_supported": true,
        "vrr_enabled": false,
        "modes": [
            {"width": 2560, "height": 1440, "refresh_rate": 59_951, "is_preferred": false},
            {"width": 2560, "height": 1440, "refresh_rate": 60_000, "is_preferred": true},
            {"width": 1920, "height": 1080, "refresh_rate": 60_000, "is_preferred": false},
        ],
        "logical": {"scale": 1.5, "transform": "Normal"},
    })
}

/// The rows inside the row of `parent`, by name, each with its side.
fn inside<'a>(rows: &'a [Value], parent: &str) -> Vec<(&'a str, &'a Value)> {
    rows.iter()
        .filter(|row| row["parent"] == parent)
        .map(|row| (row["name"].as_str().unwrap_or(""), &row["side"]))
        .collect()
}

fn found<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["key"] == key)
        .unwrap_or(&Value::Null)
}

#[test]
fn a_lit_screens_settings_each_say_how_its_set_inside_its_row() {
    let rows = rows("DP-2", &monitor(), &words());
    assert_eq!(
        inside(&rows, "row:displays:display:DP-2"),
        [
            ("Scale", &json!([{"said": "1.5\u{d7}"}])),
            ("Refresh", &json!([{"said": "59.951 Hz"}])),
            ("Variable Refresh", &json!([{"said": "off"}])),
            ("Resolution", &json!([{"said": "2560 \u{d7} 1440"}])),
            ("Rotation", &json!([{"said": "upright"}])),
        ],
        "the settings weren't Scale, Refresh, Variable Refresh, Resolution and Rotation"
    );
    let scale = found(&rows, "row:displays:setting:DP-2:scale");
    assert_eq!(scale["thing"], "displays:DP-2:scale");
    assert_eq!(scale["glyph"], "scale");
    assert_eq!(scale["depth"], 1);
    assert_eq!(
        scale["acts"],
        json!([{"key": "verb:more", "step": "rows"}]),
        "a setting's Return doesn't step into its choices"
    );
}

#[test]
fn a_scale_is_chosen_as_a_multiple_found_by_its_share_and_typed_as_the_line_types_it() {
    let rows = rows("DP-2", &monitor(), &words());
    let at = "row:displays:setting:DP-2:scale";
    assert_eq!(
        inside(&rows, at)
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        [
            "1\u{d7}",
            "1.25\u{d7}",
            "1.5\u{d7}",
            "1.75\u{d7}",
            "2\u{d7}",
            "2.5\u{d7}",
            "3\u{d7}"
        ],
        "the scales offered aren't each said as a multiple"
    );
    let chosen = found(&rows, "row:displays:choice:DP-2:scale-1.5");
    assert_eq!(chosen["thing"], "verb:display-scale-1.5");
    assert_eq!(chosen["side"], json!([{"said": "chosen"}]));
    assert_eq!(chosen["depth"], 2);
    assert_eq!(
        chosen["held"],
        json!({"keywords": "Scale 150%", "fill": {"setting": "scale", "value": "1.5"}}),
        "the choice isn't found by its share or filled as :setl scale=1.5"
    );
    assert_eq!(
        Act::asked(&chosen["acts"][0]["ask"]).map(|act| act.request()),
        Some(json!({"Output": {"output": "DP-2",
            "action": {"Scale": {"scale": {"Specific": 1.5}}}}})),
        "the choice doesn't set its scale"
    );
    assert_eq!(chosen["acts"][0]["key"], "verb:display-set");
    let one = found(&rows, "row:displays:choice:DP-2:scale-1");
    assert_eq!(one["side"], json!([]), "a scale not at it is said chosen");
    assert_eq!(one["held"]["fill"]["value"], "1");
}

#[test]
fn refresh_offers_each_rate_at_its_size_by_every_digit_the_preferred_noted() {
    let rows = rows("DP-2", &monitor(), &words());
    let rates: Vec<(&Value, &Value, &Value)> = rows
        .iter()
        .filter(|row| row["parent"] == "row:displays:setting:DP-2:refresh")
        .map(|row| (&row["key"], &row["name"], &row["note"]))
        .collect();
    assert_eq!(
        rates,
        [
            (
                &json!("row:displays:choice:DP-2:mode-0"),
                &json!("59.951 Hz"),
                &json!("")
            ),
            (
                &json!("row:displays:choice:DP-2:mode-1"),
                &json!("60 Hz"),
                &json!("preferred")
            ),
        ],
        "the rates aren't each mode's at the size it runs"
    );
    let mut custom = monitor();
    custom["is_custom_mode"] = json!(true);
    assert_eq!(
        found(
            &super::rows("DP-2", &custom, &words()),
            "row:displays:choice:DP-2:mode-0"
        )["note"],
        "custom",
        "a custom mode niri runs isn't noted so"
    );
    let sixty = found(&rows, "row:displays:choice:DP-2:mode-1");
    assert_eq!(
        sixty["acts"][0]["ask"]["action"],
        json!({"Mode": {"mode": {"Specific": {"width": 2560, "height": 1440, "refresh": 60.0}}}}),
    );
    assert_eq!(sixty["held"]["fill"]["value"], "60");
}

#[test]
fn resolution_offers_each_size_once_and_niri_picks_its_rate() {
    let rows = rows("DP-2", &monitor(), &words());
    let sizes = inside(&rows, "row:displays:setting:DP-2:resolution");
    assert_eq!(
        sizes,
        [
            ("2560 \u{d7} 1440", &json!([{"said": "chosen"}])),
            ("1920 \u{d7} 1080", &json!([])),
        ],
        "a size two modes share isn't offered once"
    );
    let full = found(&rows, "row:displays:choice:DP-2:size-2560x1440");
    assert_eq!(
        full["note"], "preferred",
        "a size any mode is preferred at isn't"
    );
    assert_eq!(full["held"]["fill"]["value"], "2560x1440");
    assert_eq!(
        full["acts"][0]["ask"]["action"],
        json!({"Mode": {"mode": {"Specific": {"width": 2560, "height": 1440, "refresh": null}}}}),
    );
}

#[test]
fn a_setting_with_one_choice_only_says_how_its_set() {
    let mut output = monitor();
    output["modes"] =
        json!([{"width": 2880, "height": 1920, "refresh_rate": 120_000, "is_preferred": true}]);
    let rows = rows("eDP-1", &output, &words());
    let refresh = found(&rows, "row:displays:setting:eDP-1:refresh");
    assert_eq!(refresh["side"], json!([{"said": "120 Hz"}]));
    assert_eq!(refresh["acts"], json!([]), "one rate is stepped into");
    assert_eq!(refresh["glyph"], "");
    assert!(
        inside(&rows, "row:displays:setting:eDP-1:refresh").is_empty(),
        "one rate is offered as a choice"
    );
}

#[test]
fn variable_refresh_flips_as_a_switch_and_says_how_it_stands() {
    let mut output = monitor();
    let off = found(
        &rows("DP-2", &output, &words()),
        "row:displays:setting:DP-2:variable",
    )
    .clone();
    assert_eq!(off["kind"], "switch");
    assert_eq!(off["glyph"], "variableRefresh");
    assert_eq!(off["held"], json!({"on": false}));
    assert_eq!(
        off["acts"],
        json!([{"key": "displays:variable", "ask": {"ask": "screen", "do": "set",
            "output": "DP-2", "action": {"Vrr": {"vrr": {"vrr": true, "on_demand": false}}},
            "said": "Variable Refresh", "done": "on"}}]),
        "Return doesn't turn it on"
    );
    output["vrr_enabled"] = json!(true);
    let rows = rows("DP-2", &output, &words());
    let on = found(&rows, "row:displays:setting:DP-2:variable");
    assert_eq!(on["side"], json!([{"said": "on"}]));
    assert_eq!(on["acts"][0]["ask"]["done"], "off");
    output["vrr_supported"] = json!(false);
    assert!(
        found(
            &super::rows("DP-2", &output, &words()),
            "row:displays:setting:DP-2:variable"
        )
        .is_null(),
        "a screen without it offers it"
    );
}

#[test]
fn rotation_offers_the_four_turns_then_the_four_mirrored_each_typed_in_its_words() {
    let rows = rows("DP-2", &monitor(), &words());
    let turns: Vec<(&Value, &Value)> = rows
        .iter()
        .filter(|row| row["parent"] == "row:displays:setting:DP-2:rotation")
        .map(|row| (&row["name"], &row["held"]["fill"]["value"]))
        .collect();
    assert_eq!(turns.len(), 8, "not every one of niri's turns is offered");
    assert_eq!(
        turns[..2],
        [
            (&json!("upright"), &json!("upright")),
            (&json!("a quarter turn left"), &json!("left"))
        ],
        "a turn isn't said and typed in the vocabulary's words"
    );
    assert_eq!(
        found(&rows, "row:displays:choice:DP-2:turn-90")["acts"][0]["ask"]["action"],
        json!({"Transform": {"transform": "90"}}),
    );
}

#[test]
fn a_screen_running_no_mode_offers_only_its_scale_and_rotation() {
    let mut output = monitor();
    output["current_mode"] = Value::Null;
    let rows = rows("DP-2", &output, &words());
    assert_eq!(
        inside(&rows, "row:displays:display:DP-2")
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        ["Scale", "Rotation"],
        "a screen running no mode offers its mode's settings"
    );
}
