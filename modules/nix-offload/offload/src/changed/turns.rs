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
//!
//! Runs waiting for a turn take it in the order they came, each holding a
//! ticket in a line beside the turns ([`Ticket`]). On 26 November, with
//! whichever waiter looked first taking a freed turn, three runs waited
//! out their callers' 90-minute limits while later runs went ahead, and
//! none said how long it might be.
//!
//! Long because its cases are over half of it: each holds real locks on
//! the turns' and the line's files, which only a folder of its own shows.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    // A free turn is taken at once only while no run waits in line, so a
    // run that has just come never goes ahead of one that's waited. A line
    // that can't be read is no line, so the turns still bound the runs.
    let (turn, unusable) = if Ticket::waiting(turns, name, None).unwrap_or(0) == 0 {
        free(turns, name, count)?
    } else {
        (None, Vec::new())
    };
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

/// How often a waiting run says where it stands in line.
const SAID_EVERY: Duration = Duration::from_mins(5);

/// One of `count` turns named `name` once one is free and no run that
/// came before this one still waits for it, looking again every half
/// second, and saying every [`SAID_EVERY`] how many wait ahead. Where the
/// line can't be used, said once, the first to look takes a freed turn,
/// as before there was a line, rather than the run going out uncounted.
fn waited(turns: &Path, name: &str, count: u32) -> io::Result<fs::File> {
    let ticket = Ticket::taken(turns, name)
        .inspect_err(|error| {
            eprintln!(
                "nix-offload: the line for the turns named {name} can't be used ({error}), so \
                 this takes the first turn freed"
            );
        })
        .ok();
    let ahead_of = |ticket: &Option<Ticket>| {
        ticket.as_ref().map_or(0, |ticket| {
            ticket.looked();
            Ticket::waiting(turns, name, Some(ticket)).unwrap_or(0)
        })
    };
    let began = Instant::now();
    let first = ahead_of(&ticket);
    let mut said = began;
    loop {
        let ahead = ahead_of(&ticket);
        if ahead == 0
            && let (Some(turn), _) = free(turns, name, count)?
        {
            return Ok(turn);
        }
        if said.elapsed() >= SAID_EVERY {
            eprintln!("nix-offload: {}", standing(ahead, first, began.elapsed()));
            said = Instant::now();
        }
        // A run holds its turn for minutes, so half a second's lag in
        // taking a freed one costs nothing and keeps the wait cheap.
        thread::sleep(Duration::from_millis(500));
    }
}

/// Where a run stands that has waited `waited` with `ahead` runs still
/// before it, of the `first` that were when it came: how long the rest
/// is likely to take at the pace those before it have gone.
fn standing(ahead: usize, first: usize, waited: Duration) -> String {
    let minutes = waited.as_secs() / 60;
    let gone = first.saturating_sub(ahead);
    if ahead == 0 {
        return format!("this run is next for a turn, after {minutes} min waiting");
    }
    let runs = if ahead == 1 {
        "1 run waits".to_owned()
    } else {
        format!("{ahead} runs wait")
    };
    if gone == 0 {
        return format!("{runs} ahead of this one, and none has had its turn in {minutes} min");
    }
    let likely = waited.as_secs() * ahead as u64 / gone as u64 / 60;
    format!(
        "{runs} ahead of this one; at the pace since it came, its turn is about {likely} min \
         off"
    )
}

/// How long a waiting run's ticket may go unlooked at before it stops
/// counting: a run stopped while it waits, as by a ^Z, keeps its lock but
/// doesn't look, and would otherwise hold every run behind it while turns
/// sat free. It counts again once it looks, so it keeps its place.
const LOOKED_WITHIN: Duration = Duration::from_secs(30);

/// The folder of the line for `name`, refused when it's a link, so no run
/// clears plain files in a folder a link names.
fn line_of(turns: &Path, name: &str) -> io::Result<PathBuf> {
    let line = turns.join(format!("{name}.line"));
    match fs::symlink_metadata(&line) {
        Ok(found) if !found.is_dir() => Err(io::Error::other(format!(
            "{} isn't a folder of its own",
            line.display()
        ))),
        Ok(_) => Ok(line),
        Err(error) => Err(error),
    }
}

