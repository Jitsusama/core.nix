//! How long a key typed into pi waited, from pi-keys' samples
//! (modules/pi-keys): a few times a day it reads the threads a key passes
//! while Joel is at a pi window, and models what a key would have waited.
//! The keys are modelled, not his own, so these figures are a stand-in:
//! our side's, never a spec's.

use std::fs;
use std::path::Path;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use crate::follow::date_of;

/// One sample's line, as much of it as the day keeps.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sample {
    /// When it ended, in seconds since the epoch.
    pub ended: i64,
    /// How many keys it modelled.
    pub keys: u64,
    /// Their median wait, in milliseconds.
    pub p50_ms: f64,
    /// The wait at most one key in a hundred passed.
    pub p99_ms: f64,
    /// The longest.
    pub max_ms: f64,
    /// A frame of the screen it was read at.
    pub frame_ms: f64,
}

/// The samples that ended on `date` in `zone`, from pi-keys' files in
/// `dir`, one a UTC day; a day in a zone spans two of them.
#[must_use]
pub fn read(dir: &Path, date: &str, zone: &TimeZone) -> Vec<Sample> {
    let Ok(day) = date.parse::<Date>() else {
        return Vec::new();
    };
    [day.yesterday(), Ok(day), day.tomorrow()]
        .into_iter()
        .flatten()
        .filter_map(|utc| fs::read_to_string(dir.join(format!("{utc}.jsonl"))).ok())
        .flat_map(|file| {
            file.lines()
                .filter_map(|line| serde_json::from_str::<Sample>(line).ok())
                .collect::<Vec<_>>()
        })
        .filter(|sample| sample.keys > 0 && date_of(sample.ended * 1_000_000, zone) == date)
        .collect()
}

/// The sample whose p99 was worst.
#[must_use]
pub fn worst(samples: &[Sample]) -> Option<&Sample> {
    samples.iter().max_by(|a, b| a.p99_ms.total_cmp(&b.p99_ms))
}

/// The day's typing in pi in a line, as `health` shows it.
#[must_use]
pub fn said(samples: &[Sample]) -> Option<String> {
    let worst = worst(samples)?;
    let keys: u64 = samples.iter().map(|sample| sample.keys).sum();
    let frames = if worst.frame_ms > 0.0 {
        format!(", {:.1} frames", worst.p99_ms / worst.frame_ms)
    } else {
        String::new()
    };
    let taken = match samples.len() {
        1 => "1 sample".to_owned(),
        many => format!("{many} samples"),
    };
    Some(format!(
        "typing in pi, modelled: {keys} keys in {taken}, the worst p99 {:.0} ms{frames}",
        worst.p99_ms
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_days_samples_are_read_across_the_utc_days_it_spans() {
        let dir = tempfile::tempdir().expect("no folder for the samples");
        // 2026-11-12 in UTC-5 runs from 05:00 UTC that day to 05:00 the next.
        let line = |ended: i64, p99: f64| {
            format!(
                "{{\"ended\":{ended},\"keys\":400,\"modelled\":true,\"p50_ms\":4.1,\
                 \"p99_ms\":{p99},\"max_ms\":40.0,\"frame_ms\":16.67,\"call\":false}}"
            )
        };
        fs::write(
            dir.path().join("2026-11-12.jsonl"),
            format!(
                "{}\n{}\n",
                line(1_794_459_600 - 60, 9.0),
                line(1_794_470_400, 21.0)
            ),
        )
        .expect("the first day wasn't written");
        fs::write(
            dir.path().join("2026-11-13.jsonl"),
            format!("{}\nnot json\n", line(1_794_544_000, 12.0)),
        )
        .expect("the second day wasn't written");
        let zone = TimeZone::fixed(jiff::tz::offset(-5));
        let samples = read(dir.path(), "2026-11-12", &zone);
        assert_eq!(
            samples
                .iter()
                .map(|sample| sample.p99_ms)
                .collect::<Vec<_>>(),
            [21.0, 12.0],
            "a sample of another day was read, or one of this day missed"
        );
        assert_eq!(
            said(&samples).as_deref(),
            Some("typing in pi, modelled: 800 keys in 2 samples, the worst p99 21 ms, 1.3 frames"),
            "the day's typing was said wrongly"
        );
    }
}
