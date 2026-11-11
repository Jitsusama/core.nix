//! The programs part's cases, apart from programs.rs so it keeps to its
//! length: which streams a device holds, how a program's row and its
//! parts say it, and where it can be sent.

use serde_json::{Value, json};

use super::*;
use crate::rows::tests::device;

fn words() -> Words {
    Words::from_vocabulary(
        &json!({"things": [
            {"id": "sound", "name": "Sound", "said": {
                "muted": "muted", "chosen": "chosen", "yes": "yes", "no": "no",
                "nothing": "nothing", "recording-none": "no program is listening",
                "paused": "paused", "paused-at": "paused at {level}", "the-default": "the default",
                "delay-note": "{frames} frames at {rate}, what it asks to be given at once",
            }},
            {"id": "sound:programs", "name": "Playing"},
            {"id": "sound:recorders", "name": "Recording"},
            {"id": "sound:mute", "name": "Muted", "verb": "mute",
                "states": {"muted": {"verb": "unmute"}}},
            {"id": "verb:program:mute", "name": "Muted", "glyph": "volume",
                "states": {"muted": {"glyph": "silent"}}},
            {"id": "verb:program:louder", "name": "Louder"},
            {"id": "verb:program:quieter", "name": "Quieter"},
            {"id": "sound:plays-on", "name": "Plays On"},
            {"id": "sound:records-from", "name": "Records From"},
            {"id": "verb:sound-sent", "name": "{device}"},
            {"id": "sound:rate", "name": "Sample Rate"},
            {"id": "sound:delay", "name": "Delay"},
        ]})
        .to_string(),
    )
    .expect("the words read")
}

/// A program's stream as the daemon's records say one, linked to `ends`.
fn stream(id: u32, sink: bool, program: &str, ends: &[&str]) -> Value {
    json!({
        "key": format!("stream:{id}"), "kind": "stream", "id": id,
        "name": format!("{program}-{id}"), "sink": sink, "volume": 0.5, "volumes": [0.125],
        "muted": false, "ends": ends, "target": "",
        "properties": {"application.name": program, "node.rate": "1/48000",
            "node.latency": "1024/48000"},
    })
}

/// Firefox by its desktop entry's name, the rest by the names they give.
fn named(given: &str) -> Named {
    let (name, glyph) = match given {
        "firefox" => ("Firefox", json!({"code": "e745"})),
        other => (other, json!("program")),
    };
    Named {
        name: name.to_owned(),
        glyph,
    }
}

/// Every row served for `name`'s programs among these records.
fn served(records: &[Value], name: &str) -> Vec<Value> {
    let devices: Vec<Device> = records.iter().filter_map(Device::of).collect();
    let device = devices
        .iter()
        .find(|device| device.name == name)
        .expect("the device is among the records");
    let sendable = Sendable {
        devices: devices
            .iter()
            .filter(|each| each.sink == device.sink && each.property("filter.smart") != "true")
            .map(|each| (each, each.name.to_owned()))
            .collect(),
    };
    let words = words();
    let around = Around {
        records,
        devices: &devices,
        sendable: &sendable,
        words: &words,
        naming: &named,
    };
    programs_part(device, &around, &format!("row:sound:{name}"))
}

fn keyed<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    let found = rows.iter().find(|row| row["key"] == key);
    found.unwrap_or_else(|| panic!("no row {key} among {rows:#?}"))
}

fn speakers_chosen() -> Value {
    let mut speakers = device("speakers", true, "Speakers", &json!({"device.api": "alsa"}));
    speakers["chosen"] = json!(true);
    speakers
}

// A program reaches the speakers through the tuning in front of them,
// so it plays through them; its two streams are one program's.
#[test]
fn a_program_through_a_filter_plays_through_its_device() {
    let tuning = device(
        "tuning",
        true,
        "Tuning",
        &json!({"filter.smart": "true", "filter.smart.target": "{ \"device.api\" = \"alsa\" }"}),
    );
    let records = [
        speakers_chosen(),
        tuning,
        stream(70, true, "firefox", &["tuning"]),
        stream(71, true, "firefox", &["speakers"]),
        stream(72, true, "pw-cat", &["speakers"]),
        stream(73, false, "recorder", &["microphone"]),
    ];
    let rows = served(&records, "speakers");
    let holder = keyed(&rows, "row:sound:speakers:programs");
    assert_eq!(holder["name"], "Playing", "the holder: {holder}");
    assert_eq!(
        holder["side"][0]["said"], "Firefox, pw-cat",
        "A to Z: {holder}"
    );
    let firefox = keyed(&rows, "row:sound:speakers:programs:program:Firefox");
    assert_eq!(firefox["held"]["streams"], json!([70, 71]), "{firefox}");
    assert_eq!(
        firefox["thing"], "sound:speakers:program:Firefox",
        "{firefox}"
    );
    assert_eq!(
        firefox["glyph"],
        json!({"code": "e745"}),
        "its own mark: {firefox}"
    );
}

// An output nothing plays through has nothing to hold; a microphone no
// program listens to says so.
#[test]
fn an_input_says_no_program_is_listening() {
    let records = [
        speakers_chosen(),
        device("microphone", false, "Microphone", &json!({})),
    ];
    assert_eq!(served(&records, "speakers"), Vec::<Value>::new());
    let rows = served(&records, "microphone");
    let none = keyed(&rows, "row:sound:microphone:recorders");
    let figures = (&none["side"][0]["said"], &none["note"]);
    assert_eq!(
        figures,
        (&json!("nothing"), &json!("no program is listening"))
    );
}

