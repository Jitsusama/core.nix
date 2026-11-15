//! An Apple TV as a source: what its `AirPlay` receiver says of itself at
//! `/info`, asked of each Apple TV eero lists (contract/airplay/ holds a
//! real one's answer, redacted). The Apple TV is the device eero already
//! lists by its hardware address, so it stays a client there; what model
//! it is and the build it runs become its own, and the rest of its answer
//! stays under `detail.airplay`, apart from the keys it pairs with and
//! the address it was asked from.
//!
//! Only `/info` is read, without pairing: what it's playing, its volume
//! and its remote wait on Joel's yes to pair, since tvOS says them only
//! to a paired device.

use std::fmt::Write;
use std::io::Cursor;

use serde_json::{Map, Number, Value};

use crate::model::{Source, SourceState};

/// The source's name, and the key its answer sits under in `detail`.
pub const NAME: &str = "airplay";

/// The port every `AirPlay` receiver listens on.
pub const PORT: u16 = 7000;

/// What an Apple TV answers at `/info`: a binary property list.
pub type Answer = Vec<u8>;

/// The keys of an answer net leaves out: those that pair a device with it,
/// since they say nothing of the network and a pairing is Joel's to make,
/// and the address the ask came from, which is ours, on a new port each
/// minute.
const LEFT_OUT: [&str; 4] = ["pk", "pi", "psi", "senderAddress"];

/// Where an Apple TV says what it is.
pub fn url(address: &str) -> String {
    format!("http://{address}:{PORT}/info")
}

/// What an Apple TV said, for the device eero lists by `mac`.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub mac: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub detail: Value,
}

/// The Apple TVs eero lists, each by its hardware address and the address
/// eero reaches it at: a device Apple made that eero takes for a media
/// streamer, so the phones, tablets and Macs aren't asked.
pub fn listed(eero: &Value) -> Vec<(String, String)> {
    let Some(Value::Array(devices)) = eero.get("devices") else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|device| {
            let said = |key| device.get(key).and_then(Value::as_str);
            said("make") == Some("Apple") && said("kind") == Some("media_streamer")
        })
        .filter_map(|device| {
            let mac = device.get("mac")?.as_str()?;
            let address = device.get("address")?.as_str()?;
            Some((mac.to_owned(), address.to_owned()))
        })
        .collect()
}

/// Each Apple TV as it answered, and the source, or none when eero lists
/// no Apple TV. The source answers while any Apple TV does.
pub fn read(said: Vec<(String, Result<Answer, String>)>, now: u64) -> (Option<Source>, Vec<Found>) {
    if said.is_empty() {
        return (None, Vec::new());
    }
    let mut found = Vec::new();
    let mut reason = None;
    for (mac, answer) in said {
        match answer.and_then(|answer| heard(mac, &answer)) {
            Ok(one) => found.push(one),
            Err(why) => reason = reason.or(Some(why)),
        }
    }
    let answering = !found.is_empty();
    let source = Source {
        name: NAME.into(),
        what: "the Apple TVs eero lists, read and never changed".into(),
        state: if answering {
            SourceState::Answering
        } else {
            SourceState::Unreachable
        },
        read_at: answering.then_some(now),
        since: (!answering).then_some(now),
        reason: if answering { None } else { reason },
    };
    (Some(source), found)
}

/// One Apple TV from its answer, kept whole in its own words but for the
/// keys it pairs with.
fn heard(mac: String, answer: &[u8]) -> Result<Found, String> {
    let list = plist::Value::from_reader(Cursor::new(answer))
        .map_err(|error| format!("its answer isn't a property list: {error}"))?;
    let Value::Object(mut detail) = json(list) else {
        return Err("its answer isn't a dictionary".into());
    };
    for key in LEFT_OUT {
        detail.remove(key);
    }
    let said = |key: &str| detail.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(Found {
        mac,
        model: said("model").map(|model| named(&model)),
        firmware: said("osBuildVersion"),
        detail: Value::Object(detail),
    })
}

