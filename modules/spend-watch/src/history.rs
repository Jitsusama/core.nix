//! The history: everything read, kept for good in SQLite, and rebuildable
//! from its sources, so it needs no backup: each feed is told what's kept
//! of its bill as it starts (`feed::kept_line`), so one built afresh is
//! told it again. A shape that only grows is brought up to date where it
//! stands, which costs no backfill; one that doesn't read is set aside and
//! the sources read again; and one a newer spend-watch keeps is left as it
//! is, since that one still reads all of it.
//!
//! The daemon writes it and the command line reads it, both at once, so it
//! runs in WAL mode with a patient lock.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::time::Duration;

/// Bumped whenever a table's shape or meaning changes.
pub const SCHEMA: i64 = 4;

/// Each hour's turns summed by what the picture splits them by, kept as
/// they're added and forgotten. Every pass reads every hour, and summing
/// the turns again each time cost most of a pass once they were counted
/// in hundreds of thousands; these are a fiftieth as many.
macro_rules! hours {
    () => {
        "
CREATE TABLE turn_hours (
    hour INTEGER NOT NULL, session TEXT NOT NULL, quest TEXT, model TEXT NOT NULL,
    kind TEXT NOT NULL, turns INTEGER NOT NULL, cost REAL NOT NULL);
CREATE INDEX turn_hours_key ON turn_hours (hour, session, quest, model, kind);
CREATE TRIGGER turn_hours_add AFTER INSERT ON turns BEGIN
    INSERT INTO turn_hours SELECT NEW.hour, NEW.session, NEW.quest, NEW.model, NEW.kind, 0, 0
        WHERE NOT EXISTS (SELECT 1 FROM turn_hours WHERE hour = NEW.hour
            AND session = NEW.session AND quest IS NEW.quest AND model = NEW.model
            AND kind = NEW.kind);
    UPDATE turn_hours SET turns = turns + 1, cost = cost + NEW.cost
        WHERE hour = NEW.hour AND session = NEW.session AND quest IS NEW.quest
            AND model = NEW.model AND kind = NEW.kind;
END;
CREATE TRIGGER turn_hours_take AFTER DELETE ON turns BEGIN
    UPDATE turn_hours SET turns = turns - 1, cost = cost - OLD.cost
        WHERE hour = OLD.hour AND session = OLD.session AND quest IS OLD.quest
            AND model = OLD.model AND kind = OLD.kind;
    DELETE FROM turn_hours WHERE turns = 0 AND hour = OLD.hour AND session = OLD.session
        AND quest IS OLD.quest AND model = OLD.model AND kind = OLD.kind;
END;
CREATE TRIGGER turns_stay BEFORE UPDATE ON turns BEGIN
    SELECT RAISE(ABORT, 'a turn is added or forgotten, never changed');
END;
"
    };
}

/// What brings each older shape up to the next, where only something was
/// added: `UPGRADES[n - 1]` takes shape n to n + 1.
const UPGRADES: [&str; 3] = [
    "ALTER TABLE caps ADD COLUMN resets REAL;",
    "ALTER TABLE health ADD COLUMN since REAL;",
    concat!(
        hours!(),
        "INSERT INTO turn_hours SELECT hour, session, quest, model, kind, COUNT(*), SUM(cost)
         FROM turns GROUP BY hour, session, quest, model, kind;"
    ),
];

const TABLES: &str = concat!(
    "
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
    message TEXT NOT NULL, sign_in TEXT, at REAL NOT NULL,
    since REAL, PRIMARY KEY (feed, source));
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
",
    hours!()
);

pub struct History {
    pub db: Connection,
}

fn sql(error: rusqlite::Error) -> String {
    error.to_string()
}

