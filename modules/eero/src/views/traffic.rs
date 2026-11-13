//! `eero traffic`: the bytes the home network and each node moved over the
//! last day, each way's total and spark, then every hour's figures. The
//! record keeps them as eero names them, usage.

use super::clock::{self, Clock};
use super::figures::{bytes, unknown};
use super::graph::spark;
use super::text::{Align, Ink, Line, Page, Table, muted, plain, strong};
use super::{counted, head, node_name, titled};
use crate::model::{HomeNetwork, Series, Usage};

#[derive(serde::Serialize)]
struct NodeTraffic<'a> {
    id: &'a str,
    location: &'a Option<String>,
    usage: &'a Option<Usage>,
}

#[derive(serde::Serialize)]
struct Figures<'a> {
    /// The home network's own, as the internet carries it.
    usage: &'a Option<Usage>,
    nodes: Vec<NodeTraffic<'a>>,
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let nodes = model
        .nodes
        .iter()
        .map(|node| NodeTraffic {
            id: &node.id,
            location: &node.location,
            usage: &node.usage,
        })
        .collect();
    super::json::slice(
        model,
        "eero.traffic/1",
        Figures {
            usage: &model.internet.usage,
            nodes,
        },
    )
}

pub fn page(model: &HomeNetwork, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    let mut each: Vec<(&str, Option<&Usage>)> =
        vec![(model.name.as_str(), model.internet.usage.as_ref())];
    each.extend(
        model
            .nodes
            .iter()
            .map(|node| (node_name(node), node.usage.as_ref())),
    );
    let mut hours: Vec<&str> = each
        .iter()
        .filter_map(|(_, usage)| *usage)
        .flat_map(|usage| usage.download.hours.iter().chain(&usage.upload.hours))
        .map(|hour| hour.at.as_str())
        .collect();
    hours.sort_by_key(|at| clock::parse(at));
    hours.dedup();
    let title = titled(
        model,
        Some(format!("traffic over {}", counted(hours.len(), "hour"))),
    );
    head(&mut page, model, clock, title);

    let mut totals = Table::new(&[
        Align::Left,
        Align::Point,
        Align::Left,
        Align::Point,
        Align::Left,
    ]);
    totals.header(&["", "down", "", "up", ""]);
    let graph = |series: &Series| {
        #[expect(clippy::cast_precision_loss, reason = "a graph's height")]
        let values: Vec<f64> = series.hours.iter().map(|hour| hour.bytes as f64).collect();
        spark(&values, ink.coloured())
    };
    for (name, usage) in &each {
        let Some(usage) = usage else {
            totals.row(vec![
                vec![strong(*name)],
                unknown(),
                Vec::new(),
                unknown(),
                Vec::new(),
            ]);
            continue;
        };
        totals.row(vec![
            vec![strong(*name)],
            bytes(usage.download.total_bytes),
            graph(&usage.download),
            bytes(usage.upload.total_bytes),
            graph(&usage.upload),
        ]);
    }
    page.extend(totals.lines(0));
    page.blank();

    let mut aligns = vec![Align::Left, Align::Left];
    aligns.extend(std::iter::repeat_n(Align::Point, each.len() * 2));
    let mut table = Table::new(&aligns);
    let headings: Vec<String> = each
        .iter()
        .flat_map(|(name, _)| [format!("{name} down"), "up".to_owned()])
        .collect();
    let mut header = vec!["", "hour"];
    header.extend(headings.iter().map(String::as_str));
    table.header(&header);
    let mut last_day = None;
    for at in hours {
        let when = clock::parse(at);
        let day = when.map(|when| clock.day(when));
        let mut cells: Vec<Line> = vec![
            // The day leads only the first hour of each, so the hours read
            // as runs within it.
            match &day {
                Some(day) if last_day.as_ref() != Some(day) => vec![muted(day)],
                _ => Vec::new(),
            },
            when.map_or_else(|| vec![plain(at)], |when| vec![plain(clock.hour(when))]),
        ];
        last_day = day;
        for (_, usage) in &each {
            for series in [usage.map(|u| &u.download), usage.map(|u| &u.upload)] {
                let hour = series.and_then(|series| series.hours.iter().find(|hour| hour.at == at));
                cells.push(hour.map_or_else(unknown, |hour| bytes(hour.bytes)));
            }
        }
        table.row(cells);
    }
    page.extend(table.lines(0));
    page
}
