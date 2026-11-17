//! The keeper's daily count. Once a day ends where Joel is, the ledgers'
//! keeper hears how many pains he said in it, how many of them asked how
//! something stands, how many went to their owners and how many turns
//! stalled him: one line a day, in place of a note for every pain heard
//! too late for its owner, which the keeper already knew of.

use super::heard::Heard;
use super::stalled;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The kind sore.rs gives a status ask.
pub const ASKED: &str = "asked how it stands";

/// Far enough past a day's start to land in the next day, whether the
/// clocks went forward or back between them.
const PAST_A_DAY_MS: i64 = super::DAY_MS + 2 * 3_600_000;

/// How a day stood.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub heard: usize,
    pub asked: usize,
    pub routed: usize,
    pub carried: usize,
    pub unsent: usize,
    pub stalled: usize,
}

/// The day being counted, kept in day.json between runs.
#[derive(Serialize, Deserialize)]
struct Counting {
    day: String,
    began: i64,
}

/// The status asks he said since `since`.
pub fn asked_since(kept: &[Heard], since: i64) -> usize {
    kept.iter()
        .filter(|h| h.at >= since && !stalled::is(h) && h.kinds.iter().any(|k| k == ASKED))
        .count()
}

/// How the time from `began` until `ended` stood.
pub fn of(kept: &[Heard], began: i64, ended: i64) -> Day {
    let mut day = Day::default();
    for h in kept.iter().filter(|h| h.at >= began && h.at < ended) {
        if stalled::is(h) {
            day.stalled += 1;
            continue;
        }
        day.heard += 1;
        day.asked += usize::from(h.kinds.iter().any(|k| k == ASKED));
        day.routed += usize::from(h.routed_at.is_some());
        day.carried += usize::from(h.carried);
        day.unsent += usize::from(h.routed_at.is_none() && !h.carried);
    }
    day
}

/// The keeper's line for `name`, a day as YYYY-MM-DD.
pub fn note(name: &str, day: &Day) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "Joel's day, {name}, as pain-points hear counted it: {} pain{} heard, {} asking how \
         something stands (aimed at none); {} sent to their owners, {} the ledgers already \
         carried and {} heard too late for anyone; {} turn{} stalled him.",
        day.heard,
        s(day.heard),
        day.asked,
        day.routed,
        day.carried,
        day.unsent,
        day.stalled,
        s(day.stalled),
    )
}

/// Sends the keeper the count of each day that ended since the last run,
/// once, oldest first. A first run starts counting the day it runs in and
/// sends nothing, since it can't know what came before it was heard. A
/// count that didn't go is sent again next run, and the days after it
/// wait behind it.
///
/// # Errors
///
/// When day.json can't be written.
pub fn count(
    state: &Path,
    kept: &[Heard],
    at: i64,
    offset_s: i64,
    send: &mut dyn FnMut(&str) -> bool,
) -> Result<(), String> {
    let path = state.join("day.json");
    let (today, began) = crate::local_day(at, offset_s);
    // A file that can't be read starts the count again today: a lost day's
    // count costs less than a listener that stops routing over it.
    let held = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Counting>(&text).ok());
    let mut counting = held.unwrap_or(Counting {
        day: today.clone(),
        began,
    });
    while counting.day != today && counting.began < began {
        let (next, ended) = crate::local_day(counting.began + PAST_A_DAY_MS, offset_s);
        if !send(&note(&counting.day, &of(kept, counting.began, ended))) {
            break;
        }
        counting = Counting {
            day: next,
            began: ended,
        };
    }
    crate::write_whole(
        &path,
        &serde_json::to_string(&counting).map_err(|e| e.to_string())?,
    )
}
