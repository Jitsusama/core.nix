//! When the tokens run out: the one warning, written into status.json for
//! the shell to raise, of when a cap that refuses the account is reached at
//! the last hour's rate, and when it's back under.
//!
//! A cap like the proxy's is enforced by a job that runs about hourly. It
//! counts the week's spend, and at the first run that finds it at the cap,
//! the account is refused until a later run finds it back under. So the
//! tokens run out at a run, never between, and the warning names that run.
//! The count a run makes trails the bill by some hours and catches up in
//! jumps: on 31 October its rises from run to run were -$106, $360, $160,
//! -$79, $524, $212 and then $584, refusing at 10:02, at about twice the
//! hour's rate. So each run ahead is taken to rise by the larger of the
//! hour's rate less what rolls out of the week and the largest rise of the
//! last six runs, and the warning names the soonest run that could refuse.
//!
//! The levels are by the time left, since what Joel can do (claim a raise,
//! pause the session spending most) takes minutes, and a run is an hour
//! apart: under four hours ("hours"), time to slow down; under an hour
//! ("hour"), the next run or the one after; under fifteen minutes
//! ("minutes"), the next run; and refused ("out"). Each level reached is a
//! new notice, and nothing else is: a level is left only once the time
//! left passes its own bound by half an hour, so a rate that wavers can't
//! raise it again and again.
//!
//! A cap limits one budget, so its rate, what rolls out of it, the
//! session spending most and a refusal are all that budget's alone: a
//! team's token spending fast says nothing of Joel's own cap.

use crate::budgets::Budgets;
use crate::caps::Cap;
use crate::history::{History, sql as fail};
use crate::money::whole;
use crate::time::{HOUR, Zone, hour_of, hour_start};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    // Ordered from the most pressing, so the lesser of two is the sooner.
    Out,
    Minutes,
    Hour,
    Hours,
    /// The cap couldn't be read lately, while it was close: said in the
    /// row, never raised as a notice.
    Unread,
}

impl Level {
    /// The time left under which a level holds, in seconds.
    fn under(self) -> f64 {
        match self {
            Level::Out => 0.0,
            Level::Minutes => 15.0 * 60.0,
            Level::Hour => HOUR,
            Level::Hours | Level::Unread => 4.0 * HOUR,
        }
    }
}

/// How much past a level's own bound the time left must rise before the
/// level is left, so it isn't raised again by a rate that wavers.
const LEAVE: f64 = 30.0 * 60.0;
/// How old the cap's reading may be before it's taken as not read: two
/// runs missed.
const STALE: f64 = 2.5 * HOUR;
/// How much of the cap must be used before a stale reading is said.
const UNREAD_FROM: f64 = 0.75;
/// How many runs ahead are looked at for the cap to be reached, or left.
const AHEAD: usize = 7 * 24;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quiet {
    pub title: String,
    pub value: String,
    pub context: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Act {
    /// spends, to see what spends; pause, to pause a session.
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub level: Level,
    /// Whether it's pressing enough to show over what Joel is doing.
    pub now: bool,
    pub title: String,
    pub value: String,
    pub context: String,
    pub hint: String,
    pub quiet: Quiet,
    pub actions: Vec<Act>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<String>,
    /// When the tokens run out, or ran out; for tests and the replay.
    #[serde(skip)]
    pub runs_out: Option<f64>,
    /// When it's back under, if spending stops when it runs out.
    #[serde(skip)]
    pub back: Option<f64>,
    /// The key of the cap it speaks for, whose own notices it replaces.
    #[serde(skip)]
    pub cap: String,
    /// The key of that cap's row as the spend collection lists it, so the
    /// launcher's Tokens row opens the same drawer rather than one of its
    /// own (X0G8U9, 2026-11-08).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub row: String,
}

/// What the outlook is worked out from, read from the history.
#[derive(Debug, Clone, PartialEq)]
pub struct Inputs {
    pub cap: Cap,
    /// Spent over the last hour, in dollars.
    pub rate: f64,
    /// What rolls out of the cap's window in each hour from now on, the
    /// first being the hour now.
    pub rolling: Vec<f64>,
    /// The session that spent most over the last hour, and its quest.
    pub top: Option<(String, Option<String>)>,
    /// How much the cap's count rose at each of its last runs.
    pub rises: Vec<f64>,
    /// When a provider last refused a session for being over a cap, if
    /// that's since the cap's reading and the session hasn't spent since.
    pub refused: Option<f64>,
    /// Whose count the cap's reading is, as its feed's label names it, so
    /// the figure says where it's from: the feed's name until the daemon
    /// gives the label.
    pub counter: String,
}

/// What a cap is read against: the budget it limits, and every budget's
/// word on which that is.
struct Capped<'a> {
    budgets: &'a Budgets,
    budget: String,
}

