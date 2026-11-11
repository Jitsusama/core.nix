//! Sound's rows, served ready in the form every daemon serves its rows in
//! (../ready-rows): each output and input the launcher lists, outputs
//! first and each way A to Z by the name it's given, worded from the
//! launcher's words.json, so the shell only places and draws them.
//!
//! They're worked out from the device records (devices.rs), as the
//! launcher's Sound.qml works its rows out from the sound server's nodes
//! today, rule for rule: what a device is called, the glyph it wears,
//! whether it's the one chosen, its volume as a bar and its note. Whether
//! a jack with nothing plugged in or a display's output with no display
//! is listed comes next, from desk-facts and from the shell telling the
//! displays it has.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde_json::{Value, json};

/// The launcher's place for Sound's rows.
pub const PLACE: &str = "sound";

/// A middle dot between the parts of a note, as the launcher's are.
const APART: &str = " \u{b7} ";

/// The machine's own speakers, microphones and jack, by the end of the
/// node name the sound server gives them for their port, and the
/// vocabulary's word for each, as Sounds.js's `BUILT_IN`: their own names
/// ("Speaker", "Microphones") read as one speaker, many microphones, and
/// nothing of where they are.
const BUILT_IN: [(&str, &str); 3] = [
    ("HiFi__Speaker__sink", "speakers"),
    ("HiFi__Headphones__sink", "jack"),
    ("HiFi__Headset__source", "jack-microphone"),
];

/// The launcher's words for Sound: its things' names by id, and what
/// Sound says, by its own ids, as words.json holds them.
#[derive(Debug, Default)]
pub struct Words {
    names: BTreeMap<String, String>,
    said: BTreeMap<String, String>,
}

impl Words {
    /// The words of the launcher's vocabulary, words.json as it's read.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Words> {
        let vocabulary: Value = serde_json::from_str(text)?;
        let things = vocabulary["things"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        let mut words = Words::default();
        for thing in things {
            let (Some(id), Some(name)) = (thing["id"].as_str(), thing["name"].as_str()) else {
                continue;
            };
            words.names.insert(id.to_owned(), name.to_owned());
            if id == PLACE
                && let Some(said) = thing["said"].as_object()
            {
                for (key, value) in said {
                    if let Some(value) = value.as_str() {
                        words.said.insert(key.clone(), value.to_owned());
                    }
                }
            }
        }
        Ok(words)
    }

    /// What Sound says by an id, with what's in braces filled in, or the
    /// id itself where the vocabulary has no words for it, as Sound.qml's
    /// `said` does.
    fn said(&self, id: &str, fill: &[(&str, &str)]) -> String {
        let words = self.said.get(id).map_or(id, String::as_str);
        filled(words, fill)
    }

    /// A thing's name by its id, with what's in braces filled in, or ""
    /// where the vocabulary names no such thing.
    fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        self.names
            .get(id)
            .map_or_else(String::new, |words| filled(words, fill))
    }
}

fn filled(words: &str, fill: &[(&str, &str)]) -> String {
    let mut said = words.to_owned();
    for (key, value) in fill {
        said = said.replace(&format!("{{{key}}}"), value);
    }
    said
}

/// One device as its record says it, read once for each of its rules.
struct Device<'a> {
    record: &'a Value,
    name: &'a str,
    sink: bool,
}

impl<'a> Device<'a> {
    fn of(record: &'a Value) -> Option<Device<'a>> {
        Some(Device {
            record,
            name: record["name"].as_str()?,
            sink: record["sink"].as_bool()?,
        })
    }

    fn property(&self, key: &str) -> &'a str {
        self.record["properties"][key].as_str().unwrap_or("")
    }

    /// Its own short name, rather than the sound card's long one before
    /// it.
    fn nicknamed(&self) -> &'a str {
        [self.text("nick"), self.text("description"), self.name]
            .into_iter()
            .find(|name| !name.is_empty())
            .unwrap_or("")
    }

    fn text(&self, field: &str) -> &'a str {
        self.record[field].as_str().unwrap_or("")
    }

    /// Its Bluetooth profile, "a2dp-sink", or "" for none of its own.
    fn profile(&self) -> &'a str {
        self.property("api.bluez5.profile")
    }

    fn muted(&self) -> bool {
        self.record["muted"].as_bool().unwrap_or(false)
    }

    fn chosen(&self) -> bool {
        self.record["chosen"].as_bool().unwrap_or(false)
    }

    /// How loud it's heard, as a share: none while it's muted.
    fn audible(&self) -> f64 {
        if self.muted() {
            return 0.0;
        }
        self.record["volume"].as_f64().unwrap_or(0.0)
    }

    /// A display's output, by what the sound server calls it.
    fn display(&self) -> bool {
        let known = format!("{} {}", self.name, self.text("description")).to_lowercase();
        known.contains("hdmi") || known.contains("displayport")
    }

    /// The Bluetooth address its sound is of, from its properties or, for
    /// a headset microphone's stand-in, its name, or "".
    fn address(&self) -> &'a str {
        let said = self.property("api.bluez5.address");
        if !said.is_empty() {
            return said;
        }
        self.name
            .strip_prefix("bluez_input.")
            .filter(|address| is_address(address))
            .unwrap_or("")
    }
}

