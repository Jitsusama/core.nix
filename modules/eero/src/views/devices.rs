//! `eero devices`, every device on now under the node it's on and then
//! each eero knows that isn't, and `eero device`, one device with every
//! figure eero has for it.

use super::clock::Clock;
use super::figures::{count, figure, maybe, rate, said, signed, unknown, words, yes};
use super::marks::marks;
use super::text::{Align, Line, Page, Table, muted, plain, strong};
use super::{
    counted, device_name, facts, head, labelled, location, off_name, on_node, titled, when,
};
use crate::model::{Device, DeviceOff, HomeNetwork, Rate};

/// The fewest cells a device's name is shortened to, which still shows
/// both its ends.
pub const NAMES: usize = 12;

#[derive(serde::Serialize)]
struct Every<'a> {
    devices: &'a [Device],
    devices_off: &'a [DeviceOff],
}

#[derive(serde::Serialize)]
struct One<'a> {
    device: &'a Device,
}

pub fn all_json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        "eero.devices/1",
        Every {
            devices: &model.devices,
            devices_off: &model.devices_off,
        },
    )
}

pub fn one_json(model: &HomeNetwork, device: &Device) -> Result<String, String> {
    super::json::slice(model, "eero.device/1", One { device })
}

pub fn all(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    let title = titled(
        model,
        Some(format!("{} on now", counted(model.devices.len(), "device"))),
    );
    head(&mut page, model, clock, title);
    let mut table = table(true);
    let mut groups: Vec<Option<&str>> = model
        .nodes
        .iter()
        .map(|node| Some(node.id.as_str()))
        .collect();
    // A device on a node the record doesn't list still shows, under its
    // node's id, so nothing on now is left out.
    for device in &model.devices {
        if !groups.contains(&device.node.as_deref()) {
            groups.push(device.node.as_deref());
        }
    }
    let mut first = true;
    for id in groups {
        let on = on_node(model, id);
        if on.is_empty() {
            continue;
        }
        // A line of air between nodes, and the node named on its first
        // row only, so its devices read as one group.
        if !first {
            table.across(Vec::new());
        }
        first = false;
        for (at, device) in on.into_iter().enumerate() {
            let node = if at == 0 {
                vec![strong(location(model, id))]
            } else {
                Vec::new()
            };
            row(&mut table, Some(node), device);
        }
    }
    // The devices on now and those that aren't lead with the same
    // columns, which line up from one table into the next.
    page.shared_table(0, table);
    off(&mut page, model, clock);
    page
}

/// The devices eero knows that aren't on now, the last on first: where and
/// how each was on then, in the columns the devices on now lead with, and
/// when. Their link's figures are of a link that's gone, so the model
/// doesn't keep them.
fn off(page: &mut Page, model: &HomeNetwork, clock: &Clock) {
    if model.devices_off.is_empty() {
        return;
    }
    page.blank();
    page.line(vec![strong(format!(
        "{} not on now",
        counted(model.devices_off.len(), "device")
    ))]);
    page.blank();
    let mut table = Table::new(&[Align::Left; 7]);
    table.header(&[
        "node", "device", "kind", "band", "last on", "profile", "made by",
    ]);
    // Where and when it was last on say most of a device that's gone.
    table.gives_way_by(&["kind", "profile", "made by", "band"]);
    table.shortens(1, NAMES);
    for device in &model.devices_off {
        let band = if device.ethernet {
            vec![plain("Ethernet")]
        } else {
            said(device.band.as_deref())
        };
        let profile = device.profile.as_deref().map(|id| profile_named(model, id));
        table.row(vec![
            maybe(device.node.as_deref(), |id| {
                vec![plain(location(model, Some(id)))]
            }),
            vec![plain(off_name(device))],
            maybe(device.kind.as_deref(), |kind| vec![plain(words(kind))]),
            band,
            maybe(
                device.last_active.as_deref().and_then(super::clock::parse),
                |at| vec![plain(clock.ago(at))],
            ),
            said(profile),
            said(device.manufacturer.as_deref()),
        ]);
    }
    page.shared_table(0, table);
}

/// The name of the profile with an id, or the id where eero names none.
fn profile_named<'a>(model: &'a HomeNetwork, id: &'a str) -> &'a str {
    model
        .profiles
        .iter()
        .find(|profile| profile.id == id)
        .and_then(|profile| profile.name.as_deref())
        .unwrap_or(id)
}

