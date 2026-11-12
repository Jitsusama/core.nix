//! A device's filters, served inside its row as Sound.qml made them: the
//! Tuning in front of an output, or the Voice Processing in front of an
//! input, a smart filter aimed at it by what it asks a device to be
//! (`filter.smart.target`), so programs reach the device through it. It
//! says whether sound is passing through now, which its record's `flows`
//! says (devices.rs), and a step in, each filter by name with its level.

use serde_json::{Value, json};

use crate::parts::decibels;
use crate::programs::aims;
use crate::rows::{Device, PLACE, Words, bar, kind, named, percent};

/// The filters part of `device`, among `devices`, and each filter's rows
/// inside it, or none where no filter is aimed at it.
pub(crate) fn filters_rows(
    device: &Device,
    devices: &[Device],
    words: &Words,
    parent: &str,
) -> Vec<Value> {
    let found: Vec<&Device> = devices
        .iter()
        .filter(|filter| aims(filter, device))
        .collect();
    if found.is_empty() {
        return Vec::new();
    }
    let id = if device.sink { "tuning" } else { "voice" };
    let part = format!("{parent}:{id}");
    let thing = format!("sound:{}:{id}", device.name);
    let state = if found.iter().any(|filter| flowing(filter, device)) {
        "running"
    } else {
        "on"
    };
    let side = words.said(state, &[]);
    // A filter not yet set up to move sound has no level to show, as a
    // node Quickshell hasn't bound has no audio.
    let shown: Vec<&&Device> = found
        .iter()
        .filter(|filter| {
            filter.record["volumes"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
        })
        .collect();
    let names: Vec<String> = found.iter().map(|filter| filter.name.to_owned()).collect();
    let levels: Vec<String> = shown
        .iter()
        .map(|filter| format!("{} {}", named(filter, devices, words), level(filter, words)))
        .collect();
    let mut rows = vec![json!({
        "key": part,
        "thing": thing,
        "place": PLACE,
        "kind": "figures",
        "parent": parent,
        "name": words.name(&format!("sound:{id}"), &[]),
        "glyph": kind(device),
        "hue": "machine",
        "depth": 1,
        "side": [{"said": side}],
        "note": words.said(id, &[]),
        "acts": [{"key": "verb:copy", "copy": side}, {"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name, "filters": names, "levels": levels},
    })];
    for filter in shown {
        let (key, thing) = (
            format!("{part}:{}", filter.name),
            format!("{thing}:{}", filter.name),
        );
        let figure =
            |key: &str, thing: &str, fields: Value| figure_row(device, &part, key, thing, fields);
        let called = named(filter, devices, words);
        let passing = if flowing(filter, device) {
            "running"
        } else {
            "waiting"
        };
        rows.push(figure(
            &key,
            &thing,
            json!({
                "name": words.name("sound:filter", &[]),
                "side": [{"said": called}],
                "note": words.said(passing, &[]),
                "acts": [{"key": "verb:copy", "copy": called}],
            }),
        ));
        let said = level(filter, words);
        let gain = if device.sink {
            "sound:volume"
        } else {
            "sound:gain"
        };
        rows.push(figure(
            &format!("{key}:level"),
            &format!("{thing}:level"),
            json!({
                "name": words.name(gain, &[]),
                "side": [{"said": said}],
                "graph": bar(filter.audible()),
                "note": decibels(filter.volume()),
                "acts": [{"key": "verb:copy", "copy": said}],
            }),
        ));
    }
    rows
}

/// Whether sound is passing through `filter` to or from `device` now.
fn flowing(filter: &Device, device: &Device) -> bool {
    filter.record["flows"]
        .as_array()
        .is_some_and(|flows| flows.iter().any(|end| end == device.name))
}

/// How loud `filter` is, as its level reads, or that it's muted.
fn level(filter: &Device, words: &Words) -> String {
    if filter.muted() {
        words.said("muted", &[])
    } else {
        percent(filter.volume())
    }
}

/// One figure a step inside a filters part, with `fields` its own.
fn figure_row(device: &Device, part: &str, key: &str, thing: &str, fields: Value) -> Value {
    let mut row = json!({
        "key": key,
        "thing": thing,
        "place": PLACE,
        "kind": "figure",
        "parent": part,
        "glyph": "",
        "hue": "machine",
        "depth": 2,
        "held": {"node": device.name},
    });
    if let (Some(row), Value::Object(fields)) = (row.as_object_mut(), fields) {
        row.extend(fields);
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::tests::{device, words};

    const AIMED: &str = "{ \"device.api\" = \"alsa\" }";

    fn filter(name: &str, sink: bool, flows: &[&str]) -> Value {
        let mut filter = device(
            name,
            sink,
            "",
            &json!({"filter.smart": "true", "filter.smart.target": AIMED}),
        );
        filter["description"] = json!("Speech Clarity");
        filter["volume"] = json!(0.5);
        filter["volumes"] = json!([0.125, 0.125]);
        filter["flows"] = json!(flows);
        filter
    }

    fn served(records: &[Value], at: usize) -> Vec<Value> {
        let devices: Vec<Device> = records.iter().filter_map(Device::of).collect();
        filters_rows(&devices[at], &devices, &words(), "row:sound:it")
    }

    // A filter aimed at a device is its Tuning, running while its sound
    // passes to it, and a step in, the filter by name and its level.
    #[test]
    fn a_filter_aimed_at_a_device_is_its_tuning() {
        let speakers = device("speakers", true, "", &json!({"device.api": "alsa"}));
        let records = [speakers, filter("clarity", true, &["speakers"])];
        let rows = served(&records, 0);
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:sound:it:tuning",
                "row:sound:it:tuning:clarity",
                "row:sound:it:tuning:clarity:level"
            ]
        );
        assert_eq!(rows[0]["name"], "Tuning");
        assert_eq!(rows[0]["side"][0]["said"], "running");
        assert_eq!(rows[0]["held"]["filters"], json!(["clarity"]));
        assert_eq!(rows[0]["held"]["levels"], json!(["Speech Clarity 50%"]));
        assert_eq!(rows[1]["side"][0]["said"], "Speech Clarity");
        assert_eq!(rows[1]["note"], "running");
        assert_eq!(rows[2]["name"], "Volume");
        assert_eq!(rows[2]["side"][0]["said"], "50%");
        assert_eq!(rows[2]["note"], "-18.1 dB");
    }

    // A filter whose sound passes elsewhere is on but waiting; one aimed
    // at the other way, or at another kind of device, isn't this one's.
    #[test]
    fn a_filter_not_passing_sound_is_on_and_waiting() {
        let microphone = device("microphone", false, "", &json!({"device.api": "alsa"}));
        let bluetooth = device("headset", false, "", &json!({"device.api": "bluez5"}));
        let records = [
            microphone,
            bluetooth,
            filter("voice", false, &["headset"]),
            filter("tuning", true, &[]),
        ];
        let rows = served(&records, 0);
        assert_eq!(rows[0]["key"], "row:sound:it:voice");
        assert_eq!(rows[0]["name"], "Voice Processing");
        assert_eq!(rows[0]["side"][0]["said"], "on");
        assert_eq!(rows[1]["note"], "waiting");
        assert_eq!(rows[2]["name"], "Gain");
        assert!(
            served(&records, 1).is_empty(),
            "nothing is aimed at a Bluetooth one"
        );
    }
}
