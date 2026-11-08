//! Comparing a record with its golden answer, for the tests that hold the
//! model and the reader to `home-network.json`, and the contract's answers
//! and the history it's drawn from.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::history::History;
use crate::model::{Cloud, CloudState, HomeNetwork};
use crate::notices::{Kind, Notice, Urgency};
use crate::probe::Reading;
use crate::read::{self, Answers};
use crate::time;

/// The model's golden answer, drawn from the contract's files and the
/// history `history` keeps.
pub const MODEL: &str = include_str!("../home-network.json");

/// When the golden model was read, the contract's own time.
pub const READ: &str = "2026-11-07T22:54:43Z";

/// The `data` of a contract file.
pub fn contract(file: &str) -> Value {
    let path = format!("{}/contract/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let answer: Value = serde_json::from_str(&text).unwrap();
    answer["data"].clone()
}

fn by_node(file: impl Fn(&str) -> String) -> BTreeMap<String, Value> {
    ["2001", "2002", "2003"]
        .into_iter()
        .map(|id| (id.to_owned(), contract(&file(id))))
        .collect()
}

/// Every answer the contract holds, as the daemon keeps them.
pub fn contract_answers() -> Answers {
    Answers {
        network: contract("network.json"),
        devices: contract("net.devices.json"),
        connections: by_node(|id| format!("eero.{id}.connections.json")),
        speed_tests: contract("net.speedtest.json"),
        thread: contract("net.thread.json"),
        outages: contract("net.cellular_backup_events.json"),
        usage: contract("usage.json"),
        node_usage: by_node(|id| format!("usage.eero.{id}.json")),
        profiles: contract("net.profiles.json"),
        events: contract("net.app_events.json"),
    }
}

pub fn answering() -> Cloud {
    Cloud {
        state: CloudState::Answering,
        since: READ.into(),
        reason: None,
    }
}

/// The tablet that roams, the laptop that joins and the phone that leaves.
const ROAMER: &str = "02:00:00:00:00:01";
const JOINER: &str = "02:00:00:00:00:0e";
const LEAVER: &str = "02:00:00:00:00:19";

/// How each read before the last moves the figures: each device's signal
/// in dB, its receive rate and its speed as a share, each radio's busy
/// points. The last read is the contract's as it stands.
const SIGNAL_DB: [i16; 6] = [-3, -1, -4, -2, 1, 0];
const RX_SHARE: [f64; 6] = [1.0, 0.9, 0.75, 1.0, 1.0, 1.0];
const SPEED_SHARE: [f64; 6] = [0.6, 1.8, 0.3, 1.2, 0.9, 1.0];
const BUSY_PCT: [i16; 6] = [4, -3, 9, 2, -2, 0];

/// The times of each read's look at the internet from home: the gateway
/// node, the quicker public resolver, the gateway node resolving.
const GATEWAY_MS: [f64; 6] = [2.1, 1.8, 2.4, 1.9, 2.2, 2.0];
const INTERNET_MS: [f64; 6] = [11.8, 12.4, 13.1, 11.6, 12.0, 11.9];
const DNS_MS: [f64; 6] = [14.2, 13.7, 15.9, 13.9, 14.8, 14.1];

/// The outage the probe saw, a minute and a half before dawn four days
/// earlier: looks every 10 seconds, since eero couldn't be reached, the
/// gateway node answering throughout and nothing beyond it.
const OUTAGE_FROM: &str = "2026-11-03T03:12:30Z";
const OUTAGE_LOOKS: usize = 11;

/// What the daemon told over those days, oldest first, each at the read or
/// look that saw it, with the subjects it told and its words: the probe's
/// outage out and back, the laptop eero first saw two days before, and the
/// firmware every node runs, waiting and then updated to as the nodes
/// rebooted.
const TOLD: [(&str, Kind, &[&str], &str, &str); 5] = [
    (
        "2026-11-03T03:12:40Z",
        Kind::InternetOut,
        &["2026-11-03T03:12:40Z"],
        "The internet is out",
        "Nothing past the home network answers this machine.",
    ),
    (
        "2026-11-03T03:14:10Z",
        Kind::InternetBack,
        &["2026-11-03T03:12:40Z"],
        "The internet is back",
        "It was out for 1 min 30 s.",
    ),
    (
        "2026-11-05T02:08:13Z",
        Kind::Joined,
        &["02:00:00:00:00:0e 2026-11-05T02:07:45.318Z"],
        "laptop computer 1 joined the home network",
        "Never seen before, on Study over Wi-Fi at 5 GHz.",
    ),
    (
        "2026-11-06T16:21:33Z",
        Kind::UpdateWaiting,
        &["v7.17.4-5"],
        "A firmware update is waiting",
        "eero updates the nodes to v7.17.4-5 at 02:00.",
    ),
    (
        "2026-11-07T07:24:13Z",
        Kind::Updated,
        &["2001 v7.17.4-5", "2002 v7.17.4-5", "2003 v7.17.4-5"],
        "The nodes updated to v7.17.4-5",
        "From v7.17.3-9.",
    ),
];

/// The history behind the golden model: six reads five minutes apart, as
/// the daemon reads with nobody listening, the last at the model's read,
/// each the contract with its figures moved a little, and a look at the
/// internet with each. The tablet roams to the Den's 5 GHz from the
/// Study's 2.4 GHz, the laptop joins and a phone on the edge of the Den's
/// reach leaves, and the Study's backhaul drops a bar for a read. Before
/// them, the probe saw an outage, and the daemon told what changed.
pub fn history() -> History {
    let mut history = History::in_memory();
    for (at, kind, subjects, summary, body) in TOLD {
        // The history keeps no notice's urgency, nor what it was about.
        let notice = Notice {
            about: None,
            summary: summary.into(),
            body: body.into(),
            urgency: Urgency::Normal,
            tells: subjects.iter().map(|s| (kind, (*s).to_owned())).collect(),
        };
        history
            .record_notice(&notice, time::to_unix(at).unwrap())
            .unwrap();
    }
    let from = time::to_unix(OUTAGE_FROM).unwrap();
    for (n, at) in (0..OUTAGE_LOOKS).zip((from..).step_by(10)) {
        let out = n > 0 && n < OUTAGE_LOOKS - 1;
        let look = Reading {
            home: true,
            gateway_ms: Some(1.9),
            internet_ms: (!out).then_some(12.2),
            dns_ms: (!out).then_some(14.5),
            reachable: !out,
        };
        history.record_probe(&look, at).unwrap();
    }
    let last = time::to_unix(READ).unwrap();
    for step in 0..6 {
        let at = last - 300 * (5 - i64::try_from(step).unwrap());
        history.record(&moved(step, at), at, true).unwrap();
        let look = Reading {
            home: true,
            gateway_ms: Some(GATEWAY_MS[step]),
            internet_ms: Some(INTERNET_MS[step]),
            dns_ms: Some(DNS_MS[step]),
            reachable: true,
        };
        history.record_probe(&look, at).unwrap();
    }
    history
}

/// The contract as read at `at`, the `step`th of the history's reads.
fn moved(step: usize, at: i64) -> HomeNetwork {
    let mut answers = contract_answers();
    if step < 4 {
        let devices = answers.devices.as_array_mut().unwrap();
        let leaver = devices.iter_mut().find(|d| d["mac"] == LEAVER).unwrap();
        leaver["connected"] = Value::Bool(true);
    }
    let mut model = read::home_network(&answers, &time::from_unix(at), answering()).unwrap();
    if step < 2 {
        model.devices.retain(|d| d.mac != JOINER);
    }
    for device in &mut model.devices {
        if step < 3 && device.mac == ROAMER {
            device.node = Some("2002".into());
            device.band = Some("2.4 GHz".into());
        }
        device.signal_dbm = device.signal_dbm.map(|s| s + SIGNAL_DB[step]);
        if let Some(rx) = &mut device.rx {
            rx.mbps = rx
                .mbps
                .map(|mbps| (mbps * RX_SHARE[step] * 10.0).round() / 10.0);
        }
        let share = |mbps: f64| (mbps * SPEED_SHARE[step] * 1e7).round() / 1e7;
        device.speed.down_mbps = device.speed.down_mbps.map(share);
        device.speed.up_mbps = device.speed.up_mbps.map(share);
    }
    for node in &mut model.nodes {
        if step == 1 && node.id == "2002" {
            node.backhaul.bars = Some(4);
        }
        for radio in &mut node.radios {
            radio.busy_pct = radio
                .busy_pct
                .map(|busy| u8::try_from((i16::from(busy) + BUSY_PCT[step]).max(0)).unwrap());
        }
    }
    model
}

/// The paths where two values differ, so a failure names the figure rather
/// than printing the whole network. Numbers count as equal by value, since
/// `1` and `1.0` are the same figure.
pub fn differences(left: &Value, right: &Value) -> Vec<String> {
    let mut out = Vec::new();
    walk("", left, right, &mut out);
    out
}

fn walk(path: &str, left: &Value, right: &Value, out: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            for key in l.keys().chain(r.keys().filter(|k| !l.contains_key(*k))) {
                match (l.get(key), r.get(key)) {
                    (Some(a), Some(b)) => walk(&format!("{path}.{key}"), a, b, out),
                    _ => out.push(format!("{path}.{key}: only on one side")),
                }
            }
        }
        (Value::Array(l), Value::Array(r)) if l.len() == r.len() => {
            for (i, (a, b)) in l.iter().zip(r).enumerate() {
                walk(&format!("{path}[{i}]"), a, b, out);
            }
        }
        (Value::Array(l), Value::Array(r)) => {
            out.push(format!("{path}: {} items against {}", l.len(), r.len()));
        }
        (Value::Number(a), Value::Number(b)) if a.as_f64() == b.as_f64() => {}
        (a, b) if a != b => out.push(format!("{path}: {a} against {b}")),
        _ => {}
    }
}
