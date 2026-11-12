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
use crate::model::{BandSecurity, HomeNetwork, Internet, Subnet};

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
        .chain(addresses(model))
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
        Some(true) => vec![plain("provider up")],
        Some(false) => vec![run(&marks().failed, Role::Alert), plain(" provider down")],
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
            "provider",
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
                with_prefix(net.lease.address.as_deref(), net.lease.mask.as_deref()),
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
                ("security", security(&net.security)),
                ("Wi-Fi mode", said(net.wireless_mode.as_deref())),
                ("Wi-Fi 5 compatibility", switch(net.ac_compatibility)),
                ("name clash", yes(net.name_clash)),
                ("MLO mode", state(net.mlo_mode.as_deref())),
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

/// The security each band uses, as its acronym: WPA2 where they agree,
/// else 2.4 GHz WPA2 · 5 GHz WPA3. It stands beside WPA3, which is the
/// switch saying how the network is set, not what a band is using.
fn security(bands: &[BandSecurity]) -> Line {
    let Some(first) = bands.first() else {
        return unknown();
    };
    // Where every band agrees, the one word says it, band by band only
    // where they differ (VKCF3Z, 2026-11-08).
    if bands.iter().all(|band| band.security == first.security) {
        return vec![plain(first.security.to_uppercase())];
    }
    facts(
        bands
            .iter()
            .map(|band| labelled(&band.band, vec![plain(band.security.to_uppercase())]))
            .collect(),
    )
}

/// How the network hands out addresses: each subnet and its DHCP range,
/// the addresses kept for a device, and what's let in from outside, each
/// saying none rather than giving way, since none is the figure.
fn addresses<'a>(model: &HomeNetwork) -> Vec<(&'a str, Facts<'a>)> {
    let kept = &model.internet.addresses;
    let mut each: Facts = kept
        .subnets
        .iter()
        .map(|subnet| {
            let label = match subnet.kind.as_deref() {
                Some("main") => "main",
                Some("guest") => "guest",
                _ => "subnet",
            };
            (label, subnet_line(subnet))
        })
        .collect();
    if kept.reservations.is_empty() {
        each.push(("reserved", vec![plain("none")]));
    }
    for reserved in &kept.reservations {
        // The device's own name where eero lists it, else the label the
        // reservation was given, and that label too when it says more.
        let listed = super::listed_name(model, &reserved.mac);
        let device = listed.as_deref();
        let label = reserved.description.as_deref();
        let name = device.or(label).unwrap_or(&reserved.mac);
        let more = match (device, label) {
            (Some(device), Some(label)) if device != label => labelled("label", vec![plain(label)]),
            _ => Vec::new(),
        };
        each.push((
            "reserved",
            facts(vec![
                said(reserved.address.as_deref()),
                vec![plain(name)],
                more,
            ]),
        ));
    }
    // eero's own words for each, since none has given its shape.
    for (label, list) in [
        ("forwards", &kept.forwards),
        ("IPv6 pinholes", &kept.pinholes),
    ] {
        if list.is_empty() {
            each.push((label, vec![plain("none")]));
        }
        for one in list {
            each.push((label, vec![plain(one.to_string())]));
        }
    }
    vec![("addresses", each)]
}

/// A state eero words as disabled or enabled, said off or on, as every
/// setting's state is; another word as eero gives it.
fn state(said_as: Option<&str>) -> Line {
    said(said_as.map(|word| match word {
        "disabled" => "off",
        "enabled" => "on",
        other => other,
    }))
}

/// An address with its mask as a prefix, 10.0.0.0/22, as every address and
/// mask here is said, the mask whole only where it's no prefix.
fn with_prefix(address: Option<&str>, mask: Option<&str>) -> Line {
    match (address, mask.and_then(prefix)) {
        (Some(address), Some(bits)) => vec![plain(format!("{address}/{bits}"))],
        (address, _) => facts(vec![
            said(address),
            maybe(mask, |mask| labelled("mask", vec![plain(mask)])),
        ]),
    }
}

/// A subnet: how it stands first, a part that isn't so giving way, then
/// its address and prefix and the range DHCP hands out.
fn subnet_line(subnet: &Subnet) -> Line {
    let address = with_prefix(subnet.address.as_deref(), subnet.mask.as_deref());
    let range = match (subnet.first.as_deref(), subnet.last.as_deref()) {
        (Some(first), Some(last)) => labelled("DHCP", vec![plain(format!("{first} to {last}"))]),
        _ => Vec::new(),
    };
    let part = |so: bool, word: &str| if so { vec![plain(word)] } else { Vec::new() };
    facts(vec![
        part(subnet.on == Some(false), "off"),
        part(subnet.internet == Some(false), "no internet"),
        part(subnet.open == Some(true), "open"),
        address,
        range,
    ])
}

/// The prefix length a mask such as 255.255.252.0 is, none for a mask
/// whose ones don't run unbroken from the left.
fn prefix(mask: &str) -> Option<u32> {
    let bits = mask
        .split('.')
        .map(|part| part.parse::<u8>().ok())
        .collect::<Option<Vec<u8>>>()?;
    let [a, b, c, d] = bits.as_slice() else {
        return None;
    };
    let whole = u32::from_be_bytes([*a, *b, *c, *d]);
    let ones = whole.leading_ones();
    (whole.checked_shl(ones).unwrap_or(0) == 0).then_some(ones)
}

/// What eero counted and measured: nodes and devices, the speed tests,
/// the day's traffic and the outages it recorded.
fn counts<'a>(net: &Internet, clock: &Clock, ink: &Ink) -> Vec<(&'a str, Facts<'a>)> {
    let counts = &net.nodes;
    let number =
        |value: Option<u32>, word: &str| maybe(value, |n| vec![plain(format!("{n} {word}"))]);
    // A part that's none gives way, as docs/design.md has counts do.
    let any = |value: Option<u32>, word: &str| {
        if value == Some(0) {
            Vec::new()
        } else {
            number(value, word)
        }
    };
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
            "counted",
            vec![
                (
                    "nodes",
                    facts(vec![
                        number(counts.online, "online"),
                        any(counts.offline, "offline"),
                        any(counts.pending, "pending"),
                    ]),
                ),
                (
                    "devices",
                    maybe(net.devices, |n| vec![plain(format!("{n} known"))]),
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
