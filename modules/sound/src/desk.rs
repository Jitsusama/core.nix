//! What desk-facts reads of each sound card from the kernel
//! (modules/desk-facts), from the sound.json it keeps in its folder in the
//! runtime directory: kept on each device's record as the facts its card
//! is found by, and served last inside the device's row, each copying
//! what it says. The daemon follows the file, so the shell reads none
//! for Sound.

use std::collections::BTreeMap;
use std::path::Path;

use programs::notify::Notify;
use serde_json::{Value, json};

use crate::rows::{Device, PLACE, Words};

/// The file desk-facts keeps what each sound card says in.
pub const DESK: &str = "sound.json";

/// The facts asked of a card most, which lead the rest in the file's
/// order, as Sound.qml's `factRows` led them.
const ASKED_FIRST: [&str; 3] = ["lighting", "playing", "recording"];

/// What desk-facts says of each card, by its key, from its sound.json;
/// none where it says nothing readable, as when it's gone.
#[must_use]
pub fn desk(said: &str) -> BTreeMap<String, Value> {
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|said| said.as_object().cloned())
        .map(|cards| cards.into_iter().collect())
        .unwrap_or_default()
}

/// What desk-facts says in `folder`, or nothing where it has written
/// nothing yet.
#[must_use]
pub fn read(folder: &Path) -> BTreeMap<String, Value> {
    std::fs::read_to_string(folder.join(DESK))
        .map(|said| desk(&said))
        .unwrap_or_default()
}

/// Hears desk-facts write its file again, as when a card is plugged in,
/// and gives `changed` what it says each time, until inotify fails.
/// desk-facts writes the file whole and renames it into place, so a
/// reading never meets it half written; the folder is made here where
/// desk-facts hasn't yet, since it removes only its files when it stops.
pub fn follow(folder: &Path, mut changed: impl FnMut(BTreeMap<String, Value>)) {
    let notify = match Notify::new() {
        Ok(notify) => notify,
        Err(why) => {
            eprintln!("<4>inotify isn't there, so desk-facts writing isn't heard: {why}");
            return;
        }
    };
    let _ = std::fs::create_dir_all(folder);
    if notify.watch(folder).is_none() {
        eprintln!(
            "<4>couldn't watch {}, so desk-facts writing isn't heard",
            folder.display()
        );
        return;
    }
    // Read again once watched, so nothing written before the watch is
    // missed.
    changed(read(folder));
    while let Ok(changes) = notify.changes() {
        if changes.iter().any(|(_, name)| name == DESK) {
            changed(read(folder));
        }
    }
}

/// The key the file says a device's card by: a card with one device each
/// way by its ALSA id and direction ("MV7:in"), one with several, as the
/// machine's own has its speakers, headphones and displays, by its
/// device's number too ("sofsoundwire,2:out"), whichever the file holds.
fn key_of(device: &Device, desk: &BTreeMap<String, Value>) -> Option<String> {
    let card = device.property("alsa.id");
    if card.is_empty() {
        return None;
    }
    let way = if device.sink { "out" } else { "in" };
    let number = [
        device.property("alsa.device"),
        device.property("api.alsa.pcm.device"),
    ]
    .into_iter()
    .find(|number| !number.is_empty());
    number
        .map(|number| format!("{card},{number}:{way}"))
        .into_iter()
        .chain([format!("{card}:{way}")])
        .find(|key| desk.contains_key(key))
}

/// Puts on each device's record what desk-facts says of its card, by the
/// key it's found by; a card it says nothing of has none.
pub fn told(records: &mut [Value], desk: &BTreeMap<String, Value>) {
    for record in records.iter_mut() {
        let Some(key) = Device::of(record).and_then(|device| key_of(&device, desk)) else {
            continue;
        };
        record["facts"] = json!({"key": key, "said": desk[&key]});
    }
}