/// Whether a name's rest is a Bluetooth address, `AA:BB:CC:DD:EE:FF`, as
/// `WirePlumber` names a headset microphone's stand-in for it.
fn is_address(text: &str) -> bool {
    let parts: Vec<&str> = text.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// The vocabulary's word for what a device is called, by its node's
/// name: the machine's own by their port, a headset's microphone as its,
/// or "" for one whose own name says what it is. A microphone that shares
/// its name with an output, as a USB microphone with a headphone socket
/// does, is its device's microphone too, so the two rows never read the
/// same.
pub fn called(name: &str, shared: bool) -> &'static str {
    if let Some((_, word)) = BUILT_IN.iter().find(|(end, _)| name.ends_with(end)) {
        return word;
    }
    if built_in_microphones(name) {
        return "microphones";
    }
    if name.starts_with("bluez_input.") || shared {
        return "its-microphone";
    }
    ""
}

/// Whether a node name ends as the machine's own microphones' do,
/// `HiFi__Mic1__source` or `HiFi__DMic__source`.
fn built_in_microphones(name: &str) -> bool {
    let Some(rest) = name.strip_suffix("__source") else {
        return false;
    };
    let Some((_, port)) = rest.rsplit_once("HiFi__") else {
        return false;
    };
    let port = port.strip_prefix('D').unwrap_or(port);
    port.strip_prefix("Mic")
        .is_some_and(|number| number.chars().all(|c| c.is_ascii_digit()))
}

/// What a device is, from what the sound server knows of it and its
/// card, which says what `BlueZ` says a Bluetooth one is: worn, through a
/// display, or the machine's own. Headphones are headphones and a speaker
/// a speaker to Joel, however they're joined.
fn kind(device: &Device) -> &'static str {
    let known = [
        device.name,
        device.nicknamed(),
        device.property("device.form-factor"),
        device.text("form"),
    ]
    .join(" ")
    .to_lowercase();
    if !device.sink {
        return if known.contains("headset") {
            "headset"
        } else {
            "microphone"
        };
    }
    if device.display() {
        return "display";
    }
    if known.contains("head") {
        return "headphones";
    }
    "speaker"
}

/// A name's runs of digits and of the rest, the rest in lower case, as
/// Order.js's `pieces`.
fn pieces(name: &str) -> Vec<String> {
    let mut runs: Vec<String> = Vec::new();
    for c in name.to_lowercase().chars() {
        let digit = c.is_ascii_digit();
        match runs.last_mut() {
            Some(run) if run.chars().all(|d| d.is_ascii_digit()) == digit => run.push(c),
            _ => runs.push(c.to_string()),
        }
    }
    runs
}

/// A to Z, as names are read: numbers by their value, so Tab 9 comes
/// before Tab 10, as Order.js's `byName`.
fn by_name(one: &str, other: &str) -> Ordering {
    let (a, b) = (pieces(one), pieces(other));
    for (a, b) in a.iter().zip(&b) {
        let numbers = a.starts_with(|c: char| c.is_ascii_digit())
            && b.starts_with(|c: char| c.is_ascii_digit());
        let said = if numbers {
            let value = |run: &str| run.parse::<u128>().unwrap_or(u128::MAX);
            value(a).cmp(&value(b))
        } else {
            a.cmp(b)
        };
        if said != Ordering::Equal {
            return said;
        }
    }
    a.len().cmp(&b.len())
}

/// A share as a bar, in twentieths, a share above nothing one at least,
/// so a level that leaves the drawing as it was changes no row.
fn bar(share: f64) -> Value {
    let mut steps = (share.clamp(0.0, 1.0) * 20.0).round();
    if share > 0.0 && steps < 1.0 {
        steps = 1.0;
    }
    json!({"shape": "bar", "values": [steps / 20.0], "warn": false})
}

/// A share as the launcher says it, "40%".
fn percent(share: f64) -> String {
    format!("{}%", (share * 100.0).round())
}

