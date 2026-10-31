//! Caps: a provider's limit on spend over a rolling window, or over set
//! periods that reset, as feeds give them (or one set for a machine with no
//! feed), with their history of changes and a clock in chances: from the
//! forecast's simulated paths, how likely the window's spend is to reach
//! the warning and the cap within a day and before an override ends, and,
//! while over, when the oldest hours rolling out of the window, or the
//! period resetting, bring it back under.

use crate::forecast::{Paths, Shape};
use crate::history::{History, sql as fail};
use crate::stats::{count, signed};
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
    /// When a cap counted over periods next resets; none for a rolling one.
    pub resets: Option<f64>,
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

const COLUMNS: &str = "feed, name, label, allowance, window_hours, spent, warn, fallback, \
    state, ends, ends_known, at, resets";

fn cap(row: &rusqlite::Row) -> rusqlite::Result<Cap> {
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
        resets: row.get(12)?,
    })
}

/// Each cap as its feed last gave it.
pub fn latest(history: &History) -> Result<Vec<Cap>, String> {
    let mut statement = history
        .db
        .prepare(&format!(
            "SELECT {COLUMNS} FROM caps c WHERE at = (SELECT MAX(at) FROM caps d
             WHERE d.feed = c.feed AND d.name = c.name) ORDER BY feed, name"
        ))
        .map_err(fail)?;
    let rows = statement.query_map([], cap).map_err(fail)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(fail)
}

/// A cap as it was last read at or before `at`, for replaying a time gone by.
pub fn earlier_at(history: &History, of: &Cap, at: f64) -> Result<Option<Cap>, String> {
    let mut statement = history
        .db
        .prepare(&format!(
            "SELECT {COLUMNS} FROM caps WHERE feed = ?1 AND name = ?2 AND at <= ?3
             ORDER BY at DESC LIMIT 1"
        ))
        .map_err(fail)?;
    let mut rows = statement
        .query_map(rusqlite::params![of.feed, of.name, at], cap)
        .map_err(fail)?;
    rows.next().transpose().map_err(fail)
}

/// A period's cap as it was last read in each of up to `periods` periods
/// before its own, newest first; none for a rolling one.
pub fn earlier(history: &History, of: &Cap, periods: usize) -> Result<Vec<Cap>, String> {
    let Some(resets) = of.resets else {
        return Ok(Vec::new());
    };
    let mut statement = history
        .db
        .prepare(&format!(
            "SELECT {COLUMNS} FROM caps c WHERE feed = ?1 AND name = ?2 AND resets < ?3
             AND at = (SELECT MAX(at) FROM caps d WHERE d.feed = c.feed AND d.name = c.name
             AND d.resets = c.resets) ORDER BY resets DESC LIMIT ?4"
        ))
        .map_err(fail)?;
    let rows = statement
        .query_map(
            rusqlite::params![of.feed, of.name, resets, signed(periods)],
            cap,
        )
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
    /// What rolls out of the window, or goes with a reset, in the next day.
    pub freeing_in_day: f64,
}

/// Spend expected in each coming hour, for one path of daily totals: each
/// day's total spread by its weekday's hour shape.
fn hourly(path: &[f64], shape: &Shape, zone: &Zone, now: f64, hours: usize) -> Vec<f64> {
    let today = zone.day(now);
    (0..hours)
        .map(|k| {
            let at = now + count(k) * HOUR;
            let day = zone.day(at);
            // A coming hour's day is never before today's.
            let total = usize::try_from(day - today)
                .ok()
                .and_then(|index| path.get(index))
                .copied()
                .unwrap_or(0.0);
            total * shape.shares[Zone::weekday(day)][zone.local_hour(at) as usize]
        })
        .collect()
}

/// How a cap's spend leaves it: an hour at a time as a rolling window
/// passes, or all at once when a period resets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Window<'a> {
    /// The window's hours oldest first, ending now, so `past[k]` is what
    /// rolls out k hours from now.
    Rolling(&'a [f64]),
    /// The next reset, and every `hours` after it.
    Resets { at: f64, hours: usize },
}

