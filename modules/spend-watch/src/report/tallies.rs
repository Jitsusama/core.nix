//! What each pi session and each quest's family spent, as figures ready
//! to draw beside them, for the shell's rows rather than a report.

use super::Period;
use crate::money::dollars;
use crate::picture::Picture;
use serde::Serialize;
use std::collections::HashMap;

/// What each pi session has spent in a period, by its whole id, as a
/// figure ready to draw beside it, so the shell can say what a session it
/// sees costs without reading a report's names. Another tool's spend and
/// spend no log knows have no session, and a session that spent less than
/// a cent says nothing.
pub fn by_session(picture: &Picture, period: Period) -> std::collections::BTreeMap<String, String> {
    spent_by_session(picture, period)
        .into_iter()
        .map(|(session, spent)| {
            (
                session,
                crate::money::maybe_about(dollars(spent.dollars), spent.estimated),
            )
        })
        .collect()
}

/// What a pi session spent, as a figure rather than words: its dollars
/// and whether any of them was estimated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Spent {
    pub dollars: f64,
    pub estimated: bool,
}

/// What each pi session has spent in a period, by its whole id, as
/// `by_session` says it but as figures, so the launcher's Agents can add
/// up the sessions of a network, each once, and say the sum as these
/// are said (ready-rows' money), rather than adding up words. A session
/// that spent less than a cent is left out, as there.
pub fn spent_by_session(
    picture: &Picture,
    period: Period,
) -> std::collections::BTreeMap<String, Spent> {
    sessions_spent(picture, period)
        .into_iter()
        .filter(|(_, (dollars_spent, _))| *dollars_spent >= 0.005)
        .map(|(session, (dollars, estimated))| (session, Spent { dollars, estimated }))
        .collect()
}

/// What every pi session together spent in a period, said as each of
/// `by_session`'s is, and empty when it's under a cent: the same sum, so
/// the launcher's Agents says it without adding up strings, and it agrees
/// with its rows.
pub fn by_sessions(picture: &Picture, period: Period) -> String {
    let (total, estimated) = sessions_spent(picture, period)
        .into_values()
        .fold((0.0, false), |(total, estimated), (dollars_spent, its)| {
            (total + dollars_spent, estimated || its)
        });
    if total < 0.005 {
        return String::new();
    }
    crate::money::maybe_about(dollars(total), estimated)
}

/// Each pi session's spend in a period, by its whole id, and whether any
/// of it was estimated; spend that came from no session, or from another
/// machine, is no session's.
fn sessions_spent(picture: &Picture, period: Period) -> HashMap<String, (f64, bool)> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let mut spent: HashMap<String, (f64, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let entry = spent.entry(piece.session.clone()).or_insert((0.0, false));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
    }
    spent
}

/// What a pi session's helpers cost in a period: each kind of run it
/// started (its councils, judges, fleets) by its plural, costliest first,
/// by the session's whole id. A session whose spend was all its own has
/// none, and a kind under a cent is left out.
pub fn helpers_by_session(
    picture: &Picture,
    period: Period,
) -> std::collections::BTreeMap<String, Vec<Helper>> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let mut spent: HashMap<(&str, &str), (f64, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        let Some(kind) = piece.kind.strip_prefix("subagent:") else {
            continue;
        };
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let entry = spent
            .entry((piece.session.as_str(), kind))
            .or_insert((0.0, false));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
    }
    let mut by: std::collections::BTreeMap<String, Vec<(f64, Helper)>> =
        std::collections::BTreeMap::default();
    for ((session, kind), (dollars_spent, estimated)) in spent {
        if dollars_spent < 0.005 {
            continue;
        }
        by.entry(session.to_owned()).or_default().push((
            dollars_spent,
            Helper {
                on: format!("{kind}s"),
                spent: crate::money::maybe_about(dollars(dollars_spent), estimated),
            },
        ));
    }
    by.into_iter()
        .map(|(session, mut helpers)| {
            helpers.sort_by(|(one, a), (other, b)| other.total_cmp(one).then(a.on.cmp(&b.on)));
            (
                session,
                helpers.into_iter().map(|(_, helper)| helper).collect(),
            )
        })
        .collect()
}