/// Every row Sound lists, outputs first and each way by name.
pub fn rows(records: &[Value], words: &Words) -> Vec<Value> {
    let devices: Vec<Device> = records
        .iter()
        .filter_map(Device::of)
        .filter(|device| device.property("filter.smart") != "true")
        .collect();
    let named: Vec<String> = devices
        .iter()
        .map(|device| named(device, &devices, words))
        .collect();
    let mut order: Vec<usize> = (0..devices.len()).collect();
    order.sort_by(|&one, &other| {
        devices[other]
            .sink
            .cmp(&devices[one].sink)
            .then_with(|| by_name(&named[one], &named[other]))
    });
    order
        .into_iter()
        .map(|at| row(&devices[at], &named[at], &devices, words))
        .collect()
}

/// What a device is called, saying the thing itself (Sound.qml's `named`).
fn named(device: &Device, devices: &[Device], words: &Words) -> String {
    let own = device.nicknamed();
    let shared = !device.sink
        && devices
            .iter()
            .any(|other| other.sink && other.nicknamed() == own);
    match called(device.name, shared) {
        "" => own.to_owned(),
        word => {
            let name = words.name(&format!("sound:called:{word}"), &[("device", own)]);
            if name.is_empty() {
                own.to_owned()
            } else {
                name
            }
        }
    }
}

/// Whether listening through it would switch a headset from music to
/// calls, as anything reading its microphone's stand-in does: an input
/// that's `WirePlumber`'s stand-in, its headset's profile carrying music.
fn switches(device: &Device, devices: &[Device]) -> bool {
    if device.sink || device.property("bluez5.loopback") != "true" {
        return false;
    }
    let address = device.address();
    if address.is_empty() {
        return false;
    }
    let headset = if device.profile().is_empty() {
        devices
            .iter()
            .find(|one| one.property("api.bluez5.address") == address && !one.profile().is_empty())
    } else {
        Some(device)
    };
    headset.is_some_and(|headset| headset.profile().starts_with("a2dp"))
}

