//! eero as a source: the record the eero daemon keeps of the home network
//! (modules/eero/home-network.json), read from its file so eero's cloud
//! keeps one reader. Each node is one device, a router and an access point
//! when it's the gateway and an access point otherwise; each device on
//! them is a client; each backhaul and each client's join is a link. The
//! whole of what eero says of each stays under `detail.eero`.

use serde_json::{Map, Value};

use crate::model::{
    Device, End, Internet, Link, LinkState, Medium, Port, PortKind, Radio, Role, Source,
    SourceState,
};
use crate::time;

/// The source's name, as devices, links and `sources` name it.
pub const NAME: &str = "eero";

/// What the record says of the home network: its nodes and the devices on
/// them, and the links between them, including the gateway's cable to the
/// modem when the record says it has one.
pub struct Read {
    pub source: Source,
    pub devices: Vec<Device>,
    pub links: Vec<Link>,
    pub internet: Option<Internet>,
}

/// The record as the source says it, or why it isn't there.
pub fn read(record: Result<Value, String>, now: u64) -> Read {
    let record = match record {
        Ok(record) => record,
        Err(why) => {
            return Read {
                source: Source {
                    name: NAME.into(),
                    what: WHAT.into(),
                    state: SourceState::Unreachable,
                    read_at: None,
                    since: Some(now),
                    reason: Some(why),
                },
                devices: Vec::new(),
                links: Vec::new(),
                internet: None,
            };
        }
    };
    let read_at = record
        .get("read")
        .and_then(Value::as_str)
        .and_then(time::parse);
    let nodes = list(&record, "nodes");
    let mut devices: Vec<Device> = nodes
        .iter()
        .map(|node| node_device(node, read_at))
        .collect();
    let mut links: Vec<Link> = nodes
        .iter()
        .filter_map(|node| backhaul(node, &nodes))
        .collect();
    links.extend(nodes.iter().filter_map(|node| wan(node)));
    for client in list(&record, "devices") {
        devices.push(client_device(client, read_at));
        links.extend(join(client, &nodes));
    }
    Read {
        source: source(&record, read_at),
        devices,
        links,
        internet: internet(&record),
    }
}

/// The internet, from the record's block of that name: the provider eero
/// names and its last speed test, which eero runs from the gateway, so it
/// measures the internet rather than any Wi-Fi.
fn internet(record: &Value) -> Option<Internet> {
    let block = record.get("internet")?;
    let test = block.get("speed_test");
    let rate = |key| test.and_then(|test| test.get(key)).and_then(Value::as_f64);
    let internet = Internet {
        provider: text(block, "isp").map(Into::into),
        down_mbps: rate("down_mbps"),
        up_mbps: rate("up_mbps"),
        measured_at: test.and_then(|test| text(test, "at")).and_then(time::parse),
        source: NAME.into(),
    };
    (internet.provider.is_some() || internet.down_mbps.is_some() || internet.up_mbps.is_some())
        .then_some(internet)
}

/// What the source reads, in words.
const WHAT: &str = "eero's cloud, through the eero daemon's record";

/// Whether eero's cloud is answering the daemon, as the record says.
fn source(record: &Value, read_at: Option<u64>) -> Source {
    let cloud = record.get("cloud");
    let said = |key| {
        cloud
            .and_then(|cloud| cloud.get(key))
            .and_then(Value::as_str)
    };
    let state = match said("state") {
        Some("answering") => SourceState::Answering,
        Some("signed-out") => SourceState::SignedOut,
        _ => SourceState::Unreachable,
    };
    let since = (state != SourceState::Answering)
        .then(|| said("since").and_then(time::parse))
        .flatten();
    let reason = (state != SourceState::Answering).then(|| {
        said("reason").map_or_else(
            || format!("eero's cloud is {}", said("state").unwrap_or("unknown")),
            str::to_owned,
        )
    });
    Source {
        name: NAME.into(),
        what: WHAT.into(),
        state,
        read_at,
        since,
        reason,
    }
}

