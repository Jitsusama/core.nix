//! The home network drawn from what eero's cloud answered: each read's
//! `data`, as contract/README.md says it, into the model's one record.
//!
//! A figure missing from an answer, or an answer not yet read, is `None`
//! in the model rather than a guess, so a field eero drops costs one figure.
//! The reads come in at different cadences, so each part of the record is
//! drawn from whichever answer is newest for it. What the history keeps is
//! left empty here, for history/ to draw.

mod devices;
mod nodes;
mod profiles;

use crate::model::{
    Addresses, BandSecurity, Cloud, Dns, Event, Firmware, Guest, HomeNetwork, Hour, Internet,
    InternetHistory, Lease, NodeCounts, Reservation, Series, SpeedTest, Subnet, Thread, Usage,
};
use crate::time;
use devices::{devices, devices_off};
use nodes::nodes;
use profiles::profiles;
use serde_json::Value;
use std::collections::BTreeMap;

/// The `data` of each of eero's answers the record is drawn from, `Null`
/// until it's been read.
#[derive(Debug, Clone, Default)]
pub struct Answers {
    /// `networks/<id>`, which embeds every node's record in `eeros.data`.
    pub network: Value,
    /// `networks/<id>/devices`.
    pub devices: Value,
    /// `eeros/<id>/connections`, by node id.
    pub connections: BTreeMap<String, Value>,
    /// `networks/<id>/speedtest`.
    pub speed_tests: Value,
    /// `networks/<id>/thread`.
    pub thread: Value,
    /// `networks/<id>/cellular_backup_events`.
    pub outages: Value,
    /// `networks/<id>/data_usage`, hourly over the last day.
    pub usage: Value,
    /// `networks/<id>/data_usage/eeros/<id>`, hourly over the last day, by
    /// node id.
    pub node_usage: BTreeMap<String, Value>,
    /// `networks/<id>/profiles`.
    pub profiles: Value,
    /// `networks/<id>/data_usage/profiles/<id>`, hourly over the last day,
    /// by profile id.
    pub profile_usage: BTreeMap<String, Value>,
    /// `networks/<id>/app_events`.
    pub events: Value,
    /// `networks/<id>/subnets_config`.
    pub subnets: Value,
    /// `networks/<id>/routing`, which embeds reservations, forwards and
    /// pinholes.
    pub routing: Value,
    /// `networks/<id>/wpa3_per_band`.
    pub security: Value,
    /// `networks/<id>/ac_compat`.
    pub ac_compatibility: Value,
    /// `networks/<id>/network_scan`.
    pub scan: Value,
}

/// The home network as eero answered, read at `read` while the cloud is
/// `cloud`; `None` until the network itself has been read, since every
/// other answer hangs from it.
pub fn home_network(answers: &Answers, read: &str, cloud: Cloud) -> Option<HomeNetwork> {
    let network = &answers.network;
    if network.is_null() {
        return None;
    }
    Some(HomeNetwork {
        read: read.to_owned(),
        cloud,
        name: text(network, "/name").unwrap_or_default(),
        internet: internet(answers),
        nodes: nodes(answers),
        devices: devices(&answers.devices),
        devices_off: devices_off(&answers.devices),
        profiles: profiles(&answers.profiles, &answers.profile_usage),
        events: events(&answers.events),
        joins_and_leaves: Vec::new(),
        told: Vec::new(),
    })
}

