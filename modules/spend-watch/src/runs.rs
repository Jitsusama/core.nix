//! The harness's runs of subagents, from its observability database, read
//! only. Subagents keep no log of their own, so this is where their spend
//! is known: each run names its agent, kind, persona and model, the
//! session that started it and, since the harness began recording them,
//! the sessions its agents ran as, which is what lets a feed's bill for a
//! subagent's session be traced back to the session and quest above it.

use rusqlite::{Connection, OpenFlags, params};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub run: String,
    pub agent: String,
    pub kind: String,
    pub model: String,
    pub persona: String,
    pub parent: Option<String>,
    pub started: f64,
    pub ended: Option<f64>,
    pub cost: f64,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub sessions: Vec<String>,
}

/// Every run the harness recorded, or none when it has no database yet.
#[expect(
    clippy::cast_precision_loss,
    reason = "token counts are far below 2^52, so f64 holds them exactly"
)]
pub fn read(path: &Path) -> Result<Vec<Run>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("opening {}: {error}", path.display()))?;
    db.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    let columns: Vec<String> = db
        .prepare("SELECT name FROM pragma_table_info('runs')")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()
        })
        .map_err(|error| error.to_string())?;
    let has = |name: &str| columns.iter().any(|column| column == name);
    let optional = |name: &str| {
        if has(name) {
            name.to_owned()
        } else {
            "NULL".to_owned()
        }
    };
    let query = format!(
        "SELECT run_id, subagent_id, kind, model, persona, {}, started_at, {}, cost_total,
         tokens_input, tokens_output, tokens_cache_read, tokens_cache_write, {} FROM runs",
        optional("session_id"),
        optional("ended_at"),
        optional("subagent_session_ids"),
    );
    let mut statement = db.prepare(&query).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let sessions: Option<String> = row.get(13)?;
            Ok(Run {
                run: row.get(0)?,
                agent: row.get(1)?,
                kind: row.get(2)?,
                model: bare(&row.get::<_, Option<String>>(3)?.unwrap_or_default()),
                persona: row.get(4)?,
                parent: row.get(5)?,
                started: crate::time::from_milliseconds(row.get::<_, i64>(6)?),
                ended: row
                    .get::<_, Option<i64>>(7)?
                    .map(crate::time::from_milliseconds),
                cost: row.get(8)?,
                input: row.get::<_, i64>(9)? as f64,
                output: row.get::<_, i64>(10)? as f64,
                cache_read: row.get::<_, i64>(11)? as f64,
                cache_write: row.get::<_, i64>(12)? as f64,
                sessions: sessions
                    .and_then(|text| serde_json::from_str(&text).ok())
                    .unwrap_or_default(),
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

/// A model as pi's logs name it: a run names some with their provider
/// first ("anthropic/claude-opus-5"), which would split one model in two.
pub fn bare(model: &str) -> String {
    model.rsplit('/').next().unwrap_or(model).to_owned()
}

/// Replaces the history's runs with what the harness holds now.
pub fn store(history: &crate::history::History, runs: &[Run]) -> Result<(), String> {
    let db = &history.db;
    db.execute("DELETE FROM runs", [])
        .map_err(|error| error.to_string())?;
    let mut insert = db
        .prepare(
            "INSERT OR REPLACE INTO runs VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )
        .map_err(|error| error.to_string())?;
    for run in runs {
        insert
            .execute(params![
                run.run,
                run.agent,
                run.kind,
                run.model,
                run.persona,
                run.parent,
                run.started,
                run.ended,
                run.cost,
                run.input,
                run.output,
                run.cache_read,
                run.cache_write,
                serde_json::to_string(&run.sessions).unwrap_or_else(|_| "[]".into()),
            ])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "a whole second in milliseconds divides by a thousand exactly"
    )]
    fn a_harness_from_before_sessions_were_recorded_still_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("runs.db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE runs (run_id TEXT, subagent_id TEXT, kind TEXT, model TEXT,
             persona TEXT, tokens_input INTEGER, tokens_output INTEGER,
             tokens_cache_read INTEGER, tokens_cache_write INTEGER, cost_total REAL,
             started_at INTEGER);
             INSERT INTO runs VALUES ('r1', 'a1', 'fleet', 'opus', 'critic', 10, 20, 30, 40,
             1.5, 1791950171000);",
        )
        .unwrap();
        drop(db);
        let runs = read(&path).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].parent, None);
        assert!(runs[0].sessions.is_empty());
        assert_eq!(runs[0].started, 1_791_950_171.0);
    }

    #[test]
    fn no_database_is_no_runs() {
        assert!(read(Path::new("/nonexistent/runs.db")).unwrap().is_empty());
    }
}
