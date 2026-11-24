//! Every date an ask in the orchestrator's `asks.jsonl` was given, read for
//! the slips measure.
//!
//! The list's keeper writes each date as an entry of the ask's `promised`,
//! oldest first: `expected`, when it was due, and `given`, when it was set,
//! each an RFC 3339 time with its offset. A step kept by its date before
//! the ask moves on, such as an account sent ahead of its fix, gets `met`
//! in the same form on its own entry, as on 18 November's
//! `{"expected":"2026-11-19T21:00:00-05:00","given":"2026-11-19T18:28:31-05:00",
//! "met":"2026-11-19T19:59:00-05:00"}`. An ask written again is read from
//! its latest line, so the keeper carries `met` forward on each rewrite.

use serde_json::Value;

use super::seconds;

/// One date an ask was given: when it was `expected`, when the date after
/// it was given, if one was, and when its owner said it was `met`.
pub struct Promise {
    pub expected: i64,
    pub moved: Option<i64>,
    pub met: Option<i64>,
}

/// Each date `ask` was given: its `promised`, oldest first, then its
/// `expected` where that isn't the last promised, given at a time unknown.
/// A `met` holds only on a date the ask has since moved on from, so its
/// last date is kept by landing alone, and only when it came after its
/// date was given, since a step can't be kept before it was promised.
/// An entry that isn't an object, as twenty bare strings were on 24
/// November, says no date, so it's passed over rather than read as one
/// that hides the move after the date before it.
pub fn promised(ask: &Value) -> Vec<Promise> {
    let mut given: Vec<(Option<i64>, Option<i64>, Option<i64>)> = ask["promised"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p.is_object())
        .map(|p| {
            (
                seconds(&p["expected"]),
                seconds(&p["given"]),
                seconds(&p["met"]),
            )
        })
        .collect();
    let current = seconds(&ask["expected"]);
    if current.is_some() && given.last().map(|(date, _, _)| *date) != Some(current) {
        given.push((current, None, None));
    }
    given
        .iter()
        .enumerate()
        .filter_map(|(i, &(date, set, met))| {
            let moved = given.get(i + 1).and_then(|(_, moved, _)| *moved);
            Some(Promise {
                expected: date?,
                moved,
                met: met.filter(|&met| moved.is_some() && set.is_none_or(|set| met >= set)),
            })
        })
        .collect()
}
