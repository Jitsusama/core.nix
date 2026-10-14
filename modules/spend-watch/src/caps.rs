//! Caps: a provider's limit on spend over a rolling window, as feeds give
//! them (or one set for a machine with no feed), with their history of
//! changes and a clock in chances: from the forecast's simulated paths,
//! how likely the window's spend is to reach the warning and the cap
//! within a day and before an override ends, and, while over, when the
//! oldest hours rolling out of the window bring it back under.

use crate::forecast::{Paths, Shape};
use crate::history::History;
use crate::time::{HOUR, Zone};

#[derive(Debug, Clone, PartialEq)]
pub struct Cap {
    pub feed: String,
    pub name: String,
    pub label: String,
    pub allowance: Option<f64>,
    pub window_hours: u32,
    pub spent: f64,
    pub warn: f64,
    pub fallback: Option<f64>,
    pub state: String,
    pub ends: Option<i64>,
    pub ends_known: bool,
    pub at: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub feed: String,
    pub name: String,
    pub at: f64,
    pub kind: String,
    pub before: Option<f64>,
    pub after: Option<f64>,
    pub spent: f64,
}

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

/// Each cap as its feed last gave it.
pub fn latest(history: &History) -> Result<Vec<Cap>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT feed, name, label, allowance, window_hours, spent, warn, fallback, state,
             ends, ends_known, at FROM caps c WHERE at = (SELECT MAX(at) FROM caps d
             WHERE d.feed = c.feed AND d.name = c.name) ORDER BY feed, name",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Cap {
                feed: row.get(0)?,
                name: row.get(1)?,
                label: row.get(2)?,
                allowance: row.get(3)?,
                window_hours: row.get(4)?,
                spent: row.get(5)?,
                warn: row.get(6)?,
                fallback: row.get(7)?,
                state: row.get(8)?,
                ends: row
                    .get::<_, Option<String>>(9)?
                    .and_then(|day| crate::time::parse_day(&day)),
                ends_known: row.get(10)?,
                at: row.get(11)?,
            })
        })
        .map_err(fail)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(fail)
}

pub fn events(history: &History, since: f64) -> Result<Vec<Event>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT feed, name, at, kind, before, after, spent FROM cap_events WHERE at >= ?1
             ORDER BY at, rowid",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([since], |row| {
            Ok(Event {
                feed: row.get(0)?,
                name: row.get(1)?,
                at: row.get(2)?,
                kind: row.get(3)?,
                before: row.get(4)?,
                after: row.get(5)?,
                spent: row.get(6)?,
            })
        })
        .map_err(fail)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(fail)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clock {
    pub warn_within_day: f64,
    pub cap_within_day: f64,
    /// The chance of reaching it before an override ends, when one does.
    pub cap_before_end: Option<f64>,
    /// The chance the spend would be over the allowance that follows the
    /// override, the moment it ends.
    pub over_after_end: Option<f64>,
    /// While over: when it's back under with nothing more spent.
    pub frees_at: Option<f64>,
    /// What rolls out of the window in the next day.
    pub freeing_in_day: f64,
}

/// Spend expected in each coming hour, for one path of daily totals: each
/// day's total spread by its weekday's hour shape.
fn hourly(path: &[f64], shape: &Shape, zone: &Zone, now: f64, hours: usize) -> Vec<f64> {
    let today = zone.day(now);
    (0..hours)
        .map(|k| {
            let at = now + k as f64 * HOUR;
            let day = zone.day(at);
            let index = (day - today) as usize;
            let total = path.get(index).copied().unwrap_or(0.0);
            total * shape.shares[Zone::weekday(day)][zone.local_hour(at) as usize]
        })
        .collect()
}

