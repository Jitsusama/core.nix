//! Carrying out a plan: reclaim a batch, let the frees settle, measure
//! them, and plan again.

use crate::band::{Band, Space, Thresholds};
use crate::plan::{Candidate, Class, plan};
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

/// How long a batch may go before its frees are settled and measured.
/// Settling asks the filesystem to commit, so it's done no more often.
const BATCH_SECONDS: f64 = 5.0;

/// A batch freeing under this share of its prediction made no progress,
/// and this many of them running stop the pass: something is holding the
/// extents, a snapshot, a reflink or a hard link, and deleting on won't help.
const STALL_SHARE: f64 = 0.1;
const STALL_BATCHES: usize = 3;

/// The filesystem a pass works on.
pub trait Disk {
    fn space(&mut self) -> io::Result<Space>;
    /// Waits until what was deleted shows as free.
    fn settle(&mut self) -> io::Result<()>;
    /// Monotonic seconds.
    fn now(&self) -> f64;
}

/// Reclaims one candidate, judging it once more first.
pub trait Reclaimer {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Reclaimed,
    /// Judged again and kept, with why.
    Kept(String),
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// A healthy disk's upkeep: everything due, whatever free space says.
    Upkeep,
    /// Free space back to the target.
    Reach,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    Reached,
    Exhausted,
    Stalled,
    Failed(String),
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Learned {
    ratios: BTreeMap<String, (f64, f64)>,
}

impl Learned {
    /// The weight a kind's past keeps against each new measurement, so its
    /// ratio follows what it frees now.
    const MEMORY: f64 = 0.8;

    /// The share of its logical size a kind has been measured to free. A
    /// kind never measured is predicted at its size.
    pub fn ratio(&self, class: Class) -> f64 {
        match self.ratios.get(class.kind()) {
            Some(&(freed, logical)) if logical > 0.0 => (freed / logical).clamp(0.05, 2.0),
            _ => 1.0,
        }
    }

    pub fn learn(&mut self, class: Class, logical: f64, freed: f64) {
        let entry = self
            .ratios
            .entry(class.kind().to_owned())
            .or_insert((0.0, 0.0));
        *entry = (
            entry.0 * Self::MEMORY + freed,
            entry.1 * Self::MEMORY + logical,
        );
    }
}

/// What a pass did.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub end: End,
    /// The worst band it saw, whose windows it planned by.
    pub band: Band,
    /// Free space gained, measured.
    pub freed: u64,
    pub actions: Vec<(PathBuf, Outcome)>,
}

/// Takes what the plan says until free space reaches the target, nothing
/// is left to take, or deletes stop freeing anything. Each batch is
/// settled and measured before the next is planned, so a writer filling
/// the disk as it goes is planned for, and what each kind frees is learned
/// from the measurement, never the sum of sizes. The pass plans by the
/// worst band it has seen, so a disk freed out of urgent into pressure
/// still reaches the target. A batch that stalls teaches nothing, since a
/// ratio learned from it would plan everything.
pub fn pass(
    disk: &mut impl Disk,
    reclaimer: &mut impl Reclaimer,
    candidates: &[Candidate],
    thresholds: &Thresholds,
    previous: Band,
    goal: Goal,
    learned: &mut Learned,
) -> Report {
    let mut report = Report {
        end: End::Exhausted,
        band: previous,
        freed: 0,
        actions: Vec::new(),
    };
    let mut taken = vec![false; candidates.len()];
    let (mut band, mut stalls) = (previous, 0);
    loop {
        let before = match disk.space() {
            Ok(space) => space,
            Err(error) => return failed(report, error),
        };
        band = thresholds.band(&before, band);
        report.band = report.band.max(band);
        let remaining = match goal {
            Goal::Upkeep => None,
            Goal::Reach if before.free >= thresholds.target => {
                report.end = End::Reached;
                return report;
            }
            Goal::Reach => Some(thresholds.target - before.free),
        };
        let windows = if goal == Goal::Upkeep {
            Band::Healthy
        } else {
            report.band
        };
        let left: Vec<usize> = (0..candidates.len()).filter(|&i| !taken[i]).collect();
        let offered: Vec<Candidate> = left.iter().map(|&i| candidates[i].clone()).collect();
        let steps = plan(&offered, windows, None, |class| learned.ratio(class));
        if steps.is_empty() {
            report.end = End::Exhausted;
            return report;
        }

        let started = disk.now();
        let (mut predicted, mut batch) = (0, Vec::new());
        for step in steps {
            let index = left[step.index];
            taken[index] = true;
            let candidate = &candidates[index];
            let outcome = reclaimer.reclaim(candidate);
            if outcome == Outcome::Reclaimed {
                predicted += step.predicted;
                batch.push(candidate);
            }
            report.actions.push((candidate.path.clone(), outcome));
            let enough = remaining.is_some_and(|remaining| predicted >= remaining);
            if enough || disk.now() - started >= BATCH_SECONDS {
                break;
            }
        }
        if batch.is_empty() {
            continue;
        }
        if let Err(error) = disk.settle() {
            return failed(report, error);
        }
        let after = match disk.space() {
            Ok(space) => space,
            Err(error) => return failed(report, error),
        };
        let freed = after.free.saturating_sub(before.free);
        report.freed += freed;
        if (freed as f64) < predicted as f64 * STALL_SHARE {
            stalls += 1;
            if stalls >= STALL_BATCHES {
                report.end = End::Stalled;
                return report;
            }
            continue;
        }
        stalls = 0;
        // Each kind is credited its share of the batch's logical bytes.
        let logical: u64 = batch.iter().map(|candidate| candidate.size).sum();
        for candidate in batch {
            let share = candidate.size as f64 / logical.max(1) as f64;
            learned.learn(candidate.class, candidate.size as f64, freed as f64 * share);
        }
    }
}

fn failed(mut report: Report, error: io::Error) -> Report {
    report.end = End::Failed(error.to_string());
    report
}
