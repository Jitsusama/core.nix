//! Whether a gate runs on this builder, and how a keep's VM tests' traces
//! give way to one. A gate holds a lock, shared, for as long as it runs;
//! a keep beside it traces its VM tests `AT_ONCE_BESIDE_A_GATE` at a time,
//! as memory allows, and begins none after `BESIDE_A_GATE` but the first,
//! so the next head's record isn't held back. On 13 November keeps that
//! traced none beside a gate recorded no VM test all evening, and gates
//! judged from records 50 to 160 files behind main and evaluated 400 to 650
//! checks.

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// How many VM tests a keep beside a gate traces at once, unless
/// `CHANGED_CHECKS_AT_ONCE_BESIDE_A_GATE` gives it. One at a time, keeps
/// beside gates on 14 November traced 6 to 25 of the 80 to 101 they had to
/// in their 5 minutes, and left 66 to 90 for a later keep, which a gate
/// nearly always shared too. On a lent builder busy with other work, 8
/// chunks of 40 checks took a median 144 s beside 1 at once, 135 s beside
/// 2 and 147 s beside 4, which finished 19 or 20 traces a round to 1's 8.
const AT_ONCE_BESIDE_A_GATE: usize = 4;

/// How many VM tests a keep beside a gate traces at once.
fn at_once_beside_a_gate() -> usize {
    std::env::var("CHANGED_CHECKS_AT_ONCE_BESIDE_A_GATE")
        .ok()
        .and_then(|traces| traces.parse().ok())
        .unwrap_or(AT_ONCE_BESIDE_A_GATE)
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
/// `at_once` at a time while one runs, as memory allows, and, past `bound`,
/// none begun while one runs but the first.
pub struct Paced {
    lock: PathBuf,
    said: AtomicBool,
    began: Instant,
    bound: Duration,
    at_once: usize,
    one_went: AtomicBool,
    memory_stalled: fn() -> bool,
}

impl Paced {
    pub fn beside_gates() -> Self {
        Self::beside_gates_at(
            lock(),
            at_once_beside_a_gate(),
            crate::pressure::memory_stalled,
        )
    }

    fn beside_gates_at(lock: PathBuf, at_once: usize, memory_stalled: fn() -> bool) -> Self {
        Self {
            lock,
            said: AtomicBool::new(false),
            began: Instant::now(),
            bound: beside_a_gate(),
            at_once,
            one_went: AtomicBool::new(false),
            memory_stalled,
        }
    }

    /// Whether another trace may begin beside the first and the `asked`
    /// that `Asked` counts under way.
    pub fn may_begin(&self, asked: usize) -> bool {
        if a_gate_runs(&self.lock) {
            if !self.said.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "changed-checks: a gate runs here, so this keep traces its VM tests alone \
                     {} at a time while it does",
                    self.at_once.max(1)
                );
            }
            if asked + 1 >= self.at_once {
                return false;
            }
        }
        !(self.memory_stalled)()
    }

    /// Whether to begin no more traces: the first always goes, so every
    /// keep beside a gate traces one.
    pub fn stops(&self) -> bool {
        self.one_went.swap(true, Ordering::Relaxed)
            && self.began.elapsed() >= self.bound
            && a_gate_runs(&self.lock)
    }
}

thread_local! {
    /// Whether this thread was let begin a trace it hasn't yet begun.
    static LET_BEGIN: Cell<bool> = const { Cell::new(false) };
}

/// The traces under way that were begun on asking, as every one but a
/// run's first is (`checks::each_at_once`), so a pace may count them. A yes
/// holds a place at once, so threads asking together can't all take the
/// same last one.
#[derive(Default)]
pub struct Asked {
    under_way: AtomicUsize,
    asking: Mutex<()>,
}

impl Asked {
    /// Whether this thread may begin another trace, as `may_begin` says
    /// given how many begun on asking are under way.
    pub fn ask(&self, may_begin: impl Fn(usize) -> bool) -> bool {
        let _alone = self.asking.lock().unwrap_or_else(PoisonError::into_inner);
        let yes = may_begin(self.under_way.load(Ordering::Relaxed));
        if yes && !LET_BEGIN.replace(true) {
            self.under_way.fetch_add(1, Ordering::Relaxed);
        }
        yes
    }

    /// Say this thread begins a trace, until what this gives is dropped.
    pub fn begun(&self) -> Begun<'_> {
        Begun {
            asked: self,
            on_asking: LET_BEGIN.replace(false),
        }
    }
}

/// A trace under way, counted while it was begun on asking.
pub struct Begun<'a> {
    asked: &'a Asked,
    on_asking: bool,
}

impl Drop for Begun<'_> {
    fn drop(&mut self) {
        if self.on_asking {
            self.asked.under_way.fetch_sub(1, Ordering::Relaxed);
        }
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
    fn beside_a_gate_a_keep_traces_four_at_once_and_never_five() {
        let scratch = Scratch::new("gating-four");
        let lock = scratch.join("changed-checks/gating.lock");
        let _gate = held(&lock).expect("the gate's lock");
        let paced = Paced::beside_gates_at(lock, 4, || false);
        let asked = Asked::default();
        let (under_way, most) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let items: Vec<usize> = (0..8).collect();
        crate::checks::each_at_once(
            &items,
            16,
            || asked.ask(|asked| paced.may_begin(asked)),
            |_| {
                let _begun = asked.begun();
                let now = under_way.fetch_add(1, Ordering::Relaxed) + 1;
                most.fetch_max(now, Ordering::Relaxed);
                // Each waits, for up to a second, until four are under way,
                // so the run can't end before the threads that asked begin.
                let began = Instant::now();
                while under_way.load(Ordering::Relaxed) < 4
                    && began.elapsed() < Duration::from_secs(1)
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                most.fetch_max(under_way.load(Ordering::Relaxed), Ordering::Relaxed);
                under_way.fetch_sub(1, Ordering::Relaxed);
                Ok::<_, ()>(())
            },
        )
        .expect("traced");
        assert_eq!(
            most.into_inner(),
            4,
            "beside a gate, the most traces under way at once wasn't 4"
        );
        assert!(
            !paced.may_begin(3),
            "beside a gate, a fifth trace was let begin beside four"
        );
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