fn row(device: &Device, name: &str, devices: &[Device], words: &Words) -> Value {
    let glyph = match (device.muted(), device.sink) {
        (true, true) => "silent",
        (true, false) => "muted",
        (false, _) => kind(device),
    };
    let volume = percent(device.record["volume"].as_f64().unwrap_or(0.0));
    let mut note = vec![words.said("volume", &[("level", &volume)])];
    if device.muted() {
        note.push(words.said("muted", &[]));
    }
    if switches(device, devices) {
        note.push(words.said("to-calls", &[]));
    }
    let side: Vec<Value> = if device.chosen() {
        vec![json!({"said": words.said("chosen", &[])})]
    } else {
        Vec::new()
    };
    json!({
        "key": format!("row:{PLACE}:{}", device.name),
        "thing": format!("sound:{}", device.name),
        "place": PLACE,
        "kind": "device",
        "name": name,
        "glyph": glyph,
        "hue": "machine",
        "depth": 0,
        "side": side,
        "graph": bar(device.audible()),
        "note": note.join(APART),
        // Return steps in, as every device's does, to its parts.
        "acts": [{"key": "verb:more", "step": "rows"}],
        "held": {"node": device.name, "sink": device.sink},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(
            &json!({"things": [
                {"id": "sound", "name": "Sound", "said": {
                    "volume": "volume {level}", "muted": "muted", "chosen": "chosen",
                    "to-calls": "using it switches the headset to calls",
                }},
                {"id": "sound:called:speakers", "name": "Built-In Speakers"},
                {"id": "sound:called:microphones", "name": "Built-In Microphones"},
                {"id": "sound:called:its-microphone", "name": "{device} Microphone"},
            ]})
            .to_string(),
        )
        .expect("the words read")
    }

    fn device(name: &str, sink: bool, nick: &str, properties: &Value) -> Value {
        json!({
            "key": format!("device:{name}"), "kind": "device", "name": name,
            "nick": nick, "description": "", "sink": sink, "volume": 0.4,
            "muted": false, "chosen": false, "properties": properties, "form": "",
        })
    }

    fn names(rows: &[Value]) -> Vec<&str> {
        rows.iter()
            .map(|row| row["name"].as_str().unwrap_or(""))
            .collect()
    }

    // The machine's own are named for the thing itself, a microphone
    // sharing an output's name is that device's, and outputs come first,
    // each way A to Z, numbers by their value.
    #[test]
    fn devices_are_named_for_what_they_are_and_listed_in_order() {
        let none = json!({});
        let records = [
            device("alsa_input.pci.HiFi__Mic1__source", false, "Mic1", &none),
            device("usb-mv7-in", false, "MV7", &none),
            device("usb-mv7-out", true, "MV7", &none),
            device(
                "alsa_output.pci.HiFi__Speaker__sink",
                true,
                "Speaker",
                &none,
            ),
            device("usb-desk-10", true, "Desk 10", &none),
            device("usb-desk-9", true, "Desk 9", &none),
        ];
        let rows = rows(&records, &words());
        assert_eq!(
            names(&rows),
            [
                "Built-In Speakers",
                "Desk 9",
                "Desk 10",
                "MV7",
                "Built-In Microphones",
                "MV7 Microphone",
            ]
        );
        assert_eq!(
            rows[0]["key"],
            "row:sound:alsa_output.pci.HiFi__Speaker__sink"
        );
        assert_eq!(
            rows[0]["thing"],
            "sound:alsa_output.pci.HiFi__Speaker__sink"
        );
        assert_eq!(
            rows[0]["held"],
            json!({"node": "alsa_output.pci.HiFi__Speaker__sink", "sink": true})
        );
    }

    // A row wears what the device is, struck through while muted, says
    // its volume as a bar in twentieths, and is marked where it's chosen.
    #[test]
    fn a_row_says_how_a_device_stands() {
        let none = json!({});
        let mut speakers = device("usb-desk", true, "Desk", &none);
        speakers["chosen"] = json!(true);
        speakers["volume"] = json!(0.4321);
        let mut microphone = device("usb-desk-in", false, "Desk Mic", &none);
        microphone["muted"] = json!(true);
        let mut quiet = device("usb-quiet", true, "Quiet", &none);
        quiet["volume"] = json!(0.01);
        let rows = rows(&[speakers, microphone, quiet], &words());
        assert_eq!(rows[0]["glyph"], "speaker");
        assert_eq!(rows[0]["side"], json!([{"said": "chosen"}]));
        assert_eq!(
            rows[0]["graph"],
            json!({"shape": "bar", "values": [0.45], "warn": false})
        );
        assert_eq!(rows[0]["note"], "volume 43%");
        assert_eq!(rows[1]["graph"]["values"], json!([0.05]));
        assert_eq!(rows[2]["glyph"], "muted");
        assert_eq!(rows[2]["side"], json!([]));
        assert_eq!(rows[2]["graph"]["values"], json!([0.0]));
        assert_eq!(rows[2]["note"], "volume 40% \u{b7} muted");
    }

    // Bluetooth headphones are headphones by what BlueZ says of their
    // card, a headset's microphone is its, and listening through its
    // stand-in while the headset plays music would switch it to calls.
    #[test]
    fn a_headset_says_what_it_is_and_what_listening_would_do() {
        let address = "AA:BB:CC:DD:EE:FF";
        let mut headphones = device(
            "bluez_output.AA_BB_CC_DD_EE_FF.1",
            true,
            "WH-1000XM4",
            &json!({"api.bluez5.address": address, "api.bluez5.profile": "a2dp-sink"}),
        );
        headphones["form"] = json!("headphone audio-headphones");
        let stand_in = device(
            &format!("bluez_input.{address}"),
            false,
            "WH-1000XM4",
            &json!({"bluez5.loopback": "true"}),
        );
        let rows = rows(&[headphones, stand_in], &words());
        assert_eq!(rows[0]["glyph"], "headphones");
        assert_eq!(names(&rows), ["WH-1000XM4", "WH-1000XM4 Microphone"]);
        assert_eq!(
            rows[1]["note"],
            "volume 40% \u{b7} using it switches the headset to calls"
        );
        assert_eq!(rows[0]["note"], "volume 40%");
    }

    // Kept as every daemon keeps its rows, a volume moving by less than
    // the launcher shows sends nothing, and moving by more sends its own
    // device's row alone.
    #[test]
    fn a_volume_moving_sends_its_own_row_alone() {
        let none = json!({});
        let mut records = vec![device("a", true, "A", &none), device("b", true, "B", &none)];
        let mut kept = ready_rows::kept::Kept::default();
        kept.take(rows(&records, &words()), Vec::new());
        records[0]["volume"] = json!(0.401);
        assert!(
            kept.take(rows(&records, &words()), Vec::new()).is_empty(),
            "a move the launcher wouldn't show sends nothing"
        );
        records[0]["volume"] = json!(0.6);
        let moved = kept.take(rows(&records, &words()), Vec::new());
        let keys: Vec<&Value> = moved.changed.iter().map(|record| &record["key"]).collect();
        assert_eq!(keys, ["row:sound:a"]);
        assert_eq!(moved.changed[0]["row"]["note"], "volume 60%");
    }

    // A smart filter in front of a device, as the speakers' tuning is,
    // isn't a device of its own.
    #[test]
    fn a_filter_isnt_listed() {
        let filter = device(
            "speakers-tuned",
            true,
            "Tuned",
            &json!({"filter.smart": "true"}),
        );
        assert_eq!(rows(&[filter], &words()), Vec::<Value>::new());
    }
}
