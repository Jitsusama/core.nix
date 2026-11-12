//! The parts the daemon serves inside each device's row (../ready-rows),
//! each a step in from it, in the order Sound.qml lists a device's parts:
//! using it, where it isn't the one chosen; whether it's Muted, one switch;
//! its Volume, or a microphone's Gain, a setting with its levels a step
//! in; the programs playing through it or recording from it
//! (programs.rs); its Port, where its card offers more than one; its
//! Channels (channels.rs); a Bluetooth device's Profile; and last, what
//! desk-facts says of its card (desk.rs). Each act asks the daemon
//! (acts.rs), so the shell runs nothing to do it.
//!
//! The rest of a device's parts, its Level, filters and hardware, are
//! still made by Sound.qml from the sound server.

use serde_json::{Value, json};

use crate::channels::channels_rows;
use crate::desk::fact_rows;
use crate::programs::{Around, Naming, Sendable, programs_part};
use crate::rows::{APART, Device, PLACE, Words, bar, carried, kind, percent, rows, switches, worn};

/// How far Louder and Quieter move a volume, as Sound.qml's `step`.
pub(crate) const STEP: f64 = 0.05;

/// The levels a volume can be set to at once, as Sound.qml's `levels`.
const LEVELS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

/// A volume cubed is the sound server's, so its decibels are 60 times its
/// log, as a mixer says them (Sounds.js's `decibels`).
const DECIBELS_CUBED: f64 = 60.0;

/// Every row Sound's place serves: each device's row, as rows.rs lists
/// them, and its parts after it, its programs named as `naming` names
/// them.
pub fn served(records: &[Value], words: &Words, naming: Naming) -> Vec<Value> {
    let devices: Vec<Device> = records.iter().filter_map(Device::of).collect();
    let tops = rows(records, words);
    // Where a program can be sent: each device listed its way, in Sound's
    // order, by its row's name.
    let listed = |sink: bool| Sendable {
        devices: tops
            .iter()
            .filter_map(|top| {
                let device = devices.iter().find(|one| top["held"]["node"] == one.name)?;
                let name = top["name"].as_str().unwrap_or(device.name);
                (device.sink == sink).then(|| (device, name.to_owned()))
            })
            .collect(),
    };
    let (outputs, inputs) = (listed(true), listed(false));
    let around = |sink: bool| Around {
        records,
        devices: &devices,
        sendable: if sink { &outputs } else { &inputs },
        words,
        naming,
    };
    tops.iter()
        .flat_map(|top| {
            let parts = devices
                .iter()
                .find(|device| top["held"]["node"] == device.name)
                .map(|device| of(device, top, &around(device.sink)))
                .unwrap_or_default();
            std::iter::once(top.clone()).chain(parts)
        })
        .collect()
}

fn of(device: &Device, top: &Value, around: &Around) -> Vec<Value> {
    let (devices, words) = (around.devices, around.words);
    let parent = format!("row:{PLACE}:{}", device.name);
    let mut parts = Vec::new();
    if !device.chosen() {
        let named = top["name"].as_str().unwrap_or("");
        parts.push(use_row(device, named, devices, words, &parent));
    }
    parts.push(mute_row(device, words, &parent));
    parts.extend(volume_rows(device, words, &parent));
    parts.extend(programs_part(device, around, &parent));
    parts.extend(port_rows(device, words, &parent));
    parts.extend(channels_rows(device, words, &parent));
    parts.extend(profile_row(device, devices, words, &parent));
    parts.extend(fact_rows(device, words, &parent));
    parts
}

/// A Bluetooth device's Profile: what its sound is for now, music or
/// calls, said in those words, and that listening through a headset
/// microphone's stand-in switches it to calls while it's music. Its
/// codec and link are said once, in Bluetooth, which the note points to.
fn profile_row(device: &Device, devices: &[Device], words: &Words, parent: &str) -> Option<Value> {
    let carries = carried(device, devices);
    if carries.is_empty() {
        return None;
    }
    let note = if switches(device, devices) {
        "profile-switches"
    } else {
        "profile-elsewhere"
    };
    Some(json!({
        "key": format!("{parent}:profile"),
        "thing": format!("sound:{}:profile", device.name),
        "place": PLACE,
        "kind": "figure",
        "parent": parent,
        "name": words.name("sound:profile", &[]),
        "glyph": "",
        "hue": "machine",
        "depth": 1,
        "side": [{"said": words.said(carries, &[])}],
        "note": words.said(note, &[]),
        "acts": [],
        "held": {"node": device.name},
    }))
}

