//! The dates the asks list was told of too late to warn whoever waits on
//! them: each moved later with a quarter of an hour or less left, as one
//! moved six minutes before its date on 25 November with no word. A date
//! that passes before it moves, or never moves, is a slip, which the slips
//! measure counts, so it isn't counted here too.

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
/// less to go before the date it moved. A move is when the date after it
/// was `given`, from the ask's `promised`, to a later date, since one given
/// again or brought forward keeps nobody waiting longer. A date its ask
/// landed by, or its owner said was `met`, before the move needs none, so
/// it isn't counted; nor is a move given while its ask was [`parked`] on a
/// ruling, which owes none while it stands, or put down. An ask parked or
/// put down after its move was told late keeps the move, since the word
/// came too late all the same. Each ask's dates are read from its latest
/// line, which carries every date it was given, so a move is counted
/// once however often the ask is written again.
#[must_use]
pub fn told_late(text: &str, now: i64, which: Tracking) -> Measure {
    let mut written: BTreeMap<[String; 2], (usize, Vec<Value>)> = BTreeMap::new();
    for (order, ask) in lines(text).enumerate() {
        let asked = [ask["at"].to_string(), ask["words"].to_string()];
        written
            .entry(asked)
            .or_insert_with(|| (order, Vec::new()))
            .1
            .push(ask);
    }
    let mut asks: Vec<(usize, Vec<Value>)> = written.into_values().collect();
    asks.sort_by_key(|(order, _)| *order);
    let mut late: Vec<Late> = asks
        .iter()
        .flat_map(|(_, versions)| moved_late(versions, now, which))
        .collect();
    late.sort_by_key(|(when, _, _)| *when);
    Measure {
        key: which.key(KEY),
        class: Class::Slip,
        says: "dates moved later with 15 minutes or less left, in the last day".to_owned(),
        now: count(late.len()),
        target: Some(0.0),
        owner: late.first().map(|(_, _, o)| o.clone()).unwrap_or_default(),
        behind: late.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// Each move of an ask's dates told late by `now`, as [`told_late`] reads
/// them, from `versions`, every line the ask was written in, oldest first.
fn moved_late(versions: &[Value], now: i64, which: Tracking) -> Vec<Late> {
    let Some(ask) = versions.last() else {
        return Vec::new();
    };
    let words = short(ask["words"].as_str().unwrap_or_default());
    let owner = first_owner(ask["owner"].as_str().unwrap_or_default());
    let landed = seconds(&ask["landed"]);
    promised(ask)
        .iter()
        .filter_map(|promise| {
            let (moved, due_after) = (promise.moved?, promise.to?);
            let kept = landed.is_some_and(|landed| landed <= moved)
                || promise.met.is_some_and(|met| met <= moved);
            let near = (promise.expected - LEFT..promise.expected).contains(&moved);
            let told = moved <= now && moved > now - DAY && near && due_after > promise.expected;
            let owed = owed_at(versions, moved, which);
            (told && !kept && owed).then(|| {
                let left = moved_left(promise.expected - moved);
                (moved, format!("{words}, {left} · {owner}"), owner.clone())
            })
        })
        .collect()
}

/// Whether the ask of `versions` owed a word when its date was moved at
/// `moved`: of the kind `which`, neither parked nor put down, on the first
/// line that gave the date after the move, or on the latest when none
/// says when it was given.
fn owed_at(versions: &[Value], moved: i64, which: Tracking) -> bool {
    let gave = |line: &&Value| {
        line["promised"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|promise| seconds(&promise["given"]) == Some(moved))
    };
    versions
        .iter()
        .find(gave)
        .or(versions.last())
        .is_some_and(|line| tracking::of(line) == Some(which) && parked::ruling(line).is_none())
}

/// How a move `left` seconds before its date reads on the card.
fn moved_left(left: i64) -> String {
    if left >= 60 {
        format!("moved {} min before its date", left / 60)
    } else {
        "moved in its last minute".to_owned()
    }
}

#[cfg(test)]
mod tests;
