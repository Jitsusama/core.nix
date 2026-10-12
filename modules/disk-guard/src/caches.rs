//! Caches that refill on demand, taken from urgent on, cheapest to refill
//! first.
//!
//! A cache without a protocol goes whole, as one candidate: its top-level
//! entries can't be told apart safely, since a database left beside the
//! write-ahead log of the one deleted can be corrupted when it next opens.
//! Go's build cache is read entry by entry, as Go's own trim reads it.

use crate::band::Band;
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub last_used: f64,
    pub size: u64,
}

/// Go's names: 256 directories made when the cache opens, which stay
/// since a running go writes into them, each holding an action's entry
/// (`-a`) and its output (`-d`).
static GO_DIR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9a-f]{2}$").expect("the directory pattern compiles"));
static GO_ENTRY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9a-f]{64}-[ad]$").expect("the entry pattern compiles"));

/// Every entry of Go's build cache, dated by its modification time, which
/// Go moves forward when it uses one (at most hourly).
pub fn go_build(cache: &Path) -> Vec<Entry> {
    let names = |dir: &Path| {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
            .collect::<Vec<_>>()
    };
    let mut found = Vec::new();
    for (name, dir) in names(cache) {
        if !GO_DIR.is_match(&name) || !fs::symlink_metadata(&dir).is_ok_and(|meta| meta.is_dir()) {
            continue;
        }
        for (name, path) in names(&dir) {
            if !GO_ENTRY.is_match(&name) {
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let last_used = meta
                .modified()
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .map_or(0.0, |at| at.as_secs_f64());
            found.push(Entry {
                path,
                last_used,
                size: meta.len(),
            });
        }
    }
    found
}

/// One of Go's entries, longest unused first within its rank.
pub fn go_entry(entry: &Entry, rank: u8, from: Band, awake: &Awake, now: f64) -> Candidate {
    Candidate {
        path: entry.path.clone(),
        class: Class::Cache { rank, from },
        idle: awake.since(entry.last_used.min(now), now).max(0.0),
        size: entry.size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

/// A whole cache, which its band takes at once.
pub fn whole(path: &Path, rank: u8, from: Band, size: u64) -> Candidate {
    Candidate {
        path: path.to_owned(),
        class: Class::Cache { rank, from },
        idle: 0.0,
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