/// A node's id as a device's.
fn node_id(node: &Value) -> Option<String> {
    text(node, "id").map(|id| format!("eero:{id}"))
}

fn node_device(node: &Value, read_at: Option<u64>) -> Device {
    let gateway = node.get("gateway").and_then(Value::as_bool) == Some(true);
    Device {
        id: node_id(node).unwrap_or_else(|| "eero:unknown".into()),
        name: text(node, "location").unwrap_or("eero").into(),
        roles: if gateway {
            vec![Role::Router, Role::AccessPoint]
        } else {
            vec![Role::AccessPoint]
        },
        source: NAME.into(),
        read_at,
        vendor: Some("eero".into()),
        model: text(node, "model").map(Into::into),
        hardware: text(node, "model_number").map(Into::into),
        firmware: text(node, "firmware").map(Into::into),
        serial: text(node, "serial").map(Into::into),
        macs: text(node, "mac").map(Into::into).into_iter().collect(),
        addresses: text(node, "address").map(Into::into).into_iter().collect(),
        up_s: node.get("up_s").and_then(Value::as_u64),
        ports: list(node, "ports").into_iter().map(port).collect(),
        radios: list(node, "radios").into_iter().map(radio).collect(),
        modem: None,
        path: None,
        detail: detail(node),
    }
}

fn port(port: &Value) -> Port {
    let linked = port.get("linked").and_then(Value::as_bool);
    Port {
        name: text(port, "name").unwrap_or("?").into(),
        kind: PortKind::Ethernet,
        link: linked.map(|up| if up { LinkState::Up } else { LinkState::Down }),
        speed_mbps: number(port, "mbps"),
        duplex: None,
    }
}

fn radio(radio: &Value) -> Radio {
    Radio {
        name: text(radio, "band").unwrap_or("?").into(),
        band: text(radio, "band").map(Into::into),
        channel: number(radio, "channel"),
        width_mhz: number(radio, "width_mhz"),
        busy_pct: number(radio, "busy_pct"),
        power_dbm: radio
            .get("power_dbm")
            .and_then(Value::as_i64)
            .and_then(|power| i32::try_from(power).ok()),
    }
}

/// How a node reaches the one before it: a cable from its upstream port,
/// or its radio on the band eero names.
fn backhaul(node: &Value, nodes: &[&Value]) -> Option<Link> {
    let backhaul = node.get("backhaul")?;
    let upstream = text(backhaul, "upstream")?;
    let before = nodes
        .iter()
        .find(|other| text(other, "location") == Some(upstream))?;
    let (a, b) = (node_id(node)?, node_id(before)?);
    if text(backhaul, "kind") == Some("ethernet") {
        let port = list(node, "ports")
            .into_iter()
            .find(|port| port.get("upstream").and_then(Value::as_bool) == Some(true));
        return Some(Link {
            a: End {
                device: a,
                port: port.and_then(|port| text(port, "name")).map(Into::into),
                radio: None,
            },
            b: End {
                device: b,
                port: None,
                radio: None,
            },
            medium: Medium::Ethernet,
            speed_mbps: port.and_then(|port| number(port, "mbps")),
            channel: None,
            signal_dbm: None,
            source: NAME.into(),
        });
    }
    let band = text(backhaul, "radio");
    let ours = band.and_then(|band| on_band(node, band));
    let theirs = band.and_then(|band| on_band(before, band));
    Some(Link {
        a: End {
            device: a,
            port: None,
            radio: ours.and_then(|radio| text(radio, "band")).map(Into::into),
        },
        b: End {
            device: b,
            port: None,
            radio: theirs.and_then(|radio| text(radio, "band")).map(Into::into),
        },
        medium: Medium::Wifi,
        speed_mbps: None,
        channel: ours.and_then(|radio| number(radio, "channel")),
        signal_dbm: None,
        source: NAME.into(),
    })
}

