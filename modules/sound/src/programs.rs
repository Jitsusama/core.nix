//! The programs Playing through an output, or Recording from an input,
//! served inside the device's row as Sound.qml's `programsPart` made them:
//! a row holding each program, its streams on the device grouped by the
//! program they're of, A to Z by name; inside each, whether it's Muted,
//! Louder and Quieter, where it Plays On or Records From, and its first
//! stream's Sample Rate and Delay. A program's acts ask the daemon with
//! its streams' ids (acts.rs), so the shell runs nothing to do them.
//!
//! A program is named, and wears the mark, its desktop entry gives it,
//! as the daemon finds one (`windows::entries`, `windows::marks`); a stream
//! counts for a device when it's linked to it, or to a filter in front
//! of it, where programs reach the device while the filter's there.

use serde_json::{Value, json};

use crate::parts::{STEP, capitalised, stepped};
use crate::rows::{APART, Device, PLACE, Words, bar, kind, percent};

/// What a program is called, and the glyph it wears: its own mark, or
/// the launcher's mark for a program where none is known.
pub struct Named {
    pub name: String,
    pub glyph: Value,
}

/// How the daemon names a program, by the name its stream gives.
pub type Naming<'a> = &'a dyn Fn(&str) -> Named;

/// Milliseconds in a second, for a stream's Delay, and hertz in a
/// kilohertz, for its rate.
const MILLISECONDS: f64 = 1000.0;

/// How many figures a Delay is said to, as Numbers.js's `significant`.
const SIGNIFICANT: i32 = 3;

/// One program's streams on a device, as its row says them.
struct Program<'a> {
    name: String,
    glyph: Value,
    streams: Vec<&'a Value>,
}

impl<'a> Program<'a> {
    fn ids(&self) -> Vec<&Value> {
        self.streams.iter().map(|stream| &stream["id"]).collect()
    }

    /// The loudest of its streams, as the shell shows a volume.
    fn loudest(&self) -> f64 {
        self.streams
            .iter()
            .filter_map(|stream| stream["volume"].as_f64())
            .fold(0.0, f64::max)
    }

    fn hushed(&self) -> bool {
        self.streams.iter().all(|stream| stream["muted"] == true)
    }

    fn paused(&self) -> bool {
        self.streams
            .iter()
            .all(|stream| stream["properties"]["pulse.corked"] == "true")
    }

    /// Where its streams are sent: the one target they share, or "" where
    /// they share none, as Targets.js's `sentTo`.
    fn sent(&self) -> &'a str {
        fn target(stream: &Value) -> &str {
            stream["target"].as_str().unwrap_or("")
        }
        let first = self.streams.first().map_or("", |stream| target(stream));
        if self.streams.iter().all(|stream| target(stream) == first) {
            first
        } else {
            ""
        }
    }
}

/// What a smart filter's `filter.smart.target` asks of a device, as
/// Sounds.js's `target` reads it: each `"key" = "value"` or `key: "value"`.
fn aimed(text: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < text.len() {
        match aimed_at(text, at) {
            Some((key, value, end)) => {
                found.push((key, value));
                at = end;
            }
            None => at += text[at..].chars().next().map_or(1, char::len_utf8),
        }
    }
    found
}

/// One `"key" = "value"` starting at `at`, and where it ends.
fn aimed_at(text: &str, at: usize) -> Option<(&str, &str, usize)> {
    let keyed = |c: char| c.is_ascii_alphanumeric() || "_.-".contains(c);
    let rest = &text[at..];
    let start = usize::from(rest.starts_with('"'));
    let length = rest[start..]
        .find(|c: char| !keyed(c))
        .unwrap_or(rest.len() - start);
    if length == 0 {
        return None;
    }
    let key = &rest[start..start + length];
    let after = rest[start + length..]
        .strip_prefix('"')
        .unwrap_or(&rest[start + length..]);
    let after = after.trim_start().strip_prefix(['=', ':'])?.trim_start();
    let quoted = after.strip_prefix('"')?;
    let value = &quoted[..quoted.find('"')?];
    let end = text.len() - quoted.len() + value.len() + 1;
    Some((key, value, end))
}

/// Whether a filter stands in front of a device: one that puts itself
/// there, aimed at it by what it asks a device to be, every one of which
/// the device is. A target asking nothing aims at nothing.
fn aims(filter: &Device, device: &Device) -> bool {
    let wanted = aimed(filter.property("filter.smart.target"));
    filter.sink == device.sink
        && filter.property("filter.smart") == "true"
        && !wanted.is_empty()
        && wanted
            .iter()
            .all(|(key, value)| device.property(key) == *value)
}

