//! How things stand, worked out from the history in one pass: the day,
//! the week and the month, how today compares with a usual day by this
//! hour, the forecast, the caps and their clocks, where the money went to
//! waste, and what each source is saying. Detectors, the status files and
//! reports all read this, so they never disagree.

use crate::caps::{self, Cap, Clock};
use crate::config::Config;
use crate::estimate::{Ratios, Shift};
use crate::forecast::{self, Days, Paths, Shape};
use crate::history::{History, sql as fail};
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
    fn of(pieces: &[Piece]) -> Figure {
        Figure {
            dollars: series::total(pieces),
            estimated: series::estimated(pieces),
            source: series::weakest(pieces),
        }
    }
}

/// How a day can stand against the usual ones by the same hour: a word,
/// since the margin's header holds it beside the day's figure, and how far
/// ahead is a burn's or a forecast's to say.
pub const WORDS: [&str; 3] = ["quiet", "usual", "ahead"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Standing {
    /// One of `WORDS`, or nothing while there are too few days to say.
    pub word: String,
    pub percentile: f64,
    /// A usual day's spend by this hour: the median of the comparable days.
    pub usual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapView {
    pub cap: Cap,
    /// The budget it limits, by name.
    pub budget: String,
    /// Whose it is, as its findings' keys say: its feed's, or its budget's
    /// for a cap of the machine's own.
    pub owner: String,
    /// The window's spend now: the feed's figure plus what's been billed
    /// since it was given. Nothing, for a share.
    pub spent: f64,
    /// The share of a plan's window used now, for a share: none
    /// since a reset nobody has read past.
    pub used: Option<f64>,
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
    /// When it came to stand as it does.
    pub since: f64,
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
    /// The same, each budget's apart, by its name.
    pub budget_hours: HashMap<String, BTreeMap<i64, f64>>,
    pub budgets: crate::budgets::Budgets,
    /// Every piece the history holds, in hour order, for a report of any
    /// period. A period's are a slice of these, never a copy: a piece
    /// holds eight strings, and a pass reads some fifty periods.
    pub pieces: Vec<Piece>,
    pub ratios: Ratios,
    /// Models the bill now prices apart from how it did.
    pub shifts: Vec<Shift>,
    pub caps: Vec<CapView>,
    pub events: Vec<caps::Event>,
    pub refusals: Vec<(f64, String)>,
    pub sources: Vec<SourceState>,
    pub actions: Vec<FeedAction>,
    pub said: Vec<(String, String)>,
    pub waste: Waste,
    /// The share rebuilt in each of the eight weeks before the last seven
    /// days, of those that spent anything.
    pub waste_before: Vec<f64>,
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

/// How long the windows a pass reads over weeks are kept between a running
/// daemon's passes. Ten minutes moves a thirty-day window by 0.02%, while
/// reading one again took a fifth of each pass on 7 November, when the
/// history held 1.5 million turns.
pub const SETTLED: f64 = 600.0;

/// What a running daemon keeps from one pass to the next, so that a pass
/// every thirty seconds reads again only what can have moved.
#[derive(Debug, Default)]
pub struct Kept {
    /// The eight weeks before the last seven days, which are whole days
    /// gone, by the day they were read on: exact until the day changes.
    waste_before: Option<(i64, Vec<f64>)>,
    /// The last seven days' waste and the last thirty days' work, by when
    /// they were read and the week's start they were read from.
    weeks: Option<Weeks>,
    /// The eight weeks of spend by minute a burn is judged on.
    pub minutes: crate::detect::Minutes,
}

#[derive(Debug)]
struct Weeks {
    at: f64,
    week_start: f64,
    waste: Waste,
    work: Vec<waste::Work>,
}

impl Kept {
    fn waste_before(
        &mut self,
        history: &History,
        zone: &Zone,
        today: i64,
    ) -> Result<Vec<f64>, String> {
        if let Some((day, shares)) = &self.waste_before
            && *day == today
        {
            return Ok(shares.clone());
        }
        let shares = waste_before(history, zone, today)?;
        self.waste_before = Some((today, shares.clone()));
        Ok(shares)
    }

    fn weeks(
        &mut self,
        history: &History,
        week_start: f64,
        now: f64,
    ) -> Result<(Waste, Vec<waste::Work>), String> {
        if let Some(weeks) = &self.weeks
            && weeks.week_start.total_cmp(&week_start).is_eq()
            && (0.0..SETTLED).contains(&(now - weeks.at))
        {
            return Ok((weeks.waste.clone(), weeks.work.clone()));
        }
        let waste = waste::waste(history, week_start, now + 1.0)?;
        let work = waste::work(history, now - 30.0 * DAY, now + 1.0)?;
        self.weeks = Some(Weeks {
            at: now,
            week_start,
            waste: waste.clone(),
            work: work.clone(),
        });
        Ok((waste, work))
    }
}

impl Picture {
    /// How things stand now, read whole.
    pub fn read(history: &History, config: &Config, now: f64) -> Result<Picture, String> {
        Picture::read_keeping(history, config, now, &mut Kept::default())
    }

    /// How things stand now, taking what `kept` holds that can't have moved
    /// since it was read, and keeping what's read for the next pass.
    pub fn read_keeping(
        history: &History,
        config: &Config,
        now: f64,
        kept: &mut Kept,
    ) -> Result<Picture, String> {
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
        let budgets = crate::budgets::Budgets::of(config);
        let inputs = Inputs::read(history, &budgets, first_hour, now_hour + 1)?;
        let (ratios, shifts) = learned(&inputs, &zone);
        let pieces = inputs.pieces(&ratios);
        let (hours, budget_hours) = hours_of(&pieces);
        let today = zone.day(now);
        let first_day = zone.day(crate::time::hour_start(first_hour)).min(today);
        let daily = daily(&zone, &hours, first_day, today);
        let figure = |from: i64, to: i64| Figure::of(span(&pieces, &zone, from, to));
        let days = Days {
            first: first_day,
            dollars: daily.clone(),
        };
        // The forecast learns from whole days, so it's made from yesterday.
        let history_days = days.before(daily.len() - 1);
        let weight = mixture_weight(&history_days);
        let paths = if history_days.dollars.len() >= 14 {
            forecast::mixture(&history_days, 7, today.cast_unsigned(), weight)
        } else {
            Vec::new()
        };
        let shape = shape(&zone, &hours, now_hour);
        let standing = standing(&zone, &hours, now, daily.last().copied().unwrap_or(0.0));
        let forecast = Forecast {
            paths: &paths,
            shape: &shape,
            hours: &hours,
        };
        let caps = cap_views(history, &budgets, &zone, &budget_hours, &forecast, now)?;
        let events = caps::held(caps::events(history, now - 30.0 * DAY)?, now);
        let refusals = read_refusals(history)?;
        let sources = read_sources(history)?;
        let actions = read_actions(history)?;
        let said = read_said(history, now)?;
        let logs_at: Option<f64> = history
            .db
            .query_row("SELECT MAX(at) FROM turns", [], |row| row.get(0))
            .map_err(fail)?;
        let week_start = zone.day_start(today - 6);
        let waste_before = kept.waste_before(history, &zone, today)?;
        let (waste, work) = kept.weeks(history, week_start, now)?;
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
            ratios,
            shifts,
            caps,
            events,
            refusals,
            sources,
            actions,
            said,
            waste,
            waste_before,
            work,
            roots: roots(&quests),
            titles: quests
                .iter()
                .map(|quest| (quest.id.clone(), quest.title.clone()))
                .collect(),
            quests,
            logs_at,
            fed: inputs.fed,
            zone: zone.clone(),
            hours: hours.clone(),
            budget_hours,
            budgets,
            lead: (
                config.override_lead.iter().copied().max().unwrap_or(3),
                config.override_lead.iter().copied().min().unwrap_or(1),
            ),
            pieces,
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
    pub fn between(&self, from: i64, to: i64) -> &[Piece] {
        span(&self.pieces, &self.zone, from, to)
    }

    /// Today's pieces.
    pub fn pieces_today(&self) -> &[Piece] {
        let today = self.zone.day(self.now);
        self.between(today, today)
    }

    /// The last seven days' pieces, today's among them.
    pub fn pieces_week(&self) -> &[Piece] {
        let today = self.zone.day(self.now);
        self.between(today - 6, today)
    }

    /// The last thirty days' pieces, today's among them.
    pub fn pieces_month(&self) -> &[Piece] {
        let today = self.zone.day(self.now);
        self.between(today - 29, today)
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

/// Every hour's dollars, and each budget's apart.
fn hours_of(pieces: &[Piece]) -> (BTreeMap<i64, f64>, HashMap<String, BTreeMap<i64, f64>>) {
    let mut hours: BTreeMap<i64, f64> = BTreeMap::new();
    let mut budget_hours: HashMap<String, BTreeMap<i64, f64>> = HashMap::new();
    for piece in pieces {
        *hours.entry(piece.hour).or_default() += piece.dollars;
        *budget_hours
            .entry(piece.budget.clone())
            .or_default()
            .entry(piece.hour)
            .or_default() += piece.dollars;
    }
    (hours, budget_hours)
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

/// Each model's ratio of bill to list, and the prices that have moved,
/// whose new ratios the ratios take up at once: a month's ratio lags a
/// price that's moved, so the hours the bill hasn't come for are
/// estimated at the price as it is now.
fn learned(inputs: &Inputs, zone: &Zone) -> (Ratios, Vec<Shift>) {
    let learning = inputs.learning();
    let mut ratios = Ratios::learn(&learning);
    let shifts = crate::estimate::shifts(&learning, |hour| zone.day(crate::time::hour_start(hour)));
    for shift in &shifts {
        if let Some(ratio) = ratios.models.get_mut(&shift.model) {
            ratio.ratio = shift.now;
        }
    }
    (ratios, shifts)
}

/// Each local day's dollars from `first_day` to `today`, from the hours'.
fn daily(zone: &Zone, hours: &BTreeMap<i64, f64>, first_day: i64, today: i64) -> Vec<f64> {
    // The first day is never after today, so the count is at least one.
    let count = usize::try_from(today - first_day + 1).unwrap_or(0);
    let mut daily = vec![0.0; count];
    for (&hour, &dollars) in hours {
        let day = zone.day(crate::time::hour_start(hour));
        if (first_day..=today).contains(&day)
            && let Ok(index) = usize::try_from(day - first_day)
        {
            daily[index] += dollars;
        }
    }
    daily
}

/// How a day's spend falls across the hours of the week, from the eight
/// weeks before this hour.
fn shape(zone: &Zone, hours: &BTreeMap<i64, f64>, now_hour: i64) -> Shape {
    Shape::learn(
        &hours
            .range(now_hour - 24 * 56..now_hour)
            .map(|(&hour, &dollars)| {
                let at = crate::time::hour_start(hour);
                (Zone::weekday(zone.day(at)), zone.local_hour(at), dollars)
            })
            .collect::<Vec<_>>(),
    )
}

fn read_refusals(history: &History) -> Result<Vec<(f64, String)>, String> {
    let mut statement = history
        .db
        .prepare("SELECT at, model FROM refusals ORDER BY at")
        .map_err(fail)?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)
}

fn read_sources(history: &History) -> Result<Vec<SourceState>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT feed, source, state, message, sign_in, at, coalesce(since, at)
             FROM health ORDER BY feed, source",
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
                since: row.get(6)?,
            })
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)
}

fn read_actions(history: &History) -> Result<Vec<FeedAction>, String> {
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
        .map_err(fail)
}

/// What the provider said in the last quarter hour, which still stands.
fn read_said(history: &History, now: f64) -> Result<Vec<(String, String)>, String> {
    let mut statement = history
        .db
        .prepare("SELECT severity, text FROM said WHERE at >= ?1 ORDER BY severity, text")
        .map_err(fail)?;
    statement
        .query_map([now - 900.0], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)
}

/// The share of each of the eight weeks before this one that was rebuilt
/// context, for the weeks that spent.
fn waste_before(history: &History, zone: &Zone, today: i64) -> Result<Vec<f64>, String> {
    Ok((1..=8)
        .map(|weeks| {
            let first = today - 6 - 7 * weeks;
            waste::share(history, zone.day_start(first), zone.day_start(first + 7))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}

/// Each quest's root, the quest at the top of its family.
fn roots(quests: &[crate::quests::Quest]) -> HashMap<String, String> {
    quests
        .iter()
        .map(|quest| {
            (
                quest.id.clone(),
                crate::quests::root(quests, &quest.id).to_owned(),
            )
        })
        .collect()
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
            .map_or(1.0, |s| s.crps)
    };
    let (one, other) = (
        crps("state space").max(1e-6),
        crps("recent weeks").max(1e-6),
    );
    (other / (one + other)).clamp(0.2, 0.8)
}

/// How many periods before a period cap's own its share is judged over.
const EARLIER_PERIODS: usize = 7;

/// The share of what's billed that a feed's cap counts: a premium models'
/// day counts only those models, and a weekly cap may count another
/// tool's spend beside it. It's the feed's figure against what was billed
/// over the same window, and all of it for a cap of the account's own or
/// a window with nothing billed. A period's cap is judged over the periods
/// before it too, since one just begun has too little billed to say.
fn counted(cap: &Cap, earlier: &[Cap], hours: &BTreeMap<i64, f64>) -> f64 {
    if cap.feed.is_empty() {
        return 1.0;
    }
    let (mut spent, mut billed) = (0.0, 0.0);
    for reading in std::iter::once(cap).chain(earlier) {
        let last = crate::time::hour_of(reading.at);
        // A period's cap counts from the period's start, not a window back.
        let first = match reading.resets {
            Some(resets) => crate::time::hour_of(resets - f64::from(reading.window_hours) * HOUR),
            None => last - i64::from(reading.window_hours) + 1,
        };
        spent += reading.spent;
        billed += hours.range(first..=last).map(|(_, d)| d).sum::<f64>();
    }
    if billed <= 0.0 {
        1.0
    } else {
        (spent / billed).clamp(0.0, 2.0)
    }
}

/// A period's cap resets on its own, whether or not its feed says so again:
/// the first reset after now, from the one after it was read.
fn next_reset(cap: &Cap, now: f64) -> Option<f64> {
    let first = cap.resets?;
    let length = f64::from(cap.window_hours) * HOUR;
    let passed = ((now - first) / length).floor().max(-1.0) + 1.0;
    Some(first + passed * length)
}

#[expect(
    clippy::float_cmp,
    reason = "a day that spent the very same as today is a tie, counted half as a mid-rank \
              percentile counts it"
)]
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
    let below = stats::count(comparable.iter().filter(|value| **value < so_far).count());
    let equal = stats::count(comparable.iter().filter(|value| **value == so_far).count());
    let percentile = (below + equal / 2.0) / stats::count(comparable.len());
    let word = if percentile <= 0.1 {
        WORDS[0]
    } else if percentile <= 0.9 {
        WORDS[1]
    } else {
        WORDS[2]
    };
    Standing {
        word: word.into(),
        percentile,
        usual: stats::median(&comparable),
    }
}

/// What each cap's clock is judged on: the forecast's paths of the whole
/// account's spend, how a day falls across its hours, and every hour's.
struct Forecast<'a> {
    paths: &'a Paths,
    shape: &'a Shape,
    hours: &'a BTreeMap<i64, f64>,
}

