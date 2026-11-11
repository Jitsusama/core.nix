//! The devices drawn from eero's device list: those on the network, each
//! with its node, band, link rate and packets, and those off it, the last
//! on first.

use super::nodes::port_mbps;
use super::profiles::profile_id;
use super::{digits_in, flag, float, id_of, leading_number, list, number, text, texts, time};
use crate::model::{Device, DeviceHistory, DeviceOff, DevicePort, Packets, Rate, Speed};
use serde_json::Value;
use std::cmp::Reverse;

pub(super) fn devices(answer: &Value) -> Vec<Device> {
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

/// The devices eero knows that aren't on now, the last on first.
pub(super) fn devices_off(answer: &Value) -> Vec<DeviceOff> {
    let mut off: Vec<DeviceOff> = list(answer, "")
        .filter(|record| flag(record, "/connected") != Some(true))
        .filter_map(device)
        .map(|device| DeviceOff {
            mac: device.mac,
            name: device.name,
            nickname: device.nickname,
            hostname: device.hostname,
            kind: device.kind,
            make: device.make,
            model: device.model,
            manufacturer: device.manufacturer,
            private: device.private,
            first_seen: device.first_seen,
            last_active: device.last_active,
            node: device.node,
            ethernet: device.ethernet,
            band: device.band,
            profile: device.profile,
        })
        .collect();
    // eero's times are all written one way, so they sort as text, and one
    // it gave no time comes last.
    off.sort_by(|a, b| {
        b.last_active
            .cmp(&a.last_active)
            .then_with(|| a.mac.cmp(&b.mac))
    });
    off
}

pub(super) fn device(record: &Value) -> Option<Device> {
    let link = record.get("connectivity").unwrap_or(&Value::Null);
    // eero says 0 for a device on Ethernet, which has no radio, rather
    // than leaving it out, and 0 would read as 2.4 GHz.
    let frequency_mhz = number(link, "/frequency").filter(|mhz| *mhz > 0);
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
pub(super) fn band(mhz: u32) -> &'static str {
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
pub(super) fn rate(info: Option<&Value>, bare_mbps: Option<f64>) -> Option<Rate> {
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