/// The devices' table: a row each, its link's figures and its speed now,
/// after a column for its node when it holds several nodes' devices.
pub fn table(with_node: bool) -> Table {
    let mut aligns = Vec::new();
    let mut headings = Vec::new();
    if with_node {
        aligns.push(Align::Left);
        headings.push("node");
    }
    aligns.extend([
        Align::Left,
        Align::Left,
        Align::Left,
        Align::Right,
        Align::Point,
        Align::Point,
        Align::Point,
        Align::Right,
        Align::Right,
        Align::Point,
        Align::Right,
        Align::Right,
        Align::Point,
        Align::Point,
        Align::Left,
    ]);
    headings.extend([
        "device", "kind", "band", "ch", "signal", "SNR", "rx", "MCS", "NSS", "tx", "MCS", "NSS",
        "down", "up", "address",
    ]);
    let mut table = Table::new(&aligns);
    table.header(&headings);
    // Into a narrow terminal what says least of how a device is doing goes
    // first: its kind, which its name mostly says, then how its radio makes
    // each rate, its channel and address, the way out, and its traffic; its
    // node, name and signal stay.
    table.gives_way_by(&[
        "kind", "NSS", "MCS", "ch", "address", "tx", "SNR", "up", "down", "rx", "band",
    ]);
    table.shortens(usize::from(with_node), NAMES);
    table
}

pub fn row(table: &mut Table, node: Option<Line>, device: &Device) {
    let number = |value: Option<u8>| maybe(value, |n| vec![plain(n.to_string())]);
    let (rx, tx) = (device.rx.as_ref(), device.tx.as_ref());
    let band = if device.ethernet {
        vec![plain("Ethernet")]
    } else {
        said(device.band.as_deref())
    };
    let mut cells: Vec<Line> = node.into_iter().collect();
    cells.extend([
        vec![plain(device_name(device))],
        maybe(device.kind.as_deref(), |kind| vec![plain(words(kind))]),
        band,
        maybe(device.channel, |channel| vec![plain(channel.to_string())]),
        maybe(device.signal_dbm, |dbm| figure(signed(dbm.into()), "dBm")),
        maybe(device.snr_db, |db| figure(db.to_string(), "dB")),
        maybe(rx.and_then(|rate| rate.mbps), rate),
        number(rx.and_then(|rate| rate.mcs)),
        number(rx.and_then(|rate| rate.streams)),
        maybe(tx.and_then(|rate| rate.mbps), rate),
        number(tx.and_then(|rate| rate.mcs)),
        number(tx.and_then(|rate| rate.streams)),
        maybe(device.speed.down_mbps, rate),
        maybe(device.speed.up_mbps, rate),
        said(device.address.as_deref()),
    ]);
    table.row(cells);
}

/// One direction's rate, and how the radio makes it.
fn link(rate: Option<&Rate>) -> Line {
    let Some(rate) = rate else { return unknown() };
    let mut parts = vec![maybe(rate.mbps, super::figures::rate)];
    if let Some(mcs) = rate.mcs {
        parts.push(vec![muted("MCS "), plain(mcs.to_string())]);
    }
    if let Some(streams) = rate.streams {
        parts.push(vec![plain(counted(streams.into(), "stream"))]);
    }
    if let Some(mhz) = rate.width_mhz {
        parts.push(figure(mhz.to_string(), "MHz"));
    }
    if let Some(ns) = rate.guard_ns {
        parts.push(figure(ns.to_string(), "ns guard"));
    }
    if let Some(generation) = rate.generation.as_deref() {
        parts.push(vec![plain(generation)]);
    }
    facts(parts)
}

pub fn one(model: &HomeNetwork, device: &Device, clock: &Clock) -> Page {
    let mut page = Page::default();
    let title = vec![
        strong(device_name(device)),
        muted(format!(" {} ", marks().separator)),
        plain(&device.mac),
    ];
    head(&mut page, model, clock, title);
    let mut shown = who(model, device, clock);
    shown.extend(link_facts(device));
    shown.extend(carried(model, device));
    page.facts(0, shown);
    page
}