/// Each cap read, the feeds' and the machine's own, each against only
/// its own budget's hours.
fn cap_views(
    history: &History,
    budgets: &crate::budgets::Budgets,
    zone: &Zone,
    budget_hours: &HashMap<String, BTreeMap<i64, f64>>,
    forecast: &Forecast,
    now: f64,
) -> Result<Vec<CapView>, String> {
    let fed = caps::latest(history)?;
    let own = own_caps(budgets, &fed, budget_hours, now);
    let none = BTreeMap::new();
    // Until the first hour a budget other than the default spent, every
    // turn was the default's because nothing told them apart.
    let told = budget_hours
        .iter()
        .filter(|(name, _)| **name != budgets.default)
        .filter_map(|(_, hours)| hours.keys().next().copied())
        .min();
    let mut out = Vec::new();
    for cap in fed.into_iter().chain(own) {
        let budget = budgets.of_record(&cap.feed, &cap.budget);
        let owner = caps::owner(&cap, &budgets.default);
        let hours = budget_hours.get(&budget).unwrap_or(&none);
        let view = match cap.unit {
            caps::Unit::Share => share_view(cap, now),
            caps::Unit::Dollars => {
                let earlier = caps::earlier(history, &cap, EARLIER_PERIODS)?;
                let now_hour = crate::time::hour_of(now);
                let from = share_from(told, i64::from(cap.window_hours), now_hour);
                let of_all = budget_share(hours, forecast.hours, from, now_hour);
                dollars_view(cap, &earlier, hours, of_all, forecast, zone, now)
            }
        };
        out.push(CapView {
            budget,
            owner,
            ..view
        });
    }
    Ok(out)
}

