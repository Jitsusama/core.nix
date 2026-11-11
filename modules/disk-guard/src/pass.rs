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

    /// Whether a stop waits, which ends the pass before its next
    /// candidate. Once one waits the pacer stops resting, and on 10
    /// November a pass of thousands of cache files went on unpaced until
    /// systemd killed the guard 90 s after its stop, its desktop shell gone
    /// with it.
    fn stopping(&self) -> bool {
        crate::commands::stop_waiting()
    }
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
    /// What producers whose files said something new offer, as upkeep
    /// takes it, without the rest of upkeep's survey.
    Produce,
    /// Free space back to the target.
    Reach,
    /// What Joel asked cleared: everything given, as a critically full
    /// disk's windows take it, whatever free space says.
    Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    Reached,
    Exhausted,
    Stalled,
    /// A stop came, and what was left waits for the next start.
    Stopped,
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

/// The line a pass leaves in the guard's log once it has tried to take
/// something, said at `when`; a pass that only judged and kept says nothing.
pub fn told(report: &Report, seconds: f64, when: &str) -> Option<String> {
    let tried = report
        .actions
        .iter()
        .filter(|(_, outcome)| !matches!(outcome, Outcome::Kept(_)))
        .count();
    let took = report
        .actions
        .iter()
        .filter(|(_, outcome)| *outcome == Outcome::Reclaimed)
        .count();
    let band = while_band(report.band);
    (tried > 0).then(|| {
        format!(
            "disk-guard: at {when} a pass {band} took {took} of the {tried} it tried, \
             freeing {} in {seconds:.1} s",
            crate::units::shown(report.freed)
        )
    })
}

/// How a pass on a disk at least filling went wrong where nobody would
/// see it: what it tried all failed or was kept and it freed nothing, or
/// something it ran timed out. On 11 November a laptop's system guard
/// went 90 minutes with every delete of the store's garbage refused,
/// saying only that it had nothing left to clear.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Failing {
    /// What it tried and didn't keep on judging again.
    pub tried: usize,
    pub failed: usize,
    pub timed_out: bool,
    /// Why the last that failed did.
    pub last: String,
}

/// Whether `why`, an action's failure, says it ran out of time.
pub fn timed_out(why: &str) -> bool {
    why.contains(" ran past ")
}

/// How `report` went wrong, if it's a pass that ought to be told.
pub fn failing(report: &Report) -> Option<Failing> {
    let failures: Vec<&str> = report
        .actions
        .iter()
        .filter_map(|(_, outcome)| match outcome {
            Outcome::Failed(why) => Some(why.as_str()),
            _ => None,
        })
        .collect();
    let last = failures.last()?;
    let timed_out = failures.iter().any(|why| timed_out(why));
    let tried = report
        .actions
        .iter()
        .filter(|(_, outcome)| !matches!(outcome, Outcome::Kept(_)))
        .count();
    (report.band >= Band::Filling && (report.freed == 0 || timed_out)).then(|| Failing {
        tried,
        failed: failures.len(),
        timed_out,
        last: gist(last),
    })
}

/// The most a failure's reason says in a notice or a log line.
const GIST_CHARS: usize = 300;

/// A failure's reason in one line: a command's error stream can run to
/// pages of warnings and retries before the error that ended it, as Nix's
/// did asking each substituter four times, so the last line that says
/// `error:` is the one, else the first, cut to `GIST_CHARS`.
fn gist(why: &str) -> String {
    let lines = || why.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = lines()
        .rfind(|line| line.contains("error:"))
        .or_else(|| lines().next())
        .unwrap_or(why);
    match line.char_indices().nth(GIST_CHARS) {
        Some((end, _)) => format!("{}...", &line[..end]),
        None => line.to_owned(),
    }
}

/// The error line a failing pass leaves in the guard's log, said at
/// `when`.
pub fn failed_line(failing: &Failing, report: &Report, when: &str) -> String {
    let freed = if report.freed == 0 {
        "freed nothing".to_owned()
    } else {
        format!("freed only {}", crate::units::shown(report.freed))
    };
    let timing = if failing.timed_out {
        ", one timing out"
    } else {
        ""
    };
    format!(
        "disk-guard: error: at {when} a pass {} {freed}: {} of the {} it tried failed{timing}; \
         the last: {}",
        while_band(report.band),
        failing.failed,
        failing.tried,
        failing.last
    )
}

fn while_band(band: Band) -> &'static str {
    match band {
        Band::Healthy => "while healthy",
        Band::Filling => "while filling",
        Band::NearlyFull => "while nearly full",
        Band::CriticallyFull => "while critically full",
    }
}

/// Takes what the plan says until free space reaches the target, nothing
/// is left to take, or deletes stop freeing anything. Each batch is
/// settled and measured before the next is planned, so a writer filling
/// the disk as it goes is planned for, and what each kind frees is learned
/// from the measurement, never the sum of sizes. The pass plans by the
/// worst band it has seen, so a disk freed from nearly full to filling
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
            Goal::Upkeep | Goal::Produce | Goal::Clear => None,
            Goal::Reach if before.free >= thresholds.target => {
                report.end = End::Reached;
                return report;
            }
            Goal::Reach => Some(thresholds.target - before.free),
        };
        let windows = match goal {
            Goal::Upkeep | Goal::Produce => Band::Healthy,
            Goal::Clear => Band::CriticallyFull,
            Goal::Reach => report.band,
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
            if reclaimer.stopping() {
                report.end = End::Stopped;
                return report;
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
