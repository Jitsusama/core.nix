//! `eero nodes`, every node on a row of its own, and `eero node`, one node
//! whole: its state, backhaul, addresses, firmware and age, each radio and
//! each port, and the devices on it.

use super::clock::{self, Clock, span};
use super::devices;
use super::figures::{bytes, figure, maybe, radio, rate, said, signed, switch, unknown};
use super::graph::{bar, spark};
use super::marks::marks;
use super::text::{Align, Ink, Line, Page, Role, Table, muted, plain, run, strong};
use super::{counted, facts, head, labelled, node_name, titled, when};
use crate::model::{BackhaulKind, HomeNetwork, Node, Usage};

/// A busy share this high warns, as the design has every graph warn.
const BUSY_WARNS_PCT: u8 = 95;

#[derive(serde::Serialize)]
struct Every<'a> {
    nodes: &'a [Node],
}

#[derive(serde::Serialize)]
struct One<'a> {
    node: &'a Node,
    /// The devices on it now.
    devices: Vec<&'a crate::model::Device>,
}

pub fn all_json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        "eero.nodes/1",
        Every {
            nodes: &model.nodes,
        },
    )
}

pub fn one_json(model: &HomeNetwork, node: &Node) -> Result<String, String> {
    let devices = super::json::on(model, |device| {
        device.node.as_deref() == Some(node.id.as_str())
    });
    super::json::slice(model, "eero.node/1", One { node, devices })
}

pub fn all(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(model.nodes.len(), "node"))),
    );
    let mut table = Table::new(&[
        Align::Left,
        Align::Left,
        Align::Left,
        Align::Left,
        Align::Left,
        Align::Point,
        Align::Right,
        Align::Right,
        Align::Right,
        Align::Left,
        Align::Left,
        Align::Left,
    ]);
    table.header(&[
        "node",
        "model",
        "state",
        "heartbeat",
        "backhaul",
        "bars",
        "devices",
        "Wi-Fi",
        "Ethernet",
        "firmware",
        "up",
        "address",
    ]);
    for node in &model.nodes {
        let number = |value: Option<u32>| maybe(value, |n| vec![plain(n.to_string())]);
        table.row(vec![
            vec![strong(node_name(node))],
            said(node.model.as_deref()),
            state(node),
            heartbeat(node, clock),
            backhaul(node),
            maybe(node.backhaul.bars, |bars| figure(bars.to_string(), "of 5")),
            number(node.devices.all),
            number(node.devices.wifi),
            number(node.devices.ethernet),
            firmware(node),
            maybe(node.up_s, |up| vec![plain(span(up))]),
            said(node.address.as_deref()),
        ]);
    }
    page.extend(table.lines(0));
    page
}

/// Whether a node is up: eero's word, marked as failed unless online.
pub fn state(node: &Node) -> Line {
    match node.state.as_deref() {
        Some(state) if state.eq_ignore_ascii_case("online") => vec![plain("online")],
        Some(state) => vec![
            run(&marks().failed, Role::Alert),
            plain(format!(" {}", state.to_lowercase())),
        ],
        None => unknown(),
    }
}

/// When a node last said it was up, marked as failed when eero says its
/// heartbeat isn't.
fn heartbeat(node: &Node, clock: &Clock) -> Line {
    let at = node.last_heartbeat.as_deref().and_then(clock::parse);
    let mut line = match node.heartbeat_ok {
        Some(false) => vec![run(&marks().failed, Role::Alert), plain(" ")],
        _ => Vec::new(),
    };
    match at {
        Some(at) => line.push(plain(clock.ago(at))),
        None if line.is_empty() => return unknown(),
        None => line.push(plain("missed")),
    }
    line
}

/// How a node reaches the one before it: Ethernet, or Wi-Fi from its
/// upstream over a radio.
pub fn backhaul(node: &Node) -> Line {
    match node.backhaul.kind {
        BackhaulKind::Ethernet => vec![plain("Ethernet")],
        BackhaulKind::WiFi => {
            let mut line = vec![plain("Wi-Fi")];
            if let Some(said) = node.backhaul.radio.as_deref() {
                line.push(plain(format!(" {}", radio(said))));
            }
            if let Some(upstream) = node.backhaul.upstream.as_deref() {
                line.push(muted(" from "));
                line.push(plain(upstream));
            }
            line
        }
    }
}

fn firmware(node: &Node) -> Line {
    let mut line = said(node.firmware.as_deref());
    if node.update_available == Some(true) {
        line.push(plain(" "));
        line.push(run(&marks().on, Role::Warning));
        line.push(plain(" update"));
    }
    line
}

