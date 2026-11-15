//! Turns at sending a `there --changed` run to a builder: so many from
//! this machine at once, and fewer to any one builder, the rest waiting
//! here for one.
//!
//! On 14 November, from 20:00, sessions started runs faster than the lent
//! builders ran them: 24 at once at most, against 9 to 17 earlier in the
//! day. No more finished in the hour, about 52 either way, so each run
//! only took longer, a median of 1,016 s against 470 to 700, and timing
//! subtests broke under the load. Nothing waited for a lent builder, so
//! the queue read empty and the overload showed only as load.

use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::Duration;

/// How many runs go from this machine to builders at once.
pub const AT_ONCE: u32 = 8;

/// How many of them go to any one builder.
pub const EACH: u32 = 4;

/// A turn: had, held while its files are, or not had because `host` was
/// full and the run may go to another builder instead.
#[derive(Debug)]
pub enum Turn {
    Had(Vec<fs::File>),
    Full,
}

/// A turn at sending a run to `host`, its files under `turns`: one of
/// [`AT_ONCE`] for this machine where the run is `counted`, as one to a
/// lent builder is, then one of [`EACH`] for `host`. Each is waited for
/// unless the run `moves`, when either being full gives [`Turn::Full`]
/// at once, so it goes to the next builder, a warm VM builder among them,
/// which keeps its own four and isn't counted. The machine's turn is
/// taken first and the builder's only while it's held, so no two runs
/// each wait on the other's. A turn that can't be had at all, its folder
/// unwritable, is said, and the run goes on without one.
pub fn sending(turns: &Path, host: &str, moves: bool, counted: bool) -> Turn {
    taken(turns, host, moves, counted).unwrap_or_else(|error| {
        eprintln!("nix-offload: couldn't wait for a turn at sending this: {error}");
        Turn::Had(Vec::new())
    })
}

/// [`sending`]'s turn, or why it couldn't be had.
fn taken(turns: &Path, host: &str, moves: bool, counted: bool) -> io::Result<Turn> {
    fs::create_dir_all(turns)?;
    let mut held = Vec::new();
    if counted {
        held.push(match free(turns, "changed", AT_ONCE)? {
            Some(turn) => turn,
            None if moves => return Ok(Turn::Full),
            None => {
                eprintln!(
                    "nix-offload: {AT_ONCE} changed-checks runs already go out from this machine, \
                 so this waits its turn"
                );
                waited(turns, "changed", AT_ONCE)?
            }
        });
    }
    let name = format!("changed-{host}");
    held.push(match free(turns, &name, EACH)? {
        Some(turn) => turn,
        None if moves => return Ok(Turn::Full),
        None => {
            eprintln!(
                "nix-offload: {host} already runs {EACH} of this machine's changed-checks runs, \
                 so this waits its turn"
            );
            waited(turns, &name, EACH)?
        }
    });
    Ok(Turn::Had(held))
}

/// One of `count` turns named `name` once one is free, looking again
/// every half second, so whichever waiter looks first after a run ends
/// takes its turn.
fn waited(turns: &Path, name: &str, count: u32) -> io::Result<fs::File> {
    loop {
        if let Some(turn) = free(turns, name, count)? {
            return Ok(turn);
        }
        // A run holds its turn for minutes, so half a second's lag in
        // taking a freed one costs nothing and keeps the wait cheap.
        thread::sleep(Duration::from_millis(500));
    }
}