/// A property list's value in JSON: data as hexadecimal, since JSON holds
/// no bytes, and a date in its XML form.
fn json(value: plist::Value) -> Value {
    match value {
        plist::Value::Array(items) => Value::Array(items.into_iter().map(json).collect()),
        plist::Value::Dictionary(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key, json(value)))
                .collect::<Map<_, _>>(),
        ),
        plist::Value::Boolean(yes) => Value::Bool(yes),
        plist::Value::Data(bytes) => {
            Value::String(bytes.iter().fold(String::new(), |mut hex, byte| {
                // Writing to a String never fails.
                let _ = write!(hex, "{byte:02x}");
                hex
            }))
        }
        plist::Value::Date(date) => Value::String(date.to_xml_format()),
        plist::Value::Real(real) => Number::from_f64(real).map_or(Value::Null, Value::Number),
        plist::Value::Integer(integer) => integer
            .as_unsigned()
            .map(Value::from)
            .or_else(|| integer.as_signed().map(Value::from))
            .unwrap_or(Value::Null),
        plist::Value::String(text) => Value::String(text),
        _ => Value::Null,
    }
}

/// The name Apple sells a model under, from the identifier it answers
/// with; one not named here is said as its identifier.
fn named(identifier: &str) -> String {
    match identifier {
        "AppleTV5,3" => "Apple TV HD".into(),
        "AppleTV6,2" => "Apple TV 4K (1st generation)".into(),
        "AppleTV11,1" => "Apple TV 4K (2nd generation)".into(),
        "AppleTV14,1" => "Apple TV 4K (3rd generation)".into(),
        other => other.into(),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs;

    use serde_json::json;

    /// The contract's Apple TV: its hardware address and address, as the
    /// eero record the collector's tests add it to gives them.
    pub const MAC: &str = "02:00:00:00:00:80";
    pub const ADDRESS: &str = "10.0.0.80";

    const NOW: u64 = 1_794_702_033;

    /// What the contract's Apple TV answered.
    pub fn answer() -> Answer {
        let file = format!("{}/contract/airplay/info.plist", env!("CARGO_MANIFEST_DIR"));
        fs::read(&file).expect("the contract has the Apple TV's answer")
    }

    /// The Apple TV as eero lists it.
    pub fn listing() -> Value {
        json!({
            "mac": MAC, "name": "Apple TV", "nickname": null,
            "hostname": "Den", "kind": "media_streamer", "make": "Apple",
            "manufacturer": "Apple, Inc.", "address": ADDRESS,
            "addresses": [ADDRESS], "node": "2001", "ethernet": false,
            "band": "5 GHz", "channel": 149,
        })
    }

    #[test]
    fn an_apple_eero_takes_for_a_media_streamer_is_listed_and_no_other() {
        let mut phone = listing();
        phone["mac"] = json!("02:00:00:00:00:81");
        phone["kind"] = json!("phone");
        let mut roku = listing();
        roku["mac"] = json!("02:00:00:00:00:82");
        roku["make"] = json!("Roku");
        let eero = json!({"devices": [listing(), phone, roku]});
        assert_eq!(
            listed(&eero),
            vec![(MAC.to_owned(), ADDRESS.to_owned())],
            "only the Apple media streamer is asked"
        );
    }

    #[test]
    fn the_answer_says_the_model_by_its_name_and_the_build_it_runs() {
        let (_, found) = read(vec![(MAC.into(), Ok(answer()))], NOW);
        let found = found.first().expect("the Apple TV is heard");
        assert_eq!(
            found.model.as_deref(),
            Some("Apple TV 4K (1st generation)"),
            "its model"
        );
        assert_eq!(found.firmware.as_deref(), Some("23L781"), "its build");
        assert_eq!(
            found.detail["model"],
            json!("AppleTV6,2"),
            "its identifier is kept"
        );
        assert_eq!(
            found.detail["receiverHDRCapability"],
            json!("4k30"),
            "its answer is kept"
        );
    }

    #[test]
    fn the_keys_it_pairs_with_and_the_askers_address_are_left_out() {
        let (_, found) = read(vec![(MAC.into(), Ok(answer()))], NOW);
        let detail = &found.first().expect("the Apple TV is heard").detail;
        for key in LEFT_OUT {
            assert!(detail.get(key).is_none(), "{key} is kept");
        }
    }

    #[test]
    fn an_answer_that_isnt_a_property_list_says_so() {
        let (source, found) = read(vec![(MAC.into(), Ok(b"<html>".to_vec()))], NOW);
        let source = source.expect("the source is said");
        assert!(found.is_empty(), "nothing is heard from a page");
        assert_eq!(source.state, SourceState::Unreachable, "the source answers");
        assert!(
            source
                .reason
                .is_some_and(|why| why.contains("property list")),
            "the reason doesn't say what was wrong"
        );
    }

    #[test]
    fn no_apple_tv_listed_is_no_source() {
        assert_eq!(
            read(Vec::new(), NOW),
            (None, Vec::new()),
            "a source is said for nothing"
        );
    }
}
