//! A Sony AV receiver as a source: what its `UPnP` description and its
//! read-only SOAP actions say, asked of each receiver eero lists, without
//! registering as its remote (contract/sony/ holds a real one's answers,
//! redacted). The receiver is the device eero already lists by its
//! hardware address, so it takes the receiver role there; its volume and
//! what it's playing are its receiver block, and every answer stays under
//! `detail.sony`, by the name of what was asked.
//!
//! Only reads are asked: the description, the volume and mute, what its
//! transport is doing, and Sony's system information. Registering as a
//! remote, sending text or a key, or setting anything waits on Joel's yes.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::model::{Receiver, Source, SourceState};
use crate::xml;

/// The source's name, and the key its answers sit under in `detail`.
pub const NAME: &str = "sony";

/// The port a Sony receiver serves its `UPnP` description and services on.
pub const PORT: u16 = 8080;

/// Each answer's text, by the name of what was asked: `description`, a
/// SOAP action's name, `ActionList` and `getSystemInformation`.
pub type Answers = BTreeMap<String, String>;

/// One read of a receiver: a plain GET, or a SOAP action posted to its
/// service, which only asks.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub name: String,
    pub url: String,
    pub soap: Option<Soap>,
}

/// A SOAP action: the header that names it, and the envelope to post.
#[derive(Debug, Clone, PartialEq)]
pub struct Soap {
    pub action: String,
    pub body: String,
}

/// What a receiver said, for the device eero lists by `mac`.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub mac: String,
    pub model: Option<String>,
    pub receiver: Receiver,
    pub detail: Value,
}

/// The receivers eero lists, each by its hardware address and the address
/// eero reaches it at: a device whose maker is Sony that eero takes for
/// audio, so a Sony television or console isn't asked.
pub fn listed(eero: &Value) -> Vec<(String, String)> {
    let Some(Value::Array(devices)) = eero.get("devices") else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|device| {
            let said = |key| device.get(key).and_then(Value::as_str);
            let sony = ["make", "manufacturer"]
                .into_iter()
                .any(|key| said(key).is_some_and(|maker| maker.starts_with("Sony")));
            sony && said("kind") == Some("audio")
        })
        .filter_map(|device| {
            let mac = device.get("mac")?.as_str()?;
            let address = device.get("address")?.as_str()?;
            Some((mac.to_owned(), address.to_owned()))
        })
        .collect()
}

/// The reads each service is asked, by the service's type: each only
/// asks, and the volume and mute are of the receiver's master channel.
const READS: [(&str, &str); 5] = [
    ("RenderingControl", "GetVolume"),
    ("RenderingControl", "GetMute"),
    ("AVTransport", "GetTransportInfo"),
    ("AVTransport", "GetMediaInfo"),
    ("AVTransport", "GetPositionInfo"),
];