/// `past` is the window's hours oldest first, ending now, so `past[k]` is
/// what rolls out k hours from now.
#[allow(clippy::too_many_arguments)]
pub fn clock(
    spent: f64,
    allowance: f64,
    warn: f64,
    fallback: Option<f64>,
    ends_at: Option<f64>,
    past: &[f64],
    paths: &Paths,
    shape: &Shape,
    zone: &Zone,
    now: f64,
) -> Clock {
    let horizon = 24 * paths.first().map_or(1, Vec::len);
    let end_hour = ends_at.map(|end| ((end - now) / HOUR).ceil().max(0.0) as usize);
    let rolled = |k: usize| past.iter().take(k).sum::<f64>();
    let (mut warned, mut capped, mut before_end, mut over_after) = (0, 0, 0, 0);
    for path in paths {
        let coming = hourly(path, shape, zone, now, horizon);
        let mut running = 0.0;
        let (mut w, mut c, mut b) = (false, false, false);
        for k in 1..=horizon {
            running += coming[k - 1];
            let window = spent - rolled(k) + running;
            if k <= 24 {
                w |= window >= allowance * warn;
                c |= window >= allowance;
            }
            if let Some(end) = end_hour {
                if k <= end {
                    b |= window >= allowance;
                }
                if k == end.max(1)
                    && let Some(fallback) = fallback
                {
                    over_after += usize::from(window >= fallback);
                }
            }
        }
        warned += usize::from(w);
        capped += usize::from(c);
        before_end += usize::from(b);
    }
    let n = paths.len().max(1) as f64;
    let frees_at = (spent >= allowance).then(|| {
        (1..=past.len())
            .find(|&k| spent - rolled(k) < allowance)
            .map_or(now + past.len() as f64 * HOUR, |k| now + k as f64 * HOUR)
    });
    Clock {
        warn_within_day: warned as f64 / n,
        cap_within_day: capped as f64 / n,
        cap_before_end: end_hour.map(|_| before_end as f64 / n),
        over_after_end: end_hour.and(fallback).map(|_| over_after as f64 / n),
        frees_at,
        freeing_in_day: rolled(24),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn even() -> Shape {
        Shape {
            shares: [[1.0 / 24.0; 24]; 7],
        }
    }

    #[test]
    fn over_the_cap_it_frees_when_old_hours_roll_out() {
        // $100 an hour for the window's first ten hours, nothing since.
        let mut past = vec![100.0; 10];
        past.extend(vec![0.0; 158]);
        let zone = Zone::utc();
        let now = 1_791_950_400.0;
        let clock = clock(
            4100.0,
            4000.0,
            0.9,
            None,
            None,
            &past,
            &vec![vec![0.0; 7]; 10],
            &even(),
            &zone,
            now,
        );
        // Two hours roll out $200, bringing $4,100 to $3,900.
        assert_eq!(clock.frees_at, Some(now + 2.0 * HOUR));
        assert_eq!(clock.cap_within_day, 1.0);
    }

    #[test]
    fn the_chance_of_the_cap_follows_the_paths() {
        let zone = Zone::utc();
        let now = 1_791_936_000.0; // midnight
        let past = vec![0.0; 168];
        // Half the paths spend $600 tomorrow... today, half nothing.
        let mut paths = vec![vec![600.0; 7]; 50];
        paths.extend(vec![vec![0.0; 7]; 50]);
        let clock = clock(
            3500.0,
            4000.0,
            0.9,
            None,
            None,
            &past,
            &paths,
            &even(),
            &zone,
            now,
        );
        assert!((clock.cap_within_day - 0.5).abs() < 1e-9);
        assert!((clock.warn_within_day - 0.5).abs() < 1e-9);
        assert_eq!(clock.frees_at, None);
    }

    #[test]
    fn an_override_ending_is_judged_against_what_follows_it() {
        let zone = Zone::utc();
        let now = 1_791_936_000.0;
        let past = vec![0.0; 168];
        let paths = vec![vec![500.0; 7]; 10];
        // Ends in two days, at $5,000 against a $4,000 fallback.
        let clock = clock(
            4000.0,
            15_000.0,
            0.9,
            Some(4000.0),
            Some(now + 48.0 * HOUR),
            &past,
            &paths,
            &even(),
            &zone,
            now,
        );
        assert_eq!(clock.cap_before_end, Some(0.0));
        assert_eq!(clock.over_after_end, Some(1.0));
    }
}
