//! Snapshots a running program keeps, named for its pid, which it leaves
//! behind when it dies.

use crate::activity::{Process, runs};
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use crate::units::{float, wall};
use regex_lite::Regex;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// When this boot began, in wall seconds, and how many clock ticks a
/// process's start time counts per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boot {
    pub wall: f64,
    pub ticks: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub path: PathBuf,
    pub pid: u32,
    /// When it appeared, in wall seconds.
    pub born: f64,
}

/// A process's start can't be read finer than a tick, and the boot's wall
/// time drifts by however long reading it took, so a process starting up
/// to this long after a snapshot appeared may still be the one that made
/// it, and keeps it.
const SLACK: f64 = 1.0;

/// Every entry of `dir` whose name `pattern` matches with a pid, dated by
/// when it appeared: its birth time, or its change time where the
/// filesystem keeps no birth time.
pub fn entries(dir: &Path, pattern: &Regex) -> Vec<Snapshot> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in listing.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| pattern.captures(name))
            .and_then(|found| found.name("pid")?.as_str().parse().ok())
        else {
            continue;
        };
        let Ok(meta) = entry.metadata() else { continue };
        let born = meta
            .created()
            .ok()
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok());
        let born = born.map_or(wall(meta.ctime(), 0), |at| at.as_secs_f64());
        found.push(Snapshot {
            path: entry.path(),
            pid,
            born,
        });
    }
    found
}

/// Whether a snapshot's maker is gone: no process has its pid, runs one of
/// `commands` and started before the snapshot appeared, so a pid the
/// kernel gave to someone since keeps nothing alive. A process with its pid
/// that can't be read keeps it, since it can't be ruled out.
pub fn orphaned(snapshot: &Snapshot, processes: &[Process], commands: &[&str], boot: Boot) -> bool {
    let maker = |process: &&Process| {
        let started = boot.wall + float(process.start_ticks) / boot.ticks;
        let runs_one = commands.iter().any(|command| runs(process, command));
        !process.readable || (runs_one && started <= snapshot.born + SLACK)
    };
    !processes
        .iter()
        .filter(|process| process.pid == snapshot.pid)
        .any(|process| maker(&process))
}

pub fn candidate(snapshot: &Snapshot, size: u64, awake: &Awake, now: f64) -> Candidate {
    Candidate {
        path: snapshot.path.clone(),
        class: Class::Snapshot,
        idle: awake.since(snapshot.born.min(now), now).max(0.0),
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
