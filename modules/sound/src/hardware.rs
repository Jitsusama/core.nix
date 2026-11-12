//! A device's Hardware, served inside its row as Sound.qml made it: the
//! chip it goes through, as its card's components name them, picked by
//! what the device is, unless desk-facts names it, by its maker too, among
//! the facts after it; then a USB card's ids and its name, and the bits
//! its samples carry. A card built into the machine is named by its
//! driver ("sof-soundwire"), which says nothing, so it isn't.

use serde_json::{Value, json};

use crate::rows::{Device, PLACE, Words, kind};

/// The facts desk-facts names a card's chip by, which its components
/// then needn't.
const CHIP_FACTS: [&str; 2] = ["codec", "amplifiers"];

/// The chips a sound card's components name, in capitals as their makers
/// write them. "cfg-amp:4 hs:cs42l45 mic:cs42l45-dmic spk:cs35l56" is four
/// CS35L56 amplifiers, a CS42L45 for the headset and its digital
/// microphones.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Chips {
    amplifiers: u32,
    speaker: String,
    headset: String,
    microphone: String,
    digital: bool,
    usb: String,
}

/// The chips `text`, a card's `alsa.components`, names.
pub(crate) fn components(text: &str) -> Chips {
    let mut chips = Chips::default();
    for word in text.split_whitespace() {
        if let Some(usb) = usb_ids(word) {
            chips.usb = usb;
            continue;
        }
        let Some((key, value)) = word.split_once(':') else {
            continue;
        };
        match key {
            "cfg-amp" => chips.amplifiers = value.parse().unwrap_or(0),
            "spk" => chips.speaker = value.to_uppercase(),
            "hs" => chips.headset = value.to_uppercase(),
            "mic" => {
                let chip = value.strip_suffix("-dmic");
                chips.digital = chip.is_some();
                chips.microphone = chip.unwrap_or(value).to_uppercase();
            }
            _ => {}
        }
    }
    chips
}

/// A USB card's maker and product ids, "USB0d8c:0014" as "0d8c:0014".
fn usb_ids(word: &str) -> Option<String> {
    let ids = word
        .get(..3)?
        .eq_ignore_ascii_case("usb")
        .then(|| &word[3..])?;
    let (maker, product) = ids.split_once(':')?;
    let hex = |id: &str| id.len() == 4 && id.chars().all(|digit| digit.is_ascii_hexdigit());
    (hex(maker) && hex(product)).then(|| ids.to_lowercase())
}

/// Whether desk-facts names the device's chip among its card's facts.
fn told(device: &Device) -> bool {
    device.record["facts"]["said"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|fact| fact["name"].as_str())
        .any(|name| CHIP_FACTS.contains(&name.to_lowercase().as_str()))
}

/// Each figure its Hardware holds, by its id and what it says.
fn figures(device: &Device, words: &Words) -> Vec<(&'static str, String)> {
    let chips = components(device.property("alsa.components"));
    let what = format!(
        "{} {}",
        device.property("device.profile.description"),
        device.nicknamed()
    )
    .to_lowercase();
    let mut figures = Vec::new();
    if !told(device) {
        if what.contains("head") {
            figures.push(("codec", chips.headset.clone()));
        } else if device.sink && !chips.speaker.is_empty() {
            let count = chips.amplifiers.to_string();
            figures.push((
                "amplifiers",
                if chips.amplifiers > 0 {
                    words.said("amplifiers", &[("count", &count), ("chip", &chips.speaker)])
                } else {
                    chips.speaker.clone()
                },
            ));
        } else if !device.sink && !chips.microphone.is_empty() {
            figures.push((
                "microphones",
                if chips.digital {
                    words.said("digital", &[("chip", &chips.microphone)])
                } else {
                    chips.microphone.clone()
                },
            ));
        }
    }
    figures.push(("usb-id", chips.usb));
    if device.property("device.bus") != "pci" {
        figures.push(("card", device.property("alsa.card_name").to_owned()));
    }
    let bits = device.property("alsa.resolution_bits");
    if !bits.is_empty() {
        figures.push(("resolution", words.said("bits", &[("count", bits)])));
    }
    figures.retain(|(_, said)| !said.is_empty());
    figures
}

