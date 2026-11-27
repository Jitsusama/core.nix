//! The dates the asks list was told of too late to warn whoever waits on
//! them: each moved later with a quarter of an hour or less left, as one
//! moved six minutes before its date on 25 November with no word. A date
//! that passes before it moves, or never moves, is a slip, which the slips
//! measure counts, so it isn't counted here too.

use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::Offset;
use serde_json::Value;

use super::promised::promised;
use super::{Class, DAY, Measure, asks, count, first_owner, parked, seconds, short};
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
/// came too late all the same: parked in place on its one line, as the
/// keeper parks, it's parked after the move when its note says so, and
/// put down in place, when its `closed` names a later minute. Each
/// ask's dates are read from its latest line, which carries every date
/// it was given, so a move is counted once however often the ask is
/// written again.
#[must_use]
pub fn told_late(text: &str, now: i64, which: Tracking) -> Measure {
    let mut written: BTreeMap<[String; 2], (usize, Vec<Value>)> = BTreeMap::new();
    for (order, ask) in asks(text).enumerate() {
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
/// says when it was given. Parked on that line, it still owed one when
/// its note says it was parked in a minute after the move's.
fn owed_at(versions: &[Value], moved: i64, which: Tracking) -> bool {
    let line = versions.iter().find(|line| given_at(line, moved).is_some());
    line.or(versions.last()).is_some_and(|line| {
        let parked_after = || {
            given_at(line, moved)
                .and_then(|given| parked_since(line, given))
                .is_some_and(|park| after(park, moved))
        };
        kind_when(line, moved) == Some(which) && (parked::ruling(line).is_none() || parked_after())
    })
}

/// The kind the ask on `line` was of when its date moved at `moved`: put
/// down in place in a minute after the move, as its `closed` says, it was
/// still of its kind then. Put down by words alone, no time says when.
fn kind_when(line: &Value, moved: i64) -> Option<Tracking> {
    if seconds(&line["closed"]).is_some_and(|closed| after(closed, moved)) {
        let mut open = line.clone();
        open["closed"] = Value::Null;
        return tracking::of(&open);
    }
    tracking::of(line)
}

/// Whether `then` fell in a minute after the one `moved` did, since the
/// keeper's notes give only the minute.
fn after(then: i64, moved: i64) -> bool {
    then.div_euclid(60) > moved.div_euclid(60)
}

/// The time, as written, that `line`'s `promised` says a date was given
/// at `moved`, if it says so.
fn given_at(line: &Value, moved: i64) -> Option<&str> {
    let promises = line["promised"].as_array()?;
    let promise = promises
        .iter()
        .find(|promise| seconds(&promise["given"]) == Some(moved))?;
    promise["given"].as_str()
}

/// When the ask on `line` was parked on the ruling it names, by the first
/// entry of its note saying so. The keeper's note only grows, entries
/// joined by " | ", each opening with the local minute of the pass that
/// wrote it and no zone, as "2026-11-24 19:26 J7XHQN: parked: b95ea57b",
/// so the minute is read in the offset of `given`, a time the same keeper
/// wrote. The park standing is the last entry naming its ruling, since an
/// ask let go and parked again keeps its first park's entry too. None when
/// no entry says so in that shape.
fn parked_since(line: &Value, given: &str) -> Option<i64> {
    let ruling = parked::ruling(line)?;
    let offset = offset_of(given)?;
    let said = format!("parked: {ruling}");
    line["note"].as_str()?.rsplit(" | ").find_map(|entry| {
        let (stamp, rest) = entry.trim().split_at_checked(16)?;
        let (_, words) = rest.split_once(": ")?;
        if !words.starts_with(&said) {
            return None;
        }
        let minute: DateTime = format!("{}:00", stamp.replacen(' ', "T", 1)).parse().ok()?;
        offset.to_timestamp(minute).ok().map(Timestamp::as_second)
    })
}

/// The offset a time written as `given` ends with, "Z" or "-05:00".
fn offset_of(given: &str) -> Option<Offset> {
    if given.ends_with('Z') {
        return Some(Offset::UTC);
    }
    let tail = given.get(given.len().checked_sub(6)?..)?;
    let (sign, clock) = tail.split_at_checked(1)?;
    let (hours, minutes) = clock.split_once(':')?;
    let seconds = hours.parse::<i32>().ok()? * 3600 + minutes.parse::<i32>().ok()? * 60;
    let seconds = match sign {
        "+" => seconds,
        "-" => -seconds,
        _ => return None,
    };
    Offset::from_seconds(seconds).ok()
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
