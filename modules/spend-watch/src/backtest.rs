//! The backtest: how the forecast would have done on every day of the
//! history, how often the burn would have been found and the caps asked,
//! replayed day by day, so its thresholds are judged on real spend.

use crate::config::Config;
use crate::forecast;
use crate::forecast::Days;
use crate::history::{History, sql as fail};
use crate::money::{dollars, percent};
use crate::picture::Picture;
use crate::report::{Report, Row};
use crate::stats;
use crate::time::{DAY, Zone};
use std::collections::BTreeMap;

/// Each local day's largest burn score, replayed with the allowance a
/// working hour of the eight weeks before it gave, and that working hour.
pub fn burn_days(
    history: &History,
    picture: &Picture,
) -> Result<BTreeMap<i64, (f64, f64)>, String> {
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
    let mut hours: BTreeMap<i64, f64> = BTreeMap::new();
    for (minute, dollars) in &spent {
        *hours.entry(minute / 60).or_default() += dollars;
    }
    // A working hour before a day: the median of the hours of the eight
    // weeks before it that spent at least a dollar, as the burn takes it.
    let working = |day: i64| {
        let to = crate::time::hour_of(zone.day_start(day));
        let from =
            crate::time::hour_of(zone.day_start(day - stats::signed(crate::detect::BURN_DAYS)));
        let active: Vec<f64> = hours
            .range(from..to)
            .map(|(_, spent)| *spent)
            .filter(|spent| *spent >= 1.0)
            .collect();
        if active.len() < 5 {
            f64::NAN
        } else {
            stats::median(&active)
        }
    };
    let mut working_by_day: BTreeMap<i64, f64> = BTreeMap::new();
    let mut score = 0.0;
    let mut out: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for minute in first..=last {
        let at = crate::time::minute_start(minute);
        let day = zone.day(at);
        let working = *working_by_day.entry(day).or_insert_with(|| working(day));
        if !working.is_finite() {
            continue;
        }
        let allowance = 2.0 * working / 60.0;
        score = f64::max(
            0.0,
            score + spent.get(&minute).copied().unwrap_or(0.0) - allowance,
        );
        let entry = out.entry(day).or_insert((0.0, working));
        entry.0 = entry.0.max(score);
    }
    Ok(out)
}

pub fn backtest(history: &History, picture: &Picture, config: &Config) -> Result<Report, String> {
    let days = &picture.days;
    let mut rows = forecast_rows(days);
    let mut notes = Vec::new();
    if rows.is_empty() {
        notes.push("Too few days to score the forecast: eight weeks and a day are needed.".into());
    }
    let alarms = burn_alarms(history, picture, config)?;
    let (caps, refused_days) = cap_asks(history, picture)?;
    let zone = &picture.zone;
    let weeks: Vec<i64> = {
        let first = days.first + 28;
        let last = zone.day(picture.now);
        (first..=last)
            .filter(|day| Zone::weekday(*day) == 0)
            .collect()
    };
    if !weeks.is_empty() {
        // A burn is listed, never told, so its days are counted as found.
        let total: u32 = alarms.values().sum();
        notes.push(format!(
            "The burn would have been found on {total} day{} in {} weeks, at most {} in a week.",
            if total == 1 { "" } else { "s" },
            weeks.len(),
            alarms.values().max().copied().unwrap_or(0),
        ));
        let cap_total: u32 = caps.values().sum();
        notes.push(format!(
            "The caps would have asked {cap_total} times, outside the week's few: cap changes, notices \
             and the {refused_days} days refused."
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
        rest: None,
    })
}

/// A row for each of the forecast's models and horizons, scored on every
/// whole day of the history; today is still going, so it's left out.
fn forecast_rows(days: &Days) -> Vec<Row> {
    let mut rows = Vec::new();
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
    rows
}

/// The burn's alarms by the Monday of their week, each day judged by the
/// threshold its trailing weeks set.
fn burn_alarms(
    history: &History,
    picture: &Picture,
    config: &Config,
) -> Result<BTreeMap<i64, u32>, String> {
    let window = crate::detect::BURN_DAYS;
    let burn = burn_days(history, picture)?;
    let rate = 1.0 / config.detectors.burn_run_length_days;
    let mut alarms: BTreeMap<i64, u32> = BTreeMap::new();
    let values: Vec<(i64, f64, f64)> = burn.iter().map(|(d, (s, w))| (*d, *s, *w)).collect();
    for (index, (day, max, working)) in values.iter().enumerate() {
        if index < window {
            continue;
        }
        let past: Vec<f64> = values[index - window..index]
            .iter()
            .map(|(_, s, _)| *s)
            .collect();
        let threshold = stats::quantile(&past, 1.0 - rate).max(*working);
        if *max > threshold {
            *alarms.entry(Zone::week_start(*day)).or_default() += 1;
        }
    }
    Ok(alarms)
}

/// The caps' events that would have asked and the days refused, by the
/// Monday of their week, and how many days were refused.
fn cap_asks(history: &History, picture: &Picture) -> Result<(BTreeMap<i64, u32>, usize), String> {
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
        *caps.entry(Zone::week_start(day)).or_default() += 1;
    }
    let mut refused_days: Vec<i64> = picture
        .refusals
        .iter()
        .map(|(at, _)| zone.day(*at))
        .collect();
    refused_days.dedup();
    for day in &refused_days {
        *caps.entry(Zone::week_start(*day)).or_default() += 1;
    }
    Ok((caps, refused_days.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::{Cost, Event, Kind, Tokens, Turn};

    const NOW: f64 = 1_791_988_200.0;

    fn turn(history: &History, at: f64, dollars: f64) {
        history
            .add_event(&Event::Turn(Turn {
                session: "s".into(),
                at,
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: dollars,
                    ..Cost::default()
                },
                quest: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }

    // A burn is listed, never told, so the backtest counts the days it
    // would have been found, not notices against the few a week.
    #[test]
    fn a_burn_is_counted_as_found_not_asked() {
        let history = History::memory();
        let today = (NOW / DAY).floor() * DAY;
        for back in 1..90 {
            let day = today - f64::from(back) * DAY;
            for minute in (9 * 60..17 * 60).step_by(10) {
                turn(&history, day + f64::from(minute) * 60.0, 1.0);
            }
        }
        for minute in 0..180 {
            turn(
                &history,
                today - DAY + f64::from(2 * 60 + minute) * 60.0,
                10.0,
            );
        }
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let report = backtest(&history, &picture, &config).unwrap();
        let burn = report
            .notes
            .iter()
            .find(|note| note.starts_with("The burn"))
            .unwrap();
        assert_eq!(
            burn, "The burn would have been found on 1 day in 9 weeks, at most 1 in a week.",
            "{:?}",
            report.notes
        );
    }
}