/// A share of a plan's window as its feed read it, or none used
/// once it has reset since. A share is never turned into dollars, so it
/// has no forecast of its own.
fn share_view(cap: Cap, now: f64) -> CapView {
    let resets = next_reset(&cap, now);
    let window = f64::from(cap.window_hours) * HOUR;
    let used = match resets {
        Some(next) if cap.at < next - window => Some(0.0),
        _ => cap.used,
    };
    CapView {
        budget: String::new(),
        owner: String::new(),
        spent: 0.0,
        used,
        clock: None,
        ends_at: None,
        resets,
        cap,
    }
}

/// A cap in dollars: its spend now, from the feed's figure and what its
/// budget has billed since, and its clock, from the forecast at the
/// budget's share of all spend and the cap's share of its budget's.
fn dollars_view(
    cap: Cap,
    earlier: &[Cap],
    hours: &BTreeMap<i64, f64>,
    of_all: f64,
    forecast: &Forecast,
    zone: &Zone,
    now: f64,
) -> CapView {
    let now_hour = crate::time::hour_of(now);
    let window = i64::from(cap.window_hours);
    let cap_share = counted(&cap, earlier, hours);
    // What's been billed since the feed's figure, hours wholly after it:
    // none when the figure came this hour.
    let after = crate::time::hour_of(cap.at) + 1;
    let billed = |from: i64| cap_share * hours.range(from..=now_hour).map(|(_, d)| d).sum::<f64>();
    let since: f64 = if cap.feed.is_empty() || after > now_hour {
        0.0
    } else {
        billed(after)
    };
    let resets = next_reset(&cap, now);
    let spent = match resets {
        // Read in a period that's since reset: what's billed since.
        Some(next) if cap.at < next - f64::from(cap.window_hours) * HOUR => billed(
            crate::time::hour_of(next - f64::from(cap.window_hours) * HOUR),
        ),
        _ => cap.spent + since,
    };
    // What rolls out next: the window's oldest hours first.
    let past: Vec<f64> = (now_hour - window + 1..=now_hour)
        .map(|hour| cap_share * hours.get(&hour).copied().unwrap_or(0.0))
        .collect();
    let leaving = match resets {
        Some(at) => caps::Window::Resets {
            at,
            hours: cap.window_hours as usize,
        },
        None => caps::Window::Rolling(&past),
    };
    let paths = scaled(forecast.paths, cap_share * of_all);
    let ends_at = cap.ends.map(|day| zone.day_start(day + 1));
    let still = vec![vec![0.0; 7]];
    let paths = match cap.allowance {
        Some(_) if !paths.is_empty() => Some(paths.as_ref()),
        // Over with no forecast, it's still back once its spend leaves.
        Some(allowance) if spent >= allowance => Some(&still),
        _ => None,
    };
    let clock = cap.allowance.zip(paths).map(|(allowance, paths)| {
        caps::clock(
            spent,
            allowance,
            cap.warn,
            cap.fallback,
            ends_at,
            leaving,
            paths,
            forecast.shape,
            zone,
            now,
        )
    });
    CapView {
        budget: String::new(),
        owner: String::new(),
        spent,
        used: None,
        clock,
        ends_at,
        resets,
        cap,
    }
}

