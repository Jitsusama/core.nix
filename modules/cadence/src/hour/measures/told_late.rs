//! The dates the asks list was told of too late to warn whoever waits on
//! them: each moved with a quarter of an hour or less left, or after it
//! passed, as one moved six minutes before its date on 25 November with
//! no word, and each still open a quarter of an hour or less from its
//! date with no move said.

use std::collections::BTreeMap;

use serde_json::Value;

use super::promised::promised;
use super::{Class, DAY, Measure, count, first_owner, lines, parked, seconds, short};
use crate::tracking::{self, Tracking};

/// The key of Joel's dates told late; the list's own add their kind to it.
pub const KEY: &str = "dates-told-late";

/// How long before its date a move still warns whoever waits on it.
const LEFT: i64 = 15 * 60;

/// A date told late: when, the line naming it and its owner.
type Late = (i64, String, String);

/// The dates of asks of the kind `which` in the asks list `text` told
/// late by `now`: each move said in the day before `now` with `LEFT` or
/// less to go before the date it moved, or after it, and each date still
/// open within `LEFT` of `now` with no move said. A move is when the
/// date after it was `given`, from the ask's `promised`. A date its ask
/// landed by, or its owner said was `met`, before the move needs none, so
/// it isn't counted; nor is any date of an ask [`parked`] on a ruling,
/// which owes none while it stands, or of one put down. Each ask is read
/// from its latest line, which carries every date it was given, so a
/// move is counted once however often the ask is written again.
#[must_use]
pub fn told_late(text: &str, now: i64, which: Tracking) -> Measure {
    let mut latest: BTreeMap<[String; 2], (usize, Value)> = BTreeMap::new();
    for (order, ask) in lines(text).enumerate() {
        let asked = [ask["at"].to_string(), ask["words"].to_string()];
        let first = latest.get(&asked).map_or(order, |(first, _)| *first);
        latest.insert(asked, (first, ask));
    }
    let mut asks: Vec<(usize, Value)> = latest.into_values().collect();
    asks.sort_by_key(|(order, _)| *order);
    let mut late: Vec<Late> = asks
        .iter()
        .filter(|(_, ask)| tracking::of(ask) == Some(which) && parked::ruling(ask).is_none())
        .flat_map(|(_, ask)| late_in(ask, now))
        .collect();
    late.sort_by_key(|(when, _, _)| *when);
    Measure {
        key: which.key(KEY),
        class: Class::Slip,
        says: "dates moved with 15 minutes or less left, or after they passed, in the last day, \
               and open ones 15 minutes or less away with no move"
            .to_owned(),
        now: count(late.len()),
        target: Some(0.0),
        owner: late.first().map(|(_, _, o)| o.clone()).unwrap_or_default(),
        behind: late.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// Each date of `ask` told late by `now`, as [`told_late`] reads them.
fn late_in(ask: &Value, now: i64) -> Vec<Late> {
    let words = short(ask["words"].as_str().unwrap_or_default());
    let owner = first_owner(ask["owner"].as_str().unwrap_or_default());
    let landed = seconds(&ask["landed"]);
    let given = promised(ask);
    let mut late: Vec<Late> = given
        .iter()
        .filter_map(|promise| {
            let moved = promise.moved?;
            let kept = landed.is_some_and(|landed| landed <= moved)
                || promise.met.is_some_and(|met| met <= moved);
            let told = moved <= now && moved > now - DAY && moved > promise.expected - LEFT;
            (told && !kept).then(|| {
                let left = moved_left(promise.expected - moved);
                (moved, format!("{words}, {left} · {owner}"), owner.clone())
            })
        })
        .collect();
    let due = given.last().map(|promise| promise.expected);
    if let Some(due) = due.filter(|&due| landed.is_none() && due > now && due <= now + LEFT) {
        let line = format!(
            "{words}, due in {} min with no move · {owner}",
            (due - now) / 60
        );
        late.push((due, line, owner));
    }
    late
}

/// How a move `left` seconds before its date reads on the card.
fn moved_left(left: i64) -> String {
    if left >= 60 {
        format!("moved {} min before its date", left / 60)
    } else if left > 0 {
        "moved in its last minute".to_owned()
    } else {
        "moved after its date passed".to_owned()
    }
}

#[cfg(test)]
mod tests;
