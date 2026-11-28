//! The burn: spend running far above what a working hour spends, found
//! minute by minute from the logs, and the minutes a running daemon keeps
//! for it between its passes.

use super::{finding, go_to};
use crate::config::Config;
use crate::findings::{Finding, Severity};
use crate::history::{History, sql as fail};
use crate::money::{dollars, whole};
use crate::picture::Picture;
use crate::stats;
use crate::time::{DAY, Zone};
use std::collections::{BTreeMap, HashMap};

/// The days of history a burn's threshold is judged on: long enough that
/// its quantile is more than the largest day of a short window.
pub const BURN_DAYS: usize = 56;

/// Each minute's spend, and the session that spent most of it.
type ByMinute = BTreeMap<i64, (f64, String)>;

/// Spend by minute from the logs, at list prices, kept between a running
/// daemon's passes. Reading eight weeks of turns again took a sixth of each
/// pass on 7 November, so the hour under way is read every time, and the
/// hours gone only once `hour_moves`, which every turn added or forgotten
/// moves, says one of them changed. Summing their `turn_hours` to tell
/// whether one had changed read those rows back off the disk each pass, 22
/// to 61 MB on 24 November, once the machine had dropped them from memory.
#[derive(Debug, Default)]
pub struct Minutes {
    gone: Option<Gone>,
    /// How many times the hours gone were read whole.
    whole: usize,
}

#[derive(Debug)]
struct Gone {
    /// The hours read, from `first` to before `split`.
    first: i64,
    split: i64,
    /// The last move among them when read.
    moved: i64,
    by_minute: ByMinute,
}

impl Minutes {
    /// Spend by minute since the minute `from` falls in.
    pub fn read(&mut self, history: &History, from: f64, now: f64) -> Result<ByMinute, String> {
        let start = crate::time::minute_of(from);
        let split = crate::time::hour_of(now);
        let kept = match self.gone.take() {
            Some(gone)
                if gone.split == split
                    && gone.first <= crate::time::hour_of(from)
                    && last_move(history, gone.first, split)? == gone.moved =>
            {
                gone
            }
            _ => {
                self.whole += 1;
                let first = crate::time::hour_of(from);
                Gone {
                    first,
                    split,
                    moved: last_move(history, first, split)?,
                    by_minute: minutes(
                        history,
                        crate::time::minute_start(start),
                        crate::time::hour_start(split),
                    )?,
                }
            }
        };
        let mut out: ByMinute = kept
            .by_minute
            .range(start..)
            .map(|(minute, spent)| (*minute, spent.clone()))
            .collect();
        out.extend(minutes(history, crate::time::hour_start(split), f64::MAX)?);
        // A write rolled back hands its moves out again, so hours read
        // inside one not yet committed are read again next time.
        self.gone = history.db.is_autocommit().then_some(kept);
        Ok(out)
    }
}

/// The last move of any hour from `first` to before `to`, which every turn
/// added to or forgotten from one of them raises.
fn last_move(history: &History, first: i64, to: i64) -> Result<i64, String> {
    history
        .db
        .query_row(
            "SELECT COALESCE(MAX(move), 0) FROM hour_moves WHERE hour >= ?1 AND hour < ?2",
            [first, to],
            |row| row.get(0),
        )
        .map_err(fail)
}

/// Spend by minute from the logs, at list prices, from `from` to before `to`.
fn minutes(history: &History, from: f64, to: f64) -> Result<ByMinute, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT CAST(at / 60 AS INTEGER), session, SUM(cost) FROM turns
             WHERE at >= ?1 AND at < ?2 GROUP BY 1, 2",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([from, to], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(fail)?;
    let mut out = ByMinute::new();
    let mut most: HashMap<i64, f64> = HashMap::new();
    for row in rows {
        let (minute, session, spent) = row.map_err(fail)?;
        let entry = out.entry(minute).or_insert((0.0, String::new()));
        entry.0 += spent;
        let top = most.entry(minute).or_insert(f64::NEG_INFINITY);
        if spent > *top {
            *top = spent;
            entry.1 = session;
        }
    }
    Ok(out)
}