/// The first hour a budget's share of all spend is measured from: the
/// last four weeks, but none from before budgets were first told apart,
/// when every turn was the default's, unless that leaves less than the
/// cap's own window.
fn share_from(told: Option<i64>, window: i64, now_hour: i64) -> i64 {
    let weeks = now_hour - 28 * 24 + 1;
    let window = now_hour - window + 1;
    told.map_or(weeks, |told| told.max(weeks).min(window))
}

/// How much of all spend a budget's was from an hour on, which the
/// forecast of all of it is scaled by for the budget's caps; all of it
/// while nothing was spent.
fn budget_share(
    budget: &BTreeMap<i64, f64>,
    all: &BTreeMap<i64, f64>,
    from: i64,
    now_hour: i64,
) -> f64 {
    let recent = from..=now_hour;
    let total: f64 = all.range(recent.clone()).map(|(_, d)| d).sum();
    if total <= 0.0 {
        return 1.0;
    }
    (budget.range(recent).map(|(_, d)| d).sum::<f64>() / total).clamp(0.0, 1.0)
}

/// The allowances in dollars the config declares, counted here from each
/// budget's hours: each unless a feed reads its budget's of the same
/// name, and the machine's own cap only while no feed gives its budget
/// one, as it always has.
fn own_caps(
    budgets: &crate::budgets::Budgets,
    fed: &[Cap],
    budget_hours: &HashMap<String, BTreeMap<i64, f64>>,
    now: f64,
) -> Vec<Cap> {
    let read = |budget: &str, name: Option<&str>| {
        fed.iter().any(|cap| {
            budgets.of_record(&cap.feed, &cap.budget) == budget
                && name.is_none_or(|name| cap.name == name)
        })
    };
    let none = BTreeMap::new();
    budgets
        .all
        .iter()
        .flat_map(|budget| {
            budget
                .allowances
                .iter()
                .map(move |allowance| (budget, allowance))
        })
        .filter(|(_, allowance)| allowance.unit == caps::Unit::Dollars)
        .filter(|(budget, allowance)| {
            !read(
                &budget.name,
                (!allowance.stands_in).then_some(allowance.name.as_str()),
            )
        })
        .map(|(budget, allowance)| {
            let hours = budget_hours.get(&budget.name).unwrap_or(&none);
            own_cap(&budget.name, allowance, hours, now)
        })
        .collect()
}