/// An ask of the daemon's act `act` on the device, with `more` beside it.
fn ask(device: &Device, act: &str, more: &Value) -> Value {
    let mut asked = json!({"ask": "act", "act": act, "key": format!("device:{}", device.name)});
    if let (Some(asked), Some(more)) = (asked.as_object_mut(), more.as_object()) {
        asked.extend(more.clone());
    }
    asked
}

/// Playing sound through it, or listening through it, its likeliest act,
/// saying where listening would switch a headset to calls. Its line names
/// the device, as `:set output=` or `input=` does.
fn use_row(device: &Device, named: &str, devices: &[Device], words: &Words, parent: &str) -> Value {
    let state = if device.sink { "" } else { "input" };
    let note = if switches(device, devices) {
        words.said("profile-switches", &[])
    } else {
        String::new()
    };
    json!({
        "key": format!("{parent}:use"),
        "thing": "verb:sound-use",
        "place": PLACE,
        "kind": "act",
        "parent": parent,
        "name": words.name_in("verb:sound-use", state, &[]),
        "glyph": kind(device),
        "hue": "action",
        "depth": 1,
        "side": [],
        "note": note,
        "acts": [{"key": "verb:sound-use", "ask": ask(device, "use", &json!({}))}],
        "held": {"node": device.name, "fill": {"name": named}, "state": state},
    })
}

/// Whether it's Muted: one switch, keeping its name as it flips, its side
/// saying yes or no, its one act named Mute or Unmute, as words.json's
/// verb says it in the state it's in (docs/design.md, "A switch is one
/// row that keeps its name as it flips"; ../ready-rows, an act's name).
fn mute_row(device: &Device, words: &Words, parent: &str) -> Value {
    let muted = device.muted();
    let flips = capitalised(&words.verb_in("sound:mute", if muted { "muted" } else { "" }));
    json!({
        "key": format!("{parent}:mute"),
        "thing": format!("sound:{}:mute", device.name),
        "place": PLACE,
        "kind": "switch",
        "parent": parent,
        "name": words.name("sound:mute", &[]),
        "glyph": worn(device),
        "hue": "action",
        "depth": 1,
        "side": [{"said": words.said(if muted { "yes" } else { "no" }, &[])}],
        "acts": [{
            "key": "sound:mute",
            "name": flips,
            "ask": ask(device, "mute", &json!({"muted": !muted})),
        }],
        "held": {"node": device.name, "muted": muted},
    })
}

