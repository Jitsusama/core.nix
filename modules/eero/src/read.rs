//! The home network drawn from what eero's cloud answered: each read's
//! `data`, as contract/README.md says it, into the model's one record.
//!
//! A figure missing from an answer, or an answer not yet read, is `None`
//! in the model rather than a guess, so a field eero drops costs one figure.
//! The reads come in at different cadences, so each part of the record is
//! drawn from whichever answer is newest for it. What the history keeps is
//! left empty here, for history.rs to draw.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde_json::Value;

use crate::model::{
    Backhaul, BackhaulKind, Cloud, Device, DeviceHistory, DevicePort, Dns, Event, Filters,
    Firmware, Guest, HomeNetwork, Hour, Internet, InternetHistory, Lease, Light, Member, Node,
    NodeCounts, NodeDevices, NodeHistory, Packets, Port, Profile, Radio, RadioHistory, Rate, Series, Speed,
    SpeedTest, Thread, Usage,
};
use crate::time;

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

/// The nodes, the gateway first since everything hangs from it, then in
/// eero's order.
fn nodes(answers: &Answers) -> Vec<Node> {
    let mut nodes: Vec<Node> = list(&answers.network, "/eeros/data")
        .filter_map(|record| node(record, answers))
        .collect();
    nodes.sort_by_key(|node| !node.gateway);
    nodes
}

fn node(record: &Value, answers: &Answers) -> Option<Node> {
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
fn port_mbps(speed: Option<&Value>) -> Option<u32> {
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

fn devices(answer: &Value) -> Vec<Device> {
    let mut devices: Vec<Device> = list(answer, "")
        .filter(|record| flag(record, "/connected") == Some(true))
        .filter_map(device)
        .collect();
    devices.sort_by(|a, b| {
        let node = |d: &Device| (d.node.is_none(), d.node.clone());
        let signal = |d: &Device| Reverse(d.signal_dbm.unwrap_or(i16::MIN));
        (node(a), signal(a), &a.mac).cmp(&(node(b), signal(b), &b.mac))
    });
    devices
}

fn device(record: &Value) -> Option<Device> {
    let link = record.get("connectivity").unwrap_or(&Value::Null);
    let frequency_mhz = number(link, "/frequency");
    Some(Device {
        mac: text(record, "/mac")?,
        this_machine: false,
        name: text(record, "/display_name"),
        nickname: text(record, "/nickname"),
        hostname: text(record, "/hostname"),
        kind: text(record, "/device_type"),
        make: text(record, "/inferred_make"),
        model: text(record, "/inferred_model"),
        manufacturer: text(record, "/manufacturer"),
        private: flag(record, "/is_private"),
        address: text(record, "/ip"),
        addresses: texts(record, "/ips"),
        first_seen: time(record, "/first_seen"),
        first_active: time(record, "/first_active"),
        last_active: time(record, "/last_active"),
        node: text(record, "/source/url").and_then(|url| id_of(&url)),
        ethernet: flag(record, "/wireless").map_or_else(
            || text(record, "/connection_type").as_deref() == Some("wired"),
            |wifi| !wifi,
        ),
        ssid: text(record, "/ssid"),
        auth: text(record, "/auth"),
        subnet: text(record, "/subnet_kind"),
        guest: flag(record, "/is_guest"),
        band: frequency_mhz
            .map(band)
            .or_else(|| interface_band(record))
            .map(str::to_owned),
        channel: number(record, "/channel"),
        frequency_mhz,
        bssid: text(link, "/bssid"),
        signal_dbm: text(link, "/signal").and_then(|s| leading_number(&s)),
        // eero says 0 for a device whose radio reports no SNR, as those
        // that report their rate alone do; a link at 0 dB wouldn't hold.
        snr_db: number(link, "/snr").filter(|snr| *snr > 0),
        score: float(link, "/score"),
        bars: number(link, "/score_bars"),
        rx: rate(
            link.get("rx_rate_info"),
            text(link, "/rx_bitrate").and_then(|r| leading_number(&r)),
        ),
        tx: rate(link.get("tx_rate_info"), None),
        retry_pct: float(link, "/tx_retry_pct"),
        idle_ms: number(link, "/inactive_time_ms"),
        packets: packets(link.get("packet_stats")),
        port: link
            .get("ethernet_status")
            .filter(|s| !s.is_null())
            .map(|s| DevicePort {
                name: text(s, "/port_name"),
                mbps: port_mbps(s.get("speed")),
            }),
        speed: Speed {
            down_mbps: float(record, "/usage/down_mbps"),
            up_mbps: float(record, "/usage/up_mbps"),
        },
        profile: profile_id(record.get("profile")),
        paused: flag(record, "/paused"),
        blocked: flag(record, "/blacklisted"),
        history: DeviceHistory::default(),
        roams: Vec::new(),
    })
}

/// The band a frequency in MHz is in.
fn band(mhz: u32) -> &'static str {
    match mhz {
        ..3000 => "2.4 GHz",
        3000..5925 => "5 GHz",
        _ => "6 GHz",
    }
}

/// The band from the coarser `interface` eero gives when it gives no
/// frequency: `{"frequency": "5", "frequency_unit": "GHz"}`.
fn interface_band(record: &Value) -> Option<&'static str> {
    if text(record, "/interface/frequency_unit").as_deref() != Some("GHz") {
        return None;
    }
    match text(record, "/interface/frequency")?.as_str() {
        "2.4" => Some("2.4 GHz"),
        "5" => Some("5 GHz"),
        "6" => Some("6 GHz"),
        _ => None,
    }
}

