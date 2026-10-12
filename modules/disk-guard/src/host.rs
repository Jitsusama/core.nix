//! The guard's hold on this machine: what its processes are doing, read
//! from `/proc`.

use crate::activity::{self, Process, Use};
use crate::reclaim::Looker;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