impl Capped<'_> {
    fn is(&self, budget: &str) -> bool {
        budget == self.budget
    }

    /// Whether a turn on a provider, under what its log said, drew on it.
    fn spent_by(&self, provider: &str, logged: Option<&str>) -> bool {
        let budget = self.budgets.of_turn(provider, logged);
        self.is(budget.as_deref().unwrap_or(&self.budgets.default))
    }
}

/// The last refusal since the cap's reading that its session hasn't got
/// past, of a session drawing on the cap's budget as it was refused: the
/// tokens have run out whatever the forecast says, since the cap's next
/// reading is up to an hour off.
fn refused(history: &History, capped: &Capped, cap: &Cap, now: f64) -> Result<Option<f64>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT r.at, (SELECT t.provider FROM turns t WHERE t.session = r.session
                 AND t.at <= r.at ORDER BY t.at DESC LIMIT 1),
             (SELECT t.budget FROM turns t WHERE t.session = r.session
                 AND t.at <= r.at ORDER BY t.at DESC LIMIT 1)
             FROM refusals r WHERE r.at > ?1 AND r.at <= ?2
             AND NOT EXISTS (SELECT 1 FROM turns t WHERE t.session = r.session
             AND t.at > r.at AND t.at <= ?2)",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map(rusqlite::params![cap.at, now], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(fail)?;
    let mut last: Option<f64> = None;
    for row in rows {
        let (at, provider, logged) = row.map_err(fail)?;
        if capped.spent_by(provider.as_deref().unwrap_or(""), logged.as_deref()) {
            last = Some(last.map_or(at, |last| last.max(at)));
        }
    }
    Ok(last)
}

/// How many of the cap's last runs its rises are read from.
const RISES: i64 = 6;

/// How the cap's count rose from run to run over its last runs, a run an
/// hour, each its last reading, as they stood at `now`.
fn rises(history: &History, cap: &Cap, now: f64) -> Result<Vec<f64>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT spent FROM caps c WHERE feed = ?1 AND budget = ?2 AND name = ?3
             AND at <= ?4 AND at > ?5
             AND at = (SELECT MAX(at) FROM caps d WHERE d.feed = c.feed
             AND d.budget = c.budget AND d.name = c.name
             AND CAST(d.at / 3600 AS INTEGER) = CAST(c.at / 3600 AS INTEGER) AND d.at <= ?4)
             ORDER BY at",
        )
        .map_err(fail)?;
    let since = now - HOUR * (crate::stats::count(usize::try_from(RISES).unwrap_or(6)) + 1.0);
    let counts = statement
        .query_map(
            rusqlite::params![cap.feed, cap.budget, cap.name, now, since],
            |row| row.get::<_, f64>(0),
        )
        .map_err(fail)?
        .collect::<Result<Vec<f64>, _>>()
        .map_err(fail)?;
    Ok(counts.windows(2).map(|pair| pair[1] - pair[0]).collect())
}

/// The cap to watch: the one with an allowance over the longest window,
/// since it's the one a refusal lasts longest under.
fn watched(caps: Vec<Cap>) -> Option<Cap> {
    caps.into_iter()
        .filter(|cap| cap.allowance.is_some_and(|allowance| allowance > 0.0))
        .max_by_key(|cap| cap.window_hours)
}