/// Whether `named` is a ticket's name, as [`Ticket::taken`] makes them:
/// when it came, in 24 digits, then the run's process and its count, so
/// nothing else in the line's folder is ever counted or cleared.
fn is_ticket(named: &str) -> bool {
    let parts: Vec<_> = named.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 24
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

/// A run's place in the line for the turns named `name`: a file in
/// `<name>.line/`, named by when it came, locked while its run waits and
/// removed once it doesn't. A ticket is locked before it's put in the
/// line, so one found unlocked is a dead run's, and is cleared.
#[derive(Debug)]
struct Ticket {
    at: PathBuf,
    held: fs::File,
}

impl Ticket {
    /// A ticket at the end of the line for `name`.
    fn taken(turns: &Path, name: &str) -> io::Result<Self> {
        /// Tells apart tickets one process takes in the same nanosecond.
        static TAKEN: AtomicU64 = AtomicU64::new(0);
        fs::create_dir_all(turns.join(format!("{name}.line")))?;
        let line = line_of(turns, name)?;
        let came = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let named = format!(
            "{came:024}-{}-{}",
            std::process::id(),
            TAKEN.fetch_add(1, Ordering::Relaxed)
        );
        let making = line.join(format!(".{named}"));
        let held = crate::there::open_turn(&making, true).map_err(io::Error::other)?;
        held.lock()?;
        let at = line.join(named);
        fs::rename(&making, &at).inspect_err(|_| {
            let _ = fs::remove_file(&making);
        })?;
        Ok(Self { at, held })
    }

    /// Marks the ticket looked at now, so it keeps counting ([`LOOKED_WITHIN`]).
    fn looked(&self) {
        let _ = self.held.set_modified(SystemTime::now());
    }

    /// How many runs wait in line for `name`, of those before `mine` when
    /// it's given: a ticket counts while its run holds its lock and has
    /// looked within [`LOOKED_WITHIN`], and one left by a run that died is
    /// cleared as it's found.
    fn waiting(turns: &Path, name: &str, mine: Option<&Self>) -> io::Result<usize> {
        let line = match line_of(turns, name) {
            Ok(line) => line,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
        };
        let tickets = fs::read_dir(&line)?;
        let mine = mine.and_then(|mine| mine.at.file_name().map(ToOwned::to_owned));
        let mut ahead = 0;
        for ticket in tickets.flatten() {
            let named = ticket.file_name();
            let before = mine.as_ref().is_none_or(|mine| named < *mine);
            if !is_ticket(&named.to_string_lossy()) || !before {
                continue;
            }
            // A ticket that's a link or can't be read is no run's.
            let Ok(file) = crate::there::open_turn(&ticket.path(), false) else {
                continue;
            };
            // Shared, so a ticket another run is clearing still reads as
            // no live run's rather than one that waits.
            match file.try_lock_shared() {
                Err(fs::TryLockError::WouldBlock) => {
                    let looked = file.metadata().and_then(|found| found.modified());
                    let lately = looked
                        .ok()
                        .and_then(|at| at.elapsed().ok())
                        .is_none_or(|since| since < LOOKED_WITHIN);
                    if lately {
                        ahead += 1;
                    }
                }
                Ok(()) => {
                    let _ = fs::remove_file(ticket.path());
                }
                Err(fs::TryLockError::Error(_)) => {}
            }
        }
        Ok(ahead)
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.at);
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

    /// Starts a run to `host`, uncounted, that waits for its turn, and
    /// gives what tells when it has one, once its ticket is in line.
    fn waiter(
        turns: &Path,
        host: &str,
        tickets: usize,
    ) -> (std::sync::mpsc::Receiver<()>, thread::JoinHandle<Turn>) {
        let (told, heard) = std::sync::mpsc::channel();
        let (path, owned) = (turns.to_owned(), host.to_owned());
        let waiting = thread::spawn(move || {
            let turn = sending(&path, &owned, false, false);
            let _ = told.send(());
            turn
        });
        let name = format!("changed-{host}");
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while Ticket::waiting(turns, &name, None).unwrap() < tickets {
            assert!(
                std::time::Instant::now() < until,
                "a waiting run never took its ticket"
            );
            thread::sleep(Duration::from_millis(10));
        }
        (heard, waiting)
    }

    #[test]
    fn runs_waiting_for_a_turn_take_it_in_the_order_they_came() {
        let turns = tempfile::tempdir().unwrap();
        let mut held: Vec<_> = (0..EACH)
            .map(|_| sending(turns.path(), "builder", false, false))
            .collect();
        let (first, first_run) = waiter(turns.path(), "builder", 1);
        let (second, second_run) = waiter(turns.path(), "builder", 2);
        held.pop();
        assert_eq!(
            first.recv_timeout(Duration::from_secs(10)),
            Ok(()),
            "the run that came first didn't take the turn given back"
        );
        assert!(
            second.recv_timeout(Duration::from_secs(1)).is_err(),
            "the run that came second took a turn while four were held"
        );
        held.pop();
        assert_eq!(
            second.recv_timeout(Duration::from_secs(10)),
            Ok(()),
            "the run that came second never had the next turn"
        );
        drop((first_run.join(), second_run.join()));
    }

    #[test]
    fn a_run_that_comes_while_another_waits_doesnt_go_ahead_of_it() {
        let turns = tempfile::tempdir().unwrap();
        let mut held: Vec<_> = (0..EACH)
            .map(|_| sending(turns.path(), "builder", false, false))
            .collect();
        let (_waited, waiting) = waiter(turns.path(), "builder", 1);
        held.pop();
        assert!(
            matches!(sending(turns.path(), "builder", true, false), Turn::Full),
            "a run that came last took the turn a waiting run was owed"
        );
        drop(held);
        drop(waiting.join());
    }

    #[test]
    fn a_dead_runs_ticket_is_cleared_rather_than_waited_on() {
        let turns = tempfile::tempdir().unwrap();
        let line = turns.path().join("changed-builder.line");
        fs::create_dir_all(&line).unwrap();
        let dead = line.join(format!("{:024}-1-0", 1));
        fs::write(&dead, "").unwrap();
        assert!(
            matches!(sending(turns.path(), "builder", true, false), Turn::Had(_)),
            "a run waited on a ticket no run holds"
        );
        assert!(!dead.exists(), "a dead run's ticket was left in line");
    }

    // A run stopped while it waits keeps its lock but doesn't look, so its
    // ticket stops holding the line, and holds it again once it looks.
    #[test]
    fn a_ticket_not_looked_at_lately_doesnt_hold_the_line() {
        let turns = tempfile::tempdir().unwrap();
        let line = turns.path().join("changed-builder.line");
        fs::create_dir_all(&line).unwrap();
        let stopped = fs::File::create(line.join(format!("{:024}-1-0", 1))).unwrap();
        stopped.lock().unwrap();
        stopped
            .set_modified(SystemTime::now() - LOOKED_WITHIN * 2)
            .unwrap();
        assert!(
            matches!(sending(turns.path(), "builder", true, false), Turn::Had(_)),
            "a stopped run's ticket held the line while every turn was free"
        );
        stopped.set_modified(SystemTime::now()).unwrap();
        assert!(
            matches!(sending(turns.path(), "builder", true, false), Turn::Full),
            "a run went ahead of one that waits and has just looked"
        );
    }

    // A line that's a link is no line: nothing in the folder it names is
    // cleared, and the turns still bound the runs.
    #[test]
    fn a_line_that_is_a_link_clears_nothing_where_it_points() {
        let turns = tempfile::tempdir().unwrap();
        let elsewhere = turns.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let kept = [
            elsewhere.join("notes"),
            elsewhere.join(format!("{:024}-1-0", 1)),
        ];
        for file in &kept {
            fs::write(file, "").unwrap();
        }
        std::os::unix::fs::symlink(&elsewhere, turns.path().join("changed-builder.line")).unwrap();
        let held: Vec<_> = (0..EACH)
            .map(|_| sending(turns.path(), "builder", true, false))
            .collect();
        assert!(
            held.iter()
                .all(|turn| matches!(turn, Turn::Had(files) if files.len() == 1)),
            "a run lost its turn to a line that's a link: {held:?}"
        );
        assert!(
            matches!(sending(turns.path(), "builder", true, false), Turn::Full),
            "a fifth run went to a builder running four past a linked line"
        );
        assert!(
            kept.iter().all(|file| file.exists()),
            "a file the linked line names was cleared"
        );
    }

    #[test]
    fn a_waiting_run_says_how_many_are_ahead_and_how_long_is_likely() {
        let minutes = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(
            standing(0, 3, minutes(20)),
            "this run is next for a turn, after 20 min waiting"
        );
        assert_eq!(
            standing(1, 1, minutes(5)),
            "1 run waits ahead of this one, and none has had its turn in 5 min"
        );
        assert_eq!(
            standing(4, 4, minutes(10)),
            "4 runs wait ahead of this one, and none has had its turn in 10 min"
        );
        assert_eq!(
            standing(3, 6, minutes(30)),
            "3 runs wait ahead of this one; at the pace since it came, its turn is about \
             30 min off",
            "three gone in thirty minutes leaves three more about thirty off"
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