/// A device's Hardware inside its row, saying its first figure, Return
/// copying it and a step in holding each, or nothing where its card says
/// none.
pub(crate) fn hardware_rows(device: &Device, words: &Words, parent: &str) -> Vec<Value> {
    let figures = figures(device, words);
    let Some((_, first)) = figures.first() else {
        return Vec::new();
    };
    let part = format!("{parent}:hardware");
    let thing = format!("sound:{}:hardware", device.name);
    let mut rows = vec![json!({
        "key": part,
        "thing": thing,
        "place": PLACE,
        "kind": "figures",
        "parent": parent,
        "name": words.name("sound:hardware", &[]),
        "glyph": kind(device),
        "hue": "machine",
        "depth": 1,
        "side": [{"said": first}],
        "note": "",
        "acts": [{"key": "verb:copy", "copy": first}, {"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name},
    })];
    rows.extend(figures.iter().map(|(id, said)| {
        json!({
            "key": format!("{part}:{id}"),
            "thing": format!("{thing}:{id}"),
            "place": PLACE,
            "kind": "figure",
            "parent": part,
            "name": words.name(&format!("sound:{id}"), &[]),
            "glyph": "",
            "hue": "machine",
            "depth": 2,
            "side": [{"said": said}],
            "note": "",
            "acts": [{"key": "verb:copy", "copy": said}],
            "held": {"node": device.name, "figure": id},
        })
    }));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::tests::{device, words};

    fn figures_of(record: &Value) -> Vec<(String, String)> {
        let device = Device::of(record).expect("a device");
        hardware_rows(&device, &words(), "row:sound:it")
            .iter()
            .skip(1)
            .map(|row| {
                let said = row["side"][0]["said"].as_str().unwrap_or("");
                (
                    row["name"].as_str().unwrap_or("").to_owned(),
                    said.to_owned(),
                )
            })
            .collect()
    }

    const MACHINES: &str = "cfg-amp:4 hs:cs42l45 mic:cs42l45-dmic spk:cs35l56";

    #[test]
    fn a_cards_components_name_its_chips() {
        let chips = components(MACHINES);
        assert_eq!(
            (
                chips.amplifiers,
                chips.speaker.as_str(),
                chips.headset.as_str()
            ),
            (4, "CS35L56", "CS42L45"),
            "{chips:?}"
        );
        assert!(chips.digital, "its microphones are digital");
        assert_eq!(chips.microphone, "CS42L45");
        assert_eq!(components("USB0D8C:0014 junk:").usb, "0d8c:0014");
        assert_eq!(
            components("USBgggg:0014"),
            Chips::default(),
            "no ids, nothing"
        );
    }

    // The machine's speakers say their amplifiers, its microphones their
    // digital chip, its headphones the headset's codec, each by its bits;
    // a card built in is never named.
    #[test]
    fn the_machines_own_say_the_chip_each_goes_through() {
        let properties = |extra: Value| {
            let mut all = json!({"alsa.components": MACHINES, "device.bus": "pci",
                "alsa.card_name": "sof-soundwire", "alsa.resolution_bits": "24"});
            all.as_object_mut()
                .expect("an object")
                .extend(extra.as_object().expect("an object").clone());
            all
        };
        let speakers = device("speakers", true, "Speakers", &properties(json!({})));
        let microphones = device("mics", false, "Microphones", &properties(json!({})));
        let headphones = device(
            "headphones",
            true,
            "Headphones",
            &properties(json!({"device.profile.description": "Headphones"})),
        );
        let said = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(name, said)| ((*name).to_owned(), (*said).to_owned()))
                .collect()
        };
        assert_eq!(
            figures_of(&speakers),
            said(&[("Amplifiers", "4 × CS35L56"), ("Resolution", "24 bit")])
        );
        assert_eq!(
            figures_of(&microphones),
            said(&[
                ("Microphones", "CS42L45, digital"),
                ("Resolution", "24 bit")
            ])
        );
        assert_eq!(
            figures_of(&headphones),
            said(&[("Chip", "CS42L45"), ("Resolution", "24 bit")])
        );
    }

    // A USB card says its ids and its name, and no chip where desk-facts
    // names it among the facts after; one saying nothing has no Hardware.
    #[test]
    fn a_usb_card_says_its_ids_and_name_and_no_chip_desk_facts_names() {
        let mut desk = device(
            "desk",
            true,
            "Desk",
            &json!({"alsa.components": "USB0d8c:0014 spk:cm108", "device.bus": "usb",
                "alsa.card_name": "Desk Speakers"}),
        );
        desk["facts"] = json!({"key": "Desk:out",
            "said": [{"name": "codec", "value": "CM108"}]});
        let device_of = Device::of(&desk).expect("a device");
        let rows = hardware_rows(&device_of, &words(), "row:sound:desk");
        assert_eq!(
            rows[0]["side"][0]["said"], "0d8c:0014",
            "its first figure leads"
        );
        assert_eq!(rows[1]["thing"], "sound:desk:hardware:usb-id");
        assert_eq!(rows.len(), 3, "its ids and its name: {rows:?}");
        let made = device("made", true, "Made", &json!({"device.bus": "pci"}));
        let made_of = Device::of(&made).expect("a device");
        assert!(
            hardware_rows(&made_of, &words(), "row:sound:made").is_empty(),
            "a card saying nothing has no Hardware"
        );
    }
}
