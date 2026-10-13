//! Who writes how much: each process's `write_bytes`, sampled each step a
//! disk is below healthy, so the guard can name what's filling it and, at
//! critical with nothing left to take, freeze it.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// One process as a step saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub pid: u32,
    pub start: u64,
    pub parent: u32,
    pub uid: u32,
    pub name: String,
    pub written: u64,
    pub cgroup: Option<String>,
}

/// A process and what it wrote within a window.
#[derive(Clone, Debug, PartialEq)]
pub struct Writer {
    pub pid: u32,
    pub start: u64,
    pub name: String,
    pub written: u64,
    /// Bytes a second, over the window.
    pub rate: f64,
    pub cgroup: Option<String>,
}

/// The longest window anything asks about; older samples are dropped.
const KEPT: f64 = 600.0;

/// What each process has written, step by step, by pid and start, so a
/// pid reused by another process starts afresh.
#[derive(Debug, Default)]
pub struct Writers {
    seen: HashMap<(u32, u64), Track>,
}

#[derive(Debug)]
struct Track {
    name: String,
    cgroup: Option<String>,
    samples: VecDeque<(f64, u64)>,
}

impl Writers {
    /// Takes a step's samples at `at`; a process not among them is gone.
    pub fn observe(&mut self, samples: Vec<Sample>, at: f64) {
        let mut seen = HashMap::with_capacity(samples.len());
        for sample in samples {
            let key = (sample.pid, sample.start);
            let mut track = self.seen.remove(&key).unwrap_or(Track {
                name: sample.name,
                cgroup: sample.cgroup.clone(),
                samples: VecDeque::new(),
            });
            track.cgroup = sample.cgroup;
            track.samples.push_back((at, sample.written));
            while track.samples.len() > 2 && track.samples[1].0 <= at - KEPT {
                track.samples.pop_front();
            }
            seen.insert(key, track);
        }
        self.seen = seen;
    }

    /// Every process that wrote within the last `window` seconds, most
    /// first. A sample from before the window is its start where there
    /// is one, so what was written before it counts for nothing.
    pub fn recent(&self, window: f64, now: f64) -> Vec<Writer> {
        let since = now - window;
        let mut writers: Vec<Writer> = self
            .seen
            .iter()
            .filter_map(|(&(pid, start), track)| {
                let &(last_at, last) = track.samples.back()?;
                let base = track
                    .samples
                    .iter()
                    .rev()
                    .find(|(at, _)| *at <= since)
                    .or_else(|| track.samples.front())?;
                let written = last.saturating_sub(base.1);
                let seconds = last_at - base.0;
                (written > 0 && seconds > 0.0).then(|| Writer {
                    pid,
                    start,
                    name: track.name.clone(),
                    written,
                    rate: written as f64 / seconds,
                    cgroup: track.cgroup.clone(),
                })
            })
            .collect();
        writers.sort_by_key(|writer| std::cmp::Reverse(writer.written));
        writers
    }

    /// The one process that wrote more than half of everything written
    /// within the window, if one did.
    pub fn dominant(&self, window: f64, now: f64) -> Option<Writer> {
        let writers = self.recent(window, now);
        let total: u64 = writers.iter().map(|writer| writer.written).sum();
        writers
            .into_iter()
            .next()
            .filter(|top| top.written.saturating_mul(2) > total)
    }
}

/// Kernel threads write nothing of their own to freeze.
const PF_KTHREAD: u64 = 0x0020_0000;

/// Every process `proc` lists whose writes can be read, which for an
/// account is its own and for root every one.
pub fn sample(proc: &Path) -> Vec<Sample> {
    let Ok(entries) = fs::read_dir(proc) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let dir = entry.path();
            let stat = fs::read_to_string(dir.join("stat")).ok()?;
            let (open, close) = (stat.find('(')?, stat.rfind(')')?);
            let name = stat.get(open + 1..close)?.to_owned();
            // Fields from the state (the third) on; field n is at n - 3.
            let fields: Vec<&str> = stat.get(close + 1..)?.split_whitespace().collect();
            let flags: u64 = fields.get(6)?.parse().ok()?;
            if fields.first() == Some(&"Z") || flags & PF_KTHREAD != 0 {
                return None;
            }
            let io = fs::read_to_string(dir.join("io")).ok()?;
            let written = io
                .lines()
                .find_map(|line| line.strip_prefix("write_bytes:")?.trim().parse().ok())?;
            let cgroup = fs::read_to_string(dir.join("cgroup"))
                .ok()
                .and_then(|text| {
                    text.lines()
                        .find_map(|line| line.strip_prefix("0::").map(str::to_owned))
                });
            Some(Sample {
                pid,
                start: fields.get(19)?.parse().ok()?,
                parent: fields.get(1)?.parse().ok()?,
                uid: fs::metadata(&dir).ok()?.uid(),
                name,
                written,
                cgroup,
            })
        })
        .collect()
}

/// A process and every process descended from it.
pub fn descendants(samples: &[Sample], pid: u32) -> Vec<u32> {
    let mut found = vec![pid];
    let mut next = 0;
    while let Some(&parent) = found.get(next) {
        next += 1;
        for sample in samples {
            if sample.parent == parent && !found.contains(&sample.pid) {
                found.push(sample.pid);
            }
        }
    }
    found
}