impl History {
    /// Opens the history, setting aside one that doesn't read so it's
    /// built again from the sources. One a newer spend-watch keeps is left
    /// as it is and refused: an older system booted again shouldn't cost
    /// the bill the newer one was told.
    pub fn open(path: &Path) -> Result<History, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        if let Some(shape) = History::newer(path) {
            return Err(format!(
                "{} is kept by a newer spend-watch, at shape {shape} where this one \
                 reads up to {SCHEMA}, so it's left as it is",
                path.display()
            ));
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
            let aside = History::aside(path, schema.unwrap_or(0));
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

    /// The shape of the history at `path`, when a newer spend-watch keeps
    /// it, read without writing anything.
    pub fn newer(path: &Path) -> Option<i64> {
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let db = Connection::open_with_flags(path, flags).ok()?;
        db.busy_timeout(Duration::from_secs(10)).ok()?;
        let shape: i64 = db
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
                row.get::<_, String>(0)
            })
            .ok()?
            .parse()
            .ok()?;
        (shape > SCHEMA).then_some(shape)
    }

    /// Where a history of `shape` is set aside: a name nothing holds yet,
    /// since one set aside before may be all that's left of its bill.
    fn aside(path: &Path, shape: i64) -> std::path::PathBuf {
        let named = |count: u32| {
            let tail = if count == 1 {
                String::new()
            } else {
                format!(".{count}")
            };
            path.with_extension(format!("db.schema-{shape}{tail}"))
        };
        (1..)
            .map(named)
            .find(|aside| !aside.exists())
            .expect("a free name")
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

    // A machine booted into an older system runs an older watch on the
    // history a newer one kept, whose feeds won't say their bill again.
    #[test]
    fn a_newer_shape_is_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let newer = (SCHEMA + 1).to_string();
        {
            let history = History::open(&path).unwrap();
            history.set_meta("schema", &newer).unwrap();
            history.set_meta("kept", "yes").unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        assert_eq!(History::newer(&path), Some(SCHEMA + 1));
        let refused = History::open(&path).err().unwrap();
        assert!(refused.contains("a newer spend-watch"), "{refused}");
        assert_eq!(std::fs::read(&path).unwrap(), before);
        // SQLite's own files beside it are the history's, not a copy.
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| !name.starts_with("history.db") || name.contains("schema"))
            .collect();
        assert_eq!(names, Vec::<String>::new());
        let db = Connection::open(&path).unwrap();
        let kept: String = db
            .query_row("SELECT value FROM meta WHERE key = 'kept'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kept, "yes");
    }

    #[test]
    fn a_shape_this_one_reads_is_not_newer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        assert_eq!(History::newer(&path), None);
        History::open(&path).unwrap();
        assert_eq!(History::newer(&path), None);
    }

    #[test]
    fn a_history_set_aside_never_replaces_one_set_aside_before() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        for kept in ["first", "second"] {
            let history = History::open(&path).unwrap();
            history.set_meta("schema", "0").unwrap();
            history.set_meta("kept", kept).unwrap();
        }
        History::open(&path).unwrap();
        let kept = |name: &str| {
            let db = Connection::open(dir.path().join(name)).unwrap();
            db.query_row("SELECT value FROM meta WHERE key = 'kept'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap()
        };
        assert_eq!(kept("history.db.schema-0"), "first");
        assert_eq!(kept("history.db.schema-0.2"), "second");
    }

    // The first shape, as a machine has it: its caps gain the column a
    // reset is kept in, its sources the one for how long they've stood,
    // and its bill stays where it is.
    #[test]
    fn the_first_shape_is_brought_up_to_date_where_it_stands() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                &TABLES
                    .replace(hours!(), "")
                    .replace("\n    resets REAL,", "")
                    .replace("\n    since REAL,", ""),
            )
            .unwrap();
            db.execute_batch(
                "INSERT INTO meta VALUES ('schema', '1');
                 INSERT INTO health VALUES ('proxy', 'meter', 'fine', '', NULL, 1.0);
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
        let since: Option<f64> = history
            .db
            .query_row("SELECT since FROM health", [], |row| row.get(0))
            .unwrap();
        assert_eq!(since, None);
        assert!(!dir.path().join("history.db.schema-1").exists());
    }

    fn turn(history: &History, session: &str, hour: i64, quest: Option<&str>, cost: f64) {
        history
            .db
            .execute(
                "INSERT INTO turns VALUES (?1, ?2, ?3, 'opus', 'anthropic', 'agent', ?4,
                 0, 0, 0, 0, 0, ?5, 0, 0, NULL, 0, 0, NULL)",
                params![session, (hour * 3600) as f64, hour, quest, cost],
            )
            .unwrap();
    }

    /// The hours as kept, against the turns summed afresh.
    fn hours_hold(history: &History) {
        let read = |sql: &str| -> Vec<(i64, String, Option<String>, i64, f64)> {
            let mut statement = history.db.prepare(sql).unwrap();
            statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let kept = read(
            "SELECT hour, session, quest, turns, cost FROM turn_hours
             ORDER BY hour, session, quest",
        );
        let summed = read(
            "SELECT hour, session, quest, COUNT(*), SUM(cost) FROM turns
             GROUP BY hour, session, quest, model, kind ORDER BY hour, session, quest",
        );
        assert_eq!(kept.len(), summed.len(), "{kept:?} against {summed:?}");
        for (kept, summed) in kept.iter().zip(&summed) {
            assert_eq!(
                (&kept.0, &kept.1, &kept.2, kept.3),
                (&summed.0, &summed.1, &summed.2, summed.3)
            );
            assert!(
                (kept.4 - summed.4).abs() < 1e-9,
                "{kept:?} against {summed:?}"
            );
        }
    }

    #[test]
    fn each_hour_s_turns_are_kept_summed() {
        let history = History::memory();
        turn(&history, "a", 10, Some("q"), 1.25);
        turn(&history, "a", 10, Some("q"), 0.5);
        turn(&history, "a", 10, None, 2.0);
        turn(&history, "a", 10, None, 0.25);
        turn(&history, "b", 10, None, 3.0);
        turn(&history, "a", 11, Some("q"), 4.0);
        hours_hold(&history);
        let groups: i64 = history
            .db
            .query_row("SELECT COUNT(*) FROM turn_hours", [], |row| row.get(0))
            .unwrap();
        assert_eq!(groups, 4);
        // Forgetting part of an hour leaves the rest of it.
        history
            .db
            .execute("DELETE FROM turns WHERE cost = 0.5", [])
            .unwrap();
        hours_hold(&history);
        // A log read again from its start forgets its session's turns.
        history
            .db
            .execute("DELETE FROM turns WHERE session = 'a'", [])
            .unwrap();
        hours_hold(&history);
        turn(&history, "a", 10, None, 0.75);
        hours_hold(&history);
        assert!(
            history
                .db
                .execute("UPDATE turns SET cost = 9 WHERE session = 'b'", [])
                .is_err()
        );
    }

    // A history of the third shape has its turns summed as it's brought up
    // to date, so its picture reads as it did.
    #[test]
    fn the_third_shape_gains_its_hours() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let history = History {
                db: Connection::open(&path).unwrap(),
            };
            history
                .db
                .execute_batch(&TABLES.replace(hours!(), ""))
                .unwrap();
            history
                .db
                .execute_batch("INSERT INTO meta VALUES ('schema', '3');")
                .unwrap();
            turn(&history, "a", 10, Some("q"), 1.5);
            turn(&history, "a", 10, Some("q"), 2.5);
            turn(&history, "b", 12, None, 1.0);
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        hours_hold(&history);
        turn(&history, "b", 12, None, 1.0);
        hours_hold(&history);
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
