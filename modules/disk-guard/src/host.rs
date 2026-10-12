//! The guard's hold on this machine: what its processes are doing, read
//! from `/proc`, its filesystems, and reclaiming what a survey offered.

use crate::activity::{self, Process, Use};
use crate::anchor::Dir;
use crate::band::Space;
use crate::pass::{Disk, Outcome, Reclaimer};
use crate::plan::Candidate;
use crate::reclaim::{self, Looker};
use crate::survey::{Action, Offer};
use crate::workspaces;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How long a scan of every process is trusted for. One takes about
/// 117 ms on a desktop of 285 processes, so judging thousands of entries
/// one scan each would take minutes.
pub const FRESH: Duration = Duration::from_secs(1);

/// Every process as last scanned, read again once something has been
/// moved aside or the scan is older than its freshness: the judgement
/// after a move has to see a process that arrived just before it.
pub struct Scan {
    proc: PathBuf,
    locks: PathBuf,
    me: u32,
    fresh: Duration,
    seen: Option<(Instant, Vec<Process>)>,
}

impl Scan {
    pub fn new(proc: &Path, locks: &Path, me: u32, fresh: Duration) -> Self {
        Scan {
            proc: proc.to_owned(),
            locks: locks.to_owned(),
            me,
            fresh,
            seen: None,
        }
    }

    pub fn processes(&mut self) -> io::Result<&[Process]> {
        let stale = match &self.seen {
            Some((taken, _)) => taken.elapsed() >= self.fresh,
            None => true,
        };
        if stale {
            let taken = Instant::now();
            self.seen = Some((taken, crate::linux::processes(&self.proc)?));
        }
        Ok(self.seen.as_ref().map_or(&[], |(_, processes)| processes))
    }
}

impl Looker for Scan {
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> io::Result<Use> {
        let me = self.me;
        Ok(activity::judge(self.processes()?, me, target, checkout))
    }

    fn moved(&mut self) {
        self.seen = None;
    }

    fn waiters(&mut self, inodes: &[u64]) -> io::Result<Vec<i32>> {
        crate::linux::lock_waiters(&self.locks, self.me, inodes)
    }
}

/// One filesystem, measured and settled through a path on it.
pub struct Filesystem {
    path: PathBuf,
    started: Instant,
}

impl Filesystem {
    pub fn new(path: &Path) -> Self {
        Filesystem {
            path: path.to_owned(),
            started: Instant::now(),
        }
    }
}

impl Disk for Filesystem {
    fn space(&mut self) -> io::Result<Space> {
        crate::linux::space(&self.path)
    }

    fn settle(&mut self) -> io::Result<()> {
        crate::linux::settle(&self.path)
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

/// Reclaims what a survey offered, each as its action says, anchored to
/// the root it was found beneath. Anything no offer names is refused.
pub struct Reclaim<'a> {
    actions: HashMap<PathBuf, Action>,
    scan: &'a mut Scan,
    notes: Vec<String>,
}

impl<'a> Reclaim<'a> {
    pub fn new(offers: &[Offer], scan: &'a mut Scan) -> Self {
        let actions = offers
            .iter()
            .map(|offer| (offer.candidate.path.clone(), offer.action.clone()))
            .collect();
        Reclaim {
            actions,
            scan,
            notes: Vec::new(),
        }
    }

    /// What went wrong after a reclaim that freed its space anyway.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

impl Reclaimer for Reclaim<'_> {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let Some(action) = self.actions.get(&candidate.path) else {
            return Outcome::Failed("no source offered it".to_owned());
        };
        let root_path = match action {
            Action::Target { root }
            | Action::Variant { root }
            | Action::Entry { root }
            | Action::Workspace { root } => root,
            Action::Trash { trash } => trash,
        };
        let root = match Dir::open(root_path) {
            Ok(root) => root,
            Err(error) => {
                return Outcome::Failed(format!("opening {}: {error}", root_path.display()));
            }
        };
        match action {
            Action::Target { .. } => {
                let checkout = candidate.checkout.clone();
                reclaim::target(&root, root_path, candidate, checkout.as_deref(), self.scan)
            }
            Action::Variant { .. } => {
                reclaim::variant(&root, root_path, candidate, SystemTime::now())
            }
            Action::Entry { .. } => reclaim::entry(&root, root_path, candidate, self.scan),
            Action::Trash { .. } => reclaim::trashed(&root, root_path, candidate),
            Action::Workspace { .. } => {
                // Read before it goes: its .git files name the repositories.
                let linked = workspaces::linked(&candidate.path);
                let outcome = reclaim::entry(&root, root_path, candidate, self.scan);
                if outcome == Outcome::Reclaimed {
                    for common in linked {
                        if let Err(error) = workspaces::prune(&common) {
                            self.notes
                                .push(format!("pruning {}: {error}", common.display()));
                        }
                    }
                }
                outcome
            }
        }
    }
}
