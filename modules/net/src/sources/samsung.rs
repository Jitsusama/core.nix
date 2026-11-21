//! A Samsung TV as a source: what it says of itself at `:8001/api/v2/`,
//! asked of each Samsung box eero lists that could be one
//! (contract/samsung/ holds a real one's answer, redacted). The TV is
//! the device eero already lists by its hardware address; it becomes a
//! television, its model, its own name and whether it's on or in standby
//! become its own, and the rest of its answer stays under
//! `detail.samsung`, apart from its ids and the addresses eero already
//! gives.
//!
//! A TV heard once is remembered while the daemon runs, so a minute it
//! doesn't answer keeps its name and role rather than handing it back to
//! eero's name for it; a box never heard as a TV says nothing, so a phone
//! eero can't place doesn't count against the source. A TV asleep stays
//! on eero's list but stops answering, as Joel's Q70C does in standby, so
//! a remembered TV's silence isn't the source failing either.
//!
//! Only that page is read, without pairing: sending it a key waits on
//! Joel pressing Allow on the TV, and what it's playing, its volume and
//! its input are said only through Samsung's cloud.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::model::{Source, SourceState, Television};

/// The source's name, and the key its answer sits under in `detail`.
pub const NAME: &str = "samsung";

/// The port a Samsung TV says what it is on, to anyone, without signing in.
pub const PORT: u16 = 8001;

/// The most of an answer read: a TV's is under 2 KB, so anything past
/// this isn't one, and a box on the network can't fill each minute's
/// record.
pub const LONGEST: u64 = 64 * 1024;

/// What a TV answers at `/api/v2/`: JSON, as its text.
pub type Answer = String;

/// What a Samsung TV calls itself, which a phone or a soundbar answering
/// on the same port never does.
const A_TV: &str = "Samsung SmartTV";

/// The kinds eero gives a Samsung box that could be a TV: one it can't
/// place, as it does Joel's, and one that streams. A box eero places as
/// a phone, a tablet or a computer isn't asked; one it can't place is,
/// and says nothing unless it answers as a TV.
const KINDS: [&str; 2] = ["generic", "media_streamer"];

/// The keys of an answer, and of its `device`, that net leaves out: ids
/// that say nothing of the network, and the addresses eero's device for
/// it already holds, the hardware address of the radio it's on (which
/// the TV calls its `ssid`) among them.
const LEFT_OUT: [&str; 2] = ["id", "uri"];
const LEFT_OUT_OF_DEVICE: [&str; 7] = ["duid", "id", "udn", "ip", "wifiMac", "developerIP", "ssid"];

/// Each TV's last answer, by its hardware address, kept while the daemon
/// runs.
pub type Heard = BTreeMap<String, Answer>;

/// One Samsung box asked this minute: what it answered, and its last
/// answer as a TV when it has given one.
#[derive(Debug, Clone, PartialEq)]
pub struct Asked {
    pub mac: String,
    pub answer: Result<Answer, String>,
    pub earlier: Option<Answer>,
}

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
    /// Its answer, kept whole but for its ids and addresses; none in a
    /// minute it didn't answer.
    pub detail: Option<Value>,
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

/// What `mac` answered this minute, beside its last answer as a TV; an
/// answer that is a TV's becomes the one remembered.
pub fn remember(heard: &mut Heard, mac: String, answer: Result<Answer, String>) -> Asked {
    if let Ok(said) = &answer
        && television(String::new(), said).is_ok()
    {
        heard.insert(mac.clone(), said.clone());
    }
    Asked {
        earlier: heard.get(&mac).cloned(),
        mac,
        answer,
    }
}

/// Forgets each TV eero no longer lists, so the memory holds only the TVs
/// on the network now, however many boxes have answered as one.
pub fn forget_unlisted(heard: &mut Heard, listed: &[(String, String)]) {
    heard.retain(|mac, _| listed.iter().any(|(listed, _)| listed == mac));
}

