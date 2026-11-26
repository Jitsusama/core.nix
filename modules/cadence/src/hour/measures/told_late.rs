//! The dates the asks list was told of too late to warn whoever waits on
//! them: each moved later with a quarter of an hour or less left, as one
//! moved six minutes before its date on 25 November with no word. A date
//! that passes before it moves, or never moves, is a slip, which the slips
//! measure counts, so it isn't counted here too.

use std::collections::BTreeMap;

use jiff::civil::DateTime;
use jiff::tz::Offset;
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

/// Whether an ask owed a word when its date moved, or that it can't be
/// told: parked, with a note naming its park in words not read as a
/// minute, so the keeper's shape has drifted from what's read here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owed {
    Yes,
    No,
    Unread,
}

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
/// written again. A move whose park's minute can't be read leaves the
/// measure unjudged, saying how many, rather than owing none unseen.
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
    let mut late: Vec<Late> = Vec::new();
    let mut unread = 0;
    for (_, versions) in &asks {
        for moved in moved_late(versions, now, which) {
            match moved {
                Some(moved) => late.push(moved),
                None => unread += 1,
            }
        }
    }
    late.sort_by_key(|(when, _, _)| *when);
    let mut says = "dates moved later with 15 minutes or less left, in the last day".to_owned();
    if unread > 0 {
        let them = if unread == 1 { "move's" } else { "moves'" };
        says = format!(
            "{says} (unread: {unread} late {them} ask parked by a note whose minute can't be read)"
        );
    }
    Measure {
        key: which.key(KEY),
        class: Class::Slip,
        says,
        now: count(late.len()),
        target: (unread == 0).then_some(0.0),
        owner: late.first().map(|(_, _, o)| o.clone()).unwrap_or_default(),
        behind: late.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// Each move of an ask's dates told late by `now`, as [`told_late`] reads
/// them, from `versions`, every line the ask was written in, oldest first;
/// none for one whose owing can't be read.
fn moved_late(versions: &[Value], now: i64, which: Tracking) -> Vec<Option<Late>> {
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
            if !told || kept {
                return None;
            }
            match owed_at(versions, moved, which) {
                Owed::No => None,
                Owed::Unread => Some(None),
                Owed::Yes => {
                    let left = moved_left(promise.expected - moved);
                    Some(Some((
                        moved,
                        format!("{words}, {left} · {owner}"),
                        owner.clone(),
                    )))
                }
            }
        })
        .collect()
}

/// Whether the ask of `versions` owed a word when its date was moved at
/// `moved`: of the kind `which`, neither parked nor put down, on the first
/// line that gave the date after the move, or on the latest when none
/// says when it was given. Parked on that line, it still owed one when
/// its note says it was parked in a minute after the move's, and can't be
/// told when its note names the park in a minute that can't be read.
fn owed_at(versions: &[Value], moved: i64, which: Tracking) -> Owed {
    let line = versions.iter().find(|line| given_at(line, moved).is_some());
    let Some(line) = line.or(versions.last()) else {
        return Owed::No;
    };
    if kind_when(line, moved) != Some(which) {
        return Owed::No;
    }
    let Some(ruling) = parked::ruling(line) else {
        return Owed::Yes;
    };
    match parked_since(line, ruling, given_at(line, moved)) {
        Park::At(park) if after(park, moved) => Owed::Yes,
        Park::At(_) | Park::Unsaid => Owed::No,
        Park::Unread => Owed::Unread,
    }
}

/// When an ask was parked, as its note says: in a minute, not said, or
/// said in words whose minute can't be read.
enum Park {
    At(i64),
    Unsaid,
    Unread,
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
/// wrote. The park standing is the last entry naming `ruling`, since an
/// ask let go and parked again keeps its first park's entry too. Unsaid
/// when no entry names it; unread when the last that does opens on no
/// minute, or `given` has no offset to read it in.
fn parked_since(line: &Value, ruling: &str, given: Option<&str>) -> Park {
    let said = format!("parked: {ruling}");
    let Some(note) = line["note"].as_str() else {
        return Park::Unsaid;
    };
    let named = note.rsplit(" | ").map(str::trim).find(|entry| {
        entry
            .split_once(": ")
            .is_some_and(|(_, words)| words.starts_with(&said))
    });
    let Some(entry) = named else {
        return Park::Unsaid;
    };
    let minute = entry.get(..16).and_then(|stamp| {
        let minute: DateTime = format!("{}:00", stamp.replacen(' ', "T", 1)).parse().ok()?;
        offset_of(given?)?.to_timestamp(minute).ok()
    });
    minute.map_or(Park::Unread, |minute| Park::At(minute.as_second()))
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
