//! The net's round: for each new head of main, run every check the merges
//! since the last head it passed changed, VM tests and the rest, and
//! revert the merge that broke one.
//!
//! The checks other than VM tests run too, and first, since they take
//! seconds: on 8 November four of them were red on main by 11:00, each
//! named by changed-checks at its merge and none caught, as the net ran
//! only VM tests and a branch built them before rebasing, or not at all.
//!
//! A test fails for real when it fails twice at the same commit, which is
//! the same input. One that fails and then passes is flaky: it goes on the
//! quarantine list with the PR of the newest merge that moved it, and not
//! the whole system with it, as its owner, and nothing is reverted for
//! it, nor for a quarantined test that fails, listed at the round's head
//! or at main's newest. A test whose breaking merge is found but won't
//! revert goes on the list too, with that merge's PR as its owner, and the
//! hold is cleared, and so does one failing at the round's base as well as
//! its head, owned by the PR that last took it off the list. A run the builder never took
//! (`nix-offload` exits 75), or whose builder couldn't fetch the commit's
//! source, is tried again later, and is no failure, and so is one stopped
//! before it ended, for running too long or going silent: twice at a commit, and the test is left there with a
//! finding, blaming no merge. A test that passed at a commit never runs
//! there again.
//!
//! The round talks to git, nix and GitHub through `World`, so each of
//! those choices is tested against a stand-in (the tests below).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use serde_json::{Value, json};

use crate::movers;
use crate::running;
use crate::state::{Ran, State, now, short};
use crate::subtests;

mod settling;
use settling::Job;
pub use settling::Settler;

/// What `nix-offload` exits with when no builder took the build.
pub const UNSENT: i32 = 75;

/// How a revert or quarantine whose checks no builder took fails, so the
/// net tries it again next round rather than leave its hold for a person.
pub const UNCHECKED: &str = "no builder took its checks";

/// How a revert fails that a pause or a hold on the machine came to stop
/// while its checks ran, so the net tries it again the first round it
/// may start, rather than land what someone paused it to keep off main.
pub const PAUSED: &str = "held while its checks ran";

/// How a revert fails that GitHub refused to merge because main moved
/// after its checks ran, so the net checks it again on the new main next
/// round. On 13 November #4049 lost that race at 05:49, and the net
/// quarantined three of vm-desktop-captures' subtests at once instead.
pub const MOVED: &str = "main moved before it merged";

/// Whether a revert that failed for `why` is tried again next round.
fn tried_again(why: &str) -> bool {
    [UNCHECKED, PAUSED, MOVED]
        .iter()
        .any(|retried| why.starts_with(retried))
}

/// How a revert fails that main's newest head made needless while its
/// checks ran, so it's neither merged nor tried again.
pub const DROPPED: &str = "main's newest head passes its test, so it was dropped";

/// How a revert fails whose test fails at main's newest head with it in
/// too, so it lands nothing.
const MENDS_NOTHING: &str = "the revert mends nothing";

/// How many times a run the builder never took is sent again, two minutes
/// apart, before the round leaves it for the next.
const SENDS: u32 = 15;

/// How many times a test fails at a commit, and passes not once, before
/// it counts as broken there.
const FAILURES: usize = 2;

/// How many runs of a test at a commit may be stopped before they end,
/// for running too long or going silent, before the net leaves the test
/// there: a builder that hangs on it would hang again.
const STALLS: usize = 2;

/// How many VM tests run at once: the room three VM builders have for
/// them, at nix-offload's `VM_ROOM` each. The net's runs go as bulk, which
/// every other run goes before, so taking the room costs a session's run
/// nothing. On 6 November a round of 33 tests, about ten minutes each,
/// took 85 minutes at four at once, filling only the first of six VM
/// builders and sending every run without a wait, while 13 merges landed
/// behind it untested; at twelve, a round lasts about as long as its
/// longest test.
pub const AT_ONCE: usize = 12;

/// How many merges a bisect tries at once, spread evenly over what's left,
/// so each step leaves a quarter of it rather than a half. A step waits on
/// its slowest merge, and one that fails runs twice in a row, so halving a
/// hundred merges took seven steps of up to two VM runs each, hours in
/// which main stayed broken; by quarters it takes four. Its runs go as
/// bulk, so they take no room another session's run is waiting for. It is
/// the least a step tries: a search alone takes the room `AT_ONCE` names,
/// shared with the others under way (`Settler::splits`).
const SPLITS: usize = 3;

/// How many failed tests' searches run side by side, each trying at least
/// `SPLITS` merges a step, so together they fill the room `AT_ONCE` names.
/// On 9 November a round's failures were searched one after another, about
/// an hour each, and the round at 7b2252da was three hours old with four
/// still to search.
const SEARCHES: usize = AT_ONCE / SPLITS;

/// What changed-checks named between two heads.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Changed {
    /// The checks added or changed, VM tests and the rest.
    pub moved: Vec<String>,
    /// The checks that no longer evaluate.
    pub broken: Vec<String>,
}

impl Changed {
    /// Read changed-checks' lines, `kind name` each.
    pub fn parse(said: &str) -> Self {
        let mut changed = Self::default();
        for line in said.lines() {
            let mut words = line.split_whitespace();
            match (words.next(), words.next()) {
                (Some("added" | "changed"), Some(name)) => changed.moved.push(name.to_owned()),
                (Some("broken"), Some(name)) => changed.broken.push(name.to_owned()),
                _ => {}
            }
        }
        changed
    }

    /// The VM tests among those that moved.
    pub fn vm_tests(&self) -> Vec<String> {
        self.moved
            .iter()
            .filter(|name| name.starts_with("vm-"))
            .cloned()
            .collect()
    }

    /// Every check that moved, the others before the VM tests, which take
    /// minutes each where the others take seconds, so a broken one is
    /// found before the round's VM tests fill the builders.
    pub fn checks(&self) -> Vec<String> {
        let (tests, others): (Vec<String>, Vec<String>) = self
            .moved
            .iter()
            .cloned()
            .partition(|name| name.starts_with("vm-"));
        others.into_iter().chain(tests).collect()
    }
}

/// A merge that broke a test, and what the revert needs to say of it.
#[derive(Debug, Clone)]
pub struct Bad {
    pub commit: String,
    pub subject: String,
    pub pr: Option<u64>,
    pub test: String,
    pub log: String,
}

/// A change the net made and merged on main.
#[derive(Debug, Clone)]
pub struct Merged {
    pub pr: u64,
    pub commit: Option<String>,
}

/// What the round asks of git, nix and GitHub.
pub trait World: Sync {
    /// Why nothing may start now, if something says so: a pause, or a
    /// hold on the whole machine.
    fn held(&self) -> Option<String>;
    /// Fetch main and give its head.
    fn head(&self) -> Result<String, String>;
    /// Main's head as it was last fetched, without asking the remote.
    fn fetched(&self) -> Option<String>;
    /// The first-parent commits after `base` up to `head`, oldest first.
    fn merges(&self, base: &str, head: &str) -> Result<Vec<String>, String>;
    /// A commit's subject line.
    fn subject(&self, commit: &str) -> String;
    /// A commit's first parent: main as it was before the merge.
    fn parent(&self, commit: &str) -> Result<String, String>;
    /// What changed-checks names between `base` and `head`.
    fn changed(&self, base: &str, head: &str) -> Result<Changed, String>;
    /// Whether `commit` has the check at all.
    fn has_check(&self, commit: &str, check: &str) -> Result<bool, String>;
    /// The derivation a check builds at a commit, which names everything
    /// it runs.
    fn drv(&self, commit: &str, check: &str) -> Result<String, String>;
    /// Build a check at a commit, its log written to `log`, and give its
    /// exit, none when it was stopped.
    fn build(&self, commit: &str, check: &str, log: &Path) -> Option<i32>;
    /// The tests quarantined whole at `head`.
    fn quarantined(&self, head: &str) -> Vec<String>;
    /// The subtests of `test` quarantined at `head`, none when it's listed
    /// whole or not at all.
    fn listed_subtests(&self, head: &str, test: &str) -> BTreeSet<String>;
    /// Revert a bad merge on main through a PR merged on its own checks,
    /// once `still`, asked with main's newest head and the revert merged
    /// into it, says it's needed: true to land, false to drop it as
    /// needless, or why it lands nothing.
    fn revert(
        &self,
        bad: &Bad,
        still: &dyn Fn(&str, &str) -> Result<bool, String>,
    ) -> Result<Merged, String>;
    /// The PR whose merge, at or before `head`, last took off the
    /// quarantine list what fails in `test` now, the `failing` subtests or
    /// where none are named the test, if one did.
    fn unlisted_by(&self, test: &str, failing: &BTreeSet<String>, head: &str) -> Option<u64>;
    /// Put a flaky test on the quarantine list through a merged PR,
    /// excusing only `subtests` of it where any are named.
    fn quarantine(
        &self,
        test: &str,
        subtests: &BTreeSet<String>,
        owner: Option<u64>,
        commit: &str,
        reason: &str,
    ) -> Result<Merged, String>;
    /// Whether main holds `commit`.
    fn on_main(&self, commit: &str) -> bool;
    /// Whether the machine already runs `commit`, false when that can't be
    /// told, so a hold stands.
    fn runs(&self, commit: &str) -> bool;
    /// Wait this many seconds.
    fn wait(&self, seconds: u64);
    /// Stop the runs of `check` at `commit` under way, and any begun until
    /// `keep_runs`, since nothing needs what they'd find: each ends as
    /// stopped.
    fn drop_runs(&self, commit: &str, check: &str);
    /// Let runs of `check` at `commit` go on again after `drop_runs`.
    fn keep_runs(&self, commit: &str, check: &str);
    /// Whether runs of `check` at `commit` are dropped now.
    fn dropped(&self, commit: &str, check: &str) -> bool;
}

/// How a test ended at a commit, over every run of it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    /// Failed, then passed, at the same commit.
    Flaky,
    /// Failed twice at the same commit.
    Failed,
    /// The builder never took it.
    Unsent,
    /// Stopped twice at the same commit before it ended, so it says
    /// nothing of the merge.
    Stalled,
}

/// What a round came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Round {
    /// Nothing new to look at, or held.
    Idle,
    /// Every test the head changed passed, so it's the next base.
    Passed(String),
    /// A test failed for real at the head.
    Failed(String),
    /// Some test wasn't taken, so the head is looked at again.
    Unfinished(String),
}

/// Run `check` at `commit` until it has passed or failed twice there,
/// counting the runs it had before. A run stopped before it ended counts
/// towards neither, and two of them at a commit leave the test there.
/// A second failure is first looked for on the subtests the first failed
/// in alone (`confirmed_apart`). Runs of it there already going, for a
/// round or a search, are waited for and counted rather than run again
/// beside them.
pub fn attempt(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
    let turn = state.turn(commit, check);
    let _turn = turn
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    attempt_in_turn(world, state, commit, check)
}

/// `attempt`, by one holding the turn of `check` at `commit`.
fn attempt_in_turn(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
    let before = state.runs(commit, check);
    let mut failures = before.iter().filter(|ran| **ran == Ran::Failed).count();
    let mut stalls = before.iter().filter(|ran| **ran == Ran::Stalled).count();
    if before.contains(&Ran::Passed) {
        return if failures > 0 {
            Outcome::Flaky
        } else {
            Outcome::Passed
        };
    }
    let mut apart_tried = false;
    while failures < FAILURES {
        if stalls >= STALLS {
            return Outcome::Stalled;
        }
        if failures + 1 == FAILURES && !apart_tried {
            apart_tried = true;
            match confirmed_apart(world, state, commit, check) {
                None => return Outcome::Unsent,
                Some(true) => {
                    failures += 1;
                    continue;
                }
                Some(false) => {}
            }
        }
        // Every run before was a failure, so the next is one past them; a
        // stall's log is the next run's to write over.
        let Some(ran) = run_once(world, state, commit, check, failures + 1) else {
            return Outcome::Unsent;
        };
        if ran == Ran::Passed {
            return if failures > 0 {
                Outcome::Flaky
            } else {
                Outcome::Passed
            };
        }
        if ran == Ran::Stalled {
            stalls += 1;
            state.say(&format!(
                "{check} stalled at {} ({stalls} of {STALLS}): it was stopped before it ended, \
                 so it isn't counted as a failure",
                short(commit)
            ));
            continue;
        }
        failures += 1;
        state.say(&format!(
            "{check} failed at {} ({failures} of 2)",
            short(commit)
        ));
    }
    Outcome::Failed
}

/// Run `check` once at `commit`, its log numbered `run`, sending it again
/// while no builder takes it, and record how it ended; none when no
/// builder ever took it.
fn run_once(
    world: &dyn World,
    state: &State,
    commit: &str,
    check: &str,
    run: usize,
) -> Option<Ran> {
    let log = state.log(commit, check, u32::try_from(run).unwrap_or(0));
    let mut sends = 0;
    let (code, took) = loop {
        let began = std::time::Instant::now();
        let code = world.build(commit, check, &log);
        // A run stopped because nothing needs it now is no outcome, and no
        // stall: the test ran fine as far as it went.
        if world.dropped(commit, check) {
            return None;
        }
        if code != Some(UNSENT) {
            break (code, began.elapsed().as_secs());
        }
        sends += 1;
        if sends >= SENDS {
            state.say(&format!("no builder took {check} at {}", short(commit)));
            return None;
        }
        world.wait(120);
    };
    let ran = match code {
        Some(0) => Ran::Passed,
        Some(_) if excused(world, state, commit, check, &log) => Ran::Passed,
        Some(_) => Ran::Failed,
        None => Ran::Stalled,
    };
    if let Err(why) = state.record(commit, check, ran, code, &log, took) {
        state.say(&why);
    }
    Some(ran)
}

/// Whether `check`'s last failure at `commit` is confirmed by running the
/// subtests it failed in alone, as a search's steps do, and failing again:
/// none when nothing needs it now. On 14 November the second whole run of
/// vm-desktop-launcher-waits-on-joel took 630 s before its search began,
/// where its failed subtest alone takes about 45. What this can't confirm,
/// a narrowed check, a run naming no subtest, one that passes alone or
/// can't be built or selected, is left to a whole run, so a flake is still
/// told from a break only by the whole test passing.
fn confirmed_apart(world: &dyn World, state: &State, commit: &str, check: &str) -> Option<bool> {
    if !subtests::subtests_of(check).is_empty() {
        return Some(false);
    }
    let listed = world
        .fetched()
        .map(|newest| world.listed_subtests(&newest, check))
        .unwrap_or_default();
    let first = state.log(commit, check, 1);
    let failed = &subtests::failed_in(crate::world::lines_of(&first)) - &listed;
    let narrowed = subtests::narrowed(check, &[failed]);
    if narrowed == check {
        return Some(false);
    }
    // The run is logged as the whole test's second, where a search reads
    // which subtests failed in both, and a whole run writes over it.
    let log = state.log(commit, check, 2);
    let began = std::time::Instant::now();
    let code = world.build(commit, &narrowed, &log);
    if world.dropped(commit, check) {
        return None;
    }
    let lines: Vec<String> = crate::world::lines_of(&log).collect();
    let failed = matches!(code, Some(code) if code != 0 && code != UNSENT)
        && !subtests::none_selected(&lines)
        && !subtests::unbuildable(&lines);
    if !failed {
        state.say(&format!(
            "{check} didn't fail again at {} on {} alone, so it runs whole",
            short(commit),
            subtests::subtests_of(&narrowed).join(", ")
        ));
        return Some(false);
    }
    let took = began.elapsed().as_secs();
    if let Err(why) = state.record(commit, check, Ran::Failed, code, &log, took) {
        state.say(&why);
    }
    state.say(&format!(
        "{check} failed at {} (2 of 2), on {} alone",
        short(commit),
        subtests::subtests_of(&narrowed).join(", ")
    ));
    Some(true)
}

/// Whether a failed run of `check` at `commit`, its log at `log`, failed
/// only in subtests main's list names for its test, so it counts as
/// passed and what follows a pass follows (quarantine.rs). The list is
/// main's as last fetched, which its owners keep, rather than the
/// commit's: a search runs at commits from before an entry was written.
fn excused(world: &dyn World, state: &State, commit: &str, check: &str, log: &Path) -> bool {
    let Some(newest) = world.fetched() else {
        return false;
    };
    let test = subtests::whole(check);
    let listed = world.listed_subtests(&newest, test);
    let failing = subtests::failed_in(crate::world::lines_of(log));
    if !crate::quarantine::excuses(&listed, &failing) {
        return false;
    }
    state.say(&format!(
        "{check} failed at {} only in subtests main lists as quarantined ({}), so it counts \
         as passed",
        short(commit),
        failing.into_iter().collect::<Vec<_>>().join(", ")
    ));
    true
}

/// The subtests that failed in `test`'s runs at `commit`, which the net
/// lists it by when it quarantines it: none where no run's log names one,
/// and then the whole test is listed.
fn failing_at(state: &State, commit: &str, test: &str) -> BTreeSet<String> {
    (1..=FAILURES)
        .filter_map(|run| u32::try_from(run).ok())
        .flat_map(|run| subtests::failed_in(crate::world::lines_of(&state.log(commit, test, run))))
        .collect()
}

