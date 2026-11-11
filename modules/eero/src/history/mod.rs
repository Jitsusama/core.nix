//! The home network's history: what moves, kept for 30 days in SQLite,
//! since eero's cloud gives each figure only as it stands now.
//!
//! After each read of the network the daemon records a sample of each
//! device's signal, SNR, rates and speed, and each node's backhaul bars and
//! each radio's channel, power and how busy it is; a device joining,
//! leaving or roaming to another node or band; every speed test eero gives;
//! and every outage eero records. After each of the daemon's own looks at
//! the internet (probe.rs) it records what the look found, and an outage
//! the look sees from the home network. It keeps at most one sample in any
//! `SAMPLE_EVERY_S`, so a listener asking for reads every few seconds
//! can't make it grow faster than the 30-second pace, and keeping 30 days
//! bounds it.
//!
//! Rows are small on purpose: a device or a radio is a number in each
//! sample, a time is seconds since 1970, and a link rate is whole kbit/s,
//! since eero gives rates in steps of 100 kbit/s. A speed is kept as eero
//! gives it, to the bit per second.
//!
//! Each model the daemon writes has the history drawn into it: the last
//! hour's samples as series, the last day's roams and comings and goings,
//! and 30 days of speed tests and outages.
//!
//! It also keeps each change a notice told of (notices.rs), by its kind
//! and subject, so a restart never tells one twice, and what the notice
//! said, so the model carries 30 days of what was told.

mod drawing;
mod samples;

use crate::model::HomeNetwork;
use crate::notices::{Kind, Notice, Said, Told};
use crate::probe::Reading;
use rusqlite::{Connection, OptionalExtension, Params, Row, Transaction, params};
use samples::{device_samples, eero_outages, moves, node_samples, speed_tests};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The shape of the tables, as `PRAGMA user_version` holds it. Bumped
/// whenever a table's shape or meaning changes.
const SHAPE: i64 = 1;

/// How long anything is kept.
pub const KEPT_S: i64 = 30 * DAY_S;

const DAY_S: i64 = 24 * 60 * 60;

/// The least time between two samples. Under the 30-second pace, so each
/// read at it is sampled however its time rounds to the second.
pub const SAMPLE_EVERY_S: i64 = 25;

/// The longest gap between two reads across which a device's coming,
/// going or roaming is recorded: twice the slowest pace the network is
/// read at. Past it, nobody was watching, so what changed is taken as the
/// new state without saying when it happened.
pub const WATCHED_GAP_S: i64 = 10 * 60;

/// How often what's past `KEPT_S` is dropped.
const PRUNE_EVERY_S: i64 = 60 * 60;

/// How far back a series in the model reaches.
pub const SERIES_S: i64 = 60 * 60;

/// How far back the model's roams and comings and goings reach, and the
/// most it shows of each, so a device flapping can't swell it.
const MOVES_S: i64 = DAY_S;
const ROAMS_SHOWN: usize = 20;
const JOINS_AND_LEAVES_SHOWN: i64 = 50;

/// The most told changes the model shows, so a node flapping all month
/// can't swell it: a notice's words come to a few hundred bytes, so these
/// take about 10 kB at most.
const TOLD_SHOWN: i64 = 50;

const TABLES: &str = "
CREATE TABLE devices (id INTEGER PRIMARY KEY, mac TEXT NOT NULL UNIQUE, name TEXT);
CREATE TABLE radios (id INTEGER PRIMARY KEY, node TEXT NOT NULL, band TEXT NOT NULL,
    UNIQUE (node, band));
CREATE TABLE device_samples (at INTEGER NOT NULL, device INTEGER NOT NULL,
    signal_dbm INTEGER, snr_db INTEGER, rx_kbps INTEGER, tx_kbps INTEGER,
    down_mbps REAL, up_mbps REAL, PRIMARY KEY (at, device)) WITHOUT ROWID;
CREATE TABLE node_samples (at INTEGER NOT NULL, node TEXT NOT NULL, bars INTEGER,
    PRIMARY KEY (at, node)) WITHOUT ROWID;
CREATE TABLE radio_samples (at INTEGER NOT NULL, radio INTEGER NOT NULL, channel INTEGER,
    busy_pct INTEGER, power_dbm INTEGER, PRIMARY KEY (at, radio)) WITHOUT ROWID;
