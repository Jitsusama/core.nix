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
pub const SCHEMA: i64 = 11;

/// The most the write-ahead log keeps on disk once checkpointed. SQLite
/// otherwise keeps it at the largest it ever grew: on 7 November 321 MB,
/// beside a 400 MB history, every frame of it already checkpointed.
const WAL_LIMIT: i64 = 32 * 1024 * 1024;

/// Each hour's turns summed by what the picture splits them by, kept as
/// they're added and forgotten. Every pass reads every hour, and summing
/// the turns again each time cost most of a pass once they were counted
/// in hundreds of thousands; these are a fiftieth as many. A turn's
/// provider and the budget its log said are among what they're split by,
/// since the budget a turn draws on is worked out from both.
macro_rules! hours {
    () => {
        "
CREATE TABLE turn_hours (
    hour INTEGER NOT NULL, session TEXT NOT NULL, quest TEXT, model TEXT NOT NULL,
    kind TEXT NOT NULL, provider TEXT NOT NULL, budget TEXT, turns INTEGER NOT NULL,
    cost REAL NOT NULL);
CREATE INDEX turn_hours_key ON turn_hours (hour, session, quest, model, kind);
CREATE TRIGGER turn_hours_add AFTER INSERT ON turns BEGIN
    INSERT INTO turn_hours SELECT NEW.hour, NEW.session, NEW.quest, NEW.model, NEW.kind,
        NEW.provider, NEW.budget, 0, 0
        WHERE NOT EXISTS (SELECT 1 FROM turn_hours WHERE hour = NEW.hour
            AND session = NEW.session AND quest IS NEW.quest AND model = NEW.model
            AND kind = NEW.kind AND provider = NEW.provider AND budget IS NEW.budget);
    UPDATE turn_hours SET turns = turns + 1, cost = cost + NEW.cost
        WHERE hour = NEW.hour AND session = NEW.session AND quest IS NEW.quest
            AND model = NEW.model AND kind = NEW.kind AND provider = NEW.provider
            AND budget IS NEW.budget;
END;
CREATE TRIGGER turn_hours_take AFTER DELETE ON turns BEGIN
    UPDATE turn_hours SET turns = turns - 1, cost = cost - OLD.cost
        WHERE hour = OLD.hour AND session = OLD.session AND quest IS OLD.quest
            AND model = OLD.model AND kind = OLD.kind AND provider = OLD.provider
            AND budget IS OLD.budget;
    DELETE FROM turn_hours WHERE turns = 0 AND hour = OLD.hour AND session = OLD.session
        AND quest IS OLD.quest AND model = OLD.model AND kind = OLD.kind
        AND provider = OLD.provider AND budget IS OLD.budget;
END;
CREATE TRIGGER turns_stay BEFORE UPDATE ON turns BEGIN
    SELECT RAISE(ABORT, 'a turn is added or forgotten, never changed');
END;
"
    };
}