/// Like `attempt`, but every run still needed to call `check` broken at
/// `commit` starts at once, for a search and a fix-forward check, where
/// a failure is likely and the search waits on it. A failure is then
/// confirmed in one run's time rather than two in a row, which on
/// 7 November was about five minutes of each step; the cost is a run
/// beside the first where the first passes. A stall leaves the rest to
/// `attempt`, one at a time.
fn attempt_together(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
    let turn = state.turn(commit, check);
    let _turn = turn
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let before = state.runs(commit, check);
    let failures = before.iter().filter(|ran| **ran == Ran::Failed).count();
    if !before.contains(&Ran::Passed) && failures + 1 < FAILURES {
        let ended: Vec<Option<Ran>> = std::thread::scope(|scope| {
            let runs: Vec<_> = (failures + 1..=FAILURES)
                .map(|run| scope.spawn(move || run_once(world, state, commit, check, run)))
                .collect();
            runs.into_iter()
                .map(|run| run.join().unwrap_or(None))
                .collect()
        });
        if ended.contains(&None) {
            return Outcome::Unsent;
        }
        let failed = ended
            .iter()
            .filter(|ran| **ran == Some(Ran::Failed))
            .count();
        state.say(&format!(
            "{check} failed {failed} of the {} runs at once at {}",
            ended.len(),
            short(commit)
        ));
    }
    attempt_in_turn(world, state, commit, check)
}

/// Run each check at `commit`, `AT_ONCE` at a time and longest first, and
/// give each outcome in the order asked. `then` hears each outcome the moment its check
/// ends, on the thread that ran it, while the others still run.
pub fn attempt_all(
    world: &dyn World,
    state: &State,
    commit: &str,
    checks: &[String],
    then: &(dyn Fn(&str, Outcome) + Sync),
) -> Vec<Outcome> {
    let next = Mutex::new(0_usize);
    let order = longest_first(checks, &state.lasted());
    let outcomes = Mutex::new(vec![Outcome::Unsent; checks.len()]);
    std::thread::scope(|scope| {
        for _ in 0..AT_ONCE.min(checks.len()) {
            scope.spawn(|| {
                loop {
                    let nth = {
                        let mut next = next
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let nth = *next;
                        *next += 1;
                        nth
                    };
                    let Some(&index) = order.get(nth) else {
                        break;
                    };
                    let check = &checks[index];
                    if world.held().is_some() {
                        continue;
                    }
                    let outcome = attempt(world, state, commit, check);
                    then(check, outcome);
                    outcomes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[index] = outcome;
                }
            });
        }
    });
    outcomes
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Find the merge among `merges` (oldest first, the last failing `check`)
/// that broke it, the commit before the first being where it passed. A
/// commit without the check counts as passing it, so what it finds is
/// checked by `before` before it's blamed. None when a run wasn't taken, so it
/// can't be told, or when `go_on`, asked before each step after the first,
/// says to stop. Each step tries as many merges as `splits` says then.
pub fn bisect(
    world: &dyn World,
    state: &State,
    merges: &[String],
    check: &str,
    go_on: &dyn Fn() -> bool,
    splits: &dyn Fn() -> usize,
) -> Option<String> {
    // `good` is how many of the merges are known to pass, `bad` the index
    // of the first known to fail.
    let (mut good, mut bad) = known(state, merges, check)?;
    let mut first = true;
    while good < bad {
        if !std::mem::take(&mut first) && !go_on() {
            return None;
        }
        let tried = spread(good, bad, splits());
        let passed = step(world, state, merges, check, &tried)?;
        // The first merge tried that fails is where it may have broken, and
        // the last before it to pass is past where it can have. A pass after
        // a failure tells nothing, since the failure already bounds it, and
        // a run there may have been stopped for that.
        if let Some(first) = passed.iter().position(|ok| *ok == Some(false)) {
            bad = tried[first];
        }
        if let Some(&last) = tried
            .iter()
            .zip(&passed)
            .rev()
            .find(|&(&index, &ok)| ok == Some(true) && index < bad)
            .map(|(index, _)| index)
        {
            good = last + 1;
        }
    }
    merges.get(bad).cloned()
}

/// Whether `check` passes at each of the `tried` merges, run side by side,
/// none where a run past the first failure was stopped. Once a merge fails,
/// those after it can't move where the step finds it broke, so their runs
/// are stopped rather than waited for: on 14 November a step's runs at
/// four merges past the failure ran 12 minutes longer on crowded builders
/// before vm-desktop-says-what-happens' search could go on. None when a
/// run before the first failure wasn't taken or couldn't be told.
fn step(
    world: &dyn World,
    state: &State,
    merges: &[String],
    check: &str,
    tried: &[usize],
) -> Option<Vec<Option<bool>>> {
    let (told, heard) = std::sync::mpsc::channel();
    let (passed, dropped) = std::thread::scope(|scope| {
        for (nth, &index) in tried.iter().enumerate() {
            let told = told.clone();
            scope.spawn(move || {
                let _ = told.send((nth, passes(world, state, &merges[index], check)));
            });
        }
        drop(told);
        let mut passed = vec![None; tried.len()];
        let mut dropped = vec![false; tried.len()];
        for (nth, ok) in heard {
            passed[nth] = ok;
            if ok != Some(false) {
                continue;
            }
            let later = tried
                .iter()
                .zip(&passed)
                .zip(dropped.iter_mut())
                .skip(nth + 1);
            for ((&index, ran), stopped) in later {
                if ran.is_none() && !*stopped {
                    world.drop_runs(&merges[index], check);
                    *stopped = true;
                }
            }
        }
        (passed, dropped)
    });
    for (&index, _) in tried.iter().zip(&dropped).filter(|(_, stopped)| **stopped) {
        world.keep_runs(&merges[index], check);
    }
    let first_failure = passed
        .iter()
        .position(|ok| *ok == Some(false))
        .unwrap_or(tried.len());
    passed[..first_failure]
        .iter()
        .all(Option::is_some)
        .then_some(passed)
}

/// Up to `splits` indices between `good` and `bad`, spread evenly and each
/// one different: one is the middle, as a bisect's halving takes it.
fn spread(good: usize, bad: usize, splits: usize) -> Vec<usize> {
    let left = bad - good;
    let tried = splits.min(left);
    (1..=tried)
        .map(|nth| good + left * nth / (tried + 1))
        .collect()
}

/// Whether `check` passes at `commit` for a bisect, a commit without it
/// passing; None when a run wasn't taken or it couldn't be told.
fn passes(world: &dyn World, state: &State, commit: &str, check: &str) -> Option<bool> {
    match world.has_check(commit, check) {
        Ok(false) => Some(true),
        Ok(true) => match attempt_together(world, state, commit, check) {
            Outcome::Passed | Outcome::Flaky => Some(true),
            Outcome::Failed => Some(false),
            Outcome::Unsent | Outcome::Stalled => None,
        },
        Err(why) => {
            state.say(&format!(
                "couldn't tell whether {} has {check}: {why}",
                short(commit)
            ));
            None
        }
    }
}

/// Where a bisect over `merges` starts: just after the last merge where
/// `check` is recorded passing, and at the first after that recorded
/// failing twice, or else the last merge. A test that passed at a merge and
/// fails at one before it now fails for something no merge did, such as the
/// date, and searching the merges before the pass would blame one of them.
fn known(state: &State, merges: &[String], check: &str) -> Option<(usize, usize)> {
    let last = merges.len().checked_sub(1)?;
    let runs: Vec<Vec<Ran>> = merges
        .iter()
        .map(|commit| state.runs(commit, check))
        .collect();
    // A pass of the whole test at a commit is a pass of any of its subtests.
    let whole = subtests::whole(check);
    let good = runs
        .iter()
        .zip(merges)
        .rposition(|(ran, commit)| {
            ran.contains(&Ran::Passed)
                || (whole != check && state.runs(commit, whole).contains(&Ran::Passed))
        })
        .map_or(0, |index| (index + 1).min(last));
    let bad = (good..=last)
        .find(|&index| {
            runs[index]
                .iter()
                .filter(|&&ran| ran == Ran::Failed)
                .count()
                >= FAILURES
        })
        .unwrap_or(last);
    Some((good, bad))
}

/// How a test did just before the merge a bisect found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Before {
    Passed,
    Failed,
    /// The check didn't exist before it.
    Absent,
    /// The merge didn't change the check, so it can't have broken it.
    Unchanged,
    Unsent,
}

/// How `check` did at the first parent of `commit`, the merge a bisect
/// found, before it's blamed: the bisect counts its start as passing
/// without a run there, and a commit without the check as passing it, so
/// a test failing since before the start, or a split that only moved a
/// failing case under a new name, would otherwise be blamed on a good
/// merge. So would one that began failing for something outside the code,
/// such as the date, at every commit after its last pass: the bisect then
/// finds the merge just after that pass, which changed nothing it runs.
fn before(world: &dyn World, state: &State, commit: &str, check: &str) -> Result<Before, String> {
    let parent = world.parent(commit)?;
    if !world.has_check(&parent, check)? {
        return Ok(Before::Absent);
    }
    // Only the one test is compared: changed-checks would evaluate every
    // check at both commits, which on 5 November took the laptop over an
    // hour while the round waited.
    if world.drv(&parent, check)? == world.drv(commit, check)? {
        return Ok(Before::Unchanged);
    }
    Ok(match attempt(world, state, &parent, check) {
        Outcome::Passed | Outcome::Flaky => Before::Passed,
        Outcome::Failed => Before::Failed,
        Outcome::Unsent | Outcome::Stalled => Before::Unsent,
    })
}

/// Tell the owner that `test` fails at `commit`, the merge a bisect found,
/// which `before` says didn't break it, and revert and hold nothing.
fn blames_none(
    world: &dyn World,
    state: &State,
    test: &str,
    commit: &str,
    log: &str,
    not_broken_here: Before,
) -> Result<(), String> {
    let (kind, why) = match not_broken_here {
        Before::Absent => (
            "failed-at-birth",
            "the first merge to have it, so it may only have moved a case already failing, \
             as a split does",
        ),
        Before::Unchanged => (
            "failed-unchanged",
            "a merge that didn't change it, so something outside the code broke it, such as \
             the date or the builders",
        ),
        _ => (
            "failed-before",
            "and before it too, so no merge since broke it",
        ),
    };
    // No merge is to blame for a test failing unchanged, so its finding
    // names no owner.
    let owner = (not_broken_here != Before::Unchanged)
        .then(|| pr_of(&world.subject(commit)))
        .flatten();
    state.find(&json!({
        "kind": kind, "test": test, "commit": commit,
        "owner_pr": owner, "log": log, "at": now(),
    }))?;
    state.say(&format!(
        "{test} fails at {}, {why}: nothing is reverted or held",
        short(commit)
    ));
    Ok(())
}

/// Whether `test`, failing at `head`, passes at main's newest head, so a
/// fix forward has landed since; if so its owner is told. `commit` is the
/// merge a search found broke it, none before the search. A round and its
/// search take hours while merges keep landing: on 5 November both tests
/// the round at b74ab91b searched for were fixed forward, and the search
/// held the net nine hours behind main. So it is asked beside a search,
/// which it then stops, and again before a revert, which would only undo
/// the merge a fix sits on. A newest head that builds the test's very
/// derivation fails it as `head` does, so it isn't run there: on 7 November
/// two such runs took 28 minutes of a search.
///
/// A test that fails one of its runs at the newest head isn't fixed but
/// flaky there, so it is listed, as a round lists one, and still nothing
/// is searched for or reverted. On 9 November vm-desktop-captures failed
/// both runs at 06483ebf3aea and one of two at the newest head, was taken
/// as fixed forward, and went on failing 7 of 10 runs on sessions' branches.
fn fixed_forward(
    world: &dyn World,
    state: &State,
    head: &str,
    test: &str,
    commit: Option<&str>,
) -> Result<bool, String> {
    let newest = world.head()?;
    if newest == head || same_drv(world, &newest, head, test) {
        return Ok(false);
    }
    let outcome = attempt_together(world, state, &newest, test);
    told_fixed(world, state, head, &newest, test, commit, outcome)
}

/// Whether `outcome`, `test`'s at main's newest head `newest`, says it
/// was fixed forward since failing at `head`, or broken by `commit`; if so
/// its owner is told, and a test flaky there is listed.
fn told_fixed(
    world: &dyn World,
    state: &State,
    head: &str,
    newest: &str,
    test: &str,
    commit: Option<&str>,
    outcome: Outcome,
) -> Result<bool, String> {
    if !matches!(outcome, Outcome::Passed | Outcome::Flaky) {
        return Ok(false);
    }
    let owner = commit.and_then(|commit| pr_of(&world.subject(commit)));
    if outcome == Outcome::Flaky {
        if !state.found("flaky", test, newest)
            && !world.quarantined(newest).contains(&test.to_owned())
        {
            list_flaky(world, state, newest, test, owner)?;
        }
        state.say(&format!(
            "{test} {} at {}, and fails one of two runs at main's newest head {}, so it is \
             listed there: nothing is searched for, reverted or held",
            if commit.is_some() { "broke" } else { "fails" },
            short(commit.unwrap_or(head)),
            short(newest)
        ));
        return Ok(true);
    }
    // Before a search no merge is known, so the finding names the head it
    // failed at and no owner.
    state.find(&json!({
        "kind": "fixed-forward", "test": test, "commit": commit.unwrap_or(head),
        "owner_pr": owner,
        "fixed_at": newest, "at": now(),
    }))?;
    let what = if commit.is_some() { "broke" } else { "fails" };
    state.say(&format!(
        "{test} {what} at {}, but passes at main's newest head {}, so it was fixed forward: \
         nothing is searched for, reverted or held",
        short(commit.unwrap_or(head)),
        short(newest)
    ));
    Ok(true)
}

/// The PR that owns `test` in a round from `base` to `head`: the newest
/// merge in it that moved the test's derivation, or added the test, since
/// that is what the round ran it for. The round's head may have touched
/// nothing of it, as when the net's own quarantine PR was the head.
///
/// A merge that moves the whole system moves every VM test's derivation,
/// so it is passed over when it also moved a test of another family among
/// the round's `tests` (`vm-desktop-*` against `vm-install-*`): on 6
/// November #2048, a change to docs/design.md alone, which the aesthetic
/// skill installs, was named owner of every flaky desktop test. None when
/// no merge since `base` moved it alone. Each merge asks for two
/// derivations, or four, so it's read only for an outcome that names an
/// owner.
fn owner_of(
    world: &dyn World,
    base: &str,
    head: &str,
    test: &str,
    tests: &[String],
) -> Option<u64> {
    let other = tests.iter().find(|other| family(other) != family(test));
    let merges = world.merges(base, head).ok()?;
    merges.iter().rev().find_map(|merge| {
        let before = world.parent(merge).ok()?;
        let everything = other.is_some_and(|other| moved(world, merge, &before, other));
        (moved(world, merge, &before, test) && !everything)
            .then(|| pr_of(&world.subject(merge)))
            .flatten()
    })
}

/// Whether `merge` moved `test`'s derivation from the commit `before` it,
/// or added the test.
fn moved(world: &dyn World, merge: &str, before: &str, test: &str) -> bool {
    match (world.drv(merge, test), world.drv(before, test)) {
        (Ok(now), Ok(was)) => now != was,
        (Ok(_), Err(_)) => true,
        (Err(_), _) => false,
    }
}

/// A VM test's family, the words its name opens with: `vm-desktop` for
/// `vm-desktop-opens-and-sets`, whose tests share a machine.
fn family(test: &str) -> &str {
    test.match_indices('-')
        .nth(1)
        .map_or(test, |(at, _)| &test[..at])
}

/// The PR a merge commit's subject names, as GitHub writes it.
pub fn pr_of(subject: &str) -> Option<u64> {
    let rest = subject.strip_prefix("Merge pull request #")?;
    rest.split_whitespace().next()?.parse().ok()
}

/// Whether `test` builds the same derivation at both commits, so it does
/// there what it does here; not when either fails to evaluate.
fn same_drv(world: &dyn World, one: &str, other: &str, test: &str) -> bool {
    matches!(
        (world.drv(one, test), world.drv(other, test)),
        (Ok(one), Ok(other)) if one == other
    )
}

/// The merges from `base` to `head` that could have broken `test`, said
/// when there are several to search.
fn to_search(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    test: &str,
) -> Result<Vec<String>, String> {
    let all = world.merges(base, head)?;
    let merges = movers::movers(world, &all, test);
    if merges.len() > 1 {
        state.say(&format!(
            "finding which of {} merges broke {test}, the {} that changed it",
            all.len(),
            merges.len()
        ));
    }
    Ok(merges)
}

/// Keeps in the hold how its revert went: the PR and commit that landed,
/// or that it's to be tried again because no builder took its checks.
fn keep_revert(
    state: &State,
    hold: &mut Value,
    reverted: &Result<Merged, String>,
) -> Result<(), String> {
    match reverted {
        Ok(merged) => {
            hold["revert_pr"] = json!(merged.pr);
            hold["revert_commit"] = json!(merged.commit);
            hold["revert_again"] = json!(false);
        }
        Err(why) if tried_again(why) => hold["revert_again"] = json!(true),
        Err(_) => hold["revert_again"] = json!(false),
    }
    state.set_hold(hold)
}

/// What each round begins with: a revert tried again, switches let through
/// a hold the machine already runs, and a hold cleared whose revert is on
/// main.
fn tidy_the_hold(world: &dyn World, state: &State) -> Result<(), String> {
    revert_again(world, state)?;
    running::let_through_once_it_runs(world, state)?;
    clear_if_reverted(world, state)
}

/// Try again a revert whose checks no builder took, as each round begins:
/// on 8 November the lent builders were down when the net blamed #2263,
/// and its hold sat with no revert until one was landed by hand.
fn revert_again(world: &dyn World, state: &State) -> Result<(), String> {
    let Some(mut hold) = state.hold() else {
        return Ok(());
    };
    let (true, Some(commit), Some(test)) = (
        hold["revert_again"] == true,
        hold["bad"].as_str().map(str::to_owned),
        hold["test"].as_str().map(str::to_owned),
    ) else {
        return Ok(());
    };
    let subject = world.subject(&commit);
    let found = Bad {
        commit: commit.clone(),
        pr: pr_of(&subject),
        subject,
        test: test.clone(),
        log: state.log(&commit, &test, 2).display().to_string(),
    };
    let failed_at = hold["failed_at"].as_str().map(str::to_owned);
    let wanted = revert_wanted(world, state, &found, failed_at.as_deref());
    let reverted = world.revert(&found, &wanted);
    keep_revert(state, &mut hold, &reverted)?;
    if dropped(state, &found, &reverted)? {
        return Ok(());
    }
    state.say(&match &reverted {
        Ok(merged) => format!(
            "reverted {} for {test} in #{}, its checks taken this time",
            short(&commit),
            merged.pr
        ),
        Err(why) => format!("couldn't revert {} yet: {why}", short(&commit)),
    });
    Ok(())
}

