//! `eero firmware`: what the home network runs, what's next and when it
//! updates, and each node's own firmware and the end of its support.

use super::clock::Clock;
use super::figures::{maybe, said, unknown, yes};
use super::marks::marks;
use super::text::{Align, Line, Page, Role, Table, muted, plain, run, strong};
use super::{head, node_name, titled, when};
use crate::model::{Firmware, HomeNetwork};

/// A node's own firmware and the end of its support.
#[derive(serde::Serialize)]
struct NodeFirmware<'a> {
    id: &'a str,
    location: &'a Option<String>,
    firmware: &'a Option<String>,
    update_available: &'a Option<bool>,
    support_ends: &'a Option<String>,
}

#[derive(serde::Serialize)]
struct Figures<'a> {
    firmware: &'a Firmware,
    nodes: Vec<NodeFirmware<'a>>,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let nodes = model
        .nodes
        .iter()
        .map(|node| NodeFirmware {
            id: &node.id,
            location: &node.location,
            firmware: &node.firmware,
            update_available: &node.update_available,
            support_ends: &node.support_ends,
        })
        .collect();
    super::json::slice(
        model,
        "eero.firmware/1",
        Figures {
            firmware: &model.internet.firmware,
            nodes,
        },
    )
}

/// The home network's firmware, as facts, for this view and the
/// internet's.
pub fn facts<'a>(firmware: &Firmware, clock: &Clock) -> Vec<(&'a str, Line)> {
    vec![
        ("running", said(firmware.running.as_deref())),
        ("next", said(firmware.next.as_deref())),
        (
            "update waiting",
            match firmware.has_update {
                Some(true) => vec![run(&marks().on, Role::Warning), plain(" yes")],
                other => yes(other),
            },
        ),
        ("can update now", yes(firmware.can_update_now)),
        (
            "updates at",
            maybe(firmware.update_hour, |hour| {
                vec![
                    plain(format!("{hour:02}:00")),
                    muted(", the network's time"),
                ]
            }),
        ),
        (
            "last started",
            when(firmware.last_started.as_deref(), clock),
        ),
    ]
}

pub fn page(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some("firmware".into())),
    );
    page.facts(0, facts(&model.internet.firmware, clock));
    page.blank();
    let mut table = Table::new(&[Align::Left, Align::Left, Align::Left, Align::Left]);
    table.header(&["node", "firmware", "update", "support ends"]);
    for node in &model.nodes {
        table.row(vec![
            vec![strong(node_name(node))],
            said(node.firmware.as_deref()),
            match node.update_available {
                Some(true) => vec![run(&marks().on, Role::Warning), plain(" waiting")],
                Some(false) => vec![plain("none")],
                None => unknown(),
            },
            when(node.support_ends.as_deref(), clock),
        ]);
    }
    page.extend(table.lines(0));
    page
}
