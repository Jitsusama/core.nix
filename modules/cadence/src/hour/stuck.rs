//! The measures out of spec three days running, each with its owner, named
//! on the card once a day. Joel asked that nothing be out of spec in the
//! improvement loop; a measure the drag never reaches can stay out for
//! days unseen, so the hour keeps each one's unbroken run out of its
//! target and says which have run three days.

use std::collections::BTreeMap;
use std::fmt::Write;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use super::card::Kept;
use super::lines::figure;
use super::measures::Measure;
use crate::DAY;

/// How long a measure stays out of spec before the card names it.
const RUNNING: i64 = 3 * DAY;

/// A measure's run out of its target: when the hour first judged it out,
/// and when it last did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Out {
    pub since: i64,
    pub seen: i64,
}

/// Carries each measure's run out of spec to `now`. One judged past its
/// target starts a run or goes on with it; one judged within its target
/// ends its run. One the hour couldn't judge, or didn't read, leaves its
/// run as it was, since an hour that couldn't look says nothing of it;
/// though one unjudged for a whole day is let go, so a measure the card no
/// longer reads can't stay out for good.
pub fn track(out: &mut BTreeMap<String, Out>, measures: &[Measure], now: i64) {
    for m in measures {
        if m.target.is_none() {
            continue;
        }
        if m.past().is_some() {
            out.entry(m.key.clone())
                .and_modify(|run| run.seen = now)
                .or_insert(Out {
                    since: now,
                    seen: now,
                });
        } else {
            out.remove(&m.key);
        }
    }
    out.retain(|_, run| run.seen > now - DAY);
}

/// Carries `kept`'s runs to `now`, and names on `card` the measures out
/// three days running, on the day's first card that has one. The day is
/// kept once they are named, so a measure that turns three days later
/// that day is named the next.
pub fn daily(card: &mut String, kept: &mut Kept, measures: &[Measure], now: i64, tz: &TimeZone) {
    track(&mut kept.out, measures, now);
    let day = date(now, tz);
    if kept.out_told.as_deref() == Some(day.as_str()) {
        return;
    }
    let lines = named(&kept.out, measures, now, tz);
    if !lines.is_empty() {
        card.push_str(&lines);
        kept.out_told = Some(day);
    }
}

/// The day `at` falls on where `tz` says, as `2026-11-14`.
fn date(at: i64, tz: &TimeZone) -> String {
    Timestamp::from_second(at)
        .map(|at| at.to_zoned(tz.clone()).date().to_string())
        .unwrap_or_default()
}

/// The card's lines naming each measure judged out this hour that has
/// been out three days running, with its owner, or none when no measure
/// has. Dates are the day each run began, where `tz` says.
#[must_use]
pub fn named(
    out: &BTreeMap<String, Out>,
    measures: &[Measure],
    now: i64,
    tz: &TimeZone,
) -> String {
    let mut lines = String::new();
    for m in measures {
        let (Some(target), Some(_)) = (m.target, m.past()) else {
            continue;
        };
        let Some(run) = out.get(&m.key).filter(|run| run.since <= now - RUNNING) else {
            continue;
        };
        let began = date(run.since, tz);
        let owner = if m.owner.is_empty() {
            "no owner"
        } else {
            m.owner.as_str()
        };
        let _ = writeln!(
            lines,
            "  - {}: {} against at most {}, out since {began}, {owner}",
            m.says,
            figure(m.now),
            figure(target),
        );
    }
    if lines.is_empty() {
        return lines;
    }
    format!("Out of spec three days running, each with its owner:\n{lines}")
}

#[cfg(test)]
mod tests;