/// Clear the hold once its revert is on main, fetching main first when the
/// revert isn't on it as last fetched: on 7 November a revert merged at
/// 16:02, and the hold waited 13 minutes for the next round's fetch while
/// the round named flaky tests' owners.
fn clear_if_reverted(world: &dyn World, state: &State) -> Result<(), String> {
    let hold = state.hold();
    if let Some(revert) = hold
        .as_ref()
        .and_then(|hold| hold["revert_commit"].as_str())
        && (world.on_main(revert) || (world.head().is_ok() && world.on_main(revert)))
    {
        state.clear_hold()?;
        state.say(&format!(
            "the revert {} is on main, so the hold is cleared",
            short(revert)
        ));
    }
    Ok(())
}

/// Tell the owner of `head` of any check that no longer evaluates there.
fn find_broken(
    world: &dyn World,
    state: &State,
    changed: &Changed,
    base: &str,
    head: &str,
) -> Result<(), String> {
    if changed.broken.is_empty() {
        return Ok(());
    }
    state.find(&json!({
        "kind": "broken",
        "test": changed.broken.join(" "),
        "commit": head,
        "since": base,
        "owner_pr": pr_of(&world.subject(head)),
        "at": now(),
    }))?;
    Ok(())
}

/// One round, and everything it found settled before it answers, as the
/// tests want. The first error it or a job it handed ended in is its
/// answer.
#[cfg(test)]
fn round(world: &dyn World, state: &State, start: &str) -> Result<Round, String> {
    let settler = Settler::new(world, state);
    let round = std::thread::scope(|scope| {
        settler.start(scope);
        let round = round_beside(world, state, start, &settler);
        settler.close();
        round
    })?;
    settler.error().map_or(Ok(round), Err)
}

/// One round: look at main's head, if there's something new to look at.
/// `start` is the base of the first round, before any head has passed.
/// What takes longer than its tests to settle goes to `settler`, and the
/// round ends with its tests.
pub fn round_beside(
    world: &dyn World,
    state: &State,
    start: &str,
    settler: &Settler,
) -> Result<Round, String> {
    settler.take_up();
    let aged = state.age_out_logs(std::time::SystemTime::now());
    if aged > 0 {
        state.say(&format!("removed {aged} logs of runs over three days old"));
    }
    if let Some(why) = world.held() {
        state.say(&format!("held, starting nothing: {why}"));
        if let Err(error) = state.held(&why) {
            state.say(&error);
        }
        return Ok(Round::Idle);
    }
    let (passed, seen) = state.heads();
    // A round cut short, as every switch that changes the net cuts one,
    // takes up the head it began on, whose runs are recorded. At main's
    // newest head each test would run again, for hours, and on 5 November
    // three switches in a row cut the round before its search for a bad
    // merge began.
    let head = match state.under_way() {
        Some(began) if passed.as_deref() != Some(began.as_str()) => began,
        _ => world.head()?,
    };
    // A settle under way writes the hold itself, so the round leaves its
    // tidying to the next round rather than waiting on it.
    settler
        .write_unless_busy(|| tidy_the_hold(world, state))
        .unwrap_or(Ok(()))?;
    let base = passed.clone().unwrap_or_else(|| start.to_owned());
    if seen.as_deref() == Some(head.as_str()) || base == head {
        return Ok(Round::Idle);
    }
    state.begin(&head)?;
    state.say(&format!("comparing {} with {}", short(&base), short(&head)));
    let changed = world.changed(&base, &head)?;
    let hold = clear_if_fixed_forward(world, state, settler, &changed, &head)?;
    find_broken(world, state, &changed, &base, &head)?;
    // A test held for a revert not yet merged is left until it is; one held
    // with no revert, as when the revert wouldn't apply, runs again, since
    // a merge since may have fixed it.
    let pending = hold
        .as_ref()
        .filter(|hold| hold["revert_commit"].is_string())
        .and_then(|hold| hold["test"].as_str().map(str::to_owned));
    let tests: Vec<String> = changed
        .checks()
        .into_iter()
        .filter(|test| Some(test) != pending.as_ref())
        .collect();
    let quarantined = world.quarantined(&head);
    let on_hold = |test: &str| hold.as_ref().is_some_and(|hold| hold["test"] == test);
    let owner_in = |test: &str| owner_of(world, &base, &head, test, &tests);
    let left_be = |test: &str| {
        quarantined.iter().any(|kept| kept == test)
            || on_hold(test)
            || listed_on_main(world, &head, test)
    };
    let breaking = Breaking::new(world, state, settler, &base, &head, left_be);
    let outcomes = attempt_all(world, state, &head, &tests, &|test, ran| {
        breaking.heard(test, ran);
    });
    let listed = newest_known(world)
        .map(|newest| world.quarantined(&newest))
        .unwrap_or_default();
    let mut failed = false;
    let mut unfinished = false;
    for (test, outcome) in tests.iter().zip(outcomes) {
        let kept_out = quarantined.contains(test) || listed.contains(test);
        match outcome {
            Outcome::Passed => {
                if on_hold(test) {
                    let how = format!("passes again at {}", short(&head));
                    settler.write(|| clear_hold_of(state, test, &how))?;
                }
            }
            Outcome::Flaky if !kept_out && !state.found("flaky", test, &head) => {
                settler.hand(Job::Flaky {
                    head: head.clone(),
                    test: test.clone(),
                    owner: owner_in(test),
                });
            }
            Outcome::Flaky => {}
            Outcome::Failed if kept_out => {
                state.find(&json!({
                    "kind": "failed-in-quarantine",
                    "test": test,
                    "commit": head,
                    "owner_pr": owner_in(test),
                    "log": state.log(&head, test, 2).display().to_string(),
                    "at": now(),
                }))?;
            }
            // Its merge was found as it failed, above.
            Outcome::Failed => failed = true,
            Outcome::Unsent => unfinished = true,
            // A test that stalls twice at the head is held like one that
            // fails there, so it runs again only once its inputs change,
            // but no merge is blamed for it.
            Outcome::Stalled => {
                failed = true;
                stalled(state, &head, test, owner_in(test))?;
            }
        }
    }
    if unfinished && !failed {
        return Ok(Round::Unfinished(head));
    }
    // A head that failed is the next base all the same: the test that
    // failed is held, and its revert, or a fix forward, changes it, so the
    // next head runs it again, while the tests that passed here aren't
    // run again at every head until it's fixed.
    if failed {
        state.set_heads(&head, &head)?;
        return Ok(Round::Failed(head));
    }
    state.set_heads(&head, &head)?;
    state.say(&format!("{} passed every check it changed", short(&head)));
    Ok(Round::Passed(head))
}

/// The indices of `checks` in the order they start: those that took
/// longest at their newest pass first, and before them those never seen
/// to pass, which may be as long. A round takes as long as its last test to end,
/// and started by name, on 14 November the round from 05:18 began
/// vm-guide-installs, 26 minutes long, 46 minutes in, and ended with it.
fn longest_first(checks: &[String], lasted: &std::collections::HashMap<String, u64>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..checks.len()).collect();
    order.sort_by_key(|&index| {
        std::cmp::Reverse(lasted.get(&checks[index]).copied().unwrap_or(u64::MAX))
    });
    order
}

/// How long the quick look rests between looks at main's head.
const QUICK_EVERY: std::time::Duration = std::time::Duration::from_secs(300);

