//! The cases for Screens' rows and drawers, from what niri and
//! desk-facts say, beside the screens module so it stays short enough to
//! read.

use super::*;
use crate::light::Light;

/// The launcher's words for its Screens, in the file the socket's
/// tests hand the daemon too.
fn vocabulary() -> Value {
    serde_json::from_str(include_str!("../../tests/screen-words.json"))
        .expect("the test's words are JSON")
}

/// The words, every one the daemon reads there, which acts.rs's cases
/// are said in too.
pub(super) fn words() -> Words {
    let (words, lacked) = Words::read(&vocabulary());
    assert!(lacked.is_empty(), "the test's words lack {lacked:?}");
    words
}

/// A screen as niri says one: lit at its first mode, unless `logical`
/// is null.
fn output(make: &str, model: &str, logical: &Value) -> Value {
    json!({
        "make": make, "model": model, "current_mode": 0,
        "modes": [{"width": 2560, "height": 1600, "refresh_rate": 59_951}],
        "logical": logical,
    })
}

fn lit(scale: f64) -> Value {
    json!({"x": 0, "y": 0, "scale": scale, "transform": "Normal"})
}

fn desk() -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "HDMI-A-1".to_owned(),
            output("Dell Inc.", "U2723QE", &lit(1.5)),
        ),
        ("eDP-1".to_owned(), output("BOE", "0x0BCA", &lit(1.75))),
        ("DP-2".to_owned(), output("LG", "0x5B11", &Value::Null)),
    ])
}

fn names(rows: &[Value]) -> Vec<&str> {
    rows.iter().filter_map(|row| row["name"].as_str()).collect()
}

#[test]
fn the_built_in_screen_leads_then_each_by_its_connector_then_turning_them_off() {
    let rows = rows(&desk(), Some("eDP-1"), &words());
    assert_eq!(
        names(&rows),
        [
            "Built-in Screen",
            "LG",
            "Dell Inc. U2723QE",
            "Turn Every Screen Off"
        ],
        "the built-in first, the rest by connector, a numbered model left out"
    );
    assert_eq!(rows[0]["key"], "row:displays:display:eDP-1");
    assert_eq!(rows[0]["thing"], "display:eDP-1");
    assert_eq!(rows[0]["glyph"], "display");
    assert_eq!(
        rows[0]["note"],
        "2560 \u{d7} 1600 \u{b7} 59.951 Hz \u{b7} scale 1.75\u{d7}"
    );
    assert_eq!(rows[0]["held"]["output"], "eDP-1");
    assert_eq!(
        rows[0]["held"]["standing"], "lit",
        "the shell can't tell a lit screen from its row"
    );
    assert_eq!(
        rows[2]["held"]["screen"], "Dell Inc. U2723QE",
        "the agents daemon can't tell which screen a session sat on"
    );
    assert!(
        rows[0]["held"].get("niri").is_none(),
        "a screen's row carries niri's whole record, past what it is: {}",
        rows[0]["held"]
    );
    assert_eq!(rows[3]["key"], "row:displays:sleep");
    assert_eq!(rows[3]["note"], "a key wakes them \u{b7} hubs stay on");
}

// What the agents daemon keeps a place by is niri's every part of it, the
// numbered model and serial its name leaves out among them, and never the
// built-in's name.
#[test]
fn a_screen_holds_what_it_is_by_its_maker_model_and_serial() {
    let mut panel = output("LG Display", "0x07C6", &lit(1.75));
    panel["serial"] = json!("S1");
    let mut dock = output("HP Inc.", "HP Z27k G3", &lit(1.0));
    dock["serial"] = json!("");
    let desk = BTreeMap::from([("eDP-1".to_owned(), panel), ("DP-4".to_owned(), dock)]);
    let rows = rows(&desk, Some("eDP-1"), &words());
    assert_eq!(rows[0]["name"], "Built-in Screen");
    assert_eq!(rows[0]["held"]["screen"], "LG Display 0x07C6 S1");
    assert_eq!(
        rows[1]["held"]["screen"], "HP Inc. HP Z27k G3",
        "a screen niri gives an empty serial is held without one"
    );
}

