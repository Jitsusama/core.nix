//! Where money bought nothing, from pi's logs: cache written to rebuild
//! context a session already had, by cause, and each session's cost per
//! unit of work beside its peers'. These are at pi's list prices, since
//! only the logs know a turn's cache; the ratio of bill to list scales
//! them when there's a bill.

use crate::history::History;
use crate::stats;
use std::collections::HashMap;

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

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

pub fn waste(history: &History, from: f64, to: f64) -> Result<Waste, String> {
    let db = &history.db;
    let (spent, cache_write, rebuilt): (f64, f64, f64) = db
        .query_row(
            "SELECT COALESCE(SUM(cost),0), COALESCE(SUM(cost_cache_write),0),
             COALESCE(SUM(rebuilt_cost),0) FROM turns WHERE at >= ?1 AND at < ?2",
            [from, to],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(fail)?;
    let mut statement = db
        .prepare(
            "SELECT cause, SUM(rebuilt_cost) FROM turns WHERE at >= ?1 AND at < ?2
             AND cause IS NOT NULL AND cause != 'fresh' GROUP BY cause ORDER BY 2 DESC",
        )
        .map_err(fail)?;
    let causes = statement
        .query_map([from, to], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    let worst = db
        .query_row(
            "SELECT session, SUM(rebuilt_cost) FROM turns WHERE at >= ?1 AND at < ?2
             AND cause = 'gap' GROUP BY session ORDER BY 2 DESC LIMIT 1",
            [from, to],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
        )
        .ok();
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
        .prepare(
            "SELECT session, MAX(quest), COUNT(*), SUM(cost), SUM(cache_read),
             SUM(input + cache_read + cache_write),
             SUM(CASE WHEN cause = 'gap' THEN rebuilt_cost ELSE 0 END)
             FROM turns WHERE at >= ?1 AND at < ?2 AND kind = 'agent' GROUP BY session",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([from, to], |row| {
            let turns: f64 = row.get::<_, i64>(2)? as f64;
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

/// Rebuilt dollars by cause, summed by any key, for the collection's cuts.
pub fn rebuilt_by(
    history: &History,
    from: f64,
    to: f64,
    key: &str,
) -> Result<Vec<(String, f64)>, String> {
    let column = match key {
        "quest" => "COALESCE(quest, '')",
        "model" => "model",
        "session" => "session",
        _ => "cause",
    };
    let mut statement = history
        .db
        .prepare(&format!(
            "SELECT {column}, SUM(rebuilt_cost) FROM turns WHERE at >= ?1 AND at < ?2
             AND cause IS NOT NULL AND cause != 'fresh' GROUP BY 1 ORDER BY 2 DESC"
        ))
        .map_err(fail)?;
    let rows = statement
        .query_map([from, to], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(fail)?;
    let mut out: HashMap<String, f64> = HashMap::new();
    for row in rows {
        let (name, dollars) = row.map_err(fail)?;
        *out.entry(name).or_default() += dollars;
    }
    let mut out: Vec<(String, f64)> = out.into_iter().collect();
    out.sort_by(|one, other| other.1.total_cmp(&one.1));
    Ok(out)
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

    #[test]
    fn a_costly_session_among_many_usual_ones_is_found() {
        let mut sessions: Vec<Work> = (0..40)
            .map(|i| work(&format!("s{i}"), 60.0, 0.5 + 0.01 * (i % 7) as f64))
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
            .map(|i| work(&format!("s{i}"), 60.0, 0.5 + 0.01 * (i % 7) as f64))
            .collect();
        sessions.push(work("brief", 6.0, 1.8));
        assert!(outliers(&sessions, 0.05, 25.0).is_empty());
    }
}