impl Window<'_> {
    /// The spend counted k hours from now, from what's counted now and
    /// `since[j]`, what's simulated over the next j hours.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the hours to the reset are whole and at least zero, and a count past \
                  usize saturates, which is past every hour the paths reach"
    )]
    fn counted(&self, spent: f64, since: &[f64], k: usize, now: f64) -> f64 {
        match *self {
            Window::Rolling(past) => {
                let rolled: f64 = past.iter().take(k).sum();
                spent - rolled + since[k] - since[k.saturating_sub(past.len())]
            }
            Window::Resets { at, hours } => {
                // The reset falls in the hour `first` hours from now, and
                // that hour's spend counts toward the period it starts.
                let first = ((at - now) / HOUR).floor().max(0.0) as usize;
                if k <= first {
                    spent + since[k]
                } else {
                    let hours = hours.max(1);
                    let last = first + (k - 1 - first) / hours * hours;
                    since[k] - since[last]
                }
            }
        }
    }

    /// While over, when it's back under with nothing more spent.
    fn frees(&self, spent: f64, allowance: f64, now: f64) -> f64 {
        match *self {
            Window::Rolling(past) => {
                let mut left = spent;
                for (k, hour) in past.iter().enumerate() {
                    left -= hour;
                    if left < allowance {
                        return now + count(k + 1) * HOUR;
                    }
                }
                now + count(past.len()) * HOUR
            }
            Window::Resets { at, .. } => at,
        }
    }

    /// What leaves the window in the next day with nothing more spent.
    fn freeing_in_day(&self, spent: f64, now: f64) -> f64 {
        match *self {
            Window::Rolling(past) => past.iter().take(24).sum(),
            Window::Resets { at, .. } if at - now <= 24.0 * HOUR => spent,
            Window::Resets { .. } => 0.0,
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a clock is judged on a cap's figures, its window and the forecast's paths, \
              none of which belong together anywhere else"
)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the hours to an end are whole and at least zero, and a count past usize \
              saturates, which the horizon then filters out"
)]
pub fn clock(
    spent: f64,
    allowance: f64,
    warn: f64,
    fallback: Option<f64>,
    ends_at: Option<f64>,
    window: Window,
    paths: &Paths,
    shape: &Shape,
    zone: &Zone,
    now: f64,
) -> Clock {
    let horizon = 24 * paths.first().map_or(1, Vec::len);
    // An end past what the paths reach can't be judged by them.
    let end_hour = ends_at
        .map(|end| ((end - now) / HOUR).ceil().max(0.0) as usize)
        .filter(|end| end.max(&1) <= &horizon);
    let (mut warned, mut capped, mut before_end, mut over_after) = (0, 0, 0, 0);
    for path in paths {
        let coming = hourly(path, shape, zone, now, horizon);
        // What's simulated from now, by the k-th hour.
        let mut since = vec![0.0; horizon + 1];
        for k in 1..=horizon {
            since[k] = since[k - 1] + coming[k - 1];
        }
        let (mut w, mut c, mut b) = (false, false, false);
        for k in 1..=horizon {
            let window = window.counted(spent, &since, k, now);
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
    let n = count(paths.len().max(1));
    let frees_at = (spent >= allowance).then(|| window.frees(spent, allowance, now));
    Clock {
        warn_within_day: count(warned) / n,
        cap_within_day: count(capped) / n,
        cap_before_end: end_hour.map(|_| count(before_end) / n),
        over_after_end: end_hour.and(fallback).map(|_| count(over_after) / n),
        frees_at,
        freeing_in_day: window.freeing_in_day(spent, now),
    }
}

/// How long a change of allowance has to stand before it's told. A
/// provider's figure can flicker for one reading and come back on the
/// next; ten minutes is past a feed's slowest re-read of a moving figure,
/// five minutes while pi is idle, so a flicker has come back by then.
pub const HELD: f64 = 10.0 * 60.0;

/// The changes worth telling, kept in order: a change of allowance only
/// once it has stood for `HELD`, unless the cap warns or refuses at it,
/// since then it's why pi is being warned or turned away; and neither a
/// change nor any after it on the same cap when a later one took the cap
/// back to where it was within that time. Changes of state pass as they
/// are.
#[expect(
    clippy::float_cmp,
    reason = "a reading's events all carry the very instant it was read at, so only \
              equality finds the ones that came with a change"
)]
pub fn held(events: Vec<Event>, now: f64) -> Vec<Event> {
    let changes = |event: &Event| matches!(event.kind.as_str(), "lowered" | "raised" | "lapsed");
    let pressing = |event: &Event| {
        events.iter().any(|other| {
            other.feed == event.feed
                && other.name == event.name
                && other.at == event.at
                && matches!(other.kind.as_str(), "notified" | "refused")
        })
    };
    let told: Vec<bool> = events
        .iter()
        .map(|event| !changes(event) || now - event.at >= HELD || pressing(event))
        .collect();
    let mut flickered = vec![false; events.len()];
    for (first, event) in events.iter().enumerate() {
        if !changes(event) || flickered[first] {
            continue;
        }
        let back = events
            .iter()
            .enumerate()
            .skip(first + 1)
            .find(|(_, later)| {
                later.feed == event.feed
                    && later.name == event.name
                    && changes(later)
                    && later.at - event.at < HELD
                    && later.after == event.before
            });
        if let Some((last, _)) = back {
            for (index, later) in events.iter().enumerate().take(last + 1).skip(first) {
                if later.feed == event.feed && later.name == event.name && changes(later) {
                    flickered[index] = true;
                }
            }
        }
    }
    events
        .into_iter()
        .zip(told.into_iter().zip(flickered))
        .filter(|(_, (told, flickered))| *told && !flickered)
        .map(|(event, _)| event)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A period's earlier readings are each period's last, newest first, and
    // a rolling cap has none.
    #[test]
    fn a_period_s_cap_is_known_by_each_period_s_last_reading() {
        let history = History::memory();
        history
            .db
            .execute_batch(
                "INSERT INTO caps VALUES
                 ('proxy', 'premium', 10.0, 'premium day', 1000, 24, 5, 0.5, NULL, 'ok', NULL, 0,
                  100.0),
                 ('proxy', 'premium', 20.0, 'premium day', 1000, 24, 7, 0.5, NULL, 'ok', NULL, 0,
                  100.0),
                 ('proxy', 'premium', 110.0, 'premium day', 1000, 24, 9, 0.5, NULL, 'ok', NULL, 0,
                  200.0),
                 ('proxy', 'premium', 120.0, 'premium day', 1000, 24, 1, 0.5, NULL, 'ok', NULL, 0,
                  300.0),
                 ('proxy', 'weekly', 120.0, 'weekly cap', 15000, 168, 4, 0.9, NULL, 'ok', NULL, 0,
                  NULL);",
            )
            .unwrap();
        let caps = latest(&history).unwrap();
        let spent = |cap: &Cap, periods| -> Vec<f64> {
            earlier(&history, cap, periods)
                .unwrap()
                .iter()
                .map(|reading| reading.spent)
                .collect()
        };
        assert_eq!(spent(&caps[0], 7), [9.0, 7.0]);
        assert_eq!(spent(&caps[0], 1), [9.0]);
        assert!(spent(&caps[1], 7).is_empty());
    }

    fn change(name: &str, at: f64, kind: &str, before: f64, after: f64) -> Event {
        Event {
            feed: "proxy".into(),
            name: name.into(),
            at,
            kind: kind.into(),
            before: Some(before),
            after: Some(after),
            spent: 0.0,
        }
    }

    fn kinds(events: &[Event]) -> Vec<(&str, &str)> {
        events
            .iter()
            .map(|event| (event.name.as_str(), event.kind.as_str()))
            .collect()
    }

    // The proxy's meter said a premium day of $300 for one reading and
    // $1,000 again thirty seconds later, twice in an afternoon.
    #[test]
    fn a_cap_that_flickers_back_is_not_told() {
        let at = 1_792_012_983.0;
        let events = vec![
            change("premium", at, "lowered", 1000.0, 300.0),
            change("premium", at + 30.0, "raised", 300.0, 1000.0),
            change("weekly", at + 60.0, "raised", 4000.0, 15000.0),
        ];
        assert_eq!(kinds(&held(events, at + HOUR)), [("weekly", "raised")]);
        // Through a step on the way back, too.
        let events = vec![
            change("premium", at, "lowered", 1000.0, 300.0),
            change("premium", at + 30.0, "raised", 300.0, 500.0),
            change("premium", at + 60.0, "raised", 500.0, 1000.0),
        ];
        assert!(held(events, at + HOUR).is_empty());
    }

    #[test]
    fn a_change_is_told_once_it_has_stood() {
        let at = 1_792_012_983.0;
        let fell = || vec![change("premium", at, "lowered", 1000.0, 300.0)];
        assert!(held(fell(), at + 60.0).is_empty());
        assert_eq!(kinds(&held(fell(), at + HELD)), [("premium", "lowered")]);
        // A state is said as it changes, so it isn't held back.
        let refusing = Event {
            kind: "refused".into(),
            ..change("weekly", at, "refused", 15000.0, 15000.0)
        };
        assert_eq!(
            kinds(&held(vec![refusing.clone()], at)),
            [("weekly", "refused")]
        );
        // A change the cap refuses at, as an override lapsing does, is
        // told as it happens: it's why pi is being turned away.
        let lapsed = change("weekly", at, "lapsed", 15000.0, 15000.0);
        assert_eq!(
            kinds(&held(vec![lapsed, refusing], at + 60.0)),
            [("weekly", "lapsed"), ("weekly", "refused")]
        );
    }

    // A fall that stood for hours happened, and so did its return.
    #[test]
    fn a_change_undone_after_it_stood_is_kept() {
        let at = 1_792_000_000.0;
        let events = vec![
            change("premium", at, "lowered", 1000.0, 300.0),
            change("premium", at + 3.0 * HOUR, "raised", 300.0, 1000.0),
        ];
        assert_eq!(
            kinds(&held(events, at + 4.0 * HOUR)),
            [("premium", "lowered"), ("premium", "raised")]
        );
    }

    fn even() -> Shape {
        Shape {
            shares: [[1.0 / 24.0; 24]; 7],
        }
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "a share of ten paths that all reach the cap is exactly one"
    )]
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
            Window::Rolling(&past),
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
            Window::Rolling(&past),
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
        // Ends in two days, $3,500 now and $1,000 more by then, against a
        // $4,000 fallback.
        let clock = clock(
            3500.0,
            15_000.0,
            0.9,
            Some(4000.0),
            Some(now + 48.0 * HOUR),
            Window::Rolling(&past),
            &paths,
            &even(),
            &zone,
            now,
        );
        assert_eq!(clock.cap_before_end, Some(0.0));
        assert_eq!(clock.over_after_end, Some(1.0));
    }

    // A day's cap never holds more than a day of spend, however long the
    // override runs; and one ending past the paths can't be judged.
    #[test]
    fn a_short_window_rolls_simulated_spend_out() {
        let zone = Zone::utc();
        let now = 1_791_936_000.0;
        let past = vec![0.0; 24];
        let paths = vec![vec![600.0; 7]; 10];
        let judged = |ends: f64| {
            clock(
                0.0,
                1000.0,
                0.9,
                Some(500.0),
                Some(now + ends * HOUR),
                Window::Rolling(&past),
                &paths,
                &even(),
                &zone,
                now,
            )
        };
        let three_days = judged(72.0);
        assert_eq!(three_days.cap_before_end, Some(0.0));
        assert_eq!(three_days.over_after_end, Some(1.0));
        let ten_days = judged(240.0);
        assert_eq!(ten_days.cap_before_end, None);
        assert_eq!(ten_days.over_after_end, None);
    }

    // A day's cap that resets at midnight, two hours off, with $850 of
    // $1,000 spent and $40 an hour to come: it warns before the reset and
    // never reaches the cap after, where a rolling day would by 04:00.
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "a share of paths that all reach the cap is exactly one"
    )]
    fn a_period_s_spend_goes_all_at_once_when_it_resets() {
        let zone = Zone::utc();
        let now = 1_791_936_000.0 + 22.0 * HOUR;
        let paths = vec![vec![960.0; 8]; 10];
        let period = |spent: f64| {
            let window = Window::Resets {
                at: now + 2.0 * HOUR,
                hours: 24,
            };
            clock(
                spent,
                1000.0,
                0.9,
                None,
                None,
                window,
                &paths,
                &even(),
                &zone,
                now,
            )
        };
        let under = period(850.0);
        assert_eq!((under.warn_within_day, under.cap_within_day), (1.0, 0.0));
        assert_eq!((under.frees_at, under.freeing_in_day), (None, 850.0));
        let rolling = clock(
            850.0,
            1000.0,
            0.9,
            None,
            None,
            Window::Rolling(&[0.0; 24]),
            &paths,
            &even(),
            &zone,
            now,
        );
        assert_eq!(rolling.cap_within_day, 1.0);
        // Over it, it's back under when it resets, whatever has been spent.
        let over = period(1100.0);
        assert_eq!(over.frees_at, Some(now + 2.0 * HOUR));
    }

    // A reset an hour off, then one a day after: $50 an hour counts
    // toward each day from its own start.
    #[test]
    #[expect(clippy::float_cmp, reason = "sums of whole dollars are exact in f64")]
    fn every_period_after_the_first_resets_too() {
        let now = 1_791_936_000.0;
        let window = Window::Resets {
            at: now + HOUR,
            hours: 24,
        };
        let mut since = vec![0.0];
        for k in 1..=60 {
            since.push(50.0 * f64::from(k));
        }
        let counted = |k: usize| window.counted(0.0, &since, k, now);
        assert_eq!(counted(1), 50.0);
        assert_eq!(counted(2), 50.0);
        assert_eq!(counted(21), 1000.0);
        assert_eq!(counted(25), 1200.0);
        assert_eq!(counted(26), 50.0);
    }
}