/// The hours as the fourth shape first kept them, before they were split
/// by provider and budget, which taking a third shape up builds.
macro_rules! hours_4 {
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

/// Each hour's turns summed by what the waste and work figures split them
/// by, kept as they're added and forgotten, for the reason `turn_hours`
/// is: a week's or a month's figures summed from the turns fetched each
/// one at random through `turns_at`, and on 21 November a pass read 22 GB
/// that way from a history of 2 GB. Kept apart from `turn_hours`, which
/// every pass reads for every hour, since splitting those by cause too
/// would grow every one of those reads. Its sums are the turns' but for
/// rounding, as `turn_hours`' are: each adds and takes a turn as it comes.
macro_rules! waste_hours {
    () => {
        "
CREATE TABLE waste_hours (
    hour INTEGER NOT NULL, session TEXT NOT NULL, quest TEXT, model TEXT NOT NULL,
    kind TEXT NOT NULL, cause TEXT, turns INTEGER NOT NULL, cost REAL NOT NULL,
    cost_cache_write REAL NOT NULL, rebuilt_cost REAL NOT NULL, cache_read REAL NOT NULL,
    prompt REAL NOT NULL);
CREATE INDEX waste_hours_key ON waste_hours (hour, session, quest, model, kind, cause);
CREATE TRIGGER waste_hours_add AFTER INSERT ON turns BEGIN
    INSERT INTO waste_hours SELECT NEW.hour, NEW.session, NEW.quest, NEW.model, NEW.kind,
        NEW.cause, 0, 0, 0, 0, 0, 0
        WHERE NOT EXISTS (SELECT 1 FROM waste_hours WHERE hour = NEW.hour
            AND session = NEW.session AND quest IS NEW.quest AND model = NEW.model
            AND kind = NEW.kind AND cause IS NEW.cause);
    UPDATE waste_hours SET turns = turns + 1, cost = cost + NEW.cost,
        cost_cache_write = cost_cache_write + NEW.cost_cache_write,
        rebuilt_cost = rebuilt_cost + NEW.rebuilt_cost,
        cache_read = cache_read + NEW.cache_read,
        prompt = prompt + NEW.input + NEW.cache_read + NEW.cache_write
        WHERE hour = NEW.hour AND session = NEW.session AND quest IS NEW.quest
            AND model = NEW.model AND kind = NEW.kind AND cause IS NEW.cause;
END;
CREATE TRIGGER waste_hours_take AFTER DELETE ON turns BEGIN
    UPDATE waste_hours SET turns = turns - 1, cost = cost - OLD.cost,
        cost_cache_write = cost_cache_write - OLD.cost_cache_write,
        rebuilt_cost = rebuilt_cost - OLD.rebuilt_cost,
        cache_read = cache_read - OLD.cache_read,
        prompt = prompt - OLD.input - OLD.cache_read - OLD.cache_write
        WHERE hour = OLD.hour AND session = OLD.session AND quest IS OLD.quest
            AND model = OLD.model AND kind = OLD.kind AND cause IS OLD.cause;
    DELETE FROM waste_hours WHERE turns = 0 AND hour = OLD.hour AND session = OLD.session
        AND quest IS OLD.quest AND model = OLD.model AND kind = OLD.kind
        AND cause IS OLD.cause;
END;
"
    };
}

/// Each budget a session asked for and couldn't draw on, and why, from
/// the harness's budget entries.
macro_rules! unresolved {
    () => {
        "
CREATE TABLE unresolved (session TEXT NOT NULL, at REAL NOT NULL, budget TEXT NOT NULL,
    missing TEXT NOT NULL);
"
    };
}

/// Each turn, refusal and unresolved budget once, whichever log said it.
/// A session copied to another machine and gone on there keeps its id, so
/// both machines' logs hold the turns from before the copy, and each holds
/// its own after; read together they're the session's turns once. A turn
/// is told by its kind and tokens as well as its instant, so two calls
/// logged in the same millisecond are still two.
macro_rules! once {
    () => {
        "
CREATE UNIQUE INDEX turns_once ON turns (session, at, model, kind, input, output);
CREATE UNIQUE INDEX refusals_once ON refusals (session, at, model);
CREATE UNIQUE INDEX unresolved_once ON unresolved (session, at, budget);
"
    };
}

/// Each time an allowance went over: from when, until when it came back
/// under (none while it's over), and what every source said as it went,
/// kept here as well as in the report written from it.
macro_rules! crossings {
    () => {
        "
CREATE TABLE crossings (
    budget TEXT NOT NULL, allowance TEXT NOT NULL, at REAL NOT NULL, back REAL,
    path TEXT NOT NULL, said TEXT NOT NULL, PRIMARY KEY (budget, allowance, at));
"
    };
}

/// What a feed billed, by everything a record keys it by: an empty budget
/// is the feed's own.
macro_rules! billed {
    ($name:literal) => {
        concat!(
            "CREATE TABLE ",
            $name,
            " (
    feed TEXT NOT NULL, source TEXT NOT NULL, hour INTEGER NOT NULL, tool TEXT NOT NULL,
    model TEXT NOT NULL, vendor TEXT NOT NULL, session TEXT NOT NULL, agent TEXT NOT NULL,
    token TEXT NOT NULL, input REAL NOT NULL, cached REAL NOT NULL, cache_write REAL NOT NULL,
    output REAL NOT NULL, dollars REAL NOT NULL, provisional INTEGER NOT NULL,
    at REAL NOT NULL, budget TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (feed, source, hour, tool, model, vendor, session, agent, token, budget));
"
        )
    };
}

/// Each reading of a feed's caps, for its budget: an empty one is the
/// feed's own. A share's is what's used, and nothing spent.
macro_rules! caps {
    ($name:literal) => {
        concat!(
            "CREATE TABLE ",
            $name,
            " (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, label TEXT NOT NULL,
    allowance REAL, window_hours INTEGER NOT NULL, spent REAL NOT NULL, warn REAL NOT NULL,
    fallback REAL, state TEXT NOT NULL, ends TEXT, ends_known INTEGER NOT NULL,
    resets REAL, budget TEXT NOT NULL DEFAULT '', unit TEXT NOT NULL DEFAULT 'dollars',
    used REAL, PRIMARY KEY (feed, budget, name, at));
"
        )
    };
}

macro_rules! cap_events {
    ($name:literal) => {
        concat!(
            "CREATE TABLE ",
            $name,
            " (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, kind TEXT NOT NULL,
    before REAL, after REAL, spent REAL NOT NULL, budget TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (feed, budget, name, at, kind));
"
        )
    };
}

