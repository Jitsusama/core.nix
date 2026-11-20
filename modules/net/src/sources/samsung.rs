//! A Samsung TV as a source: what it says of itself at `:8001/api/v2/`,
//! asked of each Samsung box eero lists that could be one
//! (contract/samsung/ holds a real one's answer, redacted). The TV is
//! the device eero already lists by its hardware address; it becomes a
//! television, its model, its own name and whether it's on or in standby
//! become its own, and the rest of its answer stays under
//! `detail.samsung`, apart from its ids and the addresses eero already
//! gives.
//!
//! Only that page is read, without pairing: sending it a key waits on
//! Joel pressing Allow on the TV, and what it's playing, its volume and
//! its input are said only through Samsung's cloud.

use serde_json::{Map, Value};

use crate::model::{Source, SourceState, Television};

/// The source's name, and the key its answer sits under in `detail`.
pub const NAME: &str = "samsung";

/// The port a Samsung TV says what it is on, to anyone, without signing in.
pub const PORT: u16 = 8001;

/// What a TV answers at `/api/v2/`: JSON, as its text.
pub type Answer = String;

/// What a Samsung TV calls itself, which a phone or a soundbar answering
/// on the same port never does.
const A_TV: &str = "Samsung SmartTV";

/// The kinds eero gives a Samsung box that could be a TV: one it can't
/// place, as it does Joel's, and one that streams. A phone, a tablet or a
/// computer is never asked.
const KINDS: [&str; 2] = ["generic", "media_streamer"];

/// The keys of an answer, and of its `device`, that net leaves out: ids
/// that say nothing of the network, and the addresses eero's device for
/// it already holds.
const LEFT_OUT: [&str; 2] = ["id", "uri"];
const LEFT_OUT_OF_DEVICE: [&str; 6] = ["duid", "id", "udn", "ip", "wifiMac", "developerIP"];

/// Where a Samsung TV says what it is.
pub fn url(address: &str) -> String {
    format!("http://{address}:{PORT}/api/v2/")
}

/// What a Samsung TV said, for the device eero lists by `mac`.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub mac: String,
    /// The name it was given on its own screen.
    pub name: Option<String>,
    pub model: Option<String>,
    pub television: Television,
    pub detail: Value,
}

/// The Samsung boxes eero lists that could be a TV, each by its hardware
/// address and the address eero reaches it at.
pub fn listed(eero: &Value) -> Vec<(String, String)> {
    let Some(Value::Array(devices)) = eero.get("devices") else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|device| {
            let said = |key| device.get(key).and_then(Value::as_str);
            said("make") == Some("Samsung")
                && said("kind").is_some_and(|kind| KINDS.contains(&kind))
        })
        .filter_map(|device| {
            let mac = device.get("mac")?.as_str()?;
            let address = device.get("address")?.as_str()?;
            Some((mac.to_owned(), address.to_owned()))
        })
        .collect()
}

/// Each TV as it answered, and the source, or none when eero lists no
/// Samsung box that could be one. The source answers while any TV does.
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
        what: "the Samsung TVs eero lists, read and never changed".into(),
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

