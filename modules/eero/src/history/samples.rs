//! What a read of the network leaves in the history: each device's and
//! node's sample, each device's move, and the speed tests and outages
//! eero gives.

use super::{KEPT_S, WATCHED_GAP_S, kbps, sql};
use crate::model::HomeNetwork;
use rusqlite::{Transaction, params};
use std::collections::BTreeMap;

/// Where a device is: its node and its band.
type Place = (Option<String>, Option<String>);

/// Samples each device on now, and gives each one's place by its number.
pub(super) fn device_samples(
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

pub(super) fn node_samples(tx: &Transaction, model: &HomeNetwork, at: i64) -> Result<(), String> {
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
pub(super) fn moves(
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
pub(super) fn speed_tests(tx: &Transaction, model: &HomeNetwork, now: i64) -> Result<(), String> {
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
pub(super) fn eero_outages(tx: &Transaction, model: &HomeNetwork, at: i64) -> Result<(), String> {
    for outage in &model.internet.outages {
        tx.execute(
            "INSERT OR IGNORE INTO eero_outages VALUES (?1, ?2)",
            params![outage.to_string(), at],
        )
        .map_err(sql)?;
    }
    Ok(())
}