/// What brings each older shape up to the next, where only something was
/// added: `UPGRADES[n - 1]` takes shape n to n + 1.
const UPGRADES: [&str; 10] = [
    "ALTER TABLE caps ADD COLUMN resets REAL;",
    "ALTER TABLE health ADD COLUMN since REAL;",
    concat!(
        hours_4!(),
        "INSERT INTO turn_hours SELECT hour, session, quest, model, kind, COUNT(*), SUM(cost)
         FROM turns GROUP BY hour, session, quest, model, kind;"
    ),
    "ALTER TABLE sessions ADD COLUMN kind TEXT; ALTER TABLE sessions ADD COLUMN name TEXT;",
    // A turn read before budgets were has none its log said, so it draws
    // on the budget claiming its provider, or the default.
    concat!(
        "ALTER TABLE turns ADD COLUMN budget TEXT;
         DROP TRIGGER turn_hours_add; DROP TRIGGER turn_hours_take; DROP TRIGGER turns_stay;
         DROP TABLE turn_hours;",
        hours!(),
        "INSERT INTO turn_hours SELECT hour, session, quest, model, kind, provider, budget,
         COUNT(*), SUM(cost) FROM turns
         GROUP BY hour, session, quest, model, kind, provider, budget;"
    ),
    // What a feed said before a record could name its budget was its own,
    // and a cap before shares were was in dollars. The budget is part of
    // what keys each, so a feed can say two budgets' alike, and SQLite
    // only keys a table as it's made.
    concat!(
        billed!("billed_7"),
        "INSERT INTO billed_7 SELECT *, '' FROM billed; DROP TABLE billed;
         ALTER TABLE billed_7 RENAME TO billed; CREATE INDEX billed_hour ON billed (hour);",
        caps!("caps_7"),
        "INSERT INTO caps_7 SELECT *, '', 'dollars', NULL FROM caps; DROP TABLE caps;
         ALTER TABLE caps_7 RENAME TO caps;",
        cap_events!("cap_events_7"),
        "INSERT INTO cap_events_7 SELECT *, '' FROM cap_events; DROP TABLE cap_events;
         ALTER TABLE cap_events_7 RENAME TO cap_events;"
    ),
    crossings!(),
    unresolved!(),
    concat!(
        // A session read before machines were is this machine's, and a
        // turn kept twice before is kept once.
        "ALTER TABLE sessions ADD COLUMN machine TEXT;
         DELETE FROM turns WHERE rowid NOT IN
             (SELECT MIN(rowid) FROM turns GROUP BY session, at, model, kind, input, output);
         DELETE FROM refusals WHERE rowid NOT IN
             (SELECT MIN(rowid) FROM refusals GROUP BY session, at, model);
         DELETE FROM unresolved WHERE rowid NOT IN
             (SELECT MIN(rowid) FROM unresolved GROUP BY session, at, budget);",
        once!()
    ),
    concat!(
        waste_hours!(),
        "INSERT INTO waste_hours SELECT hour, session, quest, model, kind, cause, COUNT(*),
         SUM(cost), SUM(cost_cache_write), SUM(rebuilt_cost), SUM(cache_read),
         SUM(input + cache_read + cache_write) FROM turns
         GROUP BY hour, session, quest, model, kind, cause;"
    ),
];

const TABLES: &str = concat!(
    "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (
    path TEXT PRIMARY KEY, identity TEXT NOT NULL, offset INTEGER NOT NULL,
    carry TEXT NOT NULL, session TEXT);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY, cwd TEXT NOT NULL, repo TEXT NOT NULL, started REAL NOT NULL,
    kind TEXT, name TEXT,
    machine TEXT);
CREATE TABLE turns (
    session TEXT NOT NULL, at REAL NOT NULL, hour INTEGER NOT NULL, model TEXT NOT NULL,
    provider TEXT NOT NULL, kind TEXT NOT NULL, quest TEXT,
    input REAL NOT NULL, output REAL NOT NULL, cache_read REAL NOT NULL,
    cache_write REAL NOT NULL, cache_write_long REAL NOT NULL,
    cost REAL NOT NULL, cost_cache_write REAL NOT NULL, cost_output REAL NOT NULL,
    gap REAL, rebuilt REAL NOT NULL, rebuilt_cost REAL NOT NULL, cause TEXT,
    budget TEXT);
CREATE INDEX turns_hour ON turns (hour);
CREATE INDEX turns_session ON turns (session);
CREATE INDEX turns_at ON turns (at);
CREATE TABLE refusals (session TEXT NOT NULL, at REAL NOT NULL, model TEXT NOT NULL,
    said TEXT NOT NULL);
",
    unresolved!(),
    "
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
",
    billed!("billed"),
    "CREATE INDEX billed_hour ON billed (hour);
CREATE TABLE settled (feed TEXT NOT NULL, source TEXT NOT NULL, through INTEGER NOT NULL,
    PRIMARY KEY (feed, source));
",
    caps!("caps"),
    cap_events!("cap_events"),
    "CREATE TABLE health (feed TEXT NOT NULL, source TEXT NOT NULL, state TEXT NOT NULL,
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
    crossings!(),
    hours!(),
    once!(),
    waste_hours!()
);

pub struct History {
    pub db: Connection,
}

/// A database error as the text every caller reports, for `map_err`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err hands over the error by value, and this is the fn it calls"
)]
pub fn sql(error: rusqlite::Error) -> String {
    error.to_string()
}