/// What a source billed the cap's budget in each hour from `from` to
/// `to`.
fn billed(
    history: &History,
    capped: &Capped,
    source: &str,
    from: i64,
    to: i64,
) -> Result<Vec<f64>, String> {
    let mut out = vec![0.0; usize::try_from(to - from).unwrap_or(0)];
    let mut statement = history
        .db
        .prepare(
            "SELECT hour, feed, budget, SUM(dollars) FROM billed WHERE source = ?1
             AND hour >= ?2 AND hour < ?3 GROUP BY hour, feed, budget",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map(rusqlite::params![source, from, to], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .map_err(fail)?;
    for row in rows {
        let (hour, feed, budget, dollars) = row.map_err(fail)?;
        if !capped.is(&capped.budgets.of_record(&feed, &budget)) {
            continue;
        }
        if let Some(slot) = usize::try_from(hour - from)
            .ok()
            .and_then(|i| out.get_mut(i))
        {
            *slot += dollars;
        }
    }
    Ok(out)
}

/// The session that spent most of the cap's budget over the last hour,
/// and its quest.
fn top(
    history: &History,
    capped: &Capped,
    hour: i64,
) -> Result<Option<(String, Option<String>)>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT session, quest, provider, budget, SUM(cost) FROM turn_hours
             WHERE hour >= ?1 AND hour <= ?2 GROUP BY session, quest, provider, budget",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([hour - 1, hour], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(fail)?;
    let mut spent: std::collections::BTreeMap<String, (f64, Option<String>)> =
        std::collections::BTreeMap::new();
    for row in rows {
        let (session, quest, provider, logged, cost) = row.map_err(fail)?;
        if capped.spent_by(&provider, logged.as_deref()) {
            let entry = spent.entry(session).or_insert((0.0, None));
            entry.0 += cost;
            entry.1 = entry.1.take().or(quest);
        }
    }
    Ok(spent
        .into_iter()
        .max_by(|one, other| one.1.0.total_cmp(&other.1.0))
        .map(|(session, (_, quest))| (session, quest)))
}

/// How the last hour's rate is read: live, from the meter's hour so far
/// and the part of the hour before it still within the last sixty
/// minutes; or as a replay of a time gone by, from the last whole hour's
/// bill, since the meter's hour is kept only as it last stood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    Live,
    Replay,
}

/// What the history says, as it stood at `now` when replaying.
pub fn read(
    history: &History,
    budgets: &Budgets,
    now: f64,
    rate: Rate,
) -> Result<Option<Inputs>, String> {
    let caps = match rate {
        Rate::Live => crate::caps::latest(history)?,
        Rate::Replay => crate::caps::latest(history)?
            .into_iter()
            .filter_map(|cap| crate::caps::earlier_at(history, &cap, now).ok().flatten())
            .collect(),
    };
    let Some(cap) = watched(caps) else {
        return Ok(None);
    };
    let capped = Capped {
        budgets,
        budget: budgets.of_record(&cap.feed, &cap.budget),
    };
    let billed = |source: &str, from: i64, to: i64| billed(history, &capped, source, from, to);
    let hour = hour_of(now);
    let into = (now - hour_start(hour)) / HOUR;
    let rate = match rate {
        Rate::Live => {
            let meter = billed("meter", hour - 1, hour + 1)?;
            let before = if meter[0] > 0.0 {
                meter[0]
            } else {
                billed("warehouse", hour - 1, hour)?[0]
            };
            meter[1] + before * (1.0 - into)
        }
        Rate::Replay => {
            let last = billed("warehouse", hour - 1, hour)?[0];
            if last > 0.0 {
                last
            } else {
                billed("meter", hour - 1, hour)?[0]
            }
        }
    };
    let window = i64::from(cap.window_hours);
    let rolling = billed(
        "warehouse",
        hour - window,
        hour - window + i64::try_from(AHEAD).unwrap_or(0) + 1,
    )?;
    let top = top(history, &capped, hour)?;
    let rises = rises(history, &cap, now)?;
    let refused = refused(history, &capped, &cap, now)?;
    let counter = cap.feed.clone();
    Ok(Some(Inputs {
        cap,
        rate,
        rolling,
        top,
        rises,
        refused,
        counter,
    }))
}