/// The node's radio on the band a backhaul names, as `5.8GHz`: eero's
/// radios are named `2.4 GHz`, `5 GHz low` and `5 GHz high`.
fn on_band<'a>(node: &'a Value, band: &str) -> Option<&'a Value> {
    let named = match band {
        "2.4GHz" => "2.4 GHz",
        "5.2GHz" => "5 GHz low",
        "5.8GHz" => "5 GHz high",
        _ => return None,
    };
    list(node, "radios")
        .into_iter()
        .find(|radio| text(radio, "band") == Some(named))
}

/// The gateway's cable out to the modem, from its WAN port.
fn wan(node: &Value) -> Option<Link> {
    let port = list(node, "ports").into_iter().find(|port| {
        port.get("wan").and_then(Value::as_bool) == Some(true)
            && port.get("linked").and_then(Value::as_bool) == Some(true)
    })?;
    Some(Link {
        a: End {
            device: crate::sources::modem::ID.into(),
            port: Some("ethernet".into()),
            radio: None,
        },
        b: End {
            device: node_id(node)?,
            port: text(port, "name").map(Into::into),
            radio: None,
        },
        medium: Medium::Ethernet,
        speed_mbps: number(port, "mbps"),
        channel: None,
        signal_dbm: None,
        source: NAME.into(),
    })
}

fn client_device(client: &Value, read_at: Option<u64>) -> Device {
    let mac = text(client, "mac").unwrap_or("?");
    let name = ["nickname", "name", "hostname"]
        .into_iter()
        .find_map(|key| text(client, key))
        .unwrap_or(mac);
    let wifi = client.get("ethernet").and_then(Value::as_bool) != Some(true);
    Device {
        id: format!("mac:{mac}"),
        name: name.into(),
        roles: vec![Role::Client],
        source: NAME.into(),
        read_at,
        vendor: ["manufacturer", "make"]
            .into_iter()
            .find_map(|key| text(client, key))
            .map(Into::into),
        model: text(client, "model").map(Into::into),
        hardware: None,
        firmware: None,
        serial: None,
        macs: vec![mac.into()],
        addresses: client_addresses(client),
        up_s: None,
        ports: Vec::new(),
        radios: if wifi {
            vec![Radio {
                name: client_radio(client),
                band: text(client, "band").map(Into::into),
                channel: number(client, "channel"),
                width_mhz: client.get("rx").and_then(|rx| number(rx, "width_mhz")),
                busy_pct: None,
                power_dbm: None,
            }]
        } else {
            Vec::new()
        },
        modem: None,
        path: None,
        detail: detail(client),
    }
}

/// A client's addresses, the one eero reaches it by first, since eero's
/// list can lead with another of its links, such as a virtual machine's
/// bridge, which nothing else on the home network reaches.
fn client_addresses(client: &Value) -> Vec<String> {
    let mut addresses: Vec<String> = text(client, "address")
        .map(Into::into)
        .into_iter()
        .collect();
    for address in list(client, "addresses")
        .into_iter()
        .filter_map(Value::as_str)
    {
        if !addresses.iter().any(|known| known == address) {
            addresses.push(address.into());
        }
    }
    addresses
}

/// A client's radio, named by the band it joins on, as eero names it
/// (`5 GHz`), so a link reads band to band.
fn client_radio(client: &Value) -> String {
    text(client, "band").unwrap_or("Wi-Fi").into()
}

/// A client's way onto its node: its radio to the node's radio on the
/// same channel, or a cable.
fn join(client: &Value, nodes: &[&Value]) -> Option<Link> {
    let mac = text(client, "mac")?;
    let node = nodes
        .iter()
        .find(|node| text(node, "id") == text(client, "node"))?;
    let speed = |way: &str| {
        client
            .get(way)
            .and_then(|way| way.get("mbps"))
            .and_then(Value::as_f64)
            .and_then(whole)
    };
    if client.get("ethernet").and_then(Value::as_bool) == Some(true) {
        return Some(Link {
            a: End {
                device: format!("mac:{mac}"),
                port: None,
                radio: None,
            },
            b: End {
                device: node_id(node)?,
                port: text(client, "port").map(Into::into),
                radio: None,
            },
            medium: Medium::Ethernet,
            speed_mbps: None,
            channel: None,
            signal_dbm: None,
            source: NAME.into(),
        });
    }
    let channel = number(client, "channel");
    let radio = list(node, "radios")
        .into_iter()
        .find(|radio| channel.is_some() && number(radio, "channel") == channel);
    Some(Link {
        a: End {
            device: format!("mac:{mac}"),
            port: None,
            radio: Some(client_radio(client)),
        },
        b: End {
            device: node_id(node)?,
            port: None,
            radio: radio.and_then(|radio| text(radio, "band")).map(Into::into),
        },
        medium: Medium::Wifi,
        speed_mbps: speed("rx"),
        channel,
        signal_dbm: client
            .get("signal_dbm")
            .and_then(Value::as_i64)
            .and_then(|dbm| i32::try_from(dbm).ok()),
        source: NAME.into(),
    })
}