#[test]
fn a_screen_says_how_it_stands_and_the_focused_one_only_beside_another_lit() {
    let rows = rows(&desk(), Some("eDP-1"), &words());
    assert_eq!(rows[0]["side"], json!([{"said": "focused"}]));
    assert_eq!(rows[1]["side"], json!([{"said": "off"}]));
    assert_eq!(rows[1]["note"], "off", "an unlaid screen runs nothing");
    assert_eq!(rows[2]["side"], json!([]));
    let alone = BTreeMap::from([("eDP-1".to_owned(), output("BOE", "", &lit(2.0)))]);
    let rows = every(&alone, &BTreeMap::new(), Some("eDP-1"), &Light::default());
    assert_eq!(
        rows[0]["side"],
        json!([]),
        "the only screen lit isn't said focused"
    );
}

#[test]
fn a_screen_dark_by_name_says_its_hub_is_on() {
    let mut dark = output("Dell Inc.", "U2723QE", &lit(1.0));
    dark["powered_off"] = json!(true);
    let outputs = BTreeMap::from([("DP-1".to_owned(), dark)]);
    let rows = rows(&outputs, None, &words());
    assert_eq!(rows[0]["side"], json!([{"said": "dark"}]));
    assert_eq!(rows[0]["note"], "hub on");
    assert_eq!(rows.len(), 1, "with none lit, none is turned off");
}

#[test]
fn the_place_counts_its_screens_and_how_many_are_on_in_its_words() {
    let words = words();
    let said = |outputs: &BTreeMap<String, Value>| {
        summary(outputs, &[], &words)["summary"]["said"].clone()
    };
    assert_eq!(summary(&desk(), &[], &words)["key"], "summary:displays");
    assert_eq!(
        said(&desk()),
        "2 of 3 on",
        "a screen niri lays nothing out on is on"
    );
    let mut screens = BTreeMap::from([("eDP-1".to_owned(), output("BOE", "", &lit(2.0)))]);
    assert_eq!(
        said(&screens),
        "1 screen",
        "the only screen isn't counted as one of many"
    );
    screens.insert("DP-1".to_owned(), output("Dell Inc.", "U2723QE", &lit(1.0)));
    assert_eq!(
        said(&screens),
        "2 screens",
        "with every screen on, how many on is said"
    );
    if let Some(dell) = screens.get_mut("DP-1") {
        dell["powered_off"] = json!(true);
    }
    assert_eq!(said(&screens), "1 of 2 on", "a screen dark by name is on");
}

#[test]
fn turning_every_screen_off_says_what_it_does_to_which_and_how_its_typed() {
    let mut outputs = desk();
    let mut dark = output("Dell Inc.", "P2422H", &lit(1.0));
    dark["powered_off"] = json!(true);
    outputs.insert("DP-1".to_owned(), dark);
    let made = rows(&outputs, None, &words());
    let sleep = made
        .iter()
        .find(|row| row["key"] == "row:displays:sleep")
        .expect("Turn Every Screen Off is served while a screen is lit");
    assert_eq!(
        sleep["drawer"],
        json!({
            "eyebrow": "Screens",
            "title": "Turn Every Screen Off",
            "body": concat!(
                "Turns every screen's picture off at once, ",
                "the windows staying where they are."
            ),
            "facts": [
                {"name": "Turns Off", "said": "Built-in Screen \u{b7} Dell Inc. U2723QE"},
                {"name": "Typed", "said": ":set noscreens", "identifies": true},
            ],
        }),
        "its drawer doesn't say what it does, to the lit screens alone, and its : line"
    );
}

#[test]
fn return_focuses_a_lit_screen_and_turns_on_any_other_and_every_one_off() {
    let mut outputs = desk();
    let mut dark = output("Dell Inc.", "U2723QE", &lit(1.0));
    dark["powered_off"] = json!(true);
    outputs.insert("DP-1".to_owned(), dark);
    let made = rows(&outputs, None, &words());
    let acts: Vec<(&str, Value)> = made
        .iter()
        .map(|row| {
            let act = &row["acts"][0];
            (
                row["key"].as_str().unwrap_or(""),
                json!([act["key"], act["ask"]]),
            )
        })
        .collect();
    let screen = |done: &str, output: &str| json!({"ask": "screen", "do": done, "output": output});
    assert_eq!(
        acts,
        [
            (
                "row:displays:display:eDP-1",
                json!(["displays:focus", screen("focus", "eDP-1")])
            ),
            (
                "row:displays:display:DP-1",
                json!(["verb:display-on", screen("wake", "DP-1")])
            ),
            (
                "row:displays:display:DP-2",
                json!(["verb:display-on", screen("on", "DP-2")])
            ),
            (
                "row:displays:display:HDMI-A-1",
                json!(["displays:focus", screen("focus", "HDMI-A-1")])
            ),
            (
                "row:displays:sleep",
                json!(["displays:sleep", {"ask": "screen", "do": "sleep"}])
            ),
        ],
        "a dark screen is turned on by name, being niri's still"
    );
}