/// Each TV as it answered, or as it last answered while it's silent, and
/// the source, or none when no box has answered as a TV. The source is
/// unreachable only while no TV answers as one and a TV it has heard
/// answers as something else; a TV that's silent is asleep, so the source
/// still answers.
pub fn read(asked: Vec<Asked>, now: u64) -> (Option<Source>, Vec<Found>) {
    let mut found = Vec::new();
    let mut answering = false;
    let mut odd = None;
    for one in asked {
        let (heard, answered) = match one.answer {
            Ok(answer) => (television(one.mac.clone(), &answer), true),
            Err(why) => (Err(why), false),
        };
        // A box that hasn't answered as a TV this minute is kept, saying
        // so, when it has before, and otherwise left out, so a box that
        // isn't a TV never counts as the source failing.
        match (heard, one.earlier) {
            (Ok(tv), _) => {
                answering = true;
                found.push(tv);
            }
            (Err(why), Some(earlier)) => {
                if let Ok(tv) = television(one.mac, &earlier) {
                    if answered {
                        let name = tv.name.clone().unwrap_or_else(|| "a TV".into());
                        odd = odd.or(Some(format!("{name}: {why}")));
                    }
                    found.push(Found {
                        television: Television {
                            power: None,
                            answering: false,
                        },
                        detail: None,
                        ..tv
                    });
                }
            }
            (Err(_), None) => {}
        }
    }
    if found.is_empty() {
        return (None, found);
    }
    let failing = if answering { None } else { odd };
    let source = Source {
        name: NAME.into(),
        what: "the Samsung TVs eero lists, read and never changed".into(),
        state: if failing.is_none() {
            SourceState::Answering
        } else {
            SourceState::Unreachable
        },
        read_at: failing.is_none().then_some(now),
        since: failing.is_some().then_some(now),
        reason: failing,
    };
    (Some(source), found)
}