/// One TV from its answer, kept whole in its own words but for its ids
/// and addresses.
fn heard(mac: String, answer: &str) -> Result<Found, String> {
    let Ok(Value::Object(mut detail)) = serde_json::from_str::<Value>(answer) else {
        return Err("its answer isn't a JSON object".into());
    };
    for key in LEFT_OUT {
        detail.remove(key);
    }
    let mut device = match detail.remove("device") {
        Some(Value::Object(device)) => device,
        _ => Map::new(),
    };
    let kind = device.get("type").and_then(Value::as_str);
    if kind != Some(A_TV) {
        return Err(format!(
            "it isn't a TV: it says it's {}",
            kind.unwrap_or("nothing")
        ));
    }
    for key in LEFT_OUT_OF_DEVICE {
        device.remove(key);
    }
    let said = |key: &str| device.get(key).and_then(Value::as_str).map(str::to_owned);
    let (name, model, power) = (said("name"), said("modelName"), said("PowerState"));
    detail.insert("device".into(), Value::Object(device));
    Ok(Found {
        mac,
        name,
        model,
        television: Television { power },
        detail: Value::Object(detail),
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs;

    use serde_json::json;

    /// The contract's TV: its hardware address and address, as the eero
    /// record the collector's tests add it to gives them.
    pub const MAC: &str = "02:00:00:00:00:90";
    pub const ADDRESS: &str = "10.0.0.90";

    const NOW: u64 = 1_794_702_033;

    /// What the contract's TV answered.
    pub fn answer() -> Answer {
        let file = format!(
            "{}/contract/samsung/api-v2.json",
            env!("CARGO_MANIFEST_DIR")
        );
        fs::read_to_string(&file).expect("the contract has the TV's answer")
    }

    /// The TV as eero lists it: by the name it gives when it takes an
    /// address, with no nickname, and as a box eero can't place.
    pub fn listing() -> Value {
        json!({
            "mac": MAC, "name": "samsung", "nickname": null,
            "hostname": "samsung", "kind": "generic", "make": "Samsung",
            "manufacturer": "Samsung Electronics Co.,Ltd", "address": ADDRESS,
            "addresses": [ADDRESS], "node": "2001", "ethernet": false,
            "band": "5 GHz", "channel": 149,
        })
    }

    fn one(answer: Answer) -> (Option<Source>, Vec<Found>) {
        read(vec![(MAC.into(), Ok(answer))], NOW)
    }

    #[test]
    fn a_samsung_that_could_be_a_tv_is_listed_and_no_phone_or_other_maker() {
        let mut phone = listing();
        phone["mac"] = json!("02:00:00:00:00:91");
        phone["kind"] = json!("phone");
        let mut streamer = listing();
        streamer["mac"] = json!("02:00:00:00:00:92");
        streamer["kind"] = json!("media_streamer");
        let mut lg = listing();
        lg["mac"] = json!("02:00:00:00:00:93");
        lg["make"] = json!("LG");
        let eero = json!({"devices": [listing(), phone, streamer, lg]});
        assert_eq!(
            listed(&eero),
            vec![
                (MAC.to_owned(), ADDRESS.to_owned()),
                ("02:00:00:00:00:92".to_owned(), ADDRESS.to_owned())
            ],
            "only the Samsung boxes that could be a TV are asked"
        );
    }

    #[test]
    fn the_answer_says_its_name_model_and_power() {
        let (source, found) = one(answer());
        let found = found.first().expect("the TV is heard");
        assert_eq!(found.name.as_deref(), Some("Living Room"), "its own name");
        assert_eq!(found.model.as_deref(), Some("QN55Q70CAFXZC"), "its model");
        assert_eq!(found.television.power.as_deref(), Some("on"), "its power");
        assert_eq!(
            found.detail["device"]["OS"],
            json!("Tizen"),
            "its answer is kept"
        );
        assert_eq!(
            found.detail["version"],
            json!("2.0.25"),
            "its answer is kept"
        );
        assert_eq!(
            source.map(|source| source.state),
            Some(SourceState::Answering),
            "the source isn't answering"
        );
    }

    #[test]
    fn a_tv_in_standby_says_so() {
        let standby = answer().replace("\"PowerState\": \"on\"", "\"PowerState\": \"standby\"");
        let (_, found) = one(standby);
        assert_eq!(
            found
                .first()
                .and_then(|found| found.television.power.as_deref()),
            Some("standby"),
            "its power isn't as it said"
        );
    }

    #[test]
    fn its_ids_and_addresses_are_left_out() {
        let (_, found) = one(answer());
        let detail = &found.first().expect("the TV is heard").detail;
        for key in LEFT_OUT {
            assert!(detail.get(key).is_none(), "{key} is kept");
        }
        for key in LEFT_OUT_OF_DEVICE {
            assert!(detail["device"].get(key).is_none(), "device.{key} is kept");
        }
    }

    #[test]
    fn a_samsung_that_isnt_a_tv_says_what_it_is() {
        let soundbar = answer().replace(A_TV, "Samsung Soundbar");
        let (source, found) = one(soundbar);
        let source = source.expect("the source is said");
        assert!(found.is_empty(), "a soundbar is heard as a TV");
        assert_eq!(source.state, SourceState::Unreachable, "the source answers");
        assert_eq!(
            source.reason.as_deref(),
            Some("it isn't a TV: it says it's Samsung Soundbar"),
            "the reason doesn't say what it is"
        );
    }

    #[test]
    fn an_answer_that_isnt_json_says_so() {
        let (source, found) = one("<html>".into());
        assert!(found.is_empty(), "nothing is heard from a page");
        assert!(
            source
                .and_then(|source| source.reason)
                .is_some_and(|why| why.contains("JSON")),
            "the reason doesn't say what was wrong"
        );
    }

    #[test]
    fn one_tv_answering_keeps_the_source_answering() {
        let (source, found) = read(
            vec![
                ("02:00:00:00:00:92".into(), Err("it didn't answer".into())),
                (MAC.into(), Ok(answer())),
            ],
            NOW,
        );
        assert_eq!(found.len(), 1, "the TV that answered isn't heard");
        assert_eq!(
            source.map(|source| (source.state, source.reason)),
            Some((SourceState::Answering, None)),
            "one silent box makes the source unreachable"
        );
    }

    #[test]
    fn a_home_with_no_samsung_asks_nothing_and_says_no_source() {
        let mut lg = listing();
        lg["make"] = json!("LG");
        let asked = listed(&json!({"devices": [lg]}));
        assert!(asked.is_empty(), "another maker's box is asked: {asked:?}");
        let said = asked
            .into_iter()
            .map(|(mac, _)| (mac, Ok(answer())))
            .collect();
        assert_eq!(
            read(said, NOW),
            (None, Vec::new()),
            "a source is said for nothing"
        );
    }
}