#[test]
fn a_screen_naming_nothing_is_its_connector() {
    let outputs = BTreeMap::from([("DP-3".to_owned(), output("", "", &lit(1.0)))]);
    assert_eq!(names(&rows(&outputs, None, &words()))[0], "DP-3");
}

#[test]
fn a_rate_keeps_every_digit_and_a_scale_two_places() {
    assert_eq!(hertz(60_000), "60 Hz");
    assert_eq!(hertz(59_951), "59.951 Hz");
    assert_eq!(hertz(143_900), "143.9 Hz");
    assert_eq!(scale(2.0), "2\u{d7}");
    assert_eq!(scale(1.25), "1.25\u{d7}");
    assert_eq!(scale(1.333_333), "1.33\u{d7}");
}

/// The place's own rows where desk-facts says nothing of any screen, and
/// none is looked at, so no light is said; the settings inside a screen's
/// row are settings/tests.rs's.
fn rows(outputs: &BTreeMap<String, Value>, focused: Option<&str>, words: &Words) -> Vec<Value> {
    let unseen = Seen {
        light: &Light::default(),
        reading: crate::light::Reading::Waiting,
        now: 0,
        looked: false,
    };
    super::rows(outputs, &BTreeMap::new(), focused, true, &unseen, words)
        .into_iter()
        .filter(|row| row.get("parent").is_none())
        .collect()
}

/// Every row, those inside a screen's too, where none is looked at, with
/// `light` held of the light from a look before.
fn every(
    outputs: &BTreeMap<String, Value>,
    desk: &BTreeMap<String, Value>,
    focused: Option<&str>,
    light: &Light,
) -> Vec<Value> {
    let unseen = Seen {
        light,
        reading: crate::light::Reading::Read,
        now: 500,
        looked: false,
    };
    super::rows(outputs, desk, focused, true, &unseen, &words())
}

/// The keys of the place's own rows, those beside the screens, in the
/// order served.
fn placed(made: &[Value]) -> Vec<&str> {
    made.iter()
        .filter(|row| row.get("parent").is_none())
        .filter_map(|row| row["key"].as_str())
        .collect()
}