// A program's row says how loud it is, and paused where every stream is;
// muted, its switch says yes and flips to unmute, every stream at once.
#[test]
fn a_program_says_how_loud_it_is_and_acts_on_every_stream() {
    let mut first = stream(70, true, "firefox", &["speakers"]);
    let mut second = stream(71, true, "firefox", &["speakers"]);
    for one in [&mut first, &mut second] {
        one["properties"]["pulse.corked"] = json!("true");
    }
    second["volume"] = json!(0.8);
    let rows = served(
        &[speakers_chosen(), first.clone(), second.clone()],
        "speakers",
    );
    let key = "row:sound:speakers:programs:program:Firefox";
    assert_eq!(keyed(&rows, key)["side"][0]["said"], "80% \u{b7} paused");
    let holder = keyed(&rows, "row:sound:speakers:programs");
    let playing = &holder["held"]["playing"];
    assert_eq!(playing, "Firefox paused at 80%", "the drawer's Playing");
    let held = &keyed(&rows, key)["held"];
    assert_eq!(
        (&held["muted"], &held["volume"]),
        (&json!(false), &json!(0.8))
    );
    let louder = &keyed(&rows, &format!("{key}:louder"))["acts"][0]["ask"];
    let wanted = json!({"ask": "act", "act": "volume", "key": "program:speakers:Firefox",
        "streams": [70, 71], "volume": 0.85, "muted": false});
    assert_eq!(louder, &wanted, "louder, from the loudest");
    first["muted"] = json!(true);
    second["muted"] = json!(true);
    let rows = served(&[speakers_chosen(), first, second], "speakers");
    let mute = keyed(&rows, &format!("{key}:mute"));
    let stands = (
        &mute["side"][0]["said"],
        &mute["glyph"],
        &mute["acts"][0]["name"],
    );
    assert_eq!(stands, (&json!("yes"), &json!("silent"), &json!("Unmute")));
    assert_eq!(mute["acts"][0]["ask"]["muted"], false, "{mute}");
    assert_eq!(keyed(&rows, key)["side"][0]["said"], "muted \u{b7} paused");
}

// Plays On names the default and the device it follows; sent to one,
// that one is chosen, and each choice asks a send of every stream.
#[test]
fn a_program_plays_on_the_default_or_where_its_sent() {
    let headphones = device(
        "headphones",
        true,
        "Headphones",
        &json!({"object.serial": "61"}),
    );
    let firefox = stream(70, true, "firefox", &["speakers"]);
    let records = [speakers_chosen(), headphones.clone(), firefox.clone()];
    let rows = served(&records, "speakers");
    let key = "row:sound:speakers:programs:program:Firefox:plays-on";
    assert_eq!(
        keyed(&rows, key)["side"][0]["said"],
        "the default \u{b7} speakers"
    );
    let default = keyed(&rows, &format!("{key}:"));
    let says = (
        &default["name"],
        &default["note"],
        &default["side"][0]["said"],
    );
    assert_eq!(
        says,
        (&json!("the default"), &json!("speakers"), &json!("chosen"))
    );
    let ask = &keyed(&rows, &format!("{key}:headphones"))["acts"][0]["ask"];
    let wanted = json!({"ask": "act", "act": "send", "key": "program:speakers:Firefox",
        "streams": [70], "target": "headphones"});
    assert_eq!(ask, &wanted, "a send of every stream");
    let mut sent = firefox;
    sent["target"] = json!("61");
    let rows = served(&[speakers_chosen(), headphones, sent], "speakers");
    assert_eq!(
        keyed(&rows, key)["side"][0]["said"],
        "headphones",
        "sent by its serial"
    );
    let chosen = keyed(&rows, &format!("{key}:headphones"));
    assert_eq!(chosen["side"][0]["said"], "chosen", "{chosen}");
}

// Its first stream's Sample Rate and Delay, as Sound.qml said them.
#[test]
fn a_program_says_its_rate_and_delay() {
    let rows = served(
        &[speakers_chosen(), stream(70, true, "pw-cat", &["speakers"])],
        "speakers",
    );
    let key = "row:sound:speakers:programs:program:pw-cat";
    assert_eq!(
        keyed(&rows, &format!("{key}:rate"))["side"][0]["said"],
        "48 kHz"
    );
    let delay = keyed(&rows, &format!("{key}:delay"));
    let says = (&delay["side"][0]["said"], &delay["note"]);
    let note = "1024 frames at 48 kHz, what it asks to be given at once";
    assert_eq!(says, (&json!("21.3 ms"), &json!(note)));
}

#[test]
fn a_filters_target_is_read_as_sounds_js_reads_it() {
    let said = "{ \"media.class\" = \"Audio/Sink\", node.name: \"speakers\" }";
    let wanted = vec![("media.class", "Audio/Sink"), ("node.name", "speakers")];
    assert_eq!(aimed(said), wanted);
    assert_eq!(aimed("nothing aimed"), Vec::<(&str, &str)>::new());
}

#[test]
fn a_figure_keeps_three_significant_digits() {
    let said: Vec<String> = [21.333, 5.3333, 0.6667, 9.9996, 0.0]
        .into_iter()
        .map(significant)
        .collect();
    assert_eq!(said, ["21.3", "5.33", "0.667", "10.0", "0"]);
    assert_eq!(
        (kilohertz(44_100), kilohertz(48_000)),
        ("44.1 kHz".into(), "48 kHz".into())
    );
}
