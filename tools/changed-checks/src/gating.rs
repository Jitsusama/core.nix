//! Whether a gate runs on this builder, and how a keep's VM tests' traces
//! give way to one. A gate holds a lock, shared, for as long as it runs;
//! a keep beside it traces its VM tests one at a time, as memory allows,
//! and begins none after `BESIDE_A_GATE` but the first, so the next head's
//! record isn't held back. On 13 November keeps that traced none beside a
//! gate recorded no VM test all evening, and gates judged from records 50
//! to 160 files behind main and evaluated 400 to 650 checks.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a keep beside a gate goes on beginning VM tests' traces, one
/// at a time, unless `CHANGED_CHECKS_BESIDE_A_GATE` gives it in seconds.
const BESIDE_A_GATE: Duration = Duration::from_mins(5);

/// How long a keep beside a gate goes on beginning VM tests' traces.
fn beside_a_gate() -> Duration {
    std::env::var("CHANGED_CHECKS_BESIDE_A_GATE")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(BESIDE_A_GATE, Duration::from_secs)
}

/// Where a gate holds a lock, shared, for as long as it runs.
pub fn lock() -> PathBuf {
    crate::cache().join("changed-checks/gating.lock")
}

/// Say that a gate runs here until what this gives is dropped; nothing
/// when the lock can't be had, and the gate runs regardless.
pub fn held(lock: &Path) -> Option<fs::File> {
    fs::create_dir_all(lock.parent()?).ok()?;
    let held = fs::File::create(lock).ok()?;
    held.lock_shared().ok()?;
    Some(held)
}

/// Whether a gate runs here, as its lock says: a keep tries it whole and
/// gives it back at once, so a gate starting then waits only that long.
/// A lock tried whole holds against every other open of it, this
/// process's too, so a keep's threads ask one at a time: asking together,
/// each took another's try for a gate and the keep stopped. The try is let
/// go of before the file closes: a program another thread starts holds a
/// copy of the file until it runs, and the lock with it, which closing
/// leaves held and letting go frees.
fn a_gate_runs(lock: &Path) -> bool {
    static ASKING: Mutex<()> = Mutex::new(());
    let _alone = ASKING.lock().unwrap_or_else(PoisonError::into_inner);
    let Ok(tried) = fs::File::create(lock) else {
        return false;
    };
    match tried.try_lock() {
        Ok(()) => {
            let _ = tried.unlock();
            false
        }
        Err(fs::TryLockError::WouldBlock) => true,
        Err(fs::TryLockError::Error(_)) => false,
    }
}

/// How a keep paces its VM tests' traces beside the gates on its builder:
/// one at a time while one runs, as memory allows, and, past `bound`, none
/// begun while one runs but the first.
pub struct Paced {
    lock: PathBuf,
    said: AtomicBool,
    began: Instant,
    bound: Duration,
    one_went: AtomicBool,
}

impl Paced {
    pub fn beside_gates() -> Self {
        Self {
            lock: lock(),
            said: AtomicBool::new(false),
            began: Instant::now(),
            bound: beside_a_gate(),
            one_went: AtomicBool::new(false),
        }
    }

    /// Whether another trace may begin beside those under way.
    pub fn may_begin(&self) -> bool {
        if a_gate_runs(&self.lock) {
            if !self.said.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "changed-checks: a gate runs here, so this keep traces its VM tests alone \
                     one at a time while it does"
                );
            }
            return false;
        }
        !crate::pressure::memory_stalled()
    }

    /// Whether to begin no more traces: the first always goes, so every
    /// keep beside a gate traces one.
    pub fn stops(&self) -> bool {
        self.one_went.swap(true, Ordering::Relaxed)
            && self.began.elapsed() >= self.bound
            && a_gate_runs(&self.lock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn a_gate_holding_its_lock_is_seen_running_until_every_gate_lets_go() {
        let scratch = Scratch::new("gating");
        let lock = scratch.join("changed-checks/gating.lock");
        assert!(!a_gate_runs(&lock), "no gate ran, yet one was seen");
        let gate = held(&lock).expect("the gate's lock");
        assert!(a_gate_runs(&lock), "a gate ran unseen");
        let other = held(&lock).expect("a second gate shares it");
        drop(gate);
        assert!(a_gate_runs(&lock), "the second gate ran unseen");
        drop(other);
        // A program another test starts meanwhile holds a copy of the lock's
        // file until it runs, and with it the lock, so it's let go once that
        // copy is: seen held just after both let go here (13 November).
        let let_go = (0..200).any(|_| {
            let runs = a_gate_runs(&lock);
            if runs {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            !runs
        });
        assert!(let_go, "a gate was seen a second after both let go");
    }

    #[test]
    fn a_keeps_threads_asking_together_never_take_one_another_for_a_gate() {
        // Each of a keep's 16 tracing threads asks before it begins a VM
        // test, and a lock tried whole is held, for that instant, against
        // every other open of it, this process's too. Meanwhile the others
        // start nix, and a program started holds a copy of each file open
        // until it runs, with any lock on it.
        let scratch = Scratch::new("gating-together");
        let lock = scratch.join("changed-checks/gating.lock");
        fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
        let asked = AtomicBool::new(false);
        let seen = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !asked.load(Ordering::Relaxed) {
                    let _ = std::process::Command::new("changed-checks-starts-nothing").status();
                }
            });
            let asking: Vec<_> = (0..crate::alone::AT_ONCE)
                .map(|_| scope.spawn(|| (0..200).filter(|_| a_gate_runs(&lock)).count()))
                .collect();
            let seen = asking
                .into_iter()
                .map(|thread| thread.join().expect("asked"))
                .sum::<usize>();
            asked.store(true, Ordering::Relaxed);
            seen
        });
        assert_eq!(
            seen, 0,
            "with no gate, a keep's threads saw one {seen} times"
        );
    }
}
