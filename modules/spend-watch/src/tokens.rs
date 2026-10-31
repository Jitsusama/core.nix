//! When the tokens run out: the one warning, written into status.json for
//! the shell to raise, of when a cap that refuses the account is reached at
//! the last hour's rate, and when it's back under.
//!
//! A cap like the proxy's is enforced by a job that runs about hourly. It
//! counts the week's spend, and at the first run that finds it at the cap,
//! the account is refused until a later run finds it back under. So the
//! tokens run out at a run, never between, and the warning names that run.
//! The count a run makes trails the bill by some hours, so from one reading
//! to the next it moves by about what's spent in an hour less what rolls
//! out of the week, which is what's projected forward.
//!
//! The levels are by the time left, since what Joel can do (claim a raise,
//! pause the session spending most) takes minutes, and a run is an hour
//! apart: under four hours ("hours"), time to slow down; under an hour
//! ("hour"), the next run or the one after; under fifteen minutes
//! ("minutes"), the next run; and refused ("out"). Each level reached is a
//! new notice, and nothing else is: a level is left only once the time
//! left passes its own bound by half an hour, so a rate that wavers can't
//! raise it again and again.

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
}

/// The cap to watch: the one with an allowance over the longest window,
/// since it's the one a refusal lasts longest under.
fn watched(caps: Vec<Cap>) -> Option<Cap> {
    caps.into_iter()
        .filter(|cap| cap.allowance.is_some_and(|allowance| allowance > 0.0))
        .max_by_key(|cap| cap.window_hours)
}

fn billed(history: &History, source: &str, from: i64, to: i64) -> Result<Vec<f64>, String> {
    let mut out = vec![0.0; usize::try_from(to - from).unwrap_or(0)];
    let mut statement = history
        .db
        .prepare(
            "SELECT hour, SUM(dollars) FROM billed WHERE source = ?1 AND hour >= ?2
             AND hour < ?3 GROUP BY hour",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map(rusqlite::params![source, from, to], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(fail)?;
    for row in rows {
        let (hour, dollars) = row.map_err(fail)?;
        if let Some(slot) = usize::try_from(hour - from)
            .ok()
            .and_then(|i| out.get_mut(i))
        {
            *slot = dollars;
        }
    }
    Ok(out)
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
pub fn read(history: &History, now: f64, rate: Rate) -> Result<Option<Inputs>, String> {
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
    let hour = hour_of(now);
    let into = (now - hour_start(hour)) / HOUR;
    let rate = match rate {
        Rate::Live => {
            let meter = billed(history, "meter", hour - 1, hour + 1)?;
            let before = if meter[0] > 0.0 {
                meter[0]
            } else {
                billed(history, "warehouse", hour - 1, hour)?[0]
            };
            meter[1] + before * (1.0 - into)
        }
        Rate::Replay => {
            let last = billed(history, "warehouse", hour - 1, hour)?[0];
            if last > 0.0 {
                last
            } else {
                billed(history, "meter", hour - 1, hour)?[0]
            }
        }
    };
    let window = i64::from(cap.window_hours);
    let rolling = billed(
        history,
        "warehouse",
        hour - window,
        hour - window + i64::try_from(AHEAD).unwrap_or(0) + 1,
    )?;
    let top = history
        .db
        .query_row(
            "SELECT session, quest FROM turn_hours WHERE hour >= ?1 AND hour <= ?2
             GROUP BY session ORDER BY SUM(cost) DESC LIMIT 1",
            [hour - 1, hour],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .ok();
    Ok(Some(Inputs {
        cap,
        rate,
        rolling,
        top,
    }))
}

/// Where the count stands at each run from the cap's reading on: the
/// reading, then each run an hour after the one before, moved by the
/// rate less what rolls out in the hour it ends in, spending nothing
/// after the run `stop`.
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
        let spent = if stop.is_some_and(|stop| run > stop) {
            0.0
        } else {
            inputs.rate
        };
        count = (count + spent - rolled).max(0.0);
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
    } else {
        let five = ((seconds / 60.0 / 5.0).round() * 5.0).max(5.0);
        format!("About {five} Min")
    }
}

/// The start of the keys of the watched cap's findings, which the warning
/// speaks for.
#[must_use]
pub fn key(inputs: &Inputs) -> String {
    format!("cap:{}:{}:", inputs.cap.feed, inputs.cap.name)
}

/// The outlook now, given the level said before, if any.
#[must_use]
pub fn outlook(inputs: &Inputs, before: Option<Level>, zone: &Zone, now: f64) -> Option<Tokens> {
    let cap = &inputs.cap;
    let allowance = cap.allowance?;
    let used = cap.spent / allowance;
    let key = key(inputs);
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
    let percent = format!("{:.0}% used", used * 100.0);
    let over = cap.state == "refused" || cap.spent >= allowance;
    if !over && now - cap.at > STALE {
        if used < UNREAD_FROM {
            return None;
        }
        let since = format!("can't be read since {}", zone.clock(cap.at, now));
        return Some(Tokens {
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
            actions: acts,
            unread: Some(since),
            runs_out: None,
            back: None,
            cap: key,
        });
    }
    // The run at which it's reached, and the first after it back under if
    // nothing more is spent from then.
    let ahead = counts(inputs, now, None);
    let reached = if over {
        Some(0)
    } else {
        ahead
            .iter()
            .position(|&(at, count)| at > cap.at && count >= allowance)
    }?;
    let runs_out = ahead[reached].0;
    let back = counts(inputs, now, Some(reached))
        .into_iter()
        .skip(reached + 1)
        .find(|&(_, count)| count < allowance)
        .map(|(at, _)| at);
    let left = (runs_out - now).max(0.0);
    let level = if over {
        Level::Out
    } else {
        let fresh = [Level::Minutes, Level::Hour, Level::Hours]
            .into_iter()
            .find(|level| left < level.under());
        // A level said before is kept until the time left rises past its
        // own bound by half an hour, so a wavering rate doesn't raise it
        // again.
        let held = before
            .filter(|held| matches!(held, Level::Minutes | Level::Hour | Level::Hours))
            .filter(|held| left < held.under() + LEAVE);
        match (fresh, held) {
            (Some(fresh), Some(held)) => fresh.min(held),
            (fresh, held) => fresh.or(held)?,
        }
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
                "Tokens run out in {} at this hour's rate{then}",
                minutes(left).to_lowercase()
            ),
        )
    };
    let rate = if over {
        percent.clone()
    } else {
        format!("at {} an hour · {percent}", whole(inputs.rate))
    };
    let quiet_rate = if over {
        percent
    } else {
        format!("at this hour's rate · {percent}")
    };
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
        actions: acts,
        unread: None,
        runs_out: Some(runs_out),
        back,
        cap: key,
    })
}

