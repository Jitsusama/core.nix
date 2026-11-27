//! A Samsung TV as a source: what it says of itself at `:8001/api/v2/`,
//! asked of each Samsung box eero lists that could be one, and of each
//! box that has answered as one whatever eero calls it later
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
//! Only that page is read here, without pairing: keys are pressed by
//! the command (television/), and what it's playing, its volume and its
//! input are said only through Samsung's cloud.

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

/// What a box did when asked this minute. A TV asleep says nothing, so
/// only a reply that isn't a TV's page (an error, a redirect or more than
/// a TV says) counts as it answering as something else.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// It answered with a page, as its text.
    Page(Answer),
    /// It answered, but with no page: why.
    Refused(String),
    /// It didn't answer: why.
    Silent(String),
}

/// What a Samsung TV calls itself, which a phone or a soundbar answering
/// on the same port never does.
const A_TV: &str = "Samsung SmartTV";

/// The kinds eero gives a Samsung box that could be a TV: one it can't
/// place, as it does Joel's, and one that streams. A box eero places as
/// a phone, a tablet or a computer isn't asked; one it can't place is,
/// and says nothing unless it answers as a TV. A box that has answered
/// as a TV is asked whatever eero later calls it (`ask`).
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
    pub reply: Reply,
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

/// This minute's ask of each TV eero's record lists, through `read`,
/// which asks the box at an address, remembering each TV in `tvs`. Only
/// a record net read, listing devices, says which TVs have left, so a
/// minute net couldn't read eero's record (the daemon's `Err`: the file
/// is missing, can't be opened or doesn't parse) asks none and forgets
/// none, and a TV keeps its name. A record whose devices are missing,
/// null, empty or not a list was written before eero's device list was
/// first read, not by a home with nothing on it, so it is taken the same
/// way. A TV is forgotten only once eero knows it neither on nor off:
/// one eero lists as off keeps its name for the minute it's back. A TV
/// that has answered is asked at the address eero lists it on, whatever
/// maker or kind eero gives it then, since its answer, not eero's guess,
/// said it was a TV. One eero lists on without an address can't be asked,
/// so it is said silent, keeping its name and role for as long as eero
/// lists it so.
/// Through an outage of eero's cloud, its daemon keeps the last list it
/// read, so the TVs on it are still asked.
pub fn ask<E>(
    tvs: &mut Heard,
    eero: &Result<Value, E>,
    mut read: impl FnMut(&str) -> Reply,
) -> Vec<Asked> {
    let Ok(eero) = eero else {
        return Vec::new();
    };
    let lists_devices = eero
        .get("devices")
        .and_then(Value::as_array)
        .is_some_and(|devices| !devices.is_empty());
    if !lists_devices {
        return Vec::new();
    }
    forget_unknown(tvs, eero);
    let mut asking = listed(eero);
    let mut unaskable = Vec::new();
    for (mac, address) in on(eero) {
        let Some(earlier) = tvs.get(mac) else {
            continue;
        };
        if asking.iter().any(|(listed, _)| listed == mac) {
            continue;
        }
        match address {
            Some(address) => asking.push((mac.to_owned(), address.to_owned())),
            None => unaskable.push(Asked {
                earlier: Some(earlier.clone()),
                mac: mac.to_owned(),
                reply: Reply::Silent("eero lists it without an address to ask it at".to_owned()),
            }),
        }
    }
    asking
        .into_iter()
        .map(|(mac, address)| {
            let reply = read(&address);
            remember(tvs, mac, reply)
        })
        .chain(unaskable)
        .collect()
}

/// The hardware address of each device eero lists on, and the address
/// eero reaches it at when it gives one.
fn on(eero: &Value) -> impl Iterator<Item = (&str, Option<&str>)> {
    eero.get("devices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|device| {
            let mac = device.get("mac").and_then(Value::as_str)?;
            Some((mac, device.get("address").and_then(Value::as_str)))
        })
}

/// What `mac` answered this minute, beside its last answer as a TV; an
/// answer that is a TV's becomes the one remembered.
fn remember(heard: &mut Heard, mac: String, reply: Reply) -> Asked {
    if let Reply::Page(said) = &reply
        && television(String::new(), said).is_ok()
    {
        heard.insert(mac.clone(), said.clone());
    }
    Asked {
        earlier: heard.get(&mac).cloned(),
        mac,
        reply,
    }
}

/// Forgets each TV eero knows neither on nor off, so the memory holds
/// only TVs eero still knows, however many boxes have answered as one.
fn forget_unknown(heard: &mut Heard, eero: &Value) {
    let known: Vec<&str> = ["devices", "devices_off"]
        .into_iter()
        .filter_map(|list| eero.get(list).and_then(Value::as_array))
        .flatten()
        .filter_map(|device| device.get("mac").and_then(Value::as_str))
        .collect();
    heard.retain(|mac, _| known.contains(&mac.as_str()));
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
        let (heard, answered) = match one.reply {
            Reply::Page(answer) => (television(one.mac.clone(), &answer), true),
            Reply::Refused(why) => (Err(format!("its answer isn't a TV's: {why}")), true),
            Reply::Silent(why) => (Err(why), false),
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
        // Read this minute only when a TV answered, so a night of standby
        // isn't said as a reading just now.
        read_at: answering.then_some(now),
        since: failing.is_some().then_some(now),
        reason: failing,
    };
    (Some(source), found)
}

/// Whether `answer` is a TV's saying it's on, as one woken from standby
/// says once it's up.
pub fn on_now(answer: &str) -> bool {
    television(String::new(), answer)
        .is_ok_and(|found| found.television.power.as_deref() == Some("on"))
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
    // A word left blank is one the TV didn't say, so it reads as unsaid
    // rather than as a bare label.
    let said = |key: &str| {
        device
            .get(key)
            .and_then(Value::as_str)
            .filter(|said| !said.trim().is_empty())
            .map(str::to_owned)
    };
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

/// The tests, and the TV and its listing the collector's tests share.
#[cfg(test)]
#[path = "samsung_tests.rs"]
pub mod tests;
