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
/// each wait on the other's. A turn whose file can't be used is said and
/// left out, as the uplink's are, so the run waits on the rest; one that
/// can't be had at all, its folder unwritable or none of its files usable,
/// is said, and the run goes on without it, keeping whichever of the two
/// can be had, so neither limit lifts the other.
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
    let mut machines = None;
    if counted {
        let full = format!("{AT_ONCE} changed-checks runs already go out from this machine");
        match turn_of(turns, ("changed", AT_ONCE), moves, &full) {
            Ok(Some(turn)) => held.push(turn),
            Ok(None) => return Ok(Turn::Full),
            // The builder's turn is still taken, so its four still hold
            // where only the machine's eight can't.
            Err(error) => machines = Some(error),
        }
    }
    let name = format!("changed-{host}");
    let full = format!("{host} already runs {EACH} of this machine's changed-checks runs");
    let builders = match turn_of(turns, (&name, EACH), moves, &full) {
        Ok(Some(turn)) => {
            held.push(turn);
            None
        }
        Ok(None) => {
            if let Some(line) = unhad(host, counted, machines.as_ref(), None) {
                eprintln!("nix-offload: {line}");
            }
            return Ok(Turn::Full);
        }
        Err(error) => Some(error),
    };
    if let Some(line) = unhad(host, counted, machines.as_ref(), builders.as_ref()) {
        eprintln!("nix-offload: {line}");
    }
    Ok(Turn::Had(held))
}

/// What a run to `host`, `counted` among the machine's or not, says when
/// the machine's turns or the builder's can't be had, why each couldn't
/// given; said once, after both are known, so it never says one counts a
/// run that the next line says nothing counts. Nothing when both could.
fn unhad(
    host: &str,
    counted: bool,
    machines: Option<&io::Error>,
    builders: Option<&io::Error>,
) -> Option<String> {
    Some(match (machines, builders) {
        (None, None) => return None,
        (Some(machines), None) => format!(
            "this machine's turns can't be had ({machines}), so this run is counted only by \
             its builder's"
        ),
        (None, Some(builders)) if counted => format!(
            "{host}'s turns can't be had ({builders}), so this run holds only the machine's"
        ),
        (None, Some(builders)) => {
            format!("{host}'s turns can't be had ({builders}), so this run goes on without one")
        }
        (Some(machines), Some(builders)) => format!(
            "neither this machine's turns ({machines}) nor {host}'s ({builders}) can be had, \
             so this run goes on without one"
        ),
    })
}

/// One of the `count` turns named `name`, waited for while all are held,
/// saying so as `full` says it, unless the run `moves`, when none is
/// given; turns whose files can't be used are said once and left out.
fn turn_of(
    turns: &Path,
    (name, count): (&str, u32),
    moves: bool,
    full: &str,
) -> io::Result<Option<fs::File>> {
    let (turn, unusable) = free(turns, name, count)?;
    match turn {
        Some(turn) => Ok(Some(turn)),
        None if moves => Ok(None),
        None => {
            // Said only as it waits, as the uplink's are, so a run that
            // takes a free turn or moves on says nothing of a wait.
            if !unusable.is_empty() {
                eprintln!(
                    "nix-offload: not every turn named {name} can be used ({}), so this waits \
                     on the rest",
                    unusable.join("; ")
                );
            }
            eprintln!("nix-offload: {full}, so this waits its turn");
            waited(turns, name, count).map(Some)
        }
    }
}

/// One of `count` turns named `name` once one is free, looking again
/// every half second, so whichever waiter looks first after a run ends
/// takes its turn.
fn waited(turns: &Path, name: &str, count: u32) -> io::Result<fs::File> {
    loop {
        if let (Some(turn), _) = free(turns, name, count)? {
            return Ok(turn);
        }
        // A run holds its turn for minutes, so half a second's lag in
        // taking a freed one costs nothing and keeps the wait cheap.
        thread::sleep(Duration::from_millis(500));
    }
}

