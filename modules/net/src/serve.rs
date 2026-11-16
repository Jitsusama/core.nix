//! The home network's record as the daemon serves it on its socket, in the
//! shell's daemon interface (docs/daemons.md): split by kind, a record a
//! device, a link, a source and a concern, and the internet's one, so a
//! watch is told only what moved rather than the whole network each minute.
//!
//! Each record is the model's own fields with its key and kind, the facts
//! network.json holds, never words of the page's or the launcher's. Two
//! things change shape on the way:
//!
//! - A link carries `crossings`, how many devices' routes to the internet
//!   cross it, worked out here from their paths, so a page draws a link
//!   everything hangs on apart from one a single device uses without
//!   counting routes itself. It counts routes, not traffic.
//! - A concern's own kind, busy or weak, is `concern`, since `kind` is the
//!   record's.
//!
//! The record's `collected_at` isn't served: each record carries its own
//! time where it has one, a device's and a source's `read_at` and the
//! internet's `measured_at`.

use std::collections::BTreeMap;

use daemon_serve::asks::Domain;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::model::{Network, SourceState};

/// The daemon as its clients know it.
pub const DOMAIN: Domain = Domain {
    name: "net",
    kinds: &["device", "link", "source", "internet", "concern"],
    sole: &["internet"],
    named_only: &[],
};

/// Every record the network holds, by kind.
pub fn records(network: &Network) -> Vec<Value> {
    let mut records = Vec::new();
    for device in &network.devices {
        records.push(record("device", &format!("device:{}", device.id), device));
    }
    let crossed = crossings(network);
    let mut linked = BTreeMap::new();
    for link in &network.links {
        let ends = sorted(&link.a.device, &link.b.device);
        // Two links between the same two devices, as a second cable
        // would be, are told apart by how many came before.
        let seen: &mut u32 = linked.entry(ends.clone()).or_default();
        *seen += 1;
        let key = match *seen {
            1 => format!("link:{}|{}", ends.0, ends.1),
            nth => format!("link:{}|{}#{nth}", ends.0, ends.1),
        };
        let mut said = record("link", &key, link);
        said["crossings"] = json!(crossed.get(&ends).copied().unwrap_or(0));
        records.push(said);
    }
    for source in &network.sources {
        records.push(record("source", &format!("source:{}", source.name), source));
    }
    if let Some(internet) = &network.internet {
        records.push(record("internet", "internet", internet));
    }
    for concern in &network.concerns {
        let mut fields = serde_json::to_value(concern).unwrap_or_else(|_| json!({}));
        let own = fields["kind"].take();
        let mut key = format!("concern:{}:{}", own.as_str().unwrap_or(""), concern.device);
        for part in [&concern.on, &concern.other].into_iter().flatten() {
            key.push(':');
            key.push_str(part);
        }
        fields["concern"] = own;
        records.push(record("concern", &key, &fields));
    }
    records
}

/// How each source stands, by its name: answering, or its state with why.
pub fn sources(network: &Network) -> Value {
    let sources: Map<String, Value> = network
        .sources
        .iter()
        .map(|source| {
            let state = json!(source.state);
            let said = match (source.state, &source.reason) {
                (SourceState::Answering, _) => state,
                (_, Some(reason)) => json!({"state": state, "reason": reason}),
                (_, None) => json!({"state": state}),
            };
            (source.name.clone(), said)
        })
        .collect();
    Value::Object(sources)
}

/// A model's value as a record: its fields, with its key and kind.
fn record(kind: &str, key: &str, value: &impl Serialize) -> Value {
    let mut said = serde_json::to_value(value).unwrap_or_else(|_| json!({}));
    if let Value::Object(fields) = &mut said {
        fields.insert("key".to_owned(), json!(key));
        fields.insert("kind".to_owned(), json!(kind));
    }
    said
}