/// What each quest spent in a period with every quest under it, at any
/// depth, by the quest's id: a quest's family, as an orchestrator's row
/// says it, since a quest's sessions orchestrate the quests under it.
/// Only this machine's: spend no log here knows, and spend on no quest,
/// is in none. A family that spent less than a cent says nothing.
pub fn by_family(picture: &Picture, period: Period) -> std::collections::BTreeMap<String, Family> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let parents: HashMap<&str, &str> = picture
        .quests
        .iter()
        .filter_map(|quest| Some((quest.id.as_str(), quest.parent.as_deref()?)))
        .collect();
    // Each quest's own and under, and whether either was estimated.
    let mut spent: HashMap<&str, (f64, f64, bool, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        let Some(quest) = piece.quest.as_deref() else {
            continue;
        };
        if piece.session == "elsewhere" {
            continue;
        }
        let estimated = piece.source.estimated() && piece.dollars > 0.005;
        let own = spent.entry(quest).or_default();
        own.0 += piece.dollars;
        own.2 |= estimated;
        // Up through its parents, each once, so a family whose record names
        // itself above itself still ends.
        let mut seen = vec![quest];
        let mut at = quest;
        while let Some(&parent) = parents.get(at) {
            if seen.contains(&parent) {
                break;
            }
            seen.push(parent);
            let above = spent.entry(parent).or_default();
            above.1 += piece.dollars;
            above.3 |= estimated;
            at = parent;
        }
    }
    let figure = |amount: f64, estimated: bool| {
        crate::money::maybe_about(dollars(amount), estimated && amount >= 0.005)
    };
    spent
        .into_iter()
        .filter(|(_, (own, under, _, _))| own + under >= 0.005)
        .map(|(quest, (own, under, own_estimated, under_estimated))| {
            (
                quest.to_owned(),
                Family {
                    all: figure(own + under, own_estimated || under_estimated),
                    own: figure(own, own_estimated),
                    under: figure(under, under_estimated),
                },
            )
        })
        .collect()
}

/// A quest's family's spend: all of it, the quest's own, and what the
/// quests under it spent, each a figure ready to draw.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Family {
    pub all: String,
    pub own: String,
    pub under: String,
}

/// One kind of run a session started, and what it cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Helper {
    pub on: String,
    pub spent: String,
}

/// How a pi session's day has gone, for the launcher's Agents: its spend
/// hour by hour as a graph the shell draws, and what rebuilding its
/// context cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionDay {
    pub graph: Spark,
    /// At list, empty when under a cent.
    pub rebuilt: String,
}

/// A history as the shell's Graph draws it: values from nothing to one,
/// the oldest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spark {
    pub shape: &'static str,
    pub values: Vec<f64>,
}

/// The most hours a spark holds, which is what the shell's Graph shows.
const SPARKED: usize = 20;

/// Each pi session's day so far, by its whole id, for those that spent a
/// cent or more today. Each hour is said over the largest hour any session
/// had today, so sparks side by side compare, and none says an amount.
pub fn today_by_session(picture: &Picture) -> std::collections::BTreeMap<String, SessionDay> {
    let today = picture.zone.day(picture.now);
    let first = crate::time::hour_of(picture.zone.day_start(today));
    let last = crate::time::hour_of(picture.now).max(first);
    let first = first.max(last + 1 - i64::try_from(SPARKED).unwrap_or(i64::MAX));
    let shown = usize::try_from(last + 1 - first).unwrap_or(0);
    let mut hours: HashMap<&str, Vec<f64>> = HashMap::new();
    for piece in picture.pieces_today() {
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let Ok(at) = usize::try_from(piece.hour - first) else {
            continue;
        };
        let spent = hours
            .entry(piece.session.as_str())
            .or_insert_with(|| vec![0.0; shown]);
        if let Some(hour) = spent.get_mut(at) {
            *hour += piece.dollars;
        }
    }
    let spent = by_session(picture, Period::Today);
    let top = hours.values().flatten().copied().fold(0.0_f64, f64::max);
    spent
        .into_keys()
        .map(|session| {
            // A session whose spend today came before the hours shown reads
            // as flat, not as no graph.
            let values = hours.get(session.as_str()).map_or_else(
                || vec![0.0; shown],
                |spent| {
                    spent
                        .iter()
                        .map(|hour| {
                            if top > 0.0 {
                                (hour / top).clamp(0.0, 1.0)
                            } else {
                                0.0
                            }
                        })
                        .collect()
                },
            );
            let rebuilt = picture
                .rebuilt_today
                .get(&session)
                .copied()
                .filter(|rebuilt| *rebuilt >= 0.005)
                .map_or_else(String::new, dollars);
            let day = SessionDay {
                graph: Spark {
                    shape: "spark",
                    values,
                },
                rebuilt,
            };
            (session, day)
        })
        .collect()
}