/// Runs `step` with SQLite's sorts going through files, then puts them back
/// in memory whether it worked or not. A step sorts every turn at once,
/// unlike a pass, and once rather than every few minutes: on a history of
/// 3.7 M turns the eleventh shape's took 640 MB in memory and 18 MB
/// through files, so the disk takes it, and the passes after go back to
/// memory.
fn on_disk<T>(db: &Connection, step: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    db.pragma_update(None, "temp_store", "FILE").map_err(sql)?;
    let stepped = step();
    let restored = db.pragma_update(None, "temp_store", "MEMORY").map_err(sql);
    stepped.and_then(|done| restored.map(|()| done))
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
        History::shape(path).filter(|shape| *shape > SCHEMA)
    }

    /// The shape of the history at `path`, when opening it would bring it
    /// up to this one's first, read without writing anything. On a large
    /// history that takes minutes and holds it throughout.
    pub fn older(path: &Path) -> Option<i64> {
        History::shape(path).filter(|shape| (1..SCHEMA).contains(shape))
    }

    fn shape(path: &Path) -> Option<i64> {
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let db = Connection::open_with_flags(path, flags).ok()?;
        db.busy_timeout(Duration::from_secs(10)).ok()?;
        db.query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get::<_, String>(0)
        })
        .ok()?
        .parse()
        .ok()
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
        (1..=u32::MAX)
            .map(named)
            .find(|aside| !aside.exists())
            .expect("a free name")
    }

    /// Brings a history of an older shape up to this one, all at once or
    /// not at all.
    fn upgrade(&self, from: i64) -> Result<(), String> {
        let first = usize::try_from(from - 1)
            .map_err(|_| format!("the history's shape {from} has no upgrade"))?;
        let steps: String = UPGRADES[first..].concat();
        on_disk(&self.db, || {
            self.db
                .execute_batch(&format!(
                    "BEGIN IMMEDIATE; {steps} UPDATE meta SET value = '{SCHEMA}' \
                     WHERE key = 'schema'; COMMIT;"
                ))
                .map_err(|error| {
                    let _ = self.db.execute_batch("ROLLBACK;");
                    format!("the history couldn't be brought up to date: {error}")
                })
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
        db.pragma_update(None, "journal_size_limit", WAL_LIMIT)
            .map_err(sql)?;
        db.pragma_update(None, "synchronous", "NORMAL")
            .map_err(sql)?;
        // A pass's picture sorts and groups more turns than SQLite keeps in
        // memory, and by default it spills them to files in the temporary
        // folder: four of them, about 62 MB, every pass. Writeback reaches
        // them before they're deleted, so on a busy day the watch wrote
        // about 200 GB to the disk, ahead of Joel's own saves in its queue.
        // In memory they cost about that much RAM for the length of a pass.
        db.pragma_update(None, "temp_store", "MEMORY")
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
        // A row kept before 25 November says `device:inode`; only its inode
        // is held against the log's, so a history that moved to the inode
        // alone reads none of its logs again for it.
        self.db
            .query_row(
                "SELECT offset, carry FROM files
                 WHERE path = ?1 AND substr(identity, instr(identity, ':') + 1) = ?2",
                params![path, identity],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .ok()
            .flatten()
            // Only offsets from `set_file` are kept, and none is below zero.
            .and_then(|(offset, carry)| Some((u64::try_from(offset).ok()?, carry)))
    }

    /// Forgets a log, for one replaced or read again from its start. A
    /// session's only log takes its turns with it, so what the log no
    /// longer says is no longer counted. One of several keeps them, since
    /// which log said what isn't kept: reading it again from its start
    /// counts only turns not held already, and the turn first read stays.
    /// A log gone from disk still counts as one of them: what it said was
    /// spent, and the history is the only record left of it, so clearing
    /// old logs away never lowers what was counted.
    pub fn forget_file(&self, path: &str) -> Result<(), String> {
        let forget = self.db.unchecked_transaction().map_err(sql)?;
        let session: Option<String> = forget
            .query_row("SELECT session FROM files WHERE path = ?1", [path], |row| {
                row.get(0)
            })
            .optional()
            .map_err(sql)?
            .flatten();
        if let Some(session) = session {
            let others: bool = forget
                .query_row(
                    "SELECT EXISTS (SELECT 1 FROM files WHERE session = ?1 AND path != ?2)",
                    params![session, path],
                    |row| row.get(0),
                )
                .map_err(sql)?;
            if !others {
                for table in ["turns", "refusals", "unresolved"] {
                    forget
                        .execute(
                            &format!("DELETE FROM {table} WHERE session = ?1"),
                            [&session],
                        )
                        .map_err(sql)?;
                }
            }
        }
        forget
            .execute("DELETE FROM files WHERE path = ?1", [path])
            .map_err(sql)?;
        forget.commit().map_err(sql)
    }

    /// Whether turns of a session from no later than `until` are counted
    /// under it, as a parent's are once its log is read up to a fork.
    pub fn holds_turns(&self, id: &str, until: f64) -> bool {
        self.db
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM turns WHERE session = ?1 AND at <= ?2)",
                rusqlite::params![id, until],
                |row| row.get(0),
            )
            .unwrap_or(false)
    }

    /// When a session of this machine's began, as its log here says, or
    /// none for a session that isn't this machine's.
    pub fn this_machines_start(&self, id: &str) -> Option<f64> {
        self.db
            .query_row(
                "SELECT started FROM sessions WHERE id = ?1 AND machine IS NULL",
                [id],
                |row| row.get(0),
            )
            .ok()
    }

    /// Forgets what a session was counted for before `until`, for a copy
    /// pi carried it into that was read whole before its parent was
    /// known: those turns are its parent's, counted under the parent.
    pub fn forget_copied(&self, session: &str, until: f64) -> Result<(), String> {
        let forget = self.db.unchecked_transaction().map_err(sql)?;
        for table in ["turns", "refusals", "unresolved"] {
            forget
                .execute(
                    &format!("DELETE FROM {table} WHERE session = ?1 AND at < ?2"),
                    params![session, until],
                )
                .map_err(sql)?;
        }
        forget.commit().map_err(sql)
    }

    pub fn set_file(
        &self,
        path: &str,
        identity: &str,
        offset: u64,
        carry: &str,
        session: Option<&str>,
    ) -> Result<(), String> {
        // No file is longer than an i64 can count, so this always fits.
        let offset = i64::try_from(offset)
            .map_err(|_| format!("{path} is read past where an offset can be kept"))?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO files VALUES (?1, ?2, ?3, ?4, ?5)",
                params![path, identity, offset, carry, session],
            )
            .map(|_| ())
            .map_err(sql)
    }

    pub fn add_event(&self, event: &crate::logs::Event) -> Result<(), String> {
        use crate::logs::Event;
        match event {
            Event::Session {
                id,
                cwd,
                at,
                kind,
                machine,
            } => {
                // A log read again keeps the name its session was given. A
                // session once read from a log here stays this machine's, as
                // it was here first, and what a brought log says of it
                // changes nothing. One only elsewhere is the machine's whose
                // log was read from its start last, so a machine renamed, or
                // its logs brought somewhere new, takes its sessions along.
                self.db
                    .execute(
                        "INSERT INTO sessions VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)
                         ON CONFLICT (id) DO UPDATE SET cwd = excluded.cwd,
                             repo = excluded.repo, started = excluded.started,
                             kind = excluded.kind, machine = excluded.machine
                         WHERE sessions.machine IS NOT NULL OR excluded.machine IS NULL",
                        params![id, cwd, crate::logs::repo_of(cwd), at, kind, machine],
                    )
                    .map_err(sql)?;
            }
            Event::Named { session, name } => {
                self.db
                    .execute(
                        "UPDATE sessions SET name = ?2 WHERE id = ?1",
                        params![session, name],
                    )
                    .map_err(sql)?;
            }
            Event::Turn(turn) => {
                self.db
                    .execute(
                        "INSERT OR IGNORE INTO turns VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                         ?10, ?11,
                         ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                            turn.cause.as_ref().map(crate::logs::Cause::name),
                            turn.budget,
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
                        "INSERT OR IGNORE INTO refusals VALUES (?1, ?2, ?3, ?4)",
                        params![session, at, model, said],
                    )
                    .map_err(sql)?;
            }
            Event::Unresolved {
                session,
                at,
                budget,
                missing,
            } => {
                self.db
                    .execute(
                        "INSERT OR IGNORE INTO unresolved VALUES (?1, ?2, ?3, ?4)",
                        params![session, at, budget, missing],
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
    use std::fs;

    #[test]
    fn a_history_sorts_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::open(&dir.path().join("history.db")).unwrap();
        let store: i64 = history
            .db
            .pragma_query_value(None, "temp_store", |row| row.get(0))
            .unwrap();
        // 2 is MEMORY.
        assert_eq!(store, 2);
    }

    #[test]
    fn on_disk_sorts_through_files_and_then_in_memory_again() {
        let history = History::memory();
        let store = || -> i64 {
            history
                .db
                .pragma_query_value(None, "temp_store", |row| row.get(0))
                .unwrap()
        };
        // 1 is FILE and 2 is MEMORY.
        assert_eq!(
            on_disk(&history.db, || Ok(store())),
            Ok(1),
            "a step sorted other than through files"
        );
        assert_eq!(store(), 2, "the passes after a step sort through files");
        let failed: Result<(), String> = on_disk(&history.db, || Err("it failed".into()));
        assert_eq!(
            failed,
            Err("it failed".into()),
            "a failed step said something else"
        );
        assert_eq!(
            store(),
            2,
            "the passes after a failed step sort through files"
        );
    }

    #[test]
    fn the_log_shrinks_back_once_a_large_write_is_checkpointed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let history = History::open(&path).unwrap();
        let large = 2 * WAL_LIMIT;
        history
            .db
            .execute_batch(&format!(
                "CREATE TABLE big (b BLOB); INSERT INTO big VALUES (zeroblob({large}));"
            ))
            .unwrap();
        let log = dir.path().join("history.db-wal");
        let size = |limit: i64| u64::try_from(limit).unwrap();
        assert!(fs::metadata(&log).unwrap().len() > size(large));
        history
            .db
            .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |_| Ok(()))
            .unwrap();
        history.set_meta("after", "a small write").unwrap();
        assert!(fs::metadata(&log).unwrap().len() <= size(WAL_LIMIT));
    }

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
                &sixth_shape()
                    .replace(hours!(), "")
                    .replace("\n    resets REAL,", "")
                    .replace("\n    since REAL,", "")
                    .replace(SESSIONS_5, "")
                    .replace(TURNS_6, ""),
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
                "INSERT INTO turns (session, at, hour, model, provider, kind, quest, input,
                 output, cache_read, cache_write, cache_write_long, cost, cost_cache_write,
                 cost_output, gap, rebuilt, rebuilt_cost, cause) VALUES
                 (?1, ?2 + (SELECT COUNT(*) FROM turns), ?3, 'opus', 'anthropic', 'agent', ?4,
                 0, 0, 0, 0, 0, ?5, 0, 0, NULL, 0, 0, NULL)",
                params![session, crate::time::hour_start(hour), hour, quest, cost],
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
                .execute_batch(
                    &sixth_shape()
                        .replace(hours!(), "")
                        .replace(SESSIONS_5, "")
                        .replace(TURNS_6, ""),
                )
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

    /// What shape 5 added to a session.
    const SESSIONS_5: &str = ",\n    kind TEXT, name TEXT";

    /// What shape 6 added to a turn.
    const TURNS_6: &str = ",\n    budget TEXT";

    /// What shape 10 added to a session.
    const SESSIONS_10: &str = ",\n    machine TEXT";

    /// The tables as the fifth shape kept them.
    fn fifth_shape() -> String {
        sixth_shape()
            .replace(TURNS_6, "")
            .replace(hours!(), hours_4!())
    }

    /// The tables as the sixth shape kept them, before a feed's record
    /// could name its budget.
    fn sixth_shape() -> String {
        const BILLED: &str = "CREATE TABLE billed (
    feed TEXT NOT NULL, source TEXT NOT NULL, hour INTEGER NOT NULL, tool TEXT NOT NULL,
    model TEXT NOT NULL, vendor TEXT NOT NULL, session TEXT NOT NULL, agent TEXT NOT NULL,
    token TEXT NOT NULL, input REAL NOT NULL, cached REAL NOT NULL, cache_write REAL NOT NULL,
    output REAL NOT NULL, dollars REAL NOT NULL, provisional INTEGER NOT NULL,
    at REAL NOT NULL,
    PRIMARY KEY (feed, source, hour, tool, model, vendor, session, agent, token));