/// Where the count stands at each run from the cap's reading on: the
/// reading, then each run an hour after the one before, moved by the
/// larger of the rate less what rolls out in the hour it ends in and the
/// largest recent rise; after the run `stop`, nothing more is spent and
/// only what rolls out moves it.
fn counts(inputs: &Inputs, now: f64, stop: Option<usize>) -> Vec<(f64, f64)> {
    let cap = &inputs.cap;
    let mut count = cap.spent;
    let mut out = vec![(cap.at, count)];
    for run in 1..=AHEAD {
        let at = cap.at + HOUR * crate::stats::count(run);
        let rolled = usize::try_from(hour_of(at) - hour_of(now))
            .ok()
            .and_then(|k| inputs.rolling.get(k))
            .copied()
            .unwrap_or(0.0);
        let rise = if stop.is_some_and(|stop| run > stop) {
            -rolled
        } else {
            inputs
                .rises
                .iter()
                .copied()
                .fold(inputs.rate - rolled, f64::max)
        };
        count = (count + rise).max(0.0);
        out.push((at, count));
    }
    out
}

fn minutes(seconds: f64) -> String {
    let hours = seconds / HOUR;
    if hours >= 1.5 {
        format!("About {} Hours", hours.round())
    } else if hours >= 0.75 {
        "About an Hour".into()
    } else if seconds < 5.0 * 60.0 {
        // Not a count ticking to nothing, since the run comes when it does.
        "Under 5 Min".into()
    } else {
        let five = (seconds / 60.0 / 5.0).round() * 5.0;
        format!("About {five} Min")
    }
}

/// The start of the keys of the watched cap's findings, which the warning
/// speaks for.
#[must_use]
pub fn key(inputs: &Inputs) -> String {
    // A feed's cap is its feed's whatever the default budget is.
    format!(
        "cap:{}:{}:",
        crate::caps::owner(&inputs.cap, ""),
        inputs.cap.name
    )
}

/// The key the spend collection gives its cap's row (`collection.rs`,
/// `cap_line`), which differs from `key`: a row is one cap's, whoever's.
fn row(inputs: &Inputs) -> String {
    format!("cap:{}", inputs.cap.name)
}

/// What Joel can do: see what spends, and pause the session spending most.
fn acts(inputs: &Inputs) -> Vec<Act> {
    let mut acts = vec![Act {
        kind: "spends".into(),
        name: "See What Spends".into(),
        session: None,
        quest: None,
    }];
    if let Some((session, quest)) = &inputs.top {
        let short = quest
            .as_deref()
            .and_then(|quest| quest.rsplit('-').next())
            .unwrap_or("the session spending most");
        acts.push(Act {
            kind: "pause".into(),
            name: format!("Pause {short}"),
            session: Some(session.clone()),
            quest: quest.clone(),
        });
    }
    acts
}

/// Said in the row while the cap's reading is too old to project from.
fn unread(inputs: &Inputs, zone: &Zone, now: f64) -> Tokens {
    let since = format!("can't be read since {}", zone.clock(inputs.cap.at, now));
    Tokens {
        level: Level::Unread,
        now: false,
        title: "Tokens Can't Be Read".into(),
        value: String::new(),
        context: since.clone(),
        hint: format!("Tokens {since}"),
        quiet: Quiet {
            title: "Tokens Can't Be Read".into(),
            value: String::new(),
            context: since.clone(),
            hint: format!("Tokens {since}"),
            unread: Some(since.clone()),
        },
        actions: acts(inputs),
        unread: Some(since),
        runs_out: None,
        back: None,
        cap: key(inputs),
        row: row(inputs),
    }
}

/// The level for the time left, keeping one said before until the time
/// left rises past its own bound by half an hour, so a wavering rate
/// doesn't raise it again.
fn level(left: f64, before: Option<Level>) -> Option<Level> {
    let fresh = [Level::Minutes, Level::Hour, Level::Hours]
        .into_iter()
        .find(|level| left < level.under());
    let held = before
        .filter(|held| matches!(held, Level::Minutes | Level::Hour | Level::Hours))
        .filter(|held| left < held.under() + LEAVE);
    match (fresh, held) {
        (Some(fresh), Some(held)) => Some(fresh.min(held)),
        (fresh, held) => fresh.or(held),
    }
}

