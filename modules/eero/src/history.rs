//! The home network's history: what moves, kept for 30 days in SQLite,
//! since eero's cloud gives each figure only as it stands now.
//!
//! After each read of the network the daemon records a sample of each
//! device's signal, SNR, rates and speed, and each node's backhaul bars and
//! each radio's channel, power and how busy it is; a device joining,
//! leaving or roaming to another node or band; every speed test eero gives;
//! and every outage eero records. It keeps at most one sample in any
//! `SAMPLE_EVERY_S`, so a listener asking for reads every few seconds
//! can't make it grow faster than the 30-second pace, and keeping 30 days
//! bounds it.
//!
//! Rows are small on purpose: a device or a radio is a number in each
//! sample, a time is seconds since 1970, and a link rate is whole kbit/s,
//! since eero gives rates in steps of 100 kbit/s. A speed is kept as eero
//! gives it, to the bit per second.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::model::HomeNetwork;

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
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
";

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
        match history.shape()? {
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
        }
    }

    /// A history kept in memory alone, for when the file can't be opened,
    /// so the daemon still draws the last hour while it runs.
    pub fn in_memory() -> History {
        let db = Connection::open_in_memory().expect("SQLite opens a database in memory");
        History { db, pruned: None }
            .made()
            .expect("the tables are made in memory")
    }

    fn connect(path: &Path) -> Result<History, String> {
        let db = Connection::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        db.busy_timeout(Duration::from_secs(5)).map_err(sql)?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(sql)?;
        Ok(History { db, pruned: None })
    }

    fn shape(&self) -> Result<i64, String> {
        self.db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
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
                 COMMIT;"
            ))
            .map_err(sql)?;
        self.pruned = Some(now);
        Ok(())
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

        assert!(moves(&history).is_empty());
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
}
