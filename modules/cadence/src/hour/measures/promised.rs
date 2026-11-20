//! Every date an ask in the orchestrator's `asks.jsonl` was given, read for
//! the slips measure.

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
pub fn promised(ask: &Value) -> Vec<Promise> {
    let mut given: Vec<(Option<i64>, Option<i64>, Option<i64>)> = ask["promised"]
        .as_array()
        .into_iter()
        .flatten()
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
        .filter_map(|(i, (date, _, met))| {
            Some(Promise {
                expected: (*date)?,
                moved: given.get(i + 1).and_then(|(_, moved, _)| *moved),
                met: *met,
            })
        })
        .collect()
}
