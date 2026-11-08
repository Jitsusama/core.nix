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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Params, Row, Transaction, params};
use serde_json::Value;

use crate::model::{
    DeviceHistory, HomeNetwork, JoinOrLeave, JoinOrLeaveKind, NodeHistory, Outage, OutageSeenBy,
    Probe, ProbeHistory, RadioHistory, Roam, SpeedTest, ToldChange,
};
use crate::notices::{Kind, Notice, Said, Told};
use crate::probe::Reading;
use crate::time;

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

    /// Draws what the history holds into the model, as it stands at `now`,
    /// replacing what an earlier drawing put there.
    pub fn draw(&self, model: &mut HomeNetwork, now: i64) -> Result<(), String> {
        let mut devices = self.device_series(now - SERIES_S)?;
        let mut roams = self.roams(now - MOVES_S)?;
        for device in &mut model.devices {
            device.history = devices.remove(&device.mac).unwrap_or_default();
            device.roams = roams.remove(&device.mac).unwrap_or_default();
        }
        let mut nodes = self.node_series(now - SERIES_S)?;
        let mut radios = self.radio_series(now - SERIES_S)?;
        for node in &mut model.nodes {
            node.history = nodes.remove(&node.id).unwrap_or_default();
            for radio in &mut node.radios {
                let key = (node.id.clone(), radio.band.clone());
                radio.history = radios.remove(&key).unwrap_or_default();
            }
        }
        model.joins_and_leaves = self.joins_and_leaves(now - MOVES_S)?;
        let internet = &mut model.internet;
        internet.speed_tests = self.speed_tests(&internet.speed_tests, now - KEPT_S)?;
        internet.history.outages = self.outages(now - KEPT_S)?;
        internet.history.probes = self.probe_series(now - SERIES_S)?;
        internet.probe = self.last_probe()?;
        self.draw_told(model, now)
    }

    /// Draws what was told into the model, as it stands at `now`, so a
    /// notice raised after the rest was drawn is in the model it's told
    /// from.
    pub fn draw_told(&self, model: &mut HomeNetwork, now: i64) -> Result<(), String> {
        model.told = self.told(now - KEPT_S)?;
        Ok(())
    }

    /// The changes told with words since `since`, newest first: a notice
    /// telling of several, as the nodes updating together, is one, since
    /// each change it told was recorded with its words at its time. Those
    /// recorded before notices kept their words have none to show, and an
    /// event eero lists is shown from its list.
    fn told(&self, since: i64) -> Result<Vec<ToldChange>, String> {
        self.rows(
            "SELECT DISTINCT at, kind, summary, body FROM told
             WHERE summary IS NOT NULL AND kind != ?1 AND at > ?2
             ORDER BY at DESC, kind, summary LIMIT ?3",
            params![Kind::Event.name(), since, TOLD_SHOWN],
            |row| {
                Ok(ToldChange {
                    at: time::from_unix(row.get(0)?),
                    kind: row.get(1)?,
                    summary: row.get(2)?,
                    body: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            },
        )
    }

    fn device_series(&self, since: i64) -> Result<BTreeMap<String, DeviceHistory>, String> {
        let mut series: BTreeMap<String, DeviceHistory> = BTreeMap::new();
        let rows = self.rows(
            "SELECT mac, at, signal_dbm, snr_db, rx_kbps, tx_kbps, down_mbps, up_mbps
             FROM device_samples JOIN devices ON devices.id = device
             WHERE at > ?1 ORDER BY at",
            [since],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )?;
        for (mac, at, signal, snr, rx, tx, down, up) in rows {
            let device = series.entry(mac).or_default();
            device.at.push(time::from_unix(at));
            device.signal_dbm.push(signal);
            device.snr_db.push(snr);
            device.rx_mbps.push(mbps(rx));
            device.tx_mbps.push(mbps(tx));
            device.down_mbps.push(down);
            device.up_mbps.push(up);
        }
        Ok(series)
    }

    fn node_series(&self, since: i64) -> Result<BTreeMap<String, NodeHistory>, String> {
        let mut series: BTreeMap<String, NodeHistory> = BTreeMap::new();
        let rows = self.rows(
            "SELECT node, at, bars FROM node_samples WHERE at > ?1 ORDER BY at",
            [since],
            |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?)),
        )?;
        for (node, at, bars) in rows {
            let node = series.entry(node).or_default();
            node.at.push(time::from_unix(at));
            node.bars.push(bars);
        }
        Ok(series)
    }

    /// Each radio's series, by its node's id and its band.
    fn radio_series(&self, since: i64) -> Result<BTreeMap<(String, String), RadioHistory>, String> {
        let mut series: BTreeMap<(String, String), RadioHistory> = BTreeMap::new();
        let rows = self.rows(
            "SELECT node, band, at, channel, busy_pct, power_dbm
             FROM radio_samples JOIN radios ON radios.id = radio
             WHERE at > ?1 ORDER BY at",
            [since],
            |row| {
                Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;
        for (radio, at, channel, busy, power) in rows {
            let radio = series.entry(radio).or_default();
            radio.at.push(time::from_unix(at));
            radio.channel.push(channel);
            radio.busy_pct.push(busy);
            radio.power_dbm.push(power);
        }
        Ok(series)
    }

    /// Each device's roams since `since`, by its hardware address, newest
    /// first.
    fn roams(&self, since: i64) -> Result<BTreeMap<String, Vec<Roam>>, String> {
        let mut roams: BTreeMap<String, Vec<Roam>> = BTreeMap::new();
        let rows = self.rows(
            "SELECT mac, at, from_node, node, from_band, band
             FROM moves JOIN devices ON devices.id = device
             WHERE kind = 'roamed' AND at > ?1 ORDER BY at DESC",
            [since],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Roam {
                        at: time::from_unix(row.get(1)?),
                        from_node: row.get(2)?,
                        node: row.get(3)?,
                        from_band: row.get(4)?,
                        band: row.get(5)?,
                    },
                ))
            },
        )?;
        for (mac, roam) in rows {
            let device = roams.entry(mac).or_default();
            if device.len() < ROAMS_SHOWN {
                device.push(roam);
            }
        }
        Ok(roams)
    }

    fn joins_and_leaves(&self, since: i64) -> Result<Vec<JoinOrLeave>, String> {
        self.rows(
            "SELECT at, kind, mac, name, node, band
             FROM moves JOIN devices ON devices.id = device
             WHERE kind IN ('joined', 'left') AND at > ?1
             ORDER BY at DESC, mac LIMIT ?2",
            [since, JOINS_AND_LEAVES_SHOWN],
            |row| {
                Ok(JoinOrLeave {
                    at: time::from_unix(row.get(0)?),
                    kind: if row.get::<_, String>(1)? == "joined" {
                        JoinOrLeaveKind::Joined
                    } else {
                        JoinOrLeaveKind::Left
                    },
                    mac: row.get(2)?,
                    name: row.get(3)?,
                    node: row.get(4)?,
                    band: row.get(5)?,
                })
            },
        )
    }

    /// eero's speed tests and those kept since `since` that eero no longer
    /// gives, newest first.
    fn speed_tests(&self, eeros: &[SpeedTest], since: i64) -> Result<Vec<SpeedTest>, String> {
        let at = |test: &SpeedTest| test.at.as_deref().and_then(time::to_unix);
        let given: Vec<i64> = eeros.iter().filter_map(at).collect();
        let kept = self.rows(
            "SELECT at, down_mbps, up_mbps FROM speed_tests WHERE at > ?1",
            [since],
            |row| Ok((row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let mut tests = eeros.to_vec();
        tests.extend(
            kept.into_iter()
                .filter(|(at, _, _)| !given.contains(at))
                .map(|(at, down_mbps, up_mbps)| SpeedTest {
                    at: Some(time::from_unix(at)),
                    down_mbps,
                    up_mbps,
                }),
        );
        tests.sort_by_key(|test| std::cmp::Reverse(at(test)));
        Ok(tests)
    }

    /// The outages since `since`, newest first: the probe's by when they
    /// began, eero's by when they were first seen.
    fn outages(&self, since: i64) -> Result<Vec<Outage>, String> {
        let mut outages = self.rows(
            "SELECT start, ended FROM probe_outages WHERE coalesce(ended, last_down) > ?1",
            [since],
            |row| {
                let start: i64 = row.get(0)?;
                let ended: Option<i64> = row.get(1)?;
                let outage = Outage {
                    seen_by: OutageSeenBy::Probe,
                    start: Some(time::from_unix(start)),
                    end: ended.map(time::from_unix),
                    eero: None,
                };
                Ok((start, outage))
            },
        )?;
        outages.extend(self.rows(
            "SELECT record, seen FROM eero_outages WHERE seen > ?1",
            [since],
            |row| {
                let record: String = row.get(0)?;
                let outage = Outage {
                    seen_by: OutageSeenBy::Eero,
                    start: None,
                    end: None,
                    eero: serde_json::from_str::<Value>(&record).ok(),
                };
                Ok((row.get(1)?, outage))
            },
        )?);
        outages.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        Ok(outages.into_iter().map(|(_, outage)| outage).collect())
    }

    fn probe_series(&self, since: i64) -> Result<ProbeHistory, String> {
        let mut series = ProbeHistory::default();
        for probe in self.probes("WHERE at > ?1 ORDER BY at", since)? {
            series.at.push(probe.at);
            series.gateway_ms.push(probe.gateway_ms);
            series.internet_ms.push(probe.internet_ms);
            series.dns_ms.push(probe.dns_ms);
            series.reachable.push(probe.reachable);
        }
        Ok(series)
    }

    /// The newest look, however old: its time says how current it is.
    fn last_probe(&self) -> Result<Option<Probe>, String> {
        Ok(self
            .probes("WHERE at > ?1 ORDER BY at DESC LIMIT 1", i64::MIN)?
            .pop())
    }

    /// The looks `rest` of a query picks, from `since`.
    fn probes(&self, rest: &str, since: i64) -> Result<Vec<Probe>, String> {
        self.rows(
            &format!(
                "SELECT at, home, gateway_ms, internet_ms, dns_ms, reachable FROM probes {rest}"
            ),
            [since],
            |row| {
                Ok(Probe {
                    at: time::from_unix(row.get(0)?),
                    home: row.get(1)?,
                    gateway_ms: row.get(2)?,
                    internet_ms: row.get(3)?,
                    dns_ms: row.get(4)?,
                    reachable: row.get(5)?,
                })
            },
        )
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

/// Where a device is: its node and its band.
type Place = (Option<String>, Option<String>);

/// Samples each device on now, and gives each one's place by its number.
fn device_samples(
    tx: &Transaction,
    model: &HomeNetwork,
    at: i64,
) -> Result<BTreeMap<i64, Place>, String> {
    let mut places = BTreeMap::new();
    for device in &model.devices {
        let id: i64 = tx
            .query_row(
                "INSERT INTO devices (mac, name) VALUES (?1, ?2)
                 ON CONFLICT (mac) DO UPDATE SET name = coalesce(excluded.name, name)
                 RETURNING id",
                params![device.mac, device.name],
                |row| row.get(0),
            )
            .map_err(sql)?;
        tx.execute(
            "INSERT OR REPLACE INTO device_samples VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                at,
                id,
                device.signal_dbm,
                device.snr_db,
                kbps(device.rx.as_ref().and_then(|r| r.mbps)),
                kbps(device.tx.as_ref().and_then(|r| r.mbps)),
                device.speed.down_mbps,
                device.speed.up_mbps,
            ],
        )
        .map_err(sql)?;
        places.insert(id, (device.node.clone(), device.band.clone()));
    }
    Ok(places)
}

fn node_samples(tx: &Transaction, model: &HomeNetwork, at: i64) -> Result<(), String> {
    for node in &model.nodes {
        tx.execute(
            "INSERT OR REPLACE INTO node_samples VALUES (?1, ?2, ?3)",
            params![at, node.id, node.backhaul.bars],
        )
        .map_err(sql)?;
        for radio in &node.radios {
            let id: i64 = tx
                .query_row(
                    "INSERT INTO radios (node, band) VALUES (?1, ?2)
                     ON CONFLICT (node, band) DO UPDATE SET node = node RETURNING id",
                    params![node.id, radio.band],
                    |row| row.get(0),
                )
                .map_err(sql)?;
            tx.execute(
                "INSERT OR REPLACE INTO radio_samples VALUES (?1, ?2, ?3, ?4, ?5)",
                params![at, id, radio.channel, radio.busy_pct, radio.power_dbm],
            )
            .map_err(sql)?;
        }
    }
    Ok(())
}

/// Records each device that came, went or roamed since the last sample
/// at `last`, if that was close enough to say it happened between, and
/// keeps who's where now for the next.
fn moves(
    tx: &Transaction,
    now: &BTreeMap<i64, Place>,
    at: i64,
    last: Option<i64>,
) -> Result<(), String> {
    let mut statement = tx
        .prepare("SELECT device, node, band FROM devices_on")
        .map_err(sql)?;
    let before: BTreeMap<i64, Place> = statement
        .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))
        .map_err(sql)?
        .collect::<Result<_, _>>()
        .map_err(sql)?;
    drop(statement);
    if last.is_some_and(|last| at - last <= WATCHED_GAP_S) {
        let record = |device: i64, kind: &str, place: &Place, from: Option<&Place>| {
            tx.execute(
                "INSERT OR REPLACE INTO moves VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    at,
                    device,
                    kind,
                    place.0,
                    place.1,
                    from.and_then(|f| f.0.as_ref()),
                    from.and_then(|f| f.1.as_ref()),
                ],
            )
            .map(|_| ())
            .map_err(sql)
        };
        for (device, place) in now {
            match before.get(device) {
                None => record(*device, "joined", place, None)?,
                Some(was) if was != place => record(*device, "roamed", place, Some(was))?,
                Some(_) => {}
            }
        }
        for (device, was) in before.iter().filter(|(d, _)| !now.contains_key(d)) {
            record(*device, "left", was, None)?;
        }
    }
    tx.execute("DELETE FROM devices_on", []).map_err(sql)?;
    for (device, (node, band)) in now {
        tx.execute(
            "INSERT INTO devices_on VALUES (?1, ?2, ?3)",
            params![device, node, band],
        )
        .map_err(sql)?;
    }
    Ok(())
}