/// One TV from its answer, kept whole in its own words but for its ids
/// and addresses, or why the answer isn't a TV's.
fn television(mac: String, answer: &str) -> Result<Found, String> {
    let Ok(Value::Object(mut detail)) = serde_json::from_str::<Value>(answer) else {
        return Err("its answer isn't a TV's: it isn't a JSON object".into());
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
            "its answer isn't a TV's: it says it's {}",
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
        television: Television {
            power,
            answering: true,
        },
        detail: Some(Value::Object(detail)),
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

    /// Another box eero lists, by its hardware address.
    const OTHER: &str = "02:00:00:00:00:92";

    const NOW: u64 = 1_794_702_033;

    /// What the contract's TV answered.
    pub fn answer() -> Answer {
        let file = format!(
            "{}/contract/samsung/api-v2.json",
            env!("CARGO_MANIFEST_DIR")
        );
        fs::read_to_string(&file).expect("the contract has the TV's answer")
    }

    /// The contract's TV, asked once, as the daemon's first minute asks it.
    pub fn asked() -> Asked {
        remember(&mut Heard::new(), MAC.into(), Ok(answer()))
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

    fn silent() -> Result<Answer, String> {
        Err("it didn't answer: timed out".into())
    }

    fn one(answer: Answer) -> (Option<Source>, Vec<Found>) {
        read(
            vec![remember(&mut Heard::new(), MAC.into(), Ok(answer))],
            NOW,
        )
    }

    #[test]
    fn a_samsung_that_could_be_a_tv_is_listed_and_no_phone_or_other_maker() {
        let mut phone = listing();
        phone["mac"] = json!("02:00:00:00:00:91");
        phone["kind"] = json!("phone");
        let mut streamer = listing();
        streamer["mac"] = json!(OTHER);
        streamer["kind"] = json!("media_streamer");
        let mut lg = listing();
        lg["mac"] = json!("02:00:00:00:00:93");
        lg["make"] = json!("LG");
        let eero = json!({"devices": [listing(), phone, streamer, lg]});
        assert_eq!(
            listed(&eero),
            vec![
                (MAC.to_owned(), ADDRESS.to_owned()),
                (OTHER.to_owned(), ADDRESS.to_owned())
            ],
            "only the Samsung boxes that could be a TV are asked"
        );
    }

    #[test]
    fn eeros_golden_record_lists_its_generic_samsung_and_no_other() {
        let eero: Value = serde_json::from_str(include_str!("../../../eero/home-network.json"))
            .expect("eero's golden record reads");
        let asked = listed(&eero);
        assert_eq!(
            asked.len(),
            1,
            "eero's one generic Samsung isn't asked alone: {asked:?}"
        );
    }

    #[test]
    fn the_answer_says_its_name_model_and_power() {
        let (source, found) = one(answer());
        let found = found.first().expect("the TV is heard");
        assert_eq!(found.name.as_deref(), Some("Living Room"), "its own name");
        assert_eq!(found.model.as_deref(), Some("QN55Q70CAFXZC"), "its model");
        assert_eq!(found.television.power.as_deref(), Some("on"), "its power");
        let detail = found.detail.as_ref().expect("its answer is kept");
        assert_eq!(detail["device"]["OS"], json!("Tizen"), "its answer is kept");
        assert_eq!(detail["version"], json!("2.0.25"), "its answer is kept");
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
        let detail = found.first().and_then(|found| found.detail.clone());
        let kept = serde_json::to_string(&detail).expect("its detail writes");
        for said in [
            "00000000-0000-0000-0000-000000000090",
            ADDRESS,
            MAC,
            "02:00:00:00:20:01",
            "0.0.0.0",
        ] {
            assert!(!kept.contains(said), "{said} is kept: {kept}");
        }
        assert!(
            kept.contains("QN55Q70CAFXZC"),
            "the rest of its answer went too: {kept}"
        );
    }

    #[test]
    fn a_samsung_that_isnt_a_tv_is_left_out_without_a_source() {
        let soundbar = answer().replace(A_TV, "Samsung Soundbar");
        let mut heard = Heard::new();
        let asked = remember(&mut heard, MAC.into(), Ok(soundbar));
        assert!(heard.is_empty(), "a soundbar is remembered as a TV");
        assert_eq!(
            read(vec![asked], NOW),
            (None, Vec::new()),
            "a soundbar says something"
        );
    }

    #[test]
    fn a_box_never_heard_as_a_tv_says_nothing_when_silent() {
        let asked = remember(&mut Heard::new(), OTHER.into(), silent());
        assert_eq!(
            read(vec![asked], NOW),
            (None, Vec::new()),
            "a silent phone says something"
        );
    }

    #[test]
    fn a_tv_heard_once_keeps_its_name_while_silent() {
        let mut heard = Heard::new();
        remember(&mut heard, MAC.into(), Ok(answer()));
        let asked = remember(&mut heard, MAC.into(), silent());
        let (source, found) = read(vec![asked], NOW);
        let [tv] = found.as_slice() else {
            panic!("the silent TV should be one: {found:?}");
        };
        assert_eq!(
            tv.name.as_deref(),
            Some("Living Room"),
            "its name went with its answer"
        );
        assert_eq!(
            tv.model.as_deref(),
            Some("QN55Q70CAFXZC"),
            "its model went with its answer"
        );
        assert_eq!(
            tv.television,
            Television {
                power: None,
                answering: false
            },
            "its power is said from an old answer"
        );
        assert_eq!(tv.detail, None, "an old answer is kept as this minute's");
        assert_eq!(
            source.map(|source| (source.state, source.reason)),
            Some((SourceState::Answering, None)),
            "a TV asleep reads as the source failing"
        );
    }

    #[test]
    fn a_tv_heard_once_that_answers_as_something_else_keeps_its_name() {
        let mut heard = Heard::new();
        remember(&mut heard, MAC.into(), Ok(answer()));
        let odd = answer().replace(A_TV, "Samsung Soundbar");
        let asked = remember(&mut heard, MAC.into(), Ok(odd));
        let (source, found) = read(vec![asked], NOW);
        let [tv] = found.as_slice() else {
            panic!("the TV should be kept as one: {found:?}");
        };
        assert_eq!(
            tv.name.as_deref(),
            Some("Living Room"),
            "its name went with its answer"
        );
        assert_eq!(
            tv.model.as_deref(),
            Some("QN55Q70CAFXZC"),
            "its model went with its answer"
        );
        assert!(
            !tv.television.answering,
            "an answer that isn't a TV's reads as one"
        );
        assert_eq!(
            source.map(|source| (source.state, source.reason)),
            Some((
                SourceState::Unreachable,
                Some("Living Room: its answer isn't a TV's: it says it's Samsung Soundbar".into())
            )),
            "an odd answer isn't said as the source failing, with what it answered"
        );
    }

    #[test]
    fn a_tv_eero_no_longer_lists_is_forgotten() {
        let mut heard = Heard::new();
        remember(&mut heard, MAC.into(), Ok(answer()));
        remember(&mut heard, OTHER.into(), Ok(answer()));
        forget_unlisted(&mut heard, &[(OTHER.to_owned(), ADDRESS.to_owned())]);
        let kept = heard.keys().collect::<Vec<_>>();
        assert_eq!(kept, [OTHER], "a TV that left is still remembered");
    }

    #[test]
    fn an_answer_that_isnt_json_is_never_remembered() {
        let mut heard = Heard::new();
        remember(&mut heard, MAC.into(), Ok("<html>".into()));
        assert!(heard.is_empty(), "a page is remembered as a TV");
    }

    #[test]
    fn one_tv_answering_keeps_the_source_answering() {
        let mut heard = Heard::new();
        remember(&mut heard, OTHER.into(), Ok(answer()));
        let (source, found) = read(
            vec![
                remember(&mut heard, OTHER.into(), silent()),
                remember(&mut heard, MAC.into(), Ok(answer())),
            ],
            NOW,
        );
        assert_eq!(found.len(), 2, "both TVs aren't said");
        assert_eq!(
            source.map(|source| (source.state, source.reason)),
            Some((SourceState::Answering, None)),
            "one silent TV makes the source unreachable"
        );
    }

    #[test]
    fn a_home_with_no_samsung_asks_nothing_and_says_no_source() {
        let mut lg = listing();
        lg["make"] = json!("LG");
        let asked = listed(&json!({"devices": [lg]}));
        assert!(asked.is_empty(), "another maker's box is asked: {asked:?}");
        assert_eq!(
            read(Vec::new(), NOW),
            (None, Vec::new()),
            "a source is said for nothing"
        );
    }
}
