//! A device's drawer's cases, apart from `device_drawer.rs` as
//! `drawer.rs`'s are: where it's served, what it says first and last, the format it says
//! once, how it's joined, and its channels' levels.

use serde_json::{Value, json};

use super::*;
use crate::drawer::tests::{facts, lines};
use crate::programs::Named;
use crate::rows::tests::{device, speakers_chosen as speakers, words};

/// The rows served of these records, then `more` the test adds beside
/// them, as desk facts are, and the drawer of the device named `name`.
fn drawn(records: &[Value], more: &[Value], name: &str) -> (Vec<Value>, Value) {
    let naming = |given: &str| Named {
        name: given.to_owned(),
        glyph: json!("program"),
    };
    let mut ready = crate::parts::served(records, &words(), &naming);
    ready.extend(more.iter().cloned());
    let served = crate::drawer::served(records, &ready, &[], &words(), &naming);
    let key = format!("drawer:sound:{name}");
    let drawer = served
        .into_iter()
        .find(|drawer| drawer["key"] == key.as_str())
        .unwrap_or_else(|| panic!("no drawer {key} served"));
    (ready, drawer)
}

/// The names of a drawer's facts, in its order.
fn named(drawer: &Value) -> Vec<String> {
    facts(drawer)
        .iter()
        .map(|fact| fact["name"].as_str().unwrap_or("").to_owned())
        .collect()
}

// A device's drawer is served beside its row, which names it, so how the
// device keeps time can change without the row; it says where the device
// is joined and which way it goes, over the name its row gives it.
#[test]
fn a_devices_drawer_is_served_beside_the_row_naming_it() {
    let (ready, drawer) = drawn(&[speakers()], &[], "speakers");
    assert_eq!(drawer["kind"], "drawer");
    assert_eq!(drawer["drawer"]["title"], "Speakers");
    assert_eq!(drawer["drawer"]["eyebrow"], "on this machine \u{b7} output");
    let row = ready
        .iter()
        .find(|row| row["key"] == "row:sound:speakers")
        .unwrap_or_else(|| panic!("no speakers row among {ready:#?}"));
    assert_eq!(
        row["drawer"], drawer["key"],
        "the row names its drawer: {row}"
    );
}

// What moves most leads, its State in PipeWire's words, and what it's
// known by comes last, each marked as an identifier: the ALSA Device it
// opens and its Graph ID; the Priority that picks it comes before them.
#[test]
fn a_devices_drawer_leads_with_its_state_and_ends_with_what_its_known_by() {
    let properties = json!({"device.api": "alsa", "priority.session": "1000",
        "api.alsa.path": "hw:0,0"});
    let mut record = device("speakers", true, "Speakers", &properties);
    record["id"] = json!(42);
    record["state"] = json!("running");
    let (_, drawer) = drawn(&[record], &[], "speakers");
    let said = lines(&drawer);
    assert_eq!(
        said.first().map(String::as_str),
        Some("State: running"),
        "{said:#?}"
    );
    let last: Vec<&str> = said
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(String::as_str)
        .collect();
    assert_eq!(
        last,
        ["Priority: 1000", "ALSA Device: hw:0,0", "Graph ID: 42"],
        "{said:#?}"
    );
    let marked: Vec<&Value> = facts(&drawer)
        .iter()
        .filter(|fact| fact["identifies"] == true)
        .collect();
    assert_eq!(
        marked.len(),
        2,
        "only the path and the id identify: {marked:#?}"
    );
}

// A device with nothing to say of its state, priority or path says none
// of them, rather than a name with nothing after it.
#[test]
fn a_device_saying_nothing_of_itself_leaves_it_out() {
    let (_, drawer) = drawn(&[speakers()], &[], "speakers");
    let names = named(&drawer);
    for left in ["State", "Priority", "ALSA Device", "Graph ID", "Format"] {
        assert!(
            !names.iter().any(|name| name == left),
            "{left} said: {names:?}"
        );
    }
}

// The format the graph moves its audio in is said once: as Format where
// the card says none, not at all where the card's playing says the same,
// and as Graph Format where the card's says another.
#[test]
fn the_graphs_format_is_said_once() {
    let mut record = speakers();
    record["format"] = json!({"rate": 48000, "sample": "S32LE"});
    let (_, drawer) = drawn(std::slice::from_ref(&record), &[], "speakers");
    let format = facts(&drawer)
        .iter()
        .find(|fact| fact["name"] == "Format")
        .unwrap_or_else(|| panic!("no Format in {:#?}", lines(&drawer)));
    let graph = format["said"].as_str().unwrap_or("").to_owned();
    assert!(graph.starts_with("48 kHz"), "the graph's format: {graph}");
    let playing = |said: &str| {
        json!({"key": "row:sound:speakers:fact:playing", "kind": "fact",
            "parent": "row:sound:speakers", "name": "Playing At",
            "side": [{"said": said}], "held": {"fact": "playing"}})
    };
    let (_, same) = drawn(
        std::slice::from_ref(&record),
        &[playing(&graph)],
        "speakers",
    );
    let names = named(&same);
    assert!(
        !names
            .iter()
            .any(|name| name == "Format" || name == "Graph Format"),
        "said again beside the card's: {names:?}"
    );
    let (_, other) = drawn(&[record], &[playing("44.1 kHz")], "speakers");
    let said = lines(&other);
    assert!(
        said.contains(&format!("Graph Format: {graph}")),
        "{said:#?}"
    );
    assert!(
        said.contains(&"Playing At: 44.1 kHz".to_owned()),
        "{said:#?}"
    );
}

// Where it's joined leads its eyebrow: over Bluetooth by the name the
// vocabulary gives it, by USB, through a display, or built in.
#[test]
fn a_devices_eyebrow_says_how_its_joined() {
    let joined = |name: &str, bus: &str| {
        let record = device(name, true, "Out", &json!({"device.bus": bus}));
        let (_, drawer) = drawn(&[record], &[], name);
        drawer["drawer"]["eyebrow"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    };
    assert_eq!(joined("headphones", "bluetooth"), "Bluetooth \u{b7} output");
    assert_eq!(joined("pebble", "usb"), "USB \u{b7} output");
    assert_eq!(
        joined("alsa_output.hdmi-stereo", "pci"),
        "display \u{b7} output"
    );
    assert_eq!(joined("speakers", "pci"), "built in \u{b7} output");
}

// Channels all at one level say they're balanced; otherwise each says its
// own, a stereo pair's without "front", since it has no back.
#[test]
fn channels_say_their_levels_balanced_or_each_its_own() {
    let owned = |all: &[&str]| -> Vec<String> { all.iter().map(|one| (*one).to_owned()).collect() };
    let pair = owned(&["Front left", "Front right"]);
    assert_eq!(
        channel_levels(&pair, &owned(&["80%", "80%"]), "balanced"),
        ["balanced"]
    );
    assert_eq!(
        channel_levels(&pair, &owned(&["100%", "80%"]), "balanced"),
        ["left 100%", "right 80%"]
    );
    let more = owned(&["Front left", "Front right", "Rear left"]);
    assert_eq!(
        channel_levels(&more, &owned(&["100%", "80%", "60%"]), "balanced"),
        ["front left 100%", "front right 80%", "rear left 60%"]
    );
    assert!(
        channel_levels(&[], &[], "balanced").is_empty(),
        "no channels, no levels"
    );
}