/// An allowance of the machine's own, read from what its budget has billed.
fn own_cap(
    budget: &str,
    declared: &crate::config::AllowanceConfig,
    hours: &BTreeMap<i64, f64>,
    now: f64,
) -> Cap {
    let now_hour = crate::time::hour_of(now);
    let mut cap = Cap {
        feed: String::new(),
        budget: budget.to_owned(),
        name: declared.name.clone(),
        label: crate::budgets::window_name(declared.window_hours),
        unit: caps::Unit::Dollars,
        allowance: declared.allowance,
        window_hours: declared.window_hours,
        spent: 0.0,
        warn: declared.warn,
        fallback: None,
        state: "ok".into(),
        ends: None,
        ends_known: true,
        resets: declared.resets.as_deref().and_then(crate::time::parse),
        used: None,
        at: now,
    };
    // A period counts from its start; a rolling window, its last hours.
    let first = match next_reset(&cap, now) {
        Some(next) => crate::time::hour_of(next - f64::from(declared.window_hours) * HOUR),
        None => now_hour - i64::from(declared.window_hours) + 1,
    };
    cap.spent = hours.range(first..=now_hour).map(|(_, d)| d).sum();
    cap
}

/// The forecast's paths at a cap's share of what's billed, as they are
/// when it counts all of it.
#[expect(
    clippy::float_cmp,
    reason = "a share of exactly one is what `counted` gives a cap that counts everything, \
              and scaling by it would only copy the paths"
)]
fn scaled(paths: &Paths, share: f64) -> std::borrow::Cow<'_, Paths> {
    if share == 1.0 {
        std::borrow::Cow::Borrowed(paths)
    } else {
        std::borrow::Cow::Owned(
            paths
                .iter()
                .map(|path| path.iter().map(|day| day * share).collect())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(feed: &str, spent: f64) -> Cap {
        Cap {
            feed: feed.into(),
            budget: String::new(),
            name: "premium".into(),
            label: "Premium Day".into(),
            unit: caps::Unit::Dollars,
            used: None,
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
    #[expect(
        clippy::float_cmp,
        reason = "whole dollars sum exactly, so the share is the very division written here"
    )]
    fn a_period_s_cap_counts_from_its_start_and_resets_on_its_own() {
        let mut day = cap("proxy", 300.0);
        day.at = 100.0 * HOUR;
        day.resets = Some(106.0 * HOUR);
        let hours: BTreeMap<i64, f64> = (77..=100).map(|hour| (hour, 50.0)).collect();
        // The 19 hours from 82 to 100 billed $950.
        assert_eq!(counted(&day, &[], &hours), 300.0 / 950.0);
        assert_eq!(next_reset(&day, 101.0 * HOUR), Some(106.0 * HOUR));
        assert_eq!(next_reset(&day, 106.0 * HOUR), Some(130.0 * HOUR));
        assert_eq!(next_reset(&day, 131.0 * HOUR), Some(154.0 * HOUR));
        assert_eq!(next_reset(&cap("proxy", 0.0), 101.0 * HOUR), None);
    }

    // $50 an hour billed for the day: a premium day that counted none of it
    // simulates none, and one that counted half simulates half.
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "whole dollars sum exactly, and these shares are exact binary fractions"
    )]
    fn a_cap_counts_its_share_of_what_is_billed() {
        let hours: BTreeMap<i64, f64> = (77..=100).map(|hour| (hour, 50.0)).collect();
        assert_eq!(counted(&cap("proxy", 0.0), &[], &hours), 0.0);
        assert_eq!(counted(&cap("proxy", 600.0), &[], &hours), 0.5);
        assert_eq!(counted(&cap("", 0.0), &[], &hours), 1.0);
        assert_eq!(counted(&cap("proxy", 10.0), &[], &BTreeMap::new()), 1.0);
    }

    // A premium day read just after midnight has nothing billed in it yet,
    // which says nothing of its share; the days before, which counted none
    // of what was billed, say it's none.
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "whole dollars sum exactly, and these shares are exact binary fractions"
    )]
    fn a_period_just_begun_takes_its_share_from_the_ones_before() {
        let period = |at: f64, resets: f64, spent: f64| {
            let mut day = cap("proxy", spent);
            day.at = at;
            day.resets = Some(resets);
            day
        };
        let today = period(100.0 * HOUR + 60.0, 124.0 * HOUR, 0.0);
        let hours: BTreeMap<i64, f64> = (52..=99).map(|hour| (hour, 50.0)).collect();
        let earlier = [
            period(99.0 * HOUR + 3000.0, 100.0 * HOUR, 0.0),
            period(75.0 * HOUR + 3000.0, 76.0 * HOUR, 0.0),
        ];
        assert_eq!(counted(&today, &earlier, &hours), 0.0);
        // Half of what each earlier day billed counted, and today has just
        // begun at the same: still half.
        let earlier = [
            period(99.0 * HOUR + 3000.0, 100.0 * HOUR, 600.0),
            period(75.0 * HOUR + 3000.0, 76.0 * HOUR, 600.0),
        ];
        assert_eq!(counted(&today, &earlier, &hours), 0.5);
        // With nothing known before either, all of it counts.
        assert_eq!(counted(&today, &[], &hours), 1.0);
    }

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z

    /// The proxy's cap the default, a team's token with an allowance of
    /// the machine's own, and a Claude plan the proxy's feed reads a share
    /// of.
    fn budgeted() -> Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
                 "whose": "employer", "default": true},
                {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team",
                 "allowances": [{"name": "week", "allowance": 400, "windowHours": 168,
                                 "warn": 0.9}]},
                {"name": "claude", "label": "Claude Pro", "kind": "plan",
                 "whose": "own", "providers": ["claude-code"]},
            ],
            "feeds": [{"name": "proxy", "label": "the AI proxy", "command": ["f"]}],
        }))
        .unwrap()
    }

    fn turn(history: &History, session: &str, budget: &str, at: f64, cost: f64) {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        history
            .add_event(&Event::Turn(Turn {
                session: session.into(),
                at,
                model: "m".into(),
                provider: "anthropic".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: cost,
                    ..Cost::default()
                },
                quest: None,
                budget: Some(budget.into()),
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }

    // A running daemon reads the windows that span weeks at most every
    // SETTLED seconds, and today's spend on every pass.
    #[test]
    fn a_kept_picture_reads_the_weeks_again_only_once_they_could_have_moved() {
        let history = History::memory();
        let mut kept = Kept::default();
        turn(&history, "one", "proxy", NOW - HOUR, 10.0);
        let first = Picture::read_keeping(&history, &budgeted(), NOW, &mut kept).unwrap();
        assert_eq!(first.work.len(), 1);
        turn(&history, "two", "proxy", NOW - 60.0, 20.0);
        let soon = Picture::read_keeping(&history, &budgeted(), NOW + 60.0, &mut kept).unwrap();
        assert!(
            soon.today.dollars > first.today.dollars,
            "{} then {}",
            first.today.dollars,
            soon.today.dollars
        );
        assert_eq!(
            soon.work.len(),
            1,
            "the month's work as it was a minute ago"
        );
        let later = Picture::read_keeping(&history, &budgeted(), NOW + SETTLED, &mut kept).unwrap();
        assert_eq!(later.work.len(), 2);
        let fresh = Picture::read(&history, &budgeted(), NOW + 60.0).unwrap();
        assert_eq!(fresh.work.len(), 2, "a picture read alone keeps nothing");
    }

    fn store(history: &History, line: &serde_json::Value) {
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(history, "proxy", &record, NOW).unwrap();
    }

    // A budget's share of all spend counts no hour from before budgets
    // were told apart, when every turn was the default's, but never less
    // than the cap's window, nor more than four weeks.
    #[test]
    fn a_budget_s_share_counts_from_when_budgets_were_told_apart() {
        let now = 10_000;
        assert_eq!(share_from(None, 168, now), now - 28 * 24 + 1);
        assert_eq!(share_from(Some(now - 400), 168, now), now - 400);
        assert_eq!(share_from(Some(now - 2), 168, now), now - 167);
        assert_eq!(share_from(Some(now - 5000), 168, now), now - 28 * 24 + 1);
        // Four weeks of the default alone, then a day that was half a
        // team's: from the day budgets were told apart, the default's
        // share is half, not nearly all.
        let all: BTreeMap<i64, f64> = (now - 671..=now).map(|hour| (hour, 2.0)).collect();
        let default: BTreeMap<i64, f64> = (now - 671..=now)
            .map(|hour| (hour, if hour > now - 24 { 1.0 } else { 2.0 }))
            .collect();
        let from = share_from(Some(now - 23), 24, now);
        assert!((budget_share(&default, &all, from, now) - 0.5).abs() < 1e-9);
    }

    // A team's token spending fast moves only its own cap: the proxy's
    // cap counts what the proxy's budget spent since its reading, and an
    // allowance of the machine's own counts only its budget's hours.
    #[test]
    fn each_cap_counts_only_its_budget_s_spend() {
        let history = History::memory();
        let read = crate::time::hour_start(crate::time::hour_of(NOW) - 2) + 60.0;
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
                "allowance": 1000, "windowHours": 168, "spent": 100, "warn": 0.9,
                "state": "ok", "at": crate::time::rfc3339(read),
            }),
        );
        turn(&history, "me", "", NOW - HOUR, 50.0);
        turn(&history, "team", "delta", NOW - HOUR, 500.0);
        let picture = Picture::read(&history, &budgeted(), NOW).unwrap();
        let view = |name: &str| {
            picture
                .caps
                .iter()
                .find(|view| view.cap.name == name)
                .unwrap()
        };
        let weekly = view("weekly");
        assert_eq!((weekly.budget.as_str(), weekly.spent), ("proxy", 150.0));
        let week = view("week");
        assert_eq!((week.budget.as_str(), week.spent), ("delta", 500.0));
        assert_eq!(week.cap.label, "Week");
        let found = crate::detect::caps(&picture);
        let keys: Vec<&str> = found.iter().map(|finding| finding.key.as_str()).collect();
        assert_eq!(keys, ["cap:/delta:week:over"]);
    }

    // A share of a plan's window is its used share, never dollars,
    // warned at its warning, and nothing used once it has reset unread.
    #[test]
    fn a_share_is_its_used_share_until_it_resets() {
        let history = History::memory();
        let share = |used: f64, at: f64, resets: f64| {
            serde_json::json!({
                "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours",
                "budget": "claude", "unit": "share", "used": used, "windowHours": 5,
                "warn": 0.8, "state": "ok", "at": crate::time::rfc3339(at),
                "resets": crate::time::rfc3339(resets),
            })
        };
        store(&history, &share(0.85, NOW - 600.0, NOW + 2.0 * HOUR));
        let picture = Picture::read(&history, &budgeted(), NOW).unwrap();
        let view = &picture.caps[0];
        assert_eq!(
            (view.budget.as_str(), view.used, view.clock.is_none()),
            ("claude", Some(0.85), true)
        );
        let found = crate::detect::caps(&picture);
        assert_eq!(found[0].key, "cap:proxy/claude:five-hours:near");
        assert!(!found[0].detail.contains('$'), "{}", found[0].detail);
        store(&history, &share(1.0, NOW - 6.0 * HOUR, NOW - HOUR));
        let later = Picture::read(&history, &budgeted(), NOW + 1.0).unwrap();
        assert_eq!(later.caps[0].used, Some(0.85), "the newest reading stands");
        let after = Picture::read(&history, &budgeted(), NOW + 3.0 * HOUR).unwrap();
        assert_eq!(after.caps[0].used, Some(0.0));
    }
}