/// A day's traffic, each way: its total and its spark. The record keeps
/// it as eero names it, usage.
pub fn traffic(usage: Option<&Usage>, ink: &Ink) -> Line {
    let Some(usage) = usage else { return unknown() };
    let way = |series: &crate::model::Series, word: &str| {
        let mut line = bytes(series.total_bytes);
        line.push(plain(format!(" {word}  ")));
        #[expect(clippy::cast_precision_loss, reason = "a graph's height")]
        let values: Vec<f64> = series.hours.iter().map(|hour| hour.bytes as f64).collect();
        line.extend(spark(&values, ink.coloured()));
        line
    };
    let mut line = way(&usage.download, "down");
    line.push(plain("   "));
    line.extend(way(&usage.upload, "up"));
    line
}

pub fn one(model: &HomeNetwork, node: &Node, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    let mut title = vec![strong(node_name(node))];
    for part in [
        node.model.clone(),
        node.gateway.then(|| "gateway".to_owned()),
    ]
    .into_iter()
    .flatten()
    {
        title.push(muted(format!(" {} ", marks().separator)));
        title.push(plain(part));
    }
    head(&mut page, model, clock, title);
    page.facts(0, about(node, clock, ink));

    page.blank();
    page.line(vec![strong("radios")]);
    page.extend(radios(node, ink).lines(2));

    page.blank();
    page.line(vec![strong("ports")]);
    page.extend(ports(node).lines(2));

    let on = super::on_node(model, Some(node.id.as_str()));
    page.blank();
    page.line(vec![
        strong("devices"),
        muted(format!(" {} {}", marks().separator, on.len())),
    ]);
    let mut table = devices::table(false);
    for device in on {
        devices::row(&mut table, None, device);
    }
    page.extend(table.lines(2));
    page
}

/// A node's own figures: its state, backhaul, names, addresses, firmware,
/// age, light and power, devices and the day's traffic.
fn about(node: &Node, clock: &Clock, ink: &Ink) -> Vec<(&'static str, Line)> {
    let at = |stamp: Option<&str>| when(stamp, clock);
    let mut addresses: Vec<Line> = node.address.iter().map(|a| vec![plain(a)]).collect();
    addresses.extend(node.ipv6.iter().map(|a| vec![plain(a)]));
    let mut backhaul = backhaul(node);
    if let Some(bars) = node.backhaul.bars {
        backhaul = facts(vec![backhaul, figure(bars.to_string(), "of 5 bars")]);
    }
    vec![
        (
            "state",
            facts(vec![
                state(node),
                labelled("heartbeat", heartbeat(node, clock)),
            ]),
        ),
        ("backhaul", backhaul),
        (
            "model",
            facts(vec![
                said(node.model.as_deref()),
                said(node.model_number.as_deref()),
                maybe(node.serial.as_deref(), |serial| {
                    vec![muted("serial "), plain(serial)]
                }),
            ]),
        ),
        (
            "addresses",
            if addresses.is_empty() {
                unknown()
            } else {
                facts(addresses)
            },
        ),
        ("hardware", said(node.mac.as_deref())),
        (
            "firmware",
            facts(vec![
                said(node.firmware.as_deref()),
                match node.update_available {
                    Some(true) => {
                        vec![run(&marks().on, Role::Warning), plain(" an update waiting")]
                    }
                    Some(false) => vec![plain("up to date")],
                    None => unknown(),
                },
                labelled("support ends", at(node.support_ends.as_deref())),
            ]),
        ),
        (
            "up",
            facts(vec![
                maybe(node.up_s, |up| vec![plain(span(up))]),
                labelled("rebooted", at(node.last_reboot.as_deref())),
            ]),
        ),
        (
            "on eero's cloud",
            maybe(node.cloud_s, |secs| vec![plain(span(secs))]),
        ),
        ("joined", at(node.joined.as_deref())),
        (
            "light",
            node.light.as_ref().map_or_else(unknown, |light| {
                facts(vec![
                    switch(light.on),
                    maybe(light.brightness, |level| figure(level.to_string(), "%")),
                ])
            }),
        ),
        ("power", said(node.power.as_deref())),
        (
            "devices",
            facts(vec![
                maybe(node.devices.all, |n| vec![plain(n.to_string())]),
                maybe(node.devices.wifi, |n| vec![plain(format!("{n} Wi-Fi"))]),
                maybe(node.devices.ethernet, |n| {
                    vec![plain(format!("{n} Ethernet"))]
                }),
            ]),
        ),
        ("traffic", traffic(node.usage.as_ref(), ink)),
    ]
}

