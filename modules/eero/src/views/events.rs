//! `eero events`: the app's activity list, newest first, each event's time
//! in the machine's zone, its kind and eero's own words for it.

use super::clock::{self, Clock};
use super::text::{Align, Page, Table, muted, plain};
use super::{counted, head, titled};
use crate::model::{Event, HomeNetwork};

#[derive(serde::Serialize)]
struct Figures<'a> {
    events: &'a [Event],
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        Figures {
            events: &model.events,
        },
    )
}

pub fn page(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(model.events.len(), "event"))),
    );
    let mut table = Table::new(&[Align::Left, Align::Left, Align::Left, Align::Left]);
    table.header(&["", "at", "kind", "what happened"]);
    let mut last_day = None;
    for event in &model.events {
        let at = clock::parse(&event.at);
        let day = at.map(|at| clock.day(at));
        table.row(vec![
            // The day leads only its first event, so a day's events read as
            // one run.
            match &day {
                Some(day) if last_day.as_ref() != Some(day) => vec![muted(day)],
                _ => Vec::new(),
            },
            at.map_or_else(
                || vec![plain(&event.at)],
                |at| vec![plain(clock.second(at))],
            ),
            vec![plain(&event.kind)],
            vec![plain(&event.text)],
        ]);
        last_day = day;
    }
    page.extend(table.lines(0));
    page
}
