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
//!
//! The uplink, who fills this machine's own upload (src/uplink), is one
//! record more, read every few seconds and only while it's watched, since
//! reading every socket's owner costs more than a page nobody looks at is
//! worth. `Served` holds the network's records and the uplink's together,
//! since daemon-serve is handed every record at once and drops any left
//! out.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use daemon_serve::asks::Domain;
use daemon_serve::serve::Shared;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::model::{Network, SourceState};

/// The daemon as its clients know it.
pub const DOMAIN: Domain = Domain {
    name: "net",
    kinds: &["device", "link", "source", "internet", "concern", "uplink"],
    sole: &["internet", "uplink"],
    named_only: &[],
};

/// What the daemon serves, held whole: the network's records from each
/// minute's round, and the uplink's from its own loop while it's watched.
pub struct Served {
    shared: Arc<Shared>,
    held: Mutex<Held>,
}

#[derive(Default)]
struct Held {
    network: Vec<Value>,
    sources: Value,
    uplink: Option<Value>,
}

impl Held {
    /// Every record there is now, the uplink's last.
    fn whole(&self) -> Vec<Value> {
        self.network.iter().chain(&self.uplink).cloned().collect()
    }
}

impl Served {
    pub fn new(shared: Arc<Shared>) -> Arc<Self> {
        let held = Held {
            sources: json!({}),
            ..Held::default()
        };
        Arc::new(Self {
            shared,
            held: Mutex::new(held),
        })
    }

    /// Serves this round's network, keeping the uplink as it stands.
    pub fn network(&self, network: &Network) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.network = records(network);
        held.sources = sources(network);
        self.shared.change(held.whole(), held.sources.clone());
    }

    /// Serves the uplink's newest record, or none once nothing watches it,
    /// keeping the network as it stands. Only Linux reads the uplink.
    #[cfg(target_os = "linux")]
    pub fn uplink(&self, record: Option<Value>) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.uplink = record;
        self.shared.change(held.whole(), held.sources.clone());
    }

    /// Whether a watch asks for the uplink, by its kind or its key.
    #[cfg(target_os = "linux")]
    pub fn uplink_watched(&self) -> bool {
        self.shared.watches("uplink", "") || self.shared.keys_watched().contains("uplink")
    }
}

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

/// The uplink's report as its one record.
#[cfg(target_os = "linux")]
pub fn uplink_record(report: &crate::uplink::Report) -> Value {
    record("uplink", "uplink", report)
}

/// How often the uplink is read while it's watched: `net uplink`'s own
/// window, long enough for a model request or two to land in it.
#[cfg(target_os = "linux")]
const UPLINK_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Reads the uplink every `UPLINK_EVERY` while a watch asks for it,
/// reporting each sample against the one before, and sleeps until a watch
/// begins or ends otherwise, letting the record go, so a later watch
/// never hears an old one. `woken` hears each watch begin or end, and the
/// loop ends with it. One `Owners` is kept throughout, so a socket's
/// owner is looked for once.
#[cfg(target_os = "linux")]
pub fn watch_uplink(served: &Served, woken: &std::sync::mpsc::Receiver<()>) {
    use std::sync::mpsc::RecvTimeoutError;

    use crate::uplink::{self, Owners, Sample};

    let mut owners = Owners::default();
    let mut earlier: Option<Sample> = None;
    loop {
        if !served.uplink_watched() {
            if earlier.take().is_some() {
                served.uplink(None);
            }
            if woken.recv().is_err() {
                return;
            }
            continue;
        }
        match uplink::sample() {
            Ok(later) => {
                if let Some(earlier) = &earlier {
                    let report = uplink::report(earlier, &later, &mut owners);
                    served.uplink(Some(uplink_record(&report)));
                }
                earlier = Some(later);
            }
            Err(error) => {
                eprintln!("net: the kernel's socket list can't be read: {error}");
                earlier = None;
                served.uplink(None);
            }
        }
        if let Err(RecvTimeoutError::Disconnected) = woken.recv_timeout(UPLINK_EVERY) {
            return;
        }
    }
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
    fn the_uplink_is_served_beside_the_network_and_leaves_alone() {
        let mut held = Held {
            network: records(&network()),
            ..Held::default()
        };
        let alone = held.whole().len();
        assert_eq!(
            alone,
            held.network.len(),
            "the network's records, with no uplink"
        );
        held.uplink = Some(json!({"key": "uplink", "kind": "uplink"}));
        let whole = held.whole();
        assert_eq!(
            whole.len(),
            alone + 1,
            "the uplink joins the network's records"
        );
        assert_eq!(whole[alone]["key"], "uplink", "the uplink's comes last");
        held.uplink = None;
        assert_eq!(
            held.whole().len(),
            alone,
            "unwatched, the uplink leaves alone"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_uplink_is_one_record_holding_its_report() {
        use crate::uplink::{self, Owners, Sample};
        let at = |at| Sample {
            at,
            sockets: Vec::new(),
            sent: BTreeMap::new(),
            uplink: Some("wlan0".to_owned()),
        };
        let report = uplink::report(&at(0.0), &at(5.0), &mut Owners::default());
        let said = uplink_record(&report);
        assert_eq!(said["key"], "uplink", "{said}");
        assert_eq!(DOMAIN.kind_of("uplink"), Some("uplink"), "a sole kind");
        assert_eq!(said["kind"], "uplink", "{said}");
        assert_eq!(
            said["uplink"], "wlan0",
            "the report's own fields stay: {said}"
        );
        assert!(said["joel_pct"].is_number(), "{said}");
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