/// The outlook now, given the level said before, if any.
#[must_use]
pub fn outlook(inputs: &Inputs, before: Option<Level>, zone: &Zone, now: f64) -> Option<Tokens> {
    let cap = &inputs.cap;
    let allowance = cap.allowance?;
    let used = cap.spent / allowance;
    // The figure is the provider's own count at its last run, which is
    // what it refuses by, so it says whose count it is and when it ran:
    // anything else, such as its calendar-week meter, counts differently.
    let by = format!("by {}'s {} count", inputs.counter, zone.clock(cap.at, now));
    let count = format!("{} of {} {by}", whole(cap.spent), whole(allowance));
    let percent = format!("{:.0}% {by}", used * 100.0);
    let read_over = cap.state == "refused" || cap.spent >= allowance;
    let over = read_over || inputs.refused.is_some();
    if !over && now - cap.at > STALE {
        return (used >= UNREAD_FROM).then(|| unread(inputs, zone, now));
    }
    // The run at which it's reached, and the first after it and after now
    // back under if nothing more is spent from then. A refusal the cap's
    // reading doesn't show yet came from the run the forecast says, or
    // failing that the last run so far.
    let ahead = counts(inputs, now, None);
    let forecast = ahead
        .iter()
        .position(|&(at, count)| at > cap.at && count >= allowance);
    let reached = if read_over {
        Some(0)
    } else if over {
        forecast
            .filter(|&run| ahead[run].0 <= now + HOUR)
            .or_else(|| ahead.iter().rposition(|&(at, _)| at <= now))
    } else {
        forecast
    }?;
    let runs_out = inputs.refused.unwrap_or(ahead[reached].0);
    let back = counts(inputs, now, Some(reached))
        .into_iter()
        .skip(reached + 1)
        .find(|&(at, count)| at > now && count < allowance)
        .map(|(at, _)| at);
    let left = (runs_out - now).max(0.0);
    let level = if over {
        Level::Out
    } else {
        level(left, before)?
    };
    let resets = back.map_or_else(String::new, |at| format!("resets {}", zone.clock(at, now)));
    // The notice's summary, so it says the reset too.
    let then = back.map_or_else(String::new, |at| {
        format!("; they reset at {}", zone.clock(at, now))
    });
    let (title, hint) = if over {
        ("Tokens Ran Out".to_owned(), format!("Tokens ran out{then}"))
    } else {
        (
            format!("Tokens Run Out in {}", minutes(left)),
            format!(
                "Tokens run out in {} at this rate{then}",
                minutes(left).to_lowercase()
            ),
        )
    };
    let rate = if over {
        count
    } else {
        format!("{count} · {} an hour", whole(inputs.rate))
    };
    let quiet_rate = percent;
    Some(Tokens {
        level,
        now: matches!(level, Level::Out | Level::Minutes),
        title: title.clone(),
        value: resets.clone(),
        context: rate,
        hint: hint.clone(),
        quiet: Quiet {
            title,
            value: resets,
            context: quiet_rate,
            hint,
            unread: None,
        },
        actions: acts(inputs),
        unread: None,
        runs_out: Some(runs_out),
        back,
        cap: key(inputs),
        row: row(inputs),
    })
}

/// What the warning said from one instant of a replay until its words
/// changed: its level, hint and context, or nothing where it had nothing
/// to say.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Said {
    pub at: f64,
    pub level: Option<Level>,
    pub hint: Option<String>,
    pub context: Option<String>,
}

/// What the warning would have said every five minutes from `from` to
/// `to`, as the history stood each time, one a change of words.
pub fn replay(
    history: &History,
    budgets: &Budgets,
    zone: &Zone,
    from: f64,
    to: f64,
) -> Result<Vec<Said>, String> {
    let mut changes: Vec<Said> = Vec::new();
    let mut before = None;
    let mut at = from;
    while at <= to {
        let tokens = read(history, budgets, at, Rate::Replay)?
            .and_then(|inputs| outlook(&inputs, before, zone, at));
        before = tokens.as_ref().map(|tokens| tokens.level);
        let said = Said {
            at,
            level: before,
            hint: tokens.as_ref().map(|tokens| tokens.hint.clone()),
            context: tokens.as_ref().map(|tokens| tokens.context.clone()),
        };
        let same = changes.last().is_some_and(|last| {
            (&last.level, &last.hint, &last.context) == (&said.level, &said.hint, &said.context)
        });
        if !same {
            changes.push(said);
        }
        at += 300.0;
    }
    Ok(changes)
}

#[cfg(test)]
mod tests;
