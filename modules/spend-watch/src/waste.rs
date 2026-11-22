//! Where money bought nothing, from pi's logs: cache written to rebuild
//! context a session already had, by cause, and each session's cost per
//! unit of work beside its peers'. These are at pi's list prices, since
//! only the logs know a turn's cache; the ratio of bill to list scales
//! them when there's a bill.

use crate::history::{History, sql as fail};
use crate::stats;
use crate::time::{hour_of, hour_start};
use rusqlite::{OptionalExtension, ToSql};
use std::collections::HashMap;

/// A span's turns as the figures here sum them: its whole hours from
/// `waste_hours`, and the ragged edges either side from the turns
/// themselves, so any bounds read the very turns that summing them would,
/// with sums alike but for rounding. Where the zone is a whole number of
/// hours from UTC a day's start is a whole hour, so most spans read only
/// their last, unfinished hour from the turns. Each edge is its own
/// branch, so each is a range on `turns_at` alone.
const SPAN: &str = "WITH span AS (
    SELECT session, quest, model, kind, cause, turns, cost, cost_cache_write, rebuilt_cost,
        cache_read, prompt FROM waste_hours WHERE hour >= :first AND hour < :last
    UNION ALL
    SELECT session, quest, model, kind, cause, 1, cost, cost_cache_write, rebuilt_cost,
        cache_read, input + cache_read + cache_write FROM turns
        WHERE at >= :from AND at < :start
    UNION ALL
    SELECT session, quest, model, kind, cause, 1, cost, cost_cache_write, rebuilt_cost,
        cache_read, input + cache_read + cache_write FROM turns
        WHERE at >= :end AND at < :to)";

/// From `from` to before `to`, split as `SPAN` reads it.
struct Span {
    from: f64,
    to: f64,
    /// The first whole hour, and the hour after the last.
    first: i64,
    last: i64,
    /// Where the whole hours start and end; `to` itself when there are none.
    start: f64,
    end: f64,
}

impl Span {
    fn of(from: f64, to: f64) -> Span {
        let first = hour_of(from) + i64::from(hour_start(hour_of(from)) < from);
        let last = hour_of(to);
        if first < last {
            Span {
                from,
                to,
                first,
                last,
                start: hour_start(first),
                end: hour_start(last),
            }
        } else {
            Span {
                from,
                to,
                first: 0,
                last: 0,
                start: to,
                end: to,
            }
        }
    }