/// One of `count` turns named `name`, locked, if any is free, and why each
/// turn whose file couldn't be used couldn't: a turn is a file's lock, so
/// a run that dies gives its turn back with it. A turn's file is opened as
/// the uplink's are (`there::open_turn`), so none that's a link, a FIFO or
/// another file's second name is used. One bad file is left out rather
/// than ending the look, so the turns after it are still tried; only when
/// none can be used is it an error: ended at the first, one link among
/// them lifted the whole limit for every run while it stayed.
fn free(turns: &Path, name: &str, count: u32) -> io::Result<(Option<fs::File>, Vec<String>)> {
    let mut unusable = Vec::new();
    for turn in 1..=count {
        let at = turns.join(format!("{name}-{turn}"));
        let file = match crate::there::open_turn(&at, true) {
            Ok(file) => file,
            Err(why) => {
                unusable.push(why);
                continue;
            }
        };
        match file.try_lock() {
            Ok(()) => return Ok((Some(file), unusable)),
            Err(fs::TryLockError::WouldBlock) => {}
            Err(fs::TryLockError::Error(error)) => {
                unusable.push(format!("{}: {error}", at.display()));
            }
        }
    }
    if unusable.len() == count as usize {
        return Err(io::Error::other(unusable.join("; ")));
    }
    Ok((None, unusable))
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

    // A link among the turns is left out, never followed, and the run
    // waits on the rest, rather than going out counted by nothing; a
    // builder whose every turn is unusable leaves the run the machine's.
    #[test]
    fn a_turn_that_cant_be_used_is_left_out_and_the_rest_still_count() {
        let turns = tempfile::tempdir().unwrap();
        let elsewhere = turns.path().join("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, turns.path().join("changed-builder-1")).unwrap();
        let held: Vec<_> = (1..EACH)
            .map(|_| sending(turns.path(), "builder", false, true))
            .collect();
        assert!(
            held.iter()
                .all(|turn| matches!(turn, Turn::Had(files) if files.len() == 2)),
            "a run lost its turns to one link among them: {held:?}"
        );
        assert!(
            matches!(sending(turns.path(), "builder", true, true), Turn::Full),
            "a run went beside the three turns held, past the link"
        );
        assert!(!elsewhere.exists(), "the link was followed");
        drop(held);
        for turn in 1..=EACH {
            let at = turns.path().join(format!("changed-gone-{turn}"));
            std::os::unix::fs::symlink(&elsewhere, at).unwrap();
        }
        assert!(
            matches!(
                sending(turns.path(), "gone", false, true),
                Turn::Had(files) if files.len() == 1
            ),
            "a run to a builder whose turns are all unusable lost the machine's"
        );
        assert!(
            matches!(
                sending(turns.path(), "gone", false, false),
                Turn::Had(files) if files.is_empty()
            ),
            "an uncounted run to a builder whose turns are all unusable held a turn"
        );
    }

    // With a link among a builder's turns and the rest held, a run waits,
    // rather than going out or giving up, and takes the turn given back.
    #[test]
    fn a_run_waits_past_a_link_for_a_turn_given_back() {
        let turns = tempfile::tempdir().unwrap();
        let elsewhere = turns.path().join("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, turns.path().join("changed-builder-1")).unwrap();
        let mut held: Vec<_> = (1..EACH)
            .map(|_| sending(turns.path(), "builder", false, true))
            .collect();
        let (told, heard) = std::sync::mpsc::channel();
        let path = turns.path().to_owned();
        let waiting = thread::spawn(move || {
            let turn = sending(&path, "builder", false, true);
            let _ = told.send(matches!(&turn, Turn::Had(files) if files.len() == 2));
            turn
        });
        assert!(
            heard.recv_timeout(Duration::from_secs(1)).is_err(),
            "a run past a link went out beside three held turns"
        );
        held.pop();
        assert_eq!(
            heard.recv_timeout(Duration::from_secs(10)),
            Ok(true),
            "a waiting run didn't take the turn given back, with the machine's"
        );
        drop(waiting.join());
        assert!(!elsewhere.exists(), "the link was followed");
    }

    // Where none of the machine's eight can be used, a run is still
    // counted by its builder's four, which still hold.
    #[test]
    fn a_machine_whose_turns_are_all_unusable_still_counts_each_builders_four() {
        let turns = tempfile::tempdir().unwrap();
        let elsewhere = turns.path().join("elsewhere");
        for turn in 1..=AT_ONCE {
            let at = turns.path().join(format!("changed-{turn}"));
            std::os::unix::fs::symlink(&elsewhere, at).unwrap();
        }
        let held: Vec<_> = (0..EACH)
            .map(|_| sending(turns.path(), "builder", false, true))
            .collect();
        assert!(
            held.iter()
                .all(|turn| matches!(turn, Turn::Had(files) if files.len() == 1)),
            "a run lost its builder's turn to the machine's links: {held:?}"
        );
        assert!(
            matches!(sending(turns.path(), "builder", true, true), Turn::Full),
            "a fifth run went to a builder running four"
        );
        drop(held);
    }

    // Where neither side's turns can be had, a counted run goes on with
    // none, rather than waiting for ever or giving up.
    #[test]
    fn a_counted_run_whose_every_turn_is_unusable_goes_on_without_one() {
        let turns = tempfile::tempdir().unwrap();
        let elsewhere = turns.path().join("elsewhere");
        let names = (1..=AT_ONCE)
            .map(|turn| format!("changed-{turn}"))
            .chain((1..=EACH).map(|turn| format!("changed-builder-{turn}")));
        for name in names {
            std::os::unix::fs::symlink(&elsewhere, turns.path().join(name)).unwrap();
        }
        assert!(
            matches!(
                sending(turns.path(), "builder", false, true),
                Turn::Had(files) if files.is_empty()
            ),
            "a counted run whose every turn is unusable held one, or none was given"
        );
        assert!(!elsewhere.exists(), "a link was followed");
    }

    // Each side that can't be had is said once, in one line that holds
    // with the other side's, so no line says a run is counted by what the
    // next says can't be had.
    #[test]
    fn a_turn_that_cant_be_had_is_said_once_with_what_still_counts() {
        let (machines, builders) = (io::Error::other("m"), io::Error::other("b"));
        let said = |counted, machines, builders| unhad("builder", counted, machines, builders);
        assert_eq!(
            said(true, None, None),
            None,
            "a run with both turns said one"
        );
        assert_eq!(
            said(true, Some(&machines), None).as_deref(),
            Some(
                "this machine's turns can't be had (m), so this run is counted only by its \
                 builder's"
            ),
            "a run without the machine's turn didn't say its builder's still counts"
        );
        assert_eq!(
            said(true, None, Some(&builders)).as_deref(),
            Some("builder's turns can't be had (b), so this run holds only the machine's"),
            "a counted run without its builder's turn didn't say the machine's holds"
        );
        assert_eq!(
            said(false, None, Some(&builders)).as_deref(),
            Some("builder's turns can't be had (b), so this run goes on without one"),
            "an uncounted run without its builder's turn said something holds it"
        );
        assert_eq!(
            said(true, Some(&machines), Some(&builders)).as_deref(),
            Some(
                "neither this machine's turns (m) nor builder's (b) can be had, so this run \
                 goes on without one"
            ),
            "a run with neither turn didn't say so in one line"
        );
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
