//! `eero radio`: every node's radios side by side, band by band, so the
//! nodes sharing a channel, and the busiest of them, show at a glance.

use super::clock::Clock;
use super::nodes::{band_rank, radio_row, radio_table};
use super::text::{Ink, Page, plain};
use super::{counted, head, node_name, titled};
use crate::model::{HomeNetwork, Radio};

#[derive(serde::Serialize)]
struct NodeRadios<'a> {
    id: &'a str,
    location: &'a Option<String>,
    radios: &'a [Radio],
}

#[derive(serde::Serialize)]
struct Figures<'a> {
    nodes: Vec<NodeRadios<'a>>,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let nodes = model
        .nodes
        .iter()
        .map(|node| NodeRadios {
            id: &node.id,
            location: &node.location,
            radios: &node.radios,
        })
        .collect();
    super::json::slice(model, "eero.radio/1", Figures { nodes })
}

pub fn page(model: &HomeNetwork, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    let all: Vec<(&str, &Radio)> = model
        .nodes
        .iter()
        .flat_map(|node| {
            node.radios
                .iter()
                .map(move |radio| (node_name(node), radio))
        })
        .collect();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(all.len(), "radio"))),
    );
    let mut bands: Vec<&str> = all.iter().map(|(_, radio)| radio.band.as_str()).collect();
    bands.sort_by_key(|band| band_rank(band));
    bands.dedup();
    let mut table = radio_table(true);
    for band in bands {
        // The band leads its first row only, so its nodes read as one group.
        for (at, (node, radio)) in all
            .iter()
            .filter(|(_, radio)| radio.band == band)
            .enumerate()
        {
            let label = if at == 0 {
                vec![plain(band)]
            } else {
                Vec::new()
            };
            radio_row(&mut table, Some((label, node)), radio, ink);
        }
    }
    page.table(0, table);
    page
}