    /// The span's bounds, by the names `SPAN` gives them.
    fn named(&self) -> [(&'static str, &dyn ToSql); 6] {
        [
            (":from", &self.from),
            (":to", &self.to),
            (":first", &self.first),
            (":last", &self.last),
            (":start", &self.start),
            (":end", &self.end),
        ]
    }
}

/// `select`, reading a span's turns as the table `span`.
fn over(select: &str) -> String {
    format!("{SPAN} {select}")
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Waste {
    pub spent: f64,
    pub cache_write: f64,
    pub rebuilt: f64,
    /// Rebuilt dollars by cause, largest first.
    pub causes: Vec<(String, f64)>,
    /// The session that rebuilt most after pauses, with its dollars.
    pub worst: Option<(String, f64)>,
}

/// The share of a span's spend that went on rebuilding context a session
/// already had, counted as `waste` counts it, or none if it spent nothing.
pub fn share(history: &History, from: f64, to: f64) -> Result<Option<f64>, String> {
    let (spent, rebuilt): (f64, f64) = history
        .db
        .query_row(
            &over(
                "SELECT COALESCE(SUM(cost),0), COALESCE(SUM(CASE WHEN cause IS NOT NULL
                 AND cause != 'fresh' THEN rebuilt_cost ELSE 0 END),0) FROM span",
            ),
            &Span::of(from, to).named()[..],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(fail)?;
    Ok((spent >= 1.0).then(|| rebuilt / spent.max(rebuilt)))
}

/// Dollars under this are what a kept sum's rounding leaves of nothing,
/// so a figure that names a session or a cause passes over them.
const ROUNDING: f64 = 1e-9;

/// What each session spent rebuilding context it already had in a span,
/// by its whole id, counted as `waste` counts it: a session that rebuilt
/// nothing isn't named.
pub fn rebuilt_by_session(
    history: &History,
    from: f64,
    to: f64,
) -> Result<HashMap<String, f64>, String> {
    let mut statement = history
        .db
        .prepare(&over(
            "SELECT session, SUM(rebuilt_cost) FROM span
             WHERE cause IS NOT NULL AND cause != 'fresh' GROUP BY session",
        ))
        .map_err(fail)?;
    statement
        .query_map(&Span::of(from, to).named()[..], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(fail)?
        .filter(|row| {
            row.as_ref()
                .map_or(true, |(_, dollars)| *dollars > ROUNDING)
        })
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(fail)
}

pub fn waste(history: &History, from: f64, to: f64) -> Result<Waste, String> {
    let db = &history.db;
    let span = Span::of(from, to);
    let (spent, cache_write, rebuilt): (f64, f64, f64) = db
        .query_row(
            &over(
                "SELECT COALESCE(SUM(cost),0), COALESCE(SUM(cost_cache_write),0),
                 COALESCE(SUM(rebuilt_cost),0) FROM span",
            ),
            &span.named()[..],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(fail)?;
    let mut statement = db
        .prepare(&over(
            "SELECT cause, SUM(rebuilt_cost) FROM span
             WHERE cause IS NOT NULL AND cause != 'fresh' GROUP BY cause ORDER BY 2 DESC",
        ))
        .map_err(fail)?;
    let causes: Vec<(String, f64)> = statement
        .query_map(&span.named()[..], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?
        .into_iter()
        .filter(|cause| cause.1 > ROUNDING)
        .collect();
    let worst = db
        .query_row(
            &over(
                "SELECT session, SUM(rebuilt_cost) FROM span
                 WHERE cause = 'gap' GROUP BY session ORDER BY 2 DESC LIMIT 1",
            ),
            &span.named()[..],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
        )
        .optional()
        .map_err(fail)?
        .filter(|worst| worst.1 > ROUNDING);
    Ok(Waste {
        spent,
        cache_write,
        rebuilt: causes.iter().map(|cause| cause.1).sum::<f64>().min(rebuilt),
        causes,
        worst,
    })
}

/// One session's work, for comparing it with its peers.
#[derive(Debug, Clone, PartialEq)]
pub struct Work {
    pub session: String,
    pub quest: Option<String>,
    pub turns: f64,
    pub dollars: f64,
    pub per_turn: f64,
    /// The share of its input that came from cache.
    pub cached: f64,
    pub rebuilt_after_gaps: f64,
}

pub fn work(history: &History, from: f64, to: f64) -> Result<Vec<Work>, String> {
    let mut statement = history
        .db
        .prepare(&over(
            "SELECT session, MAX(quest), SUM(turns), SUM(cost), SUM(cache_read), SUM(prompt),
             SUM(CASE WHEN cause = 'gap' THEN rebuilt_cost ELSE 0 END)
             FROM span WHERE kind = 'agent' GROUP BY session",
        ))
        .map_err(fail)?;
    let rows = statement
        .query_map(&Span::of(from, to).named()[..], |row| {
            // A count of turns is an integer, which rusqlite reads as an
            // f64 just as a cast would.
            let turns: f64 = row.get(2)?;
            let dollars: f64 = row.get(3)?;
            let read: f64 = row.get(4)?;
            let prompt: f64 = row.get(5)?;
            Ok(Work {
                session: row.get(0)?,
                quest: row.get(1)?,
                turns,
                dollars,
                per_turn: dollars / turns.max(1.0),
                cached: if prompt > 0.0 { read / prompt } else { 0.0 },
                rebuilt_after_gaps: row.get(6)?,
            })
        })
        .map_err(fail)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(fail)
}

/// A session costing far more per turn than its peers, after shrinking
/// each toward the usual by how few turns it has (empirical Bayes), so a
/// session of three expensive turns isn't flagged on noise. Each session
/// gets a p-value from its robust z, and Benjamini-Hochberg holds the
/// false discovery rate across all of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Outlier {
    pub work: Work,
    /// How many times the usual cost per turn.
    pub times: f64,
    pub usual_cached: f64,
    pub excess: f64,
    pub p: f64,
}

/// Turns' worth of the usual a session's figure is shrunk toward.
const PRIOR_TURNS: f64 = 20.0;

pub fn outliers(sessions: &[Work], false_discovery: f64, floor: f64) -> Vec<Outlier> {
    let kept: Vec<&Work> = sessions
        .iter()
        .filter(|work| work.turns >= 5.0 && work.dollars > 0.0)
        .collect();
    if kept.len() < 8 {
        return Vec::new();
    }
    let logs: Vec<f64> = kept.iter().map(|work| work.per_turn.ln()).collect();
    let centre = stats::median(&logs);
    let scale = stats::mad(&logs).max(0.05);
    let usual_cached = stats::median(&kept.iter().map(|work| work.cached).collect::<Vec<_>>());
    let shrunk: Vec<f64> = kept
        .iter()
        .zip(&logs)
        .map(|(work, log)| {
            let weight = work.turns / (work.turns + PRIOR_TURNS);
            centre + weight * (log - centre)
        })
        .collect();
    let p: Vec<f64> = shrunk
        .iter()
        .map(|log| stats::upper_tail((log - centre) / scale))
        .collect();
    let found = stats::discoveries(&p, false_discovery);
    let mut out: Vec<Outlier> = kept
        .iter()
        .enumerate()
        .filter(|(index, _)| found[*index])
        .map(|(index, work)| {
            let usual = centre.exp();
            Outlier {
                work: (*work).clone(),
                times: work.per_turn / usual,
                usual_cached,
                excess: (work.per_turn - usual) * work.turns,
                p: p[index],
            }
        })
        .filter(|outlier| outlier.excess >= floor)
        .collect();
    out.sort_by(|one, other| other.excess.total_cmp(&one.excess));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(session: &str, turns: f64, per_turn: f64) -> Work {
        Work {
            session: session.into(),
            quest: None,
            turns,
            dollars: turns * per_turn,
            per_turn,
            cached: 0.85,
            rebuilt_after_gaps: 0.0,
        }
    }

    /// A turn of the fixture: who, when, and what it cost.
    #[derive(Clone)]
    struct Kept {
        session: String,
        quest: Option<String>,
        model: &'static str,
        kind: &'static str,
        cause: Option<&'static str>,
        at: f64,
        cost: f64,
        cache_read: f64,
    }

    impl Kept {
        fn wasted(&self) -> bool {
            self.cause.is_some_and(|cause| cause != "fresh")
        }
    }

    /// The first fixture turn's instant, 40 minutes into its hour.
    const BASE: f64 = 1_763_700_000.0;

    /// Turns over three days at ragged instants, of every cause and kind,
    /// whose costs are sixteenths so every sum is exact, and one with no
    /// quest on each bound the spans below are cut at, which decides
    /// whether a bound is counted once and on its own side, the edges of
    /// a span's whole hours among them. Every tenth
    /// has a twin a minute on, so an hour kept holds more than one turn
    /// and its count is a sum of counts, not a count of rows.
    fn kept() -> Vec<Kept> {
        const CAUSES: [Option<&str>; 4] = [None, Some("fresh"), Some("gap"), Some("compaction")];
        let mut turns: Vec<Kept> = (0..300_u32)
            .map(|i| Kept {
                session: format!("s{}", i % 5),
                quest: Some(format!("q{}", i % 3)),
                model: if i % 2 == 0 { "opus" } else { "sonnet" },
                kind: if i % 7 == 0 { "side" } else { "agent" },
                cause: CAUSES[usize::try_from(i % 4).expect("under four")],
                at: BASE + f64::from(i) * 997.3,
                cost: 0.25 * f64::from(1 + i % 7),
                cache_read: f64::from(i % 11),
            })
            .collect();
        let twins: Vec<Kept> = turns
            .iter()
            .step_by(10)
            .map(|turn| Kept {
                at: turn.at + 60.0,
                ..turn.clone()
            })
            .collect();
        turns.extend(twins);
        let whole = hour_start(hour_of(BASE) + 1);
        let bounds = [
            whole,
            whole + 30.0 * crate::time::HOUR,
            BASE + 1234.5,
            BASE + 400_000.0,
            // Where the first span's whole hours end, and just before the
            // second span's begin.
            hour_start(hour_of(BASE + 400_000.0)),
            whole + crate::time::HOUR - 0.5,
        ];
        turns.extend((0_u32..).zip(bounds).map(|(i, at)| Kept {
            session: format!("s{}", i % 5),
            quest: None,
            model: "opus",
            kind: "agent",
            cause: Some("gap"),
            at,
            cost: 2.0 + 0.5 * f64::from(i),
            cache_read: 1.0,
        }));
        turns
    }

    fn stored(turns: &[Kept]) -> History {
        let history = History::memory();
        for turn in turns {
            history
                .db
                .execute(
                    "INSERT INTO turns (session, at, hour, model, provider, kind, quest, input,
                     output, cache_read, cache_write, cache_write_long, cost, cost_cache_write,
                     cost_output, gap, rebuilt, rebuilt_cost, cause) VALUES
                     (?1, ?2, ?3, ?4, 'anthropic', ?5, ?6, 3, 1, ?7, 5, 0, ?8, ?8 / 4, 0, NULL,
                     0, ?8 / 2, ?9)",
                    rusqlite::params![
                        turn.session,
                        turn.at,
                        hour_of(turn.at),
                        turn.model,
                        turn.kind,
                        turn.quest,
                        turn.cache_read,
                        turn.cost,
                        turn.cause,
                    ],
                )
                .expect("a turn is kept");
        }
        history
    }

    /// What summing the turns themselves says of each figure, for spans
    /// on whole hours, ragged at either end, inside one hour and empty.
    #[test]
    fn every_span_reads_what_its_turns_say() {
        let turns = kept();
        let history = stored(&turns);
        let hour = crate::time::HOUR;
        let whole = hour_start(hour_of(BASE) + 1);
        let spans = [
            (BASE - 100.0, BASE + 400_000.0),
            (BASE + 1234.5, BASE + 2.0 * crate::time::DAY + 17.0),
            (whole, whole + 30.0 * hour),
            (whole + 60.0, whole + 2000.0),
            (whole - 900.0, whole + 900.0),
            (whole, whole),
        ];
        for (from, to) in spans {
            let within: Vec<&Kept> = turns
                .iter()
                .filter(|turn| turn.at >= from && turn.at < to)
                .collect();
            waste_holds(&history, from, to, &within);
            let by_session = summed(&within, |turn| turn.session.clone());
            assert_eq!(
                rebuilt_by_session(&history, from, to).expect("the sessions read"),
                by_session.into_iter().collect::<HashMap<_, _>>(),
                "the span from {from} to {to} gave other sessions' rebuilding"
            );
            let mut sessions = super::work(&history, from, to).expect("the work reads");
            sessions.sort_by(|one, other| one.session.cmp(&other.session));
            assert_eq!(
                sessions,
                agents(&within),
                "the span from {from} to {to} saw other work"
            );
        }
    }

    /// The rebuilt dollars among `within` that `waste` counts, summed by
    /// `by`, in its order.
    fn summed(within: &[&Kept], by: impl Fn(&Kept) -> String) -> Vec<(String, f64)> {
        let mut sums: HashMap<String, f64> = HashMap::new();
        for &turn in within.iter().filter(|turn| turn.wasted()) {
            *sums.entry(by(turn)).or_default() += turn.cost / 2.0;
        }
        let mut sums: Vec<(String, f64)> = sums.into_iter().collect();
        sums.sort_by(|one, other| one.0.cmp(&other.0));
        sums
    }

    /// `waste` and `share` over a span, against its turns `within`.
    fn waste_holds(history: &History, from: f64, to: f64, within: &[&Kept]) {
        let spent: f64 = within.iter().map(|turn| turn.cost).sum();
        let rebuilt: f64 = within.iter().map(|turn| turn.cost / 2.0).sum();
        let causes = summed(within, |turn| turn.cause.unwrap_or_default().into());
        let wasted: f64 = causes.iter().map(|cause| cause.1).sum();
        let said = waste(history, from, to).expect("the waste reads");
        assert_eq!(
            (said.spent, said.cache_write, said.rebuilt),
            (spent, spent / 4.0, wasted.min(rebuilt)),
            "the span from {from} to {to} summed other than its turns"
        );
        let mut said_causes = said.causes.clone();
        said_causes.sort_by(|one, other| one.0.cmp(&other.0));
        assert_eq!(
            said_causes, causes,
            "the span from {from} to {to} put rebuilding under other causes"
        );
        let gaps: Vec<&Kept> = within
            .iter()
            .copied()
            .filter(|turn| turn.cause == Some("gap"))
            .collect();
        let gaps = summed(&gaps, |turn| turn.session.clone());
        let most = gaps.iter().map(|gap| gap.1).reduce(f64::max);
        // Sessions can tie, so the one named need only be among the worst.
        assert!(
            said.worst.as_ref().map(|worst| worst.1) == most
                && said
                    .worst
                    .as_ref()
                    .is_none_or(|worst| gaps.contains(&(worst.0.clone(), worst.1))),
            "the span from {from} to {to} named {:?} the worst gap, of {gaps:?}",
            said.worst
        );
        assert_eq!(
            share(history, from, to).expect("the share reads"),
            (spent >= 1.0).then(|| wasted / spent.max(wasted)),
            "the span from {from} to {to} gave another share"
        );
    }

    /// What forgetting turns leaves of an hour's rebuilding is a
    /// rounding's worth of nothing, not nothing, and summing the turns
    /// left would name no session for it, so the kept sums don't either.
    #[test]
    fn a_rounding_left_by_forgotten_turns_names_no_session() {
        let whole = hour_start(hour_of(BASE) + 1);
        let turn = |at: f64, cost: f64| Kept {
            session: "s0".into(),
            quest: None,
            model: "opus",
            kind: "agent",
            cause: Some("gap"),
            at: whole + at,
            cost,
            cache_read: 0.0,
        };
        let history = stored(&[turn(10.0, 0.2), turn(20.0, 0.4), turn(30.0, 0.0)]);
        history
            .forget_copied("s0", whole + 25.0)
            .expect("the turns are forgotten");
        let left: f64 = history
            .db
            .query_row("SELECT rebuilt_cost FROM waste_hours", [], |row| row.get(0))
            .expect("the hour is kept");
        assert!(
            left.abs() > 0.0 && left.abs() <= ROUNDING,
            "forgetting the turns left {left}, which is no rounding to test"
        );
        assert_eq!(
            rebuilt_by_session(&history, whole, whole + crate::time::HOUR)
                .expect("the sessions read"),
            HashMap::new(),
            "a rounding's worth of rebuilding named a session"
        );
        let said = waste(&history, whole, whole + crate::time::HOUR).expect("the waste reads");
        assert_eq!(
            (said.causes, said.worst),
            (Vec::new(), None),
            "a rounding's worth of rebuilding named a cause or the worst session"
        );
    }

    /// Which table answers: an hour whole within a span is read kept, and
    /// the same hour on a span's ragged edge from its turns, so a kept sum
    /// changed by hand shows in the one and not in the other.
    #[test]
    fn a_whole_hour_is_read_kept_and_a_ragged_one_from_its_turns() {
        let history = stored(&kept());
        let first = hour_of(BASE) + 1;
        let whole = hour_start(first);
        let spent = |from: f64, to: f64| waste(&history, from, to).expect("the waste reads").spent;
        let spans = [
            (whole, whole + crate::time::HOUR),
            (whole - 900.0, whole + 900.0),
        ];
        let before = spans.map(|(from, to)| spent(from, to));
        let rows = history
            .db
            .execute(
                "UPDATE waste_hours SET cost = cost + 100 WHERE hour = ?1",
                [first],
            )
            .expect("the kept hour is changed");
        assert!(rows > 0, "the hour the spans read has no turns kept");
        let after = spans.map(|(from, to)| spent(from, to));
        let added = 100.0 * f64::from(u32::try_from(rows).expect("a few rows"));
        assert!(
            (after[0] - before[0] - added).abs() < ROUNDING,
            "a span of the whole hour read {} rather than the kept {added} more",
            after[0] - before[0]
        );
        assert!(
            (after[1] - before[1]).abs() < ROUNDING,
            "a ragged span read its edge's hour kept, {} more",
            after[1] - before[1]
        );
    }

    /// How a span is split into whole hours and the ragged edges outside
    /// them: one of whole hours, a ragged one and one inside a single
    /// hour. That the whole hours are then read kept, the test above shows
    /// by tampering with one.
    #[test]
    fn a_span_splits_into_whole_hours_and_ragged_edges() {
        let hour = crate::time::HOUR;
        let first = hour_of(BASE) + 1;
        let whole = hour_start(first);
        let split = |span: Span| (span.first, span.last, span.start, span.end);
        assert_eq!(
            split(Span::of(whole, whole + 30.0 * hour)),
            (first, first + 30, whole, whole + 30.0 * hour),
            "a span of whole hours read some of them from the turns"
        );
        assert_eq!(
            split(Span::of(whole - 900.0, whole + 2.0 * hour + 60.0)),
            (first, first + 2, whole, whole + 2.0 * hour),
            "a ragged span read other hours kept than those whole within it"
        );
        assert_eq!(
            split(Span::of(whole + 60.0, whole + 2000.0)),
            (0, 0, whole + 2000.0, whole + 2000.0),
            "a span within one hour read an hour kept"
        );
    }

    /// Each session's work among `within`, summed from its agent turns.
    fn agents(within: &[&Kept]) -> Vec<Work> {
        let mut agents: Vec<Work> = Vec::new();
        for name in (0..5).map(|i| format!("s{i}")) {
            let own: Vec<&&Kept> = within
                .iter()
                .filter(|turn| turn.session == name && turn.kind == "agent")
                .collect();
            if own.is_empty() {
                continue;
            }
            let turns = f64::from(u32::try_from(own.len()).expect("a few turns"));
            let dollars: f64 = own.iter().map(|turn| turn.cost).sum();
            let read: f64 = own.iter().map(|turn| turn.cache_read).sum();
            // Each fixture turn's input and cache write are 3 and 5.
            let prompt: f64 = own.iter().map(|turn| turn.cache_read + 8.0).sum();
            agents.push(Work {
                session: name.clone(),
                // As MAX does, passing over turns with no quest.
                quest: own.iter().filter_map(|turn| turn.quest.clone()).max(),
                turns,
                dollars,
                per_turn: dollars / turns.max(1.0),
                cached: if prompt > 0.0 { read / prompt } else { 0.0 },
                rebuilt_after_gaps: own
                    .iter()
                    .filter(|turn| turn.cause == Some("gap"))
                    .map(|turn| turn.cost / 2.0)
                    .sum(),
            });
        }
        agents
    }

    #[test]
    fn a_costly_session_among_many_usual_ones_is_found() {
        let mut sessions: Vec<Work> = (0..40)
            .map(|i| work(&format!("s{i}"), 60.0, 0.5 + 0.01 * f64::from(i % 7)))
            .collect();
        sessions.push(work("costly", 200.0, 1.8));
        let found = outliers(&sessions, 0.05, 25.0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].work.session, "costly");
        assert!(found[0].times > 3.0);
    }

    #[test]
    fn a_few_costly_turns_are_noise() {
        let mut sessions: Vec<Work> = (0..40)
            .map(|i| work(&format!("s{i}"), 60.0, 0.5 + 0.01 * f64::from(i % 7)))
            .collect();
        sessions.push(work("brief", 6.0, 1.8));
        assert_eq!(
            outliers(&sessions, 0.05, 25.0),
            [] as [crate::waste::Outlier; 0]
        );
    }
}
