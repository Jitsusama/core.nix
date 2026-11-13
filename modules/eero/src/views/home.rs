//! `eero status`: the home network at a glance, every headline figure on a
//! line of its own: the internet, the nodes, the devices, this machine,
//! the last speed test, SQM and the firmware. Each has a view of its own a step in.

use serde::Serialize;

use super::clock::{self, Clock};
use super::figures::{figure, maybe, rate, said, signed, switch, unknown};
use super::json::slice;
use super::marks::marks;
use super::text::{Line, Page, Role, plain, run};
use super::{device_name, facts, head, location, titled};
use crate::model::{Device, Firmware, HomeNetwork, NodeCounts, SpeedTest};

/// The internet's headline figures, as the page shows them.
#[derive(Serialize)]
struct Headline<'a> {
    state: &'a Option<String>,
    isp_up: &'a Option<bool>,
    mesh: &'a Option<String>,
    isp: &'a Option<String>,
    asn: &'a Option<u32>,
    public_address: &'a Option<String>,
    speed_test: &'a SpeedTest,
    sqm: &'a Option<bool>,
    firmware: &'a Firmware,
    nodes: &'a NodeCounts,
    devices: &'a Option<u32>,
}

#[derive(Serialize)]
struct Figures<'a> {
    internet: Headline<'a>,
    /// The devices on now by hardware address, which `eero devices
    /// --json` gives whole.
    devices: Vec<&'a str>,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let net = &model.internet;
    slice(
        model,
        "eero.status/1",
        Figures {
            internet: Headline {
                state: &net.state,
                isp_up: &net.isp_up,
                mesh: &net.mesh,
                isp: &net.isp,
                asn: &net.asn,
                public_address: &net.public_address,
                speed_test: &net.speed_test,
                sqm: &net.sqm,
                firmware: &net.firmware,
                nodes: &net.nodes,
                devices: &net.devices,
            },
            devices: model
                .devices
                .iter()
                .map(|device| device.mac.as_str())
                .collect(),
        },
    )
}

pub fn page(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(&mut page, model, clock, titled(model, None));
    let internet = &model.internet;
    let counts = &internet.nodes;
    let test = &internet.speed_test;
    let firmware = &internet.firmware;
    let mut lines = vec![
        (
            "internet",
            facts(vec![
                state(internet.state.as_deref(), "connected"),
                match internet.isp_up {
                    Some(true) => vec![plain("provider up")],
                    Some(false) => {
                        vec![run(&marks().failed, Role::Alert), plain(" provider down")]
                    }
                    None => unknown(),
                },
                said(internet.isp.as_deref()),
                internet
                    .asn
                    .map_or_else(unknown, |asn| vec![plain(format!("AS{asn}"))]),
                said(internet.public_address.as_deref()),
            ]),
        ),
        (
            "nodes",
            facts(vec![
                state(internet.mesh.as_deref(), "connected"),
                counted(counts.online, "online", Role::Text),
                counted(counts.offline, "offline", Role::Alert),
                counted(counts.pending, "pending", Role::Warning),
            ]),
        ),
        (
            "devices",
            facts(vec![
                vec![plain(format!("{} on now", model.devices.len()))],
                internet
                    .devices
                    .map_or_else(unknown, |count| vec![plain(format!("{count} known"))]),
            ]),
        ),
        ("speed test", speed_test(test, clock)),
        ("SQM", switch(internet.sqm)),
        (
            "firmware",
            facts(vec![
                said(firmware.running.as_deref()),
                match (firmware.has_update, firmware.next.as_deref()) {
                    (Some(true), Some(next)) => {
                        vec![
                            run(&marks().on, Role::Warning),
                            plain(format!(" {next} waiting")),
                        ]
                    }
                    (Some(true), None) => {
                        vec![run(&marks().on, Role::Warning), plain(" an update waiting")]
                    }
                    (Some(false), _) => vec![plain("up to date")],
                    (None, _) => unknown(),
                },
            ]),
        ),
    ];
    // This machine has a line of its own, under the devices, when eero
    // lists it, since it's the one device Joel most wants to find.
    if let Some(here) = model.devices.iter().find(|device| device.this_machine) {
        lines.insert(3, ("this machine", this_machine(model, here)));
    }
    page.facts(0, lines);
    page
}

/// This machine as the home network has it: its name, the node it's on,
/// and its band and signal, or that it's on Ethernet.
fn this_machine(model: &HomeNetwork, device: &Device) -> Line {
    let link = if device.ethernet {
        vec![vec![plain("Ethernet")]]
    } else {
        vec![
            said(device.band.as_deref()),
            maybe(device.signal_dbm, |dbm| figure(signed(dbm.into()), "dBm")),
        ]
    };
    let mut parts = vec![
        vec![plain(device_name(device))],
        vec![plain(location(model, device.node.as_deref()))],
    ];
    parts.extend(link);
    facts(parts)
}

/// A state eero names, marked as failed when it isn't the one all is well
/// in.
fn state(said: Option<&str>, well: &str) -> Line {
    match said {
        Some(state) if state.eq_ignore_ascii_case(well) => vec![plain(state)],
        Some(state) => vec![
            run(&marks().failed, Role::Alert),
            plain(format!(" {state}")),
        ],
        None => unknown(),
    }
}

/// A count of nodes in a state, marked in the state's role when there are
/// any. A state only a trouble is in gives way when no node is in it, as
/// docs/design.md has a count's part that's none do.
fn counted(count: Option<u32>, state: &str, role: Role) -> Line {
    match count {
        Some(0) if role != Role::Text => Vec::new(),
        Some(count) if count > 0 && role != Role::Text => {
            let mark = if role == Role::Alert {
                &marks().failed
            } else {
                &marks().on
            };
            vec![run(mark, role), plain(format!(" {count} {state}"))]
        }
        Some(count) => vec![plain(format!("{count} {state}"))],
        None => unknown(),
    }
}

/// The last speed test: down, up, and when.
pub fn speed_test(test: &SpeedTest, clock: &Clock) -> Line {
    let mbit = |value: Option<f64>, way: &str| match value {
        Some(mbps) => {
            let mut line = rate(mbps);
            line.push(plain(format!(" {way}")));
            line
        }
        None => unknown(),
    };
    let mut parts = vec![mbit(test.down_mbps, "down"), mbit(test.up_mbps, "up")];
    if let Some(at) = test.at.as_deref().and_then(clock::parse) {
        parts.push(vec![plain(format!(
            "{}, {}",
            clock.minute(at),
            clock.ago(at)
        ))]);
    }
    facts(parts)
}
