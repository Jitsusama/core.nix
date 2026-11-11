//! Each eero node drawn from its record in the network's answer: its
//! backhaul, its radios on each band, its ports and its light.

use super::{Answers, flag, id_of, list, number, signed, text, time, usage};
use crate::model::{
    Backhaul, BackhaulKind, Light, Node, NodeDevices, NodeHistory, Port, Radio, RadioHistory,
};
use serde_json::Value;

/// The nodes, the gateway first since everything hangs from it, then in
/// eero's order.
pub(super) fn nodes(answers: &Answers) -> Vec<Node> {
    let mut nodes: Vec<Node> = list(&answers.network, "/eeros/data")
        .filter_map(|record| node(record, answers))
        .collect();
    nodes.sort_by_key(|node| !node.gateway);
    nodes
}

pub(super) fn node(record: &Value, answers: &Answers) -> Option<Node> {
    let id = id_of(&text(record, "/url")?)?;
    let connections = answers.connections.get(&id).unwrap_or(&Value::Null);
    Some(Node {
        location: text(record, "/location"),
        model: text(record, "/model"),
        model_number: text(record, "/model_number"),
        serial: text(record, "/serial"),
        gateway: flag(record, "/gateway") == Some(true),
        state: text(record, "/state"),
        heartbeat_ok: flag(record, "/heartbeat_ok"),
        last_heartbeat: time(record, "/last_heartbeat"),
        backhaul: backhaul(record),
        address: text(record, "/ip_address"),
        ipv6: list(record, "/ipv6_addresses")
            .filter_map(|a| text(a, "/address"))
            .collect(),
        mac: text(record, "/mac_address"),
        firmware: text(record, "/os_version"),
        update_available: flag(record, "/update_available"),
        support_ends: time(record, "/update_status/support_expiration_date"),
        joined: time(record, "/joined"),
        last_reboot: time(record, "/last_reboot"),
        up_s: number(record, "/uptime/since_last_reboot_s"),
        cloud_s: number(record, "/uptime/since_cloud_connection_s"),
        devices: NodeDevices {
            all: number(record, "/connected_clients_count"),
            ethernet: number(record, "/connected_wired_clients_count"),
            wifi: number(record, "/connected_wireless_clients_count"),
        },
        radios: radios(record),
        ports: ports(record, connections),
        light: light(record),
        power: text(record, "/power_info/power_source"),
        usage: answers.node_usage.get(&id).and_then(usage),
        history: NodeHistory::default(),
        id,
    })
}

fn backhaul(record: &Value) -> Backhaul {
    let bars = number(record, "/mesh_quality_bars");
    if flag(record, "/wired") == Some(true) {
        return Backhaul {
            kind: BackhaulKind::Ethernet,
            upstream: None,
            radio: None,
            bars,
        };
    }
    Backhaul {
        kind: BackhaulKind::WiFi,
        upstream: text(record, "/wireless_upstream_node/name"),
        radio: text(record, "/wireless_upstream_node/primary_mesh_radio"),
        bars,
    }
}

/// eero's names for a node's radios, lowest frequency first, with the
/// model's name for each.
const BANDS: [(&str, &str); 5] = [
    ("band_2_4GHz", "2.4 GHz"),
    ("band_5GHz", "5 GHz"),
    ("band_5GHz_low", "5 GHz low"),
    ("band_5GHz_high", "5 GHz high"),
    ("band_6GHz", "6 GHz"),
];

/// Each radio, lowest frequency first; a band eero names that isn't known
/// here keeps eero's name and comes last.
fn radios(record: &Value) -> Vec<Radio> {
    let Some(Value::Object(stats)) = record.get("radio_channel_stats") else {
        return Vec::new();
    };
    let rank = |key: &str| {
        BANDS
            .iter()
            .position(|(k, _)| *k == key)
            .unwrap_or(BANDS.len())
    };
    let mut keys: Vec<&String> = stats.keys().collect();
    keys.sort_by_key(|key| rank(key));
    keys.into_iter()
        .map(|key| {
            let radio = &stats[key];
            Radio {
                band: BANDS
                    .iter()
                    .find(|(k, _)| k == key)
                    .map_or_else(|| key.clone(), |(_, name)| (*name).to_owned()),
                channel: number(radio, "/channel"),
                width_mhz: number(radio, "/channel_width"),
                power_dbm: signed(radio, "/tx_power"),
                busy_pct: number(radio, "/channel_utilization"),
                devices: number(radio, "/client_count"),
                bssid: list(record, "/bssids_with_bands")
                    .find(|b| text(b, "/band").as_deref() == Some(key))
                    .and_then(|b| text(b, "/ethernet_address")),
                history: RadioHistory::default(),
            }
        })
        .collect()
}

/// Each port, as the node's record says it now, with what only its
/// connections say: the fastest it can link at, and whether the backhaul
/// runs through it.
fn ports(record: &Value, connections: &Value) -> Vec<Port> {
    list(record, "/ethernet_status/statuses")
        .filter_map(|status| {
            let name = text(status, "/port_name")?;
            let interface = list(connections, "/ports/interfaces")
                .find(|i| text(i, "/name").as_deref() == Some(&name));
            let linked = flag(status, "/hasCarrier") == Some(true);
            let wan = flag(status, "/isWanPort") == Some(true);
            // An unlinked port still reports a speed, its slowest, which
            // isn't one it runs at.
            let mbps = linked.then(|| port_mbps(status.get("speed"))).flatten();
            let upstream = interface
                .and_then(|i| flag(i, "/is_upstream"))
                .unwrap_or_else(|| {
                    flag(status, "/isLeafWiredToUpstream") == Some(true) || (wan && linked)
                });
            Some(Port {
                linked,
                mbps,
                best_mbps: interface.and_then(|i| port_mbps(i.get("supported_speed"))),
                wan,
                upstream,
                rx_bytes: number(status, "/bandwidth_usage/rx_bytes"),
                tx_bytes: number(status, "/bandwidth_usage/tx_bytes"),
                name,
            })
        })
        .collect()
}

/// A port's speed, which eero writes as `P1000` for a gigabit.
pub(super) fn port_mbps(speed: Option<&Value>) -> Option<u32> {
    match speed? {
        Value::String(speed) => speed.trim_start_matches('P').parse().ok(),
        speed => speed.as_u64().and_then(|s| s.try_into().ok()),
    }
}

fn light(record: &Value) -> Option<Light> {
    let on = flag(record, "/led_on");
    let brightness = number(record, "/led_brightness");
    (on.is_some() || brightness.is_some()).then_some(Light { on, brightness })
}
