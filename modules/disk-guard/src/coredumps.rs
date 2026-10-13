//! Coredumps systemd-coredump keeps, which the system daemon ages out.

use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Dump {
    pub path: PathBuf,
    pub uid: u32,
    pub pid: u32,
    /// When it was taken, in wall seconds.
    pub at: f64,
    pub size: u64,
}

/// systemd-coredump's name for a finished dump:
/// `core.<process>.<uid>.<boot id>.<pid>.<microseconds>`, with its
/// compression's extension. The process name can hold dots, so the fields
/// are read from the right. A dump still being written has a hidden name,
/// which this never matches.
static NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^core\..+\.(?P<uid>\d+)\.[0-9a-f]{32}\.(?P<pid>\d+)\.(?P<usec>\d+)(\.(zst|xz|lz4))?$",
    )
    .expect("the coredump pattern compiles")
});

/// Every finished dump in `dir`, as a regular file, dated by the moment
/// its name records.
pub fn find(dir: &Path) -> Vec<Dump> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in listing.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(fields) = name.to_str().and_then(|name| NAME.captures(name)) else {
            continue;
        };
        let (Ok(uid), Ok(pid), Ok(usec)) = (
            fields["uid"].parse::<u32>(),
            fields["pid"].parse::<u32>(),
            fields["usec"].parse::<u64>(),
        ) else {
            continue;
        };
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        found.push(Dump {
            path: entry.path(),
            uid,
            pid,
            at: usec as f64 / 1e6,
            size: meta.len(),
        });
    }
    found
}

/// A dump as the planner takes it: one under a day old is kept for
/// critical, an older one goes at urgent.
pub fn candidate(dump: &Dump, awake: &Awake, now: f64) -> Candidate {
    Candidate {
        path: dump.path.clone(),
        class: Class::Coredump,
        idle: awake.since(dump.at.min(now), now).max(0.0),
        size: dump.size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
