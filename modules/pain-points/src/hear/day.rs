//! The keeper's daily count. Once a day ends where Joel is, the ledgers'
//! keeper hears how many pains he said in it, how many of them asked how
//! something stands, how many went to their owners and how many turns
//! stalled him: one line a day, in place of a note for every pain heard
//! too late for its owner, which the keeper already knew of.
//!
//! Only the day just ended is counted. A run reads at most a day back, so
//! after the listener was off longer, the days it missed weren't heard
//! whole, and a count of them would read as quiet days that never were.

use super::heard::Heard;
use super::stalled;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::Path;

/// The kind sore.rs gives a status ask.
pub const ASKED: &str = "asked how it stands";

/// How a day stood. Each pain he said is in one of routed, carried and
/// unsent.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub heard: usize,
    pub asked: usize,
    pub routed: usize,
    pub carried: usize,
    pub unsent: usize,
    pub stalled: usize,
}

/// The day being counted, kept in day.json between runs: its name, when
/// the listener began hearing it, and how many times its count was sent
/// without the node taking it.
#[derive(Serialize, Deserialize)]
struct Counting {
    day: String,
    from: i64,
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
         carried and {} sent to no one; {} turn{} stalled him.",
        from.map_or(String::new(), |from| format!(", from {from}")),
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

/// Sends the keeper the count of yesterday once, when the run before was
/// in it. A first run sends nothing, and counts its day from a day back,
/// as far as it reads. A count the node didn't take is sent again next
/// run, at most `SEND_TRIES` times. Days are told apart by his offset now,
/// so a day around a clock change is counted an hour off.
///
/// # Errors
///
/// When day.json can't be written. What it says back is a trouble for the
/// hour: no time zone, an unreadable day.json, or a count given up on.
pub fn count(
    state: &Path,
    kept: &[Heard],
    at: i64,
    offset_s: Option<i64>,
    send: &mut dyn FnMut(&str) -> bool,
) -> Result<Option<String>, String> {
    // A run that took UTC for want of his zone would split his days at
    // the wrong hour, so it waits for one that knows.
    let Some(offset_s) = offset_s else {
        return Ok(Some("his time zone couldn't be read, so no day was counted".into()));
    };
    let path = state.join("day.json");
    let (today, began) = crate::local_day(at, offset_s);
    let today_n = (began + offset_s * 1000).div_euclid(super::DAY_MS);
    // A run reads a day back, so it hears today from its start.
    let heard_back = at - super::DAY_MS;
    let keep = |counting: &Counting| {
        crate::write_whole(
            &path,
            &serde_json::to_string(counting).map_err(|e| e.to_string())?,
        )
    };
    let held = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Counting>(&text)
            .ok()
            .and_then(|c| day_number(&c.day).map(|n| (n, c))),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            keep(&Counting { day: today, from: heard_back, tries: 0 })?;
            return Ok(None);
        }
        Err(_) => None,
    };
    // An unreadable file starts the count again now: a lost day's count
    // costs less than a listener that stops routing over it.
    let Some((held_n, mut held)) = held else {
        keep(&Counting { day: today, from: heard_back, tries: 0 })?;
        return Ok(Some("day.json couldn't be read, so the day's count began again".into()));
    };
    if held.day == today {
        return Ok(None);
    }
    if held_n != today_n - 1 {
        // Days missed weren't heard whole, and a day ahead of today is a
        // clock set back; either way, today is counted from where it's
        // heard: its start, a day back, or now.
        let from = if held_n < today_n { began } else { at };
        keep(&Counting { day: today, from, tries: 0 })?;
        return Ok(None);
    }
    let start = began - super::DAY_MS;
    let from = held.from.max(start);
    let clock = (from > start).then(|| super::route::clock(from, offset_s));
    let text = note(&held.day, clock.as_deref(), &of(kept, from, began));
    if send(&text) {
        keep(&Counting { day: today, from: began, tries: 0 })?;
        return Ok(None);
    }
    held.tries += 1;
    if held.tries < super::SEND_TRIES {
        keep(&held)?;
        return Ok(None);
    }
    keep(&Counting { day: today, from: began, tries: 0 })?;
    Ok(Some(format!(
        "gave up on {}'s count after {} sends",
        held.day,
        super::SEND_TRIES
    )))
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
        for not_a_day in ["2026-02-31", "2026-13-01", "26-11-16", "2026-11-16x", "", "a-b-c"] {
            assert_eq!(day_number(not_a_day), None, "{not_a_day:?} is no day");
        }
    }
}
