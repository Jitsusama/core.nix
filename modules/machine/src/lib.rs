//! The machine daemon's own logic, apart from its socket and threads so
//! its tests reach it: what a client asks ([`interface`]), and what each
//! of the kernel's files says, made into records ([`processor`],
//! [`memory`], [`disk`], [`boot`], [`using`], [`tree`], [`heat`],
//! [`supply`], [`sleep`], [`firmware`], [`light`], and what the backlight
//! program steers the screen to, [`steered`], who has each camera open,
//! [`camera`], and the keyboards connected, caps lock and a board's layer,
//! [`keyboards`], what key-counts has tallied of each, [`counts`], and
//! what the desktop's watchers say of themselves, [`status`], each heard
//! as it's written through [`written`]), the network links and their
//! addresses, heard from the kernel's route socket ([`links`]), and the
//! user units the shell may
//! restart, [`units`], and the parts Heat's rows say, gathered from those
//! records and the processor's own telemetry ([`parts`], [`telemetry`]),
//! and Heat's, Power's and Machine's own rows made of them, ready for the
//! launcher ([`heat_rows`], [`power_rows`], [`machine_rows`],
//! [`processor_rows`], and what held the processor back and kept it from
//! resting, [`telemetry_rows`], placed and graphed through [`rows`] and
//! worded through [`words`]). Every file is read through
//! a [`Root`], `/` in use and a fixture tree in the tests, so a record is
//! a function of the files and the read before. What Firmware asks the
//! machine to do is done through [`acts`].
//!
//! A record names every figure its source has, as the source counts it,
//! with its unit in its key (`_b` bytes, `_khz`, `_c` degrees Celsius,
//! `_ms`, `_s`, `_pct` a percentage, `_ratio` 0 to 1, `_per_s`), and none
//! it doesn't have: a figure the machine can't say is null, never a guess.
//! Words are the shell's.

// A power supply's record names over forty figures in one `json!`, past
// the macro's default depth.
#![recursion_limit = "256"]

pub mod acts;
pub mod boot;
pub mod camera;
pub mod counts;
pub mod disk;
pub mod firmware;
pub mod firmware_rows;
pub mod heat;
pub mod heat_rows;
pub mod interface;
pub mod kept;
pub mod keyboards;
pub mod light;
pub mod links;
pub mod machine_rows;
pub mod memory;
pub mod parts;
pub mod power_rows;
pub mod pressure;
pub mod processor;
pub mod processor_rows;
pub mod rows;
pub mod sleep;
pub mod status;
pub mod steered;
pub mod supply;
pub mod telemetry;
pub mod telemetry_rows;
pub mod tree;
pub mod units;
pub mod using;
pub mod words;
pub mod written;

use std::path::{Path, PathBuf};

use serde_json::Value;

/// Where the kernel's files are read from.
#[derive(Clone, Debug)]
pub struct Root(PathBuf);

impl Root {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// A file under the root, by its path on the machine.
    pub fn path(&self, path: &str) -> PathBuf {
        self.0.join(path.trim_start_matches('/'))
    }

    /// What a file says, or none when it can't be read.
    pub fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.path(path)).ok()
    }

    /// What a file says, trimmed, or none when it's empty or can't be read.
    pub fn said(&self, path: &str) -> Option<String> {
        self.read(path)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    }

    /// The number a file says.
    pub fn number(&self, path: &str) -> Option<u64> {
        self.said(path)?.parse().ok()
    }

    /// The names in a folder, sorted, or none where there's no folder.
    pub fn names(&self, path: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.path(path))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// Where a link under the root points, as a path on the machine.
    pub fn resolved(&self, path: &str) -> Option<String> {
        let full = std::fs::canonicalize(self.path(path)).ok()?;
        let base = std::fs::canonicalize(&self.0).ok()?;
        let inside = full.strip_prefix(&base).ok()?;
        Some(Path::new("/").join(inside).to_string_lossy().into_owned())
    }
}

/// How a figure measured between two reads stands: since the machine
/// started, with no read before; since the last read, when that was
/// longer ago than [`STALE_MS`], as when the launcher opens again; or
/// now.
pub fn since(over_ms: Option<u64>) -> &'static str {
    match over_ms {
        None => "boot",
        Some(over) if over > STALE_MS => "asked",
        Some(_) => "now",
    }
}

/// Reads further apart than this aren't a rate of now.
pub const STALE_MS: u64 = 5000;

/// A rate a second of a counter between two reads `over_ms` apart, or
/// since boot with no read before, when `over_ms` is the time since boot;
/// none when a counter went backwards, as one reset does.
pub fn per_second(now: u64, before: Option<u64>, over_ms: Option<u64>) -> Option<f64> {
    let (before, over) = (before.unwrap_or(0), over_ms?);
    if over == 0 || now < before {
        return None;
    }
    Some(float(now - before) * 1000.0 / float(over))
}

/// A count as a float, for a share or a rate of it. Every count the
/// kernel keeps here, ticks, bytes, requests or milliseconds, stays far
/// under 2^52, below which a float holds each whole number exactly.
#[expect(
    clippy::cast_precision_loss,
    reason = "the kernel's counts stay under 2^52, where a float is exact"
)]
pub fn float(count: u64) -> f64 {
    count as f64
}

/// A figure rounded to a few places, so a record that hasn't moved says
/// so, and isn't told again for a change past what anyone reads.
pub fn rounded(value: f64, places: i32) -> Value {
    let scale = 10f64.powi(places);
    serde_json::Number::from_f64((value * scale).round() / scale).map_or(Value::Null, Value::Number)
}

/// Milliseconds on the clock that counts since boot, asleep included,
/// which reads are timed on: the wall clock steps when the time is set,
/// as it is just after boot or a wake, and a second's counts measured
/// over a stepped span would read several times too low while saying
/// "now" (Y2YBTS's review of #2276).
pub fn clock_ms() -> u64 {
    // SAFETY: clock_gettime writes only into the timespec it's given,
    // which is zeroed and ours.
    let mut said: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &raw mut said) } != 0 {
        return 0;
    }
    let seconds = u64::try_from(said.tv_sec).unwrap_or(0);
    let millis = u64::try_from(said.tv_nsec).unwrap_or(0) / 1_000_000;
    seconds.saturating_mul(1000).saturating_add(millis)
}

/// Milliseconds since the epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The CPU numbers a list such as `0-3,8` names.
pub fn cpu_list(text: &str) -> Vec<u32> {
    let mut listed = Vec::new();
    for part in text
        .trim()
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let (first, last) = part.split_once('-').unwrap_or((part, part));
        if let (Ok(first), Ok(last)) = (first.parse::<u32>(), last.parse::<u32>()) {
            listed.extend(first..=last);
        }
    }
    listed
}
