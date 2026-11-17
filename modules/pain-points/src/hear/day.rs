//! The keeper's daily count. Once a day ends where Joel is, the ledgers'
//! keeper hears how many pains he said in it, how many of them asked how
//! something stands, how many went to their owners and how many turns
//! stalled him: one line a day, in place of a note for every pain heard
//! too late for its owner, which the keeper already knew of.
//!
//! Only the day just ended is counted, and only once. A run reads at most
//! a day back, so after the listener was off longer, the days it missed
//! weren't heard whole, and a count of them would read as quiet days that
//! never were. A day it heard only part of says from when.

use super::heard::Heard;
use super::stalled;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::Path;

/// The kind sore.rs gives a status ask.
pub const ASKED: &str = "asked how it stands";

/// How long the listener may have been away from a day's start and still
/// trust the start day.json held for it: a clock change moves it an hour.
const START_DRIFT_MS: i64 = 3 * 3_600_000;

/// How a day stood. Each pain he said is in one of routed, carried, given
/// up and unsent.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub heard: usize,
    pub asked: usize,
    pub routed: usize,
    pub carried: usize,
    pub given_up: usize,
    pub unsent: usize,
    pub stalled: usize,
}

/// The day being counted, kept in day.json between runs: its name, when
/// it began where he was, when the listener began hearing it, when it
/// last ran, and how many times its count was sent without the node
/// taking it.
#[derive(Serialize, Deserialize)]
struct Counting {
    day: String,
    #[serde(default)]
    began: i64,
    from: i64,
    #[serde(default)]
    last: i64,
    #[serde(default)]
    tries: u32,
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
        // A pain sent to its owner that the keeper then carried was sent,
        // so it counts there alone.
        if h.routed_at.is_some() {
            day.routed += 1;
        } else if h.carried {
            day.carried += 1;
        } else if h.tries >= super::SEND_TRIES {
            day.given_up += 1;
        } else {
            day.unsent += 1;
        }
    }
    day
}

/// The keeper's line for `name`, a day as YYYY-MM-DD, heard `from` a time
/// of his when the listener began partway through it.
pub fn note(name: &str, from: Option<&str>, day: &Day) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "Joel's day, {name}{}, as pain-points hear counted it: {} pain{} heard, {} asking how \
         something stands (aimed at none); {} sent to their owners, {} the ledgers already \
         carried, {} given up after failed sends and {} sent to no one; {} turn{} stalled him.",
        from.map_or(String::new(), |from| format!(", from {from}")),
        day.heard,
        s(day.heard),
        day.asked,
        day.routed,
        day.carried,
        day.given_up,
        day.unsent,
        day.stalled,
        s(day.stalled),
    )
}

/// Sends the keeper the count of yesterday once, two hours into today, so
/// pains still on their way are routed and turns still running have ended.
/// A count the node didn't take is sent again next run, at most
/// `SEND_TRIES` times; a send that timed out after the node took it can't
/// be told from one it never took, so the keeper may hear a day twice. A
/// day already counted is never counted again: a clock or zone stepped
/// back waits until the day after it.
///
/// # Errors
///
/// When day.json can't be written. What it says back is a trouble for the
/// hour: an unreadable day.json, or a count given up on.
pub fn count(
    state: &Path,
    kept: &[Heard],
    at: i64,
    offset_s: i64,
    send: &mut dyn FnMut(&str) -> bool,
) -> Result<Option<String>, String> {
    let path = state.join("day.json");
    let (today, began) = crate::local_day(at, offset_s);
    let today_n = (began + offset_s * 1000).div_euclid(super::DAY_MS);
    let afresh = Counting {
        day: today.clone(),
        began,
        from: began,
        last: at,
        tries: 0,
    };
    let keep = |counting: &Counting| {
        crate::write_whole(
            &path,
            &serde_json::to_string(counting).map_err(|e| e.to_string())?,
        )
    };
    let held = match std::fs::read_to_string(&path) {
        Ok(text) => held(&text, offset_s),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            keep(&afresh)?;
            return Ok(None);
        }
        Err(_) => None,
    };
    // An unreadable file starts the count again now: a lost day's count
    // costs less than a listener that stops routing over it.
    let Some((held_n, mut held)) = held else {
        keep(&afresh)?;
        return Ok(Some(
            "day.json couldn't be read, so the day's count began again".into(),
        ));
    };
    // A run reads a day back, so after a longer gap the day is heard only
    // from a day back.
    if held.last < at - super::DAY_MS {
        held.from = held.from.max(at - super::DAY_MS);
    }
    held.last = at;
    if held_n >= today_n || (held_n == today_n - 1 && at < began + super::FRESH_MS) {
        keep(&held)?;
        return Ok(None);
    }
    if held_n < today_n - 1 {
        // Days missed weren't heard whole, so today is counted from its
        // start, and a count still being tried is said as given up.
        keep(&afresh)?;
        return Ok((held.tries > 0).then(|| given_up(&held)));
    }
    let clock = (held.from > held.began).then(|| super::route::clock(held.from, offset_s));
    let text = note(&held.day, clock.as_deref(), &of(kept, held.from, began));
    if send(&text) {
        keep(&afresh)?;
        return Ok(None);
    }
    held.tries += 1;
    if held.tries < super::SEND_TRIES {
        keep(&held)?;
        return Ok(None);
    }
    keep(&afresh)?;
    Ok(Some(given_up(&held)))
}

/// What day.json holds, numbered, with the day's start set right: one
/// kept from another zone, or never kept, is taken as its start where he
/// is now. Days that aren't days are none, as though unreadable.
fn held(text: &str, offset_s: i64) -> Option<(i64, Counting)> {
    let mut held = serde_json::from_str::<Counting>(text).ok()?;
    let n = day_number(&held.day)?;
    let start = n * super::DAY_MS - offset_s * 1000;
    if (held.began - start).abs() > START_DRIFT_MS {
        held.began = start;
    }
    held.from = held
        .from
        .clamp(held.began, held.began + super::DAY_MS + super::FRESH_MS);
    Some((n, held))
}

fn given_up(held: &Counting) -> String {
    let s = if held.tries == 1 { "" } else { "s" };
    format!(
        "gave up on {}'s count after {} send{s}",
        held.day, held.tries
    )
}

/// Days since 1970 of a YYYY-MM-DD name, by Howard Hinnant's algorithm,
/// or none when it isn't one, so a day.json naming something else is
/// read as unreadable and never said to the keeper.
fn day_number(name: &str) -> Option<i64> {
    let mut parts = name.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || (y.len(), m.len(), d.len()) != (4, 2, 2) {
        return None;
    }
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !(all_digits(y) && all_digits(m) && all_digits(d)) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let n = era * 146_097 + doe - 719_468;
    // A name like 2026-02-31 parses, but names another day.
    (crate::local_day(n * super::DAY_MS, 0).0 == name).then_some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_numbered_from_its_name_alone() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2026-11-16"), Some(20_773));
        assert_eq!(day_number("2024-02-29"), Some(19_782));
        for not_a_day in [
            "2026-02-31",
            "2026-13-01",
            "26-11-16",
            "2026-11-16x",
            "",
            "a-b-c",
        ] {
            assert_eq!(day_number(not_a_day), None, "{not_a_day:?} is no day");
        }
    }
}
