//! How things stand, worked out from the history in one pass: the day,
//! the week and the month, how today compares with a usual day by this
//! hour, the forecast, the caps and their clocks, where the money went to
//! waste, and what each source is saying. Detectors, the status files and
//! reports all read this, so they never disagree.

use crate::caps::{self, Cap, Clock};
use crate::config::Config;
use crate::estimate::Ratios;
use crate::forecast::{self, Days, Paths, Shape};
use crate::history::History;
use crate::series::{self, Inputs, Piece, Source};
use crate::stats;
use crate::time::{DAY, HOUR, Zone};
use crate::waste::{self, Waste};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Figure {
    pub dollars: f64,
    pub estimated: bool,
    pub source: Option<Source>,
}

impl Figure {
    fn of(pieces: &[&Piece]) -> Figure {
        let owned: Vec<Piece> = pieces.iter().map(|piece| (*piece).clone()).collect();
        Figure {
            dollars: series::total(&owned),
            estimated: series::estimated(&owned),
            source: series::weakest(&owned),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Standing {
    /// "quiet", "usual", "ahead of usual", "well ahead of usual", or
    /// nothing while there are too few days to say.
    pub word: String,
    pub percentile: f64,
    /// A usual day's spend by this hour: the median of the comparable days.
    pub usual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapView {
    pub cap: Cap,
    /// The window's spend now: the feed's figure plus what's been billed
    /// since it was given.
    pub spent: f64,
    pub clock: Option<Clock>,
    /// The day an override ends, when known, as the instant it does.
    pub ends_at: Option<f64>,
    /// When a period's cap next resets.
    pub resets: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceState {
    pub feed: String,
    pub name: String,
    pub state: String,
    pub message: String,
    pub sign_in: Option<String>,
    pub at: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedAction {
    pub feed: String,
    pub id: String,
    pub name: String,
    pub run: Vec<String>,
    pub fits: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub now: f64,
    pub zone: Zone,
    pub today: Figure,
    pub yesterday: Figure,
    pub week: Figure,
    pub month: Figure,
    pub standing: Standing,
    /// The forecast from today on, a week of days.
    pub paths: Paths,
    pub shape: Shape,
    pub days: Days,
    /// Every hour's best dollars, by hour.
    pub hours: BTreeMap<i64, f64>,
    /// Today's and the last week's pieces, for their cuts.
    /// Every piece the history holds, for a report of any period.
    pub pieces: Vec<Piece>,
    pub pieces_today: Vec<Piece>,
    pub pieces_week: Vec<Piece>,
    pub pieces_month: Vec<Piece>,
    pub ratios: Ratios,
    pub caps: Vec<CapView>,
    pub events: Vec<caps::Event>,
    pub refusals: Vec<(f64, String)>,
    pub sources: Vec<SourceState>,
    pub actions: Vec<FeedAction>,
    pub said: Vec<(String, String)>,
    pub waste: Waste,
    pub work: Vec<waste::Work>,
    pub quests: Vec<crate::quests::Quest>,
    /// Each quest's root, the quest at the top of its family, by id.
    pub roots: HashMap<String, String>,
    pub titles: HashMap<String, String>,
    /// When pi's logs last grew.
    pub logs_at: Option<f64>,
    pub fed: bool,
    /// How many days ahead an override's end is warned of, first and last.
    pub lead: (u32, u32),
}

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

impl Picture {
    pub fn read(history: &History, config: &Config, now: f64) -> Result<Picture, String> {
        let zone = config.zone()?;
        let first_hour: i64 = history
            .db
            .query_row(
                "SELECT MIN(h) FROM
                 (SELECT MIN(hour) AS h FROM turns UNION ALL SELECT MIN(hour) FROM billed)",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(fail)?
            .unwrap_or(crate::time::hour_of(now));
        let now_hour = crate::time::hour_of(now);
        let inputs = Inputs::read(history, first_hour, now_hour + 1)?;
        let ratios = Ratios::learn(&inputs.learning());
        let pieces = inputs.pieces(&ratios);
        let mut hours: BTreeMap<i64, f64> = BTreeMap::new();
        for piece in &pieces {
            *hours.entry(piece.hour).or_default() += piece.dollars;
        }
        let today = zone.day(now);
        let first_day = zone.day(crate::time::hour_start(first_hour)).min(today);
        let mut daily = vec![0.0; (today - first_day + 1) as usize];
        for (&hour, &dollars) in &hours {
            let day = zone.day(crate::time::hour_start(hour));
            if (first_day..=today).contains(&day) {
                daily[(day - first_day) as usize] += dollars;
            }
        }
        let in_days =
            |from: i64, to: i64| -> Vec<&Piece> { span(&pieces, &zone, from, to).iter().collect() };
        let figure = |from: i64, to: i64| Figure::of(&in_days(from, to));
        let owned = |from: i64, to: i64| in_days(from, to).into_iter().cloned().collect::<Vec<_>>();
        let days = Days {
            first: first_day,
            dollars: daily.clone(),
        };
        // The forecast learns from whole days, so it's made from yesterday.
        let history_days = days.before(daily.len() - 1);
        let weight = mixture_weight(&history_days);
        let paths = if history_days.dollars.len() >= 14 {
            forecast::mixture(&history_days, 7, today as u64, weight)
        } else {
            Vec::new()
        };
        let shape = Shape::learn(
            &hours
                .range(now_hour - 24 * 56..now_hour)
                .map(|(&hour, &dollars)| {
                    let at = crate::time::hour_start(hour);
                    (Zone::weekday(zone.day(at)), zone.local_hour(at), dollars)
                })
                .collect::<Vec<_>>(),
        );
        let standing = standing(&zone, &hours, now, daily.last().copied().unwrap_or(0.0));
        let caps = cap_views(history, config, &zone, &hours, &paths, &shape, now)?;
        let events = caps::events(history, now - 30.0 * DAY)?;
        let refusals = {
            let mut statement = history
                .db
                .prepare("SELECT at, model FROM refusals ORDER BY at")
                .map_err(fail)?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(fail)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?
        };
        let sources = {
            let mut statement = history
                .db
                .prepare(
                    "SELECT feed, source, state, message, sign_in, at FROM health
                     ORDER BY feed, source",
                )
                .map_err(fail)?;
            statement
                .query_map([], |row| {
                    Ok(SourceState {
                        feed: row.get(0)?,
                        name: row.get(1)?,
                        state: row.get(2)?,
                        message: row.get(3)?,
                        sign_in: row.get(4)?,
                        at: row.get(5)?,
                    })
                })
                .map_err(fail)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?
        };
        let actions = {
            let mut statement = history
                .db
                .prepare("SELECT feed, id, name, run, fits FROM actions ORDER BY feed, id")
                .map_err(fail)?;
            statement
                .query_map([], |row| {
                    let run: String = row.get(3)?;
                    let fits: String = row.get(4)?;
                    Ok(FeedAction {
                        feed: row.get(0)?,
                        id: row.get(1)?,
                        name: row.get(2)?,
                        run: serde_json::from_str(&run).unwrap_or_default(),
                        fits: fits
                            .split(',')
                            .filter(|fit| !fit.is_empty())
                            .map(str::to_owned)
                            .collect(),
                    })
                })
                .map_err(fail)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?
        };
        // What the provider said in the last quarter hour still stands.
        let said = {
            let mut statement = history
                .db
                .prepare("SELECT severity, text FROM said WHERE at >= ?1 ORDER BY severity, text")
                .map_err(fail)?;
            statement
                .query_map([now - 900.0], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(fail)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?
        };
        let logs_at: Option<f64> = history
            .db
            .query_row("SELECT MAX(at) FROM turns", [], |row| row.get(0))
            .map_err(fail)?;
        let week_start = zone.day_start(today - 6);
        let quests = read_quests(history)?;
        Ok(Picture {
            now,
            today: figure(today, today),
            yesterday: figure(today - 1, today - 1),
            week: figure(today - 6, today),
            month: figure(today - 29, today),
            standing,
            paths,
            shape,
            days,
            pieces_today: owned(today, today),
            pieces: pieces.clone(),
            pieces_week: owned(today - 6, today),
            pieces_month: owned(today - 29, today),
            ratios,
            caps,
            events,
            refusals,
            sources,
            actions,
            said,
            waste: waste::waste(history, week_start, now + 1.0)?,
            work: waste::work(history, now - 30.0 * DAY, now + 1.0)?,
            roots: quests
                .iter()
                .map(|quest| {
                    (
                        quest.id.clone(),
                        crate::quests::root(&quests, &quest.id).to_owned(),
                    )
                })
                .collect(),
            titles: quests
                .iter()
                .map(|quest| (quest.id.clone(), quest.title.clone()))
                .collect(),
            quests,
            logs_at,
            fed: inputs.fed,
            zone: zone.clone(),
            hours: hours.clone(),
            lead: (
                config.override_lead.iter().copied().max().unwrap_or(3),
                config.override_lead.iter().copied().min().unwrap_or(1),
            ),
        })
    }

    /// The quest at the top of a quest's family, or itself when unknown.
    pub fn root<'a>(&'a self, id: &'a str) -> &'a str {
        self.roots.get(id).map_or(id, String::as_str)
    }

    /// A quest's title, or its id when it isn't known.
    pub fn title(&self, id: &str) -> String {
        self.titles
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    }

    /// The pieces of local days `from` through `to`.
    pub fn between(&self, from: i64, to: i64) -> Vec<Piece> {
        span(&self.pieces, &self.zone, from, to).to_vec()
    }

    /// The week ahead's spend: its 10th, 50th and 90th percentiles.
    pub fn week_ahead(&self) -> Option<(f64, f64, f64)> {
        (!self.paths.is_empty()).then(|| {
            let mut totals = forecast::totals(&self.paths, 7);
            totals.sort_by(f64::total_cmp);
            (
                stats::quantile_sorted(&totals, 0.1),
                stats::quantile_sorted(&totals, 0.5),
                stats::quantile_sorted(&totals, 0.9),
            )
        })
    }

    /// Today's whole spend, by the forecast, never less than what's spent.
    pub fn today_ahead(&self) -> Option<(f64, f64, f64)> {
        (!self.paths.is_empty()).then(|| {
            let so_far = self.today.dollars;
            let left = 1.0
                - self.shape.before(
                    Zone::weekday(self.zone.day(self.now)),
                    self.zone.local_hour(self.now),
                );
            let mut totals: Vec<f64> = self
                .paths
                .iter()
                .map(|path| so_far + path[0] * left)
                .collect();
            totals.sort_by(f64::total_cmp);
            (
                stats::quantile_sorted(&totals, 0.1),
                stats::quantile_sorted(&totals, 0.5),
                stats::quantile_sorted(&totals, 0.9),
            )
        })
    }

    /// The hourly rate a usual hour like this one spends at.
    pub fn expected_rate(&self) -> f64 {
        let day = self.zone.day(self.now);
        let usual_day = stats::median(
            &self
                .days
                .dollars
                .iter()
                .rev()
                .skip(1)
                .take(28)
                .copied()
                .filter(|d| *d >= 5.0)
                .collect::<Vec<_>>(),
        );
        if usual_day.is_nan() {
            return 0.0;
        }
        usual_day * self.shape.shares[Zone::weekday(day)][self.zone.local_hour(self.now) as usize]
    }
}

/// The pieces of local days `from` through `to`, from pieces in hour
/// order: an hour belongs to the day it starts in.
fn span<'a>(pieces: &'a [Piece], zone: &Zone, from: i64, to: i64) -> &'a [Piece] {
    let (start, end) = (zone.day_start(from), zone.day_start(to + 1));
    let first = pieces.partition_point(|piece| crate::time::hour_start(piece.hour) < start);
    let last = pieces.partition_point(|piece| crate::time::hour_start(piece.hour) < end);
    &pieces[first..last.max(first)]
}

fn read_quests(history: &History) -> Result<Vec<crate::quests::Quest>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT id, title, kind, parent, status, started, updated, prs, sessions FROM quests",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([], |row| {
            let sessions: String = row.get(8)?;
            Ok(crate::quests::Quest {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                parent: row.get(3)?,
                status: row.get(4)?,
                started: row.get(5)?,
                updated: row.get(6)?,
                prs: row.get(7)?,
                sessions: serde_json::from_str(&sessions).unwrap_or_default(),
            })
        })
        .map_err(fail)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(fail)
}

/// The state space's share of the mixture, from how each model scored on
/// the last four weeks' days: the better one gets more, never all.
fn mixture_weight(days: &Days) -> f64 {
    if days.dollars.len() < 56 + 28 {
        return 0.5;
    }
    let scores = forecast::score(days, 28, 0x5eed);
    let crps = |model: &str| {
        scores
            .iter()
            .find(|s| s.model == model && s.horizon == 1)
            .map(|s| s.crps)
            .unwrap_or(1.0)
    };
    let (one, other) = (
        crps("state space").max(1e-6),
        crps("recent weeks").max(1e-6),
    );
    (other / (one + other)).clamp(0.2, 0.8)
}

/// The share of what's billed that a feed's cap counts: a premium models'
/// day counts only those models, and a weekly cap may count another
/// tool's spend beside it. It's the feed's figure against what was billed
/// over the same window, and all of it for a cap of the account's own or
/// a window with nothing billed.
fn counted(cap: &Cap, hours: &BTreeMap<i64, f64>) -> f64 {
    if cap.feed.is_empty() {
        return 1.0;
    }
    let last = crate::time::hour_of(cap.at);
    // A period's cap counts from the period's start, not a window back.
    let first = match cap.resets {
        Some(resets) => crate::time::hour_of(resets - cap.window_hours as f64 * HOUR),
        None => last - cap.window_hours as i64 + 1,
    };
    let billed: f64 = hours.range(first..=last).map(|(_, d)| d).sum();
    if billed <= 0.0 {
        1.0
    } else {
        (cap.spent / billed).clamp(0.0, 2.0)
    }
}

/// A period's cap resets on its own, whether or not its feed says so again:
/// the first reset after now, from the one after it was read.
fn next_reset(cap: &Cap, now: f64) -> Option<f64> {
    let first = cap.resets?;
    let length = cap.window_hours as f64 * HOUR;
    let passed = ((now - first) / length).floor().max(-1.0) + 1.0;
    Some(first + passed * length)
}

fn standing(zone: &Zone, hours: &BTreeMap<i64, f64>, now: f64, so_far: f64) -> Standing {
    let today = zone.day(now);
    let weekend = |day: i64| Zone::weekday(day) >= 5;
    // Each day to the same wall-clock time as now, daylight saving or not.
    let by_now = |day: i64| {
        let start = zone.day_start(day);
        let end = zone.same_time(day, now);
        let (first, last) = (crate::time::hour_of(start), crate::time::hour_of(end));
        let mut sum = 0.0;
        for (&hour, &dollars) in hours.range(first..=last) {
            let from = crate::time::hour_start(hour).max(start);
            let to = (crate::time::hour_start(hour) + HOUR).min(end);
            if to > from {
                sum += dollars * (to - from) / HOUR;
            }
        }
        sum
    };
    // Only days the history covers: one before it began isn't a $0 day.
    let first = hours
        .keys()
        .next()
        .map_or(today, |&hour| zone.day(crate::time::hour_start(hour)));
    let comparable: Vec<f64> = (1..=28)
        .map(|back| today - back)
        .filter(|&day| day >= first && weekend(day) == weekend(today))
        .map(by_now)
        .collect();
    if comparable.iter().filter(|value| **value > 0.0).count() < 5 {
        return Standing::default();
    }
    let below = comparable.iter().filter(|value| **value < so_far).count() as f64;
    let equal = comparable.iter().filter(|value| **value == so_far).count() as f64;
    let percentile = (below + equal / 2.0) / comparable.len() as f64;
    let word = if percentile <= 0.1 {
        "quiet"
    } else if percentile <= 0.9 {
        "usual"
    } else if so_far > 1.5 * stats::quantile(&comparable, 0.9) {
        "well ahead of usual"
    } else {
        "ahead of usual"
    };
    Standing {
        word: word.into(),
        percentile,
        usual: stats::median(&comparable),
    }
}

fn cap_views(
    history: &History,
    config: &Config,
    zone: &Zone,
    hours: &BTreeMap<i64, f64>,
    paths: &Paths,
    shape: &Shape,
    now: f64,
) -> Result<Vec<CapView>, String> {
    let now_hour = crate::time::hour_of(now);
    let mut caps = caps::latest(history)?;
    if caps.is_empty()
        && let Some(own) = &config.cap
    {
        let window = own.window_hours as i64;
        caps.push(Cap {
            feed: String::new(),
            name: "own".into(),
            label: format!("{}-hour cap", own.window_hours),
            allowance: Some(own.allowance),
            window_hours: own.window_hours,
            spent: hours
                .range(now_hour - window + 1..=now_hour)
                .map(|(_, d)| d)
                .sum(),
            warn: own.warn,
            fallback: None,
            state: "ok".into(),
            ends: None,
            ends_known: true,
            resets: None,
            at: now,
        });
    }
    let mut out = Vec::new();
    for cap in caps {
        let window = cap.window_hours as i64;
        let share = counted(&cap, hours);
        // What's been billed since the feed's figure, hours wholly after it:
        // none when the figure came this hour.
        let after = crate::time::hour_of(cap.at) + 1;
        let billed = |from: i64| share * hours.range(from..=now_hour).map(|(_, d)| d).sum::<f64>();
        let since: f64 = if cap.feed.is_empty() || after > now_hour {
            0.0
        } else {
            billed(after)
        };
        let resets = next_reset(&cap, now);
        let spent = match resets {
            // Read in a period that's since reset: what's billed since.
            Some(next) if cap.at < next - window as f64 * HOUR => {
                billed(crate::time::hour_of(next - window as f64 * HOUR))
            }
            _ => cap.spent + since,
        };
        // What rolls out next: the window's oldest hours first.
        let past: Vec<f64> = (now_hour - window + 1..=now_hour)
            .map(|hour| share * hours.get(&hour).copied().unwrap_or(0.0))
            .collect();
        let leaving = match resets {
            Some(at) => caps::Window::Resets {
                at,
                hours: window as usize,
            },
            None => caps::Window::Rolling(&past),
        };
        let scaled: Paths;
        let paths = if share == 1.0 {
            paths
        } else {
            scaled = paths
                .iter()
                .map(|path| path.iter().map(|day| day * share).collect())
                .collect();
            &scaled
        };
        let ends_at = cap.ends.map(|day| zone.day_start(day + 1));
        let clock = match cap.allowance {
            Some(allowance) if !paths.is_empty() => Some(caps::clock(
                spent,
                allowance,
                cap.warn,
                cap.fallback,
                ends_at,
                leaving,
                paths,
                shape,
                zone,
                now,
            )),
            Some(allowance) if spent >= allowance => Some(caps::clock(
                spent,
                allowance,
                cap.warn,
                cap.fallback,
                ends_at,
                leaving,
                &vec![vec![0.0; 7]],
                shape,
                zone,
                now,
            )),
            _ => None,
        };
        out.push(CapView {
            cap,
            spent,
            clock,
            ends_at,
            resets,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(feed: &str, spent: f64) -> Cap {
        Cap {
            feed: feed.into(),
            name: "premium".into(),
            label: "premium day".into(),
            allowance: Some(1000.0),
            window_hours: 24,
            spent,
            warn: 0.5,
            fallback: None,
            state: "ok".into(),
            ends: None,
            ends_known: false,
            resets: None,
            at: 100.0 * HOUR + 1.0,
        }
    }

    // Read at 04:00 with the day resetting at 10:00: it counts from that
    // day's start, and resets every day after.
    #[test]
    fn a_period_s_cap_counts_from_its_start_and_resets_on_its_own() {
        let mut day = cap("proxy", 300.0);
        day.at = 100.0 * HOUR;
        day.resets = Some(106.0 * HOUR);
        let hours: BTreeMap<i64, f64> = (77..=100).map(|hour| (hour, 50.0)).collect();
        // The 19 hours from 82 to 100 billed $950.
        assert_eq!(counted(&day, &hours), 300.0 / 950.0);
        assert_eq!(next_reset(&day, 101.0 * HOUR), Some(106.0 * HOUR));
        assert_eq!(next_reset(&day, 106.0 * HOUR), Some(130.0 * HOUR));
        assert_eq!(next_reset(&day, 131.0 * HOUR), Some(154.0 * HOUR));
        assert_eq!(next_reset(&cap("proxy", 0.0), 101.0 * HOUR), None);
    }

    // $50 an hour billed for the day: a premium day that counted none of it
    // simulates none, and one that counted half simulates half.
    #[test]
    fn a_cap_counts_its_share_of_what_is_billed() {
        let hours: BTreeMap<i64, f64> = (77..=100).map(|hour| (hour, 50.0)).collect();
        assert_eq!(counted(&cap("proxy", 0.0), &hours), 0.0);
        assert_eq!(counted(&cap("proxy", 600.0), &hours), 0.5);
        assert_eq!(counted(&cap("", 0.0), &hours), 1.0);
        assert_eq!(counted(&cap("proxy", 10.0), &BTreeMap::new()), 1.0);
    }
}