/// Look quickly at main's head, again and again, until the rounds are done.
pub fn quick(world: &dyn World, state: &State, settler: &Settler, done: &AtomicBool) {
    while !done.load(Ordering::Relaxed) {
        if let Err(why) = quick_beside(world, state, settler) {
            state.say(&format!("a quick look failed: {why}"));
        }
        let rested = std::time::Instant::now();
        // Rested in short steps, so the rounds ending isn't kept waiting
        // for the rest of a look's five minutes.
        while rested.elapsed() < QUICK_EVERY && !done.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

/// One quick look: run the checks other than VM tests that main's head
/// moved since the last head they ran at, beside the round, and search for
/// and revert the merge that broke one as the round does. A round keeps
/// one head for its hour or two of VM tests, so on 11 November a merge at
/// 06:33 that broke the-launcher-names-the-parts-it-builds, a check of
/// seconds, waited behind the round begun at 06:01 with nothing running
/// it, while each gate that included it failed on main's defect. The VM
/// tests stay the round's. It gives the head it looked at, or none when
/// there was nothing new, it was held, or a check wasn't taken, so the
/// next look asks again from the same base.
pub fn quick_beside(
    world: &dyn World,
    state: &State,
    settler: &Settler,
) -> Result<Option<String>, String> {
    if world.held().is_some() {
        return Ok(None);
    }
    let head = world.head()?;
    let Some(base) = state.quick().or_else(|| state.heads().0) else {
        return Ok(None);
    };
    if base == head {
        return Ok(None);
    }
    let checks: Vec<String> = world
        .changed(&base, &head)?
        .moved
        .into_iter()
        .filter(|name| !name.starts_with("vm-"))
        .collect();
    let hold = state.hold();
    let quarantined = world.quarantined(&head);
    let left_be = |test: &str| {
        quarantined.iter().any(|kept| kept == test)
            || hold.as_ref().is_some_and(|hold| hold["test"] == test)
            || listed_on_main(world, &head, test)
    };
    let breaking = Breaking::new(world, state, settler, &base, &head, left_be);
    let outcomes = attempt_all(world, state, &head, &checks, &|test, ran| {
        breaking.heard(test, ran);
    });
    if outcomes.contains(&Outcome::Unsent) {
        return Ok(None);
    }
    state.set_quick(&head)?;
    let failed = outcomes
        .iter()
        .filter(|&&ran| ran == Outcome::Failed)
        .count();
    state.say(&format!(
        "a quick look at {} ran the {} checks besides VM tests it moved since {}: {failed} \
         failed",
        short(&head),
        checks.len(),
        short(&base)
    ));
    Ok(Some(head))
}

/// A test that fails for real has its merge found and reverted the moment
/// it does, while the round's other tests still run. On 7 November
/// vm-checks-go-first failed twice at 12:46, but the search for its merge
/// waited 34 minutes behind the round's slower desktop tests, and main
/// stayed broken 113 minutes in all. The search goes to the settler, which
/// runs up to `SEARCHES` side by side and settles what each found one at a
/// time, since each writes the hold.
struct Breaking<'a, 's, F> {
    world: &'a dyn World,
    state: &'a State,
    settler: &'a Settler<'s>,
    base: &'a str,
    head: &'a str,
    /// Tests whose failure reverts nothing: quarantined, or already held.
    left_be: F,
    /// Main's newest head a held test with no revert was last run at.
    tried: Mutex<Option<String>>,
}

impl<'a, 's, F: Fn(&str) -> bool> Breaking<'a, 's, F> {
    fn new(
        world: &'a dyn World,
        state: &'a State,
        settler: &'a Settler<'s>,
        base: &'a str,
        head: &'a str,
        left_be: F,
    ) -> Self {
        Self {
            world,
            state,
            settler,
            base,
            head,
            left_be,
            tried: Mutex::new(None),
        }
    }

    /// Each test's end also clears a hold whose revert is on main, since a
    /// round can run long past it: on 7 November the revert merged at
    /// 13:48, and eight merges waited off the machine behind the hold while
    /// the round it was found in ran on.
    ///
    /// A failed test has its merge searched for at once, beside the round,
    /// unless one for it is under way from a round before.
    fn heard(&self, test: &str, outcome: Outcome) {
        if let Some(Err(why)) = self
            .settler
            .write_unless_busy(|| clear_if_reverted(self.world, self.state))
        {
            self.state.say(&why);
        }
        if outcome == Outcome::Failed && !(self.left_be)(test) {
            self.settler.hand(Job::Search {
                base: self.base.to_owned(),
                head: self.head.to_owned(),
                test: test.to_owned(),
            });
        }
        self.fixed_forward_since();
    }

    /// Each test's end also runs a test held with no revert at main's newest
    /// head, once a head, and clears the hold when it passes there. Only a
    /// round's start asked before: on 8 November #2381 fixed the held
    /// vm-desktop-launcher-goes-places forward at 12:10, while the round
    /// begun at 09:30 ran on, and every merge waited off the machine behind
    /// the hold. The run goes to the settler: run here, it held the thread
    /// that ended a test for a VM test's minutes before its next test.
    fn fixed_forward_since(&self) {
        let Ok(mut tried) = self.tried.try_lock() else {
            return;
        };
        let Some(test) = self
            .state
            .hold()
            .filter(|hold| !hold["revert_commit"].is_string())
            .and_then(|hold| hold["test"].as_str().map(str::to_owned))
        else {
            return;
        };
        let Ok(newest) = self.world.head() else {
            return;
        };
        if newest == self.head || tried.as_deref() == Some(newest.as_str()) {
            return;
        }
        *tried = Some(newest.clone());
        self.settler.hand(Job::Forward {
            head: self.head.to_owned(),
            newest,
            test,
        });
    }
}

/// Clear the hold if it still names `test`, which `passes` says how. A
/// search beside the round may have held another test since it began.
fn clear_hold_of(state: &State, test: &str, passes: &str) -> Result<(), String> {
    if !state.hold().is_some_and(|hold| hold["test"] == test) {
        return Ok(());
    }
    state.clear_hold()?;
    state.say(&format!("{test} {passes}, so the hold is cleared"));
    Ok(())
}

/// The value behind `mutex`, even one a panicking thread left, since what
/// each guards is whole between writes.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Run a test held with no revert at the new head before anything else, and
/// clear the hold the moment it passes. Every merge waits off the machine
/// while main is held, and on 6 November a fix forward merged two minutes
/// after a hold sat behind 18 minutes of older failures' checks and a whole
/// round in name order. The loop after reuses this run, as it's recorded.
/// Answers the hold as it stands afterwards.
fn clear_if_fixed_forward(
    world: &dyn World,
    state: &State,
    settler: &Settler,
    changed: &Changed,
    head: &str,
) -> Result<Option<Value>, String> {
    let hold = state.hold();
    let Some(test) = hold
        .as_ref()
        .filter(|hold| !hold["revert_commit"].is_string())
        .and_then(|hold| hold["test"].as_str().map(str::to_owned))
    else {
        return Ok(hold);
    };
    if !changed.checks().contains(&test) {
        return Ok(hold);
    }
    if !matches!(attempt(world, state, head, &test), Outcome::Passed) {
        return Ok(hold);
    }
    let passes = format!("passes again at {}", short(head));
    settler.write(|| clear_hold_of(state, &test, &passes))?;
    Ok(state.hold())
}

/// Whether main's newest head, other than the round's `head`, lists `test`
/// as quarantined. A round reads the list at its own head, but runs for
/// hours: on 9 November the round at f18ef97a began at 02:35, its owner
/// listed vm-desktop-launcher-moves-windows at 04:13, and the round went on
/// evaluating the test at 139 merges to search for one it would revert.
fn listed_on_main(world: &dyn World, head: &str, test: &str) -> bool {
    newest_known(world).is_some_and(|newest| {
        newest != head && world.quarantined(&newest).iter().any(|kept| kept == test)
    })
}

/// Main's newest head, or where it can't be fetched, its head as last
/// fetched. On 9 November the net came up five seconds before the
/// network, the round it took up heard vm-desktop-captures' recorded
/// failures at once, and a failed fetch read as main listing nothing, so a
/// search of seven steps went after a test main listed.
fn newest_known(world: &dyn World) -> Option<String> {
    world.head().ok().or_else(|| world.fetched())
}

/// List `test`, flaky at `head`, where main's newest head runs it as
/// `head` did, or where it is flaky there too. On 14 November a round
/// fifty minutes behind main listed vm-reboot-cannot-hang for a broken
/// pipe at a merge from 05:18, though #4319 had mended that very failure
/// on main at 05:54, and the list cost a merge, a switch and the subtest's
/// coverage. So where main has moved what the test runs, it is asked
/// there first: passing both runs, it was mended and isn't listed;
/// failing both, it is a break the next round searches for, since that
/// round runs the test it moved. Only where no builder answers is it
/// listed on what `head` said.
fn list_flaky_unless_mended(
    world: &dyn World,
    state: &State,
    head: &str,
    test: &str,
    owner: Option<u64>,
) -> Result<(), String> {
    let Some(newest) =
        newest_known(world).filter(|newest| newest != head && !same_drv(world, newest, head, test))
    else {
        return list_flaky(world, state, head, test, owner);
    };
    let left = match attempt_together(world, state, &newest, test) {
        Outcome::Flaky => {
            if state.found("flaky", test, &newest)
                || world.quarantined(&newest).contains(&test.to_owned())
            {
                return Ok(());
            }
            return list_flaky(world, state, &newest, test, owner);
        }
        Outcome::Unsent => return list_flaky(world, state, head, test, owner),
        Outcome::Passed => "passes both runs there, so it was mended",
        Outcome::Failed | Outcome::Stalled => {
            "doesn't pass there, so the next round searches for what broke it"
        }
    };
    state.say(&format!(
        "{test} failed, then passed, at {}, but main's newest head {} runs it otherwise and \
         {left}: it isn't listed",
        short(head),
        short(&newest)
    ));
    Ok(())
}

/// List `test`, which failed and then passed at `head`, as quarantined
/// with its owner, and tell the owner.
fn list_flaky(
    world: &dyn World,
    state: &State,
    head: &str,
    test: &str,
    owner: Option<u64>,
) -> Result<(), String> {
    let reason = format!("failed, then passed, at {}", short(head));
    let failing = failing_at(state, head, test);
    let made = (!listed_by_subtests_on_main(world, test, &failing))
        .then(|| world.quarantine(test, &failing, owner, head, &reason));
    // On 10 November a refused merge was kept only in the finding, and the
    // list it would have landed sat unmerged for an hour and a half.
    match &made {
        Some(Ok(landed)) => {
            state.say(&format!("listed {test} as quarantined in #{}", landed.pr));
        }
        Some(Err(why)) => state.say(&format!("listing {test} as quarantined failed: {why}")),
        None => state.say(&format!(
            "{test} failed, then passed, at {}, and main lists it by subtests already, so \
             its list stays as it is",
            short(head)
        )),
    }
    state.find(&json!({
        "kind": "flaky",
        "test": test,
        "commit": head,
        "owner_pr": owner,
        "log": state.log(head, test, 1).display().to_string(),
        "quarantine_pr": made.as_ref().and_then(|made| made.as_ref().ok().map(|it| it.pr)),
        "quarantine_error": made.as_ref().and_then(|made| made.as_ref().err()),
        "at": now(),
    }))?;
    Ok(())
}

/// Whether main's newest head lists `test` by subtests that hold what
/// failed in it: every subtest named in `failing`, or any, where the
/// failure named none, as a run that timed out names none. A list never
/// lists whole a test it lists by subtests (quarantine.rs), so listing
/// one again changes nothing: at 20:11 on 13 November git refused that
/// empty change for vm-desktop-launcher-finds-sessions, and the net said
/// its listing had failed.
fn listed_by_subtests_on_main(world: &dyn World, test: &str, failing: &BTreeSet<String>) -> bool {
    newest_known(world).is_some_and(|newest| {
        let listed = world.listed_subtests(&newest, test);
        !listed.is_empty() && failing.is_subset(&listed)
    })
}

/// Tell `test`'s owner that it stalled twice at `head`.
fn stalled(state: &State, head: &str, test: &str, owner: Option<u64>) -> Result<(), String> {
    state.find(&json!({
        "kind": "stalled",
        "test": test,
        "commit": head,
        "owner_pr": owner,
        "log": state.log(head, test, 1).display().to_string(),
        "at": now(),
    }))?;
    Ok(())
}

/// Whether `test` failed twice at the round's `base`. It failed then
/// before every merge since, so no search over them can find what broke
/// it, and `before` clears the merge one finds only after all its runs. On
/// 9 November vm-desktop-looks failed twice at the base while it was
/// listed, came off the list still failing, and its search took four runs
/// of 25 minutes each. A merge that broke it again after another fixed it
/// in the same round goes unblamed, which costs less than such a search.
fn failed_at_base(state: &State, base: &str, test: &str) -> bool {
    state
        .runs(base, test)
        .iter()
        .filter(|&&ran| ran == Ran::Failed)
        .count()
        >= FAILURES
}

/// List `test`, which failed twice at the round's `base` and fails at its
/// `head`, as quarantined, owned by the PR that last took off the list what
/// fails in it now, unless main lists it now. No merge since is to blame, so nothing else
/// names an owner: on 10 November vm-desktop-launcher-reads-mail and
/// keeps-up came off the list still failing, by #2603 and #2602, and failed
/// at every head for hours with nobody told.
fn list_still_red(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    test: &str,
    log: &str,
) -> Result<(), String> {
    let failing = failing_at(state, head, test);
    let owner = world.unlisted_by(test, &failing, head);
    let reason = format!(
        "failed twice at {}, and at {} after it",
        short(base),
        short(head)
    );
    let by_subtests = listed_by_subtests_on_main(world, test, &failing);
    let listed = (!by_subtests && !listed_on_main(world, head, test))
        .then(|| world.quarantine(test, &failing, owner, head, &reason));
    state.find(&json!({
        "kind": "failed-before", "test": test, "commit": head, "since": base,
        "owner_pr": owner, "log": log,
        "quarantine_pr": listed.as_ref().and_then(|made| made.as_ref().ok().map(|it| it.pr)),
        "quarantine_error": listed.as_ref().and_then(|made| made.as_ref().err()),
        "at": now(),
    }))?;
    let what = owner.map_or_else(|| "no PR".to_owned(), |pr| format!("#{pr}"));
    let after = match &listed {
        Some(Ok(made)) => format!("listed as quarantined in #{}, owned by {what}", made.pr),
        Some(Err(why)) => format!("listing it as quarantined failed: {why}"),
        None if by_subtests => {
            "main lists it by subtests already, so its list stays as it is".to_owned()
        }
        None => "main lists it as quarantined now".to_owned(),
    };
    state.say(&format!(
        "{test} fails at {}, and failed twice at {} before it, so no merge since broke it: \
         nothing is reverted or held, and it's {after}",
        short(head),
        short(base)
    ));
    Ok(())
}

/// What a search found for a test that failed for real, for the round to
/// settle one at a time, since each writes the hold.
enum Found {
    /// The merge that broke it.
    Merge(Bad),
    /// None: it failed twice at the round's base, so before every merge
    /// since; `log` is the failure at the head.
    Before { test: String, log: String },
    /// A run the search needed stalled or no builder took, so it couldn't
    /// tell which merge broke `test`; `log` is the failure at the head.
    Unsettled { test: String, log: String },
}

/// Find the merge since `base` that broke `test` at `head`, writing only
/// findings that hold nothing, so searches for other tests can run beside
/// it. None when nothing is to be held, reverted or listed: it was fixed
/// forward, or no merge is to blame.
fn search(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    test: &str,
    splits: &dyn Fn() -> usize,
) -> Result<Option<Found>, String> {
    // A round taken up again after a restart hears its failures again, and
    // one already found fixed forward was searched a second time on
    // 8 November, where runs stalled and held main.
    if state.found("fixed-forward", test, head) {
        return Ok(None);
    }
    if failed_at_base(state, base, test) {
        return Ok(Some(Found::Before {
            test: test.to_owned(),
            log: state.log(head, test, 2).display().to_string(),
        }));
    }
    let merges = to_search(world, state, base, head, test)?;
    // Each step runs only the subtests that failed in both runs at the head
    // (subtests.rs), and what it finds is checked at its parent the same way.
    // Those main lists as quarantined are left out, since they excuse a run.
    let listed = world
        .fetched()
        .map(|newest| world.listed_subtests(&newest, test))
        .unwrap_or_default();
    let at_head: Vec<_> = (1..=2)
        .map(|run| {
            &subtests::failed_in(crate::world::lines_of(&state.log(head, test, run))) - &listed
        })
        .collect();
    let narrowed = subtests::narrowed(test, &at_head);
    if narrowed != test {
        state.say(&format!(
            "searching {test} on the subtests that failed in both its runs at {}: {}",
            short(head),
            subtests::subtests_of(&narrowed).join(", ")
        ));
    }
    // Whether main's newest head fixed it is asked beside the search's first
    // step, and the search waits for the answer before its second. On
    // 7 November asking first took 16 minutes, two VM runs one after the
    // other, before the search began; a fix forward now costs at most one
    // step's runs.
    let (told, answered) = std::sync::mpsc::channel();
    let answer: Mutex<Option<Result<bool, String>>> = Mutex::new(None);
    let fixed = || {
        answer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(|| {
                answered.recv().unwrap_or_else(|_| {
                    Err(format!("asking whether {test} was fixed forward failed"))
                })
            })
            .clone()
    };
    let bad = std::thread::scope(|scope| {
        scope.spawn(|| told.send(fixed_forward(world, state, head, test, None)));
        if merges.len() == 1 {
            merges.first().cloned()
        } else {
            bisect(
                world,
                state,
                &merges,
                &narrowed,
                &|| fixed() == Ok(false),
                splits,
            )
        }
    });
    if fixed()? {
        return Ok(None);
    }
    let bad = bad.filter(|commit| !never_built(state, commit, &narrowed));
    let failing = bad.clone().unwrap_or_else(|| head.to_owned());
    let log = state.log(&failing, &narrowed, 2);
    let log = if log.exists() {
        log
    } else {
        state.log(&failing, test, 2)
    };
    let log = log.display().to_string();
    let bad = match bad {
        Some(commit) => match before(world, state, &commit, &narrowed)? {
            Before::Passed => Some(commit),
            Before::Unsent => None,
            not_broken_here => {
                blames_none(world, state, test, &commit, &log, not_broken_here)?;
                return Ok(None);
            }
        },
        None => None,
    };
    // A merge already found fixed forward stays settled when main's newest
    // head can't say now. On 14 November a net moving builds left such a
    // search kept, and the next took it up, found no builder for the newest
    // head, and quarantined vm-reboot-cannot-hang in #4312, seven minutes
    // after it had passed there. Whether main's newest head has fixed it
    // since is asked once, by `settle`, just before its revert: asked here
    // too, on 14 November each answer came at a head merged while the one
    // before was run, and #4311's revert waited 29 minutes on two in a row.
    if let Some(commit) = &bad
        && state.found("fixed-forward", test, commit)
    {
        return Ok(None);
    }
    let Some(commit) = bad else {
        return Ok(Some(Found::Unsettled {
            test: test.to_owned(),
            log,
        }));
    };
    let subject = world.subject(&commit);
    Ok(Some(Found::Merge(Bad {
        commit,
        pr: pr_of(&subject),
        subject,
        test: test.to_owned(),
        log,
    })))
}

/// Whether every failed run of `subtests` at `commit` stopped at a build
/// of something it needs, before the test ran. A search narrowed to the
/// subtests that failed at its head looks for what broke those, and a
/// merge where nothing ran says nothing of them. On 12 November
/// vm-desktop-launcher-moves-windows' kitty trial counted too many
/// letters at the head, and the four merges after its last pass didn't
/// build, #3613 having broken slack-people, so the search blamed #3613,
/// whose break was reverted already, rather than say it couldn't tell.
fn never_built(state: &State, commit: &str, subtests: &str) -> bool {
    if !subtests.contains("::") {
        return false;
    }
    let runs: Vec<Vec<String>> = (1..=FAILURES)
        .map(|run| {
            let run = u32::try_from(run).unwrap_or(0);
            crate::world::lines_of(&state.log(commit, subtests, run)).collect()
        })
        .collect();
    let built_none = runs.iter().all(|lines| {
        lines.iter().any(|line| line.contains("Cannot build"))
            && !lines.iter().any(|line| line.starts_with("vm-test-run"))
    });
    if built_none {
        state.say(&format!(
            "{subtests} never ran at {}, where something it needs didn't build, so that \
             merge isn't blamed for it",
            short(commit)
        ));
    }
    built_none
}

/// Hold switching on what a search found and revert it, list a test that
/// failed at the base too, or keep a test whose search couldn't tell to
/// run again at the next head. A test main's newest head passes is none of
/// these: what a search found is settled one at a time, so it can wait
/// behind another's revert while main moves on. On 10 November
/// vm-desktop-launcher-moves-windows' search ended at 02:47 with the test
/// still failing at main's head of then, waited 8 minutes for
/// vm-guide-recovers' revert to land, and held switches at 02:55 on that
/// answer, though #2770 had fixed it at 02:44. So it is asked once more
/// here, at no cost when main hasn't moved, since the runs at a commit are
/// kept.
fn settle(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    found: Found,
) -> Result<(), String> {
    // A search ends with no merge only when a run it needed stalled or no
    // builder took it, so the test is kept to run again at the next head
    // without holding switches.
    let found = match found {
        Found::Merge(found) => found,
        Found::Before { test, log } => {
            if fixed_forward(world, state, head, &test, None)? {
                return Ok(());
            }
            return list_still_red(world, state, base, head, &test, &log);
        }
        Found::Unsettled { test, log } => {
            state.set_hold(&json!({
                "test": test, "bad": Value::Null, "since": base, "head": head, "at": now(),
                "unsettled": true,
            }))?;
            state.say(&format!(
                "{test} fails at {}, but its search couldn't tell which merge broke it, since a \
                 run stalled, no builder took it, or it didn't build where the search needed \
                 it: switches go on, and it runs again at the next head",
                short(head)
            ));
            state.find(&json!({
                "kind": "failed", "test": test, "commit": head, "since": base,
                "owner_pr": Value::Null, "log": log,
                "revert_error": "couldn't tell which merge broke it", "at": now(),
            }))?;
            return Ok(());
        }
    };
    let (commit, test, log) = (&found.commit, &found.test, &found.log);
    // A search can take hours, and the owner may have listed the test
    // meanwhile, so the list as main has it now is read again.
    if listed_on_main(world, head, test) {
        state.find(&json!({
            "kind": "failed-in-quarantine", "test": test, "commit": commit,
            "owner_pr": found.pr, "log": log, "at": now(),
        }))?;
        state.say(&format!(
            "{test} broke at {}, but main lists it as quarantined now: nothing is reverted or held",
            short(commit)
        ));
        return Ok(());
    }
    if fixed_forward(world, state, head, test, Some(commit))? {
        return Ok(());
    }
    revert_and_tell(world, state, &found, head)
}

/// Hold switching on `found`, revert it, and tell its owner, listing its
/// test as quarantined when the revert won't apply.
fn revert_and_tell(
    world: &dyn World,
    state: &State,
    found: &Bad,
    head: &str,
) -> Result<(), String> {
    let (commit, test, log) = (&found.commit, &found.test, &found.log);
    let already_runs = world.runs(commit);
    let mut hold = json!({
        "test": test, "bad": commit, "bad_pr": found.pr, "revert_pr": Value::Null,
        "revert_commit": Value::Null, "failed_at": head, "at": now(),
        "already_runs": already_runs,
    });
    state.set_hold(&hold)?;
    if already_runs {
        state.say(&running::lets_through(commit, test));
    }
    let reverted = world.revert(found, &revert_wanted(world, state, found, Some(head)));
    keep_revert(state, &mut hold, &reverted)?;
    if dropped(state, found, &reverted)? {
        return Ok(());
    }
    let listed: Option<Result<Merged, String>> = match &reverted {
        Err(why) if !tried_again(why) => Some(list_broken(world, state, found, head, why)),
        _ => None,
    };
    state.find(&json!({
        "kind": "failed", "test": test, "commit": commit, "owner_pr": found.pr, "log": log,
        "revert_pr": reverted.as_ref().ok().map(|merged| merged.pr),
        "revert_error": reverted.as_ref().err(),
        "quarantine_pr": listed.as_ref().and_then(|made| made.as_ref().ok().map(|it| it.pr)),
        "quarantine_error": listed.as_ref().and_then(|made| made.as_ref().err()),
        "at": now(),
    }))?;
    match reverted {
        Ok(landed) => {
            state.say(&format!(
                "reverted {} for {test} in #{}",
                short(commit),
                landed.pr
            ));
            clear_if_reverted(world, state)
        }
        Err(why) => {
            state.say(&format!(
                "couldn't revert {} for {test}: {why}",
                short(commit)
            ));
            Ok(())
        }
    }
}

/// Whether a revert is needed, asked with main's newest head and the
/// revert merged into it: its test runs at both side by side, and the
/// revert lands only if the test fails without it and passes with it.
/// Passing without it, it was fixed forward, and the revert is dropped;
/// failing with it, the revert mends nothing, and lands nothing. Neither
/// is judged from runs at another commit: on 9 November the net's #2621
/// reverted #2597 at 12:47, its checks run on the main of 12:37, a minute
/// after #2617 had fixed the test forward. On 10 November #2783 reverted
/// #2705 at 03:13 for a moves-windows failure seen only at 02:32's main.
/// The main it was cut from carried #2770, which had moved the test to
/// what #2705 draws, but the test never ran there or with the revert, so
/// the revert broke the test it was for. Only a newest head building the
/// test as `failed_at` does, where it ran and failed, isn't run again.
fn revert_wanted<'a>(
    world: &'a dyn World,
    state: &'a State,
    found: &'a Bad,
    failed_at: Option<&'a str>,
) -> impl Fn(&str, &str) -> Result<bool, String> + 'a {
    move |newest: &str, reverted: &str| {
        let test = &found.test;
        let (without, with) = std::thread::scope(|scope| {
            let with = scope.spawn(|| attempt_together(world, state, reverted, test));
            let without = match failed_at {
                Some(failed) if same_drv(world, newest, failed, test) => Outcome::Failed,
                _ => attempt_together(world, state, newest, test),
            };
            (without, with.join().unwrap_or(Outcome::Unsent))
        });
        if without == Outcome::Unsent || with == Outcome::Unsent {
            return Err(format!(
                "{UNCHECKED}: no builder ran {test} at main's newest head {} with and \
                 without the revert",
                short(newest)
            ));
        }
        if told_fixed(
            world,
            state,
            newest,
            newest,
            test,
            Some(&found.commit),
            without,
        )? {
            return Ok(false);
        }
        if with == Outcome::Passed {
            // The round reads a pause only as it starts and before each
            // run, and a revert's checks take minutes: on 10 November the
            // net was paused at 07:45 to let #2832 fix #2809 forward, and
            // merged #2809's revert at 07:47 all the same.
            if let Some(why) = world.held() {
                state.say(&format!(
                    "the revert of {} is wanted, but it lands nothing while held: {why}",
                    short(&found.commit)
                ));
                return Err(format!("{PAUSED}: {why}"));
            }
            return Ok(true);
        }
        state.say(&format!(
            "{test} fails at main's newest head {} with the revert of {} too, so the revert \
             mends nothing and lands nothing",
            short(newest),
            short(&found.commit)
        ));
        Err(format!(
            "{MENDS_NOTHING}: {test} fails with the revert at main's newest head {} too",
            short(newest)
        ))
    }
}

