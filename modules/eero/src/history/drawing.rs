//! The history drawn into each model the daemon writes: the last hour's
//! samples as series, the last day's roams and comings and goings, 30
//! days of speed tests and outages, the probes and what was told.

use super::{
    History, JOINS_AND_LEAVES_SHOWN, KEPT_S, MOVES_S, ROAMS_SHOWN, SERIES_S, TOLD_SHOWN, mbps,
};
use crate::model::{
    DeviceHistory, HomeNetwork, JoinOrLeave, JoinOrLeaveKind, NodeHistory, Outage, OutageSeenBy,
    Probe, ProbeHistory, RadioHistory, Roam, SpeedTest, ToldChange,
};
use crate::notices::Kind;
use crate::time;
use rusqlite::params;
use serde_json::Value;
use std::collections::BTreeMap;

impl History {
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
}