CREATE TABLE moves (at INTEGER NOT NULL, device INTEGER NOT NULL, kind TEXT NOT NULL,
    node TEXT, band TEXT, from_node TEXT, from_band TEXT,
    PRIMARY KEY (at, device)) WITHOUT ROWID;
CREATE TABLE devices_on (device INTEGER PRIMARY KEY, node TEXT, band TEXT);
CREATE TABLE speed_tests (at INTEGER PRIMARY KEY, down_mbps REAL, up_mbps REAL);
CREATE TABLE eero_outages (record TEXT PRIMARY KEY, seen INTEGER NOT NULL);
CREATE TABLE probes (at INTEGER PRIMARY KEY, home INTEGER NOT NULL, gateway_ms REAL,
    internet_ms REAL, dns_ms REAL, reachable INTEGER NOT NULL);
CREATE TABLE probe_outages (start INTEGER PRIMARY KEY, ended INTEGER,
    last_down INTEGER NOT NULL);
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
";

/// The tables added since `SHAPE` was last bumped, each made where it's
/// missing: a table more changes no table already there, so a history of
/// this shape keeps all it holds.
const ADDED: &str = "
CREATE TABLE IF NOT EXISTS told (kind TEXT NOT NULL, subject TEXT NOT NULL,
    at INTEGER NOT NULL, summary TEXT, body TEXT, PRIMARY KEY (kind, subject)) WITHOUT ROWID;
";

/// The columns added to a table since it was made, each with the table it
/// goes in, added where missing, so a row already there keeps all it held
/// and reads the new column as `NULL`.
const ADDED_COLUMNS: [(&str, &str, &str); 2] =
    [("told", "summary", "TEXT"), ("told", "body", "TEXT")];

/// The history, open.
pub struct History {
    db: Connection,
    /// When what's past `KEPT_S` was last dropped.
    pruned: Option<i64>,
}

impl History {
    /// The history at `path`, made if it isn't there. One of a shape this
    /// daemon doesn't know is set aside beside it, rather than read wrong.
    pub fn open(path: &Path) -> Result<History, String> {
        let history = History::connect(path)?;
        let history = match history.shape()? {
            SHAPE => Ok(history),
            0 => history.made(),
            other => {
                drop(history);
                let aside = aside(path, other);
                for end in ["", "-wal", "-shm"] {
                    let _ = fs::rename(
                        format!("{}{end}", path.display()),
                        format!("{}{end}", aside.display()),
                    );
                }
                History::connect(path)?.made()
            }
        }?;
        history.added()
    }

    /// A history kept in memory alone, for when the file can't be opened,
    /// so the daemon still draws the last hour while it runs.
    pub fn in_memory() -> History {
        let db = Connection::open_in_memory().expect("SQLite opens a database in memory");
        History { db, pruned: None }
            .made()
            .and_then(History::added)
            .expect("the tables are made in memory")
    }