// The backlight program says its warmth after the screens are served, as
// the answers VM test's does, so the Warmth comes into a making that
// already placed the rest.
#[test]
fn a_warmth_told_late_still_stands_before_turn_every_screen_off() {
    let one = BTreeMap::from([(
        "Virtual-1".to_owned(),
        output("Smithay", "Winit", &lit(1.0)),
    )]);
    let mut kept = ready_rows::kept::Kept::default();
    kept.take(
        every(&one, &BTreeMap::new(), None, &Light::default()),
        Vec::new(),
    );
    let light = Light::of(&[crate::light::tests::warmed()]);
    kept.take(every(&one, &BTreeMap::new(), None, &light), Vec::new());
    let mut top: Vec<(i64, &str)> = kept
        .all()
        .filter(|record| record["row"].get("parent").is_none())
        .map(|record| {
            (
                record["row"]["order"].as_i64().unwrap_or_default(),
                record["key"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    top.sort_unstable();
    assert_eq!(
        top.iter().map(|(_, key)| *key).collect::<Vec<_>>(),
        [
            "row:displays:display:Virtual-1",
            "row:displays:warmth",
            "row:displays:sleep"
        ],
        "a Warmth told late isn't ordered beside the screens: {top:?}"
    );
}

// VKCF3Z, 2026-11-15: Night Light warms every screen, so the Warmth is
// Screens', beside the screens, in the same place with the lid open or
// shut; Follow the Room inside it only while the panel's lit, since only
// it has the room's light to follow.
#[test]
fn the_warmth_stands_beside_the_screens_with_the_lid_open_or_shut() {
    let light = Light::of(&[crate::light::tests::warmed()]);
    let made = every(&desk(), &BTreeMap::new(), None, &light);
    assert_eq!(
        placed(&made)[3..],
        ["row:displays:warmth", "row:displays:sleep"],
        "the Warmth doesn't follow the screens, before Turn Every Screen Off"
    );
    assert!(
        made.iter()
            .any(|row| row["key"] == "row:displays:switch:follow-room"),
        "the panel lit, Follow the Room isn't inside the Warmth"
    );
    let mut shut = desk();
    shut.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    let docked = every(&shut, &BTreeMap::new(), None, &light);
    assert_eq!(
        placed(&docked)[3..],
        ["row:displays:warmth", "row:displays:sleep"],
        "the lid shut, the Warmth isn't where it was"
    );
    let inside: Vec<&str> = docked
        .iter()
        .filter(|row| row["parent"] == "row:displays:warmth")
        .filter_map(|row| row["key"].as_str())
        .collect();
    assert_eq!(
        inside[..2],
        [
            "row:displays:switch:night-light",
            "row:displays:fact:warmth-sunset"
        ],
        "the lid shut, the Warmth doesn't hold the Night Light alone of its switches"
    );
}

#[test]
fn the_brightness_leads_the_screen_holding_it_and_no_screen_holds_the_warmth() {
    let light = Light::of(&[
        json!({"key": "backlight:intel_backlight", "kind": "backlight",
            "device": "intel_backlight", "brightness": 48, "max_brightness": 96,
            "connector": "eDP-1"}),
        crate::light::tests::warmed(),
    ]);
    let inside = |made: &[Value], name: &str| -> Vec<String> {
        let screen = key(&format!("display:{name}"));
        made.iter()
            .filter(|row| row["parent"] == screen.as_str())
            .filter_map(|row| row["key"].as_str().map(str::to_owned))
            .collect()
    };
    let made = every(&desk(), &BTreeMap::new(), None, &light);
    assert_eq!(
        inside(&made, "eDP-1").first().map(String::as_str),
        Some("row:displays:setting:eDP-1:brightness"),
        "the Brightness doesn't lead the built-in screen"
    );
    assert!(
        !made
            .iter()
            .any(|row| row["kind"] == "warmth" && row.get("parent").is_some()),
        "a screen holds the Warmth, which is Screens'"
    );
    let mut away = desk();
    away.insert("eDP-1".to_owned(), output("BOE", "0x0BCA", &Value::Null));
    away.insert("DP-3".to_owned(), output("LG", "27GL850", &lit(1.0)));
    let follows = BTreeMap::from([(
        "DP-3".to_owned(),
        json!([{"name": "Brightness", "value": "95%"}]),
    )]);
    let docked = every(&away, &follows, None, &light);
    let held = inside(&docked, "DP-3");
    assert_eq!(
        held.first().map(String::as_str),
        Some("row:displays:setting:DP-3:brightness"),
        "the monitor following the panel doesn't lead with the Brightness"
    );
    assert!(
        !held
            .iter()
            .any(|key| key.starts_with("row:displays:warmth")),
        "a monitor holds the Warmth, which is Screens'"
    );
}

/// A screen's drawer where desk-facts says nothing of it, and no light.
fn drawer(name: &str, output: &Value, words: &Words) -> Value {
    super::drawer(name, output, &Value::Null, Vec::new(), words)
}

/// What desk-facts says of a monitor, as it says the desk's own.
fn monitor() -> Value {
    json!([
        {"name": "maker", "value": "HP"},
        {"name": "model", "value": "HP Z27k G3"},
        {"name": "serial", "value": "CN41282GC1"},
        {"name": "size", "value": "60 \u{d7} 34 cm, 27.2 in"},
        {"name": "native", "value": "3840 \u{d7} 2160 at 60 Hz"},
        {"name": "depth", "value": "10 bit"},
        {"name": "input", "value": "DisplayPort"},
        {"name": "white", "value": "6462 0.3134765625 0.3291015625"},
        {"name": "primaries", "value":
            "0.6396484375 0.330078125 0.2998046875 0.599609375 0.150390625 0.0595703125"},
        {"name": "HDR", "value": "none"},
        {"name": "brightness", "value": "95%"},
        {"name": "hot-plug detection", "value": "always on"},
    ])
}

#[test]
fn a_monitors_edid_follows_what_niri_says_the_asked_first_leading() {
    let mut hp = panel();
    hp["physical_size"] = json!([600, 340]);
    let drawn = super::drawer("DP-4", &hp, &monitor(), Vec::new(), &words());
    let edid = said(&drawn)
        .split_once("Density=")
        .map(|(_, after)| after.to_owned())
        .unwrap_or_default();
    assert_eq!(
        edid,
        "122 ppi; Colour Depth=10 bit; \
         White Point=6,460 K \u{b7} x 0.313 y 0.329; \
         Primaries=red x 0.640 y 0.330 \u{b7} green x 0.300 y 0.600 \u{b7} \
         blue x 0.150 y 0.060; HDR=none; Maker=HP; Model=HP Z27k G3; \
         Size=60 \u{d7} 34 cm, 27.2 in; Native=3840 \u{d7} 2160 at 60 Hz; \
         Input=DisplayPort; Hot-plug detection=always on; Serial=CN41282GC1"
    );
}

#[test]
fn a_step_into_a_screen_offers_its_settings_then_its_ways_then_its_edid_to_copy() {
    let outputs = BTreeMap::from([
        ("DP-4".to_owned(), panel()),
        ("eDP-1".to_owned(), output("BOE", "0x0BCA", &lit(1.75))),
        ("DP-2".to_owned(), output("LG", "0x5B11", &Value::Null)),
    ]);
    let desk = BTreeMap::from([
        ("DP-4".to_owned(), monitor()),
        ("DP-2".to_owned(), json!([{"name": "maker", "value": "LG"}])),
    ]);
    let made = every(&outputs, &desk, None, &Light::default());
    let inside = |name: &str| -> Vec<&Value> {
        let screen = key(&format!("display:{name}"));
        made.iter()
            .filter(|row| row["parent"] == screen.as_str())
            .collect()
    };
    let kinds: Vec<&str> = inside("DP-4")
        .iter()
        .filter_map(|row| row["kind"].as_str())
        .collect();
    let settled = kinds.iter().take_while(|kind| **kind != "action").count();
    assert!(
        settled > 0 && kinds[..settled].iter().all(|kind| *kind != "fact"),
        "a lit screen's settings don't lead: {kinds:?}"
    );
    let rest: Vec<Value> = inside("DP-4").into_iter().skip(settled).cloned().collect();
    assert_eq!(
        names(&rest)[..5],
        [
            "Turn It Off",
            "Unplug It",
            "Colour Depth",
            "White Point",
            "Primaries"
        ],
        "the ways don't follow the settings, or the EDID's asked first don't lead it"
    );
    assert_eq!(
        rest.len(),
        2 + 11,
        "not every EDID fact but its brightness is a row"
    );
    assert_eq!(rest[1]["note"], "its windows move to the other screens");
    assert_eq!(
        rest[3]["acts"],
        json!([{"key": "verb:copy", "copy": "6,460 K \u{b7} x 0.313 y 0.329"}]),
        "a fact's Return doesn't copy what it says"
    );
    assert_eq!(rest[3]["thing"], "fact:displays:DP-4:white");
    let off: Vec<Value> = inside("DP-2").into_iter().cloned().collect();
    assert_eq!(
        names(&off),
        ["Turn It On", "Maker"],
        "an unplugged screen offers more than turning on and its EDID"
    );
}

#[test]
fn the_last_lit_screens_turn_it_off_closes_the_launcher_and_says_a_key_wakes_it() {
    let outputs = BTreeMap::from([("eDP-1".to_owned(), output("BOE", "", &lit(2.0)))]);
    let made = every(&outputs, &BTreeMap::new(), None, &Light::default());
    let way = made
        .iter()
        .find(|row| row["kind"] == "action" && row.get("parent").is_some())
        .expect("the last lit screen has no way to turn off");
    assert_eq!(
        (
            &way["name"],
            &way["glyph"],
            &way["note"],
            &way["acts"][0]["closes"]
        ),
        (
            &json!("Turn It Off"),
            &json!("displayOff"),
            &json!("a key wakes it"),
            &json!(true)
        ),
        "the last lit screen's darkening doesn't close the launcher, saying a key wakes it"
    );
}

#[test]
fn niris_size_stands_only_where_the_edid_says_none_and_niris_serial_leads() {
    let mut hp = panel();
    hp["serial"] = json!("from niri");
    let sizeless: Vec<Value> = monitor()
        .as_array()
        .expect("facts")
        .iter()
        .filter(|fact| fact["name"] != "size")
        .cloned()
        .collect();
    let drawn = said(&super::drawer(
        "DP-4",
        &hp,
        &json!(sizeless),
        Vec::new(),
        &words(),
    ));
    assert!(
        drawn.contains("Size=300 \u{d7} 190 mm") && drawn.ends_with("Serial=from niri"),
        "niri's size or serial isn't said: {drawn}"
    );
    assert_eq!(
        drawn.matches("Serial=").count(),
        1,
        "a serial is said twice: {drawn}"
    );
}

#[test]
fn a_white_point_or_primaries_not_whole_say_nothing_and_figures_take_the_accounts_point() {
    let facts = json!([
        {"name": "white", "value": "6462 0.3134765625"},
        {"name": "primaries", "value": "0.6 0.3 0.3 0.6 0.15 0.3125"},
    ]);
    let words = words().numbered(Numbers::new(".", ","));
    let drawn = said(&super::drawer("DP-4", &panel(), &facts, Vec::new(), &words));
    assert!(
        !drawn.contains("White Point")
            && drawn.contains(
                "Primaries=red x 0,600 y 0,300 \u{b7} green x 0,300 y 0,600 \u{b7} \
                 blue x 0,150 y 0,313"
            ),
        "a white point not whole is said, or a coordinate halfway rounds down: {drawn}"
    );
}

#[test]
fn desk_facts_file_holds_each_screen_by_connector() {
    let desk = super::desk(r#"{"DP-4": [{"name": "maker", "value": "HP"}]}"#);
    assert_eq!(desk.keys().collect::<Vec<_>>(), ["DP-4"]);
    assert!(
        super::desk("not JSON").is_empty(),
        "a file half written says something"
    );
}

/// The built-in panel as niri says it, from the cases of
/// modules/quickshell/tests/plate.qml.
fn panel() -> Value {
    json!({
        "make": "LG Display", "model": "0x07C6", "serial": null,
        "physical_size": [300, 190],
        "modes": [
            {"width": 2880, "height": 1800, "refresh_rate": 120_001, "is_preferred": true},
            {"width": 2880, "height": 1800, "refresh_rate": 60_000, "is_preferred": true}
        ],
        "current_mode": 0, "is_custom_mode": false,
        "vrr_supported": true, "vrr_enabled": false,
        "logical": {"x": 560, "y": 1440, "width": 1440, "height": 900, "scale": 2.0,
            "transform": "Normal"}
    })
}

/// A drawer's facts, each name=said, a semicolon apart.
fn said(drawer: &Value) -> String {
    drawer["facts"]
        .as_array()
        .expect("the drawer's facts")
        .iter()
        .map(|fact| format!("{}={}", fact["name"].as_str().unwrap_or(""), fact["said"]))
        .collect::<Vec<_>>()
        .join("; ")
        .replace('"', "")
}

#[test]
fn a_screen_on_says_in_its_drawer_what_it_runs_past_its_row_where_it_sits_and_offers() {
    let drawer = drawer("eDP-1", &panel(), &words());
    assert_eq!(drawer["eyebrow"], "eDP-1 \u{b7} built in");
    assert_eq!(
        drawer["eyebrowNames"], true,
        "a connector isn't set as written"
    );
    assert_eq!(
        drawer["title"], "Built-in Screen",
        "its title isn't its row's name"
    );
    assert_eq!(
        said(&drawer),
        "Screen Mode=preferred; Refresh=120.001 Hz; Logical Size=1440 \u{d7} 900; \
         Position=560, 1440; Rotation=upright; \
         Screen Modes=2 \u{b7} preferred 2880 \u{d7} 1800 at 120.001 Hz; \
         Density=244 ppi; Size=300 \u{d7} 190 mm"
    );
}

#[test]
fn a_drawer_says_custom_and_its_serial_last_as_what_identifies_it() {
    let mut on = panel();
    on["is_custom_mode"] = json!(true);
    on["serial"] = json!("0x1234");
    on["modes"][0]["is_preferred"] = json!(false);
    let drawer = drawer("DP-1", &on, &words());
    assert_eq!(drawer["eyebrow"], "DP-1 \u{b7} screen");
    assert!(
        said(&drawer).starts_with("Screen Mode=custom; Refresh=120.001 Hz; Logical Size="),
        "a custom mode isn't said so: {}",
        said(&drawer)
    );
    let facts = drawer["facts"].as_array().expect("facts");
    assert_eq!(
        facts.last(),
        Some(&json!({"name": "Serial", "said": "0x1234", "identifies": true})),
        "the serial doesn't end the drawer, identifying the screen"
    );
}

#[test]
fn a_whole_refresh_is_the_rows_and_only_a_lack_of_variable_refresh_the_drawers() {
    let mut on = panel();
    on["modes"][0]["refresh_rate"] = json!(60_000);
    on["modes"][0]["is_preferred"] = json!(false);
    assert!(
        said(&drawer("eDP-1", &on, &words())).starts_with("Logical Size="),
        "a whole refresh or a plain mode is said again: {}",
        said(&drawer("eDP-1", &on, &words()))
    );
    on["vrr_supported"] = json!(false);
    assert!(
        said(&drawer("eDP-1", &on, &words())).starts_with("Variable Refresh=unsupported; "),
        "a screen without variable refresh doesn't say so: {}",
        said(&drawer("eDP-1", &on, &words()))
    );
}

#[test]
fn a_screen_off_says_only_what_it_offers_and_is_in_the_accounts_numbers() {
    let mut off = panel();
    off["logical"] = Value::Null;
    off["current_mode"] = Value::Null;
    off["physical_size"] = json!([3000, 0]);
    let words = words().numbered(Numbers::new(".", ","));
    assert_eq!(
        said(&drawer("eDP-1", &off, &words)),
        "Screen Modes=2 \u{b7} preferred 2880 \u{d7} 1800 at 120.001 Hz; Density=24,4 ppi",
        "a screen off says what it runs, or a size niri half knows"
    );
}

#[test]
fn each_screens_row_carries_its_drawer() {
    let rows = rows(&desk(), None, &words());
    assert_eq!(rows[0]["drawer"]["title"], "Built-in Screen");
    assert_eq!(rows[2]["drawer"]["title"], "Dell Inc. U2723QE");
}

#[test]
fn niris_answer_holds_its_outputs_by_connector() {
    let answer = json!({"Ok": {"Outputs": {"eDP-1": {"name": "eDP-1"}}}});
    let outputs = outputs(&answer).expect("outputs");
    assert_eq!(outputs.keys().collect::<Vec<_>>(), ["eDP-1"]);
    assert_eq!(super::outputs(&json!({"Err": "no"})), None);
}

#[test]
fn a_vocabulary_without_a_word_says_it_by_its_id_and_which() {
    let mut vocabulary = vocabulary();
    vocabulary["things"][0]["said"]
        .as_object_mut()
        .expect("said")
        .remove("hubOn");
    let (words, lacked) = Words::read(&vocabulary);
    assert_eq!(words.hub_on, "hubOn", "a word lacked isn't said by its id");
    assert_eq!(
        words.dark,
        self::words().dark,
        "a word held isn't said as held"
    );
    assert_eq!(lacked, vec!["displays said.hubOn".to_owned()]);
}

#[test]
fn no_vocabulary_still_words_every_row_by_its_ids() {
    let (words, lacked) = Words::read(&Value::Null);
    assert_eq!(
        words.built_in, "displays:built-in",
        "the built-in isn't named by its id"
    );
    assert_eq!(
        words.sleep.name, "displays:sleep",
        "Sleep isn't named by its id"
    );
    assert_eq!(words.sleep.glyph, "", "a glyph lacked isn't none");
    assert_eq!(
        lacked.len(),
        112,
        "not every word lacked was said: {lacked:?}"
    );
    let drawer = drawer("eDP-1", &panel(), &words);
    assert_eq!(
        drawer["facts"][0]["name"], "displays:mode",
        "a fact isn't named by its id"
    );
    assert_eq!(
        drawer["facts"][4]["said"], "Normal",
        "a turn isn't said as niri says it"
    );
}

/// The launcher's own words.json, where the build hands it in.
#[test]
fn the_launchers_vocabulary_has_every_screens_words() {
    let Some(path) = option_env!("WINDOWS_TEST_WORDS") else {
        return;
    };
    let read = std::fs::read_to_string(path).expect("the launcher's words read");
    let vocabulary: Value = serde_json::from_str(&read).expect("the words are JSON");
    let (_, lacked) = Words::read(&vocabulary);
    assert!(lacked.is_empty(), "the launcher's words lack {lacked:?}");
}

mod answers;
mod light;
mod set;
