//! The backtest: how the forecast would have done on every day of the
//! history, and how often the watch would have asked for the person,
//! replayed day by day, so its thresholds are judged on real spend.

use crate::config::Config;
use crate::forecast;
use crate::history::History;
use crate::money::{dollars, percent};
use crate::picture::Picture;
use crate::report::{Report, Row};
use crate::stats;
use crate::time::{DAY, Zone};
use std::collections::BTreeMap;

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

/// Each local day's largest burn score, replayed with the allowance a
/// usual day of the four weeks before it gave.
pub fn burn_days(history: &History, picture: &Picture) -> Result<BTreeMap<i64, f64>, String> {
    let zone = &picture.zone;
    let mut statement = history
        .db
        .prepare("SELECT CAST(at / 60 AS INTEGER), SUM(cost) FROM turns GROUP BY 1 ORDER BY 1")
        .map_err(fail)?;
    let minutes: Vec<(i64, f64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    let (Some(first), Some(last)) = (minutes.first().map(|m| m.0), minutes.last().map(|m| m.0))
    else {
        return Ok(BTreeMap::new());
    };
    let spent: BTreeMap<i64, f64> = minutes.into_iter().collect();
    let usual = |day: i64| {
        let from = (day - picture.days.first - 28).max(0) as usize;
        let to = ((day - picture.days.first).max(0) as usize).min(picture.days.dollars.len());
        let active: Vec<f64> = picture.days.dollars[from.min(to)..to]
            .iter()
            .copied()
            .filter(|d| *d >= 5.0)
            .collect();
        if active.len() < 5 {
            f64::NAN
        } else {
            stats::median(&active)
        }
    };
    let mut usual_by_day: BTreeMap<i64, f64> = BTreeMap::new();
    let mut score = 0.0;
    let mut out: BTreeMap<i64, f64> = BTreeMap::new();
    for minute in first..=last {
        let at = minute as f64 * 60.0;
        let day = zone.day(at);
        let usual_day = *usual_by_day.entry(day).or_insert_with(|| usual(day));
        if !usual_day.is_finite() {
            continue;
        }
        let share = picture.shape.shares[Zone::weekday(day)][zone.local_hour(at) as usize];
        let allowance = (2.0 * usual_day * share / 60.0).max(1.0 / 60.0);
        score = f64::max(
            0.0,
            score + spent.get(&minute).copied().unwrap_or(0.0) - allowance,
        );
        let entry = out.entry(day).or_default();
        *entry = entry.max(score);
    }
    Ok(out)
}

pub fn backtest(history: &History, picture: &Picture, config: &Config) -> Result<Report, String> {
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    let days = &picture.days;
    // Every whole day; today is still going.
    let whole = days.before(days.dollars.len().saturating_sub(1));
    for score in forecast::score(&whole, whole.dollars.len(), 0x5eed) {
        rows.push(Row {
            name: format!(
                "{}, {}",
                score.model,
                if score.horizon == 1 {
                    "a day"
                } else {
                    "a week"
                }
            ),
            dollars: score.crps,
            share: 0.0,
            shown: format!(
                "CRPS {} against a mean of {}; 80% range held {}, 95% held {}; {} origins",
                dollars(score.crps),
                dollars(score.mean),
                percent(score.held80),
                percent(score.held95),
                score.origins
            ),
            estimated: false,
        });
    }
    if rows.is_empty() {
        notes.push("Too few days to score the forecast: eight weeks and a day are needed.".into());
    }
    // The burn's alarms, by the threshold each day's trailing weeks set.
    let window = crate::detect::BURN_DAYS;
    let burn = burn_days(history, picture)?;
    let rate = 1.0 / config.detectors.burn_run_length_days;
    let mut alarms: BTreeMap<i64, u32> = BTreeMap::new();
    let values: Vec<(i64, f64)> = burn.iter().map(|(d, s)| (*d, *s)).collect();
    for (index, (day, max)) in values.iter().enumerate() {
        if index < window {
            continue;
        }
        let past: Vec<f64> = values[index - window..index]
            .iter()
            .map(|(_, s)| *s)
            .collect();
        let usual_floor = {
            let from = (day - days.first - 28).max(0) as usize;
            let to = ((day - days.first).max(0) as usize).min(days.dollars.len());
            stats::median(
                &days.dollars[from.min(to)..to]
                    .iter()
                    .copied()
                    .filter(|d| *d >= 5.0)
                    .collect::<Vec<_>>(),
            ) / 12.0
        };
        let threshold = stats::quantile(&past, 1.0 - rate).max(if usual_floor.is_finite() {
            usual_floor
        } else {
            0.0
        });
        if *max > threshold {
            *alarms.entry(day - Zone::weekday(*day) as i64).or_default() += 1;
        }
    }
    // The caps' events that would have asked, and days refused.
    let mut statement = history
        .db
        .prepare(
            "SELECT at FROM cap_events WHERE kind IN ('lowered', 'lapsed', 'notified', 'refused')",
        )
        .map_err(fail)?;
    let cap_events: Vec<f64> = statement
        .query_map([], |row| row.get(0))
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    let zone = &picture.zone;
    let mut caps: BTreeMap<i64, u32> = BTreeMap::new();
    for at in &cap_events {
        let day = zone.day(*at);
        *caps.entry(day - Zone::weekday(day) as i64).or_default() += 1;
    }
    let mut refused_days: Vec<i64> = picture
        .refusals
        .iter()
        .map(|(at, _)| zone.day(*at))
        .collect();
    refused_days.dedup();
    for day in &refused_days {
        *caps.entry(day - Zone::weekday(*day) as i64).or_default() += 1;
    }
    let weeks: Vec<i64> = {
        let first = days.first + 28;
        let last = zone.day(picture.now);
        (first..=last)
            .filter(|day| Zone::weekday(*day) == 0)
            .collect()
    };
    if !weeks.is_empty() {
        let counts: Vec<f64> = weeks
            .iter()
            .map(|week| {
                f64::from(
                    alarms
                        .get(week)
                        .copied()
                        .unwrap_or(0)
                        .min(config.notices.budget),
                )
            })
            .collect();
        let over = weeks
            .iter()
            .filter(|week| alarms.get(week).copied().unwrap_or(0) > config.notices.budget)
            .count();
        let total: u32 = alarms.values().sum();
        notes.push(format!(
            "The burn would have asked {total} times in {} weeks: {:.1} a week, the most {} in \
             one; {over} weeks past the budget of {}.",
            weeks.len(),
            stats::mean(&counts),
            alarms.values().max().copied().unwrap_or(0),
            config.notices.budget
        ));
        let cap_total: u32 = caps.values().sum();
        notes.push(format!(
            "The caps would have asked {cap_total} times, outside the budget: cap changes, notices \
             and the {} days refused.",
            refused_days.len()
        ));
        for (week, count) in alarms.iter().filter(|(_, c)| **c > 0).rev().take(8) {
            rows.push(Row {
                name: format!("week of {}", Zone::day_name(*week)),
                dollars: f64::from(*count),
                share: 0.0,
                shown: format!(
                    "{count} burn{}{}",
                    if *count == 1 { "" } else { "s" },
                    caps.get(week)
                        .map_or(String::new(), |c| format!(", {c} from caps"))
                ),
                estimated: false,
            });
        }
    }
    let first = Zone::iso_day(days.first);
    let last = Zone::iso_day(zone.day(picture.now - DAY));
    Ok(Report {
        title: "Backtest".into(),
        period: "backtest".into(),
        from: first,
        to: last,
        total: 0.0,
        shown: String::new(),
        source: String::new(),
        rows,
        notes,
    })
}
