//! A day of the probe's looks, said as the figures a review holds the home
//! network to: how many looks were taken from home and away, the 95th
//! percentile of each time a look from home took, how many found the
//! internet out, and the minutes the internet was out as the looks saw it.
//! Only looks from home count, since one from elsewhere says nothing of
//! the home network. These are idle-time figures, not under load: a look
//! is a few small connects every few minutes.

use rusqlite::params;
use serde::Serialize;

use super::{History, sql};

/// The share of looks a percentile leaves below it.
const PERCENTILE: f64 = 0.95;

const MINUTE_S: f64 = 60.0;

/// A day's figures, as the day's file holds them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Day {
    /// The day, in the machine's own zone, as `2026-11-12`.
    pub date: String,
    /// Looks taken from home.
    pub probes: u32,
    /// Looks taken from elsewhere, which count for nothing else.
    pub away_probes: u32,
    pub gateway_p95_ms: Option<f64>,
    pub internet_p95_ms: Option<f64>,
    pub dns_p95_ms: Option<f64>,
    /// Looks from home that found the internet out.
    pub unreachable: u32,
    /// Minutes the internet was out, as the looks from home saw it, to a
    /// tenth.
    pub outage_minutes: f64,
}

impl History {
    /// The figures of the looks taken from `from` up to `to`, in seconds
    /// since 1970, the day being `date`.
    pub fn day(&self, date: &str, from: i64, to: i64) -> Result<Day, String> {
        let looks: Vec<(bool, Option<f64>, Option<f64>, Option<f64>, bool)> = self
            .db
            .prepare(
                "SELECT home, gateway_ms, internet_ms, dns_ms, reachable FROM probes
                 WHERE at >= ?1 AND at < ?2",
            )
            .and_then(|mut statement| {
                statement
                    .query_map(params![from, to], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    })?
                    .collect()
            })
            .map_err(sql)?;
        let home: Vec<_> = looks.iter().filter(|look| look.0).collect();
        let out_s: i64 = self
            .db
            .query_row(
                "SELECT coalesce(sum(min(coalesce(ended, last_down), ?2) - max(start, ?1)), 0)
                 FROM probe_outages WHERE start < ?2 AND coalesce(ended, last_down) > ?1",
                params![from, to],
                |row| row.get(0),
            )
            .map_err(sql)?;
        Ok(Day {
            date: date.to_owned(),
            probes: count(home.len()),
            away_probes: count(looks.len() - home.len()),
            gateway_p95_ms: percentile(home.iter().filter_map(|look| look.1)),
            internet_p95_ms: percentile(home.iter().filter_map(|look| look.2)),
            dns_p95_ms: percentile(home.iter().filter_map(|look| look.3)),
            unreachable: count(home.iter().filter(|look| !look.4).count()),
            outage_minutes: tenths(seconds(out_s) / MINUTE_S),
        })
    }
}

/// The 95th percentile by nearest rank, to a tenth; none of no times.
fn percentile(times: impl Iterator<Item = f64>) -> Option<f64> {
    let mut sorted: Vec<f64> = times.collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    let rank = (PERCENTILE * how_many(sorted.len())).ceil();
    let at = rank_index(rank).clamp(1, sorted.len()) - 1;
    Some(tenths(sorted[at]))
}

fn tenths(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a day's outage is far short of 2^52 seconds"
)]
fn seconds(value: i64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a day holds far fewer than 2^52 looks"
)]
fn how_many(len: usize) -> f64 {
    len as f64
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a rank is a small whole number, at least zero"
)]
fn rank_index(rank: f64) -> usize {
    rank as usize
}

#[cfg(test)]
#[path = "day_tests.rs"]
mod tests;
