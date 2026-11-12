//! A device's drawer, served beside its row as a `drawer` record the row
//! names by key, as a program's is (drawer.rs), so how it keeps time each
//! second changes the drawer alone and never a row. It says what the
//! launcher's own drawer said (VKCF3Z, 2026-11-07 and 2026-11-08): where
//! the device is joined and which way it goes, over its name; its State,
//! in `PipeWire`'s words, leading as what moves most; then the parts the
//! step into it serves, each a fact: its Volume or Gain in decibels, the
//! programs Playing through it or Recording from it, its Channels balanced
//! or each at its own level, its Tuning or Voice Processing, its Port and
//! its Profile; then what describes it, its Hardware's figures and what
//! the kernel says of its card; how it keeps time, while a watch wants
//! timings; the Format the graph moves its audio in, where the card's
//! doesn't say it already; the Priority that picks the one in use; and
//! last what it's known by, marked as identifiers: the ALSA Device it
//! opens, as aplay -D takes it, a USB card's ids and its Graph ID, as
//! wpctl takes it. Whether it's Muted is its row's to say, so it isn't
//! said again; its Level is left out, since reading it opens a stream on
//! a microphone and wakes a sleeping card, which only the step in does.

use serde_json::{Value, json};

use crate::drawer::{Around, fact, graph_format, keeping};
use crate::rows::{APART, Device, PLACE, Words};

/// The key of a device's drawer, which its row names.
pub(crate) fn key(device: &Device) -> String {
    format!("drawer:{PLACE}:{}", device.name)
}

/// The drawer of a device its row calls `name`, from the parts `ready`
/// serves inside that row.
pub(crate) fn drawer(device: &Device, name: &str, ready: &[Value], around: &Around) -> Value {
    let words = around.words;
    let top = format!("row:{PLACE}:{}", device.name);
    let joined = joined_by(device, words);
    let way = if device.sink { "plate-out" } else { "plate-in" };
    // Where it is and what it is, in our own words, so in Label's
    // capitals (VKCF3Z, 2026-11-09): one joined by nothing is the
    // machine's own.
    let eyebrow = [
        if joined.is_empty() {
            words.said("joined-here", &[])
        } else {
            joined
        },
        words.said(way, &[]),
    ]
    .join(APART);
    let mut facts = Vec::new();
    let state = device.record["state"].as_str().unwrap_or("");
    if !state.is_empty() {
        facts.push(fact(words, "state", state));
    }
    facts.extend(parts(device, &top, ready, words));
    facts.extend(keeping(device.record, around));
    facts.extend(graph_said(device, &top, ready, around));
    let priority = device.property("priority.session");
    if !priority.is_empty() {
        facts.push(fact(words, "priority", priority));
    }
    let alsa = device.property("api.alsa.path");
    if !alsa.is_empty() {
        facts.push(identifies(fact(words, "alsa-device", alsa)));
    }
    let usb = format!("{top}:hardware:usb-id");
    let usb = ready.iter().filter(|row| row["key"] == usb.as_str());
    facts.extend(usb.filter_map(|row| told(row, &said(row))).map(identifies));
    if let Some(id) = device.record["id"].as_u64() {
        facts.push(identifies(fact(words, "graph-id", &id.to_string())));
    }
    json!({
        "key": key(device),
        "kind": "drawer",
        "drawer": {"eyebrow": eyebrow, "title": name, "facts": facts},
    })
}

/// How a device is joined to the machine, in the vocabulary's words:
/// through a display, over Bluetooth, by USB or built in, or "" where the
/// sound server doesn't say. Bluetooth is a name, so it's said as the
/// vocabulary names it.
fn joined_by(device: &Device, words: &Words) -> String {
    let bus = device.property("device.bus");
    if device.display() {
        words.said("joined-display", &[])
    } else if !device.address().is_empty() || bus == "bluetooth" {
        words.name("bluetooth", &[])
    } else if bus == "usb" {
        words.said("joined-usb", &[])
    } else if bus == "pci" {
        words.said("joined-built-in", &[])
    } else {
        String::new()
    }
}

