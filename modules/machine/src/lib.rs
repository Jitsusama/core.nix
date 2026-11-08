//! The machine daemon's own logic, apart from its socket and threads so
//! its tests reach it: what a client asks ([`interface`]), and what each
//! of the kernel's files says, made into records ([`processor`],
//! [`memory`], [`disk`], [`boot`], [`using`]). Every file is read through
//! a [`Root`], `/` in use and a fixture tree in the tests, so a record is
//! a function of the files and the read before.
//!
//! A record names every figure its source has, as the source counts it,
//! with its unit in its key (`_b` bytes, `_khz`, `_c` degrees Celsius,
//! `_ms`, `_s`, `_pct` a percentage, `_ratio` 0 to 1, `_per_s`), and none
//! it doesn't have: a figure the machine can't say is null, never a guess.
//! Words are the shell's.

pub mod boot;
pub mod disk;
pub mod interface;
pub mod memory;
pub mod pressure;
pub mod processor;
pub mod using;

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
    #[allow(clippy::cast_precision_loss)]
    Some((now - before) as f64 * 1000.0 / over as f64)
}

/// A figure rounded to a few places, so a record that hasn't moved says
/// so, and isn't told again for a change past what anyone reads.
pub fn rounded(value: f64, places: i32) -> Value {
    let scale = 10f64.powi(places);
    serde_json::Number::from_f64((value * scale).round() / scale).map_or(Value::Null, Value::Number)
}

/// Milliseconds since the epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

/// The CPU numbers a list such as `0-3,8` names.
pub fn cpu_list(text: &str) -> Vec<u32> {
    let mut listed = Vec::new();
    for part in text.trim().split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (first, last) = part.split_once('-').unwrap_or((part, part));
        if let (Ok(first), Ok(last)) = (first.parse::<u32>(), last.parse::<u32>()) {
            listed.extend(first..=last);
        }
    }
    listed
}
