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

use super::card::{Kept, owner_or_none};
use super::lines::figure;
use super::measures::{Class, FINDINGS, Measure};
use super::owing::by_owner;
use crate::DAY;

/// How long a measure stays out of spec before the card names it.
const RUNNING: i64 = 3 * DAY;

/// How far from the hour its timer may fire, so a run three days old but
/// for a few minutes is named on that hour rather than the next.
const SLACK: i64 = 10 * 60;

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
/// longer reads can't stay out for good. Two measures under one key are
/// one run, out while either is. A clock set back can't leave a run begun
/// or seen after `now`, which would never be named nor let go.
pub fn track(out: &mut BTreeMap<String, Out>, measures: &[Measure], now: i64) {
    let mut judged: BTreeMap<&str, bool> = BTreeMap::new();
    for m in measures.iter().filter(|m| m.target.is_some()) {
        *judged.entry(m.key.as_str()).or_default() |= m.past().is_some();
    }
    for (key, past) in judged {
        if past {
            out.entry(key.to_owned())
                .and_modify(|run| {
                    run.since = run.since.min(now);
                    run.seen = now;
                })
                .or_insert(Out {
                    since: now,
                    seen: now,
                });
        } else {
            out.remove(key);
        }
    }
    out.retain(|_, run| run.seen > now - DAY && run.seen <= now);
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
pub fn named(out: &BTreeMap<String, Out>, measures: &[Measure], now: i64, tz: &TimeZone) -> String {
    let mut lines = String::new();
    let due = now - RUNNING + SLACK;
    for m in measures {
        let (Some(target), Some(_)) = (m.target, m.past()) else {
            continue;
        };
        let Some(run) = out.get(&m.key).filter(|run| run.since <= due) else {
            continue;
        };
        let began = date(run.since, tz);
        let _ = writeln!(
            lines,
            "  - {}: {} against at most {}, out since {began}, {}",
            m.says,
            figure(m.now),
            figure(target),
            whose(m),
        );
    }
    if lines.is_empty() {
        return lines;
    }
    format!("Out of spec three days running, each with its owner:\n{lines}")
}

/// Who owns `m`, as its drag would be filed: each miss or finding by its
/// own owner, a clean or cell measure's lines by the owners they name, and
/// any other by the measure's owner.
fn whose(m: &Measure) -> String {
    let owners = by_owner(m);
    if m.class == Class::Miss {
        "each miss's owner".to_owned()
    } else if m.key == FINDINGS {
        "each finding's owner".to_owned()
    } else if owners.is_empty() {
        owner_or_none(&m.owner).to_owned()
    } else {
        owners.into_keys().collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
mod tests;