/// What to ask a receiver once its description is read: each service's
/// reads, then its list of Sony's own actions. A service the description
/// doesn't offer isn't asked.
pub fn asks(description: &str, address: &str) -> Vec<Ask> {
    let Some((_, root)) = xml::read(description) else {
        return Vec::new();
    };
    let device = root.get("device");
    let services = match device.and_then(|device| xml::find(device, "serviceList")) {
        Some(list) => match list.get("service") {
            Some(Value::Array(services)) => services.iter().collect(),
            Some(service) => vec![service],
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    let mut asks: Vec<Ask> = READS
        .iter()
        .filter_map(|(service, action)| {
            let kind = format!("urn:schemas-upnp-org:service:{service}:1");
            let control = services
                .iter()
                .find(|said| said.get("serviceType").and_then(Value::as_str) == Some(&kind))?
                .get("controlURL")?
                .as_str()?;
            let channel = if *service == "RenderingControl" {
                "<Channel>Master</Channel>"
            } else {
                ""
            };
            Some(Ask {
                name: (*action).to_owned(),
                url: at(address, control),
                soap: Some(Soap {
                    action: format!("\"{kind}#{action}\""),
                    body: format!(
                        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
                         <s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" \
                         s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\
                         <s:Body><u:{action} xmlns:u=\"{kind}\">\
                         <InstanceID>0</InstanceID>{channel}</u:{action}></s:Body></s:Envelope>"
                    ),
                }),
            })
        })
        .collect();
    if let Some(list) = device
        .and_then(|device| xml::find(device, "X_CERS_ActionList_URL"))
        .and_then(Value::as_str)
    {
        asks.push(Ask {
            name: "ActionList".into(),
            url: list.to_owned(),
            soap: None,
        });
    }
    asks
}

/// A URL the description gives, which may be a path on the receiver.
fn at(address: &str, url: &str) -> String {
    if url.starts_with("http://") {
        url.to_owned()
    } else {
        format!("http://{address}:{PORT}{url}")
    }
}

/// Where a receiver's system information is asked, from its list of
/// Sony's own actions.
pub fn system(action_list: &str, _address: &str) -> Option<String> {
    let (_, list) = xml::read(action_list)?;
    let actions = match list.get("action")? {
        Value::Array(actions) => actions.iter().collect(),
        action => vec![action],
    };
    actions
        .into_iter()
        .find(|action| action.get("name").and_then(Value::as_str) == Some("getSystemInformation"))?
        .get("url")?
        .as_str()
        .map(str::to_owned)
}

/// Each receiver as it answered, and the source, or none when eero lists
/// no receiver. The source answers while any receiver does.
pub fn read(
    said: Vec<(String, Result<Answers, String>)>,
    now: u64,
) -> (Option<Source>, Vec<Found>) {
    if said.is_empty() {
        return (None, Vec::new());
    }
    let mut found = Vec::new();
    let mut reason = None;
    for (mac, answers) in said {
        match answers {
            Ok(answers) => found.push(heard(mac, &answers)),
            Err(why) => reason = reason.or(Some(why)),
        }
    }
    let answering = !found.is_empty();
    let source = Source {
        name: NAME.into(),
        what: "the Sony receivers eero lists, asked only to read".into(),
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

/// One receiver from its answers, each kept in its own words: the
/// description's fields as the receiver's own, and beside them each other
/// answer by what was asked, a SOAP reply as its response's fields.
fn heard(mac: String, answers: &Answers) -> Found {
    let mut detail = Map::new();
    for (name, text) in answers {
        let Some((_, value)) = xml::read(text) else {
            continue;
        };
        let kept = match name.as_str() {
            "description" => {
                if let Some(Value::Object(device)) = value.get("device") {
                    detail.extend(device.clone());
                }
                None
            }
            "ActionList" | "getSystemInformation" => Some(value),
            action => xml::find(&value, &format!("{action}Response")).cloned(),
        };
        if let Some(kept) = kept {
            detail.insert(name.clone(), kept);
        }
    }
    let said = |answer: &str, field: &str| {
        detail
            .get(answer)
            .and_then(|value| value.get(field))
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    };
    let model = detail
        .get("modelName")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let words = |text: &str| text.to_lowercase().replace('_', " ");
    let receiver = Receiver {
        volume: said("GetVolume", "CurrentVolume").and_then(|volume| volume.parse().ok()),
        loudest: None,
        muted: said("GetMute", "CurrentMute").map(|muted| muted != "0"),
        state: said("GetTransportInfo", "CurrentTransportState").map(doing),
        medium: said("GetMediaInfo", "PlayMedium").map(words),
        track: said("GetPositionInfo", "TrackURI")
            .or_else(|| said("GetMediaInfo", "CurrentURI"))
            .map(str::to_owned),
    };
    Found {
        mac,
        model,
        receiver,
        detail: Value::Object(detail),
    }
}

/// What a transport is doing, in plain words, from `UPnP`'s state.
fn doing(state: &str) -> String {
    state.to_lowercase().replace('_', " ")
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs;

    use serde_json::json;

    /// The contract's receiver: its hardware address and address, as the
    /// eero record the collector's tests add it to gives them.
    pub const MAC: &str = "02:00:00:00:00:60";
    pub const ADDRESS: &str = "10.0.0.60";

    const NOW: u64 = 1_794_702_033;

    /// What the contract's receiver answered, each by what was asked.
    pub fn answers() -> Answers {
        [
            "description",
            "GetVolume",
            "GetMute",
            "GetTransportInfo",
            "GetMediaInfo",
            "GetPositionInfo",
            "RenderingControl",
            "ActionList",
            "getSystemInformation",
        ]
        .into_iter()
        .map(|name| {
            let file = format!("{}/contract/sony/{name}.xml", env!("CARGO_MANIFEST_DIR"));
            let text = fs::read_to_string(&file).expect("the contract has each answer");
            (name.to_owned(), text)
        })
        .collect()
    }

    /// The receiver as eero lists it.
    pub fn listing() -> Value {
        json!({
            "mac": MAC, "name": "Stereo", "nickname": "Stereo",
            "hostname": "str-dn1050000060", "kind": "audio", "make": "Sony",
            "manufacturer": "Sony Corporation", "address": ADDRESS,
            "addresses": [ADDRESS], "node": "2003", "ethernet": false,
            "band": "2.4 GHz", "channel": 6,
        })
    }

    #[test]
    fn a_sony_eero_takes_for_audio_is_listed_and_no_other() {
        let eero = json!({"devices": [
            listing(),
            {"mac": "02:00:00:00:00:61", "make": "Sony", "kind": "tv", "address": "10.0.0.61"},
            {"mac": "02:00:00:00:00:62", "make": "HP", "kind": "audio", "address": "10.0.0.62"},
        ]});
        assert_eq!(
            listed(&eero),
            [(MAC.to_owned(), ADDRESS.to_owned())],
            "the wrong devices were taken for receivers"
        );
    }

    #[test]
    fn each_service_is_asked_only_what_reads() {
        let asks = asks(&answers()["description"], ADDRESS);
        let names: Vec<&str> = asks.iter().map(|ask| ask.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "GetVolume",
                "GetMute",
                "GetTransportInfo",
                "GetMediaInfo",
                "GetPositionInfo",
                "RenderingControl",
                "ActionList",
            ],
            "the reads asked changed"
        );
        let volume = &asks[0];
        assert_eq!(
            volume.url, "http://10.0.0.60:8080/RenderingControl/ctrl",
            "the volume isn't asked of its service"
        );
        let soap = volume.soap.as_ref().expect("the volume is a SOAP action");
        assert_eq!(
            soap.action, "\"urn:schemas-upnp-org:service:RenderingControl:1#GetVolume\"",
            "the action's header is wrong"
        );
        assert!(
            soap.body.contains("<Channel>Master</Channel>"),
            "the volume asked of no channel: {}",
            soap.body
        );
        assert_eq!(
            asks[5].url, "http://10.0.0.60:8080/RenderingControl/desc.xml",
            "the volume's scale isn't asked of its service's description"
        );
        assert!(asks[5].soap.is_none(), "a service's description was posted to");
        assert_eq!(
            asks[6].url, "http://10.0.0.60:50001/cers/ActionList.xml",
            "the action list isn't where the description says"
        );
        assert!(asks[6].soap.is_none(), "the action list was posted to");
    }

    #[test]
    fn a_url_naming_another_host_is_never_asked() {
        let description = answers()["description"]
            .replace(
                "<controlURL>/RenderingControl/ctrl</controlURL>",
                "<controlURL>http://10.9.9.9/RenderingControl/ctrl</controlURL>",
            )
            .replace(
                "<SCPDURL>/RenderingControl/desc.xml</SCPDURL>",
                "<SCPDURL>http://10.0.0.60@10.9.9.9/desc.xml</SCPDURL>",
            )
            .replace(
                "<controlURL>/AVTransport/ctrl</controlURL>",
                "<controlURL>http://10.0.0.60:52323/AVTransport/ctrl</controlURL>",
            )
            .replace(
                "http://10.0.0.60:50001/cers/ActionList.xml",
                "http://10.9.9.9:50001/cers/ActionList.xml",
            );
        let asks = asks(&description, ADDRESS);
        let names: Vec<&str> = asks.iter().map(|ask| ask.name.as_str()).collect();
        assert_eq!(
            names,
            ["GetTransportInfo", "GetMediaInfo", "GetPositionInfo"],
            "a URL on another host was asked, or one on the receiver wasn't"
        );
        assert_eq!(
            asks[0].url, "http://10.0.0.60:52323/AVTransport/ctrl",
            "another port on the receiver itself wasn't asked"
        );
    }

    #[test]
    fn the_system_information_is_where_the_action_list_says() {
        assert_eq!(
            system(&answers()["ActionList"], ADDRESS).as_deref(),
            Some("http://10.0.0.60:50001/cers/getSystemInformation"),
            "the system information wasn't found"
        );
    }

    #[test]
    fn a_system_information_on_another_host_is_never_asked() {
        let list = answers()["ActionList"].replace(
            "http://10.0.0.60:50001/cers/getSystemInformation",
            "http://10.9.9.9:50001/cers/getSystemInformation",
        );
        assert_eq!(
            system(&list, ADDRESS),
            None,
            "a URL on another host was asked"
        );
    }

    #[test]
    fn each_transport_state_reads_in_plain_words() {
        for (state, words) in [
            ("PLAYING", "playing"),
            ("PAUSED_PLAYBACK", "paused"),
            ("STOPPED", "stopped"),
            ("NO_MEDIA_PRESENT", "nothing to play"),
            ("TRANSITIONING", "starting"),
            ("X_DMR_BUFFERING", "x dmr buffering"),
        ] {
            assert_eq!(doing(state), words, "{state} reads wrong");
        }
    }

    #[test]
    fn a_receiver_says_its_model_volume_and_what_its_doing() {
        let (source, found) = read(vec![(MAC.into(), Ok(answers()))], NOW);
        let source = source.expect("a receiver makes a source");
        assert_eq!(source.state, SourceState::Answering, "it isn't answering");
        let [found] = found.as_slice() else {
            panic!("one receiver should be found: {found:?}");
        };
        assert_eq!(found.mac, MAC, "it's found for another device");
        assert_eq!(
            found.model.as_deref(),
            Some("STR-DN1050"),
            "its model wasn't read"
        );
        assert_eq!(
            found.receiver,
            Receiver {
                volume: Some(27),
                loudest: Some(100),
                muted: Some(false),
                state: Some("nothing to play".into()),
                medium: None,
                track: None,
            },
            "its receiver block is wrong"
        );
        assert_eq!(
            found.detail["friendlyName"],
            json!("STR-DN1050 000060"),
            "its description wasn't kept in its own words"
        );
        assert_eq!(
            found.detail["GetVolume"]["CurrentVolume"],
            json!("27"),
            "its volume's answer wasn't kept"
        );
        assert_eq!(
            found.detail["getSystemInformation"]["generation"],
            json!("2013"),
            "its system information wasn't kept"
        );
        assert_eq!(
            found.detail["GetTransportInfo"]["CurrentTransportState"],
            json!("NO_MEDIA_PRESENT"),
            "its transport's own state wasn't kept"
        );
    }

    #[test]
    fn a_receiver_that_doesnt_answer_is_said_and_nothing_made_up() {
        let (source, found) = read(vec![(MAC.into(), Err("timed out".into()))], NOW);
        let source = source.expect("a listed receiver makes a source");
        assert_eq!(
            source.state,
            SourceState::Unreachable,
            "it read as answering"
        );
        assert_eq!(
            source.reason.as_deref(),
            Some("timed out"),
            "the reason was lost"
        );
        assert!(found.is_empty(), "a receiver was made up: {found:?}");
    }

    #[test]
    fn no_receiver_listed_is_no_source() {
        let (source, found) = read(Vec::new(), NOW);
        assert!(source.is_none(), "a source with nothing to read was said");
        assert!(found.is_empty(), "a receiver was made up");
    }
}
