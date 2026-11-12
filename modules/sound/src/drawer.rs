//! A program's drawer, served beside its row as a `drawer` record the row
//! names by key (ready-rows.md's drawers), so what moves most in it, how
//! its streams keep time each second, changes the drawer alone and never
//! a row. It says what Sound.qml's `programPlate` said (VKCF3Z, 2026-11-08
//! and 2026-11-09): each of the program's streams on the device, loudest
//! then newest first (by its graph ID, which PipeWire gives out rising but
//! reuses, so newest only roughly), a group headed by what it plays where
//! there's more than one, or that said as its Stream where there's one.
//! Under it, how it keeps time, while a watch wants timings; its Format;
//! its Sample Rate, only where its Format doesn't open with it, and its
//! Delay; its Role, in PipeWire's word as given; the filter it plays
//! through or records from, where one stands in front of the device; and
//! what identifies it, ending its group: its Process ID, Graph ID and
//! Client ID. Its name is the row's, and so is whether it's paused or
//! muted, so neither is said again; its eyebrow says where it plays and
//! that it's a stream, set as written since it names the device.

use std::cmp::Ordering;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::programs::{Naming, Program, aims, fraction, kilohertz, programs};
use crate::rows::{APART, Device, PLACE, Words, named, rows};

/// A share as a percentage, and milliseconds in a second.
const PERCENT: f64 = 100.0;
const MILLISECONDS: f64 = 1000.0;

/// The key of a program's drawer on a device, which its row names.
pub(crate) fn key(device: &Device, program: &str) -> String {
    format!("drawer:{PLACE}:{}:program:{program}", device.name)
}

/// What a drawer is made from, besides the program it's of.
struct Around<'a> {
    devices: &'a [Device<'a>],
    timings: &'a [Value],
    words: &'a Words,
    numbers: Numbers,
}

/// The drawer of every program playing through a device Sound lists, or
/// recording from one, worded from `words`, with how each stream keeps
/// time where `timings` says it.
pub fn served(records: &[Value], timings: &[Value], words: &Words, naming: Naming) -> Vec<Value> {
    let devices: Vec<Device> = records.iter().filter_map(Device::of).collect();
    let around = Around {
        devices: &devices,
        timings,
        words,
        numbers: Numbers::default(),
    };
    let mut drawers = Vec::new();
    for top in rows(records, words) {
        let Some(device) = devices.iter().find(|one| top["held"]["node"] == one.name) else {
            continue;
        };
        let name = top["name"].as_str().unwrap_or(device.name);
        for program in programs(device, records, &devices, naming) {
            drawers.push(drawer(device, name, &program, &around));
        }
    }
    drawers
}

fn drawer(device: &Device, name: &str, program: &Program, around: &Around) -> Value {
    let words = around.words;
    let way = if device.sink {
        "stream-on"
    } else {
        "stream-from"
    };
    let eyebrow = [
        words.said(way, &[("device", name)]),
        words.said("plate-stream", &[]),
    ]
    .join(APART);
    let loudness = |stream: &&Value| stream["volume"].as_f64().unwrap_or(0.0);
    let id = |stream: &&Value| stream["id"].as_u64().unwrap_or(0);
    let mut streams = program.streams.clone();
    streams.sort_by(|a, b| {
        loudness(b)
            .partial_cmp(&loudness(a))
            .unwrap_or(Ordering::Equal)
            .then_with(|| id(b).cmp(&id(a)))
    });
    let headed = streams.len() > 1;
    let facts: Vec<Value> = streams
        .iter()
        .flat_map(|stream| stream_facts(device, stream, headed, &program.name, around))
        .collect();
    json!({
        "key": key(device, &program.name),
        "kind": "drawer",
        "drawer": {"eyebrow": eyebrow, "eyebrowNames": true, "title": program.name,
            "facts": facts},
    })
}

/// A fact named by the vocabulary's `sound:<id>`, saying `said`.
fn fact(words: &Words, id: &str, said: &str) -> Value {
    json!({"name": words.name(&format!("sound:{id}"), &[]), "said": said})
}

/// What a stream plays says something only where it isn't empty, "-"
/// (pw-cat's, playing what it's given), or the program's own name again
/// (VKCF3Z, 2026-11-09); a group goes by the stream's own name then.
fn stream_facts(
    device: &Device,
    stream: &Value,
    headed: bool,
    program: &str,
    around: &Around,
) -> Vec<Value> {
    let words = around.words;
    let property = |key: &str| stream["properties"][key].as_str().unwrap_or("");
    let given = property("media.name").trim();
    let own = [program, property("application.name")];
    let same = |name: &&str| name.to_lowercase() == given.to_lowercase();
    let plays = if given == "-" || own.iter().any(same) {
        ""
    } else {
        given
    };
    let called = if plays.is_empty() {
        stream["name"].as_str().unwrap_or("")
    } else {
        plays
    };
    let mut facts = Vec::new();
    if headed && !called.is_empty() {
        facts.push(json!({"heading": called}));
    } else if !headed && !plays.is_empty() {
        facts.push(fact(words, "stream", plays));
    }
    facts.extend(keeping(stream, around));
    let format = graph_format(&stream["format"], &around.numbers);
    if !format.is_empty() {
        facts.push(fact(words, "format", &format));
    }
    if let Some((1, hertz)) = fraction(property("node.rate")) {
        let rate = kilohertz(hertz);
        let opens = format == rate || format.starts_with(&format!("{rate}{APART}"));
        if !opens {
            facts.push(fact(words, "rate", &rate));
        }
    }
    if let Some((frames, hertz)) = fraction(property("node.latency")) {
        let ms = f64::from(frames) / f64::from(hertz) * MILLISECONDS;
        let delay = format!("{} ms", around.numbers.significant(ms));
        facts.push(fact(words, "delay", &delay));
    }
    let end = if device.sink {
        "plays-on"
    } else {
        "records-from"
    };
    let described = [
        ("role", property("media.role").to_owned()),
        (end, filter_of(stream, device, around)),
    ];
    let id = stream["id"]
        .as_u64()
        .map_or_else(String::new, |id| id.to_string());
    let identifying = [
        ("process", property("application.process.id").to_owned()),
        ("graph-id", id),
        ("client", property("client.id").to_owned()),
    ];
    for (id, said) in described.iter().filter(|(_, said)| !said.is_empty()) {
        facts.push(fact(words, id, said));
    }
    for (id, said) in identifying.iter().filter(|(_, said)| !said.is_empty()) {
        let mut identifies = fact(words, id, said);
        identifies["identifies"] = json!(true);
        facts.push(identifies);
    }
    facts
}