/// Whether a revert was dropped as needless, clearing its hold, since
/// `fixed_forward` has said why and kept the finding.
fn dropped(state: &State, found: &Bad, reverted: &Result<Merged, String>) -> Result<bool, String> {
    if !matches!(reverted, Err(why) if why.starts_with(DROPPED)) {
        return Ok(false);
    }
    state.clear_hold()?;
    state.say(&format!(
        "the revert of {} for {} is dropped, so the hold is cleared",
        short(&found.commit),
        found.test
    ));
    Ok(true)
}

/// A merge pinned for breaking a test whose revert won't apply, or mends
/// nothing, has its test listed as quarantined, with the merge's PR as its
/// owner, and the hold cleared, so the net goes on to main's head and
/// switches go on while the owner fixes it forward. On 9 November the
/// revert of #2363 conflicted with the 138 merges since, and main stayed
/// held until a fix forward landed by hand 23 minutes later; the owner is
/// told in the finding. The reason says which, since `why` is the revert's
/// failure: on 11 November vm-desktop-looks failed with #3064 reverted too,
/// as a second merge broke another of its subtests, and a list saying the
/// revert won't apply would have sent its owner after a conflict.
fn list_broken(
    world: &dyn World,
    state: &State,
    found: &Bad,
    head: &str,
    why: &str,
) -> Result<Merged, String> {
    let what = found
        .pr
        .map_or_else(|| short(&found.commit).to_owned(), |pr| format!("#{pr}"));
    let reason = if why.starts_with(MENDS_NOTHING) {
        format!(
            "has been broken since {what}, but fails with it reverted too, so a later merge \
             breaks it as well; whoever finds that merge fixes it forward and takes it off \
             this list"
        )
    } else {
        format!(
            "has been broken since {what}, whose revert won't apply; its owner fixes it \
             forward and takes it off this list"
        )
    };
    let subtests = failing_at(state, head, &found.test);
    let made = world.quarantine(&found.test, &subtests, found.pr, head, &reason)?;
    state.clear_hold()?;
    state.say(&format!(
        "listed {} as quarantined in #{}, owned by {what}, so the hold is cleared",
        found.test, made.pr
    ));
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::Condvar;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// The exit the stand-in gives for a run stopped before it ended.
    const STALL: i32 = -1;

    /// A stand-in world: each (commit, check) gives the exits listed, in
    /// turn, the last again once they run out; checks named in `missing`
    /// don't exist at a commit.
    #[derive(Default)]
    struct Stand {
        head: String,
        merges: Vec<String>,
        changed: Changed,
        exits: Mutex<HashMap<(String, String), Vec<i32>>>,
        built: Mutex<Vec<(String, String)>>,
        missing: Vec<(String, String)>,
        /// Merges that change no check since the commit before them.
        same: Vec<String>,
        /// Merges that leave the one check named with them unchanged.
        still: Vec<(String, String)>,
        quarantined: Vec<String>,
        /// Tests main's newest head lists as quarantined besides those.
        listed_since: Vec<String>,
        /// What every fetch of main fails with, as before the network is
        /// up, so only its head as last fetched is known.
        fetch_fails: Option<String>,
        /// Whether every revert conflicts with main.
        reverts_conflict: bool,
        reverted: Mutex<Vec<String>>,
        /// Checks whose runs end only once a merge is reverted, or after a
        /// while without one, when they're put in `waited_out`.
        after_revert: Vec<String>,
        revert_seen: Condvar,
        /// Whether a run at main's newest head ends only once the search
        /// has run one at an older merge, or after a while without, when
        /// it's put in `waited_out`.
        newest_waits_for_search: bool,
        /// Commits whose each run waits for a second run there to start,
        /// or after a while without, when it's put in `waited_out`.
        pairs: Vec<String>,
        /// How long each run takes, as a VM test takes minutes.
        runs_for: Duration,
        /// Checks whose each run at main's newest head waits for every one
        /// of them to start there, or after a while without, when it's put
        /// in `waited_out`.
        together_at_newest: Vec<String>,
        /// Checks whose each run at a merge before the round's last waits
        /// for every one of them to have run at one, as their searches do
        /// side by side, or after a while without, when it's put in
        /// `waited_out`.
        searched_together: Vec<String>,
        build_seen: Condvar,
        waited_out: Mutex<Vec<String>>,
        reaches_main_after: usize,
        lands_on_fetch: bool,
        fetched_since_revert: Mutex<bool>,
        /// A commit main's newest head builds every check as.
        newest_builds_as: Option<String>,
        /// Commits whose each run goes on until it's dropped, or after a
        /// while without, when the commit is put in `waited_out`.
        slow: Vec<String>,
        dropped: Mutex<BTreeSet<(String, String)>>,
        put_in_quarantine: Mutex<Vec<String>>,
        /// The subtests each test put in quarantine was listed by.
        quarantine_subtests: Mutex<Vec<BTreeSet<String>>>,
        /// Tests listed by their subtests, at every commit.
        listed_by_subtest: Vec<(String, Vec<String>)>,
        /// The reason each test put in quarantine was listed for.
        quarantine_reasons: Mutex<Vec<String>>,
        /// The owner each test put in quarantine was listed with.
        quarantine_owners: Mutex<Vec<Option<u64>>>,
        /// The PR that last took a test off the list.
        unlisted_by: Option<u64>,
        /// The failing subtests each owner was asked for by.
        unlisted_for: Mutex<Vec<BTreeSet<String>>>,
        held: Option<String>,
        /// A pause that comes while a revert's checks run.
        paused_while_reverting: Option<String>,
        paused: Mutex<Option<String>>,
        /// How many reverts to come find no builder for their checks.
        unchecked_reverts: Mutex<u32>,
        /// How many reverts to come GitHub refuses, main having moved.
        moved_under_reverts: Mutex<u32>,
        /// The head main moves to while a revert's checks run.
        moves_while_reverting: Option<String>,
        moved: Mutex<Option<String>>,
        /// The heads main's newest answers in turn, the last kept, as main
        /// moves on while each answer is run.
        newest: Mutex<Vec<String>>,
        /// The merges the machine runs already.
        running: Vec<String>,
        /// The subtest pytest names in each failed run's log.
        fails_in: Option<String>,
        /// Commits where a failed run's log says only that something it
        /// needs didn't build, so the test never ran.
        unbuilt: Vec<String>,
    }

    impl Stand {
        fn exits(self, commit: &str, check: &str, exits: &[i32]) -> Self {
            self.exits
                .lock()
                .unwrap()
                .insert((commit.to_owned(), check.to_owned()), exits.to_vec());
            self
        }
        fn built(&self) -> Vec<(String, String)> {
            self.built.lock().unwrap().clone()
        }
        /// Waits while `waiting` says so of the runs begun, at most ten
        /// seconds, putting `what` in `waited_out` when the time runs out.
        fn wait_while(&self, waiting: impl FnMut(&mut Vec<(String, String)>) -> bool, what: &str) {
            let built = self.built.lock().unwrap();
            let (built, waited) = self
                .build_seen
                .wait_timeout_while(built, Duration::from_secs(10), waiting)
                .unwrap();
            drop(built);
            if waited.timed_out() {
                self.waited_out.lock().unwrap().push(what.to_owned());
            }
        }
    }

    impl World for Stand {
        fn held(&self) -> Option<String> {
            self.held
                .clone()
                .or_else(|| self.paused.lock().unwrap().clone())
        }
        fn head(&self) -> Result<String, String> {
            if let Some(error) = &self.fetch_fails {
                return Err(error.clone());
            }
            *self.fetched_since_revert.lock().unwrap() = true;
            let mut newest = self.newest.lock().unwrap();
            if newest.len() > 1 {
                return Ok(newest.remove(0));
            }
            if let Some(last) = newest.first() {
                return Ok(last.clone());
            }
            drop(newest);
            Ok(self
                .moved
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| self.head.clone()))
        }
        fn fetched(&self) -> Option<String> {
            Some(self.head.clone())
        }
        fn merges(&self, _: &str, _: &str) -> Result<Vec<String>, String> {
            Ok(self.merges.clone())
        }
        fn subject(&self, commit: &str) -> String {
            format!("Merge pull request #{} from someone/branch", commit.len())
        }
        /// The merge before in `merges`, or the start every test round
        /// here begins at, `s`, before the first.
        fn parent(&self, commit: &str) -> Result<String, String> {
            let at = self.merges.iter().position(|merge| merge == commit);
            Ok(at
                .and_then(|at| at.checked_sub(1))
                .map_or_else(|| "s".to_owned(), |before| self.merges[before].clone()))
        }
        fn changed(&self, _: &str, _: &str) -> Result<Changed, String> {
            Ok(self.changed.clone())
        }
        fn has_check(&self, commit: &str, check: &str) -> Result<bool, String> {
            Ok(!self
                .missing
                .contains(&(commit.to_owned(), check.to_owned())))
        }
        /// A merge in `same`, or in `still` with this check, builds what
        /// the merge before it built.
        fn drv(&self, commit: &str, check: &str) -> Result<String, String> {
            if let Some(like) = &self.newest_builds_as
                && commit == self.head
            {
                return self.drv(like, check);
            }
            let still = (commit.to_owned(), check.to_owned());
            if self.same.iter().any(|same| same == commit) || self.still.contains(&still) {
                return self.drv(&self.parent(commit)?, check);
            }
            Ok(format!("{commit}-{check}"))
        }
        fn build(&self, commit: &str, check: &str, log: &Path) -> Option<i32> {
            self.built
                .lock()
                .unwrap()
                .push((commit.to_owned(), check.to_owned()));
            self.build_seen.notify_all();
            // Long enough for a second asker to come while a run goes.
            std::thread::sleep(self.runs_for);
            if self.slow.iter().any(|slow| slow == commit) {
                let began = std::time::Instant::now();
                while !self.dropped(commit, check) {
                    if began.elapsed() > Duration::from_secs(10) {
                        self.waited_out.lock().unwrap().push(commit.to_owned());
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                if self.dropped(commit, check) {
                    return None;
                }
            }
            if self.newest_waits_for_search && commit == self.head {
                let searched = |built: &mut Vec<(String, String)>| {
                    !built
                        .iter()
                        .any(|(at, _)| self.merges[..self.merges.len() - 1].contains(at))
                };
                self.wait_while(searched, check);
            }
            if commit == self.head && self.together_at_newest.iter().any(|one| one == check) {
                let apart = |built: &mut Vec<(String, String)>| {
                    !self
                        .together_at_newest
                        .iter()
                        .all(|one| built.iter().any(|(at, ran)| at == commit && ran == one))
                };
                self.wait_while(apart, check);
            }
            let older = |at: &str| self.merges[..self.merges.len() - 1].iter().any(|m| m == at);
            if self.searched_together.iter().any(|one| one == check) && older(commit) {
                let apart = |built: &mut Vec<(String, String)>| {
                    !self
                        .searched_together
                        .iter()
                        .all(|one| built.iter().any(|(at, ran)| older(at) && ran == one))
                };
                self.wait_while(apart, check);
            }
            if self.pairs.iter().any(|paired| paired == commit) {
                let alone = |built: &mut Vec<(String, String)>| {
                    built.iter().filter(|(at, _)| at == commit).count() < 2
                };
                self.wait_while(alone, commit);
            }
            if self.after_revert.iter().any(|waits| waits == check) {
                let reverted = self.reverted.lock().unwrap();
                let (reverted, waited) = self
                    .revert_seen
                    .wait_timeout_while(reverted, Duration::from_secs(10), |reverted| {
                        reverted.is_empty()
                    })
                    .unwrap();
                drop(reverted);
                if waited.timed_out() {
                    self.waited_out.lock().unwrap().push(check.to_owned());
                }
            }
            let mut exits = self.exits.lock().unwrap();
            let left = exits
                .entry((commit.to_owned(), check.to_owned()))
                .or_insert_with(|| vec![0]);
            let exit = if left.len() > 1 {
                left.remove(0)
            } else {
                left[0]
            };
            if exit != 0 && self.unbuilt.iter().any(|at| at == commit) {
                let _ = fs::write(log, "error: Cannot build '/nix/store/x-needed.drv'.\n");
            } else if let Some(subtest) = self.fails_in.as_ref().filter(|_| exit != 0) {
                let line = format!("vm-test-run> FAILED vm-pytest-ab/t.py::{subtest} - Error\n");
                let _ = fs::write(log, line);
            }
            (exit != STALL).then_some(exit)
        }
        fn quarantined(&self, commit: &str) -> Vec<String> {
            let mut listed = self.quarantined.clone();
            if commit == self.head {
                listed.extend(self.listed_since.iter().cloned());
            }
            listed
        }
        fn listed_subtests(&self, _: &str, test: &str) -> BTreeSet<String> {
            self.listed_by_subtest
                .iter()
                .filter(|(listed, _)| listed == test)
                .flat_map(|(_, subtests)| subtests.iter().cloned())
                .collect()
        }
        fn revert(
            &self,
            bad: &Bad,
            still: &dyn Fn(&str, &str) -> Result<bool, String>,
        ) -> Result<Merged, String> {
            if self.reverts_conflict {
                return Err("the revert doesn't apply to main".to_owned());
            }
            let mut moved_under = self.moved_under_reverts.lock().unwrap();
            if *moved_under > 0 {
                *moved_under -= 1;
                return Err(format!("{MOVED}: #9 was refused, land.log"));
            }
            drop(moved_under);
            let mut unchecked = self.unchecked_reverts.lock().unwrap();
            if *unchecked > 0 {
                *unchecked -= 1;
                return Err(format!("{UNCHECKED}, land.log"));
            }
            drop(unchecked);
            // Main may move while the revert's checks run, and the revert
            // is merged into main's newest head before it's asked about.
            self.moved
                .lock()
                .unwrap()
                .clone_from(&self.moves_while_reverting);
            self.paused
                .lock()
                .unwrap()
                .clone_from(&self.paused_while_reverting);
            let newest = self.head()?;
            if !still(&newest, &format!("{newest}+revert"))? {
                return Err(format!("{DROPPED}, land.log"));
            }
            *self.fetched_since_revert.lock().unwrap() = false;
            self.reverted.lock().unwrap().push(bad.commit.clone());
            self.revert_seen.notify_all();
            Ok(Merged {
                pr: 9,
                commit: Some("revert".to_owned()),
            })
        }
        fn unlisted_by(&self, _: &str, failing: &BTreeSet<String>, _: &str) -> Option<u64> {
            self.unlisted_for.lock().unwrap().push(failing.clone());
            self.unlisted_by
        }
        fn quarantine(
            &self,
            test: &str,
            subtests: &BTreeSet<String>,
            owner: Option<u64>,
            _: &str,
            reason: &str,
        ) -> Result<Merged, String> {
            self.put_in_quarantine.lock().unwrap().push(test.to_owned());
            self.quarantine_subtests
                .lock()
                .unwrap()
                .push(subtests.clone());
            self.quarantine_reasons
                .lock()
                .unwrap()
                .push(reason.to_owned());
            self.quarantine_owners.lock().unwrap().push(owner);
            Ok(Merged {
                pr: 8,
                commit: None,
            })
        }
        /// A revert is on main once `reaches_main_after` runs have been
        /// built, and never when it's zero; with `lands_on_fetch`, once
        /// main is fetched after it's made.
        fn on_main(&self, _: &str) -> bool {
            if self.lands_on_fetch {
                return !self.reverted.lock().unwrap().is_empty()
                    && *self.fetched_since_revert.lock().unwrap();
            }
            self.reaches_main_after > 0 && self.built().len() >= self.reaches_main_after
        }
        fn runs(&self, commit: &str) -> bool {
            self.running.iter().any(|it| it == commit)
        }
        fn wait(&self, _: u64) {}
        fn drop_runs(&self, commit: &str, check: &str) {
            let run = (commit.to_owned(), check.to_owned());
            self.dropped.lock().unwrap().insert(run);
        }
        fn keep_runs(&self, commit: &str, check: &str) {
            let run = (commit.to_owned(), check.to_owned());
            self.dropped.lock().unwrap().remove(&run);
        }
        fn dropped(&self, commit: &str, check: &str) -> bool {
            let run = (commit.to_owned(), check.to_owned());
            self.dropped.lock().unwrap().contains(&run)
        }
    }

    /// A state of the test's own. The tests run side by side, so a number
    /// counted for each keeps two that chose the same name from emptying
    /// each other's folder.
    fn state(name: &str) -> State {
        static MADE: AtomicUsize = AtomicUsize::new(0);
        let made = MADE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "merge-net-watch-{name}-{}-{made}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let state = State::at(root);
        state.make().unwrap();
        state
    }

    fn names(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn changed(text: &str) -> Changed {
        Changed {
            moved: names(text),
            broken: Vec::new(),
        }
    }

    fn findings(state: &State) -> Vec<Value> {
        let mut found: Vec<Value> = fs::read_dir(state.root.join("findings"))
            .unwrap()
            .map(|entry| {
                serde_json::from_str(&fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap()
            })
            .collect();
        found.sort_by_key(|finding| finding["kind"].as_str().unwrap_or_default().to_owned());
        found
    }

    #[test]
    fn changed_checks_lines_are_read_by_their_kind() {
        let read = Changed::parse(
            "added    vm-a\nchanged  b\nremoved  vm-c\nbroken   vm-d\nchanged  vm-e\n",
        );
        assert_eq!(read.vm_tests(), names("vm-a vm-e"));
        assert_eq!(read.broken, names("vm-d"));
    }

    #[test]
    fn a_merge_subject_names_its_pr() {
        assert_eq!(
            pr_of("Merge pull request #1660 from someone/branch"),
            Some(1660)
        );
        assert_eq!(pr_of("fix(zsh): something"), None);
    }

    #[test]
    fn a_round_starts_its_longest_tests_first() {
        // On 14 November the round from 05:18 started vm-guide-installs,
        // 26 minutes long, 46 minutes in, by its name, and ended with it.
        let state = state("longest-first");
        let log = state.log("s", "vm-a", 1);
        let quick = names("vm-a vm-b vm-c vm-d vm-e vm-f vm-g vm-h vm-i vm-j vm-k vm-l");
        for test in &quick {
            state
                .record("s", test, Ran::Passed, Some(0), &log, 60)
                .unwrap();
        }
        state
            .record("s", "vm-m", Ran::Passed, Some(0), &log, 1500)
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed(&format!("{} vm-m vm-n", quick.join(" "))),
            runs_for: Duration::from_millis(200),
            ..Stand::default()
        };
        round(&world, &state, "s").unwrap();
        let built = world.built();
        let first: Vec<&str> = built[..AT_ONCE]
            .iter()
            .map(|(_, test)| test.as_str())
            .collect();
        assert!(
            first.contains(&"vm-m"),
            "vm-m, the longest, waited for a quick test to end: {first:?}"
        );
        assert!(
            first.contains(&"vm-n"),
            "vm-n, never seen to pass, waited for a quick test to end: {first:?}"
        );
    }

    #[test]
    fn a_head_whose_checks_pass_is_the_next_base_and_runs_every_check_it_moved() {
        let state = state("passes");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a b vm-c"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        let mut built = world.built();
        built.sort();
        assert_eq!(
            built,
            vec![
                ("h".into(), "b".into()),
                ("h".into(), "vm-a".into()),
                ("h".into(), "vm-c".into())
            ]
        );
        assert_eq!(state.heads(), (Some("h".into()), Some("h".into())));
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert_eq!(
            world.built().len(),
            3,
            "a head already passed runs nothing again"
        );
    }

    #[test]
    fn the_checks_other_than_vm_tests_go_first() {
        assert_eq!(changed("vm-a b vm-c d").checks(), names("b d vm-a vm-c"));
    }

    /// One quick look, with what it found settled before it answers.
    fn quick(world: &dyn World, state: &State) -> Option<String> {
        let settler = Settler::new(world, state);
        let looked = std::thread::scope(|scope| {
            settler.start(scope);
            let looked = quick_beside(world, state, &settler);
            settler.close();
            looked
        });
        assert_eq!(settler.error(), None, "settling what it found failed");
        looked.expect("the quick look failed")
    }

    #[test]
    fn a_quick_look_reverts_a_merge_that_breaks_a_check_and_leaves_vm_tests_be() {
        let state = state("quick");
        state.set_heads("s", "s").unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("each-mark-means-one-thing vm-b"),
            ..Stand::default()
        }
        .exits("h", "each-mark-means-one-thing", &[1, 1]);
        assert_eq!(quick(&world, &state), Some("h".into()));
        assert!(
            world.built().iter().all(|(_, check)| check != "vm-b"),
            "a quick look ran a VM test: {:?}",
            world.built()
        );
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("h"),
            "the merge that broke the check wasn't reverted"
        );
        assert_eq!(
            state.quick(),
            Some("h".into()),
            "the head looked at isn't kept"
        );
        assert_eq!(
            state.heads(),
            (Some("s".into()), Some("s".into())),
            "a quick look moved the round's heads"
        );
        let built = world.built().len();
        assert_eq!(
            quick(&world, &state),
            None,
            "a head looked at was looked at again"
        );
        assert_eq!(
            world.built().len(),
            built,
            "a second look at one head ran something"
        );
    }

    #[test]
    fn a_quick_look_with_a_check_no_builder_took_asks_again_from_its_base() {
        let state = state("quick-unsent");
        state.set_heads("s", "s").unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("b"),
            ..Stand::default()
        }
        .exits("h", "b", &[UNSENT]);
        assert_eq!(quick(&world, &state), None);
        assert_eq!(
            state.quick(),
            None,
            "a look with a check not taken was kept"
        );
    }

    #[test]
    fn a_check_other_than_a_vm_test_that_fails_twice_reverts_its_merge() {
        let state = state("fails-other");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("each-mark-means-one-thing vm-b"),
            ..Stand::default()
        }
        .exits("h", "each-mark-means-one-thing", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert_eq!(state.hold().unwrap()["test"], "each-mark-means-one-thing");
        assert!(
            state.hold_file().exists(),
            "a merge not yet running holds switches"
        );
    }

    #[test]
    fn a_hold_on_a_merge_the_machine_runs_lets_switches_through_and_still_reverts() {
        // 10 November: the net held main at 09:55 on #2798, which the
        // machine had run since 08:02, and kept three fixes off it.
        let state = state("already-runs");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("each-mark-means-one-thing"),
            running: names("h"),
            ..Stand::default()
        }
        .exits("h", "each-mark-means-one-thing", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(!state.hold_file().exists(), "a switch read a hold");
        assert_eq!(state.hold().unwrap()["already_runs"], true);
    }

    #[test]
    fn a_hold_lets_switches_through_once_the_machine_runs_its_merge() {
        let state = state("comes-to-run");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": null }))
            .unwrap();
        let mut world = Stand::default();
        crate::running::let_through_once_it_runs(&world, &state).unwrap();
        assert!(state.hold_file().exists(), "the machine doesn't run b yet");
        world.running = names("b");
        crate::running::let_through_once_it_runs(&world, &state).unwrap();
        assert!(!state.hold_file().exists(), "a switch still reads the hold");
        assert_eq!(state.hold().unwrap()["bad"], "b", "the hold is kept");
    }

    #[test]
    fn a_revert_made_needless_while_its_checks_ran_is_dropped() {
        // 9 November: #2621's checks ran on the main of 12:37, and it merged
        // at 12:47, a minute after #2617 fixed the test forward.
        let state = state("needless-revert");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("every-script-meets-shellcheck"),
            moves_while_reverting: Some("m".into()),
            ..Stand::default()
        }
        .exits("h", "every-script-meets-shellcheck", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "the revert merged though main's newest head passes"
        );
        assert_eq!(state.hold(), None);
        let found = findings(&state);
        assert!(
            found
                .iter()
                .any(|it| it["kind"] == "fixed-forward" && it["fixed_at"] == "m"),
            "{found:?}"
        );
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "a dropped revert lists nothing"
        );

        // Still failing at the head main moved to, it merges as before.
        let state = super::tests::state("still-needed-revert");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("every-script-meets-shellcheck"),
            moves_while_reverting: Some("m".into()),
            ..Stand::default()
        }
        .exits("h", "every-script-meets-shellcheck", &[1, 1])
        .exits("m", "every-script-meets-shellcheck", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
    }

    #[test]
    fn a_round_cut_short_takes_up_the_head_it_began_on() {
        let state = state("taken-up");
        state.set_heads("s", "s").unwrap();
        state.begin("h1").unwrap();
        let world = Stand {
            head: "h2".into(),
            merges: names("h1"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h1".into())
        );
        assert_eq!(world.built(), vec![("h1".into(), "vm-a".into())]);
        assert_eq!(
            state.under_way(),
            None,
            "a finished round is under way no longer"
        );
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h2".into())
        );
    }

    #[test]
    fn a_round_begun_is_written_down_until_it_ends() {
        let state = state("begun");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[UNSENT; SENDS as usize]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Unfinished("h".into())
        );
        assert_eq!(state.under_way(), Some("h".into()));
    }

    #[test]
    fn a_test_that_passed_at_a_commit_never_runs_there_again() {
        let state = state("never-again");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(world.built().len(), 1);
    }

    #[test]
    fn a_run_no_builder_took_is_sent_again_and_is_no_failure() {
        let state = state("unsent");
        let world = Stand::default().exits("h", "vm-a", &[UNSENT, UNSENT, 0]);
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(state.runs("h", "vm-a"), vec![Ran::Passed]);
        let world = Stand::default().exits("h", "vm-b", &[UNSENT]);
        assert_eq!(attempt(&world, &state, "h", "vm-b"), Outcome::Unsent);
        assert_eq!(state.runs("h", "vm-b"), vec![]);
    }

    #[test]
    fn a_test_that_fails_then_passes_is_quarantined_not_reverted() {
        let state = state("flaky");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(findings(&state)[0]["kind"], "flaky");
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_failure_is_confirmed_on_the_subtests_that_failed_alone() {
        let state = state("confirmed-apart");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0])
        .exits("h", "vm-a::test_b", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into()),
            "test_b failing again alone confirms the failure"
        );
        let at_head: Vec<String> = world
            .built()
            .into_iter()
            .filter(|(commit, _)| commit == "h")
            .map(|(_, check)| check)
            .collect();
        assert_eq!(
            at_head,
            names("vm-a vm-a::test_b"),
            "the second run is test_b alone, not the whole test again"
        );
        assert_eq!(
            state.runs("h", "vm-a"),
            vec![Ran::Failed, Ran::Failed],
            "both runs count towards the whole test"
        );
    }

    #[test]
    fn a_subtest_that_passes_alone_leaves_the_whole_test_to_say() {
        let state = state("unconfirmed-apart");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0])
        .exits("h", "vm-a::test_b", &[0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into()),
            "the whole test passing again makes it flaky"
        );
        let at_head: Vec<String> = world
            .built()
            .into_iter()
            .filter(|(commit, _)| commit == "h")
            .map(|(_, check)| check)
            .collect();
        assert_eq!(
            at_head,
            names("vm-a vm-a::test_b vm-a"),
            "test_b alone passed, so the whole test ran again to say"
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a subtest that passes alone reverts nothing"
        );
    }

    #[test]
    fn a_test_that_fails_twice_reverts_its_merge_and_holds_the_switch() {
        let state = state("fails");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        let hold = state.hold().unwrap();
        assert_eq!(hold["bad"], "h");
        assert_eq!(hold["revert_pr"], 9);
        assert_eq!(hold["revert_commit"], "revert");
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed");
        assert_eq!(found[0]["owner_pr"], 1);
        assert_eq!(
            state.heads(),
            (Some("h".into()), Some("h".into())),
            "the held test moves the base on"
        );
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Idle,
            "a head is failed once"
        );
    }

    #[test]
    fn a_test_that_fails_is_reverted_while_the_round_s_others_still_run() {
        let state = state("reverts-at-once");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a vm-slow"),
            after_revert: names("vm-slow"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "the slow test was still running when its round reverted the merge"
        );
    }

    #[test]
    fn whether_main_s_newest_head_fixed_it_is_asked_beside_the_search() {
        let state = state("forward-beside-search");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("m1 m2 h"),
            changed: changed("vm-a"),
            newest_waits_for_search: true,
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("n", "vm-a", &[1, 1])
        .exits("m2", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("m2"));
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "the search began while main's newest head was still being tried"
        );
    }

    #[test]
    fn a_hold_is_cleared_as_its_revert_merges() {
        let state = state("cleared-as-it-merges");
        state.set_heads("s", "s").unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            lands_on_fetch: true,
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(
            state.hold().is_none(),
            "the revert merged, so nothing waits for the next round"
        );
    }

    #[test]
    fn a_hold_is_cleared_as_its_revert_reaches_main_mid_round() {
        let state = state("cleared-mid-round");
        state
            .set_hold(&json!({"test": "vm-x", "bad": "b", "revert_commit": "r"}))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            reaches_main_after: 1,
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(
            state.hold().is_none(),
            "the revert reached main as the round's test ran"
        );
    }

    #[test]
    fn tests_failing_while_another_is_searched_for_ask_main_meanwhile() {
        let state = state("fixed-forward-meanwhile");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("a b c d e f g h"),
            changed: changed("vm-a vm-b"),
            together_at_newest: names("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("h", "vm-b", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "one test's search held the other's ask of main's newest head"
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(state.hold(), None);
        let fixed: Vec<Value> = findings(&state)
            .into_iter()
            .filter(|found| found["kind"] == "fixed-forward")
            .map(|found| found["test"].clone())
            .collect();
        assert_eq!(fixed.len(), 2, "both tests are found fixed forward");
    }

    #[test]
    fn tests_failing_at_once_are_searched_side_by_side_and_each_reverted() {
        let state = state("side-by-side");
        state.set_heads("s", "s").unwrap();
        let mut world = Stand {
            head: "h".into(),
            merges: names("a b c d e f g h"),
            changed: changed("vm-a vm-b"),
            searched_together: names("vm-a vm-b"),
            ..Stand::default()
        };
        for commit in names("c d e f g h") {
            world = world.exits(&commit, "vm-a", &[1, 1]);
        }
        for commit in names("f g h") {
            world = world.exits(&commit, "vm-b", &[1, 1]);
        }
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "one test's search waited for another's to end"
        );
        let mut reverted = world.reverted.lock().unwrap().clone();
        reverted.sort();
        assert_eq!(reverted, names("c f"), "each test's merge is reverted");
    }

    #[test]
    fn a_test_failing_one_run_at_mains_newest_head_is_listed_there() {
        let state = state("flaky-at-newest");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("n", "vm-a", &[1, 0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(state.hold(), None);
        let found = findings(&state);
        assert_eq!(
            (&found[0]["kind"], &found[0]["commit"]),
            (&json!("flaky"), &json!("n"))
        );
    }

    #[test]
    fn a_test_fixed_forward_since_its_round_began_reverts_nothing() {
        let state = state("fixed-forward");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(state.hold(), None);
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "fixed-forward");
        assert_eq!(found[0]["fixed_at"], "n");

        let state = super::tests::state("fixed-before-the-search");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("a b c d e f g h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        let mut built = world.built();
        built.sort();
        assert_eq!(
            built,
            [
                ("a", "vm-a"),
                ("a", "vm-a"),
                ("b", "vm-a"),
                ("b", "vm-a"),
                ("c", "vm-a"),
                ("c", "vm-a"),
                ("d", "vm-a"),
                ("d", "vm-a"),
                ("e", "vm-a"),
                ("e", "vm-a"),
                ("f", "vm-a"),
                ("f", "vm-a"),
                ("g", "vm-a"),
                ("g", "vm-a"),
                ("h", "vm-a"),
                ("h", "vm-a"),
                ("n", "vm-a"),
                ("n", "vm-a"),
            ]
            .map(|(commit, check)| (commit.to_owned(), check.to_owned())),
            "the search stops after its first step once main's head passes, each of its \
             commits and main's head run twice at once"
        );
        assert_eq!(
            state.heads(),
            (Some("h".into()), Some("h".into())),
            "the next round starts from the head and runs to main's newest"
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(state.hold(), None);
        let found = findings(&state);
        assert_eq!(
            (&found[0]["kind"], &found[0]["commit"]),
            (&json!("fixed-forward"), &json!("h"))
        );

        let state = super::tests::state("still-broken");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("n", "vm-a", &[1, 1]);
        round(&world, &state, "s").unwrap();
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
    }

    #[test]
    fn a_flaky_test_main_has_moved_is_listed_only_if_it_flakes_at_main_too() {
        // A round at h, behind main's newest head n, finds vm-a flaky.
        fn flaky_at_h(name: &str, at_n: &[i32]) -> (Stand, State) {
            let state = state(name);
            state.set_heads("s", "s").unwrap();
            state.begin("h").unwrap();
            let world = Stand {
                head: "n".into(),
                merges: names("h"),
                changed: changed("vm-a"),
                ..Stand::default()
            }
            .exits("h", "vm-a", &[1, 0])
            .exits("n", "vm-a", at_n);
            round(&world, &state, "s").unwrap();
            (world, state)
        }

        let (world, _) = flaky_at_h("flaky-mended", &[0, 0]);
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "vm-a, flaky at h but passing at main's newest head n, was listed on h's word"
        );

        let (world, state) = flaky_at_h("flaky-still", &[1, 0]);
        assert_eq!(
            *world.put_in_quarantine.lock().unwrap(),
            names("vm-a"),
            "vm-a, flaky at h and at main's newest head n, wasn't listed"
        );
        assert_eq!(
            findings(&state)[0]["commit"],
            "n",
            "vm-a was listed on h's word, not n's"
        );
    }

    #[test]
    fn a_flaky_test_is_owned_by_the_newest_merge_that_moved_it() {
        let state = state("flaky-owner");
        // The stand-in's PR is a merge's name's length: bb is #2, ccc #3.
        let world = Stand {
            head: "ccc".into(),
            merges: names("a bb ccc"),
            changed: changed("vm-a"),
            same: names("ccc"),
            ..Stand::default()
        }
        .exits("ccc", "vm-a", &[1, 0]);
        round(&world, &state, "s").unwrap();
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "flaky");
        assert_eq!(
            found[0]["owner_pr"], 2,
            "not the head's #3, which moved nothing"
        );
    }

    #[test]
    fn a_merge_that_moved_the_whole_system_owns_no_flaky_test() {
        // bb moved every test, as a change to the system all tests boot
        // does; a moved the desktop tests alone.
        let round_of = |merges: &str, name: &str| {
            let state = state(name);
            let world = Stand {
                head: "bb".into(),
                merges: names(merges),
                changed: changed("vm-desktop-a vm-install-b"),
                still: vec![("a".into(), "vm-install-b".into())],
                ..Stand::default()
            }
            .exits("bb", "vm-desktop-a", &[1, 0]);
            round(&world, &state, "s").unwrap();
            findings(&state)[0]["owner_pr"].clone()
        };
        assert_eq!(
            round_of("a bb", "whole-system-passed-over"),
            1,
            "a, which moved the desktop alone, not bb"
        );
        assert_eq!(
            round_of("bb", "whole-system-alone"),
            Value::Null,
            "no merge moved it alone"
        );
    }

    #[test]
    fn a_test_s_family_is_the_words_its_name_opens_with() {
        assert_eq!(family("vm-desktop-opens-and-sets"), "vm-desktop");
        assert_eq!(family("vm-install"), "vm-install");
    }

    #[test]
    fn a_quarantined_test_that_fails_reverts_nothing() {
        let state = state("quarantined");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            quarantined: names("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(findings(&state)[0]["kind"], "failed-in-quarantine");
    }

    #[test]
    fn a_test_failing_only_in_its_listed_subtests_passes() {
        let state = state("listed-subtest");
        state.set_heads("s", "s").unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("m1 h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            listed_by_subtest: vec![("vm-a".into(), names("test_b"))],
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into()),
            "test_b, which failed, is listed"
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a run failing only in a listed subtest was reverted"
        );
        assert!(
            world.built().iter().all(|(at, _)| at == "h"),
            "nothing is searched: {:?}",
            world.built()
        );
    }

    #[test]
    fn a_listed_test_failing_in_a_subtest_it_doesnt_list_is_searched() {
        let state = state("unlisted-subtest");
        state.set_heads("s", "s").unwrap();
        state.make().unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("m1 m2 m3 h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            listed_by_subtest: vec![("vm-a".into(), names("test_old"))],
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("m2", "vm-a::test_b", &[1, 1])
        .exits("m3", "vm-a::test_b", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("m2"),
            "only test_old is listed, and test_b broke at m2"
        );
    }

    #[test]
    fn a_flaky_test_is_listed_by_the_subtest_that_failed() {
        let state = state("flaky-subtest");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        assert_eq!(
            *world.quarantine_subtests.lock().unwrap(),
            [names("test_b").into_iter().collect::<BTreeSet<_>>()],
            "only test_b failed, so only it is listed"
        );
    }

    #[test]
    fn a_flaky_test_main_lists_by_subtests_failing_in_none_named_is_told_not_listed() {
        // At 20:11 on 13 November vm-desktop-launcher-finds-sessions timed
        // out once, naming no subtest, while main listed two of its
        // subtests; a list never lists such a test whole, so the change
        // came out empty, git refused it, and the net said it had failed.
        let state = state("flaky-held-subtest");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            listed_by_subtest: vec![("vm-a".into(), names("test_b"))],
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "main lists vm-a by test_b, yet the net listed it again"
        );
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "flaky", "its owner isn't told: {found:?}");
        assert!(
            found[0]["quarantine_error"].is_null(),
            "a test main lists was said to have failed listing: {found:?}"
        );
    }

    #[test]
    fn a_test_main_lists_is_left_be_while_main_cant_be_fetched() {
        let state = state("listed-unreachable");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("a b h"),
            changed: changed("vm-a"),
            listed_since: names("vm-a"),
            fetch_fails: Some("could not resolve host: github.com".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert!(
            world.built().iter().all(|(at, _)| at == "h"),
            "nothing is searched: {:?}",
            world.built()
        );
        assert_eq!(findings(&state)[0]["kind"], "failed-in-quarantine");
    }

    #[test]
    fn a_test_main_lists_since_its_round_began_reverts_nothing() {
        let state = state("listed-since");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("a b h"),
            changed: changed("vm-a"),
            listed_since: names("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("n", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(state.hold(), None);
        assert!(
            world.built().iter().all(|(at, _)| at == "h"),
            "nothing is searched: {:?}",
            world.built()
        );
        assert_eq!(findings(&state)[0]["kind"], "failed-in-quarantine");
    }

    #[test]
    fn a_pinned_test_whose_revert_conflicts_is_listed_with_its_owner() {
        let state = state("listed-on-conflict");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            reverts_conflict: true,
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        let reasons = world.quarantine_reasons.lock().unwrap().clone();
        assert!(
            reasons[0].contains("won't apply"),
            "a conflicting revert is listed as {reasons:?}"
        );
        assert_eq!(state.hold(), None, "switches go on");
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed");
        assert_eq!(found[0]["owner_pr"], 1);
        assert_eq!(found[0]["quarantine_pr"], 8);
    }

    #[test]
    fn a_bisect_starts_after_the_last_merge_known_to_pass() {
        let state = state("known");
        let log = state.log("m4", "vm-a", 1);
        state
            .record("m4", "vm-a", Ran::Passed, Some(0), &log, 0)
            .unwrap();
        let world = Stand {
            head: "m6".into(),
            merges: names("m1 m2 m3 m4 m5 m6"),
            changed: changed("vm-a"),
            missing: vec![("s".into(), "vm-a".into())],
            ..Stand::default()
        }
        .exits("m1", "vm-a", &[1, 1])
        .exits("m2", "vm-a", &[1, 1])
        .exits("m3", "vm-a", &[1, 1])
        .exits("m5", "vm-a", &[1, 1])
        .exits("m6", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m6".into())
        );
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("m5"),
            "what fails before a recorded pass now is no merge's doing"
        );
    }

    #[test]
    fn the_merge_that_broke_a_test_is_found_among_several() {
        let state = state("bisect");
        let world = Stand {
            head: "m5".into(),
            merges: names("m1 m2 m3 m4 m5"),
            changed: changed("vm-a"),
            missing: vec![("s".into(), "vm-a".into())],
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1])
        .exits("m4", "vm-a", &[1])
        .exits("m5", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m5".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("m2"));
        assert!(
            world.built().contains(&("m1".into(), "vm-a".into())),
            "m1, before m2, passes"
        );
    }

    #[test]
    fn a_search_runs_only_the_subtest_that_failed_at_its_head() {
        let state = state("searches-a-subtest");
        state.set_heads("s", "s").unwrap();
        state.make().unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("m1 m2 m3 h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("m2", "vm-a::test_b", &[1, 1])
        .exits("m3", "vm-a::test_b", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("m2"),
            "test_b broke at m2"
        );
        let searched: Vec<(String, String)> = world
            .built()
            .into_iter()
            .filter(|(commit, _)| ["m1", "m2", "m3"].contains(&commit.as_str()))
            .collect();
        assert!(
            !searched.is_empty() && searched.iter().all(|(_, check)| check == "vm-a::test_b"),
            "each step ran only the subtest that failed in both runs at the head: {searched:?}"
        );
    }

    #[test]
    fn only_the_merges_that_changed_a_test_are_searched() {
        let state = state("searches-what-moved");
        state.set_heads("s", "s").unwrap();
        let unmoved = names("m1 m2 m4 m5 m6 m7 h");
        let world = Stand {
            head: "h".into(),
            merges: names("m1 m2 m3 m4 m5 m6 m7 h"),
            changed: changed("vm-a"),
            still: unmoved
                .iter()
                .map(|merge| (merge.clone(), "vm-a".to_owned()))
                .collect(),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("m3"));
        let searched: Vec<String> = world
            .built()
            .into_iter()
            .map(|(commit, _)| commit)
            .filter(|commit| {
                !commit.ends_with("+revert") && !["h", "m1", "m2"].contains(&commit.as_str())
            })
            .collect();
        assert!(
            searched.is_empty(),
            "only m3 changed vm-a, so nothing is run but the first merge and the one before m3: \
             {searched:?}"
        );
    }

    #[test]
    fn a_newest_head_building_the_failing_test_unchanged_isn_t_run() {
        let state = state("newest-unchanged");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            newest_builds_as: Some("h".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(
            !world.built().contains(&("n".into(), "vm-a".into())),
            "main's newest head builds vm-a as h does, so it fails there too"
        );
    }

    fn search_for(test: &str) -> Job {
        Job::Search {
            base: "s".into(),
            head: "h".into(),
            test: test.into(),
        }
    }

    #[test]
    fn a_test_main_lists_by_its_turn_is_not_searched() {
        let state = state("listed-by-its-turn");
        let world = Stand {
            head: "n".into(),
            merges: names("a b h"),
            changed: changed("vm-b"),
            listed_since: names("vm-b vm-w1 vm-w2 vm-w3 vm-w4"),
            ..Stand::default()
        }
        .exits("n", "vm-b", &[1, 1]);
        let settler = Settler::new(&world, &state);
        for test in names("vm-w1 vm-w2 vm-w3 vm-w4") {
            settler.hand(search_for(&test));
        }
        settler.hand(search_for("vm-b"));
        std::thread::scope(|scope| {
            settler.start(scope);
            settler.close();
        });
        assert!(
            world.built().iter().all(|(at, _)| at == "n"),
            "the test whose turn came after main listed it is searched"
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a test main listed had a merge reverted"
        );
        assert!(state.settling().is_empty(), "a job settled is still kept");
    }

    #[test]
    fn a_test_with_no_room_to_search_is_asked_beside_the_round() {
        // 13 November: the round at b7a81d60bf21 ran its tests to 08:02,
        // then ran five failed tests at main's newest head one by one as
        // it handed each on, and ended past 09:30.
        let state = state("asked-beside");
        let world = Stand {
            head: "n".into(),
            merges: names("a b h"),
            changed: changed("vm-b"),
            // So the four searches before it end at their turn.
            listed_since: names("vm-w1 vm-w2 vm-w3 vm-w4"),
            ..Stand::default()
        }
        .exits("n", "vm-b", &[0, 0]);
        let settler = Settler::new(&world, &state);
        for test in names("vm-w1 vm-w2 vm-w3 vm-w4") {
            settler.hand(search_for(&test));
        }
        settler.hand(search_for("vm-b"));
        assert!(
            world.built().is_empty(),
            "handing a test on ran it at main's newest head in the round's thread"
        );
        assert!(
            settler.holds("vm-b"),
            "the test with no room wasn't handed on"
        );
        std::thread::scope(|scope| {
            settler.start(scope);
            settler.close();
        });
        assert!(
            world.built().contains(&("n".to_owned(), "vm-b".to_owned())),
            "the test with no room wasn't asked at main's newest head"
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a test fixed forward had a merge reverted"
        );
    }

    #[test]
    fn a_round_ends_with_its_tests_while_its_search_waits() {
        // 10 November: the round begun at 08:13 ran its tests to 09:02 and
        // what they found to 10:31, and every merge since waited for it.
        let state = state("ends-with-its-tests");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        let settler = Settler::new(&world, &state);
        assert_eq!(
            round_beside(&world, &state, "s", &settler).unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "the round waited for its search"
        );
        assert!(settler.holds("vm-a"), "the search wasn't handed on");
        assert_eq!(state.heads().0.as_deref(), Some("h"));
        std::thread::scope(|scope| {
            settler.start(scope);
            settler.close();
        });
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(state.settling().is_empty(), "a job settled is still kept");
    }

    #[test]
    fn a_round_runs_on_while_a_settle_writes_the_hold() {
        // 13 November: #3972's revert was settled from 07:52 to 08:22, and
        // the round's tests waited on it, one by one, as each ended.
        let state = state("beside-a-settle");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[0])
        .exits("h", "vm-b", &[0]);
        let settler = Settler::new(&world, &state);
        let (held, holding) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (ended, ending) = std::sync::mpsc::channel();
        let writer = &settler;
        std::thread::scope(|scope| {
            scope.spawn(move || {
                writer.write(|| {
                    held.send(()).unwrap();
                    let _ = released.recv();
                });
            });
            holding.recv().unwrap();
            scope.spawn(|| {
                let _ = ended.send(round_beside(&world, &state, "s", &settler));
            });
            let round = ending.recv_timeout(Duration::from_secs(5));
            release.send(()).unwrap();
            assert_eq!(
                round.ok().and_then(Result::ok),
                Some(Round::Passed("h".into())),
                "the round waited on the settle writing the hold"
            );
        });
    }

    #[test]
    fn a_net_handing_over_leaves_its_search_under_way_to_the_next() {
        // 10 November: a watcher told at 21:32 that another build was
        // installed waited on its search's runs and tested nothing to 22:51.
        let state = state("hands-over");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            runs_for: Duration::from_millis(500),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        let settler = Settler::new(&world, &state);
        settler.hand(search_for("vm-a"));
        let left = std::thread::scope(|scope| {
            settler.start(scope);
            world.wait_while(|built| built.is_empty(), "the search's runs");
            let left = settler.leave(|| (settler.holds("vm-a"), state.settling().len()));
            settler.close();
            left
        });
        assert!(
            left.0,
            "the net waited for its search to end before it left"
        );
        assert_eq!(left.1, 1, "the search wasn't kept for the next build");
    }

    #[test]
    fn a_net_leaving_as_a_settle_ends_keeps_nothing_it_settled() {
        // 14 November: a watcher told at 02:55 that another build was
        // installed left the moment its settle listed shows-then-rests in
        // #4288, at 03:08, before letting the search go, and the next
        // build searched the test again, ten VM runs to 03:21.
        let state = state("leaves-settled");
        state
            .set_hold(&json!({ "test": "vm-z", "bad": "b", "revert_commit": null }))
            .unwrap();
        let world = Stand {
            head: "n".into(),
            runs_for: Duration::from_millis(200),
            ..Stand::default()
        };
        let settler = Settler::new(&world, &state);
        settler.hand(Job::Forward {
            head: "h".into(),
            newest: "n".into(),
            test: "vm-z".into(),
        });
        let kept = std::thread::scope(|scope| {
            settler.start(scope);
            world.wait_while(|built| built.is_empty(), "the held test's run");
            // The worker lets the job go only once it has the queue, so a
            // net leaving between the two finds the job as it was left.
            let kept = settler.holding_the_queue(|| {
                let until = std::time::Instant::now() + Duration::from_secs(10);
                while state.hold().is_some() && std::time::Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(10));
                }
                assert!(state.hold().is_none(), "the pass never cleared the hold");
                settler.leave(|| state.settling().len())
            });
            settler.close();
            kept
        });
        assert_eq!(
            kept, 0,
            "a job settled as the net left was kept for the next build to do again"
        );
    }

    #[test]
    fn a_search_cut_short_by_a_restart_is_taken_up_again() {
        let state = state("search-taken-up");
        state.set_heads("h", "h").unwrap();
        state
            .keep_settling(
                "vm-a",
                &json!({"kind": "search", "base": "s", "head": "h", "test": "vm-a"}),
            )
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        assert!(state.settling().is_empty(), "a job settled is still kept");
    }

    #[test]
    fn a_test_whose_search_is_under_way_is_not_handed_again() {
        let state = state("not-handed-again");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        let settler = Settler::new(&world, &state);
        settler.hand(search_for("vm-a"));
        settler.hand(Job::Search {
            base: "h".into(),
            head: "n".into(),
            test: "vm-a".into(),
        });
        assert_eq!(state.settling().len(), 1);
        assert_eq!(
            state.settling()[0]["head"],
            "h",
            "the second search replaced the first"
        );
    }

    #[test]
    fn a_merge_s_two_runs_in_a_search_start_together() {
        let state = state("together");
        let world = Stand {
            head: "m5".into(),
            merges: names("m1 m2 m3 m4 m5"),
            changed: changed("vm-a"),
            missing: vec![("s".into(), "vm-a".into())],
            pairs: names("m2"),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1])
        .exits("m4", "vm-a", &[1])
        .exits("m5", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m5".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("m2"));
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "m2's second run starts beside its first, not after it fails"
        );
    }

    #[test]
    fn runs_going_at_a_commit_are_waited_for_not_run_again() {
        let state = state("shared");
        let world = Stand {
            head: "h".into(),
            runs_for: Duration::from_millis(300),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1]);
        let (searched, rounded) = std::thread::scope(|scope| {
            let search = scope.spawn(|| attempt_together(&world, &state, "h", "vm-a"));
            world.wait_while(|built| built.is_empty(), "the search's runs");
            let round = scope.spawn(|| attempt(&world, &state, "h", "vm-a"));
            (search.join().unwrap(), round.join().unwrap())
        });
        assert_eq!((searched, rounded), (Outcome::Failed, Outcome::Failed));
        assert_eq!(
            world.built().len(),
            2,
            "the round counts the search's two runs rather than running its own beside them"
        );
    }

    #[test]
    fn a_bisect_step_tries_merges_spread_over_what_is_left() {
        assert_eq!(spread(0, 14, 3), vec![3, 7, 10]);
        assert_eq!(spread(8, 10, 3), vec![8, 9], "each merge left once");
        assert_eq!(spread(4, 5, 3), vec![4]);
        assert_eq!(spread(2, 9, 1), vec![5], "one is the middle");
    }

    #[test]
    fn a_bisect_over_many_merges_narrows_by_quarters() {
        let state = state("quarters");
        let mut world = Stand {
            head: "m15".into(),
            merges: names("m1 m2 m3 m4 m5 m6 m7 m8 m9 m10 m11 m12 m13 m14 m15"),
            changed: changed("vm-a"),
            missing: vec![("s".into(), "vm-a".into())],
            ..Stand::default()
        };
        for broken in 11..=15 {
            world = world.exits(&format!("m{broken}"), "vm-a", &[1]);
        }
        // Searches that share the room each try the least a step does.
        let found = search(&world, &state, "s", "m15", "vm-a", &|| SPLITS).unwrap();
        settle(&world, &state, "s", "m15", found.unwrap()).unwrap();
        assert_eq!(*world.reverted.lock().unwrap(), names("m11"));
        let built = world.built();
        for tried in ["m4", "m8", "m9", "m10", "m11"] {
            assert!(
                built.contains(&(tried.into(), "vm-a".into())),
                "{tried} is tried: {built:?}"
            );
        }
        assert!(
            !built.contains(&("m12".into(), "vm-a".into())),
            "a halving's second step, m12, is never needed: {built:?}"
        );
    }

    #[test]
    fn a_bisect_step_stops_the_runs_after_a_merge_that_fails() {
        // m3's and m4's runs would go on long after m2 failed, as on
        // 14 November, when four runs past the failure took 12 minutes more.
        let state = state("stops-the-rest");
        let world = Stand {
            head: "m5".into(),
            merges: names("m1 m2 m3 m4 m5"),
            changed: changed("vm-a"),
            slow: names("m3 m4"),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, 1]);
        let found = bisect(&world, &state, &world.merges, "vm-a", &|| true, &|| SPLITS);
        assert_eq!(found.as_deref(), Some("m2"), "the bisect didn't find m2");
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "runs after m2's failure went on to their end: {:?}",
            world.waited_out.lock().unwrap()
        );
        assert!(
            state.runs("m3", "vm-a").is_empty(),
            "a stopped run at m3 was kept as an outcome"
        );
        assert!(!world.dropped("m3", "vm-a"), "m3's runs stay dropped");
    }

    #[test]
    fn a_stopped_run_is_no_failure() {
        let state = state("stopped");
        let world = Stand::default().exits("h", "vm-a", &[STALL, 1, 0]);
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Flaky);
        assert_eq!(
            state.runs("h", "vm-a"),
            vec![Ran::Stalled, Ran::Failed, Ran::Passed]
        );
    }

    #[test]
    fn a_test_stalling_twice_blames_no_merge() {
        let state = state("stalls");
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, STALL]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m2".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert!(state.hold().is_none());
        assert_eq!(findings(&state)[0]["kind"], "stalled");
        assert_eq!(
            world.built.lock().unwrap().len(),
            3,
            "one failure and two stalls, and no bisect"
        );
    }

    #[test]
    fn a_search_its_runs_stalled_through_holds_no_switch() {
        let state = state("search-stalls");
        let world = Stand {
            head: "m3".into(),
            merges: names("m1 m2 m3"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m3", "vm-a", &[1, 1])
        .exits("m1", "vm-a", &[STALL, STALL])
        .exits("m2", "vm-a", &[STALL, STALL]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m3".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert!(
            !state.hold_file().exists(),
            "nothing is known broken, so a switch isn't held"
        );
        let kept = state.hold().expect("the test is kept to run again");
        assert_eq!(kept["test"], "vm-a");
        assert_eq!(kept["unsettled"], true);
    }

    #[test]
    fn a_merge_where_the_failing_subtest_never_built_is_blamed_for_none() {
        let state = state("search-unbuilt");
        state.set_heads("s", "s").unwrap();
        state.make().unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("m1 m2 m3 h"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            unbuilt: names("m1 m2"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("m1", "vm-a::test_b", &[1, 1])
        .exits("m2", "vm-a::test_b", &[1, 1])
        .exits("m3", "vm-a::test_b", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "m1 failed only to build, which says nothing of test_b, so it's reverted for none"
        );
        assert!(
            !state.hold_file().exists(),
            "no merge is known to have broken test_b, so a switch isn't held"
        );
        let kept = state.hold().expect("the test is kept to run again");
        assert_eq!(kept["unsettled"], true, "{kept}");
    }

    #[test]
    fn a_failure_already_found_fixed_forward_is_not_searched_again() {
        let state = state("heard-again");
        state
            .find(&json!({ "kind": "fixed-forward", "test": "vm-a", "commit": "m2" }))
            .unwrap();
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        let found = search(&world, &state, "s", "m2", "vm-a", &|| SPLITS).unwrap();
        assert!(found.is_none(), "it found something to hold");
        assert!(world.built.lock().unwrap().is_empty(), "nothing runs");
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_merge_already_found_fixed_forward_is_not_reverted_when_the_newest_head_cant_say() {
        let state = state("fixed-before-it-was-taken-up");
        state
            .find(&json!({ "kind": "fixed-forward", "test": "vm-a", "commit": "m1" }))
            .unwrap();
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m1", "vm-a", &[1, 1])
        .exits("m2", "vm-a", &[1, 1]);
        let found = search(&world, &state, "s", "m2", "vm-a", &|| SPLITS).unwrap();
        assert!(
            found.is_none(),
            "m1, found fixed forward before, was blamed again"
        );
    }

    #[test]
    fn a_test_born_failing_blames_no_merge() {
        let state = state("born");
        let world = Stand {
            head: "m3".into(),
            merges: names("m1 m2 m3"),
            changed: changed("vm-a"),
            missing: vec![("s".into(), "vm-a".into()), ("m1".into(), "vm-a".into())],
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m3".into())
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a split may only have moved a failing case"
        );
        assert!(state.hold().is_none());
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed-at-birth");
        assert_eq!(found[0]["commit"], "m2");
    }

    #[test]
    fn a_test_failing_before_the_start_blames_no_merge() {
        let state = state("before");
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("s", "vm-a", &[1])
        .exits("m1", "vm-a", &[1])
        .exits("m2", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m2".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert!(state.hold().is_none());
        assert_eq!(findings(&state)[0]["kind"], "failed-before");
    }

    #[test]
    fn a_test_the_base_failed_twice_in_none_named_is_left_where_main_lists_it() {
        // A run killed at its timeout names no subtest, and a list never
        // lists whole a test it lists by subtests, so listing it made an
        // empty change git refused, as for a flaky one at 20:11 on 13
        // November.
        let state = state("still-red-held-subtest");
        let log = state.log("s", "vm-a", 1);
        for _ in 0..FAILURES {
            state
                .record("s", "vm-a", Ran::Failed, Some(1), &log, 0)
                .unwrap();
        }
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            listed_by_subtest: vec![("vm-a".into(), names("test_b"))],
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, 1]);
        round(&world, &state, "s").unwrap();
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "main lists vm-a by test_b, yet the net listed it again"
        );
        let found = findings(&state);
        assert_eq!(
            found[0]["kind"], "failed-before",
            "its owner isn't told: {found:?}"
        );
        assert!(
            found[0]["quarantine_error"].is_null(),
            "a test main lists was said to have failed listing: {found:?}"
        );
    }

    #[test]
    fn a_test_the_base_failed_twice_is_searched_no_further() {
        let state = state("base-failed");
        let log = state.log("s", "vm-a", 1);
        for _ in 0..FAILURES {
            state
                .record("s", "vm-a", Ran::Failed, Some(1), &log, 0)
                .unwrap();
        }
        let world = Stand {
            head: "m4".into(),
            merges: names("m1 m2 m3 m4"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m4", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m4".into())
        );
        let searched: Vec<_> = world
            .built()
            .into_iter()
            .filter(|(commit, _)| commit != "m4")
            .collect();
        assert!(
            searched.is_empty(),
            "it searched merges after a base that already failed: {searched:?}"
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "a merge was blamed for what failed before it"
        );
        assert!(state.hold().is_none(), "switches were held");
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed-before");
        assert_eq!(found[0]["since"], "s");
    }

    #[test]
    fn a_test_the_base_failed_twice_is_listed_owned_by_the_pr_that_took_it_off() {
        let state = state("still-red");
        let log = state.log("s", "vm-a", 1);
        for _ in 0..FAILURES {
            state
                .record("s", "vm-a", Ran::Failed, Some(1), &log, 0)
                .unwrap();
        }
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            fails_in: Some("test_b".into()),
            unlisted_by: Some(2603),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, 1]);
        round(&world, &state, "s").unwrap();
        assert_eq!(
            *world.unlisted_for.lock().unwrap(),
            [names("test_b").into_iter().collect::<BTreeSet<_>>()],
            "its owner wasn't asked for by the subtest failing now, so one who took off only \
             their own entry goes unfound"
        );
        assert_eq!(
            *world.put_in_quarantine.lock().unwrap(),
            names("vm-a"),
            "a test red at its base and its head was left off the list, owned by nobody"
        );
        assert_eq!(*world.quarantine_owners.lock().unwrap(), vec![Some(2603)]);
        let found = findings(&state);
        assert_eq!(found[0]["owner_pr"], 2603);
        assert_eq!(found[0]["quarantine_pr"], 8);
    }

    #[test]
    fn a_break_main_fixed_before_it_is_settled_holds_and_reverts_nothing() {
        let state = state("fixed-meanwhile");
        let world = Stand {
            head: "m3".into(),
            merges: names("m1 m2 m3"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1]);
        let found = search(&world, &state, "s", "m3", "vm-a", &|| SPLITS).unwrap();
        assert!(
            matches!(&found, Some(Found::Merge(bad)) if bad.commit == "m2"),
            "the search blames m2"
        );
        // Main moves on to a fix while the find waits to be settled.
        *world.moved.lock().unwrap() = Some("m4".into());
        settle(&world, &state, "s", "m3", found.unwrap()).unwrap();
        assert!(
            state.hold().is_none(),
            "switches were held for a break main's newest head had fixed"
        );
        assert!(world.reverted.lock().unwrap().is_empty(), "m2 was reverted");
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "fixed-forward");
        assert_eq!(found[0]["fixed_at"], "m4");
    }

    #[test]
    fn main_s_newest_head_is_asked_once_between_a_search_finding_a_merge_and_its_revert() {
        // Main moves on each time its newest head is asked, as it did on
        // 14 November while each answer took a VM run.
        let state = state("asked-once");
        let world = Stand {
            head: "m3".into(),
            merges: names("m1 m2 m3"),
            changed: changed("vm-a"),
            newest: Mutex::new(names("m3 m4 m5")),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, 1])
        .exits("m3", "vm-a", &[1, 1])
        .exits("m4", "vm-a", &[1, 1])
        .exits("m5", "vm-a", &[1, 1]);
        let found = search(&world, &state, "s", "m3", "vm-a", &|| SPLITS).unwrap();
        settle(&world, &state, "s", "m3", found.unwrap()).unwrap();
        let newest: Vec<_> = world
            .built()
            .into_iter()
            .filter(|(commit, _)| commit == "m4" || commit == "m5")
            .map(|(commit, _)| commit)
            .collect();
        assert!(
            newest.len() == 2 && newest[0] == newest[1],
            "main's newest head was asked again as soon as it had answered: {newest:?}"
        );
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("m2"),
            "m2 wasn't reverted"
        );
    }

    #[test]
    fn a_revert_lands_only_if_its_test_fails_without_it_and_passes_with_it_at_mains_head() {
        // Main moves on to m4 while the revert's checks run, and its test
        // is asked about there with and without the revert, never judged
        // from the main the revert was cut from, as #2783 was.
        let reverting = |name: &str, newest: &[i32], with: &[i32]| {
            let state = state(name);
            let world = Stand {
                head: "m3".into(),
                merges: names("m1 m2 m3"),
                changed: changed("vm-a"),
                moves_while_reverting: Some("m4".into()),
                ..Stand::default()
            }
            .exits("m2", "vm-a", &[1])
            .exits("m3", "vm-a", &[1])
            .exits("m4", "vm-a", newest)
            .exits("m4+revert", "vm-a", with);
            round(&world, &state, "s").unwrap();
            (state, world)
        };
        // It passes with the revert too, so only the run without it can
        // drop the revert.
        let (state, world) = reverting("revert-needless", &[0], &[0]);
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "the revert landed though main's newest head passes its test"
        );
        assert_eq!(state.hold(), None);
        let (state, world) = reverting("revert-mends-nothing", &[1], &[1]);
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "the revert landed though its test fails with it at main's newest head"
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        let reasons = world.quarantine_reasons.lock().unwrap().clone();
        assert!(
            reasons[0].contains("fails with it reverted too") && !reasons[0].contains("apply"),
            "a revert that mends nothing is listed as {reasons:?}"
        );
        assert_eq!(state.hold(), None, "the listing clears the hold");
        let (_, world) = reverting("revert-mends", &[1], &[0]);
        assert_eq!(*world.reverted.lock().unwrap(), names("m2"));
    }

    #[test]
    fn a_test_red_at_its_base_that_main_fixed_since_is_not_listed() {
        let state = state("still-red-fixed");
        let log = state.log("s", "vm-a", 1);
        for _ in 0..FAILURES {
            state
                .record("s", "vm-a", Ran::Failed, Some(1), &log, 0)
                .unwrap();
        }
        let world = Stand {
            head: "m2".into(),
            merges: names("m1 m2"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1, 1]);
        let found = search(&world, &state, "s", "m2", "vm-a", &|| SPLITS).unwrap();
        *world.moved.lock().unwrap() = Some("m3".into());
        settle(&world, &state, "s", "m2", found.unwrap()).unwrap();
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "a test main's newest head passes was listed"
        );
    }

    #[test]
    fn a_test_failing_after_a_merge_that_left_it_alone_blames_none() {
        let state = state("unchanged");
        let world = Stand {
            head: "m3".into(),
            merges: names("m1 m2 m3"),
            changed: changed("vm-a"),
            same: names("m1"),
            ..Stand::default()
        }
        .exits("m1", "vm-a", &[1])
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m3".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert!(state.hold().is_none());
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed-unchanged");
        assert_eq!(found[0]["commit"], "m1");
        assert_eq!(found[0]["owner_pr"], Value::Null);
    }

    #[test]
    fn a_test_held_for_a_revert_waits_for_it() {
        let state = state("pending");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": 9, "revert_commit": "r" }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert_eq!(world.built(), vec![("h".into(), "vm-b".into())]);
        assert!(state.hold().is_some(), "the hold waits for the revert");
        assert!(world.reverted.lock().unwrap().is_empty());
    }

    #[test]
    fn a_revert_paused_while_its_checks_ran_lands_only_once_the_pause_is_lifted() {
        // 10 November's case: the net was paused while it checked the
        // revert of #2809, and merged it two minutes later.
        let state = state("revert-paused");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            paused_while_reverting: Some("a fix forward is near".into()),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1])
        .exits("h+revert", "vm-a", &[0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert!(
            world.reverted.lock().unwrap().is_empty(),
            "the revert landed while the net was paused"
        );
        assert_eq!(
            *world.put_in_quarantine.lock().unwrap(),
            Vec::<String>::new()
        );
        let hold = state.hold().unwrap();
        assert_eq!(hold["revert_again"], true, "{hold}");

        // Still paused, the next round starts nothing; lifted, it lands.
        state.set_heads("s", "s").unwrap();
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert_eq!(*world.reverted.lock().unwrap(), Vec::<String>::new());
        *world.paused.lock().unwrap() = None;
        let world = Stand {
            paused_while_reverting: None,
            ..world
        };
        round(&world, &state, "s").unwrap();
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
    }

    #[test]
    fn a_revert_no_builder_checked_is_tried_again_next_round() {
        let state = state("revert-again");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            unchecked_reverts: Mutex::new(1),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        let hold = state.hold().unwrap();
        assert_eq!(hold["revert_again"], true, "{hold}");
        assert!(world.reverted.lock().unwrap().is_empty());

        // The next round, with a builder back, lands it before anything else.
        state.set_heads("s", "s").unwrap();
        round(&world, &state, "s").unwrap();
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        let hold = state.hold().unwrap();
        assert_eq!(hold["revert_commit"], "revert", "{hold}");
        assert_eq!(hold["revert_again"], false, "{hold}");
    }

    #[test]
    fn a_revert_main_moved_under_is_tried_again_rather_than_quarantined() {
        let state = state("revert-moved-under");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            moved_under_reverts: Mutex::new(1),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        round(&world, &state, "s").unwrap();
        let hold = state.hold().expect("the refused revert left no hold");
        assert_eq!(
            hold["revert_again"], true,
            "a revert refused as main moved wasn't kept to try again: {hold}"
        );
        assert!(
            world.put_in_quarantine.lock().unwrap().is_empty(),
            "a revert refused as main moved was quarantined instead"
        );

        state.set_heads("s", "s").unwrap();
        round(&world, &state, "s").unwrap();
        assert_eq!(
            *world.reverted.lock().unwrap(),
            names("h"),
            "the revert wasn't landed the round after main moved under it"
        );
    }

    #[test]
    fn a_revert_that_wont_apply_is_not_tried_again() {
        let state = state("revert-not-again");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": null }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        round(&world, &state, "s").unwrap();
        assert!(world.reverted.lock().unwrap().is_empty());
    }

    #[test]
    fn a_test_fixed_forward_clears_a_hold_with_no_revert() {
        let state = state("forward");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": null }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_held_test_fixed_forward_while_a_round_runs_clears_the_hold_before_it_ends() {
        let state = state("forward-mid-round");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        state
            .set_hold(&json!({ "test": "vm-z", "bad": "b", "revert_commit": null }))
            .unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("b h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(
            world.built().contains(&("n".into(), "vm-z".into())),
            "{:?}",
            world.built()
        );
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_held_test_is_run_at_the_newest_head_beside_the_round_not_in_it() {
        // 13 November: a round thread that ended a test ran the held test
        // at main's newest head itself, a VM test's minutes, before taking
        // up its next test.
        let state = state("forward-beside");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        state
            .set_hold(&json!({ "test": "vm-z", "bad": "b", "revert_commit": null }))
            .unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("b h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        let settler = Settler::new(&world, &state);
        assert_eq!(
            round_beside(&world, &state, "s", &settler).unwrap(),
            Round::Passed("h".into())
        );
        assert!(
            !world.built().contains(&("n".into(), "vm-z".into())),
            "the round ran the held test at main's newest head itself: {:?}",
            world.built()
        );
        assert!(
            settler.holds("vm-z"),
            "the held test's run wasn't handed on"
        );
        // A net restarted meanwhile takes the run up again from disk.
        let restarted = Settler::new(&world, &state);
        restarted.take_up();
        std::thread::scope(|scope| {
            restarted.start(scope);
            restarted.close();
        });
        assert!(
            world.built().contains(&("n".into(), "vm-z".into())),
            "the handed run never ran: {:?}",
            world.built()
        );
        assert!(
            state.hold().is_none(),
            "a pass at main's newest head left the hold"
        );
        assert!(state.settling().is_empty(), "a run settled is still kept");
    }

    #[test]
    fn a_held_test_runs_first_and_its_pass_clears_the_hold_at_once() {
        let state = state("first");
        state
            .set_hold(&json!({ "test": "vm-z", "bad": "b", "revert_pr": null }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a vm-b vm-z"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        let built = world.built();
        assert_eq!(built[0], ("h".into(), "vm-z".into()), "{built:?}");
        assert_eq!(built.len(), 3, "the held test's run is reused: {built:?}");
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_held_machine_starts_nothing() {
        let state = state("held");
        let world = Stand {
            head: "h".into(),
            changed: changed("vm-a"),
            held: Some("a reboot".into()),
            ..Stand::default()
        };
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert_eq!(
            world.built(),
            [] as [(std::string::String, std::string::String); 0]
        );
        let held = fs::read_to_string(state.root.join("held.json")).unwrap_or_default();
        assert!(
            held.contains("a reboot"),
            "a held net didn't say so where cadence reads it: {held}"
        );
    }
}