/// A burn: spend running far above what a working hour spends, found by
/// CUSUM minute by minute. Its allowance is twice a working hour's rate at
/// any time of day, since agents left working overnight are work, not a
/// fault; its threshold is set so that on the last `BURN_DAYS` days it
/// would have found one once in the configured run length. It's the
/// account's spend running hot, so it's one burn a day, whichever session
/// leads it, and names the one spending most now. It's a note to look at,
/// never told: the caps say when spending fast is about to cost access.
pub fn burn(
    picture: &Picture,
    history: &History,
    config: &Config,
    minutes: &mut Minutes,
) -> Result<Vec<Finding>, String> {
    let now = picture.now;
    let from = now - stats::count(BURN_DAYS) * DAY;
    let by_minute = minutes.read(history, from, now)?;
    if by_minute.is_empty() {
        return Ok(Vec::new());
    }
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let zone = &picture.zone;
    let today = zone.day(now);
    let working = working_hour(
        &by_minute,
        crate::time::minute_of(zone.day_start(today)),
        ratio,
    );
    if !working.is_finite() || working <= 0.0 {
        return Ok(Vec::new());
    }
    // Twice a working hour's rate, whatever the hour.
    let allowance = |_minute: i64| 2.0 * working / 60.0;
    let first = crate::time::minute_of(from);
    let last = crate::time::minute_of(now);
    let mut score = 0.0;
    let mut daily_max: BTreeMap<i64, f64> = BTreeMap::new();
    let mut last_hour: Vec<(f64, String)> = Vec::new();
    for minute in first..=last {
        let (spent, session) = by_minute
            .get(&minute)
            .cloned()
            .unwrap_or((0.0, String::new()));
        let spent = spent * ratio;
        score = f64::max(0.0, score + spent - allowance(minute));
        let day = zone.day(crate::time::minute_start(minute));
        let entry = daily_max.entry(day).or_default();
        *entry = entry.max(score);
        if minute > last - 60 {
            last_hour.push((spent, session));
        }
    }
    // Only the days since the logs began, since a day before them scores
    // nothing for want of spend rather than for spending as usual, and two
    // weeks of those would make a day of logs a burn.
    let began = by_minute
        .keys()
        .next()
        .map_or(today, |minute| zone.day(crate::time::minute_start(*minute)));
    let past: Vec<f64> = daily_max
        .iter()
        .filter(|(day, _)| **day >= began && **day < today)
        .map(|(_, max)| *max)
        .collect();
    if past.len() < 14 {
        return Ok(Vec::new());
    }
    let rate = 1.0 / config.detectors.burn_run_length_days;
    // The threshold: what the daily maximum passed on that share of days,
    // and never below a working hour.
    let threshold = stats::quantile(&past, 1.0 - rate).max(working);
    if score <= threshold {
        return Ok(Vec::new());
    }
    // The score falls slowly once spending stops, so a burn is also the
    // last hour past what it allows, from a session that spent it.
    let spent_hour: f64 = last_hour.iter().map(|(spent, _)| spent).sum();
    let allowed_hour: f64 = (last - 59..=last).map(allowance).sum();
    if spent_hour <= allowed_hour {
        return Ok(Vec::new());
    }
    let mut by_session: HashMap<&str, f64> = HashMap::new();
    for (spent, session) in &last_hour {
        if !session.is_empty() && *spent > 0.0 {
            *by_session.entry(session.as_str()).or_default() += spent;
        }
    }
    let Some(top) = by_session
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(s, _)| s.to_owned())
    else {
        return Ok(Vec::new());
    };
    Ok(vec![burn_finding(
        picture,
        history,
        config,
        &top,
        (spent_hour, working),
    )])
}

/// A working hour: the median of the hours before `before`, a minute, that
/// spent at least a dollar, at the bill's ratio to list.
fn working_hour(by_minute: &BTreeMap<i64, (f64, String)>, before: i64, ratio: f64) -> f64 {
    let mut hours: BTreeMap<i64, f64> = BTreeMap::new();
    for (minute, (spent, _)) in by_minute.range(..before) {
        *hours.entry(minute / 60).or_default() += spent * ratio;
    }
    stats::median(
        &hours
            .into_values()
            .filter(|spent| *spent >= 1.0)
            .collect::<Vec<_>>(),
    )
}

/// The burn's note: the last hour's spend against a working hour's, named
/// for the session spending most, `top`, and its quest.
fn burn_finding(
    picture: &Picture,
    history: &History,
    config: &Config,
    top: &str,
    (spent_hour, working): (f64, f64),
) -> Finding {
    let today = picture.zone.day(picture.now);
    let rate = 1.0 / config.detectors.burn_run_length_days;
    let quest = history
        .db
        .query_row(
            "SELECT quest FROM turns WHERE session = ?1 ORDER BY at DESC LIMIT 1",
            [top],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    let title_of = quest
        .as_ref()
        .and_then(|id| picture.quests.iter().find(|q| &q.id == id))
        .map(|q| q.title.clone());
    let mut f = finding(
        format!("burn:{}", Zone::iso_day(today)),
        "burn",
        Severity::Note,
        match &title_of {
            Some(title) => format!("a burn in {title}"),
            None => "a burn".into(),
        },
        format!(
            "{} in the last hour against {} in a working hour; past what eight weeks crossed once \
             a {}.",
            dollars(spent_hour),
            dollars(working),
            if config.detectors.burn_run_length_days >= 28.0 {
                "month".to_owned()
            } else {
                format!("{} days", config.detectors.burn_run_length_days)
            }
        ),
    );
    f.badge = format!("{}/h", whole(spent_hour));
    f.note = format!("a working hour is {}", whole(working));
    f.dollars = spent_hour;
    f.surprise = -(rate.log10());
    f.session = Some(top.to_owned());
    f.quest = quest;
    f.action = go_to(picture, top);
    f
}

#[cfg(test)]
#[path = "burn_tests.rs"]
mod tests;