/// A link rate and how it's made, or the bare rate where that's all eero
/// has, as an older device's report gives only `rx_bitrate`.
fn rate(info: Option<&Value>, bare_mbps: Option<f64>) -> Option<Rate> {
    let info = info.unwrap_or(&Value::Null);
    let rate = Rate {
        mbps: float(info, "/rate_bps").map(|bps| bps / 1e6).or(bare_mbps),
        mcs: number(info, "/mcs"),
        streams: number(info, "/nss"),
        width_mhz: text(info, "/channel_width").and_then(|w| digits_in(&w)),
        guard_ns: text(info, "/guard_interval").and_then(|g| digits_in(&g)),
        generation: text(info, "/phy_type").and_then(|phy| generation(&phy)),
    };
    let known = rate.mbps.is_some()
        || rate.mcs.is_some()
        || rate.streams.is_some()
        || rate.width_mhz.is_some()
        || rate.guard_ns.is_some()
        || rate.generation.is_some();
    known.then_some(rate)
}

/// The Wi-Fi generation of an 802.11 PHY: HT is Wi-Fi 4, VHT 5, HE 6, EHT 7.
fn generation(phy: &str) -> Option<String> {
    let number = match phy {
        "HT" => 4,
        "VHT" => 5,
        "HE" => 6,
        "EHT" => 7,
        _ => return None,
    };
    Some(format!("Wi-Fi {number}"))
}

fn packets(stats: Option<&Value>) -> Option<Packets> {
    let stats = stats.filter(|s| s.is_object())?;
    Some(Packets {
        rx_drops: number(stats, "/rx_drops"),
        tx_retries: number(stats, "/tx_retries"),
        rx_packets: number(stats, "/rx_packets"),
        tx_packets: number(stats, "/tx_packets"),
        total_packets: number(stats, "/total_packets"),
        tx_retransmit_ppm: number(stats, "/tx_retransmit_ppm"),
        tx_fail_ppm: number(stats, "/tx_fail_ppm"),
        rx_drop_ppm: number(stats, "/rx_drop_ppm"),
    })
}

/// A device's profile, which eero gives as the profile's record or url.
fn profile_id(profile: Option<&Value>) -> Option<String> {
    match profile? {
        Value::String(url) => id_of(url),
        profile => text(profile, "/url").and_then(|url| id_of(&url)),
    }
}

fn profiles(answer: &Value, usages: &BTreeMap<String, Value>) -> Vec<Profile> {
    list(answer, "")
        .filter_map(|profile| {
            let id = id_of(&text(profile, "/url")?)?;
            Some(Profile {
                name: text(profile, "/name"),
                state: text(profile, "/state/value"),
                paused: flag(profile, "/paused"),
                devices: list(profile, "/devices")
                    .filter_map(|d| text(d, "/mac"))
                    .collect(),
                members: list(profile, "/devices")
                    .filter_map(|d| {
                        Some(Member {
                            mac: text(d, "/mac")?,
                            name: text(d, "/display_name"),
                            kind: text(d, "/device_type"),
                        })
                    })
                    .collect(),
                default: flag(profile, "/default"),
                schedules: profile
                    .pointer("/schedule")
                    .and_then(Value::as_array)
                    .map(Vec::len),
                filters: filters(profile),
                usage: usages.get(&id).and_then(usage),
                id,
            })
        })
        .collect()
}