/// The parts inside a device's row as facts, in the drawer's order, each
/// by its name, saying what its row does but where the drawer says more:
/// a volume its decibels, the programs which are playing, the channels
/// each at its level and a filter each at its own. A level's bar is drawn
/// as a stroke after it.
fn parts(device: &Device, top: &str, ready: &[Value], words: &Words) -> Vec<Value> {
    let (volume, programs, filter) = if device.sink {
        ("volume", "programs", "tuning")
    } else {
        ("gain", "recorders", "voice")
    };
    let row = |part: &str| {
        let key = format!("{top}:{part}");
        ready.iter().find(|row| row["key"] == key.as_str())
    };
    let inside = |parent: &str| -> Vec<&Value> {
        ready.iter().filter(|row| row["parent"] == parent).collect()
    };
    let mut facts = Vec::new();
    if let Some(part) = row(volume) {
        facts.extend(told(part, part["note"].as_str().unwrap_or("")));
    }
    if let Some(part) = row(programs) {
        let playing = part["held"]["playing"].as_str().unwrap_or("");
        let value = if playing.is_empty() {
            said(part)
        } else {
            playing.to_owned()
        };
        facts.extend(told(part, &value));
    }
    if let Some(part) = row("channels") {
        let channels = inside(part["key"].as_str().unwrap_or(""));
        let names: Vec<String> = channels.iter().map(|row| name(row).to_owned()).collect();
        let levels: Vec<String> = channels.iter().map(|row| said(row)).collect();
        let balanced = words.said("balanced", &[]);
        let mut runs = vec![said(part)];
        runs.extend(channel_levels(&names, &levels, &balanced));
        facts.extend(told(part, &ran(&runs)));
    }
    if let Some(part) = row(filter) {
        let mut runs = vec![said(part)];
        let levels = part["held"]["levels"].as_array().into_iter().flatten();
        runs.extend(levels.filter_map(|level| Some(level.as_str()?.to_owned())));
        facts.extend(told(part, &ran(&runs)));
    }
    for part in ["port", "profile"].into_iter().filter_map(row) {
        facts.extend(told(part, &said(part)));
    }
    let usb = format!("{top}:hardware:usb-id");
    let hardware = inside(&format!("{top}:hardware"));
    let figures = hardware
        .into_iter()
        .filter(|row| row["key"] != usb.as_str());
    facts.extend(figures.filter_map(|row| told(row, &said(row))));
    let card = inside(top).into_iter().filter(|row| row["kind"] == "fact");
    facts.extend(card.filter_map(|row| told(row, &said(row))));
    facts
}

/// The format the graph moves a device's audio in, said once: as Format
/// where the card's running stream doesn't say one, not at all where it
/// says the same, and as Graph Format beside a card's that differs.
fn graph_said(device: &Device, top: &str, ready: &[Value], around: &Around) -> Option<Value> {
    let graph = graph_format(&device.record["format"], &around.numbers);
    if graph.is_empty() {
        return None;
    }
    let card = ready
        .iter()
        .filter(|row| row["parent"] == top && row["kind"] == "fact")
        .find(|row| matches!(row["held"]["fact"].as_str(), Some("playing" | "recording")))
        .map(said);
    match card {
        // Compared whole, since a graph in float over a card in integer
        // is the difference Graph Format is there to show.
        Some(card) if card == graph || card.starts_with(&format!("{graph}{APART}")) => None,
        Some(_) => Some(fact(around.words, "graph-format", &graph)),
        None => Some(fact(around.words, "format", &graph)),
    }
}

/// A device's channels each at its level, as its drawer says them after
/// its layout (VKCF3Z, 2026-11-07): `balanced` alone where every one is
/// at the same level, since a part equal to the rest gives way; and
/// otherwise each by its name and level, a stereo pair's without "front",
/// as it has no back: "left 100%", "right 80%".
fn channel_levels(names: &[String], levels: &[String], balanced: &str) -> Vec<String> {
    if levels.is_empty() {
        return Vec::new();
    }
    if levels.iter().all(|level| *level == levels[0]) {
        return vec![balanced.to_owned()];
    }
    let pair = names.len() == 2;
    names
        .iter()
        .zip(levels)
        .map(|(name, level)| {
            let lower = name.to_lowercase();
            let name = match lower.strip_prefix("front ") {
                Some(side) if pair => side.to_owned(),
                _ => lower,
            };
            format!("{name} {level}").trim().to_owned()
        })
        .collect()
}

fn name(row: &Value) -> &str {
    row["name"].as_str().unwrap_or("")
}

/// What a row says at its side, its items joined as every side's are.
fn said(row: &Value) -> String {
    let items = row["side"].as_array().into_iter().flatten();
    let said: Vec<&str> = items.filter_map(|item| item["said"].as_str()).collect();
    said.join(APART)
}

/// Values a run apart, those saying nothing left out.
fn ran(values: &[String]) -> String {
    let said: Vec<&str> = values
        .iter()
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .collect();
    said.join(APART)
}

/// A part as a fact saying `value`, its bar a stroke after it and its
/// history's spark as it is, or none where it has no name or says
/// nothing.
fn told(row: &Value, value: &str) -> Option<Value> {
    let label = name(row);
    if label.is_empty() || value.is_empty() {
        return None;
    }
    let mut fact = json!({"name": label, "said": value});
    let graph = &row["graph"];
    if graph["shape"] == "bar" {
        let warn = graph["warn"] == true;
        fact["figure"] = json!({"shape": "stroke", "values": graph["values"], "warn": warn});
    } else if graph["shape"] == "spark" {
        fact["figure"] = graph.clone();
    }
    Some(fact)
}

fn identifies(mut fact: Value) -> Value {
    fact["identifies"] = json!(true);
    fact
}

#[cfg(test)]
#[path = "device_drawer_tests.rs"]
mod tests;