/// The programs whose streams play through a device, or record from it,
/// A to Z by name, each stream in the order the server numbered them.
fn programs<'a>(
    device: &Device,
    records: &'a [Value],
    devices: &[Device],
    naming: Naming,
) -> Vec<Program<'a>> {
    let mut ends = vec![device.name];
    ends.extend(
        devices
            .iter()
            .filter(|filter| aims(filter, device))
            .map(|filter| filter.name),
    );
    let reaches = |stream: &Value| {
        stream["ends"]
            .as_array()
            .is_some_and(|linked| linked.iter().any(|end| ends.iter().any(|one| end == one)))
    };
    let mut found: Vec<Program> = Vec::new();
    for stream in records {
        if stream["kind"] != "stream" || stream["sink"] != device.sink || !reaches(stream) {
            continue;
        }
        let given = stream["properties"]["application.name"]
            .as_str()
            .or_else(|| stream["name"].as_str())
            .unwrap_or("");
        let named = naming(given);
        match found.iter_mut().find(|program| program.name == named.name) {
            Some(program) => program.streams.push(stream),
            None => found.push(Program {
                name: named.name,
                glyph: named.glyph,
                streams: vec![stream],
            }),
        }
    }
    found.sort_by_key(|program| program.name.to_lowercase());
    found
}

/// The devices a program's streams could go to, by their rows' names, in
/// Sound's order, and the one chosen, where they'd go by default.
pub(crate) struct Sendable<'a> {
    pub(crate) devices: Vec<(&'a Device<'a>, String)>,
}

impl Sendable<'_> {
    fn usual(&self) -> Option<&(&Device<'_>, String)> {
        self.devices.iter().find(|(device, _)| device.chosen())
    }

    /// The one a target names, by either name `WirePlumber` takes, as
    /// Targets.js's `names`.
    fn named(&self, target: &str) -> Option<&(&Device<'_>, String)> {
        self.devices.iter().find(|(device, _)| {
            !target.is_empty()
                && (device.name == target || device.property("object.serial") == target)
        })
    }
}

/// What's needed to serve a device's programs.
pub(crate) struct Around<'a> {
    pub(crate) records: &'a [Value],
    pub(crate) devices: &'a [Device<'a>],
    pub(crate) sendable: &'a Sendable<'a>,
    pub(crate) words: &'a Words,
    pub(crate) naming: Naming<'a>,
}

/// A row as Sound serves one: these fields, and those every row has where
/// they don't say them.
fn row(fields: Value) -> Value {
    let mut row = json!({"place": PLACE, "hue": "machine", "side": [], "note": "", "acts": []});
    if let (Some(row), Value::Object(fields)) = (row.as_object_mut(), fields) {
        row.extend(fields);
    }
    row
}

fn said(side: &str) -> Value {
    json!([{"said": side}])
}

