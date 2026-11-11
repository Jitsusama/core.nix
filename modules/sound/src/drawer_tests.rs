//! A program's drawer's cases, apart from drawer.rs so it keeps to its
//! length: what it says of each stream, in what order, and how a stream
//! keeps time once the profiler says it.

use serde_json::{Value, json};

use super::*;
use crate::programs::Named;
use crate::rows::tests::{device, speakers_chosen as speakers, words};

/// A program's stream as the daemon's records say one, linked to `ends`.
fn stream(id: u32, program: &str, volume: f64, properties: &Value) -> Value {
    let mut said = json!({"application.name": program});
    if let (Some(said), Some(more)) = (said.as_object_mut(), properties.as_object()) {
        said.extend(more.clone());
    }
    json!({
        "key": format!("stream:{id}"), "kind": "stream", "id": id,
        "name": format!("{program}-{id}"), "sink": true, "volume": volume, "volumes": [volume],
        "muted": false, "ends": ["speakers"], "target": "", "properties": said,
    })
}

fn named(given: &str) -> Named {
    Named {
        name: if given == "firefox" { "Firefox" } else { given }.to_owned(),
        glyph: json!("program"),
    }
}

/// The drawers served of these records, with these timings.
fn drawers(records: &[Value], timings: &[Value]) -> Vec<Value> {
    served(records, timings, &words(), &named)
}

fn facts(drawer: &Value) -> &Vec<Value> {
    drawer["drawer"]["facts"]
        .as_array()
        .unwrap_or_else(|| panic!("no facts in {drawer}"))
}

/// A fact's name and what it says, or its heading, each as one line.
fn lines(drawer: &Value) -> Vec<String> {
    facts(drawer)
        .iter()
        .map(|fact| match fact["heading"].as_str() {
            Some(heading) => format!("# {heading}"),
            None => format!("{}: {}", fact["name"], fact["said"]).replace('"', ""),
        })
        .collect()
}

// A program's drawer is served beside its row, which names it, so how it
// keeps time can change without the row; it says where it plays and that
// it's a stream, set as written.
#[test]
fn a_programs_drawer_is_served_beside_the_row_naming_it() {
    let records = [speakers(), stream(70, "firefox", 0.5, &json!({}))];
    let served = drawers(&records, &[]);
    assert_eq!(served.len(), 1, "one program, one drawer: {served:#?}");
    let drawer = &served[0];
    assert_eq!(drawer["key"], "drawer:sound:speakers:program:Firefox");
    assert_eq!(drawer["kind"], "drawer");
    assert_eq!(drawer["drawer"]["title"], "Firefox");
    assert_eq!(drawer["drawer"]["eyebrow"], "on Speakers \u{b7} stream");
    assert_eq!(drawer["drawer"]["eyebrowNames"], true);
    let parts = crate::parts::served(&records, &words(), &named);
    let row = parts
        .iter()
        .find(|row| row["key"] == "row:sound:speakers:programs:program:Firefox")
        .unwrap_or_else(|| panic!("no program row among {parts:#?}"));
    assert_eq!(
        row["drawer"], drawer["key"],
        "the row names its drawer: {row}"
    );
}

// A stream alone says what it plays as its Stream, its Format, its Delay,
// its Role and what identifies it; its Sample Rate gives way to a Format
// opening with it.
#[test]
fn a_stream_alone_says_what_it_plays_and_its_figures() {
    let mut playing = stream(
        70,
        "firefox",
        0.5,
        &json!({"media.name": "Song", "node.rate": "1/48000", "node.latency": "1024/48000",
            "media.role": "Music", "application.process.id": "1234", "client.id": "55"}),
    );
    playing["format"] = json!({"rate": 48_000, "sample": "F32LE"});
    let served = drawers(&[speakers(), playing], &[]);
    assert_eq!(
        lines(&served[0]),
        [
            "Stream: Song",
            "Format: 48 kHz \u{b7} 32 bit float",
            "Delay: 21.3 ms",
            "Role: Music",
            "Process ID: 1234",
            "Graph ID: 70",
            "Client ID: 55",
        ]
    );
    let identifying: Vec<&Value> = facts(&served[0])
        .iter()
        .filter(|fact| fact["identifies"] == true)
        .map(|fact| &fact["name"])
        .collect();
    assert_eq!(identifying, ["Process ID", "Graph ID", "Client ID"]);
}

