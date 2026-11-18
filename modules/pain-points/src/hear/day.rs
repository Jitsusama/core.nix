//! The keeper's daily count. Once a day ends where Joel is, the ledgers'
//! keeper hears how many pains he said in it, how many of them asked how
//! something stands, how many went to their owners and how many turns
//! stalled him: one line a day, in place of a note for every pain heard
//! too late for its owner, which the keeper already knew of.
//!
//! Each day the listener heard is counted once, after it ends. A run
//! reads at most a day back, so a day's count says when the listener was
//! off longer than that within it, and days it never ran in aren't
//! counted, since they'd read as quiet days that never were.

use super::heard::Heard;
use super::stalled;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::Path;

/// The kind sore.rs gives a status ask.
pub const ASKED: &str = "asked how it stands";

const HOUR_MS: i64 = 3_600_000;

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
/// it began where he was, when the listener last ran, when within it the
/// listener was off, how many times its count was sent without the node
/// taking it and when it last was, and whether a clock gone back before
/// it was said.
#[derive(Default, Serialize, Deserialize)]
struct Counting {
    day: String,
    #[serde(default)]
    began: i64,
    #[serde(default)]
    last: i64,
    #[serde(default)]
    off: Option<(i64, i64)>,
    #[serde(default)]
    tries: u32,
    #[serde(default)]
    tried: i64,
    #[serde(default)]
    back: bool,
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

/// The keeper's line for `name`, a day as YYYY-MM-DD, saying when within
/// it the listener was `off`, such as "from 23:00 on".
pub fn note(name: &str, off: Option<&str>, day: &Day) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "Joel's day, {name}{}, as pain-points hear counted it: {} pain{} heard, {} asking how \
         something stands (aimed at none); {} sent to their owners, {} the ledgers already \
         carried, {} given up after failed sends and {} sent to no one; {} turn{} stalled him.",
        off.map_or(String::new(), |off| format!(", the listener off {off}")),
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

/// Sends the keeper the count of each day the listener heard, once, two
/// hours after it ends, so pains still on their way are routed and turns
/// still running have ended; or when the listener next runs, after the
/// laptop slept past it. A count the node didn't take is sent again an
/// hour on, at most `SEND_TRIES` times; a send that timed out after the
/// node took it can't be told from one it never took, so the keeper may
/// hear a day twice. A day counted is never counted again: a zone moved
/// west waits for the day after it, and a clock gone back before the day
/// held is said once and waits too.
///
/// What it says back are troubles for the hour: day.json unreadable,
/// unwritable or far ahead of the clock, a clock gone back, or a count
/// given up on. None of them stops a run, since a lost day's count costs
/// less than a listener that stops routing over it.
pub fn count(
    state: &Path,
    kept: &[Heard],
    at: i64,
    offset_s: i64,
    send: &mut dyn FnMut(&str) -> bool,
) -> Vec<String> {
    let path = state.join("day.json");
    let (keep, said) = step(std::fs::read_to_string(&path), kept, at, offset_s, send);
    let written = serde_json::to_string(&keep)
        .map_err(|e| e.to_string())
        .and_then(|text| crate::write_whole(&path, &text));
    let mut troubles: Vec<String> = said.into_iter().collect();
    if let Err(e) = written {
        troubles.push(format!(
            "day.json couldn't be written ({e}), so a day may go uncounted or be counted twice"
        ));
    }
    troubles
}

/// From what day.json held, what it keeps now, and the trouble to say.
fn step(
    read: std::io::Result<String>,
    kept: &[Heard],
    at: i64,
    offset_s: i64,
    send: &mut dyn FnMut(&str) -> bool,
) -> (Counting, Option<String>) {
    let (today, began) = crate::local_day(at, offset_s);
    let today_n = (began + offset_s * 1000).div_euclid(super::DAY_MS);
    let afresh = Counting {
        day: today,
        began,
        last: at,
        ..Counting::default()
    };
    let held = match read {
        Ok(text) => held(&text, offset_s),
        Err(e) if e.kind() == ErrorKind::NotFound => return (afresh, None),
        Err(_) => None,
    };
    let Some((held_n, mut held)) = held else {
        let said = "day.json couldn't be read, so the day's count began again";
        return (afresh, Some(said.into()));
    };
    if held.began > at + super::DAY_MS || held_n > today_n + 1 {
        // No zone puts a day this far ahead: a clock that was, or a file
        // set by hand.
        let said = format!(
            "day.json named {}, ahead of the clock, so the day's count began again",
            held.day
        );
        return (afresh, Some(said));
    }
    if at < held.began {
        let said = (!held.back).then(|| {
            format!(
                "the clock went back before {} began, so that day waits to be counted",
                held.day
            )
        });
        held.back = true;
        return (held, said);
    }
    let end = if held_n + 1 == today_n {
        began
    } else {
        held.began + super::DAY_MS
    };
    // A run reads a day back, so what was said longer than that before it
    // and after the run before went unheard.
    if held.last < at - super::DAY_MS {
        let gap = (held.last.max(held.began), (at - super::DAY_MS).min(end));
        if gap.0 < gap.1 {
            held.off = Some(held.off.map_or(gap, |(s, e)| (s.min(gap.0), e.max(gap.1))));
        }
    }
    held.last = held.last.clamp(at, at + super::DAY_MS);
    let waiting = held_n >= today_n
        || (held_n + 1 == today_n && at < began + super::FRESH_MS)
        || (held.tries > 0 && held.tried <= at && held.tried > at - HOUR_MS);
    if waiting {
        return (held, None);
    }
    let off = held.off.map(|(s, e)| {
        let clock = |t| super::route::clock(t, offset_s);
        if e >= end {
            format!("from {} on", clock(s))
        } else {
            format!("from {} to {}", clock(s), clock(e))
        }
    });
    let text = note(&held.day, off.as_deref(), &of(kept, held.began, end));
    if send(&text) {
        return (afresh, None);
    }
    held.tries += 1;
    held.tried = at;
    if held.tries < super::SEND_TRIES {
        return (held, None);
    }
    (afresh, Some(given_up(&held)))
}

/// What day.json holds, numbered, with the day's start set right when no
/// zone could have given it: every zone's midnight falls from fourteen
/// hours before UTC's to twelve after. Days that aren't days are none, as
/// though unreadable.
fn held(text: &str, offset_s: i64) -> Option<(i64, Counting)> {
    let mut held = serde_json::from_str::<Counting>(text).ok()?;
    let n = day_number(&held.day)?;
    let midnight = n * super::DAY_MS;
    if !(midnight - 14 * HOUR_MS..=midnight + 12 * HOUR_MS).contains(&held.began) {
        held.began = midnight - offset_s * 1000;
    }
    held.off = held.off.filter(|(s, e)| s < e);
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