/// How a stream keeps time, as the profiler says it while a watch wants
/// timings: its Dropouts, warning past none; its Load, how much of each
/// cycle it's busy, with a stroke beside it, and over the whole of it,
/// warning, where it hadn't finished; and its Cycle. None while its time
/// isn't told.
fn keeping(stream: &Value, around: &Around) -> Vec<Value> {
    let (words, numbers) = (around.words, &around.numbers);
    let Some(timed) = around
        .timings
        .iter()
        .find(|timed| timed["id"] == stream["id"])
    else {
        return Vec::new();
    };
    let count = |field: &str| numbers.number(timed[field].as_f64().unwrap_or(0.0), 0);
    let errors = timed["errors"].as_u64().unwrap_or(0);
    let mut dropouts = fact(
        words,
        "dropouts",
        &if errors == 0 {
            words.said("none", &[])
        } else {
            count("errors")
        },
    );
    if errors > 0 {
        dropouts["hue"] = json!("warning");
    }
    let mut facts = vec![dropouts];
    let share = |value: f64| format!("{}%", numbers.significant(value * PERCENT));
    if timed["unfinished"] == true {
        let over = words.said("over", &[("share", &share(1.0))]);
        let mut load = fact(
            words,
            "load",
            &words.said("each-cycle", &[("share", &over)]),
        );
        load["hue"] = json!("warning");
        load["figure"] = json!({"shape": "stroke", "values": [1], "warn": true});
        facts.push(load);
    } else if let Some(busy) = timed["busy"].as_f64() {
        let said = words.said("each-cycle", &[("share", &share(busy))]);
        let mut load = fact(words, "load", &said);
        load["figure"] = json!({"shape": "stroke", "values": [busy.min(1.0)], "warn": false});
        facts.push(load);
    }
    let ms = numbers.significant(timed["ms"].as_f64().unwrap_or(0.0));
    let cycle = words.said("cycle", &[("frames", &count("quantum")), ("ms", &ms)]);
    facts.push(fact(words, "cycle", &cycle));
    facts
}

/// The filter in front of the device a stream is linked to, by the
/// filter's name; "" while it's linked to the device itself, which the
/// eyebrow names, or to neither.
fn filter_of(stream: &Value, device: &Device, around: &Around) -> String {
    let linked = |filter: &&Device| {
        stream["ends"]
            .as_array()
            .is_some_and(|ends| ends.iter().any(|end| end == filter.name))
    };
    around
        .devices
        .iter()
        .filter(|filter| aims(filter, device))
        .find(linked)
        .map_or_else(String::new, |filter| named(filter, &[], around.words))
}

/// The format the sound server moves a stream's audio in, "48 kHz · 32
/// bit float", its rate to three significant digits, as Sounds.js's
/// `graphFormat`; "" while it has none. Its channels are said elsewhere.
fn graph_format(format: &Value, numbers: &Numbers) -> String {
    let Some(hertz) = format["rate"].as_f64().filter(|hertz| *hertz > 0.0) else {
        return String::new();
    };
    let kilohertz = hertz / MILLISECONDS;
    // To three significant digits, as JavaScript's toPrecision(3) rounds.
    let rounded = format!("{kilohertz:.2e}")
        .parse::<f64>()
        .unwrap_or(kilohertz);
    let mut said = vec![format!("{} kHz", numbers.number(rounded, 3))];
    let sample = format["sample"].as_str().unwrap_or("");
    if !sample.is_empty() {
        said.push(sampled(sample));
    }
    said.join(APART)
}

/// A sample format's short name as words, as Sounds.js's `sample`: `F32LE`
/// is "32 bit float", `S24_32LE` "24 bit"; a name it can't read stays.
fn sampled(name: &str) -> String {
    let mut chars = name.chars();
    let kind = chars.next().filter(|kind| "SUF".contains(*kind));
    let bits: String = chars.take_while(char::is_ascii_digit).collect();
    match kind {
        Some(kind) if !bits.is_empty() => {
            let float = if kind == 'F' { " float" } else { "" };
            format!("{bits} bit{float}")
        }
        _ => name.to_owned(),
    }
}

#[cfg(test)]
#[path = "drawer_tests.rs"]
mod tests;
