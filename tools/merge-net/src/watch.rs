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
//! hold is cleared. A run the builder never took
//! (`nix-offload` exits 75), or whose builder couldn't fetch the commit's
//! source, is tried again later, and is no failure, and so is one stopped
//! before it ended, for running too long or going silent: twice at a commit, and the test is left there with a
//! finding, blaming no merge. A test that passed at a commit never runs
//! there again.
//!
//! The round talks to git, nix and GitHub through `World`, so each of
//! those choices is tested against a stand-in (the tests below).

use std::path::Path;
use std::sync::Mutex;

use serde_json::{Value, json};

use crate::state::{Ran, State, now, short};

/// What `nix-offload` exits with when no builder took the build.
pub const UNSENT: i32 = 75;

/// How a revert or quarantine whose checks no builder took fails, so the
/// net tries it again next round rather than leave its hold for a person.
pub const UNCHECKED: &str = "no builder took its checks";

/// How a revert fails that main's newest head made needless while its
/// checks ran, so it's neither merged nor tried again.
pub const DROPPED: &str = "main's newest head passes its test, so it was dropped";

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
/// bulk, so they take no room another session's run is waiting for.
const SPLITS: usize = 3;

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
    /// The tests quarantined at `head`.
    fn quarantined(&self, head: &str) -> Vec<String>;
    /// Revert a bad merge on main through a PR merged on its own checks,
    /// once `still`, asked with the main they ran on, says it's still
    /// needed.
    fn revert(&self, bad: &Bad, still: &dyn Fn(&str) -> bool) -> Result<Merged, String>;
    /// Put a flaky test on the quarantine list through a merged PR.
    fn quarantine(
        &self,
        test: &str,
        owner: Option<u64>,
        commit: &str,
        reason: &str,
    ) -> Result<Merged, String>;
    /// Whether main holds `commit`.
    fn on_main(&self, commit: &str) -> bool;
    /// Wait this many seconds.
    fn wait(&self, seconds: u64);
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
pub fn attempt(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
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
    while failures < FAILURES {
        if stalls >= STALLS {
            return Outcome::Stalled;
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
    let code = loop {
        let code = world.build(commit, check, &log);
        if code != Some(UNSENT) {
            break code;
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
        Some(_) => Ran::Failed,
        None => Ran::Stalled,
    };
    if let Err(why) = state.record(commit, check, ran, code, &log) {
        state.say(&why);
    }
    Some(ran)
}

/// Like `attempt`, but every run still needed to call `check` broken at
/// `commit` starts at once, for a search and a fix-forward check, where
/// a failure is likely and the search waits on it. A failure is then
/// confirmed in one run's time rather than two in a row, which on
/// 7 November was about five minutes of each step; the cost is a run
/// beside the first where the first passes. A stall leaves the rest to
/// `attempt`, one at a time.
fn attempt_together(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
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
    attempt(world, state, commit, check)
}

/// Run each check at `commit`, `AT_ONCE` at a time, and give each outcome
/// in the order asked. `then` hears each outcome the moment its check
/// ends, on the thread that ran it, while the others still run.
pub fn attempt_all(
    world: &dyn World,
    state: &State,
    commit: &str,
    checks: &[String],
    then: &(dyn Fn(&str, Outcome) + Sync),
) -> Vec<Outcome> {
    let next = Mutex::new(0_usize);
    let outcomes = Mutex::new(vec![Outcome::Unsent; checks.len()]);
    std::thread::scope(|scope| {
        for _ in 0..AT_ONCE.min(checks.len()) {
            scope.spawn(|| {
                loop {
                    let index = {
                        let mut next = next
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let index = *next;
                        *next += 1;
                        index
                    };
                    let Some(check) = checks.get(index) else {
                        break;
                    };
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
/// says to stop.
pub fn bisect(
    world: &dyn World,
    state: &State,
    merges: &[String],
    check: &str,
    go_on: &dyn Fn() -> bool,
) -> Option<String> {
    // `good` is how many of the merges are known to pass, `bad` the index
    // of the first known to fail.
    let (mut good, mut bad) = known(state, merges, check)?;
    let mut first = true;
    while good < bad {
        if !std::mem::take(&mut first) && !go_on() {
            return None;
        }
        let tried = spread(good, bad, SPLITS);
        let passed = std::thread::scope(|scope| {
            let runs: Vec<_> = tried
                .iter()
                .map(|&index| scope.spawn(move || passes(world, state, &merges[index], check)))
                .collect();
            runs.into_iter()
                .map(|run| run.join().ok().flatten())
                .collect::<Option<Vec<bool>>>()
        })?;
        // The first merge tried that fails is where it may have broken, and
        // the last before it to pass is past where it can have. A pass after
        // a failure tells nothing, since the failure already bounds it.
        if let Some(first) = tried.iter().zip(&passed).position(|(_, &ok)| !ok) {
            bad = tried[first];
        }
        if let Some(&last) = tried
            .iter()
            .zip(&passed)
            .rev()
            .find(|&(&index, &ok)| ok && index < bad)
            .map(|(index, _)| index)
        {
            good = last + 1;
        }
    }
    merges.get(bad).cloned()
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
    let good = runs
        .iter()
        .rposition(|ran| ran.contains(&Ran::Passed))
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
    if !matches!(outcome, Outcome::Passed | Outcome::Flaky) {
        return Ok(false);
    }
    let owner = commit.and_then(|commit| pr_of(&world.subject(commit)));
    if outcome == Outcome::Flaky {
        if !state.found("flaky", test, &newest)
            && !world.quarantined(&newest).contains(&test.to_owned())
        {
            list_flaky(world, state, &newest, test, owner)?;
        }
        state.say(&format!(
            "{test} {} at {}, and fails one of two runs at main's newest head {}, so it is \
             listed there: nothing is searched for, reverted or held",
            if commit.is_some() { "broke" } else { "fails" },
            short(commit.unwrap_or(head)),
            short(&newest)
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
        short(&newest)
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
    let merges = movers(world, &all, test);
    if merges.len() > 1 {
        state.say(&format!(
            "finding which of {} merges broke {test}, the {} that changed it",
            all.len(),
            merges.len()
        ));
    }
    Ok(merges)
}

/// The merges among `merges` (oldest first, on main's first parents) worth
/// searching for what broke `test`: those that changed its derivation from
/// the commit before them, or that it couldn't be told for. Only these can
/// have broken it, since a test whose derivation is the same at two
/// commits does there what it does here, and each is a few seconds'
/// evaluation against minutes a run. On 7 November the search among 16
/// merges took 73 minutes. The first merge stays all the same, so a test
/// failing from the range's start, which no merge changed, is still told
/// apart as broken outside the code rather than blamed on the next merge.
fn movers(world: &dyn World, merges: &[String], test: &str) -> Vec<String> {
    let mut before: Option<String> = None;
    merges
        .iter()
        .enumerate()
        .filter(|(at, merge)| {
            let now = world.drv(merge, test).ok();
            let moved = *at == 0 || now.is_none() || now != before;
            before = now;
            moved
        })
        .map(|(_, merge)| merge.clone())
        .collect()
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
        Err(why) if why.starts_with(UNCHECKED) => hold["revert_again"] = json!(true),
        Err(_) => hold["revert_again"] = json!(false),
    }
    state.set_hold(hold)
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
    let reverted = world.revert(&found, &still_broken(world, state, &found));
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

/// One round: look at main's head, if there's something new to look at.
/// `start` is the base of the first round, before any head has passed.
pub fn round(world: &dyn World, state: &State, start: &str) -> Result<Round, String> {
    if let Some(why) = world.held() {
        state.say(&format!("held, starting nothing: {why}"));
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
    revert_again(world, state)?;
    clear_if_reverted(world, state)?;
    let base = passed.clone().unwrap_or_else(|| start.to_owned());
    if seen.as_deref() == Some(head.as_str()) || base == head {
        return Ok(Round::Idle);
    }
    state.begin(&head)?;
    state.say(&format!("comparing {} with {}", short(&base), short(&head)));
    let changed = world.changed(&base, &head)?;
    let hold = clear_if_fixed_forward(world, state, &changed, &head)?;
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
    let breaking = Breaking::new(world, state, &base, &head, left_be);
    let outcomes = attempt_all(world, state, &head, &tests, &|test, ran| {
        breaking.heard(test, ran);
    });
    breaking.done()?;
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
                    state.clear_hold()?;
                    state.say(&format!(
                        "{test} passes again at {}, so the hold is cleared",
                        short(&head)
                    ));
                }
            }
            Outcome::Flaky if !kept_out && !state.found("flaky", test, &head) => {
                list_flaky(world, state, &head, test, owner_in(test))?;
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

/// A test that fails for real has its merge found and reverted the moment
/// it does, while the round's other tests still run. On 7 November
/// vm-checks-go-first failed twice at 12:46, but the search for its merge
/// waited 34 minutes behind the round's slower desktop tests, and main
/// stayed broken 113 minutes in all. One merge is looked for at a time,
/// since each writes the hold, and the first error stops the rest.
struct Breaking<'a, F> {
    world: &'a dyn World,
    state: &'a State,
    base: &'a str,
    head: &'a str,
    /// Tests whose failure reverts nothing: quarantined, or already held.
    left_be: F,
    done: Mutex<Result<(), String>>,
    /// Main's newest head a held test with no revert was last run at.
    tried: Mutex<Option<String>>,
}

impl<'a, F: Fn(&str) -> bool> Breaking<'a, F> {
    fn new(
        world: &'a dyn World,
        state: &'a State,
        base: &'a str,
        head: &'a str,
        left_be: F,
    ) -> Self {
        Self {
            world,
            state,
            base,
            head,
            left_be,
            done: Mutex::new(Ok(())),
            tried: Mutex::new(None),
        }
    }

    /// Each test's end also clears a hold whose revert is on main, since a
    /// round can run long past it: on 7 November the revert merged at
    /// 13:48, and eight merges waited off the machine behind the hold while
    /// the round it was found in ran on.
    ///
    /// A failed test that finds another's search under way asks whether
    /// main's newest head fixed it while it waits, rather than after. On
    /// 8 November six tests failed at once at 06fc027e, and their searches
    /// ran one after another from 10:30 to past 15:00, though five of the six
    /// were fixed forward and each was found so within one search step.
    ///
    /// A test whose turn comes asks again whether it's left be, since the
    /// search before it can take an hour: on 9 November main listed
    /// launcher tests at 09:22 that had been waiting since 08:06 behind
    /// other searches, each decided to be searched as it failed.
    fn heard(&self, test: &str, outcome: Outcome) {
        let searched = outcome == Outcome::Failed && !(self.left_be)(test);
        let mut done = match self.done.try_lock() {
            Ok(done) => done,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                // An error here is asked again beside the search's first
                // step, so it's left to that.
                if searched
                    && fixed_forward(self.world, self.state, self.head, test, None) == Ok(true)
                {
                    self.fixed_forward_since();
                    return;
                }
                self.done
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            }
        };
        if done.is_err() {
            return;
        }
        *done = clear_if_reverted(self.world, self.state);
        if done.is_ok() && searched && !(self.left_be)(test) {
            *done = broke(self.world, self.state, self.base, self.head, test);
        }
        drop(done);
        self.fixed_forward_since();
    }

    /// Each test's end also runs a test held with no revert at main's newest
    /// head, once a head, and clears the hold when it passes there. Only a
    /// round's start asked before: on 8 November #2381 fixed the held
    /// vm-desktop-launcher-goes-places forward at 12:10, while the round
    /// begun at 09:30 ran on, and every merge waited off the machine behind
    /// the hold. One thread asks at a time; the rest carry on.
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
        if same_drv(self.world, &newest, self.head, &test)
            || attempt(self.world, self.state, &newest, &test) != Outcome::Passed
        {
            return;
        }
        if self.state.clear_hold().is_ok() {
            self.state.say(&format!(
                "{test} passes at main's newest head {}, so the hold is cleared",
                short(&newest)
            ));
        }
    }

    fn done(self) -> Result<(), String> {
        self.done
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
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
    state.clear_hold()?;
    state.say(&format!(
        "{test} passes again at {}, so the hold is cleared",
        short(head)
    ));
    Ok(None)
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
    let made = world.quarantine(test, owner, head, &reason);
    state.find(&json!({
        "kind": "flaky",
        "test": test,
        "commit": head,
        "owner_pr": owner,
        "log": state.log(head, test, 1).display().to_string(),
        "quarantine_pr": made.as_ref().ok().map(|merged| merged.pr),
        "quarantine_error": made.err(),
        "at": now(),
    }))?;
    Ok(())
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

/// A test failed for real at `head`: find the merge since `base` that
/// broke it, hold switching on it, revert it, and tell its owner.
fn broke(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    test: &str,
) -> Result<(), String> {
    // A round taken up again after a restart hears its failures again, and
    // one already found fixed forward was searched a second time on
    // 8 November, where runs stalled and held main.
    if state.found("fixed-forward", test, head) {
        return Ok(());
    }
    // A test the base failed twice failed before every merge since, so no
    // search over them can find what broke it, and `before` clears the
    // merge one finds only after all its runs. On 9 November
    // vm-desktop-looks failed twice at the base while it was listed, came
    // off the list still failing, and its search took four runs of 25
    // minutes each. A merge that broke it again after another fixed it in
    // the same round goes unblamed, which costs less than such a search.
    let failed_at_base = state
        .runs(base, test)
        .iter()
        .filter(|&&ran| ran == Ran::Failed)
        .count();
    if failed_at_base >= FAILURES {
        let log = state.log(head, test, 2).display().to_string();
        state.find(&json!({
            "kind": "failed-before", "test": test, "commit": head, "since": base,
            "owner_pr": Value::Null, "log": log, "at": now(),
        }))?;
        state.say(&format!(
            "{test} fails at {}, and failed twice at {} before it, so no merge since broke it: \
             nothing is reverted or held",
            short(head),
            short(base)
        ));
        return Ok(());
    }
    let merges = to_search(world, state, base, head, test)?;
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
            bisect(world, state, &merges, test, &|| fixed() == Ok(false))
        }
    });
    if fixed()? {
        return Ok(());
    }
    let failing = bad.clone().unwrap_or_else(|| head.to_owned());
    let log = state.log(&failing, test, 2).display().to_string();
    let bad = match bad {
        Some(commit) => match before(world, state, &commit, test)? {
            Before::Passed => Some(commit),
            Before::Unsent => None,
            not_broken_here => {
                return blames_none(world, state, test, &commit, &log, not_broken_here);
            }
        },
        None => None,
    };
    if let Some(commit) = &bad
        && fixed_forward(world, state, head, test, Some(commit))?
    {
        return Ok(());
    }
    // A search ends with no merge only when a run it needed stalled or no
    // builder took it, so the test is kept to run again at the next head
    // without holding switches.
    let Some(commit) = bad else {
        state.set_hold(&json!({
            "test": test, "bad": Value::Null, "since": base, "head": head, "at": now(),
            "unsettled": true,
        }))?;
        state.say(&format!(
            "{test} fails at {}, but its search couldn't tell which merge broke it, since a \
             run stalled or no builder took it: switches go on, and it runs again at the next head",
            short(head)
        ));
        state.find(&json!({
            "kind": "failed", "test": test, "commit": head, "since": base,
            "owner_pr": Value::Null, "log": log, "revert_error": "couldn't tell which merge broke it",
            "at": now(),
        }))?;
        return Ok(());
    };
    let subject = world.subject(&commit);
    let found = Bad {
        commit: commit.clone(),
        pr: pr_of(&subject),
        subject,
        test: test.to_owned(),
        log: log.clone(),
    };
    // A search can take hours, and the owner may have listed the test
    // meanwhile, so the list as main has it now is read again.
    if listed_on_main(world, head, test) {
        state.find(&json!({
            "kind": "failed-in-quarantine", "test": test, "commit": commit,
            "owner_pr": found.pr, "log": log, "at": now(),
        }))?;
        state.say(&format!(
            "{test} broke at {}, but main lists it as quarantined now: nothing is reverted or held",
            short(&commit)
        ));
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
    let mut hold = json!({
        "test": test, "bad": commit, "bad_pr": found.pr, "revert_pr": Value::Null,
        "revert_commit": Value::Null, "at": now(),
    });
    state.set_hold(&hold)?;
    let reverted = world.revert(found, &still_broken(world, state, found));
    keep_revert(state, &mut hold, &reverted)?;
    if dropped(state, found, &reverted)? {
        return Ok(());
    }
    let listed: Option<Result<Merged, String>> = match &reverted {
        Err(why) if !why.starts_with(UNCHECKED) => Some(list_broken(world, state, found, head)),
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

/// Whether a revert whose checks ran on main at the commit it's asked with
/// is still needed: main hasn't moved since, or its test still fails at
/// main's newest head. On 9 November the net's #2621 reverted #2597 at
/// 12:47, its checks run on the main of 12:37, a minute after #2617 had
/// fixed the test forward, and so broke main again.
fn still_broken<'a>(
    world: &'a dyn World,
    state: &'a State,
    found: &'a Bad,
) -> impl Fn(&str) -> bool + 'a {
    move |checked_on: &str| match world.head() {
        Ok(newest) if newest != checked_on => {
            fixed_forward(world, state, checked_on, &found.test, Some(&found.commit)) != Ok(true)
        }
        _ => true,
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

/// A merge pinned for breaking a test whose revert won't apply has its test
/// listed as quarantined, with the merge's PR as its owner, and the hold
/// cleared, so the net goes on to main's head and switches go on while the
/// owner fixes it forward. On 9 November the revert of #2363 conflicted
/// with the 138 merges since, and main stayed held until a fix forward
/// landed by hand 23 minutes later; the owner is told in the finding.
fn list_broken(
    world: &dyn World,
    state: &State,
    found: &Bad,
    head: &str,
) -> Result<Merged, String> {
    let what = found
        .pr
        .map_or_else(|| short(&found.commit).to_owned(), |pr| format!("#{pr}"));
    let reason = format!(
        "broken since {what}, whose revert won't apply; its owner fixes it forward and takes \
         it off this list"
    );
    let made = world.quarantine(&found.test, found.pr, head, &reason)?;
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
        /// Tests main's newest head lists too once a search has reverted a
        /// merge.
        listed_once_reverted: Vec<String>,
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
        /// Checks whose each run at the round's last merge ends only once a
        /// search has run one at an older merge, or after a while without,
        /// when it's put in `waited_out`.
        waits_for_search: Vec<String>,
        /// Commits whose each run waits for a second run there to start,
        /// or after a while without, when it's put in `waited_out`.
        pairs: Vec<String>,
        /// Checks whose each run at main's newest head waits for every one
        /// of them to start there, or after a while without, when it's put
        /// in `waited_out`.
        together_at_newest: Vec<String>,
        build_seen: Condvar,
        waited_out: Mutex<Vec<String>>,
        reaches_main_after: usize,
        lands_on_fetch: bool,
        fetched_since_revert: Mutex<bool>,
        /// A commit main's newest head builds every check as.
        newest_builds_as: Option<String>,
        put_in_quarantine: Mutex<Vec<String>>,
        held: Option<String>,
        /// How many reverts to come find no builder for their checks.
        unchecked_reverts: Mutex<u32>,
        /// The head main moves to while a revert's checks run.
        moves_while_reverting: Option<String>,
        moved: Mutex<Option<String>>,
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
    }

    impl World for Stand {
        fn held(&self) -> Option<String> {
            self.held.clone()
        }
        fn head(&self) -> Result<String, String> {
            if let Some(error) = &self.fetch_fails {
                return Err(error.clone());
            }
            *self.fetched_since_revert.lock().unwrap() = true;
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
        fn build(&self, commit: &str, check: &str, _: &Path) -> Option<i32> {
            self.built
                .lock()
                .unwrap()
                .push((commit.to_owned(), check.to_owned()));
            self.build_seen.notify_all();
            if self.newest_waits_for_search && commit == self.head {
                let searched = |built: &mut Vec<(String, String)>| {
                    !built
                        .iter()
                        .any(|(at, _)| self.merges[..self.merges.len() - 1].contains(at))
                };
                let built = self.built.lock().unwrap();
                let (built, waited) = self
                    .build_seen
                    .wait_timeout_while(built, Duration::from_secs(10), searched)
                    .unwrap();
                drop(built);
                if waited.timed_out() {
                    self.waited_out.lock().unwrap().push(check.to_owned());
                }
            }
            if self.merges.last().is_some_and(|last| last == commit)
                && self.waits_for_search.iter().any(|waits| waits == check)
            {
                let unsearched = |built: &mut Vec<(String, String)>| {
                    !built
                        .iter()
                        .any(|(at, _)| self.merges[..self.merges.len() - 1].contains(at))
                };
                let built = self.built.lock().unwrap();
                let (built, waited) = self
                    .build_seen
                    .wait_timeout_while(built, Duration::from_secs(10), unsearched)
                    .unwrap();
                drop(built);
                if waited.timed_out() {
                    self.waited_out.lock().unwrap().push(check.to_owned());
                }
            }
            if commit == self.head && self.together_at_newest.iter().any(|one| one == check) {
                let apart = |built: &mut Vec<(String, String)>| {
                    !self
                        .together_at_newest
                        .iter()
                        .all(|one| built.iter().any(|(at, ran)| at == commit && ran == one))
                };
                let built = self.built.lock().unwrap();
                let (built, waited) = self
                    .build_seen
                    .wait_timeout_while(built, Duration::from_secs(10), apart)
                    .unwrap();
                drop(built);
                if waited.timed_out() {
                    self.waited_out.lock().unwrap().push(check.to_owned());
                }
            }
            if self.pairs.iter().any(|paired| paired == commit) {
                let alone = |built: &mut Vec<(String, String)>| {
                    built.iter().filter(|(at, _)| at == commit).count() < 2
                };
                let built = self.built.lock().unwrap();
                let (built, waited) = self
                    .build_seen
                    .wait_timeout_while(built, Duration::from_secs(10), alone)
                    .unwrap();
                drop(built);
                if waited.timed_out() {
                    self.waited_out.lock().unwrap().push(commit.to_owned());
                }
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
            (exit != STALL).then_some(exit)
        }
        fn quarantined(&self, commit: &str) -> Vec<String> {
            let mut listed = self.quarantined.clone();
            if commit == self.head {
                listed.extend(self.listed_since.iter().cloned());
                if !self.reverted.lock().unwrap().is_empty() {
                    listed.extend(self.listed_once_reverted.iter().cloned());
                }
            }
            listed
        }
        fn revert(&self, bad: &Bad, still: &dyn Fn(&str) -> bool) -> Result<Merged, String> {
            if self.reverts_conflict {
                return Err("the revert doesn't apply to main".to_owned());
            }
            let mut unchecked = self.unchecked_reverts.lock().unwrap();
            if *unchecked > 0 {
                *unchecked -= 1;
                return Err(format!("{UNCHECKED}, land.log"));
            }
            drop(unchecked);
            // Its checks ran on main as it was when the revert was cut.
            let cut_from = self.head()?;
            self.moved
                .lock()
                .unwrap()
                .clone_from(&self.moves_while_reverting);
            if !still(&cut_from) {
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
        fn quarantine(
            &self,
            test: &str,
            _: Option<u64>,
            _: &str,
            _: &str,
        ) -> Result<Merged, String> {
            self.put_in_quarantine.lock().unwrap().push(test.to_owned());
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
        fn wait(&self, _: u64) {}
    }

    fn state(name: &str) -> State {
        let root =
            std::env::temp_dir().join(format!("merge-net-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let state = State { root };
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
                ("b", "vm-a"),
                ("b", "vm-a"),
                ("d", "vm-a"),
                ("d", "vm-a"),
                ("f", "vm-a"),
                ("f", "vm-a"),
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
            .record("m4", "vm-a", Ran::Passed, Some(0), &log)
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
            .filter(|commit| !["h", "m1", "m2"].contains(&commit.as_str()))
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

    #[test]
    fn a_test_main_lists_while_it_waits_for_a_search_is_not_searched() {
        let state = state("listed-while-waiting");
        state.set_heads("s", "s").unwrap();
        state.begin("h").unwrap();
        let world = Stand {
            head: "n".into(),
            merges: names("a b h"),
            changed: changed("vm-a vm-b"),
            listed_once_reverted: names("vm-a vm-b"),
            waits_for_search: names("vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1])
        .exits("h", "vm-b", &[1])
        .exits("n", "vm-a", &[1])
        .exits("n", "vm-b", &[1]);
        round(&world, &state, "s").unwrap();
        let searched: std::collections::BTreeSet<String> = world
            .built()
            .into_iter()
            .filter(|(at, _)| at == "a" || at == "b")
            .map(|(_, test)| test)
            .collect();
        assert_eq!(
            searched.len(),
            1,
            "the test whose turn came after main listed it isn't searched: {searched:?}"
        );
        assert!(
            world.waited_out.lock().unwrap().is_empty(),
            "the second test failed while the first one's search ran"
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
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m15".into())
        );
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
        broke(&world, &state, "s", "m2", "vm-a").unwrap();
        assert!(world.built.lock().unwrap().is_empty(), "nothing runs");
        assert!(state.hold().is_none());
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
    fn a_test_the_base_failed_twice_is_searched_no_further() {
        let state = state("base-failed");
        let log = state.log("s", "vm-a", 1);
        for _ in 0..FAILURES {
            state
                .record("s", "vm-a", Ran::Failed, Some(1), &log)
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
    }
}
