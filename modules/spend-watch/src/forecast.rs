//! What the coming days are likely to cost, as distributions rather than
//! single figures, from two models simulated as whole paths and mixed:
//!
//! - Exponential smoothing in state-space form on log daily spend. Each day
//!   is active with its weekday's chance, weighted toward recent weeks; an
//!   active day's log spend is a level plus its weekday's effect plus an
//!   error, and the level moves by alpha times each error, so a busy day
//!   carries into the next. Alpha is chosen by one-step error over the
//!   window, and errors are resampled from the model's own past mistakes.
//! - The same days in each of the last eight weeks, as they were.
//!
//! Chosen by a rolling-origin backtest on 175 days of real spend, scored by
//! CRPS and by whether intervals hold what they claim: the mixture beat
//! either alone, and the hour-of-week profile tried before it came last
//! for whole weeks. Conformal recalibration was tried too and made coverage
//! worse, since spend changes regime faster than a window of past errors
//! follows; so coverage is tracked as it runs instead, and `score` reruns
//! the backtest on whatever history there is.
//!
//! Within a day, spend follows the hour-of-week shape: each weekday's share
//! of its spend by local hour, over the last eight weeks.

use crate::random::Random;
use crate::stats;

pub const SAMPLES: usize = 1000;
/// Below this a day counts as not worked.
const ACTIVE: f64 = 5.0;
const WINDOW_DAYS: usize = 112;
const HALF_LIFE_DAYS: f64 = 28.0;

/// Daily spend, contiguous, oldest first, with the weekday of the first.
#[derive(Debug, Clone)]
pub struct Days {
    pub first: i64,
    pub dollars: Vec<f64>,
}

impl Days {
    fn weekday(&self, index: usize) -> usize {
        crate::time::Zone::weekday(self.first + stats::signed(index))
    }

    /// The days before `index`, as their own history.
    #[must_use]
    pub fn before(&self, index: usize) -> Days {
        Days {
            first: self.first,
            dollars: self.dollars[..index].to_vec(),
        }
    }
}

/// One smoothing constant's fit: its mean squared error past the first
/// fortnight, the level and weekday effects it ends on, and its errors,
/// which the paths draw from.
struct Fit {
    sse: f64,
    alpha: f64,
    level: f64,
    season: [f64; 7],
    errors: Vec<f64>,
}

/// Paths: SAMPLES rows, each the horizon's days in order.
pub type Paths = Vec<Vec<f64>>;