/// A node's ports, a row each: whether it's linked, its speed against its
/// best, the bytes each way, and whether it's the WAN or the backhaul.
fn ports(node: &Node) -> Table {
    let mut table = Table::new(&[
        Align::Left,
        Align::Left,
        Align::Point,
        Align::Point,
        Align::Point,
        Align::Point,
        Align::Left,
    ]);
    table.header(&["port", "link", "speed", "best", "rx", "tx", ""]);
    for port in &node.ports {
        let mbit = |value: Option<u32>| maybe(value, |mbps| rate(mbps.into()));
        let mut linked = vec![plain(if port.linked { "linked" } else { "no link" })];
        // A link slower than its port can go is a cable or a far end worth
        // a look.
        if port.linked
            && matches!((port.mbps, port.best_mbps), (Some(speed), Some(best)) if speed < best)
        {
            linked.extend([
                plain(" "),
                run(&marks().on, Role::Warning),
                plain(" under its best"),
            ]);
        }
        let roles: Vec<&str> = [(port.wan, "WAN"), (port.upstream, "backhaul")]
            .into_iter()
            .filter_map(|(is, word)| is.then_some(word))
            .collect();
        table.row(vec![
            vec![plain(&port.name)],
            linked,
            mbit(port.mbps),
            mbit(port.best_mbps),
            maybe(port.rx_bytes, bytes),
            maybe(port.tx_bytes, bytes),
            facts(roles.into_iter().map(|word| vec![plain(word)]).collect()),
        ]);
    }
    table
}

/// A node's radios, a row each: band, channel, width, power, how busy,
/// devices and BSSID.
pub fn radios(node: &Node, ink: &Ink) -> Table {
    let mut table = radio_table(false);
    for radio in sorted(&node.radios) {
        radio_row(&mut table, None, radio, ink);
    }
    table
}

/// The radios' table, with a column naming each radio's node when it
/// holds several nodes' radios.
pub fn radio_table(with_node: bool) -> Table {
    let mut aligns = vec![Align::Left];
    let mut headings = vec!["band"];
    if with_node {
        aligns.push(Align::Left);
        headings.push("node");
    }
    aligns.extend([
        Align::Right,
        Align::Point,
        Align::Point,
        Align::Point,
        Align::Left,
        Align::Right,
        Align::Left,
    ]);
    headings.extend(["channel", "width", "power", "busy", "", "devices", "BSSID"]);
    let mut table = Table::new(&aligns);
    table.header(&headings);
    table
}

pub fn radio_row(
    table: &mut Table,
    node: Option<(Line, &str)>,
    radio: &crate::model::Radio,
    ink: &Ink,
) {
    let (band, node) = match node {
        Some((band, node)) => (band, Some(node)),
        None => (vec![plain(&radio.band)], None),
    };
    let mut cells = vec![band];
    if let Some(node) = node {
        cells.push(vec![strong(node)]);
    }
    cells.extend([
        maybe(radio.channel, |channel| vec![plain(channel.to_string())]),
        maybe(radio.width_mhz, |mhz| figure(mhz.to_string(), "MHz")),
        maybe(radio.power_dbm, |dbm| figure(signed(dbm.into()), "dBm")),
        maybe(radio.busy_pct, |pct| figure(pct.to_string(), "%")),
        maybe(radio.busy_pct, |pct| {
            bar(
                f64::from(pct) / 100.0,
                ink.coloured(),
                pct >= BUSY_WARNS_PCT,
            )
        }),
        maybe(radio.devices, |n| vec![plain(n.to_string())]),
        said(radio.bssid.as_deref()),
    ]);
    table.row(cells);
}

/// Where a band sits among the others, lowest first, so every node's
/// radios read in one order.
pub fn band_rank(band: &str) -> (u32, String) {
    let ghz = band
        .split_whitespace()
        .next()
        .and_then(|n| n.parse::<f64>().ok())
        .unwrap_or(99.0);
    let high = band.contains("high");
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a band in GHz"
    )]
    let rank = (ghz * 10.0) as u32 * 2 + u32::from(high);
    (rank, band.to_owned())
}

pub fn sorted(radios: &[crate::model::Radio]) -> Vec<&crate::model::Radio> {
    let mut sorted: Vec<_> = radios.iter().collect();
    sorted.sort_by_key(|radio| band_rank(&radio.band));
    sorted
}