/// Its Volume, or a microphone's Gain: a setting saying how it's set, and
/// a step in, Louder and Quieter, then each level it can be set to. Louder
/// is meant to be heard, so it unmutes, as a level set does.
fn volume_rows(device: &Device, words: &Words, parent: &str) -> Vec<Value> {
    let id = if device.sink { "volume" } else { "gain" };
    let state = if device.sink { "" } else { "input" };
    let volume = device.volume();
    let part = format!("{parent}:{id}");
    let level = if device.muted() {
        words.said("muted", &[])
    } else {
        percent(volume)
    };
    let choice = |key: String, thing: &str, name: String, asked: Value| {
        json!({
            "key": key,
            "thing": thing,
            "place": PLACE,
            "kind": "act",
            "parent": part,
            "name": name,
            "glyph": "volume",
            "hue": "action",
            "depth": 2,
            "side": [],
            "note": "",
            "acts": [{"key": thing, "ask": asked}],
            "held": {"node": device.name},
        })
    };
    let mut rows = vec![json!({
        "key": part,
        "thing": format!("sound:{}:{id}", device.name),
        "place": PLACE,
        "kind": "setting",
        "parent": parent,
        "name": words.name(&format!("sound:{id}"), &[]),
        "glyph": worn(device),
        "hue": "machine",
        "depth": 1,
        "side": [{"said": level}],
        "graph": bar(device.audible()),
        "note": decibels(volume),
        "acts": [{"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name},
    })];
    for (by, verb, unmutes) in [(STEP, "louder", true), (-STEP, "quieter", false)] {
        let thing = format!("verb:sound-{verb}");
        let mut more = json!({"volume": stepped(volume, by)});
        if unmutes {
            more["muted"] = json!(false);
        }
        let name = words.name(&thing, &[]);
        let asked = ask(device, "volume", &more);
        rows.push(choice(format!("{part}:{verb}"), &thing, name, asked));
    }
    for each in LEVELS {
        let said = percent(each);
        let name = words.name_in("verb:sound-to", state, &[("level", &said)]);
        let asked = ask(device, "volume", &json!({"volume": each, "muted": false}));
        let thing = format!("verb:sound-to-{said}");
        let mut row = choice(format!("{part}:{said}"), &thing, name, asked);
        if !device.muted() && (volume - each).abs() < 0.005 {
            row["side"] = json!([{"said": words.said("chosen", &[])}]);
        }
        row["note"] = json!(decibels(each));
        // Each level is the one verb, as words.json words it.
        row["acts"][0]["key"] = json!("verb:sound-to");
        rows.push(row);
    }
    rows
}

/// A verb as an act's name says it, its first letter a capital.
pub(crate) fn capitalised(verb: &str) -> String {
    let mut letters = verb.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// A volume moved by `by`, kept to whole steps from nothing to full, as
/// Sounds.js's `stepped`.
pub(crate) fn stepped(level: f64, by: f64) -> f64 {
    let kept = (((level + by) / STEP).round() * STEP).clamp(0.0, 1.0);
    (kept * 1e6).round() / 1e6
}

/// A level in decibels, to a tenth, as a mixer says it: 100% is 0 dB,
/// 50% is -18.1 dB, and nothing is -∞ dB.
pub(crate) fn decibels(level: f64) -> String {
    if level.is_nan() || level <= 0.0 {
        return "-\u{221e} dB".to_owned();
    }
    let tenths = (DECIBELS_CUBED * level.log10() * 10.0).round() / 10.0;
    if tenths.abs() < 0.05 {
        return "0 dB".to_owned();
    }
    let sign = if tenths > 0.0 { "+" } else { "" };
    format!("{sign}{tenths:.1} dB")
}

/// A device's Port, where its card offers it two or more to choose from:
/// a setting saying the one it plays through, and inside it each it can
/// be moved to, done by the port act. A port nothing's plugged into is no
/// choice, unless it's the one in use, which says so. None in use, as a
/// card not yet saying its routes, is no Port yet.
fn port_rows(device: &Device, words: &Words, parent: &str) -> Vec<Value> {
    let offered = device.record["ports"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let in_use = |port: &Value| port["chosen"] == true;
    let choosable: Vec<&Value> = offered
        .iter()
        .filter(|port| in_use(port) || port["available"] != "no")
        .collect();
    let Some(now) = choosable.iter().find(|port| in_use(port)) else {
        return Vec::new();
    };
    if choosable.len() < 2 {
        return Vec::new();
    }
    let unplugged = words.said("unplugged", &[]);
    let name_of = |port: &Value| port["name"].as_str().unwrap_or("").to_owned();
    let side = if now["available"] == "no" {
        format!("{}{APART}{unplugged}", name_of(now))
    } else {
        name_of(now)
    };
    let glyph = kind(device);
    let part = format!("{parent}:port");
    let mut rows = vec![json!({
        "key": part,
        "thing": format!("sound:{}:port", device.name),
        "place": PLACE,
        "kind": "setting",
        "parent": parent,
        "name": words.name("sound:port", &[]),
        "glyph": glyph,
        "hue": "machine",
        "depth": 1,
        "side": [{"said": side}],
        "acts": [{"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name},
    })];
    for port in choosable {
        let index = &port["index"];
        let side: Vec<Value> = if in_use(port) {
            vec![json!({"said": words.said("chosen", &[])})]
        } else {
            Vec::new()
        };
        let note = if port["available"] == "no" {
            unplugged.clone()
        } else {
            String::new()
        };
        let asked = ask(device, "port", &json!({"index": index}));
        rows.push(json!({
            "key": format!("{part}:{index}"),
            // As the launcher's own verb row was keyed, so a step held on
            // one stays where it was.
            "thing": format!("verb:sound-port-{index}"),
            "place": PLACE,
            "kind": "port",
            "parent": part,
            "name": words.name("verb:sound-port", &[("port", &name_of(port))]),
            "glyph": glyph,
            "hue": "action",
            "depth": 2,
            "side": side,
            "note": note,
            "acts": [{"key": "verb:sound-port", "ask": asked}],
            "held": {"node": device.name, "port": name_of(port), "index": index,
                "chosen": in_use(port)},
        }));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::Named;
    use crate::rows::tests::{device, words};

    /// Every row served, each program by the name it gives.
    fn served(records: &[Value], words: &Words) -> Vec<Value> {
        let naming = |given: &str| Named {
            name: given.to_owned(),
            glyph: json!("program"),
        };
        super::served(records, words, &naming)
    }

    fn keyed<'a>(rows: &'a [Value], key: &str) -> &'a Value {
        rows.iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no row {key}"))
    }

    // A headset says what its profile carries, and its microphone's
    // stand-in says its headset's, and that listening through it switches
    // that to calls while it's music. A device that isn't Bluetooth has
    // no Profile.
    #[test]
    fn a_headset_and_its_microphone_say_its_profile() {
        let address = "AC:80:0A:2E:AA:01";
        let profiled = |profile: &str| {
            device(
                &format!("bluez_output.{address}.1"),
                true,
                "WH-1000XM4",
                &json!({"api.bluez5.address": address, "api.bluez5.profile": profile}),
            )
        };
        let stand_in = device(
            &format!("bluez_input.{address}"),
            false,
            "WH-1000XM4",
            &json!({"bluez5.loopback": "true"}),
        );
        let desk = device("desk", true, "Desk", &json!({}));
        let made = served(&[profiled("a2dp-sink"), stand_in.clone(), desk], &words());
        let headset = keyed(
            &made,
            &format!("row:sound:bluez_output.{address}.1:profile"),
        );
        assert_eq!(headset["name"], "Profile");
        assert_eq!(headset["side"], json!([{"said": "music"}]));
        assert_eq!(headset["note"], "its codec and connection are in Bluetooth");
        let microphone = keyed(&made, &format!("row:sound:bluez_input.{address}:profile"));
        assert_eq!(microphone["side"], json!([{"said": "music"}]));
        assert_eq!(
            microphone["note"],
            "listening through it switches the headset to calls"
        );
        assert!(
            made.iter()
                .all(|row| row["key"] != "row:sound:desk:profile"),
            "a device that isn't Bluetooth has no Profile"
        );
        let calling = served(&[profiled("headset-head-unit"), stand_in], &words());
        let microphone = keyed(
            &calling,
            &format!("row:sound:bluez_input.{address}:profile"),
        );
        assert_eq!(microphone["side"], json!([{"said": "calls"}]));
        assert_eq!(
            microphone["note"],
            "its codec and connection are in Bluetooth"
        );
    }

    // Inside a device's row come its parts, in the order Sound lists
    // them: using it where it isn't chosen, Muted, then its Volume and the
    // levels a step in it; a microphone's is its Gain.
    #[test]
    fn a_device_serves_its_use_mute_and_volume_inside_it() {
        let mut speakers = device("desk", true, "Desk", &json!({}));
        speakers["volume"] = json!(0.5);
        let made = served(&[speakers.clone()], &words());
        let keys: Vec<&str> = made
            .iter()
            .map(|row| row["key"].as_str().unwrap_or(""))
            .collect();
        assert_eq!(
            keys,
            [
                "row:sound:desk",
                "row:sound:desk:use",
                "row:sound:desk:mute",
                "row:sound:desk:volume",
                "row:sound:desk:volume:louder",
                "row:sound:desk:volume:quieter",
                "row:sound:desk:volume:25%",
                "row:sound:desk:volume:50%",
                "row:sound:desk:volume:75%",
                "row:sound:desk:volume:100%"
            ],
            "a device's parts follow it, in Sound's order"
        );
        let using = keyed(&made, "row:sound:desk:use");
        assert_eq!(using["parent"], "row:sound:desk");
        assert_eq!(using["name"], "Play Sound Through It");
        assert_eq!(
            using["acts"],
            json!([{"key": "verb:sound-use", "ask": {
                "ask": "act", "act": "use", "key": "device:desk"}}])
        );
        assert_eq!(using["held"]["fill"], json!({"name": "Desk"}));
        let muting = keyed(&made, "row:sound:desk:mute");
        assert_eq!(muting["name"], "Muted");
        assert_eq!(muting["side"], json!([{"said": "no"}]));
        assert_eq!(muting["acts"][0]["ask"]["muted"], true);
        assert_eq!(muting["acts"][0]["name"], "Mute");
        let volume = keyed(&made, "row:sound:desk:volume");
        assert_eq!(volume["kind"], "setting");
        assert_eq!(volume["side"], json!([{"said": "50%"}]));
        assert_eq!(volume["note"], "-18.1 dB");
        let louder = keyed(&made, "row:sound:desk:volume:louder");
        assert_eq!(louder["parent"], "row:sound:desk:volume");
        assert_eq!(
            louder["acts"][0]["ask"],
            json!({"ask": "act", "act": "volume", "key": "device:desk",
                "volume": 0.55, "muted": false})
        );
        let quieter = keyed(&made, "row:sound:desk:volume:quieter");
        assert_eq!(quieter["acts"][0]["ask"]["volume"], 0.45);
        assert_eq!(quieter["acts"][0]["ask"].get("muted"), None);
        let half = keyed(&made, "row:sound:desk:volume:50%");
        assert_eq!(half["thing"], "verb:sound-to-50%");
        assert_eq!(half["name"], "Volume to 50%");
        assert_eq!(half["side"], json!([{"said": "chosen"}]));
        assert_eq!(half["acts"][0]["key"], "verb:sound-to");
        assert_eq!(keyed(&made, "row:sound:desk:volume:100%")["note"], "0 dB");

        // The one chosen isn't offered to use; muted, its switch says so,
        // its act unmutes, and its volume says muted, no level chosen.
        speakers["chosen"] = json!(true);
        speakers["muted"] = json!(true);
        let made = served(&[speakers], &words());
        let keys: Vec<&Value> = made.iter().map(|row| &row["key"]).collect();
        assert!(
            !keys.contains(&&json!("row:sound:desk:use")),
            "the chosen device offered to use: {keys:?}"
        );
        let muting = keyed(&made, "row:sound:desk:mute");
        assert_eq!(muting["side"], json!([{"said": "yes"}]));
        assert_eq!(muting["glyph"], "silent");
        assert_eq!(muting["acts"][0]["name"], "Unmute");
        assert_eq!(muting["acts"][0]["ask"]["muted"], false);
        assert_eq!(
            keyed(&made, "row:sound:desk:volume")["side"],
            json!([{"said": "muted"}])
        );
        assert_eq!(keyed(&made, "row:sound:desk:volume:50%")["side"], json!([]));

        let microphone = device("desk-in", false, "Desk Mic", &json!({}));
        let made = served(&[microphone], &words());
        assert_eq!(
            keyed(&made, "row:sound:desk-in:use")["name"],
            "Listen Through It"
        );
        let gain = keyed(&made, "row:sound:desk-in:gain");
        assert_eq!(gain["name"], "Gain");
        assert_eq!(
            keyed(&made, "row:sound:desk-in:gain:25%")["name"],
            "Gain to 25%"
        );
    }

    // Louder and Quieter keep to whole steps, from nothing to full.
    #[test]
    fn a_volume_steps_whole_steps_within_its_bounds() {
        let up = stepped(0.42, STEP);
        assert!((up - 0.45).abs() < 1e-9, "0.42 louder is {up}, not 0.45");
        let full = stepped(0.98, STEP);
        assert!((full - 1.0).abs() < 1e-9, "0.98 louder is {full}, not 1");
        let none = stepped(0.02, -STEP);
        assert!(none.abs() < 1e-9, "0.02 quieter is {none}, not 0");
        assert_eq!(decibels(0.0), "-\u{221e} dB");
        assert_eq!(decibels(1.2), "+4.8 dB");
    }

    // A device its card offers two ports or more has a Port inside its
    // row, saying the one in use, and inside that each it can be moved
    // to, done by the port act: a port nothing's plugged into is no
    // choice, unless it's in use, which says so.
    #[test]
    fn a_device_with_ports_to_choose_says_them_inside() {
        fn port(index: i64, name: &str, available: &str, chosen: bool) -> Value {
            json!({"index": index, "name": name, "available": available, "chosen": chosen})
        }
        fn ports(made: &[Value]) -> Vec<&str> {
            made.iter()
                .filter_map(|row| row["key"].as_str())
                .filter(|key| key.contains(":port"))
                .collect()
        }
        let mut analog = device("analog", true, "Analog", &json!({}));
        analog["ports"] = json!([
            port(1, "Speakers", "unknown", false),
            port(2, "Headphones", "no", true),
            port(3, "Line Out", "no", false),
            port(4, "Dock", "yes", false),
        ]);
        let made = served(&[analog.clone()], &words());
        assert_eq!(
            ports(&made),
            [
                "row:sound:analog:port",
                "row:sound:analog:port:1",
                "row:sound:analog:port:2",
                "row:sound:analog:port:4",
            ],
            "a port nothing's plugged into, not in use, is no choice"
        );
        let part = keyed(&made, "row:sound:analog:port");
        assert_eq!(part["parent"], "row:sound:analog");
        assert_eq!(part["depth"], 1);
        assert_eq!(part["kind"], "setting");
        assert_eq!(part["thing"], "sound:analog:port");
        assert_eq!(part["name"], "Port");
        assert_eq!(part["glyph"], "speaker");
        assert_eq!(
            part["side"],
            json!([{"said": "Headphones \u{b7} nothing plugged in"}])
        );
        assert_eq!(part["acts"], json!([{"key": "verb:more", "step": "rows"}]));
        let speakers = keyed(&made, "row:sound:analog:port:1");
        let headphones = keyed(&made, "row:sound:analog:port:2");
        assert_eq!(speakers["parent"], "row:sound:analog:port");
        assert_eq!(speakers["depth"], 2);
        assert_eq!(speakers["thing"], "verb:sound-port-1");
        assert_eq!(speakers["name"], "Use Speakers");
        assert_eq!(speakers["side"], json!([]));
        assert_eq!(speakers["note"], "");
        assert_eq!(
            speakers["acts"],
            json!([{"key": "verb:sound-port", "ask": {
                "ask": "act", "act": "port", "key": "device:analog", "index": 1}}])
        );
        assert_eq!(
            speakers["held"],
            json!({"node": "analog", "port": "Speakers", "index": 1, "chosen": false})
        );
        assert_eq!(headphones["side"], json!([{"said": "chosen"}]));
        assert_eq!(headphones["note"], "nothing plugged in");

        // One choice, or none in use, is no Port.
        analog["ports"] = json!([
            port(1, "Speakers", "yes", true),
            port(2, "Line Out", "no", false)
        ]);
        assert_eq!(
            ports(&served(&[analog.clone()], &words())),
            Vec::<&str>::new()
        );
        analog["ports"] = json!([
            port(1, "Speakers", "yes", false),
            port(2, "Dock", "yes", false)
        ]);
        assert_eq!(ports(&served(&[analog], &words())), Vec::<&str>::new());
    }
}