/// The last part of an eero url, which is the id of what it names.
pub fn id_of(url: &str) -> Option<String> {
    url.rsplit('/')
        .next()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// The ids of the nodes the network names, in eero's order.
pub fn node_ids(network: &Value) -> Vec<String> {
    list(network, "/eeros/data")
        .filter_map(|node| text(node, "/url"))
        .filter_map(|url| id_of(&url))
        .collect()
}

/// The id of each profile in `networks/<id>/profiles`, so its traffic can
/// be asked for.
pub fn profile_ids(profiles: &Value) -> Vec<String> {
    list(profiles, "")
        .filter_map(|profile| text(profile, "/url"))
        .filter_map(|url| id_of(&url))
        .collect()
}

fn internet(answers: &Answers) -> Internet {
    let n = &answers.network;
    Internet {
        state: text(n, "/health/internet/status").or_else(|| text(n, "/status")),
        isp_up: flag(n, "/health/internet/isp_up"),
        mesh: text(n, "/health/eero_network/status"),
        isp: text(n, "/geo_ip/isp"),
        asn: number(n, "/geo_ip/asn"),
        public_address: text(n, "/ip_settings/public_ip").or_else(|| text(n, "/wan_ip")),
        double_nat: flag(n, "/ip_settings/double_nat"),
        gateway_address: text(n, "/gateway_ip"),
        wan_type: text(n, "/wan_type"),
        lease: Lease {
            mode: text(n, "/lease/mode"),
            address: text(n, "/lease/dhcp/ip").or_else(|| text(n, "/lease/static/ip")),
            mask: text(n, "/lease/dhcp/mask").or_else(|| text(n, "/lease/static/mask")),
            router: text(n, "/lease/dhcp/router").or_else(|| text(n, "/lease/static/router")),
        },
        mode: text(n, "/connection/mode"),
        dhcp: text(n, "/dhcp/mode"),
        dns: Dns {
            mode: text(n, "/dns/mode"),
            servers: texts(n, "/dns/parent/ips"),
            caching: flag(n, "/dns/caching"),
            ipv6_mode: text(n, "/ipv6/name_servers/mode"),
        },
        ipv6: flag(n, "/ipv6_upstream"),
        upnp: flag(n, "/upnp"),
        nat_port_randomization: flag(n, "/nat_port_randomization"),
        sqm: flag(n, "/sqm"),
        band_steering: flag(n, "/band_steering"),
        wpa3: flag(n, "/wpa3"),
        wireless_mode: text(n, "/wireless_mode"),
        mlo_mode: text(n, "/mlo_mode"),
        speed_test: SpeedTest {
            at: time(n, "/speed/date"),
            down_mbps: mbps(n.pointer("/speed/down")),
            up_mbps: mbps(n.pointer("/speed/up")),
        },
        speed_tests: speed_tests(&answers.speed_tests),
        firmware: Firmware {
            running: text(n, "/updates/target_firmware"),
            next: text(n, "/updates/update_to_firmware"),
            has_update: flag(n, "/updates/has_update"),
            can_update_now: flag(n, "/updates/can_update_now"),
            update_hour: number(n, "/updates/preferred_update_hour"),
            last_started: time(n, "/updates/last_update_started"),
        },
        thread: Thread {
            // The network's own switch is read every time the network is,
            // so it's fresher than the Thread read's.
            on: flag(n, "/thread").or_else(|| flag(&answers.thread, "/enabled")),
            name: text(&answers.thread, "/name"),
            channel: number(&answers.thread, "/channel"),
        },
        guest: Guest {
            on: flag(n, "/guest_network/enabled"),
            name: text(n, "/guest_network/name"),
        },
        addresses: addresses(answers),
        security: security(&answers.security),
        ac_compatibility: flag(&answers.ac_compatibility, "/enabled"),
        name_clash: flag(&answers.scan, "/conflicting_ssid"),
        ddns: flag(n, "/ddns/enabled"),
        backup_internet: flag(n, "/backup_internet_enabled"),
        power_saving: flag(n, "/power_saving"),
        outages: list(&answers.outages, "/outages").cloned().collect(),
        nodes: NodeCounts {
            online: number(n, "/online_eeros_count"),
            offline: number(n, "/offline_eeros_count"),
            pending: number(n, "/pending_eeros_count"),
        },
        devices: number(n, "/clients/count"),
        usage: usage(&answers.usage),
        probe: None,
        history: InternetHistory::default(),
    }
}

fn addresses(answers: &Answers) -> Addresses {
    let routing = &answers.routing;
    Addresses {
        subnets: list(&answers.subnets, "")
            .map(|subnet| Subnet {
                kind: text(subnet, "/subnet_kind"),
                name: text(subnet, "/name"),
                on: flag(subnet, "/enabled"),
                address: text(subnet, "/dedicated_subnet/subnet_ip"),
                mask: text(subnet, "/dedicated_subnet/subnet_mask"),
                first: text(subnet, "/dedicated_subnet/start_ip"),
                last: text(subnet, "/dedicated_subnet/end_ip"),
                internet: flag(subnet, "/wan_access"),
                open: flag(subnet, "/open_network"),
            })
            .collect(),
        reservations: list(routing, "/reservations/data")
            .filter_map(|kept| {
                Some(Reservation {
                    mac: text(kept, "/mac")?.to_ascii_lowercase(),
                    description: text(kept, "/description"),
                    address: text(kept, "/ip"),
                })
            })
            .collect(),
        forwards: list(routing, "/forwards/data").cloned().collect(),
        pinholes: list(routing, "/pinholes/data").cloned().collect(),
    }
}

/// Each band's security, lowest band first, from eero's keys such as
/// `band_2_4_ghz`.
fn security(answer: &Value) -> Vec<BandSecurity> {
    let Some(bands) = answer.as_object() else {
        return Vec::new();
    };
    let mut each: Vec<(f64, BandSecurity)> = bands
        .iter()
        .filter_map(|(key, security)| {
            let ghz = key
                .strip_prefix("band_")?
                .strip_suffix("_ghz")?
                .replace('_', ".");
            Some((
                ghz.parse().ok()?,
                BandSecurity {
                    band: format!("{ghz} GHz"),
                    security: security.as_str()?.to_owned(),
                },
            ))
        })
        .collect();
    each.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    each.into_iter().map(|(_, band)| band).collect()
}

fn speed_tests(answer: &Value) -> Vec<SpeedTest> {
    list(answer, "")
        .map(|test| SpeedTest {
            at: time(test, "/date"),
            down_mbps: float(test, "/down_mbps"),
            up_mbps: float(test, "/up_mbps"),
        })
        .collect()
}

/// A speed eero gives as a value and its units, in Mbit/s.
fn mbps(reading: Option<&Value>) -> Option<f64> {
    let reading = reading?;
    let value = float(reading, "/value")?;
    Some(match text(reading, "/units").as_deref() {
        Some("Kbps") => value / 1000.0,
        Some("Gbps") => value * 1000.0,
        _ => value,
    })
}

/// Bytes by hour each way, from a `data_usage` answer's series.
fn usage(answer: &Value) -> Option<Usage> {
    let series = |kind: &str| {
        let found = list(answer, "/series").find(|s| text(s, "/type").as_deref() == Some(kind))?;
        Some(Series {
            total_bytes: number(found, "/sum")?,
            hours: list(found, "/values")
                .filter_map(|hour| {
                    Some(Hour {
                        at: time(hour, "/time")?,
                        bytes: number(hour, "/value")?,
                    })
                })
                .collect(),
        })
    };
    Some(Usage {
        download: series("download")?,
        upload: series("upload")?,
    })
}

/// The devices on now, by node, the strongest signal first.
/// Marks the device that's this machine, the one whose hardware address
/// is among `own`, this machine's, so the shell and the CLI can put it
/// first. It's matched by address rather than by the Wi-Fi link's BSSID,
/// so a machine on Ethernet is found too, and the address is the one the
/// interface has now, which is what eero sees even when it's private.
pub fn mark_this_machine(model: &mut HomeNetwork, own: &[String]) {
    for device in &mut model.devices {
        device.this_machine = own.iter().any(|mac| mac.eq_ignore_ascii_case(&device.mac));
    }
}

fn events(answer: &Value) -> Vec<Event> {
    list(answer, "/events")
        .filter_map(|event| {
            Some(Event {
                at: time::from_unix_ms(event.get("timestamp_ms")?.as_i64()?),
                kind: text(event, "/category")?,
                text: text(event, "/description")?,
            })
        })
        .collect()
}

/// The number a figure eero writes with its unit starts with: `-59 dBm`,
/// `48.0 MBit/s`.
fn leading_number<T: std::str::FromStr>(figure: &str) -> Option<T> {
    figure.split_whitespace().next()?.parse().ok()
}

/// The number inside one of eero's names for a setting: `WIDTH_80MHz`,
/// `GI_800NS`.
fn digits_in<T: std::str::FromStr>(name: &str) -> Option<T> {
    let digits: String = name
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn list<'a>(value: &'a Value, at: &str) -> impl Iterator<Item = &'a Value> {
    value
        .pointer(at)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn text(value: &Value, at: &str) -> Option<String> {
    value.pointer(at).and_then(Value::as_str).map(str::to_owned)
}

fn texts(value: &Value, at: &str) -> Vec<String> {
    list(value, at)
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn time(value: &Value, at: &str) -> Option<String> {
    value
        .pointer(at)
        .and_then(Value::as_str)
        .and_then(time::utc)
}

fn flag(value: &Value, at: &str) -> Option<bool> {
    value.pointer(at).and_then(Value::as_bool)
}

fn float(value: &Value, at: &str) -> Option<f64> {
    value.pointer(at).and_then(Value::as_f64)
}

fn number<T: TryFrom<u64>>(value: &Value, at: &str) -> Option<T> {
    value
        .pointer(at)
        .and_then(Value::as_u64)
        .and_then(|n| n.try_into().ok())
}

fn signed<T: TryFrom<i64>>(value: &Value, at: &str) -> Option<T> {
    value
        .pointer(at)
        .and_then(Value::as_i64)
        .and_then(|n| n.try_into().ok())
}

#[cfg(test)]
mod tests;