/// The programs part of a device, and every row inside it: none for an
/// output nothing plays through, which has nothing to hold, and a figure
/// saying no program is listening for an input, since that's worth
/// saying of a microphone.
pub(crate) fn programs_part(device: &Device, around: &Around, parent: &str) -> Vec<Value> {
    let words = around.words;
    let id = if device.sink { "programs" } else { "recorders" };
    let part = format!("{parent}:{id}");
    let thing = format!("sound:{}:{id}", device.name);
    let found = programs(device, around.records, around.devices, around.naming);
    let name = words.name(&format!("sound:{id}"), &[]);
    if found.is_empty() {
        if device.sink {
            return Vec::new();
        }
        let nothing = words.said("nothing", &[]);
        return vec![row(json!({
            "key": part, "thing": thing, "kind": "figure", "parent": parent, "depth": 1,
            "name": name, "glyph": "", "side": said(&nothing),
            "note": words.said("recording-none", &[]),
            "acts": [{"key": "verb:copy", "copy": nothing}],
            "held": {"node": device.name},
        }))];
    }
    let names: Vec<&str> = found.iter().map(|program| program.name.as_str()).collect();
    // How loud each plays, a run apart, "Spotify 100% · Chrome paused at
    // 45%", which the device's drawer says as its Playing.
    let playing: Vec<String> = found
        .iter()
        .map(|program| {
            let level = level(program, words);
            let said = if program.paused() {
                words.said("paused-at", &[("level", &level)])
            } else {
                level
            };
            format!("{} {said}", program.name)
        })
        .collect();
    let mut rows = vec![row(json!({
        "key": part, "thing": thing, "kind": "programs", "parent": parent, "depth": 1,
        "name": name, "glyph": kind(device), "side": said(&names.join(", ")),
        "acts": [{"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name, "playing": playing.join(APART)},
    }))];
    for program in &found {
        rows.extend(program_rows(device, program, around, &part));
    }
    rows
}

/// How loud a program plays: its loudest stream's level, or muted.
fn level(program: &Program, words: &Words) -> String {
    if program.hushed() {
        words.said("muted", &[])
    } else {
        percent(program.loudest())
    }
}

/// A program's row, and inside it what it can be told and what it says.
fn program_rows(device: &Device, program: &Program, around: &Around, parent: &str) -> Vec<Value> {
    let words = around.words;
    let key = format!("{parent}:program:{}", program.name);
    let thing = format!("sound:{}:program:{}", device.name, program.name);
    let hushed = program.hushed();
    let level = level(program, words);
    let side = if program.paused() {
        format!("{level}{APART}{}", words.said("paused", &[]))
    } else {
        level
    };
    // How it stands, for :setlocal's mute and volume to say.
    let held = json!({"node": device.name, "program": program.name, "streams": program.ids(),
        "sink": device.sink, "muted": hushed, "volume": program.loudest()});
    let mut rows = vec![row(json!({
        "key": key, "thing": thing, "kind": "program", "parent": parent, "depth": 2,
        "name": program.name, "glyph": program.glyph, "side": said(&side),
        "graph": bar(if hushed { 0.0 } else { program.loudest() }),
        "acts": [{"key": "verb:more", "step": "rows"}], "held": held,
    }))];
    let asked = |act: &str, more: &Value| {
        let mut asked = json!({"ask": "act", "act": act,
            "key": format!("program:{}:{}", device.name, program.name), "streams": program.ids()});
        if let (Some(asked), Some(more)) = (asked.as_object_mut(), more.as_object()) {
            asked.extend(more.clone());
        }
        asked
    };
    let state = if hushed { "muted" } else { "" };
    let flips = capitalised(&words.verb_in("sound:mute", state));
    rows.push(row(json!({
        "key": format!("{key}:mute"), "thing": format!("{thing}:mute"), "kind": "switch",
        "parent": key, "depth": 3,
        "name": words.name("sound:mute", &[]),
        "glyph": words.glyph_in("verb:program:mute", state),
        "hue": "action",
        "side": said(&words.said(if hushed { "yes" } else { "no" }, &[])),
        "acts": [{"key": "verb:program:mute", "name": flips,
            "ask": asked("mute", &json!({"muted": !hushed}))}],
        "held": {"node": device.name, "muted": hushed},
    })));
    // Louder is meant to be heard, so it unmutes, as a level set does.
    for (by, verb, unmutes) in [(STEP, "louder", true), (-STEP, "quieter", false)] {
        let named = format!("verb:program:{verb}");
        let mut more = json!({"volume": stepped(program.loudest(), by)});
        if unmutes {
            more["muted"] = json!(false);
        }
        rows.push(row(json!({
            "key": format!("{key}:{verb}"), "thing": named, "kind": "act", "parent": key,
            "depth": 3, "name": words.name(&named, &[]), "glyph": "volume", "hue": "action",
            "acts": [{"key": named, "ask": asked("volume", &more)}],
            "held": {"node": device.name},
        })));
    }
    rows.extend(sent_rows(device, program, around, &key, &thing, &asked));
    if let Some(first) = program.streams.first() {
        rows.extend(figures(first, words, &key, &thing));
    }
    rows
}

/// A program's Plays On, or Records From for one recording: the device
/// it's on, stepping in to the default and each device it could go to
/// instead (VKCF3Z, 2026-11-09), where there's another to go to. What's
/// chosen is the device it's sent to, or the default and the device that
/// is, so a stream sent somewhere is told from one following the default.
fn sent_rows(
    device: &Device,
    program: &Program,
    around: &Around,
    parent: &str,
    thing: &str,
    asked: &dyn Fn(&str, &Value) -> Value,
) -> Vec<Value> {
    let (words, among) = (around.words, around.sendable);
    if among.devices.len() < 2 {
        return Vec::new();
    }
    let id = if device.sink {
        "plays-on"
    } else {
        "records-from"
    };
    let state = if device.sink { "" } else { "input" };
    let usual = among.usual();
    let pinned = among.named(program.sent());
    let the_default = words.said("the-default", &[]);
    let side = match (pinned, usual) {
        (Some((_, name)), _) => name.clone(),
        (None, Some((_, usual))) => format!("{the_default}{APART}{usual}"),
        (None, None) => the_default.clone(),
    };
    let part = format!("{parent}:{id}");
    let mut rows = vec![row(json!({
        "key": part, "thing": format!("{thing}:{id}"), "kind": "setting", "parent": parent,
        "depth": 3, "name": words.name(&format!("sound:{id}"), &[]), "glyph": kind(device),
        "side": said(&side), "acts": [{"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name},
    }))];
    let chosen = said(&words.said("chosen", &[]));
    let mut choice = |to: Option<&(&Device, String)>| {
        let (name, value, target) = match to {
            Some((device, name)) => (name.clone(), name.clone(), device.name),
            None => (the_default.clone(), "default".to_owned(), ""),
        };
        let wears = to.or(usual).map_or("", |(device, _)| kind(device));
        let is_chosen = match (to, pinned) {
            (Some((one, _)), Some((other, _))) => one.name == other.name,
            (None, None) => true,
            _ => false,
        };
        let note = match (to, usual) {
            (None, Some((_, usual))) => usual.clone(),
            _ => String::new(),
        };
        let side = if is_chosen { chosen.clone() } else { json!([]) };
        let sends = asked("send", &json!({"target": target}));
        rows.push(row(json!({
            "key": format!("{part}:{target}"), "thing": format!("verb:sound-{id}:{target}"),
            "kind": "send", "parent": part, "depth": 4,
            "name": words.name_in("verb:sound-sent", state, &[("device", &name)]),
            "glyph": wears, "hue": "action", "side": side, "note": note,
            "acts": [{"key": "verb:sound-sent", "ask": sends}],
            "held": {"node": device.name, "program": program.name, "streams": program.ids(),
                "to": target, "fill": {"device": name, "value": value}, "state": state},
        })));
    };
    choice(None);
    for to in &among.devices {
        choice(Some(to));
    }
    rows
}

/// A stream's Sample Rate, as it asks for one, and its Delay: how long it
/// waits first, a measure by the one rule, and how that's asked for, in
/// its note.
fn figures(stream: &Value, words: &Words, parent: &str, thing: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    let figure = |id: &str, side: &str, note: String| {
        row(json!({
            "key": format!("{parent}:{id}"), "thing": format!("{thing}:{id}"),
            "kind": "figure", "parent": parent, "depth": 3,
            "name": words.name(&format!("sound:{id}"), &[]), "glyph": "", "side": said(side),
            "note": note, "acts": [{"key": "verb:copy", "copy": side}],
        }))
    };
    let property = |key: &str| stream["properties"][key].as_str().unwrap_or("");
    if let Some((1, hertz)) = fraction(property("node.rate")) {
        rows.push(figure("rate", &kilohertz(hertz), String::new()));
    }
    if let Some((frames, hertz)) = fraction(property("node.latency")) {
        let ms = f64::from(frames) / f64::from(hertz) * MILLISECONDS;
        let (frames, rate) = (frames.to_string(), kilohertz(hertz));
        let note = words.said("delay-note", &[("frames", &frames), ("rate", &rate)]);
        rows.push(figure("delay", &format!("{} ms", significant(ms)), note));
    }
    rows
}

/// What a stream's "frames/rate" says, or none where it says no rate.
fn fraction(text: &str) -> Option<(u32, u32)> {
    let (frames, hertz) = text.split_once('/')?;
    let (frames, hertz) = (frames.trim().parse().ok()?, hertz.trim().parse().ok()?);
    (hertz > 0).then_some((frames, hertz))
}

/// A rate in kilohertz, a tenth where it isn't whole: "48 kHz", "44.1 kHz".
fn kilohertz(hertz: u32) -> String {
    if hertz.is_multiple_of(1000) {
        format!("{} kHz", hertz / 1000)
    } else {
        format!("{:.1} kHz", f64::from(hertz) / MILLISECONDS)
    }
}

/// A figure to three significant digits, its last zeros kept, as
/// Numbers.js's `significant`: 21.3, 5.33, 0.667.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a figure's magnitude fits an i32"
)]
fn significant(value: f64) -> String {
    let magnitude = |value: f64| value.abs().log10().floor() as i32;
    if value.abs() < f64::MIN_POSITIVE {
        return "0".to_owned();
    }
    let scale = 10f64.powi(SIGNIFICANT - 1 - magnitude(value));
    let rounded = (value * scale).round() / scale;
    let places = usize::try_from(SIGNIFICANT - 1 - magnitude(rounded)).unwrap_or(0);
    format!("{rounded:.places$}")
}

#[cfg(test)]
#[path = "programs_tests.rs"]
mod tests;
