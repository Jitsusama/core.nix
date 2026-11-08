//! `eero internet`: every figure the record holds under the internet, the
//! home network's way out and each setting that shapes it, in sections.

use super::clock::Clock;
use super::figures::{maybe, said, switch, unknown, yes};
use super::firmware;
use super::home::speed_test;
use super::marks::marks;
use super::nodes::traffic;
use super::text::{Ink, Line, Page, Role, muted, plain, run, strong};
use super::{counted, facts, head, labelled, titled};
use crate::model::{HomeNetwork, Internet};

#[derive(serde::Serialize)]
struct Figures<'a> {
    internet: &'a Internet,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        Figures {
            internet: &model.internet,
        },
    )
}

/// Labelled facts, each a line.
type Facts<'a> = Vec<(&'a str, Line)>;

pub fn page(model: &HomeNetwork, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some("internet".into())),
    );
    let net = &model.internet;
    page.facts(0, way_out(net));
    for (title, facts) in settings(net, clock)
        .into_iter()
        .chain(counts(net, clock, ink))
    {
        page.blank();
        page.line(vec![strong(title)]);
        page.facts(2, facts);
    }
    page
}

/// eero's word for a protocol, set as the acronym it is.
fn acronym(word: &str) -> String {
    match word {
        "nat" | "dhcp" | "pppoe" => word.to_uppercase(),
        _ => word.to_owned(),
    }
}

/// A state eero names, marked as failed unless connected.
fn well(said: Option<&str>) -> Line {
    match said {
        Some(state) if state == "connected" => vec![plain(state)],
        Some(state) => vec![
            run(&marks().failed, Role::Alert),
            plain(format!(" {state}")),
        ],
        None => unknown(),
    }
}

/// How the home network reaches the internet: its state, who carries it,
/// its addresses and how it has them.
fn way_out(net: &Internet) -> Facts<'static> {
    let isp = match net.isp_up {
        Some(true) => vec![plain("ISP up")],
        Some(false) => vec![run(&marks().failed, Role::Alert), plain(" ISP down")],
        None => unknown(),
    };
    let acronymed = |word: Option<&str>| maybe(word, |word| vec![plain(acronym(word))]);
    vec![
        (
            "state",
            facts(vec![
                well(net.state.as_deref()),
                isp,
                labelled("mesh", well(net.mesh.as_deref())),
            ]),
        ),
        (
            "ISP",
            facts(vec![
                said(net.isp.as_deref()),
                maybe(net.asn, |asn| vec![plain(format!("AS{asn}"))]),
            ]),
        ),
        (
            "public address",
            facts(vec![
                said(net.public_address.as_deref()),
                labelled("double NAT", yes(net.double_nat)),
            ]),
        ),
        ("gateway address", said(net.gateway_address.as_deref())),
        ("WAN", said(net.wan_type.as_deref())),
        (
            "lease",
            facts(vec![
                acronymed(net.lease.mode.as_deref()),
                said(net.lease.address.as_deref()),
                labelled("mask", said(net.lease.mask.as_deref())),
                labelled("router", said(net.lease.router.as_deref())),
            ]),
        ),
        ("mode", acronymed(net.mode.as_deref())),
        ("DHCP", said(net.dhcp.as_deref())),
        ("IPv6", switch(net.ipv6)),
    ]
}

/// What's set: the resolvers, the network's switches, its firmware, and
/// the Thread and guest networks.
fn settings<'a>(net: &Internet, clock: &Clock) -> Vec<(&'a str, Facts<'a>)> {
    let dns = &net.dns;
    let servers: Vec<Line> = dns
        .servers
        .iter()
        .map(|server| vec![plain(server)])
        .collect();
    let servers = if servers.is_empty() {
        unknown()
    } else {
        facts(servers)
    };
    vec![
        (
            "DNS",
            vec![
                ("mode", said(dns.mode.as_deref())),
                ("servers", servers),
                ("caching", switch(dns.caching)),
                ("IPv6 mode", said(dns.ipv6_mode.as_deref())),
            ],
        ),
        (
            "settings",
            vec![
                ("SQM", switch(net.sqm)),
                ("UPnP", switch(net.upnp)),
                ("NAT port randomization", switch(net.nat_port_randomization)),
                ("band steering", switch(net.band_steering)),
                ("WPA3", switch(net.wpa3)),
                ("Wi-Fi mode", said(net.wireless_mode.as_deref())),
                ("MLO mode", said(net.mlo_mode.as_deref())),
                ("DDNS", switch(net.ddns)),
                ("backup internet", switch(net.backup_internet)),
                ("power saving", switch(net.power_saving)),
            ],
        ),
        ("firmware", firmware::facts(&net.firmware, clock)),
        (
            "Thread",
            vec![
                ("state", switch(net.thread.on)),
                ("name", said(net.thread.name.as_deref())),
                (
                    "channel",
                    maybe(net.thread.channel, |channel| {
                        vec![plain(channel.to_string())]
                    }),
                ),
            ],
        ),
        (
            "guest network",
            vec![
                ("state", switch(net.guest.on)),
                ("name", said(net.guest.name.as_deref())),
            ],
        ),
    ]
}

/// What eero counted and measured: nodes and devices, the speed tests,
/// the day's traffic and the outages it recorded.
fn counts<'a>(net: &Internet, clock: &Clock, ink: &Ink) -> Vec<(&'a str, Facts<'a>)> {
    let counts = &net.nodes;
    let number =
        |value: Option<u32>, word: &str| maybe(value, |n| vec![plain(format!("{n} {word}"))]);
    let outages: Facts = if net.outages.is_empty() {
        vec![("recorded", vec![plain("none")])]
    } else {
        // eero's own words for each, since no outage has given its shape.
        net.outages
            .iter()
            .map(|outage| ("outage", vec![plain(outage.to_string())]))
            .collect()
    };
    vec![
        (
            "what eero counts",
            vec![
                (
                    "nodes",
                    facts(vec![
                        number(counts.online, "online"),
                        number(counts.offline, "offline"),
                        number(counts.pending, "pending"),
                    ]),
                ),
                (
                    "devices",
                    maybe(net.devices, |n| vec![plain(n.to_string())]),
                ),
            ],
        ),
        (
            "speed test",
            vec![
                ("last", speed_test(&net.speed_test, clock)),
                (
                    "kept",
                    vec![
                        plain(counted(net.speed_tests.len(), "test")),
                        muted(": eero speeds"),
                    ],
                ),
            ],
        ),
        (
            "traffic",
            vec![("last day", traffic(net.usage.as_ref(), ink))],
        ),
        ("outages", outages),
    ]
}