// A rate a Format doesn't open with is said, as a stream resampled is;
// what plays saying the program's own name, or pw-cat's "-", says
// nothing.
#[test]
fn a_rate_the_format_doesnt_open_with_is_said() {
    let mut playing = stream(
        70,
        "firefox",
        0.5,
        &json!({"media.name": "Firefox", "node.rate": "1/44100"}),
    );
    playing["format"] = json!({"rate": 48_000, "sample": "S16LE"});
    let served = drawers(&[speakers(), playing], &[]);
    assert_eq!(
        lines(&served[0]),
        [
            "Format: 48 kHz \u{b7} 16 bit",
            "Sample Rate: 44.1 kHz",
            "Graph ID: 70"
        ]
    );
}

// Of several streams, the loudest comes first, then the newest; each is
// headed by what it plays, or its own name where what it plays says
// nothing.
#[test]
fn several_streams_are_headed_loudest_then_newest_first() {
    let records = [
        speakers(),
        stream(70, "firefox", 0.5, &json!({"media.name": "-"})),
        stream(71, "firefox", 0.8, &json!({"media.name": "Video"})),
        stream(72, "firefox", 0.5, &json!({"media.name": "Call"})),
    ];
    let served = drawers(&records, &[]);
    let headings: Vec<&Value> = facts(&served[0])
        .iter()
        .filter_map(|fact| fact.get("heading"))
        .collect();
    assert_eq!(headings, ["Video", "Call", "firefox-70"]);
}

// A stream that names nothing, neither what it plays nor itself, has no
// heading among several, rather than an empty one.
#[test]
fn a_stream_naming_nothing_has_no_heading() {
    let mut nameless = stream(70, "firefox", 0.5, &json!({}));
    nameless["name"] = json!("");
    let records = [
        speakers(),
        nameless,
        stream(71, "firefox", 0.8, &json!({"media.name": "Video"})),
    ];
    let served = drawers(&records, &[]);
    let headings: Vec<&Value> = facts(&served[0])
        .iter()
        .filter_map(|fact| fact.get("heading"))
        .collect();
    assert_eq!(headings, ["Video"]);
}

// A stream through the tuning in front of the device says it Plays On the
// tuning; one straight to the device says nothing of where, since the
// eyebrow names it.
#[test]
fn a_stream_through_a_filter_says_it_plays_on_the_filter() {
    let tuning = device(
        "tuning",
        true,
        "Tuning",
        &json!({"filter.smart": "true", "filter.smart.target": "{ \"device.api\" = \"alsa\" }"}),
    );
    let mut through = stream(70, "firefox", 0.5, &json!({}));
    through["ends"] = json!(["tuning"]);
    let served = drawers(&[speakers(), tuning, through], &[]);
    assert_eq!(lines(&served[0]), ["Plays On: Tuning", "Graph ID: 70"]);
}

// While the profiler says how a stream keeps time, its Dropouts, Load and
// Cycle lead; dropouts past none warn, and so does a cycle it didn't
// finish in, said as over the whole of it.
#[test]
fn a_stream_timed_says_its_dropouts_load_and_cycle() {
    let records = [speakers(), stream(70, "firefox", 0.5, &json!({}))];
    let timed = |busy: Value, unfinished: bool, errors: u32| {
        json!({"key": "timing:70", "kind": "timing", "id": 70, "quantum": 1024,
            "rate": 48_000, "ms": 21.333, "busy": busy, "unfinished": unfinished,
            "errors": errors, "drives": false})
    };
    let served = drawers(&records, &[timed(json!(0.0312), false, 0)]);
    assert_eq!(
        lines(&served[0])[..3],
        [
            "Dropouts: none",
            "Load: 3.12% of each cycle",
            "Cycle: 1,024 frames \u{b7} 21.3 ms"
        ]
    );
    let load = &facts(&served[0])[1];
    assert_eq!(
        load["figure"],
        json!({"shape": "stroke", "values": [0.0312], "warn": false})
    );
    assert_eq!(load.get("hue"), None, "a load within the cycle warns not");
    let served = drawers(&records, &[timed(Value::Null, true, 2)]);
    let said = &facts(&served[0])[..2];
    assert_eq!(said[0]["said"], "2");
    assert_eq!(said[0]["hue"], "warning", "dropouts warn: {said:?}");
    assert_eq!(said[1]["said"], "over 100% of each cycle");
    assert_eq!(said[1]["hue"], "warning", "an unfinished cycle warns");
}