/// One of `count` turns named `name`, locked, if any is free: a turn is
/// a file's lock, so a run that dies gives its turn back with it.
fn free(turns: &Path, name: &str, count: u32) -> io::Result<Option<fs::File>> {
    for turn in 1..=count {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(turns.join(format!("{name}-{turn}")))?;
        match file.try_lock() {
            Ok(()) => return Ok(Some(file)),
            Err(fs::TryLockError::WouldBlock) => {}
            Err(fs::TryLockError::Error(error)) => return Err(error),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    /// Starts `runs` runs at once, each to the host `host_of` gives it,
    /// holding its turn a while, and gives the most that held one at once,
    /// overall and to the busiest host, and how many ran.
    fn started(runs: usize, host_of: fn(usize) -> String) -> (u32, u32, usize) {
        let turns = tempfile::tempdir().unwrap();
        let running = Arc::new(AtomicU32::new(0));
        let most = Arc::new(AtomicU32::new(0));
        let per_host = Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::<
            String,
            (u32, u32),
        >::new()));
        let ready = Arc::new(Barrier::new(runs));
        let handles: Vec<_> = (0..runs)
            .map(|run| {
                let turns = turns.path().to_owned();
                let (running, most, per_host, ready) = (
                    running.clone(),
                    most.clone(),
                    per_host.clone(),
                    ready.clone(),
                );
                thread::spawn(move || {
                    let host = host_of(run);
                    ready.wait();
                    let turn = sending(&turns, &host, false, true);
                    assert!(
                        matches!(turn, Turn::Had(_)),
                        "a run that waits had {turn:?}"
                    );
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    {
                        let mut hosts = per_host.lock().unwrap();
                        let (now, most) = hosts.entry(host.clone()).or_default();
                        *now += 1;
                        *most = (*most).max(*now);
                    }
                    // Long enough that every run started meanwhile finds
                    // the turns held, as a real run's minutes do.
                    thread::sleep(Duration::from_millis(300));
                    per_host.lock().unwrap().get_mut(&host).unwrap().0 -= 1;
                    running.fetch_sub(1, Ordering::SeqCst);
                    drop(turn);
                })
            })
            .collect();
        let ran = handles
            .into_iter()
            .flat_map(thread::JoinHandle::join)
            .count();
        let busiest = per_host
            .lock()
            .unwrap()
            .values()
            .map(|(_, most)| *most)
            .max()
            .unwrap_or(0);
        (most.load(Ordering::SeqCst), busiest, ran)
    }

    #[test]
    fn twelve_runs_at_once_send_eight_and_the_rest_wait_their_turn() {
        let (most, busiest, ran) = started(12, |run| format!("builder-{}", run % 3));
        assert_eq!(most, AT_ONCE, "runs sent at once");
        assert!(busiest <= EACH, "{busiest} went to one builder at once");
        assert_eq!(ran, 12, "every run had its turn in the end");
    }

    #[test]
    fn runs_to_one_builder_go_four_at_a_time() {
        let (most, busiest, ran) = started(6, |_| "builder".to_owned());
        assert_eq!(busiest, EACH, "runs sent to one builder at once");
        assert_eq!(most, EACH, "runs sent at once");
        assert_eq!(ran, 6, "every run had its turn in the end");
    }

    #[test]
    fn a_run_that_may_move_is_told_a_full_builder_is_full() {
        let turns = tempfile::tempdir().unwrap();
        let held: Vec<_> = (0..EACH)
            .map(|_| sending(turns.path(), "builder", false, true))
            .collect();
        assert!(
            matches!(sending(turns.path(), "builder", true, true), Turn::Full),
            "a fifth run to a builder running four wasn't told it's full"
        );
        assert!(
            matches!(sending(turns.path(), "other", true, true), Turn::Had(_)),
            "a run to another builder wasn't given its turn"
        );
        drop(held);
        assert!(
            given_back(|| sending(turns.path(), "builder", true, true)),
            "a builder's turns weren't given back"
        );
    }

    /// Whether `ask` has a turn within 5 s: a child another test forks
    /// holds copies of the turns' files, and their locks, until it execs,
    /// so one given back may stay taken for that instant.
    fn given_back(ask: impl Fn() -> Turn) -> bool {
        let until = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if matches!(ask(), Turn::Had(_)) {
                return true;
            }
            if std::time::Instant::now() > until {
                return false;
            }
            // Long enough for a forked child to reach its exec.
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn a_run_that_may_move_goes_on_while_the_machines_eight_are_out() {
        let turns = tempfile::tempdir().unwrap();
        let held: Vec<_> = (0..AT_ONCE)
            .map(|run| sending(turns.path(), &format!("builder-{}", run % 2), false, true))
            .collect();
        assert!(
            matches!(sending(turns.path(), "builder-2", true, true), Turn::Full),
            "a ninth run that may move wasn't told the machine's turns are out"
        );
        drop(held);
    }

    #[test]
    fn a_warm_vm_builder_takes_none_of_the_machines_eight() {
        let turns = tempfile::tempdir().unwrap();
        let held: Vec<_> = (0..AT_ONCE)
            .map(|run| sending(turns.path(), &format!("builder-{}", run % 2), false, true))
            .collect();
        assert!(
            matches!(sending(turns.path(), "vm", true, false), Turn::Had(_)),
            "a warm VM builder waited on the lent builders' turns"
        );
        drop(held);
    }
}
