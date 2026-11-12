//! A device's Channels, served inside its row after its Port: how many it
//! has, mono, stereo or a count, with Return copying that and a step in
//! to each channel by its name at its level, its bar empty while muted,
//! its decibels in its note, as Sound.qml's `channelsPart` made them. A
//! device whose Props name no channels has no Channels.

use serde_json::{Value, json};

use crate::parts::decibels;
use crate::rows::{Device, PLACE, Words, bar, kind, percent};

/// SPA's names for the channel positions (spa/param/audio/raw.h), by id,
/// as Quickshell's `PwAudioChannel` says them.
const NAMED: [&str; 38] = [
    "Unknown",
    "N/A",
    "Mono",
    "Front Left",
    "Front Right",
    "Front Center",
    "Low Frequency Effects",
    "Side Left",
    "Side Right",
    "Front Left Center",
    "Front Right Center",
    "Rear Center",
    "Rear Left",
    "Rear Right",
    "Top Center",
    "Top Front Left",
    "Top Front Center",
    "Top Front Right",
    "Top Rear Left",
    "Top Rear Center",
    "Top Rear Right",
    "Rear Left Center",
    "Rear Right Center",
    "Front Left Wide",
    "Front Right Wide",
    "Low Frequency Effects 2",
    "Front Left High",
    "Front Center High",
    "Front Right High",
    "Top Front Left Center",
    "Top Front Right Center",
    "Top Side Left",
    "Top Side Right",
    "Low Frequency Effects Left",
    "Low Frequency Effects Right",
    "Bottom Center",
    "Bottom Left Center",
    "Bottom Right Center",
];

/// Where SPA's auxiliary channels start, each named from one up.
const AUX: u32 = 0x1000;

/// A channel's name as the launcher says it, only its first word a
/// capital, as Sounds.js's `channel`: "Front left", "Aux 1".
pub(crate) fn named(id: u32) -> String {
    let said = match usize::try_from(id).ok().and_then(|at| NAMED.get(at)) {
        Some(name) => (*name).to_owned(),
        None if (AUX..AUX * 2).contains(&id) => format!("Aux {}", id - AUX + 1),
        None => NAMED[0].to_owned(),
    };
    let lower = said.to_lowercase();
    let mut letters = lower.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// The Channels part and each channel inside it, or none for a device
/// whose Props name no channels.
pub(crate) fn channels_rows(device: &Device, words: &Words, parent: &str) -> Vec<Value> {
    let ids: Vec<u32> = device.record["channels"]
        .as_array()
        .map(|ids| {
            ids.iter()
                .filter_map(|id| u32::try_from(id.as_u64()?).ok())
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        return Vec::new();
    }
    let volumes = device.record["volumes"].as_array();
    let part = format!("{parent}:channels");
    let thing = format!("sound:{}:channels", device.name);
    let count = ids.len().to_string();
    let side = match ids.len() {
        1 => words.said("mono", &[]),
        2 => words.said("stereo", &[]),
        _ => words.said("channels", &[("count", &count)]),
    };
    let mut rows = vec![json!({
        "key": part,
        "thing": thing,
        "place": PLACE,
        "kind": "figures",
        "parent": parent,
        "name": words.name("sound:channels", &[]),
        "glyph": kind(device),
        "hue": "machine",
        "depth": 1,
        "side": [{"said": side}],
        "note": "",
        "acts": [{"key": "verb:copy", "copy": side}, {"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name},
    })];
    for (at, id) in ids.iter().enumerate() {
        // A volume is held linear; the launcher shows its cube root.
        let level = volumes
            .and_then(|volumes| volumes.get(at)?.as_f64())
            .map_or(0.0, |linear| linear.max(0.0).cbrt());
        let said = percent(level);
        rows.push(json!({
            "key": format!("{part}:{at}"),
            "thing": format!("{thing}:{at}"),
            "place": PLACE,
            "kind": "figure",
            "parent": part,
            "name": named(*id),
            "glyph": "",
            "hue": "machine",
            "depth": 2,
            "side": [{"said": said}],
            "graph": bar(if device.muted() { 0.0 } else { level }),
            "note": decibels(level),
            "acts": [{"key": "verb:copy", "copy": said}],
            "held": {"node": device.name},
        }));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::tests::{device, words};

    fn stereo(muted: bool) -> Value {
        let mut speakers = device("speakers", true, "Speakers", &json!({}));
        speakers["channels"] = json!([3, 4]);
        speakers["volumes"] = json!([1.0, 0.125]);
        speakers["muted"] = json!(muted);
        speakers
    }

    fn rows_of(record: &Value) -> Vec<Value> {
        let device = Device::of(record).expect("a device");
        channels_rows(&device, &words(), "row:sound:speakers")
    }

    // A stereo device says so, and steps into each channel at its level,
    // as the launcher shows a volume, its cube root.
    #[test]
    fn a_devices_channels_say_each_at_its_level() {
        let rows = rows_of(&stereo(false));
        let part = &rows[0];
        assert_eq!(part["key"], "row:sound:speakers:channels");
        assert_eq!(part["thing"], "sound:speakers:channels");
        assert_eq!(part["name"], "Channels");
        assert_eq!(part["side"], json!([{"said": "stereo"}]));
        assert_eq!(part["acts"][0]["copy"], "stereo");
        let said: Vec<(&str, &str, &str)> = rows[1..]
            .iter()
            .map(|row| {
                let name = row["name"].as_str().unwrap_or("");
                let side = row["side"][0]["said"].as_str().unwrap_or("");
                (name, side, row["note"].as_str().unwrap_or(""))
            })
            .collect();
        assert_eq!(
            said,
            [
                ("Front left", "100%", "0 dB"),
                ("Front right", "50%", "-18.1 dB")
            ]
        );
        assert_eq!(rows[2]["graph"]["values"], json!([0.5]));
        assert_eq!(rows[2]["parent"], "row:sound:speakers:channels");
    }

    // Muted, each channel's bar is empty, as nothing comes out.
    #[test]
    fn a_muted_devices_channels_draw_nothing() {
        let rows = rows_of(&stereo(true));
        assert_eq!(rows[1]["graph"]["values"], json!([0.0]));
        assert_eq!(rows[1]["side"], json!([{"said": "100%"}]));
    }

    #[test]
    fn one_channel_is_mono_and_more_are_counted() {
        let mut record = stereo(false);
        record["channels"] = json!([2]);
        assert_eq!(rows_of(&record)[0]["side"], json!([{"said": "mono"}]));
        record["channels"] = json!([3, 4, 5, 6, 7, 8]);
        record["volumes"] = json!(vec![1.0; 6]);
        assert_eq!(rows_of(&record)[0]["side"], json!([{"said": "6 channels"}]));
        record["channels"] = json!([]);
        assert!(
            rows_of(&record).is_empty(),
            "no channels named, no Channels"
        );
    }

    #[test]
    fn a_channel_is_named_as_the_launcher_says_it() {
        assert_eq!(named(6), "Low frequency effects");
        assert_eq!(named(AUX), "Aux 1");
        assert_eq!(named(AUX + 1), "Aux 2");
        assert_eq!(named(99), "Unknown");
    }
}