/// What a device is, what it's called, its addresses, when it came, and
/// where it joined.
fn who<'a>(model: &'a HomeNetwork, device: &'a Device, clock: &Clock) -> Vec<(&'a str, Line)> {
    let at = |stamp: Option<&str>| when(stamp, clock);
    let mut what = vec![maybe(device.kind.as_deref(), |kind| {
        vec![plain(words(kind))]
    })];
    let made: Vec<&str> = [device.make.as_deref(), device.model.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    if !made.is_empty() {
        what.push(vec![plain(made.join(" "))]);
    }
    if let Some(maker) = device.manufacturer.as_deref() {
        what.push(labelled("made by", vec![plain(maker)]));
    }
    let addresses: Vec<Line> = device.addresses.iter().map(|a| vec![plain(a)]).collect();
    let node = location(model, device.node.as_deref());
    // The address eero keeps for it, which no device has unless one's set,
    // and none said as every empty list here says it.
    let reserved = model
        .internet
        .addresses
        .reservations
        .iter()
        .find(|kept| kept.mac.eq_ignore_ascii_case(&device.mac))
        .map_or_else(|| vec![plain("none")], |kept| said(kept.address.as_deref()));
    vec![
        ("kind", facts(what)),
        (
            "name",
            facts(vec![
                said(device.name.as_deref()),
                labelled("nickname", said(device.nickname.as_deref())),
                labelled("hostname", said(device.hostname.as_deref())),
            ]),
        ),
        (
            "hardware",
            facts(vec![
                vec![plain(&device.mac)],
                match device.private {
                    Some(true) => vec![plain("a private address")],
                    Some(false) => vec![plain("its maker's address")],
                    None => unknown(),
                },
            ]),
        ),
        (
            "addresses",
            if addresses.is_empty() {
                said(device.address.as_deref())
            } else {
                facts(addresses)
            },
        ),
        ("reserved", reserved),
        ("on since", at(device.first_active.as_deref())),
        ("last active", at(device.last_active.as_deref())),
        ("first seen", at(device.first_seen.as_deref())),
        (
            "node",
            facts(vec![
                vec![strong(node)],
                vec![plain(if device.ethernet { "Ethernet" } else { "Wi-Fi" })],
            ]),
        ),
        (
            "network",
            facts(vec![
                said(device.ssid.as_deref()),
                maybe(device.auth.as_deref(), |auth| {
                    vec![plain(auth.to_uppercase())]
                }),
                labelled("subnet", said(device.subnet.as_deref())),
                labelled("guest", yes(device.guest)),
            ]),
        ),
    ]
}

/// How a device is joined: its port over Ethernet, or over Wi-Fi its
/// radio, signal, each direction's rate and its retries.
fn link_facts(device: &Device) -> Vec<(&'static str, Line)> {
    let mut shown = Vec::new();
    if device.ethernet || device.port.is_some() {
        shown.push((
            "port",
            device.port.as_ref().map_or_else(unknown, |port| {
                facts(vec![
                    labelled("port", said(port.name.as_deref())),
                    maybe(port.mbps, |mbps| rate(mbps.into())),
                ])
            }),
        ));
    }
    if !device.ethernet {
        shown.extend([
            (
                "radio",
                facts(vec![
                    said(device.band.as_deref()),
                    labelled(
                        "channel",
                        maybe(device.channel, |c| vec![plain(c.to_string())]),
                    ),
                    maybe(device.frequency_mhz, |mhz| figure(count(mhz.into()), "MHz")),
                    labelled("BSSID", said(device.bssid.as_deref())),
                ]),
            ),
            (
                "signal",
                facts(vec![
                    maybe(device.signal_dbm, |dbm| figure(signed(dbm.into()), "dBm")),
                    labelled(
                        "SNR",
                        maybe(device.snr_db, |db| figure(db.to_string(), "dB")),
                    ),
                    labelled(
                        "score",
                        maybe(device.score, |score| vec![plain(format!("{score:.3}"))]),
                    ),
                    maybe(device.bars, |bars| figure(bars.to_string(), "of 5 bars")),
                ]),
            ),
            ("rx", link(device.rx.as_ref())),
            ("tx", link(device.tx.as_ref())),
            (
                "retries",
                facts(vec![
                    maybe(device.retry_pct, |pct| figure(format!("{pct:.0}"), "%")),
                    labelled("idle", maybe(device.idle_ms, |ms| figure(count(ms), "ms"))),
                ]),
            ),
        ]);
    }
    shown
}

/// What a device carries: its packets, their faults per million, its speed
/// now, and the profile that holds it.
fn carried(model: &HomeNetwork, device: &Device) -> Vec<(&'static str, Line)> {
    let packets = device.packets.as_ref();
    let packet =
        |label: &str, value: Option<u64>| labelled(label, maybe(value, |n| vec![plain(count(n))]));
    let ppm = |label: &str, value: Option<u32>| {
        labelled(label, maybe(value, |n| vec![plain(count(n.into()))]))
    };
    let profile = device.profile.as_deref().map(|id| profile_named(model, id));
    vec![
        (
            "packets",
            facts(vec![
                packet("rx", packets.and_then(|p| p.rx_packets)),
                packet("tx", packets.and_then(|p| p.tx_packets)),
                packet("all", packets.and_then(|p| p.total_packets)),
                packet("tx retries", packets.and_then(|p| p.tx_retries)),
                packet("rx drops", packets.and_then(|p| p.rx_drops)),
            ]),
        ),
        (
            "per million",
            facts(vec![
                ppm(
                    "tx retransmitted",
                    packets.and_then(|p| p.tx_retransmit_ppm),
                ),
                ppm("tx failed", packets.and_then(|p| p.tx_fail_ppm)),
                ppm("rx dropped", packets.and_then(|p| p.rx_drop_ppm)),
            ]),
        ),
        (
            "speed",
            facts(vec![
                labelled("down", maybe(device.speed.down_mbps, rate)),
                labelled("up", maybe(device.speed.up_mbps, rate)),
            ]),
        ),
        (
            "profile",
            facts(vec![
                said(profile),
                labelled("paused", yes(device.paused)),
                labelled("blocked", yes(device.blocked)),
            ]),
        ),
    ]
}