";
        const CAPS: &str = "CREATE TABLE caps (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, label TEXT NOT NULL,
    allowance REAL, window_hours INTEGER NOT NULL, spent REAL NOT NULL, warn REAL NOT NULL,
    fallback REAL, state TEXT NOT NULL, ends TEXT, ends_known INTEGER NOT NULL,
    resets REAL, PRIMARY KEY (feed, name, at));
";
        const CAP_EVENTS: &str = "CREATE TABLE cap_events (
    feed TEXT NOT NULL, name TEXT NOT NULL, at REAL NOT NULL, kind TEXT NOT NULL,
    before REAL, after REAL, spent REAL NOT NULL, PRIMARY KEY (feed, name, at, kind));
";
        // The seventh shape kept no crossings, which the eighth gains, nor
        // unresolved budgets, which the ninth gains.
        TABLES
            .replace(crossings!(), "")
            .replace(unresolved!(), "")
            .replace(SESSIONS_10, "")
            .replace(once!(), "")
            .replace(waste_hours!(), "")
            .replace(billed!("billed"), BILLED)
            .replace(caps!("caps"), CAPS)
            .replace(cap_events!("cap_events"), CAP_EVENTS)
    }

    // The sixth shape's bill and caps are each their feed's own budget's,
    // and its caps are in dollars, kept where they stand.
    #[test]
    fn a_feed_s_records_gain_a_budget_where_they_stand() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(&sixth_shape()).unwrap();
            db.execute_batch(
                "INSERT INTO meta VALUES ('schema', '6');
                 INSERT INTO billed VALUES ('proxy', 'meter', 10, 'pi', 'm', 'v', '', '', '',
                     0, 0, 0, 0, 29.83, 1, 1.0);
                 INSERT INTO caps VALUES ('proxy', 'weekly', 1.0, 'weekly cap', 15000, 168,
                     3948.29, 0.9, 4000, 'ok', NULL, 0, NULL);
                 INSERT INTO cap_events VALUES ('proxy', 'weekly', 1.0, 'raised', 4000, 15000,
                     3948.29);",
            )
            .unwrap();
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        let billed: (String, f64) = history
            .db
            .query_row("SELECT budget, dollars FROM billed", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(billed, (String::new(), 29.83));
        let caps = crate::caps::latest(&history).unwrap();
        assert_eq!(
            (caps[0].budget.as_str(), caps[0].unit, caps[0].spent),
            ("", crate::caps::Unit::Dollars, 3948.29)
        );
        assert_eq!(crate::caps::events(&history, 0.0).unwrap().len(), 1);
        assert!(!dir.path().join("history.db.schema-6").exists());
    }

    // The fifth shape's turns, read before budgets were, gain a budget
    // none of their logs said, and each hour's sums are split by provider
    // and budget, so its picture reads as it did.
    #[test]
    fn turns_gain_a_budget_where_they_stand() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let history = History {
                db: Connection::open(&path).unwrap(),
            };
            history.db.execute_batch(&fifth_shape()).unwrap();
            history
                .db
                .execute_batch("INSERT INTO meta VALUES ('schema', '5');")
                .unwrap();
            turn(&history, "a", 10, Some("q"), 1.5);
            turn(&history, "a", 10, Some("q"), 2.5);
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        hours_hold(&history);
        let split: (String, Option<String>, i64) = history
            .db
            .query_row(
                "SELECT provider, budget, turns FROM turn_hours",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(split, ("anthropic".into(), None, 2));
        turn(&history, "a", 10, Some("q"), 1.0);
        hours_hold(&history);
        assert!(!dir.path().join("history.db.schema-5").exists());
    }

    // The fourth shape's sessions gain a kind and a name, none for those
    // read before, and its turns stay.
    #[test]
    fn sessions_gain_a_kind_where_they_stand() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let history = History {
                db: Connection::open(&path).unwrap(),
            };
            history
                .db
                .execute_batch(&fifth_shape().replace(SESSIONS_5, ""))
                .unwrap();
            history
                .db
                .execute_batch(
                    "INSERT INTO meta VALUES ('schema', '4');
                     INSERT INTO sessions VALUES ('a', '/x', 'x', 1.0);",
                )
                .unwrap();
            turn(&history, "a", 10, Some("q"), 1.5);
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        let (kind, name): (Option<String>, Option<String>) = history
            .db
            .query_row(
                "SELECT kind, name FROM sessions WHERE id = 'a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((kind, name), (None, None));
        hours_hold(&history);
        assert!(!dir.path().join("history.db.schema-4").exists());
    }

    // A session read again keeps its name.
    #[test]
    fn a_session_keeps_its_name() {
        use crate::logs::Event;
        let history = History::memory();
        let session = Event::Session {
            id: "a".into(),
            cwd: "/x".into(),
            at: 1.0,
            kind: Some("swarm".into()),
            machine: None,
        };
        history.add_event(&session).unwrap();
        history
            .add_event(&Event::Named {
                session: "a".into(),
                name: "acp@team".into(),
            })
            .unwrap();
        history.add_event(&session).unwrap();
        let row: (String, String) = history
            .db
            .query_row(
                "SELECT kind, name FROM sessions WHERE id = 'a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("swarm".into(), "acp@team".into()));
    }

    // The eighth shape kept no budget a session couldn't draw on; the
    // ninth gains them, and keeps what was already there.
    #[test]
    fn the_eighth_shape_gains_unresolved_budgets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                &TABLES
                    .replace(unresolved!(), "")
                    .replace(SESSIONS_10, "")
                    .replace(once!(), "")
                    .replace(waste_hours!(), ""),
            )
            .unwrap();
            db.execute_batch(
                "INSERT INTO meta VALUES ('schema', '8');
                 INSERT INTO refusals VALUES ('s1', 1.0, 'm', 'over');",
            )
            .unwrap();
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        history
            .add_event(&crate::logs::Event::Unresolved {
                session: "s1".into(),
                at: 2.0,
                budget: "teal".into(),
                missing: "no key for it".into(),
            })
            .unwrap();
        let kept: (i64, i64) = history
            .db
            .query_row(
                "SELECT (SELECT COUNT(*) FROM refusals), (SELECT COUNT(*) FROM unresolved)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(kept, (1, 1));
    }

    // The ninth shape knew no machine; its sessions are this machine's,
    // and a session read after is kept under the machine its log was on.
    #[test]
    fn the_ninth_shape_gains_machines() {
        use crate::logs::Event;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                &TABLES
                    .replace(SESSIONS_10, "")
                    .replace(once!(), "")
                    .replace(waste_hours!(), ""),
            )
            .unwrap();
            db.execute_batch(
                "INSERT INTO meta VALUES ('schema', '9');
                 INSERT INTO sessions VALUES ('here', '/x', 'x', 1.0, NULL, NULL);
                 INSERT INTO turns VALUES ('here', 5.0, 0, 'm', 'p', 'person', NULL,
                     1, 1, 0, 0, 0, 2.0, 0, 0, NULL, 0, 0, NULL, NULL);
                 INSERT INTO turns VALUES ('here', 5.0, 0, 'm', 'p', 'person', NULL,
                     1, 1, 0, 0, 0, 2.0, 0, 0, NULL, 0, 0, NULL, NULL);
                 INSERT INTO turns VALUES ('here', 5.0, 0, 'm', 'p', 'side', NULL,
                     1, 1, 0, 0, 0, 0.5, 0, 0, NULL, 0, 0, NULL, NULL);
                 INSERT INTO refusals VALUES ('here', 5.0, 'm', 'no');
                 INSERT INTO refusals VALUES ('here', 5.0, 'm', 'no');
                 INSERT INTO unresolved VALUES ('here', 5.0, 'teal', 'no key');
                 INSERT INTO unresolved VALUES ('here', 5.0, 'teal', 'no key');",
            )
            .unwrap();
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        history
            .add_event(&Event::Session {
                id: "there".into(),
                cwd: "/x".into(),
                at: 2.0,
                kind: None,
                machine: Some("teal".into()),
            })
            .unwrap();
        let machines: Vec<(String, Option<String>)> = history
            .db
            .prepare("SELECT id, machine FROM sessions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            machines,
            [("here".into(), None), ("there".into(), Some("teal".into()))],
            "a session kept before machines wasn't this machine's, or one read after lost its own"
        );
        // A turn held already, read again, is still kept once: the shape
        // keeps each turn by its key, not only by dropping what it held twice.
        history
            .db
            .execute_batch(
                "INSERT OR IGNORE INTO turns VALUES ('here', 5.0, 0, 'm', 'p', 'person', NULL,
                     1, 1, 0, 0, 0, 2.0, 0, 0, NULL, 0, 0, NULL, NULL);
                 INSERT OR IGNORE INTO refusals VALUES ('here', 5.0, 'm', 'no');
                 INSERT OR IGNORE INTO unresolved VALUES ('here', 5.0, 'teal', 'no key');",
            )
            .unwrap();
        let (turns, cost): (i64, f64) = history
            .db
            .query_row("SELECT SUM(turns), SUM(cost) FROM turn_hours", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(
            (turns, cost),
            (2, 2.5),
            "a turn kept twice before machines wasn't kept once, or a side call in the same \
             millisecond was taken for it, with its hour's sum"
        );
        // The waste hours are summed after the turns kept twice are dropped,
        // so they count each once too, and a turn read again adds nothing.
        waste_hours_hold(&history);
        let kept: (i64, i64) = history
            .db
            .query_row(
                "SELECT (SELECT COUNT(*) FROM refusals), (SELECT COUNT(*) FROM unresolved)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            kept,
            (1, 1),
            "a refusal or unresolved budget kept twice before machines wasn't kept once"
        );
    }

    /// What a waste hour splits its turns by besides the hour, session and
    /// cause: the quest, model and kind.
    type Split<'a> = (Option<&'a str>, &'a str, &'a str);

    /// The split most turns here are taken under.
    const MOST: Split<'static> = (Some("q"), "opus", "agent");

    /// A turn as the waste figures read it: everything they sum, by what
    /// they split it by.
    fn wasted(
        history: &History,
        session: &str,
        at: f64,
        (quest, model, kind): Split<'_>,
        cause: Option<&str>,
        cost: f64,
    ) {
        history
            .db
            .execute(
                "INSERT INTO turns (session, at, hour, model, provider, kind, quest, input,
                 output, cache_read, cache_write, cache_write_long, cost, cost_cache_write,
                 cost_output, gap, rebuilt, rebuilt_cost, cause) VALUES
                 (?1, ?2, ?3, ?4, 'anthropic', ?5, ?6, 3, 1, 5, 7, 0, ?7, ?7 / 4,
                 0, NULL, 0, ?7 / 2, ?8)",
                params![
                    session,
                    at,
                    crate::time::hour_of(at),
                    model,
                    kind,
                    quest,
                    cost,
                    cause
                ],
            )
            .unwrap();
    }

    /// The waste hours as kept, against the turns summed afresh: every
    /// key alike, and every sum alike but for rounding, since a kept sum
    /// adds and takes each turn as it comes where a fresh one adds them
    /// all at once.
    fn waste_hours_hold(history: &History) {
        type Key = (i64, String, Option<String>, String, String, Option<String>);
        const KEY: &str = "hour, session, quest, model, kind, cause";
        let read = |sql: &str| -> Vec<(Key, [f64; 6])> {
            let mut statement = history.db.prepare(sql).unwrap();
            statement
                .query_map([], |row| {
                    let key = (
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    );
                    let mut sums = [0.0; 6];
                    for (at, sum) in sums.iter_mut().enumerate() {
                        *sum = row.get(6 + at)?;
                    }
                    Ok((key, sums))
                })
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let kept = read(&format!(
            "SELECT {KEY}, turns, cost, cost_cache_write, rebuilt_cost, cache_read, prompt
             FROM waste_hours ORDER BY {KEY}"
        ));
        let summed = read(&format!(
            "SELECT {KEY}, COUNT(*), SUM(cost), SUM(cost_cache_write), SUM(rebuilt_cost),
             SUM(cache_read), SUM(input + cache_read + cache_write)
             FROM turns GROUP BY {KEY} ORDER BY {KEY}"
        ));
        let keys =
            |rows: &[(Key, [f64; 6])]| rows.iter().map(|row| row.0.clone()).collect::<Vec<_>>();
        assert_eq!(
            keys(&kept),
            keys(&summed),
            "the waste hours kept other keys than their turns have"
        );
        for ((key, kept), (_, summed)) in kept.iter().zip(&summed) {
            assert!(
                kept.iter()
                    .zip(summed)
                    .all(|(one, other)| (one - other).abs() < 1e-9),
                "the waste hour {key:?} drifted from its turns: {kept:?} against {summed:?}"
            );
        }
    }

    // The tenth shape summed no waste by the hour; the eleventh builds the
    // sums from the turns it holds, and keeps them as turns come and go.
    #[test]
    fn the_tenth_shape_gains_waste_by_the_hour() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let history = History {
                db: Connection::open(&path).unwrap(),
            };
            history
                .db
                .execute_batch(&TABLES.replace(waste_hours!(), ""))
                .unwrap();
            history
                .db
                .execute_batch("INSERT INTO meta VALUES ('schema', '10');")
                .unwrap();
            wasted(&history, "a", 10.0, MOST, None, 0.1);
            wasted(&history, "a", 20.0, MOST, Some("gap"), 0.2);
            wasted(&history, "a", 30.0, MOST, Some("gap"), 0.3);
            wasted(
                &history,
                "a",
                35.0,
                (None, "opus", "agent"),
                Some("gap"),
                0.7,
            );
            wasted(
                &history,
                "a",
                36.0,
                (Some("q"), "sonnet", "agent"),
                Some("gap"),
                1.1,
            );
            wasted(
                &history,
                "a",
                37.0,
                (Some("q"), "opus", "side"),
                Some("gap"),
                1.3,
            );
            wasted(&history, "b", 4000.0, MOST, Some("fresh"), 8.0);
        }
        let history = History::open(&path).unwrap();
        assert_eq!(history.meta("schema"), Some(SCHEMA.to_string()));
        waste_hours_hold(&history);
        let rows = |history: &History| -> i64 {
            history
                .db
                .query_row("SELECT COUNT(*) FROM waste_hours", [], |row| row.get(0))
                .unwrap()
        };
        assert_eq!(
            rows(&history),
            6,
            "the step didn't sum one row for each hour, session, quest, model, kind and cause"
        );
        wasted(&history, "a", 40.0, MOST, Some("gap"), 0.4);
        wasted(
            &history,
            "a",
            41.0,
            (None, "opus", "agent"),
            Some("gap"),
            0.9,
        );
        waste_hours_hold(&history);
        // Part of a group forgotten, as a copy's turns before its fork are,
        // leaves the rest of it summed.
        history
            .db
            .execute("DELETE FROM turns WHERE session = 'a' AND at < 21.0", [])
            .unwrap();
        history
            .db
            .execute("DELETE FROM turns WHERE at = 41.0", [])
            .unwrap();
        waste_hours_hold(&history);
        assert_eq!(
            rows(&history),
            5,
            "a group with turns left was dropped, or one with none was kept"
        );
        history
            .db
            .execute("DELETE FROM turns WHERE session = 'b'", [])
            .unwrap();
        waste_hours_hold(&history);
        assert_eq!(
            rows(&history),
            4,
            "an hour whose turns were all forgotten was kept"
        );
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