/// Everything eero says of a node or a device, under the source's name.
fn detail(said: &Value) -> Map<String, Value> {
    let mut detail = Map::new();
    detail.insert(NAME.into(), said.clone());
    detail
}

fn list<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|all| all.iter().collect())
        .unwrap_or_default()
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number<T: TryFrom<u64>>(value: &Value, key: &str) -> Option<T> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| T::try_from(n).ok())
}

/// A rate in whole megabits, as links keep it.
fn whole(mbps: f64) -> Option<u32> {
    (mbps.is_finite() && mbps >= 0.0 && mbps < f64::from(u32::MAX))
        .then(|| format!("{mbps:.0}").parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    /// One node and one laptop on its 5 GHz low radio, as the eero
    /// daemon writes them; the laptop also runs a virtual machine bridge,
    /// whose address eero happens to list first.
    fn laptop_at_home() -> Read {
        read(
            Ok(json!({
                "cloud": {"state": "answering"},
                "read": "2026-11-07T22:54:43Z",
                "nodes": [{
                    "id": "2003", "location": "Hall", "gateway": true,
                    "radios": [{"band": "5 GHz low", "channel": 52}]
                }],
                "devices": [{
                    "mac": "02:00:00:00:00:01", "name": "laptop", "node": "2003",
                    "address": "10.0.0.20",
                    "addresses": ["10.0.122.1", "10.0.0.20", "fe80::1"],
                    "band": "5 GHz", "channel": 52, "signal_dbm": -55,
                    "rx": {"mbps": 1080.6}
                }]
            })),
            1_794_092_100,
        )
    }

    #[test]
    fn a_clients_own_address_comes_first() {
        let laptop = &laptop_at_home().devices[1];
        assert_eq!(
            laptop.addresses,
            ["10.0.0.20", "10.0.122.1", "fe80::1"],
            "eero's own address for it leads, each once"
        );
    }

    #[test]
    fn a_clients_radio_is_named_by_its_band() {
        let read = laptop_at_home();
        assert_eq!(read.devices[1].radios[0].name, "5 GHz");
        assert_eq!(read.links[0].a.radio.as_deref(), Some("5 GHz"));
        assert_eq!(read.links[0].b.radio.as_deref(), Some("5 GHz low"));
    }

    #[test]
    fn a_clients_link_carries_its_signal() {
        assert_eq!(laptop_at_home().links[0].signal_dbm, Some(-55));
    }

    #[test]
    fn the_internet_is_eeros_last_speed_test_and_the_provider_it_names() {
        let read = read(
            Ok(json!({
                "cloud": {"state": "answering"},
                "internet": {
                    "isp": "Example ISP",
                    "speed_test": {
                        "down_mbps": 369.86, "up_mbps": 11.93, "at": "2026-11-05T10:52:15Z"
                    }
                }
            })),
            1_794_092_100,
        );
        assert_eq!(
            read.internet,
            Some(Internet {
                provider: Some("Example ISP".into()),
                down_mbps: Some(369.86),
                up_mbps: Some(11.93),
                measured_at: Some(1_793_875_935),
                source: NAME.into(),
            })
        );
    }

    #[test]
    fn a_record_with_no_internet_block_says_no_internet() {
        assert_eq!(laptop_at_home().internet, None);
    }
}