/// What a profile blocks. eero gives its content filters twice: the
/// unified ones, every kind, and the older Secure ones, four of them, so
/// a kind the unified ones don't say is taken from the older.
fn filters(profile: &Value) -> Filters {
    let policy = |kind: &str| {
        flag(
            profile,
            &format!("/unified_content_filters/dns_policies/{kind}"),
        )
        .or_else(|| flag(profile, &format!("/premium_dns/dns_policies/{kind}")))
    };
    let premium = |at: &str| format!("/premium_dns/{at}");
    Filters {
        set: flag(profile, "/unified_content_filters/is_content_filters_set"),
        gaming: policy("block_gaming_content"),
        illegal: policy("block_illegal_content"),
        messaging: policy("block_messaging_content"),
        pornographic: policy("block_pornographic_content"),
        social: policy("block_social_content"),
        shopping: policy("block_shopping_content"),
        streaming: policy("block_streaming_content"),
        violent: policy("block_violent_content"),
        safe_search: policy("safe_search_enabled"),
        youtube_restricted: policy("youtube_restricted"),
        ad_block: flag(profile, &premium("ad_block_settings/enabled")),
        ad_block_network: flag(profile, &premium("ad_block_settings/enabled_for_network")),
        blocked_sites: texts(profile, &premium("advanced_content_filters/blocked_list")),
        allowed_sites: texts(profile, &premium("advanced_content_filters/allowed_list")),
        blocked_apps: texts(profile, &premium("blocked_applications")),
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
mod tests {
    use super::*;
    use crate::golden::{MODEL, READ, answering, contract, contract_answers, differences, history};
    use serde_json::json;

    #[test]
    fn the_contract_and_its_history_read_into_the_model() {
        let mut read =
            home_network(&contract_answers(), READ, answering()).expect("the network was read");
        // The model is the one the daemon keeps on the machine that's the
        // first laptop, so the views have this machine to put first.
        mark_this_machine(&mut read, &["02:00:00:00:00:0e".to_owned()]);
        history()
            .draw(&mut read, time::to_unix(READ).unwrap())
            .unwrap();

        let out = differences(
            &serde_json::to_value(read).unwrap(),
            &serde_json::from_str(MODEL).unwrap(),
        );

        assert!(
            out.is_empty(),
            "differs from home-network.json:\n{}",
            out.join("\n")
        );
    }

    #[test]
    fn this_machine_is_the_device_with_one_of_its_addresses() {
        let mut read =
            home_network(&contract_answers(), READ, answering()).expect("the network was read");

        mark_this_machine(
            &mut read,
            &[
                "02:00:00:00:00:99".to_owned(),
                "02:00:00:00:00:0E".to_owned(),
            ],
        );

        let here: Vec<&str> = read
            .devices
            .iter()
            .filter(|device| device.this_machine)
            .map(|device| device.mac.as_str())
            .collect();
        assert_eq!(here, ["02:00:00:00:00:0e"]);

        mark_this_machine(&mut read, &[]);

        assert!(read.devices.iter().all(|device| !device.this_machine));
    }

    #[test]
    fn nothing_is_drawn_before_the_network_is_read() {
        let answers = Answers {
            devices: contract("net.devices.json"),
            ..Answers::default()
        };

        assert_eq!(
            home_network(&answers, "2026-11-07T22:54:43Z", answering()),
            None
        );
    }

    #[test]
    fn a_network_read_alone_leaves_the_slower_reads_unknown() {
        let answers = Answers {
            network: contract("network.json"),
            ..Answers::default()
        };

        let read = home_network(&answers, "2026-11-07T22:54:43Z", answering()).unwrap();

        assert_eq!(read.internet.usage, None);
        assert!(read.internet.speed_tests.is_empty());
        assert_eq!(read.internet.thread.name, None);
        assert_eq!(read.nodes.len(), 3);
        assert!(read.nodes.iter().all(|node| node.usage.is_none()));
        let hall = &read.nodes[0];
        assert_eq!(hall.ports[0].best_mbps, None);
        assert!(
            hall.ports[0].upstream,
            "the gateway's linked WAN port carries its backhaul"
        );
    }

    #[test]
    fn a_profile_says_its_schedules_what_it_blocks_and_its_traffic() {
        let first_usage = ("3001".to_owned(), contract("usage.profile.3001.json"));
        let answers = Answers {
            network: contract("network.json"),
            profiles: contract("net.profiles.json"),
            profile_usage: BTreeMap::from([first_usage]),
            ..Answers::default()
        };

        let read = home_network(&answers, READ, answering()).unwrap();

        let first = &read.profiles[0];
        assert_eq!((first.default, first.schedules), (Some(false), Some(0)));
        let filters = &first.filters;
        assert_eq!(filters.set, Some(false));
        assert_eq!(
            (filters.gaming, filters.violent),
            (Some(false), Some(false))
        );
        assert_eq!(filters.youtube_restricted, Some(false));
        assert_eq!(
            (filters.ad_block, filters.ad_block_network),
            (Some(false), Some(false))
        );
        assert!(filters.blocked_sites.is_empty() && filters.blocked_apps.is_empty());
        let usage = first.usage.as_ref().expect("profile 1's traffic was read");
        assert_eq!(usage.download.hours.len(), 25);
        assert_eq!(usage.upload.total_bytes, 0);
        assert_eq!(
            read.profiles[1].usage, None,
            "profile 2's traffic wasn't read"
        );
    }

    #[test]
    fn each_profile_is_named_by_its_id_to_ask_its_traffic() {
        assert_eq!(
            profile_ids(&contract("net.profiles.json")),
            ["3001", "3002", "3003"]
        );
    }

    #[test]
    fn a_kind_the_unified_filters_leave_out_is_the_older_filters_word() {
        let profile = json!({
            "premium_dns": {
                "dns_policies": {
                    "block_pornographic_content": true,
                    "block_violent_content": true
                },
                "ad_block_settings": {"enabled": true, "enabled_for_network": false},
                "advanced_content_filters": {"blocked_list": ["example.com"], "allowed_list": []},
                "blocked_applications": ["tiktok"]
            },
            "unified_content_filters": {
                "is_content_filters_set": true,
                "dns_policies": {"block_gaming_content": true, "block_violent_content": false}
            }
        });

        let filters = filters(&profile);

        assert_eq!(filters.set, Some(true));
        assert_eq!(filters.gaming, Some(true));
        assert_eq!(
            filters.pornographic,
            Some(true),
            "only the older filters say it"
        );
        assert_eq!(
            filters.violent,
            Some(false),
            "the unified filters say it first"
        );
        assert_eq!(filters.shopping, None);
        assert_eq!(filters.ad_block, Some(true));
        assert_eq!(filters.blocked_sites, ["example.com"]);
        assert_eq!(filters.blocked_apps, ["tiktok"]);
    }

    #[test]
    fn a_band_is_drawn_from_the_frequency() {
        assert_eq!(band(2437), "2.4 GHz");
        assert_eq!(band(5180), "5 GHz");
        assert_eq!(band(5925), "6 GHz");
        assert_eq!(band(6115), "6 GHz");
    }

    #[test]
    fn a_rate_eero_explains_says_how_the_radio_makes_it() {
        let info = json!({
            "rate_bps": 2_882_400_000_u64, "mcs": 13, "nss": 2,
            "guard_interval": "GI_800NS", "channel_width": "WIDTH_160MHz", "phy_type": "EHT"
        });

        let rate = rate(Some(&info), Some(48.0)).unwrap();

        assert_eq!(rate.mbps, Some(2882.4));
        assert_eq!((rate.mcs, rate.streams), (Some(13), Some(2)));
        assert_eq!((rate.width_mhz, rate.guard_ns), (Some(160), Some(800)));
        assert_eq!(rate.generation.as_deref(), Some("Wi-Fi 7"));
    }

    #[test]
    fn a_rate_eero_only_names_is_the_bare_bitrate() {
        let empty = json!({"rate_bps": null, "mcs": null, "nss": null,
            "guard_interval": null, "channel_width": null, "phy_type": null});

        let rate = rate(Some(&empty), Some(48.0)).unwrap();

        assert_eq!(rate.mbps, Some(48.0));
        assert_eq!(rate.mcs, None);
        assert_eq!(super::rate(Some(&empty), None), None);
    }

    #[test]
    fn an_ethernet_device_names_its_port_and_speed() {
        let record = json!({
            "mac": "02:00:00:00:00:40", "connected": true, "wireless": false,
            "connectivity": {"ethernet_status": {"port_name": "2", "speed": "P1000"}}
        });

        let device = device(&record).unwrap();

        assert!(device.ethernet);
        let port = device.port.unwrap();
        assert_eq!((port.name.as_deref(), port.mbps), (Some("2"), Some(1000)));
        assert_eq!(device.band, None);
    }

    #[test]
    fn only_devices_on_now_are_listed_by_node_then_signal() {
        let answer = json!([
            {"mac": "a", "connected": true, "source": {"url": "/2.2/eeros/2002"},
             "connectivity": {"signal": "-70 dBm"}},
            {"mac": "b", "connected": true, "source": {"url": "/2.2/eeros/2001"},
             "connectivity": {"signal": "-60 dBm"}},
            {"mac": "c", "connected": true, "source": {"url": "/2.2/eeros/2002"},
             "connectivity": {"signal": "-40 dBm"}},
            {"mac": "d", "connected": false, "source": {"url": "/2.2/eeros/2001"}}
        ]);

        let macs: Vec<String> = devices(&answer).into_iter().map(|d| d.mac).collect();

        assert_eq!(macs, ["b", "c", "a"]);
    }
}
