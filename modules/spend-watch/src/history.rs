//! The history: everything read, kept for good in SQLite, and rebuildable
//! from its sources, so it needs no backup. A shape that only grows is
//! brought up to date where it stands, since a feed that keeps how far it
//! read wouldn't say its bill again; one that changes otherwise is set
//! aside and the sources read again.
//!
//! The daemon writes it and the command line reads it, both at once, so it
//! runs in WAL mode with a patient lock.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::time::Duration;

/// Bumped whenever a table's shape or meaning changes.
pub const SCHEMA: i64 = 2;

/// What brings each older shape up to the next, where only columns were
/// added: `UPGRADES[n - 1]` takes shape n to n + 1.
const UPGRADES: [&str; 1] = ["ALTER TABLE caps ADD COLUMN resets REAL;"];

const TABLES: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (
    path TEXT PRIMARY KEY, identity TEXT NOT NULL, offset INTEGER NOT NULL,
    carry TEXT NOT NULL, session TEXT);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY, cwd TEXT NOT NULL, repo TEXT NOT NULL, started REAL NOT NULL);
CREATE TABLE turns (
    session TEXT NOT NULL, at REAL NOT NULL, hour INTEGER NOT NULL, model TEXT NOT NULL,
    provider TEXT NOT NULL, kind TEXT NOT NULL, quest TEXT,
    input REAL NOT NULL, output REAL NOT NULL, cache_read REAL NOT NULL,
    cache_write REAL NOT NULL, cache_write_long REAL NOT NULL,
    cost REAL NOT NULL, cost_cache_write REAL NOT NULL, cost_output REAL NOT NULL,
    gap REAL, rebuilt REAL NOT NULL, rebuilt_cost REAL NOT NULL, cause TEXT);
CREATE INDEX turns_hour ON turns (hour);
CREATE INDEX turns_session ON turns (session);
CREATE INDEX turns_at ON turns (at);
CREATE TABLE refusals (session TEXT NOT NULL, at REAL NOT NULL, model TEXT NOT NULL,
    said TEXT NOT NULL);
CREATE TABLE runs (
    run TEXT NOT NULL, agent TEXT NOT NULL, kind TEXT NOT NULL, model TEXT NOT NULL,
    persona TEXT NOT NULL, parent TEXT, started REAL NOT NULL, ended REAL,
    cost REAL NOT NULL, input REAL NOT NULL, output REAL NOT NULL,
    cache_read REAL NOT NULL, cache_write REAL NOT NULL, sessions TEXT NOT NULL,
    PRIMARY KEY (run, agent));
CREATE TABLE quests (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, kind TEXT NOT NULL, parent TEXT,
    status TEXT NOT NULL, started TEXT, updated TEXT, prs INTEGER NOT NULL,
    sessions TEXT NOT NULL);
CREATE TABLE billed (
    feed TEXT NOT NULL, source TEXT NOT NULL, hour INTEGER NOT NULL, tool TEXT NOT NULL,
    model TEXT NOT NULL, vendor TEXT NOT NULL, session TEXT NOT NULL, agent TEXT NOT NULL,
    token TEXT NOT NULL, input REAL NOT NULL, cached REAL NOT NULL, cache_write REAL NOT NULL,
    output REAL NOT NULL, dollars REAL NOT NULL, provisional INTEGER NOT NULL,
    at REAL NOT NULL,
    PRIMARY KEY (feed, source, hour, tool, model, vendor, session, agent, token));
CREATE INDEX billed_hour ON billed (hour);
CREATE TABLE settled (feed TEXT NOT NULL, source TEXT NOT NULL, through INTEGER NOT NULL,
    PRIMARY KEY (feed, source));
CREATE TABLE caps (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, label TEXT NOT NULL,
    allowance REAL, window_hours INTEGER NOT NULL, spent REAL NOT NULL, warn REAL NOT NULL,
    fallback REAL, state TEXT NOT NULL, ends TEXT, ends_known INTEGER NOT NULL,
    resets REAL, PRIMARY KEY (feed, name, at));
CREATE TABLE cap_events (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, kind TEXT NOT NULL,
    before REAL, after REAL, spent REAL NOT NULL, PRIMARY KEY (feed, name, at, kind));
CREATE TABLE health (feed TEXT NOT NULL, source TEXT NOT NULL, state TEXT NOT NULL,
    message TEXT NOT NULL, sign_in TEXT, at REAL NOT NULL, PRIMARY KEY (feed, source));
CREATE TABLE actions (feed TEXT NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL,
    run TEXT NOT NULL, fits TEXT NOT NULL, PRIMARY KEY (feed, id));