/// A fact named first as a part is: by the launcher's word for it, else
/// by itself, its first letter a capital, as Provider.qml's `partNamed`.
fn named(id: &str, words: &Words) -> String {
    let name = words.name(&format!("sound:{}", id.replace(' ', "-")), &[]);
    if !name.is_empty() {
        return name;
    }
    let mut letters = id.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// What desk-facts says of a device's card, as rows inside its row, the
/// ones asked most first, each copying what it says and holding the fact
/// it is, so the drawer and its Hardware read them.
pub(crate) fn fact_rows(device: &Device, words: &Words, parent: &str) -> Vec<Value> {
    let facts = &device.record["facts"];
    let Some(card) = facts["key"].as_str() else {
        return Vec::new();
    };
    let said: Vec<(String, String)> = facts["said"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|fact| {
            let id = fact["name"].as_str()?.to_lowercase();
            Some((id, fact["value"].as_str().unwrap_or("").to_owned()))
        })
        .collect();
    let (first, rest): (Vec<_>, Vec<_>) = said
        .into_iter()
        .partition(|(id, _)| ASKED_FIRST.contains(&id.as_str()));
    first
        .into_iter()
        .chain(rest)
        .map(|(id, value)| {
            json!({
                "key": format!("{parent}:fact:{id}"),
                // The key the shell gave the fact while it made it.
                "thing": format!("fact:{PLACE}:{card}:{id}"),
                "place": PLACE,
                "kind": "fact",
                "parent": parent,
                "name": named(&id, words),
                "glyph": "",
                "hue": "voice",
                "depth": 1,
                "side": [{"said": value}],
                "acts": [{"key": "verb:copy", "copy": value}],
                "held": {"node": device.name, "fact": id},
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::tests::{device, words};

    fn said(text: &str) -> BTreeMap<String, Value> {
        desk(text)
    }

    // A card is found by its device's number where the file says one,
    // else by its id and direction alone; one it says nothing of, or a
    // device with no card, has no facts.
    #[test]
    fn a_device_is_told_its_cards_facts() {
        let desk = said(
            r#"{"PCH,1:out": [{"name": "codec", "value": "CS42L45"}],
                "MV7:in": [{"name": "clock", "value": "asynchronous"}]}"#,
        );
        let mut records = vec![
            device(
                "speakers",
                true,
                "Speakers",
                &json!({"alsa.id": "PCH", "alsa.device": "1"}),
            ),
            device(
                "mv7",
                false,
                "MV7",
                &json!({"alsa.id": "MV7", "api.alsa.pcm.device": "0"}),
            ),
            device("mv7-out", true, "MV7", &json!({"alsa.id": "MV7"})),
            device("made", true, "Made", &json!({})),
        ];
        told(&mut records, &desk);
        assert_eq!(records[0]["facts"]["key"], "PCH,1:out");
        assert_eq!(records[1]["facts"]["key"], "MV7:in");
        assert!(records[2]["facts"].is_null(), "a way it says nothing of");
        assert!(records[3]["facts"].is_null(), "a device with no card");
    }

    // Inside its row, the facts asked most lead, the rest in the file's
    // order, each named by the launcher's word or by itself.
    #[test]
    fn a_devices_facts_are_rows_inside_it_the_asked_first() {
        let desk = said(
            r#"{"Desk:out": [{"name": "offers", "value": "48 kHz"},
                {"name": "plugged in", "value": "nothing"},
                {"name": "playing", "value": "48 kHz · 32 bit"}]}"#,
        );
        let mut records = vec![device(
            "desk",
            true,
            "Desk",
            &json!({"alsa.id": "Desk", "alsa.device": "0"}),
        )];
        told(&mut records, &desk);
        let speakers = Device::of(&records[0]).expect("a device");
        let rows = fact_rows(&speakers, &words(), "row:sound:desk");
        let said: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| {
                let name = row["name"].as_str().unwrap_or("");
                (name, row["side"][0]["said"].as_str().unwrap_or(""))
            })
            .collect();
        assert_eq!(
            said,
            [
                ("Playing At", "48 kHz · 32 bit"),
                ("Offers", "48 kHz"),
                ("Plugged in", "nothing")
            ]
        );
        assert_eq!(rows[0]["key"], "row:sound:desk:fact:playing");
        assert_eq!(rows[0]["thing"], "fact:sound:Desk:out:playing");
        assert_eq!(rows[0]["held"]["fact"], "playing");
        assert_eq!(rows[0]["acts"][0]["copy"], "48 kHz · 32 bit");
    }

    #[test]
    fn nothing_readable_says_nothing() {
        assert!(
            desk("not json").is_empty(),
            "a file half there says nothing"
        );
        assert!(desk("[]").is_empty(), "nor one that isn't keyed by card");
    }
}
