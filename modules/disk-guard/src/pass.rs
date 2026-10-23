//! Carrying out a plan: reclaim a batch, let the frees settle, measure
//! them, and plan again.

use crate::band::{Band, Space, Thresholds};
use crate::plan::{Candidate, Class, Group, plan};
use crate::units::float;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;

/// How long a batch may go before its frees are settled and measured.
/// Settling asks the filesystem to commit, so it's done no more often.
const BATCH_SECONDS: f64 = 5.0;

/// A batch freeing under this share of its prediction made no progress,
/// and this many of them running in a group stop that group: something is
/// holding its extents, a snapshot, a reflink or a hard link, and deleting
/// more of it won't help. The pass goes on to the next group, which may
/// hold what will.
const STALL_SHARE: f64 = 0.1;
const STALL_BATCHES: usize = 3;

/// The least a batch must predict for its measurement to mean anything. A
/// build writes a gigabyte in seconds, so a batch of temp files predicting
/// a few megabytes measures the writer, not itself: it neither stalls nor
/// teaches.
const STALL_FLOOR: u64 = 1 << 30;

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
/// ratio learned from it would plan everything, and a group that keeps
/// stalling is left for the next: a pass ends stalled only when it then
/// runs out of anything else to take.
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
    let mut band = previous;
    // Stalled batches running, by group, and the groups given up on.
    let mut stalls: BTreeMap<Group, usize> = BTreeMap::new();
    let mut given_up: BTreeSet<Group> = BTreeSet::new();
    loop {
        let before = match disk.space() {
            Ok(space) => space,
            Err(error) => return failed(report, &error),
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
        let mut steps = plan(&offered, windows, None, |class| learned.ratio(class));
        steps.retain(|step| !given_up.contains(&step.group));
        if steps.is_empty() {
            report.end = if given_up.is_empty() {
                End::Exhausted
            } else {
                End::Stalled
            };
            return report;
        }

        let started = disk.now();
        let (mut predicted, mut batch, mut groups) = (0, Vec::new(), Vec::new());
        for step in &steps {
            let index = left[step.index];
            let candidate = &candidates[index];
            // A command is measured alone, so what it frees is credited to
            // no one else, and is measured before anything more goes, since
            // it may have been enough.
            if candidate.class.command() && !batch.is_empty() {
                break;
            }
            taken[index] = true;
            let outcome = reclaimer.reclaim(candidate);
            let freeing = outcome == Outcome::Reclaimed;
            if freeing {
                predicted += step.predicted;
                batch.push(candidate);
                if !groups.contains(&step.group) {
                    groups.push(step.group);
                }
            }
            report.actions.push((candidate.path.clone(), outcome));
            let enough = remaining.is_some_and(|remaining| predicted >= remaining);
            let measure = freeing && candidate.class.command();
            if enough || measure || disk.now() - started >= BATCH_SECONDS {
                break;
            }
        }
        if batch.is_empty() {
            continue;
        }
        if let Err(error) = disk.settle() {
            return failed(report, &error);
        }
        let after = match disk.space() {
            Ok(space) => space,
            Err(error) => return failed(report, &error),
        };
        let freed = after.free.saturating_sub(before.free);
        report.freed += freed;
        if float(freed) < float(predicted) * STALL_SHARE {
            if predicted >= STALL_FLOOR {
                stalled(groups, &mut stalls, &mut given_up);
            }
            continue;
        }
        for group in groups {
            stalls.remove(&group);
        }
        credit(learned, &batch, freed);
    }
}

/// Counts a stalled batch against each of its groups, giving up on a group
/// once it has stalled `STALL_BATCHES` times running.
fn stalled(
    groups: Vec<Group>,
    stalls: &mut BTreeMap<Group, usize>,
    given_up: &mut BTreeSet<Group>,
) {
    for group in groups {
        let count = stalls.entry(group).or_default();
        *count += 1;
        if *count >= STALL_BATCHES {
            given_up.insert(group);
        }
    }
}

/// Teaches what a measured batch freed, each kind credited its share of the
/// batch's logical bytes.
fn credit(learned: &mut Learned, batch: &[&Candidate], freed: u64) {
    let logical: u64 = batch.iter().map(|candidate| candidate.size).sum();
    for candidate in batch {
        let share = float(candidate.size) / float(logical.max(1));
        learned.learn(candidate.class, float(candidate.size), float(freed) * share);
    }
}

fn failed(mut report: Report, error: &io::Error) -> Report {
    report.end = End::Failed(error.to_string());
    report
}