/// How many devices' routes to the internet cross each pair of devices,
/// the pair sorted, from each device's path: the device, then each hop it
/// goes through.
fn crossings(network: &Network) -> BTreeMap<(String, String), u32> {
    let mut crossed = BTreeMap::new();
    for device in &network.devices {
        let Some(path) = &device.path else { continue };
        let hops: Vec<&String> = std::iter::once(&device.id).chain(&path.through).collect();
        for pair in hops.windows(2) {
            *crossed.entry(sorted(pair[0], pair[1])).or_default() += 1;
        }
    }
    crossed
}

fn sorted(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network() -> Network {
        let text = include_str!("../tests/network.json");
        serde_json::from_str(text).expect("tests/network.json reads as a network")
    }

    fn keyed<'a>(records: &'a [Value], key: &str) -> &'a Value {
        records
            .iter()
            .find(|record| record["key"] == key)
            .unwrap_or_else(|| panic!("no record is keyed {key}"))
    }

    #[test]
    fn every_device_is_a_record_keyed_by_its_id() {
        let network = network();
        let records = records(&network);
        let devices: Vec<&Value> = records.iter().filter(|r| r["kind"] == "device").collect();
        assert_eq!(
            devices.len(),
            network.devices.len(),
            "a record for each device"
        );
        let modem = keyed(&records, "device:modem");
        assert_eq!(
            modem["id"], "modem",
            "the device's own fields stay: {modem}"
        );
        assert!(
            modem["roles"].is_array(),
            "a device keeps its roles: {modem}"
        );
    }

    #[test]
    fn a_link_is_keyed_by_its_ends_sorted_and_counts_the_routes_crossing_it() {
        let records = records(&network());
        // Every device's route to the internet leaves by the gateway's
        // link to the modem, so it's crossed by all the routed devices.
        let line = keyed(&records, "link:eero:2003|modem");
        assert_eq!(line["kind"], "link", "{line}");
        let routed = network()
            .devices
            .iter()
            .filter(|d| d.path.is_some())
            .count();
        assert_eq!(
            line["crossings"], routed,
            "every route crosses the gateway's link"
        );
        let one = keyed(&records, "link:eero:2001|mac:02:00:00:00:00:0a");
        assert_eq!(
            one["crossings"], 1,
            "a device's own link carries its route alone"
        );
    }

    #[test]
    fn a_concern_says_its_own_kind_as_concern() {
        let records = records(&network());
        let lost = keyed(&records, "concern:lost:modem:downstream channel 16");
        assert_eq!(lost["kind"], "concern", "{lost}");
        assert_eq!(lost["concern"], "lost", "{lost}");
        assert!(lost["says"].is_string(), "{lost}");
    }

    #[test]
    fn the_internet_is_one_record_and_each_source_another() {
        let network = network();
        let records = records(&network);
        let internet = keyed(&records, "internet");
        assert_eq!(internet["kind"], "internet", "{internet}");
        assert!(internet["up_mbps"].is_number(), "{internet}");
        let eero = keyed(&records, "source:eero");
        assert_eq!(eero["state"], "answering", "{eero}");
        assert!(
            records
                .iter()
                .all(|record| record.get("collected_at").is_none()),
            "no record carries the whole's time"
        );
    }

    #[test]
    fn every_key_is_one_the_domain_knows() {
        for record in records(&network()) {
            let key = record["key"].as_str().expect("a key is a string");
            assert_eq!(
                DOMAIN.kind_of(key),
                record["kind"].as_str(),
                "{key} names its kind"
            );
        }
    }

    #[test]
    fn a_source_not_answering_says_its_state_and_why() {
        let mut network = network();
        network.sources[1].state = SourceState::Unreachable;
        network.sources[1].reason = Some("it didn't answer".to_owned());
        let said = sources(&network);
        assert_eq!(said["eero"], "answering", "{said}");
        assert_eq!(
            said["modem"],
            json!({"state": "unreachable", "reason": "it didn't answer"}),
            "{said}"
        );
    }
}