CREATE TABLE reads (feed TEXT NOT NULL, source TEXT NOT NULL, at REAL NOT NULL,
    bytes REAL NOT NULL, millis REAL NOT NULL, job TEXT NOT NULL);
CREATE TABLE said (feed TEXT NOT NULL, severity TEXT NOT NULL, text TEXT NOT NULL,
    at REAL NOT NULL, PRIMARY KEY (feed, text));
CREATE TABLE findings (
    key TEXT PRIMARY KEY, detector TEXT NOT NULL, severity TEXT NOT NULL,
    title TEXT NOT NULL, detail TEXT NOT NULL, dollars REAL NOT NULL, surprise REAL NOT NULL,
    started REAL NOT NULL, updated REAL NOT NULL, cleared REAL, action TEXT,
    session TEXT, quest TEXT);
CREATE TABLE marks (key TEXT PRIMARY KEY, mark TEXT NOT NULL, at REAL NOT NULL);
CREATE TABLE notices (key TEXT NOT NULL, at REAL NOT NULL, cap INTEGER NOT NULL,
    id INTEGER NOT NULL, withdrawn REAL);
";

pub struct History {
    pub db: Connection,
}

fn sql(error: rusqlite::Error) -> String {
    error.to_string()
}

impl History {
    /// Opens the history, setting aside one of an older shape so it's
    /// built again from the sources.
    pub fn open(path: &Path) -> Result<History, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let existing = path.exists();
        let history = History::connect(path)?;
        let schema: Option<i64> = history
            .db
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .ok()
            .flatten()
            .and_then(|value| value.parse().ok());
        if schema == Some(SCHEMA) {
            return Ok(history);
        }
        if let Some(from) = schema.filter(|from| (1..SCHEMA).contains(from)) {
            history.upgrade(from)?;
            return Ok(history);
        }
        drop(history);
        if existing {
            let aside = path.with_extension(format!("db.schema-{}", schema.unwrap_or(0)));
            for suffix in ["", "-wal", "-shm"] {
                let from = format!("{}{suffix}", path.display());
                let to = format!("{}{suffix}", aside.display());
                let _ = std::fs::rename(&from, &to);
            }
        }
        let history = History::connect(path)?;
        history.db.execute_batch(TABLES).map_err(sql)?;
        history
            .db
            .execute(
                "INSERT INTO meta VALUES ('schema', ?1)",
                [SCHEMA.to_string()],
            )
            .map_err(sql)?;
        Ok(history)
    }

    /// Brings a history of an older shape up to this one, all at once or
    /// not at all.
    fn upgrade(&self, from: i64) -> Result<(), String> {
        let steps: String = UPGRADES[(from - 1) as usize..].concat();
        self.db
            .execute_batch(&format!(
                "BEGIN IMMEDIATE; {steps} UPDATE meta SET value = '{SCHEMA}' \
                 WHERE key = 'schema'; COMMIT;"
            ))
            .map_err(|error| {
                let _ = self.db.execute_batch("ROLLBACK;");
                format!("the history couldn't be brought up to date: {error}")
            })
    }

    /// An empty history in memory, for tests.
    pub fn memory() -> History {
        let db = Connection::open_in_memory().expect("an in-memory database");
        db.execute_batch(TABLES).expect("the schema");
        History { db }
    }

    fn connect(path: &Path) -> Result<History, String> {
        let db = Connection::open(path).map_err(sql)?;
        db.busy_timeout(Duration::from_secs(10)).map_err(sql)?;
        db.pragma_update(None, "journal_mode", "WAL").map_err(sql)?;
        db.pragma_update(None, "synchronous", "NORMAL")
            .map_err(sql)?;
        // Without the table, the schema reads as none.
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .map_err(sql)?;
        Ok(History { db })
    }

    pub fn meta(&self, key: &str) -> Option<String> {
        self.db
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), String> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO meta VALUES (?1, ?2)",
                params![key, value],
            )
            .map(|_| ())
            .map_err(sql)
    }

    /// Begins a transaction, first rolling back one an earlier error left
    /// open, so one failure can't wedge every write after it.
    pub fn begin(&self) -> Result<(), String> {
        if !self.db.is_autocommit() {
            self.db.execute_batch("ROLLBACK").map_err(sql)?;
        }
        self.db.execute_batch("BEGIN").map_err(sql)
    }

    /// Runs what's given in one transaction: all of it kept, or none.
    pub fn within<T>(&self, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.begin()?;
        match work() {
            Ok(done) => {
                self.commit()?;
                Ok(done)
            }
            Err(error) => {
                let _ = self.db.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    pub fn commit(&self) -> Result<(), String> {
        self.db.execute_batch("COMMIT").map_err(sql)
    }

    /// Where reading a log stopped, and what it carried, if it's the same
    /// file as before: a log replaced under the same name starts over.
    pub fn file(&self, path: &str, identity: &str) -> Option<(u64, String)> {
        self.db
            .query_row(
                "SELECT offset, carry FROM files WHERE path = ?1 AND identity = ?2",
                params![path, identity],
                |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?)),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Forgets a log's turns, for one replaced or read again from its start.
    pub fn forget_file(&self, path: &str) -> Result<(), String> {
        let session: Option<String> = self
            .db
            .query_row("SELECT session FROM files WHERE path = ?1", [path], |row| {
                row.get(0)
            })
            .optional()
            .map_err(sql)?
            .flatten();
        if let Some(session) = session {
            self.db
                .execute("DELETE FROM turns WHERE session = ?1", [&session])
                .map_err(sql)?;
            self.db
                .execute("DELETE FROM refusals WHERE session = ?1", [&session])
                .map_err(sql)?;
        }
        self.db
            .execute("DELETE FROM files WHERE path = ?1", [path])
            .map_err(sql)?;
        Ok(())
    }

    pub fn set_file(
        &self,
        path: &str,
        identity: &str,
        offset: u64,
        carry: &str,
        session: Option<&str>,
    ) -> Result<(), String> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO files VALUES (?1, ?2, ?3, ?4, ?5)",
                params![path, identity, offset as i64, carry, session],
            )
            .map(|_| ())
            .map_err(sql)
    }

    pub fn add_event(&self, event: &crate::logs::Event) -> Result<(), String> {
        use crate::logs::Event;
        match event {
            Event::Session { id, cwd, at } => {
                self.db
                    .execute(
                        "INSERT OR REPLACE INTO sessions VALUES (?1, ?2, ?3, ?4)",
                        params![id, cwd, crate::logs::repo_of(cwd), at],
                    )
                    .map_err(sql)?;
            }
            Event::Turn(turn) => {
                self.db
                    .execute(
                        "INSERT INTO turns VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                         ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                        params![
                            turn.session,
                            turn.at,
                            crate::time::hour_of(turn.at),
                            turn.model,
                            turn.provider,
                            turn.kind.name(),
                            turn.quest,
                            turn.tokens.input,
                            turn.tokens.output,
                            turn.tokens.cache_read,
                            turn.tokens.cache_write,
                            turn.tokens.cache_write_long,
                            turn.cost.total,
                            turn.cost.cache_write,
                            turn.cost.output,
                            turn.gap,
                            turn.rebuilt,
                            turn.rebuilt_cost,
                            turn.cause.as_ref().map(|cause| cause.name()),
                        ],
                    )
                    .map_err(sql)?;
            }
            Event::Refusal {
                session,
                at,
                model,
                said,
            } => {
                self.db
                    .execute(
                        "INSERT INTO refusals VALUES (?1, ?2, ?3, ?4)",
                        params![session, at, model, said],
                    )
                    .map_err(sql)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_shape_is_set_aside_and_built_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let history = History::open(&path).unwrap();
            history.set_meta("schema", "0").unwrap();
            history.set_meta("kept", "yes").unwrap();
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        assert_eq!(history.meta("kept"), None);
        assert!(dir.path().join("history.db.schema-0").exists());
    }

    // The first shape, as a machine has it: its caps gain the column a
    // reset is kept in, and its bill stays where it is.
    #[test]
    fn the_first_shape_is_brought_up_to_date_where_it_stands() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(&TABLES.replace("\n    resets REAL,", ""))
                .unwrap();
            db.execute_batch(
                "INSERT INTO meta VALUES ('schema', '1');
                 INSERT INTO caps VALUES ('proxy', 'weekly', 1.0, 'weekly cap', 15000, 168,
                     3948.29, 0.9, 4000, 'ok', NULL, 0);",
            )
            .unwrap();
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        let caps = crate::caps::latest(&history).unwrap();
        assert_eq!(
            (caps.len(), caps[0].spent, caps[0].resets),
            (1, 3948.29, None)
        );
        assert!(!dir.path().join("history.db.schema-1").exists());
    }

    #[test]
    fn the_same_shape_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        History::open(&path)
            .unwrap()
            .set_meta("kept", "yes")
            .unwrap();
        assert_eq!(
            History::open(&path).unwrap().meta("kept").as_deref(),
            Some("yes")
        );
    }
}