/// What the warning would have said every five minutes from `from` to
/// `to`, as the history stood each time, one line a change of words.
pub fn replay(history: &History, zone: &Zone, from: f64, to: f64) -> Result<String, String> {
    let mut out = String::new();
    let mut before = None;
    let mut said = String::new();
    let mut at = from;
    while at <= to {
        let tokens =
            read(history, at, Rate::Replay)?.and_then(|inputs| outlook(&inputs, before, zone, at));
        before = tokens.as_ref().map(|tokens| tokens.level);
        let line = tokens.as_ref().map_or_else(
            || "nothing to say".to_owned(),
            |tokens| {
                let level = serde_json::to_string(&tokens.level).unwrap_or_default();
                format!("{level} {}: {}", tokens.hint, tokens.context)
            },
        );
        if line != said {
            out.push_str(&format!("{} {line}\n", zone.clock(at, to)));
            said = line;
        }
        at += 300.0;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A reading on Sat 31 Oct 2026 at 05:02:34 in Toronto, and two minutes on.
    const AT: f64 = 1_793_437_354.0;
    const NOW: f64 = AT + 120.0;

    fn toronto() -> Zone {
        Zone::named("America/Toronto").expect("a zone")
    }

    fn cap(spent: f64, at: f64, state: &str) -> Cap {
        Cap {
            feed: "proxy".into(),
            name: "weekly".into(),
            label: "weekly cap".into(),
            allowance: Some(15_000.0),
            window_hours: 168,
            spent,
            warn: 0.9,
            fallback: None,
            state: state.into(),
            ends: None,
            ends_known: false,
            resets: None,
            at,
        }
    }

    fn inputs(spent: f64, rate: f64, rolled: f64) -> Inputs {
        Inputs {
            cap: cap(spent, AT, "ok"),
            rate,
            rolling: vec![rolled; AHEAD + 1],
            top: Some(("01a19269".into(), Some("QEST-20261025-5FWIUS".into()))),
        }
    }

    #[test]
    fn a_cap_far_off_says_nothing() {
        assert_eq!(
            outlook(&inputs(10_000.0, 300.0, 0.0), None, &toronto(), NOW),
            None
        );
    }

    #[test]
    fn it_names_the_run_the_cap_is_reached_at() {
        // $1,200 left at $300 an hour less $0 rolling out: the fourth run.
        let tokens = outlook(&inputs(13_800.0, 300.0, 0.0), None, &toronto(), NOW).unwrap();
        assert_eq!(tokens.level, Level::Hours);
        assert_eq!(tokens.runs_out, Some(AT + 4.0 * HOUR));
        assert_eq!(tokens.title, "Tokens Run Out in About 4 Hours");
        assert_eq!(
            tokens.hint,
            "Tokens run out in about 4 hours at this hour's rate"
        );
        assert_eq!(tokens.context, "at $300 an hour · 92% used");
        assert!(!tokens.quiet.context.contains('$'));
        assert!(!tokens.now);
    }

    #[test]
    fn what_rolls_out_slows_it() {
        // $1,200 left at $300 an hour less $100 rolling out: the sixth run.
        let tokens = outlook(&inputs(13_800.0, 300.0, 100.0), None, &toronto(), NOW);
        assert_eq!(tokens, None, "six hours off is past the first level");
        let tokens = outlook(&inputs(14_200.0, 300.0, 100.0), None, &toronto(), NOW).unwrap();
        assert_eq!(tokens.runs_out, Some(AT + 4.0 * HOUR));
        // Spending stopped then, $100 rolls out by the first run after.
        assert_eq!(tokens.back, Some(AT + 5.0 * HOUR));
        assert_eq!(tokens.value, "resets 10:02");
        assert_eq!(
            tokens.hint,
            "Tokens run out in about 4 hours at this hour's rate; they reset at 10:02"
        );
    }

    #[test]
    fn the_next_run_is_pressing() {
        let now = AT + HOUR - 600.0;
        let tokens = outlook(&inputs(14_800.0, 300.0, 0.0), None, &toronto(), now).unwrap();
        assert_eq!(tokens.level, Level::Minutes);
        assert!(tokens.now);
        assert_eq!(tokens.title, "Tokens Run Out in About 10 Min");
        assert_eq!(tokens.actions[1].name, "Pause 5FWIUS");
        assert_eq!(tokens.actions[1].session.as_deref(), Some("01a19269"));
    }

    #[test]
    fn a_wavering_rate_keeps_the_level_it_reached() {
        // Under an hour left, then a lull puts it at the second run, 70
        // minutes off: still "hour", since it's not yet past 90 minutes.
        let given = inputs(14_850.0, 75.0, 0.0);
        let now = AT + 3000.0;
        let fresh = outlook(&given, None, &toronto(), now).unwrap();
        assert_eq!(fresh.level, Level::Hours);
        let held = outlook(&given, Some(Level::Hour), &toronto(), now).unwrap();
        assert_eq!(held.level, Level::Hour);
        // 110 minutes off is past it, so it's left.
        let left = outlook(&given, Some(Level::Hour), &toronto(), AT + 600.0).unwrap();
        assert_eq!(left.level, Level::Hours);
        // And "hours" holds a little past four hours: the fifth run, 4 h 20
        // min off.
        let far = inputs(13_740.0, 300.0, 0.0);
        assert_eq!(outlook(&far, None, &toronto(), AT + 2400.0), None);
        let kept = outlook(&far, Some(Level::Hours), &toronto(), AT + 2400.0).unwrap();
        assert_eq!(kept.level, Level::Hours);
    }

    #[test]
    fn refused_it_says_when_it_is_back() {
        let mut given = inputs(15_144.0, 300.0, 200.0);
        given.cap.state = "refused".into();
        let tokens = outlook(&given, None, &toronto(), NOW).unwrap();
        assert_eq!(tokens.level, Level::Out);
        assert!(tokens.now);
        assert_eq!(tokens.title, "Tokens Ran Out");
        assert_eq!(tokens.back, Some(AT + HOUR));
        assert_eq!(tokens.hint, "Tokens ran out; they reset at 06:02");
    }

    #[test]
    fn a_reading_gone_stale_near_the_cap_is_said_in_the_row() {
        let tokens = outlook(
            &inputs(14_000.0, 300.0, 0.0),
            None,
            &toronto(),
            AT + 3.0 * HOUR,
        )
        .unwrap();
        assert_eq!(tokens.level, Level::Unread);
        assert_eq!(tokens.unread.as_deref(), Some("can't be read since 05:02"));
        assert_eq!(tokens.quiet.unread, tokens.unread);
        assert!(!tokens.now);
        assert_eq!(
            outlook(
                &inputs(9_000.0, 300.0, 0.0),
                None,
                &toronto(),
                AT + 3.0 * HOUR
            ),
            None
        );
    }

    #[test]
    fn it_reads_the_meter_and_what_rolls_out() {
        let history = History::memory();
        history
            .db
            .execute_batch(&format!(
                "INSERT INTO caps VALUES ('proxy', 'weekly', {at}, 'weekly cap', 15000, 168,
                 14000, 0.9, NULL, 'ok', NULL, 0, NULL);
                 INSERT INTO caps VALUES ('proxy', 'premium', {at}, 'premium day', 1000, 24,
                 0, 0.5, NULL, 'ok', NULL, 0, NULL);",
                at = AT
            ))
            .unwrap();
        let hour = hour_of(AT);
        let now = hour_start(hour) + 1800.0;
        let bill = |source: &str, hour: i64, dollars: f64| {
            history
                .db
                .execute(
                    "INSERT INTO billed VALUES ('proxy', ?1, ?2, 'pi', 'm', 'v', 's', 'a', 't',
                     0, 0, 0, 0, ?3, 0, 0)",
                    rusqlite::params![source, hour, dollars],
                )
                .unwrap();
        };
        bill("meter", hour, 150.0);
        bill("meter", hour - 1, 240.0);
        bill("warehouse", hour - 168, 90.0);
        let read = read(&history, now, Rate::Live).unwrap().unwrap();
        assert_eq!(read.cap.name, "weekly");
        // The half hour so far and the later half of the hour before.
        assert!((read.rate - 270.0).abs() < 1e-9);
        assert!((read.rolling[0] - 90.0).abs() < 1e-9);
    }
}