/// Every speed test the model names from the 30 days before `now`, kept
/// once each by its time.
fn speed_tests(tx: &Transaction, model: &HomeNetwork, now: i64) -> Result<(), String> {
    let internet = &model.internet;
    for test in std::iter::once(&internet.speed_test).chain(&internet.speed_tests) {
        let at = test.at.as_deref().and_then(crate::time::to_unix);
        let Some(at) = at.filter(|at| now - at <= KEPT_S) else {
            continue;
        };
        tx.execute(
            "INSERT OR IGNORE INTO speed_tests VALUES (?1, ?2, ?3)",
            params![at, test.down_mbps, test.up_mbps],
        )
        .map_err(sql)?;
    }
    Ok(())
}

/// Every outage eero records, kept as eero wrote it from when it was
/// first seen, so it outlasts eero's own list.
fn eero_outages(tx: &Transaction, model: &HomeNetwork, at: i64) -> Result<(), String> {
    for outage in &model.internet.outages {
        tx.execute(
            "INSERT OR IGNORE INTO eero_outages VALUES (?1, ?2)",
            params![outage.to_string(), at],
        )
        .map_err(sql)?;
    }
    Ok(())
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
mod tests {
    use super::*;
    use crate::golden::MODEL;
    use crate::model::Device;

    const AT: i64 = 1_794_091_483;

    fn model() -> HomeNetwork {
        serde_json::from_str(MODEL).unwrap()
    }

    fn count(history: &History, table: &str) -> i64 {
        history
            .db
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    /// Each move recorded, as `kind mac node band from_node from_band`.
    fn moves(history: &History) -> Vec<String> {
        let mut statement = history
            .db
            .prepare(
                "SELECT kind, mac, node, band, from_node, from_band FROM moves
                 JOIN devices ON devices.id = moves.device ORDER BY at, mac",
            )
            .unwrap();
        statement
            .query_map([], |row| {
                let words: Vec<String> = (0..6)
                    .map(|i| {
                        row.get::<_, Option<String>>(i)
                            .map(Option::unwrap_or_default)
                    })
                    .collect::<Result<_, _>>()?;
                Ok(words.join(" ").trim_end().to_owned())
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn a_network_read_samples_each_device_node_and_radio() {
        let mut history = History::in_memory();

        history.record(&model(), AT, true).unwrap();

        assert_eq!(count(&history, "device_samples"), 15);
        assert_eq!(count(&history, "node_samples"), 3);
        assert_eq!(count(&history, "radio_samples"), 9);
        let (signal, rx, down): (i64, i64, f64) = history
            .db
            .query_row(
                "SELECT signal_dbm, rx_kbps, down_mbps FROM device_samples
                 JOIN devices ON devices.id = device WHERE mac = '02:00:00:00:00:08'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!((signal, rx, down), (-57, 1_200_900, 11.452_299));
    }

    #[test]
    fn a_read_that_isnt_the_networks_samples_nothing() {
        let mut history = History::in_memory();

        history.record(&model(), AT, false).unwrap();

        assert_eq!(count(&history, "device_samples"), 0);
        assert_eq!(
            count(&history, "speed_tests"),
            10,
            "the speed tests of the last 30 days are kept"
        );
    }

    #[test]
    fn reads_closer_than_the_sampling_pace_add_no_sample() {
        let mut history = History::in_memory();

        history.record(&model(), AT, true).unwrap();
        history.record(&model(), AT + 5, true).unwrap();
        history.record(&model(), AT + SAMPLE_EVERY_S, true).unwrap();

        assert_eq!(count(&history, "device_samples"), 30);
    }

    #[test]
    fn a_speed_test_is_kept_once_however_often_it_is_read() {
        let mut history = History::in_memory();

        history.record(&model(), AT, true).unwrap();
        history.record(&model(), AT + 30, true).unwrap();

        assert_eq!(count(&history, "speed_tests"), 10);
    }

    #[test]
    fn coming_going_and_roaming_are_recorded_between_watched_reads() {
        let mut history = History::in_memory();
        let first = model();
        let mut second = first.clone();
        second.devices.retain(|d| d.mac != "02:00:00:00:00:03");
        let mut roamer = second.devices.remove(0);
        roamer.node = Some("2002".into());
        roamer.band = Some("2.4 GHz".into());
        let mut newcomer = roamer.clone();
        newcomer.mac = "02:00:00:00:00:40".into();
        second.devices.extend([roamer, newcomer]);

        history.record(&first, AT, true).unwrap();
        history.record(&second, AT + 30, true).unwrap();

        assert_eq!(
            moves(&history),
            [
                "left 02:00:00:00:00:03 2002 5 GHz",
                "roamed 02:00:00:00:00:0a 2002 2.4 GHz 2001 5 GHz",
                "joined 02:00:00:00:00:40 2002 2.4 GHz",
            ]
        );
    }

    #[test]
    fn nobody_comes_or_goes_on_the_first_read_or_after_a_gap() {
        let mut history = History::in_memory();
        let mut fewer = model();
        fewer.devices.truncate(3);

        history.record(&model(), AT, true).unwrap();
        history
            .record(&fewer, AT + WATCHED_GAP_S + 1, true)
            .unwrap();

        assert_eq!(moves(&history), Vec::<String>::new());
    }

    #[test]
    fn what_is_older_than_30_days_is_dropped() {
        let mut history = History::in_memory();
        history.record(&model(), AT, true).unwrap();

        history.record(&model(), AT + KEPT_S + 1, true).unwrap();

        assert_eq!(count(&history, "device_samples"), 15);
        assert_eq!(
            count(&history, "speed_tests"),
            0,
            "the contract's are older"
        );
    }

    #[test]
    fn an_outage_eero_records_is_kept_once_from_when_it_was_seen() {
        let mut history = History::in_memory();
        let mut model = model();
        model.internet.outages = vec![serde_json::json!({"kind": "isp"})];

        history.record(&model, AT, false).unwrap();
        history.record(&model, AT + 60, false).unwrap();

        let seen: i64 = history
            .db
            .query_row("SELECT seen FROM eero_outages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(seen, AT);
    }

    #[test]
    fn the_model_carries_the_last_hours_samples_oldest_first() {
        let mut history = History::in_memory();
        let mut model = model();
        for (minutes, signal) in [(90, -80), (50, -70), (20, -60)] {
            model.devices[0].signal_dbm = Some(signal);
            history.record(&model, AT - minutes * 60, true).unwrap();
        }

        history.draw(&mut model, AT).unwrap();

        let series = &model.devices[0].history;
        assert_eq!(series.signal_dbm, [Some(-70), Some(-60)]);
        assert_eq!(
            series.at,
            [time::from_unix(AT - 3000), time::from_unix(AT - 1200)]
        );
        assert_eq!(series.rx_mbps, [Some(1200.9), Some(1200.9)]);
        assert_eq!(model.nodes[0].history.bars.len(), 2);
        assert_eq!(model.nodes[0].radios[2].history.busy_pct.len(), 2);
    }

    #[test]
    fn a_device_with_no_samples_has_an_empty_series() {
        let history = History::in_memory();
        let mut model = model();

        history.draw(&mut model, AT).unwrap();

        assert_eq!(model.devices[0].history, DeviceHistory::default());
        assert_eq!(model.devices[0].roams, Vec::<Roam>::new());
        assert_eq!(model.joins_and_leaves, Vec::<JoinOrLeave>::new());
    }

    #[test]
    fn a_device_shows_its_last_days_roams_newest_first_and_at_most_20() {
        let mut history = History::in_memory();
        let mut model = model();
        let start = AT - DAY_S - 30 * 60;
        for read in 0..60 {
            let band = if read % 2 == 0 { "5 GHz" } else { "2.4 GHz" };
            model.devices[0].band = Some(band.into());
            history.record(&model, start + read * 60, true).unwrap();
        }
        let now = start + 59 * 60;

        history.draw(&mut model, now).unwrap();

        let roams = &model.devices[0].roams;
        assert_eq!(roams.len(), ROAMS_SHOWN);
        assert_eq!(roams[0].at, time::from_unix(now));
        assert_eq!(roams[0].band.as_deref(), Some("2.4 GHz"));
        assert_eq!(roams[0].from_band.as_deref(), Some("5 GHz"));
        assert_eq!(roams[0].from_node, roams[0].node);
    }

    #[test]
    fn a_speed_test_eero_has_dropped_is_still_shown_in_its_place() {
        let mut history = History::in_memory();
        let mut model = model();
        history.record(&model, AT, false).unwrap();
        let dropped = model.internet.speed_tests.remove(1);

        history.draw(&mut model, AT).unwrap();

        assert_eq!(model.internet.speed_tests.len(), 15);
        assert_eq!(model.internet.speed_tests[1], dropped);
    }

    #[test]
    fn an_outage_eero_recorded_is_drawn_as_eero_wrote_it() {
        let mut history = History::in_memory();
        let mut model = model();
        model.internet.outages = vec![serde_json::json!({"kind": "isp"})];
        history.record(&model, AT, false).unwrap();
        model.internet.outages.clear();

        history.draw(&mut model, AT + 60).unwrap();

        let outage = &model.internet.history.outages[0];
        assert_eq!(outage.seen_by, OutageSeenBy::Eero);
        assert_eq!((&outage.start, &outage.end), (&None, &None));
        assert_eq!(outage.eero, Some(serde_json::json!({"kind": "isp"})));
    }

    /// A look from home, out or not, and with what times.
    fn look(home: bool, reachable: bool) -> Reading {
        Reading {
            home,
            gateway_ms: home.then_some(1.9),
            internet_ms: reachable.then_some(12.4),
            dns_ms: (home && reachable).then_some(14.8),
            reachable,
        }
    }

    /// The probe's outages drawn, as `start..end`, in seconds after `AT`.
    fn probe_outages(history: &History, now: i64) -> Vec<String> {
        let mut model = model();
        history.draw(&mut model, now).unwrap();
        let after = |at: &Option<String>| {
            at.as_deref()
                .map(|at| (time::to_unix(at).unwrap() - AT).to_string())
                .unwrap_or_default()
        };
        model
            .internet
            .history
            .outages
            .iter()
            .filter(|outage| outage.seen_by == OutageSeenBy::Probe)
            .map(|outage| format!("{}..{}", after(&outage.start), after(&outage.end)))
            .collect()
    }

    fn looks(history: &mut History, from: i64, every: i64, found: &[(bool, bool)]) {
        for (n, (home, reachable)) in (0..).zip(found) {
            history
                .record_probe(&look(*home, *reachable), from + n * every)
                .unwrap();
        }
    }

    #[test]
    fn an_outage_the_probe_sees_runs_from_the_first_look_out_to_the_first_back() {
        let mut history = History::in_memory();
        let (up, down) = ((true, true), (true, false));

        looks(&mut history, AT, 10, &[up, down, down, down, up, up, down]);

        assert_eq!(probe_outages(&history, AT + 60), ["60..", "10..40"]);
    }

    #[test]
    fn looks_away_from_home_record_no_outage_and_leave_one_open() {
        let mut history = History::in_memory();
        let away_out = (false, false);

        looks(&mut history, AT, 10, &[away_out, away_out, (true, false)]);
        looks(&mut history, AT + 30, 10, &[away_out, (true, true)]);

        assert_eq!(probe_outages(&history, AT + 40), ["20..40"]);
    }

    #[test]
    fn after_nobody_looked_an_outage_ends_at_the_last_look_that_saw_it() {
        let mut history = History::in_memory();
        let down = (true, false);

        looks(&mut history, AT, 10, &[down, down]);
        looks(&mut history, AT + 3600, 10, &[down, (true, true)]);

        assert_eq!(probe_outages(&history, AT + 3610), ["3600..3610", "0..10"]);
    }

    #[test]
    fn the_model_carries_the_last_look_and_the_last_hours_looks() {
        let mut history = History::in_memory();
        let mut model = model();
        looks(&mut history, AT - 4000, 1000, &[(true, true); 5]);

        history.draw(&mut model, AT).unwrap();

        let probe = model.internet.probe.unwrap();
        assert_eq!(probe.at, time::from_unix(AT));
        assert_eq!(
            (probe.gateway_ms, probe.internet_ms, probe.dns_ms),
            (Some(1.9), Some(12.4), Some(14.8))
        );
        assert!(probe.home && probe.reachable);
        let looks = &model.internet.history.probes;
        assert_eq!(looks.at.len(), 4, "the last hour's");
        assert_eq!(looks.reachable, [true; 4]);
    }

    /// The model of a busy network after an hour read at the fast pace: 32
    /// devices, each with every figure, and the golden model's three nodes.
    /// It comes to about 336 kB, a third of it the series' times: written
    /// every 30 seconds that's about 11 kB/s into the runtime folder, which
    /// is memory, and a few milliseconds of the shell's parsing.
    #[test]
    fn a_busy_hour_keeps_the_model_small_enough_to_write_every_30_seconds() {
        let mut history = History::in_memory();
        let mut model = model();
        let pattern = model.devices[4].clone();
        model.devices = (0..32)
            .map(|n| Device {
                mac: format!("02:00:00:00:01:{n:02x}"),
                ..pattern.clone()
            })
            .collect();
        for read in 0..120 {
            history.record(&model, AT + read * 30, true).unwrap();
        }

        history.draw(&mut model, AT + 119 * 30).unwrap();

        let bytes = serde_json::to_string(&model).unwrap().len();
        assert_eq!(model.devices[0].history.at.len(), 120);
        assert!(bytes < 350_000, "the model takes {bytes} bytes");
    }

    #[test]
    fn a_history_of_another_shape_is_set_aside() {
        let folder = std::env::temp_dir().join(format!("eero-history-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("history.sqlite");
        let _ = fs::remove_file(&path);
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 99;")
            .unwrap();

        let history = History::open(&path).unwrap();

        assert_eq!(history.shape().unwrap(), SHAPE);
        assert!(folder.join("history.sqlite.shape-99.1").exists());
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_history_of_this_shape_from_before_the_notices_keeps_what_it_holds() {
        let folder = std::env::temp_dir().join(format!("eero-told-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("history.sqlite");
        let _ = fs::remove_file(&path);
        Connection::open(&path)
            .unwrap()
            .execute_batch(&format!(
                "BEGIN; {TABLES} PRAGMA user_version = {SHAPE};
                 INSERT INTO speed_tests VALUES ({AT}, 525.0, 48.1); COMMIT;"
            ))
            .unwrap();

        let mut history = History::open(&path).unwrap();
        history
            .record_told(&(Kind::Joined, "02:00:00:00:00:20".into()), AT)
            .unwrap();

        assert_eq!(count(&history, "speed_tests"), 1);
        assert!(history.said(Kind::Joined, "02:00:00:00:00:20"));
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_change_is_said_once_told_and_an_outage_stands_until_told_back() {
        let mut history = History::in_memory();
        let start = time::from_unix(AT);

        assert!(!history.said(Kind::InternetOut, &start));
        history
            .record_told(&(Kind::InternetOut, start.clone()), AT)
            .unwrap();
        let out = history.out_since();
        history
            .record_told(&(Kind::InternetBack, start.clone()), AT + 60)
            .unwrap();

        assert!(history.said(Kind::InternetOut, &start));
        assert_eq!(out, Some(start));
        assert_eq!(history.out_since(), None);
    }

    fn notice(kind: Kind, subjects: &[&str], summary: &str, body: &str) -> Notice {
        Notice {
            about: None,
            summary: summary.into(),
            body: body.into(),
            urgency: crate::notices::Urgency::Normal,
            tells: subjects.iter().map(|s| (kind, (*s).to_owned())).collect(),
        }
    }

    /// The told changes drawn at `now`, as `seconds-after-AT kind summary: body`.
    fn told(history: &History, now: i64) -> Vec<String> {
        let mut model = model();
        history.draw(&mut model, now).unwrap();
        model
            .told
            .iter()
            .map(|told| {
                let after = time::to_unix(&told.at).unwrap() - AT;
                format!("{after} {} {}: {}", told.kind, told.summary, told.body)
            })
            .collect()
    }

    #[test]
    fn the_model_carries_what_each_notice_said_newest_first_and_once() {
        let mut history = History::in_memory();
        let out = notice(
            Kind::InternetOut,
            &["t0"],
            "The internet is out",
            "Nothing.",
        );
        let updated = notice(
            Kind::Updated,
            &["2001 v7.18.0-1", "2002 v7.18.0-1"],
            "The nodes updated to v7.18.0-1",
            "From v7.17.4-5.",
        );
        let event = notice(Kind::Event, &["e"], "Guest network turned on.", "eero's.");

        history.record_notice(&out, AT).unwrap();
        history.record_notice(&updated, AT + 60).unwrap();
        history.record_notice(&event, AT + 90).unwrap();
        history
            .record_told(&(Kind::InternetBack, "t0".into()), AT + 120)
            .unwrap();
        history.record_notice(&out, AT + 180).unwrap();

        assert_eq!(
            told(&history, AT + 200),
            [
                "60 updated The nodes updated to v7.18.0-1: From v7.17.4-5.",
                "0 internet-out The internet is out: Nothing.",
            ],
            "one notice of two changes is one, a change settled without a notice and an \
             event eero lists aren't, and one told again keeps its first time"
        );
    }

    #[test]
    fn the_model_carries_30_days_of_told_changes_and_at_most_50() {
        let mut history = History::in_memory();
        for n in 0..60 {
            let subject = n.to_string();
            let words = notice(
                Kind::Joined,
                &[subject.as_str()],
                &format!("device {n}"),
                "",
            );
            history.record_notice(&words, AT + n * 60).unwrap();
        }

        let shown = told(&history, AT + 59 * 60);
        let past = told(&history, AT + KEPT_S + 30 * 60);

        assert_eq!(shown.len(), 50);
        assert_eq!(shown[0], "3540 joined device 59: ");
        assert_eq!(past.len(), 29, "those past 30 days are left out");
    }

    #[test]
    fn a_history_told_before_notices_kept_their_words_keeps_its_rows() {
        let folder = std::env::temp_dir().join(format!("eero-words-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("history.sqlite");
        let _ = fs::remove_file(&path);
        Connection::open(&path)
            .unwrap()
            .execute_batch(&format!(
                "BEGIN; {TABLES} PRAGMA user_version = {SHAPE};
                 CREATE TABLE told (kind TEXT NOT NULL, subject TEXT NOT NULL,
                     at INTEGER NOT NULL, PRIMARY KEY (kind, subject)) WITHOUT ROWID;
                 INSERT INTO told VALUES ('joined', '02:00:00:00:00:20', {AT}); COMMIT;"
            ))
            .unwrap();

        let mut history = History::open(&path).unwrap();
        let joined = notice(Kind::Joined, &["02:00:00:00:00:21"], "phone 3", "New.");
        history.record_notice(&joined, AT + 60).unwrap();

        assert!(history.said(Kind::Joined, "02:00:00:00:00:20"));
        assert_eq!(told(&history, AT + 60), ["60 joined phone 3: New."]);
        drop(history);
        assert!(
            History::open(&path).is_ok(),
            "a history whose words were added opens again"
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn what_was_told_is_dropped_after_30_days() {
        let mut history = History::in_memory();
        history
            .record_told(&(Kind::Updated, "2001 v7.18.0-1".into()), AT)
            .unwrap();

        history.record(&model(), AT + KEPT_S + 1, false).unwrap();

        assert!(!history.said(Kind::Updated, "2001 v7.18.0-1"));
    }
}
