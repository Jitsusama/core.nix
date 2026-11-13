//! `eero events`: what happened on the home network, newest first, as one
//! timeline of eero's activity list and the changes this machine's daemon
//! told as notices, each with its time in the machine's zone, who saw it,
//! its kind and its words, under the day it happened on. The day heads its
//! run, rather than leading a row in a column of its own, so the rest
//! keeps to a hundred columns, and a notice's body has its own line
//! beneath what it summed up for the same reason.

use std::cmp::Reverse;

use jiff::Timestamp;

use super::clock::{self, Clock};
use super::figures::sentence;
use super::text::{Align, Line, Page, Table, muted, plain};
use super::{counted, head, titled};
use crate::model::{Event, HomeNetwork, ToldChange};

#[derive(serde::Serialize)]
struct Figures<'a> {
    events: &'a [Event],
    told: &'a [ToldChange],
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        "eero.events/1",
        Figures {
            events: &model.events,
            told: &model.told,
        },
    )
}

/// One thing in the timeline, eero's or this machine's: its words, and a
/// notice's body beneath them.
struct Happened<'a> {
    stamp: &'a str,
    at: Option<Timestamp>,
    seen_by: &'static str,
    kind: &'a str,
    words: &'a str,
    body: Option<&'a str>,
}

/// eero's events and what was told, newest first: one whose time doesn't
/// read goes last, and two at one time keep eero's first, each list in
/// its own order.
fn timeline(model: &HomeNetwork) -> Vec<Happened<'_>> {
    let eeros = model.events.iter().map(|event| Happened {
        stamp: &event.at,
        at: clock::parse(&event.at),
        seen_by: "eero",
        kind: &event.kind,
        words: &event.text,
        body: None,
    });
    let told = model.told.iter().map(|told| Happened {
        stamp: &told.at,
        at: clock::parse(&told.at),
        seen_by: "this machine",
        kind: told_kind(&told.kind),
        words: &told.summary,
        body: Some(told.body.as_str()).filter(|body| !body.is_empty()),
    });
    let mut all: Vec<Happened> = eeros.chain(told).collect();
    all.sort_by_key(|happened| Reverse(happened.at));
    all
}

/// A told change's kind as eero names its own categories: what it was
/// about.
fn told_kind(kind: &str) -> &str {
    match kind {
        "internet-out" | "internet-back" => "Internet",
        "node-offline" | "node-online" => "Node",
        "joined" => "Device",
        "update-waiting" | "updated" => "Firmware",
        other => other,
    }
}

pub fn page(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    let counts: Vec<String> = [(model.events.len(), "event"), (model.told.len(), "notice")]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, noun)| counted(count, noun))
        .collect();
    let about = if counts.is_empty() {
        "nothing happened".to_owned()
    } else {
        counts.join(&format!(" {} ", super::marks::marks().separator))
    };
    head(&mut page, model, clock, titled(model, Some(about)));
    // The first column is empty, so each row stands two spaces in from the
    // day heading it.
    let mut table = Table::new(&[Align::Left; 5]);
    table.header(&["", "at", "seen by", "kind", "what happened"]);
    let mut last_day = None;
    for (n, happened) in timeline(model).into_iter().enumerate() {
        let day = happened.at.map(|at| clock.day(at));
        if n == 0 || day != last_day {
            if n > 0 {
                table.across(Vec::new());
            }
            table.across(vec![muted(day.as_deref().unwrap_or("when unknown"))]);
        }
        table.row(vec![
            Vec::new(),
            happened.at.map_or_else(
                || vec![plain(happened.stamp)],
                |at| vec![plain(clock.second(at))],
            ),
            vec![plain(happened.seen_by)],
            vec![plain(happened.kind)],
            vec![plain(sentence(happened.words))],
        ]);
        if let Some(body) = happened.body {
            let mut beneath: Vec<Line> = vec![Vec::new(); 4];
            beneath.push(vec![muted(body)]);
            table.row(beneath);
        }
        last_day = day;
    }
    page.extend(table.lines(0));
    page
}