    fn connect(path: &Path) -> Result<History, String> {
        let db = Connection::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        db.busy_timeout(Duration::from_secs(5)).map_err(sql)?;
        // SQLite otherwise keeps the write-ahead log at the largest it ever
        // grew, every frame of it long since checkpointed, as spend-watch
        // found with its own.
        db.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;
             PRAGMA journal_size_limit = 4194304;",
        )
        .map_err(sql)?;
        Ok(History { db, pruned: None })
    }

    fn shape(&self) -> Result<i64, String> {
        self.db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(sql)
    }

    fn added(self) -> Result<History, String> {
        self.db.execute_batch(ADDED).map_err(sql)?;
        for (table, column, kind) in ADDED_COLUMNS {
            let there: bool = self
                .db
                .query_row(
                    "SELECT count(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
                    params![table, column],
                    |row| row.get(0),
                )
                .map_err(sql)?;
            if !there {
                self.db
                    .execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {kind};"))
                    .map_err(sql)?;
            }
        }
        Ok(self)
    }

    /// Records a change at `at`, by its kind and subject, as told though no
    /// notice said it.
    pub fn record_told(&mut self, told: &Told, at: i64) -> Result<(), String> {
        told_row(&self.db, told, None, at)
    }

    /// Records each change a notice told at `at`, with what it said.
    pub fn record_notice(&mut self, notice: &Notice, at: i64) -> Result<(), String> {
        let tx = self.db.transaction().map_err(sql)?;
        for told in &notice.tells {
            told_row(&tx, told, Some(notice), at)?;
        }
        tx.commit().map_err(sql)
    }

    fn was_told(&self, kind: Kind, subject: &str) -> Result<bool, String> {
        self.db
            .query_row(
                "SELECT 1 FROM told WHERE kind = ?1 AND subject = ?2",
                params![kind.name(), subject],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(sql)
    }

    /// The start of the newest outage a notice said began, where none
    /// since said it ended: an outage's notices share its start as their
    /// subject.
    fn told_out(&self) -> Result<Option<String>, String> {
        self.db
            .query_row(
                "SELECT subject FROM told AS out WHERE kind = ?1 AND NOT EXISTS
                 (SELECT 1 FROM told WHERE kind = ?2 AND subject = out.subject)
                 ORDER BY at DESC LIMIT 1",
                params![Kind::InternetOut.name(), Kind::InternetBack.name()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql)
    }

    fn made(self) -> Result<History, String> {
        self.db
            .execute_batch(&format!(
                "BEGIN; {TABLES} PRAGMA user_version = {SHAPE}; COMMIT;"
            ))
            .map_err(sql)?;
        Ok(self)
    }

    /// Records what the model drawn from eero's answers at `at` says:
    /// speed tests and eero's outages always, and when the network itself
    /// was read, a sample of what moves and who came, went or roamed.
    pub fn record(
        &mut self,
        model: &HomeNetwork,
        at: i64,
        network_read: bool,
    ) -> Result<(), String> {
        let tx = self.db.transaction().map_err(sql)?;
        speed_tests(&tx, model, at)?;
        eero_outages(&tx, model, at)?;
        let last = meta(&tx, "sampled")?;
        if network_read && last.is_none_or(|last| at - last >= SAMPLE_EVERY_S) {
            let devices = device_samples(&tx, model, at)?;
            node_samples(&tx, model, at)?;
            moves(&tx, &devices, at, last)?;
            tx.execute("INSERT OR REPLACE INTO meta VALUES ('sampled', ?1)", [at])
                .map_err(sql)?;
        }
        tx.commit().map_err(sql)?;
        self.prune(at)
    }

    /// Records what a look at `at` found, and the outage it shows, if it
    /// was taken from the home network. An outage begins at the first look
    /// that finds the internet out and ends at the first that finds it
    /// back; when no look from home came for over `WATCHED_GAP_S` before
    /// that, as when this machine slept or was away, it ends at the last
    /// look that found it out, since when it came back isn't known.
    pub fn record_probe(&mut self, reading: &Reading, at: i64) -> Result<(), String> {
        let tx = self.db.transaction().map_err(sql)?;
        tx.execute(
            "INSERT OR REPLACE INTO probes VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                at,
                reading.home,
                reading.gateway_ms,
                reading.internet_ms,
                reading.dns_ms,
                reading.reachable
            ],
        )
        .map_err(sql)?;
        if reading.home {
            let watched = meta(&tx, "probed_home")?.is_some_and(|last| at - last <= WATCHED_GAP_S);
            let open: Option<(i64, i64)> = tx
                .query_row(
                    "SELECT start, last_down FROM probe_outages WHERE ended IS NULL",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sql)?;
            let lasts = match open {
                Some((start, last_down)) => {
                    let (ended, down) = match (watched, reading.reachable) {
                        (false, _) => (Some(last_down), last_down),
                        (true, true) => (Some(at), last_down),
                        (true, false) => (None, at),
                    };
                    tx.execute(
                        "UPDATE probe_outages SET ended = ?2, last_down = ?3 WHERE start = ?1",
                        params![start, ended, down],
                    )
                    .map_err(sql)?;
                    ended.is_none()
                }
                None => false,
            };
            if !reading.reachable && !lasts {
                tx.execute(
                    "INSERT OR REPLACE INTO probe_outages VALUES (?1, NULL, ?1)",
                    [at],
                )
                .map_err(sql)?;
            }
            tx.execute(
                "INSERT OR REPLACE INTO meta VALUES ('probed_home', ?1)",
                [at],
            )
            .map_err(sql)?;
        }
        tx.commit().map_err(sql)?;
        self.prune(at)
    }

    fn rows<T>(
        &self,
        query: &str,
        params: impl Params,
        each: impl FnMut(&Row) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, String> {
        let mut statement = self.db.prepare_cached(query).map_err(sql)?;
        statement
            .query_map(params, each)
            .map_err(sql)?
            .collect::<Result<_, _>>()
            .map_err(sql)
    }

    /// Drops what's older than `KEPT_S` before `now`, at most once an
    /// hour. Each sample table leads with its time, so this is a range
    /// cut, and SQLite reuses the pages it frees, so the file stays at
    /// what 30 days take.
    fn prune(&mut self, now: i64) -> Result<(), String> {
        if self.pruned.is_some_and(|at| now - at < PRUNE_EVERY_S) {
            return Ok(());
        }
        let before = now - KEPT_S;
        self.db
            .execute_batch(&format!(
                "BEGIN;
                 DELETE FROM device_samples WHERE at < {before};
                 DELETE FROM node_samples WHERE at < {before};
                 DELETE FROM radio_samples WHERE at < {before};
                 DELETE FROM moves WHERE at < {before};
                 DELETE FROM speed_tests WHERE at < {before};
                 DELETE FROM eero_outages WHERE seen < {before};
                 DELETE FROM probes WHERE at < {before};
                 DELETE FROM probe_outages WHERE coalesce(ended, last_down) < {before};
                 DELETE FROM told WHERE at < {before};
                 COMMIT;"
            ))
            .map_err(sql)?;
        self.pruned = Some(now);
        Ok(())
    }
}

/// What the history says was told, where it can say: one it can't read is
/// taken as untold and said to the journal, since a notice told twice
/// costs less than a change never told.
impl Said for History {
    fn said(&self, kind: Kind, subject: &str) -> bool {
        self.was_told(kind, subject).unwrap_or_else(|reason| {
            eprintln!("eero: {reason}");
            false
        })
    }

    fn out_since(&self) -> Option<String> {
        self.told_out().unwrap_or_else(|reason| {
            eprintln!("eero: {reason}");
            None
        })
    }
}

/// Where a history of another shape is set aside: a name nothing holds
/// yet, so one set aside before is kept too.
fn aside(path: &Path, shape: i64) -> PathBuf {
    (1..=u32::MAX)
        .map(|n| path.with_extension(format!("sqlite.shape-{shape}.{n}")))
        .find(|aside| !aside.exists())
        .expect("a free name")
}

/// Records a change told at `at`, by its kind and subject, with what its
/// notice said where one did. One already recorded keeps the time and
/// words it was first told with.
fn told_row(
    db: &Connection,
    (kind, subject): &Told,
    notice: Option<&Notice>,
    at: i64,
) -> Result<(), String> {
    db.execute(
        "INSERT OR IGNORE INTO told (kind, subject, at, summary, body)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            kind.name(),
            subject,
            at,
            notice.map(|notice| &notice.summary),
            notice.map(|notice| &notice.body)
        ],
    )
    .map(|_| ())
    .map_err(sql)
}

fn meta(tx: &Transaction, key: &str) -> Result<Option<i64>, String> {
    tx.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(sql)
}

/// A link rate kept in whole kbit/s, in Mbit/s.
#[expect(
    clippy::cast_precision_loss,
    reason = "a link rate in kbit/s is far under 2^52, so it's exact as a float"
)]
fn mbps(kbps: Option<i64>) -> Option<f64> {
    kbps.map(|kbps| kbps as f64 / 1000.0)
}

/// A link rate in whole kbit/s.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a link rate is thousands of Mbit/s at most, far inside an i64"
)]
fn kbps(mbps: Option<f64>) -> Option<i64> {
    mbps.map(|mbps| (mbps * 1000.0).round() as i64)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "it's handed to map_err, which gives the error by value"
)]
fn sql(error: rusqlite::Error) -> String {
    format!("the history: {error}")
}

#[cfg(test)]
mod tests;
