//! `eero mesh`: the nodes as a tree from the internet down, the gateway
//! first and each node under the one it hangs from, every edge its
//! backhaul: Ethernet, or Wi-Fi over a radio, and how many of five bars.

use super::clock::Clock;
use super::figures::{figure, maybe};
use super::nodes::{backhaul, state};
use super::text::{Align, Line, Page, Table, muted, plain, strong};
use super::{counted, facts, head, node_name, titled};
use crate::model::{Backhaul, BackhaulKind, HomeNetwork, Node, NodeDevices};

/// What the tree is drawn from: each node, how it's joined, and how many
/// devices hang from it.
#[derive(serde::Serialize)]
struct Branch<'a> {
    id: &'a str,
    location: &'a Option<String>,
    gateway: bool,
    state: &'a Option<String>,
    backhaul: &'a Backhaul,
    devices: &'a NodeDevices,
}

#[derive(serde::Serialize)]
struct Figures<'a> {
    nodes: Vec<Branch<'a>>,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let nodes = model
        .nodes
        .iter()
        .map(|node| Branch {
            id: &node.id,
            location: &node.location,
            gateway: node.gateway,
            state: &node.state,
            backhaul: &node.backhaul,
            devices: &node.devices,
        })
        .collect();
    super::json::slice(model, "eero.mesh/1", Figures { nodes })
}

pub fn page(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(model.nodes.len(), "node"))),
    );
    let internet = &model.internet;
    let mut table = Table::new(&[Align::Left, Align::Left, Align::Point, Align::Left]);
    // Each node's facts wrap under themselves, so the tree keeps every edge.
    table.wraps(16);
    // Into a narrow terminal its bars go, then its backhaul, which its
    // place in the tree mostly says; its state stays beside it.
    table.gives_way(vec![vec![2], vec![1]]);
    let way_out: Vec<Line> = [
        internet.isp.clone(),
        internet.asn.map(|asn| format!("AS{asn}")),
    ]
    .into_iter()
    .flatten()
    .chain(internet.public_address.clone())
    .map(|part| vec![plain(part)])
    .collect();
    // Who carries the internet stands where each node says what it is, so
    // the edges' columns hold only edges.
    table.row(vec![
        vec![strong("internet")],
        Vec::new(),
        Vec::new(),
        facts(way_out),
    ]);
    let roots: Vec<&Node> = model.nodes.iter().filter(|node| node.gateway).collect();
    let placed = |node: &Node| {
        node.gateway
            || (node.backhaul.kind == BackhaulKind::WiFi
                && node.backhaul.upstream.as_deref().is_some_and(|up| {
                    model
                        .nodes
                        .iter()
                        .any(|other| node_name(other) == up && other.id != node.id)
                }))
    };
    // A node over Ethernet that isn't the gateway, or whose upstream the
    // record doesn't list, hangs from the gateway, since that's the node
    // its cable or its lost upstream leads back to.
    let strays: Vec<&Node> = model.nodes.iter().filter(|node| !placed(node)).collect();
    let mut seen = Vec::new();
    for (at, root) in roots.iter().enumerate() {
        branch(
            &mut table,
            model,
            root,
            "",
            at + 1 == roots.len(),
            &strays,
            &mut seen,
        );
    }
    // A node the tree never reached, as in a record with no gateway, is
    // still drawn, from the internet.
    for node in &model.nodes {
        branch(&mut table, model, node, "", true, &[], &mut seen);
    }
    page.table(0, table);
    page
}

fn branch(
    table: &mut Table,
    model: &HomeNetwork,
    node: &Node,
    prefix: &str,
    last: bool,
    strays: &[&Node],
    seen: &mut Vec<String>,
) {
    if seen.contains(&node.id) {
        return;
    }
    seen.push(node.id.clone());
    let mut name = vec![muted(format!(
        "{prefix}{}",
        if last { "└─ " } else { "├─ " }
    ))];
    name.push(strong(node_name(node)));
    let mut edge = backhaul(node);
    // The edge says how it reaches the node above it, and the tree already
    // draws which node that is.
    if let Some(at) = edge.iter().position(|run| run.text == " from ") {
        edge.truncate(at);
    }
    let mut about = vec![state(node)];
    if node.gateway {
        about.insert(0, vec![plain("gateway")]);
    }
    about.push(maybe(node.devices.all, |n| {
        vec![plain(counted(n as usize, "device"))]
    }));
    table.row(vec![
        name,
        edge,
        maybe(node.backhaul.bars, |bars| {
            figure(bars.to_string(), "of 5 bars")
        }),
        facts(about),
    ]);
    let mut children: Vec<&Node> = model
        .nodes
        .iter()
        .filter(|child| {
            child.id != node.id
                && child.backhaul.kind == BackhaulKind::WiFi
                && child.backhaul.upstream.as_deref() == Some(node_name(node))
        })
        .collect();
    if node.gateway {
        children.extend(strays.iter().copied().filter(|stray| stray.id != node.id));
    }
    let inner = format!("{prefix}{}", if last { "   " } else { "│  " });
    let count = children.len();
    for (at, child) in children.into_iter().enumerate() {
        branch(table, model, child, &inner, at + 1 == count, &[], seen);
    }
}
