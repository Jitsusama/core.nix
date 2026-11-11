//! Every ready row and drawer the state makes, as the daemon serves
//! them: each place's rows taken in its order by the place's own `Kept`
//! from the ready-rows crate, which wraps each as every daemon's are and
//! spaces their order, keeping each row's where it can, so one meeting,
//! mail or document moving tells the launcher one row rather than every
//! row after it (Y2YBTS's ruling on the record's shape, 2026-11-11).

use std::collections::BTreeMap;

use jiff::tz::TimeZone;
use ready_rows::kept::Kept;
use serde_json::{Value, json};

use crate::calendar_rows;
use crate::document_rows;
use crate::rows::{self, Vocabulary};
use crate::state::State;

/// Each place's rows as last told, which keeps their order.
#[derive(Default)]
pub struct Ready {
    places: BTreeMap<String, Kept>,
}

impl Ready {
    /// Every ready row the state makes, as records keyed as rows.rs keys
    /// them, each its row under `row`, in each place's order, then each
    /// drawer, at `now` on Joel's clock, which says which meetings are
    /// over. A place whose words didn't read makes none, and the rest make
    /// theirs.
    pub fn made(&mut self, state: &State, words: &Vocabulary, now: i64) -> Vec<(String, Value)> {
        let (rows, drawers) = made(state, words, now);
        let mut shown: BTreeMap<String, Vec<Value>> = self
            .places
            .keys()
            .map(|place| (place.clone(), Vec::new()))
            .collect();
        for row in rows {
            let place = row["place"].as_str().unwrap_or_default().to_owned();
            shown.entry(place).or_default().push(row);
        }
        let mut made = Vec::new();
        for (place, rows) in shown {
            let kept = self.places.entry(place).or_default();
            kept.take(rows, Vec::new());
            // Kept holds them by key, so they're put back in the order
            // shown, which a list's limit takes the first of.
            let mut told: Vec<&Value> = kept.all().collect();
            told.sort_by_key(|record| record["row"]["order"].as_i64());
            made.extend(told.into_iter().map(|record| {
                let key = record["key"].as_str().unwrap_or_default().to_owned();
                (key, record.clone())
            }));
        }
        let drawers = drawers.into_iter().map(|(key, drawer)| {
            let record = json!({"key": key, "kind": "drawer", "drawer": drawer});
            (key, record)
        });
        made.into_iter().chain(drawers).collect()
    }
}

/// The rows, each place's in the order shown and without an `order`,
/// and the drawers, each by its key.
fn made(state: &State, words: &Vocabulary, now: i64) -> (Vec<Value>, Vec<(String, Value)>) {
    let zone = TimeZone::system();
    let (meetings, held) = (&state.meetings, &state.held);
    let drives = &state.drive.places.drives.files;
    let mut rows = Vec::new();
    let mut drawers = Vec::new();
    if let Some(mail) = &words.mail {
        rows.extend(rows::mail(&state.mail, mail));
        rows.extend(rows::lately(&state.lately, &state.mail, mail));
        drawers.extend(rows::mail_drawers(&state.mail, &state.lately, mail));
    }
    if let Some(documents) = &words.documents {
        rows.extend(document_rows::documents(&state.documents, documents));
        drawers.extend(document_rows::drawers(&state.documents, drives, documents));
    }
    if let Some(calendar) = &words.calendar {
        rows.extend(calendar_rows::meetings(
            meetings, held, now, &zone, calendar,
        ));
        drawers.extend(calendar_rows::drawers(meetings, held, now, &zone, calendar));
    }
    (rows, drawers)
}