pub fn state_space(days: &Days, horizon: usize, random: &mut Random) -> Paths {
    let n = days.dollars.len();
    let start = n.saturating_sub(WINDOW_DAYS);
    let weights: Vec<f64> = (start..n)
        .map(|i| 0.5f64.powf(stats::count(n - i) / HALF_LIFE_DAYS))
        .collect();
    let mut chance = [0.0; 7];
    let mut weight = [0.0; 7];
    for (offset, i) in (start..n).enumerate() {
        let weekday = days.weekday(i);
        weight[weekday] += weights[offset];
        if days.dollars[i] >= ACTIVE {
            chance[weekday] += weights[offset];
        }
    }
    for weekday in 0..7 {
        chance[weekday] = if weight[weekday] > 0.0 {
            chance[weekday] / weight[weekday]
        } else {
            0.0
        };
    }
    let active: Vec<(usize, f64)> = (start..n)
        .filter(|&i| days.dollars[i] >= ACTIVE)
        .map(|i| (days.weekday(i), days.dollars[i].ln()))
        .collect();
    if active.len() < 14 {
        return recent_weeks(days, horizon, random);
    }
    let mut best: Option<Fit> = None;
    for alpha in [0.05, 0.1, 0.2, 0.3, 0.5, 0.7] {
        let mut season = [0.0; 7];
        let mut level = 0.0;
        let mut errors = Vec::new();
        for _ in 0..2 {
            level = active[..7].iter().map(|day| day.1).sum::<f64>() / 7.0;
            errors.clear();
            let mut levels = Vec::with_capacity(active.len());
            for &(weekday, z) in &active {
                let error = z - (level + season[weekday]);
                errors.push(error);
                levels.push(level);
                level += alpha * error;
            }
            let mut sums = [0.0; 7];
            let mut counts = [0.0; 7];
            for (k, &(weekday, z)) in active.iter().enumerate() {
                sums[weekday] += z - levels[k];
                counts[weekday] += 1.0;
            }
            for weekday in 0..7 {
                season[weekday] = if counts[weekday] > 0.0 {
                    sums[weekday] / counts[weekday]
                } else {
                    0.0
                };
            }
            let centre = season.iter().sum::<f64>() / 7.0;
            for value in &mut season {
                *value -= centre;
            }
        }
        let tail = &errors[14.min(errors.len())..];
        let sse =
            tail.iter().map(|error| error * error).sum::<f64>() / stats::count(tail.len().max(1));
        if best.as_ref().is_none_or(|best| sse < best.sse) {
            best = Some(Fit {
                sse,
                alpha,
                level,
                season,
                errors: tail.to_vec(),
            });
        }
    }
    let Fit {
        alpha,
        level,
        season,
        errors,
        ..
    } = best.expect("at least one alpha");
    (0..SAMPLES)
        .map(|_| {
            let mut at = level;
            (0..horizon)
                .map(|h| {
                    let weekday = crate::time::Zone::weekday(days.first + stats::signed(n + h));
                    let error = *random.pick(&errors);
                    if random.unit() < chance[weekday] {
                        let spent = (at + season[weekday] + error).exp();
                        at += alpha * error;
                        spent
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

pub fn recent_weeks(days: &Days, horizon: usize, random: &mut Random) -> Paths {
    let n = days.dollars.len();
    let weeks: Vec<usize> = (1..=8).filter(|k| n >= 7 * k).collect();
    if weeks.is_empty() {
        return vec![vec![0.0; horizon]; SAMPLES];
    }
    (0..SAMPLES)
        .map(|_| {
            let k = *random.pick(&weeks);
            (0..horizon)
                .map(|h| {
                    // The same weekday k weeks back, or as near as there is.
                    let back = n + h;
                    let mut index = stats::signed(back) - 7 * stats::signed(k);
                    while index >= stats::signed(n) {
                        index -= 7;
                    }
                    usize::try_from(index.max(0))
                        .ok()
                        .and_then(|index| days.dollars.get(index))
                        .copied()
                        .unwrap_or(0.0)
                })
                .collect()
        })
        .collect()
}

/// Half the paths from each, or as the weights say.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the weight is clamped to [0, 1], so the rounded share lies in [0, SAMPLES]"
)]
pub fn mixture(days: &Days, horizon: usize, seed: u64, weight: f64) -> Paths {
    let mut random = Random::seeded(seed);
    let one = state_space(days, horizon, &mut random);
    let other = recent_weeks(days, horizon, &mut random);
    let take = (stats::count(SAMPLES) * weight.clamp(0.0, 1.0)).round() as usize;
    one.into_iter()
        .take(take)
        .chain(other.into_iter().take(SAMPLES - take))
        .collect()
}

/// Each path's total over its horizon's first `days` days.
pub fn totals(paths: &Paths, days: usize) -> Vec<f64> {
    paths
        .iter()
        .map(|path| path.iter().take(days).sum())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub model: &'static str,
    pub horizon: usize,
    pub origins: usize,
    pub crps: f64,
    pub mean: f64,
    pub held80: f64,
    pub held95: f64,
}

/// The rolling-origin backtest: from every day with eight weeks before it,
/// each model forecasts the next day and the next week, and is scored.
pub fn score(days: &Days, last: usize, seed: u64) -> Vec<Score> {
    type Model = fn(&Days, usize, &mut Random) -> Paths;
    let n = days.dollars.len();
    let first = 56.max(n.saturating_sub(last + 7));
    let mut out = Vec::new();
    let models: [(&'static str, Option<Model>); 3] = [
        ("mixture", None),
        ("state space", Some(state_space)),
        ("recent weeks", Some(recent_weeks)),
    ];
    for (name, model) in models {
        for horizon in [1usize, 7] {
            let mut crps = Vec::new();
            let mut actual = Vec::new();
            let (mut held80, mut held95) = (0, 0);
            for origin in first..n.saturating_sub(horizon - 1) {
                let history = days.before(origin);
                let paths = match model {
                    Some(model) => {
                        model(&history, horizon, &mut Random::seeded(seed + origin as u64))
                    }
                    None => mixture(&history, horizon, seed + origin as u64, 0.5),
                };
                let mut sums = totals(&paths, horizon);
                sums.sort_by(f64::total_cmp);
                let outcome: f64 = days.dollars[origin..origin + horizon].iter().sum();
                crps.push(stats::crps(&sums, outcome));
                actual.push(outcome);
                let within = |low: f64, high: f64| {
                    stats::quantile_sorted(&sums, low) <= outcome
                        && outcome <= stats::quantile_sorted(&sums, high)
                };
                held80 += usize::from(within(0.1, 0.9));
                held95 += usize::from(within(0.025, 0.975));
            }
            let origins = crps.len();
            if origins == 0 {
                continue;
            }
            out.push(Score {
                model: name,
                horizon,
                origins,
                crps: stats::mean(&crps),
                mean: stats::mean(&actual),
                held80: stats::count(held80) / stats::count(origins),
                held95: stats::count(held95) / stats::count(origins),
            });
        }
    }
    out
}

/// Each weekday's share of its spend by local hour: [weekday][hour].
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub shares: [[f64; 24]; 7],
}

impl Shape {
    /// From (weekday, local hour, dollars) for the last weeks' hours.
    pub fn learn(hours: &[(usize, u32, f64)]) -> Shape {
        let mut sums = [[0.0; 24]; 7];
        for &(weekday, hour, dollars) in hours {
            sums[weekday % 7][hour as usize % 24] += dollars;
        }
        let mut shares = [[1.0 / 24.0; 24]; 7];
        for weekday in 0..7 {
            let total: f64 = sums[weekday].iter().sum();
            if total > 0.0 {
                for hour in 0..24 {
                    // A little of an even day, so no hour is impossible.
                    shares[weekday][hour] = 0.95 * sums[weekday][hour] / total + 0.05 / 24.0;
                }
            }
        }
        Shape { shares }
    }

    /// The share of a weekday's spend before a local hour.
    pub fn before(&self, weekday: usize, hour: u32) -> f64 {
        self.shares[weekday % 7][..(hour as usize).min(24)]
            .iter()
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Weeks shaped like real work: busy weekdays, quiet weekends, noise.
    fn weeks(count: usize, level: f64, seed: u64) -> Days {
        let mut random = Random::seeded(seed);
        let first = crate::time::parse_day("2026-01-05").unwrap(); // a Monday
        let dollars = (0..count * 7)
            .map(|i| {
                let weekend = i % 7 >= 5;
                if weekend && random.unit() < 0.7 {
                    0.0
                } else {
                    level * (if weekend { 0.3 } else { 1.0 }) * (0.5 * random.normal()).exp()
                }
            })
            .collect();
        Days { first, dollars }
    }

    #[test]
    fn the_mixtures_intervals_hold_about_what_they_claim() {
        let days = weeks(30, 600.0, 3);
        let scores = score(&days, 60, 11);
        let day = scores
            .iter()
            .find(|s| s.model == "mixture" && s.horizon == 1)
            .unwrap();
        assert!(
            day.held80 > 0.65 && day.held80 < 0.97,
            "80% held {}",
            day.held80
        );
        assert!(day.held95 > 0.85, "95% held {}", day.held95);
    }

    #[test]
    fn the_state_space_follows_a_change_of_level() {
        let mut days = weeks(16, 300.0, 5);
        let later = weeks(4, 900.0, 6);
        days.dollars.extend(later.dollars);
        let mut random = Random::seeded(1);
        let paths = state_space(&days, 7, &mut random);
        let week = stats::median(&totals(&paths, 7));
        assert!(
            week > 3000.0,
            "a week forecast {week} after the jump to $900 days"
        );
    }

    #[test]
    fn the_same_seed_forecasts_the_same() {
        let days = weeks(10, 500.0, 2);
        assert_eq!(mixture(&days, 7, 9, 0.5), mixture(&days, 7, 9, 0.5));
    }

    #[test]
    fn a_shape_is_a_days_shares() {
        let shape = Shape::learn(&[(2, 9, 30.0), (2, 15, 10.0)]);
        assert!((shape.shares[2].iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(shape.before(2, 12) > 0.7 && shape.before(2, 12) < 0.8);
    }
}
