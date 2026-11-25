//! What a round found, settled beside the rounds after it: the search for
//! the merge that broke a test, the PR that lists a flaky one, the run
//! of a held test at main's newest head that may clear its hold, and a
//! Mac's comparison a merge's gate skipped.
//!
//! A round once waited for both before it ended, and every merge since
//! waited for the next round to begin. Since 9 November rounds spent 30 to
//! 110 minutes settling after their last test, and on 10 November the
//! round begun at 08:13 ran its tests to 09:02 and its searches, checks
//! and two quarantine PRs, one after another, to 10:31. A round now hands
//! them here and ends with its tests.
//!
//! Each job is kept on disk from the moment it's handed until it's done, so
//! a net restarted between rounds, as every switch that changes it does,
//! takes it up again; one that ends in an error stays there, and the next
//! round takes it up.
//!
//! Long because the queue, its lanes and how each kind of job settles
//! share one lock and one record on disk, so a job's rest, hold or end
//! can't be read apart from the queue that takes it up.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread::Scope;
use std::time::Duration;

use serde_json::{Value, json};

use super::{
    AT_ONCE, Bad, Confirming, Outcome, SEARCHES, SPLITS, World, ask_before_settling, attempt,
    attempt_confirming, clear_hold_of, fixed_forward, list_flaky_unless_mended, listed_on_main,
    lock, pr_of, revert_and_tell, same_drv, search, settle,
};
use crate::state::{Ran, State, now, short};

/// Something a round found that takes longer than its tests to settle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Find the merge since `base` that broke `test`, which failed twice at
    /// `head`, and hold and revert it.
    Search {
        base: String,
        head: String,
        test: String,
    },
    /// List `test`, which failed and then passed at `head`, as quarantined.
    Flaky {
        head: String,
        test: String,
        owner: Option<u64>,
    },
    /// Run `test`, held with no revert, at main's newest head `newest`,
    /// and clear the hold if it passes there. `head` is the round's, at
    /// which a held test that changed has run already.
    Forward {
        head: String,
        newest: String,
        test: String,
    },
    /// Compare a Mac's checks at `merge`, whose gate skipped it, as the
    /// `check` naming the merge's first parent (`macs.rs`), and revert it
    /// if it fails twice; handed at `since`, in seconds since the epoch.
    Mac {
        merge: String,
        check: String,
        since: u64,
    },
}

impl Job {
    fn test(&self) -> &str {
        match self {
            Self::Search { test, .. } | Self::Flaky { test, .. } | Self::Forward { test, .. } => {
                test
            }
            Self::Mac { check, .. } => check,
        }
    }

    fn head(&self) -> &str {
        match self {
            Self::Search { head, .. } | Self::Flaky { head, .. } | Self::Forward { head, .. } => {
                head
            }
            Self::Mac { merge, .. } => merge,
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Search { base, head, test } => {
                json!({ "kind": "search", "base": base, "head": head, "test": test })
            }
            Self::Flaky { head, test, owner } => {
                json!({ "kind": "flaky", "head": head, "test": test, "owner": owner })
            }
            Self::Forward { head, newest, test } => {
                json!({ "kind": "forward", "head": head, "newest": newest, "test": test })
            }
            Self::Mac {
                merge,
                check,
                since,
            } => {
                json!({ "kind": "mac", "merge": merge, "test": check, "since": since })
            }
        }
    }

    fn from_json(kept: &Value) -> Option<Self> {
        let text = |key: &str| kept[key].as_str().map(str::to_owned);
        match kept["kind"].as_str()? {
            "search" => Some(Self::Search {
                base: text("base")?,
                head: text("head")?,
                test: text("test")?,
            }),
            "flaky" => Some(Self::Flaky {
                head: text("head")?,
                test: text("test")?,
                owner: kept["owner"].as_u64(),
            }),
            "forward" => Some(Self::Forward {
                head: text("head")?,
                newest: text("newest")?,
                test: text("test")?,
            }),
            "mac" => Some(Self::Mac {
                merge: text("merge")?,
                check: text("test")?,
                since: kept["since"].as_u64().unwrap_or_else(now),
            }),
            _ => None,
        }
    }
}

/// How long a Mac's comparison no Mac takes is tried before it's told as
/// unrun: a day, since a Mac away overnight is usual, and its owner can
/// still compare it by hand.
const UNRUN: u64 = 24 * 60 * 60;

/// How long a Mac's comparison that compared nothing rests before it's
/// taken up again: as long as it has waited, from ten minutes to an hour.
/// A round can come a minute after the last, and each comparison holds a
/// lent builder for many minutes, so one rerun each round would spend
/// the builders on a Mac whose network is out all day.
const RESTS: (u64, u64) = (10 * 60, 60 * 60);

/// How often a net leaving looks again whether the write it waits on only
/// builds or has ended.
const LOOK_AGAIN: Duration = Duration::from_millis(200);

/// Wait `LOOK_AGAIN` before a leaving net, or a write behind it, looks
/// again, parked rather than asleep. A timed park is a futex wait, so the
/// stall guard, which tells a hung net by every thread waiting on a lock,
/// reads a leave looking at a write that never ends as it read one blocked
/// on it, the quick look having ended. A leave that only slept would hide
/// that write, and one that blocked after a while would wait out a
/// revert's Mac comparison whole, the stall it exists to end.
fn look_again() {
    std::thread::park_timeout(LOOK_AGAIN);
}

/// How many searches waiting for room may ask main's newest head at once
/// whether it fixed their test.
const ASKS: usize = 1;

/// What a worker does with the job it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    Search,
    /// Ask whether main's newest head fixed a search's test, while the
    /// searches have no room for it.
    Ask,
    List,
    Forward,
    /// Compare a Mac's checks a merge's gate skipped, one at a time, since
    /// one Mac takes them.
    Mac,
}

/// The jobs waiting and under way.
#[derive(Default)]
struct Queue {
    waiting: VecDeque<Job>,
    /// The tests of every job waiting or under way, one job a test.
    held: Vec<String>,
    /// The tests waiting whose search asked main's newest head already.
    asked: Vec<String>,
    searching: usize,
    asking: usize,
    /// The lanes under way that take one job at a time: lists, held
    /// tests' runs and a Mac's comparisons.
    alone: Vec<Lane>,
    closing: bool,
}

impl Queue {
    /// The first job waiting that may start: up to `SEARCHES` searches,
    /// `ASKS` asks for searches left without room, one list at a time,
    /// since two lists' PRs change one file, and one held test's run at a
    /// time, since only one test is held.
    fn next(&mut self) -> Option<(Job, Lane)> {
        let (index, lane) = self.waiting.iter().enumerate().find_map(|(index, job)| {
            let lane = match job {
                Job::Flaky { .. } => self.free(Lane::List),
                Job::Forward { .. } => self.free(Lane::Forward),
                Job::Mac { .. } => self.free(Lane::Mac),
                Job::Search { .. } if self.searching < SEARCHES => Some(Lane::Search),
                Job::Search { test, .. } => (self.asking < ASKS
                    && !self.asked.iter().any(|asked| asked == test))
                .then_some(Lane::Ask),
            };
            lane.map(|lane| (index, lane))
        })?;
        let job = self.waiting.remove(index)?;
        self.turn(lane, true);
        Some((job, lane))
    }

    /// `lane`, if it takes one job at a time and has none under way.
    fn free(&self, lane: Lane) -> Option<Lane> {
        (!self.alone.contains(&lane)).then_some(lane)
    }

    fn turn(&mut self, lane: Lane, starts: bool) {
        match (lane, starts) {
            (Lane::Search, true) => self.searching += 1,
            (Lane::Search, false) => self.searching -= 1,
            (Lane::Ask, true) => self.asking += 1,
            (Lane::Ask, false) => self.asking -= 1,
            (Lane::List | Lane::Forward | Lane::Mac, true) => self.alone.push(lane),
            (Lane::List | Lane::Forward | Lane::Mac, false) => {
                self.alone.retain(|under_way| *under_way != lane);
            }
        }
    }
}

/// Settles what rounds hand it, beside them, on workers of its own.
pub struct Settler<'a> {
    world: &'a dyn World,
    state: &'a State,
    queue: Mutex<Queue>,
    moved: Condvar,
    /// Taken by everything that writes the hold, one at a time.
    writing: Mutex<()>,
    /// Whether a net leaves, so a write not yet begun waits behind it.
    leaving: AtomicBool,
    first_error: Mutex<Option<String>>,
}

impl<'a> Settler<'a> {
    pub fn new(world: &'a dyn World, state: &'a State) -> Self {
        Self {
            world,
            state,
            queue: Mutex::default(),
            moved: Condvar::new(),
            writing: Mutex::new(()),
            leaving: AtomicBool::new(false),
            first_error: Mutex::new(None),
        }
    }

    /// Start the workers, one for each search and ask that may run, one for
    /// lists, one for held tests' runs and one for a Mac's comparisons, in
    /// `scope`, which ends once `close` lets them finish.
    pub fn start<'scope>(&'scope self, scope: &'scope Scope<'scope, '_>) {
        for _ in 0..=SEARCHES + ASKS + 2 {
            scope.spawn(|| self.work());
        }
    }

    /// Let the workers end once every job handed is done.
    pub fn close(&self) {
        lock(&self.queue).closing = true;
        self.moved.notify_all();
    }

    /// End with `end` now, for a build installed after this one, rather than
    /// once every job handed is done: each is kept on disk, and the new build
    /// takes it up. On 10 November a watcher told at 21:32 that another build
    /// was installed tested nothing for 79 minutes, its search waiting on two
    /// 35-minute runs. `end` runs while nothing writes the hold, so none is
    /// left half written, and no job settled but still kept.
    ///
    /// It says when a write holds it and when it goes, since on 16 November
    /// a watcher told at 08:28 hung to 08:33, when it was restarted, and
    /// nothing showed whether a write held it or its own exit did.
    ///
    /// A write that only builds, before anything it decided lands, isn't
    /// waited on: its job is kept on disk, and the next build takes it up.
    /// A revert's hold says it's under way until its outcome is kept, so a
    /// job kept mid-revert settles again in full, telling the owner, rather
    /// than being dropped as held, as it does after a net is stopped. The
    /// guard on the write is held while `end` runs, so an `end` that
    /// returns, as a test's does, leaves the write stopped at its build only
    /// until the guard goes; the watch's own `end` exits.
    ///
    /// A write asked for once the leave waits goes behind it, as a worker's
    /// ending its job would otherwise take the hold each time the write it
    /// waits on lets go, and the leave, looking again only every
    /// `LOOK_AGAIN`, never would.
    pub fn leave<R>(&self, end: impl FnOnce() -> R) -> R {
        self.leaving.store(true, Ordering::SeqCst);
        let ended = self.leave_once_free(end);
        self.leaving.store(false, Ordering::SeqCst);
        ended
    }

    /// Whether a net leaves.
    pub fn leaving(&self) -> bool {
        self.leaving.load(Ordering::SeqCst)
    }

    /// End once nothing but a build writes the hold, as `leave` says.
    fn leave_once_free<R>(&self, end: impl FnOnce() -> R) -> R {
        let mut said = false;
        loop {
            let _writing = match self.writing.try_lock() {
                Ok(writing) => writing,
                Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => {
                    if let Some(_building) = self.state.only_building() {
                        self.state.say(
                            "the write to the hold only builds, and nothing it decided has \
                             landed, so this leaves, and the next build takes its job up again",
                        );
                        return end();
                    }
                    if !said {
                        self.state
                            .say("leaving waits for a write to the hold, a settle's or a round's");
                        said = true;
                    }
                    look_again();
                    continue;
                }
            };
            self.state
                .say("nothing writes the hold now, so this leaves");
            return end();
        }
    }

    /// Run `during` while the workers wait for the queue, as a worker
    /// ending a job waits for it before letting the job go.
    #[cfg(test)]
    pub fn holding_the_queue<R>(&self, during: impl FnOnce() -> R) -> R {
        let _queue = lock(&self.queue);
        during()
    }

    /// The first error a job ended in, since the settler was made.
    pub fn error(&self) -> Option<String> {
        lock(&self.first_error).clone()
    }

    /// Whether a job for `test` is waiting or under way.
    pub fn holds(&self, test: &str) -> bool {
        lock(&self.queue).held.iter().any(|held| held == test)
    }

    /// Whether a job for `test` is held or kept on disk, as one resting
    /// is, so a round handing it again would write over its rest and day.
    pub fn keeps(&self, test: &str) -> bool {
        self.holds(test) || self.state.keeps_settling(test)
    }

    /// Whether a Mac's comparison `check` at `merge` was told to its owner
    /// and let go, reverted, or let go as fixed forward, so handing it
    /// again would start another day of tries, or decide its revert again.
    /// One that failed twice had its revert decided, though a revert
    /// dropped as the check is flaky at main's newest head leaves no
    /// finding at the merge.
    pub fn told(&self, check: &str, merge: &str) -> bool {
        ["mac-unrun", "failed", "fixed-forward"]
            .into_iter()
            .any(|kind| self.state.found(kind, check, merge))
            || self
                .state
                .runs(merge, check)
                .iter()
                .filter(|ran| **ran == Ran::Failed)
                .count()
                >= super::FAILURES
    }

    /// Run `write` while nothing else writes the hold.
    pub fn write<R>(&self, write: impl FnOnce() -> R) -> R {
        while self.leaving() {
            look_again();
        }
        let _writing = lock(&self.writing);
        self.state.writing(write)
    }

    /// Run `write` unless a settle is writing the hold, for what a round
    /// only tidies and asks again at its next test or round. A settle
    /// writes the hold through its revert's runs and its landing, for half
    /// an hour or more: on 13 November #3972's revert was settled from
    /// 07:52 to 08:22, and earlier ones from 04:53 to 06:57, and every
    /// test of the round that ended meanwhile waited on it before the next
    /// could start, so a merge at 05:26 was found only at 07:56.
    pub fn write_unless_busy<R>(&self, write: impl FnOnce() -> R) -> Option<R> {
        if self.leaving() {
            return None;
        }
        let _writing = match self.writing.try_lock() {
            Ok(writing) => writing,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return None,
        };
        Some(self.state.writing(write))
    }

    /// How many merges a search's next step tries: the room `AT_ONCE` names,
    /// shared among the searches under way, and never fewer than `SPLITS`.
    /// A search alone tries twelve merges a step, and leaves a thirteenth.
    fn splits(&self) -> usize {
        (AT_ONCE / lock(&self.queue).searching.max(1)).max(SPLITS)
    }

    /// Settle `job` beside the rounds, unless one for its test is held.
    ///
    /// A failed test that finds no room to search asks whether main's
    /// newest head fixed it first, rather than waiting to. On 8 November six
    /// tests failed at once at 06fc027e, and their searches ran one after
    /// another from 10:30 to past 15:00, though five of the six were fixed
    /// forward and each was found so within one search step. It asks on a
    /// worker here, not in the round handing it on: on 13 November the
    /// round at b7a81d60bf21 ran its tests to 08:02, then five failed tests
    /// at main's newest head one by one, and ended past 09:30.
    pub fn hand(&self, job: Job) {
        if self.holds(job.test()) {
            return;
        }
        if let Err(why) = self.state.keep_settling(job.test(), &job.to_json()) {
            self.failed(why);
        }
        let mut queue = lock(&self.queue);
        if queue.held.iter().any(|held| held == job.test()) {
            return;
        }
        queue.held.push(job.test().to_owned());
        queue.waiting.push_back(job);
        drop(queue);
        self.moved.notify_all();
    }

    /// Take up the jobs kept on disk that aren't held: those a restart cut
    /// short, and those that ended in an error.
    pub fn take_up(&self) {
        for kept in self.state.settling() {
            let Some(job) = Job::from_json(&kept) else {
                continue;
            };
            // A rest is at most `RESTS.1`, so one ending further off was
            // set before the clock stepped back, and is due rather than
            // left to rest, untried and untold, for as far as it stepped.
            if kept["after"]
                .as_u64()
                .is_some_and(|after| after > now() && after <= now() + RESTS.1)
            {
                continue;
            }
            let mut queue = lock(&self.queue);
            if queue.held.iter().any(|held| held == job.test()) {
                continue;
            }
            self.state.say(&format!("taking up {} again", job.test()));
            queue.held.push(job.test().to_owned());
            queue.waiting.push_back(job);
        }
        self.moved.notify_all();
    }

    fn work(&self) {
        let mut queue = lock(&self.queue);
        loop {
            if let Some((job, lane)) = queue.next() {
                drop(queue);
                let fixed = lane == Lane::Ask && self.ask(&job);
                let done = if lane == Lane::Ask && fixed {
                    self.end(job.test(), || Ok(()))
                } else if lane == Lane::Ask {
                    Ok(())
                } else {
                    self.run(&job)
                };
                queue = lock(&self.queue);
                queue.turn(lane, false);
                if lane == Lane::Ask && !fixed {
                    // Still broken, so it waits its turn to search, first
                    // in line as it was.
                    queue.asked.push(job.test().to_owned());
                    queue.waiting.push_front(job);
                    self.moved.notify_all();
                    continue;
                }
                queue.held.retain(|held| held != job.test());
                queue.asked.retain(|asked| asked != job.test());
                if let Err(why) = done {
                    self.failed(why);
                }
                self.moved.notify_all();
                continue;
            }
            if queue.closing && queue.waiting.is_empty() {
                return;
            }
            queue = self
                .moved
                .wait(queue)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// Settle `job`. A test whose turn comes asks again whether it's left
    /// be, since the jobs before it can take an hour: on 9 November main
    /// listed launcher tests at 09:22 that had been waiting since 08:06
    /// behind other searches, each decided to be searched as it failed.
    fn run(&self, job: &Job) -> Result<(), String> {
        let (world, state) = (self.world, self.state);
        if let Job::Forward { head, newest, test } = job {
            return self.forward(head, newest, test);
        }
        if let Job::Mac {
            merge,
            check,
            since,
        } = job
        {
            return self.mac(merge, check, *since);
        }
        if listed_on_main(world, job.head(), job.test()) || self.held_for(job.test()) {
            return self.end(job.test(), || Ok(()));
        }
        match job {
            Job::Search { base, head, test } => {
                let found = search(world, state, base, head, test, &|| self.splits())?;
                if let Some(found) = &found {
                    ask_before_settling(world, state, head, found);
                }
                self.end(test, || match found {
                    Some(found) => settle(world, state, base, head, found),
                    None => Ok(()),
                })
            }
            Job::Forward { .. } | Job::Mac { .. } => Ok(()),
            Job::Flaky { head, test, owner } => {
                let listed = if state.found("flaky", test, head)
                    || world.quarantined(head).iter().any(|kept| kept == test)
                {
                    Ok(())
                } else {
                    list_flaky_unless_mended(world, state, head, test, *owner)
                };
                self.end(test, || listed)
            }
        }
    }

    /// Write what `test`'s job settled, then let the job go in the same
    /// write, since a net leaving waits only for the write. On 14 November
    /// a net left the moment its settle listed shows-then-rests, before
    /// letting the search go, and the next build searched the test again,
    /// ten VM runs. A job whose write fails is kept, to be taken up again.
    /// A hold on `test` its revert left under way goes with the job, which
    /// was the one to settle it, as one kept mid-revert and then let go for
    /// a test main fixed or listed since is.
    fn end(&self, test: &str, write: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
        self.let_go(test, || {
            write()?;
            clear_an_unsettled_revert(self.state, test)
        })
    }

    /// Write what `test`'s job settled and let the job go, as `end` does,
    /// leaving the hold to what the write decided: a fixed-forward run
    /// that fails leaves a revert's hold to its own job.
    fn let_go(&self, test: &str, write: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
        self.write(|| {
            write()?;
            self.state.settled(test)
        })
    }

    /// Run `test` at `newest` and clear the hold if it passes there, unless
    /// the hold no longer names it with no revert, or `newest` builds the
    /// very derivation the round's `head` did, which ran it already.
    fn forward(&self, head: &str, newest: &str, test: &str) -> Result<(), String> {
        let (world, state) = (self.world, self.state);
        let unreverted = state
            .hold()
            .is_some_and(|hold| hold["test"] == test && !hold["revert_commit"].is_string());
        if !unreverted
            || same_drv(world, newest, head, test)
            || attempt(world, state, newest, test) != Outcome::Passed
        {
            return self.let_go(test, || Ok(()));
        }
        let passes = format!("passes at main's newest head {}", short(newest));
        self.let_go(test, || clear_hold_of(state, test, &passes))
    }

    /// Compare a Mac's checks at `merge` as `check`, and revert the merge
    /// if they fail twice there. One no Mac took, or that compared nothing
    /// (`UNCOMPARED`), is kept, and a round after its rest takes it up
    /// again, until it was handed `UNRUN` ago: then it's told as a finding
    /// naming its PR, and let go. One a builder took that stalled twice is
    /// told the same way at once, since it compared nothing either.
    fn mac(&self, merge: &str, check: &str, since: u64) -> Result<(), String> {
        let (world, state) = (self.world, self.state);
        // A held net starts nothing, so a rested comparison taken up while
        // it's held rests again rather than being sent. Its day is moved on
        // by the rest, so a long hold doesn't use it up and leave one
        // unsent try after it told as a day with no Mac.
        if let Some(held) = world.held() {
            state.say(&format!(
                "{check} at {} waits while the net is held: {}",
                short(merge),
                held.why
            ));
            let since = since.saturating_add(RESTS.0).min(now());
            return rest(state, merge, check, since, RESTS.0);
        }
        // The merge has landed, so a run confirming a failure or a stall
        // can wait out a rest, outside whatever outage the first fell in.
        let before = state.runs(merge, check).len();
        let outcome = attempt_confirming(world, state, merge, check, Confirming::AfterARest);
        // A first failure or stall this attempt saw rests for the run that
        // confirms it even past the day, which a run of two hours begun
        // late in it can end in: told then, the merge would go unreverted
        // and a stall be told as no Mac comparing it.
        let confirms = state.runs(merge, check).len() > before;
        // A failure a Mac saw before the runs that compared nothing is the
        // owner's to read, so the finding names its log.
        let failed = state
            .runs(merge, check)
            .contains(&Ran::Failed)
            .then(|| state.log(merge, check, 1).display().to_string());
        match outcome {
            Outcome::Unsent if !confirms && now().saturating_sub(since) >= UNRUN => {
                state.say(&format!(
                    "no Mac compared {check} at {} in a day: told, and let go",
                    short(merge)
                ));
                // A Mac that took it once and stopped unfinished did
                // compare something, so cadence says so rather than that
                // no Mac compared it, naming the log the stall wrote: each
                // run appends to the log of the failures before it, plus
                // one. Named, the budget keeps it for the owner to read.
                let runs = state.runs(merge, check);
                let stalled = runs.iter().position(|ran| *ran == Ran::Stalled).map(|at| {
                    let failures = runs[..at].iter().filter(|ran| **ran == Ran::Failed).count();
                    let run = u32::try_from(failures + 1).unwrap_or(u32::MAX);
                    state.log(merge, check, run).display().to_string()
                });
                let unrun = json!({
                    "kind": "mac-unrun", "test": check, "commit": merge,
                    "owner_pr": pr_of(&world.subject(merge)), "since": since, "at": now(),
                    "failed_log": failed,
                    "stalled_once": stalled.is_some(),
                    "log": stalled,
                });
                self.end(check, || state.find(&unrun).map(|_| ()))
            }
            Outcome::Unsent => {
                let after = now().saturating_sub(since).clamp(RESTS.0, RESTS.1);
                let ran = state.runs(merge, check);
                let why = match ran.last() {
                    Some(Ran::Failed) => "a Mac failed it once, and the run that confirms it",
                    Some(Ran::Stalled) => "it stalled once on a Mac, and the next run",
                    _ => "no Mac compared it, and it",
                };
                state.say(&format!(
                    "{check} at {}: {why} is tried in {} min",
                    short(merge),
                    after / 60
                ));
                rest(state, merge, check, since, after)
            }
            Outcome::Failed => {
                let subject = world.subject(merge);
                let found = Bad {
                    commit: merge.to_owned(),
                    pr: pr_of(&subject),
                    subject,
                    test: check.to_owned(),
                    log: state.log(merge, check, 2).display().to_string(),
                };
                self.end(check, || revert_and_tell(world, state, &found, merge))
            }
            // One a builder took that stalled twice compared nothing, so
            // it's told to its owner as one no Mac compared, which cadence
            // tells, rather than let go as if it had passed, unchecked and
            // unsaid.
            Outcome::Stalled => {
                state.say(&format!(
                    "{check} at {} stalled twice on a Mac: told, and let go",
                    short(merge)
                ));
                // Each run appends to the log of the failures before it,
                // plus one, so the last stall's is one past the failure,
                // if any; named, the budget keeps it for the owner to read.
                let stalled = state.log(merge, check, if failed.is_some() { 2 } else { 1 });
                let unrun = json!({
                    "kind": "mac-unrun", "test": check, "commit": merge, "stalled": true,
                    "owner_pr": pr_of(&world.subject(merge)), "since": since, "at": now(),
                    "failed_log": failed, "log": stalled.display().to_string(),
                });
                self.end(check, || state.find(&unrun).map(|_| ()))
            }
            Outcome::Passed | Outcome::Flaky => self.end(check, || Ok(())),
        }
    }

    /// Whether main's newest head fixed `job`'s test. An error here is
    /// asked again beside the search's first step, so it's left to that.
    fn ask(&self, job: &Job) -> bool {
        let Job::Search { head, test, .. } = job else {
            return false;
        };
        fixed_forward(self.world, self.state, head, test, None) == Ok(true)
    }

    /// Whether the hold is on `test` already, so its job has nothing left
    /// to settle; one whose revert is still under way, as a net stopped
    /// mid-revert leaves it, isn't.
    fn held_for(&self, test: &str) -> bool {
        self.state
            .hold()
            .is_some_and(|hold| hold["test"] == test && hold["reverting"] != true)
    }

    fn failed(&self, why: String) {
        self.state.say(&why);
        lock(&self.first_error).get_or_insert(why);
    }
}

/// Clear the hold on `test` when its revert was begun and its outcome never
/// kept: nothing else clears it, and it holds switches with no revert.
fn clear_an_unsettled_revert(state: &State, test: &str) -> Result<(), String> {
    let unsettled = state
        .hold()
        .is_some_and(|hold| hold["test"] == test && hold["reverting"] == true);
    if unsettled {
        state.clear_hold()?;
        state.say(&format!(
            "{test}'s hold was left mid-revert, and its job ends with nothing to revert, \
             so the hold is cleared"
        ));
    }
    Ok(())
}

/// Keep a Mac's comparison of `merge` as `check`, handed `since`, to be
/// taken up again `secs` from now.
fn rest(state: &State, merge: &str, check: &str, since: u64, secs: u64) -> Result<(), String> {
    let mut kept = Job::Mac {
        merge: merge.to_owned(),
        check: check.to_owned(),
        since,
    }
    .to_json();
    kept["after"] = json!(now() + secs);
    state.keep_settling(check, &kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    // `/proc` is Linux's alone, and a Mac's dev shell builds merge-net too.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_net_leaving_reads_to_the_stall_guard_as_waiting_on_a_lock() {
        // What `/proc` says, in 10 reads 50 ms apart, that the thread
        // `during` runs on waits in, before `after` lets it end.
        fn waits_of(during: impl FnOnce() + Send + 'static, after: impl FnOnce()) -> Vec<String> {
            let (tell, told) = std::sync::mpsc::channel();
            let running = std::thread::spawn(move || {
                let own = std::fs::read_link("/proc/thread-self")
                    .ok()
                    .and_then(|link| {
                        link.file_name()
                            .map(|tid| tid.to_string_lossy().into_owned())
                    });
                tell.send(own).unwrap();
                during();
            });
            let own = told.recv().unwrap().expect("the thread has no id");
            let mut seen = Vec::new();
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(50));
                seen.extend(
                    std::fs::read_to_string(format!("/proc/self/task/{own}/syscall"))
                        .ok()
                        .map(|syscall| crate::stall::wait_in(&syscall)),
                );
            }
            after();
            running.join().unwrap();
            seen
        }
        // A thread blocked on a lock shows the guard reads it as one on
        // the kernel the test runs on.
        let lock = std::sync::Arc::new(Mutex::new(()));
        let held = lock.lock().unwrap();
        let waiting = std::sync::Arc::clone(&lock);
        let blocking = waits_of(move || drop(waiting.lock().unwrap()), || drop(held));
        assert!(
            blocking.iter().any(|wait| crate::stall::on_a_lock(wait)),
            "a thread blocked on a lock isn't read as on one: {blocking:?}"
        );
        // The round rests between rounds in a sleep, which must never read
        // as a lock, or the guard would abort a net at rest.
        let resting = waits_of(|| std::thread::sleep(Duration::from_millis(600)), || {});
        assert!(
            !resting.iter().any(|wait| crate::stall::on_a_lock(wait)),
            "a net resting between rounds reads as on a lock: {resting:?}"
        );
        let leaving = waits_of(
            || {
                for _ in 0..5 {
                    look_again();
                }
            },
            || {},
        );
        assert!(
            leaving.iter().any(|wait| crate::stall::on_a_lock(wait))
                && !leaving.iter().any(|wait| wait.contains("sleep")),
            "a leave looking again reads as resting, so a net hung on its write \
             would never be aborted: {leaving:?}"
        );
    }

    #[test]
    fn a_search_with_no_room_is_asked_once_and_then_waits() {
        let mut queue = Queue {
            waiting: (0..=SEARCHES)
                .map(|n| Job::Search {
                    base: "s".into(),
                    head: "h".into(),
                    test: format!("vm-{n}"),
                })
                .collect(),
            ..Queue::default()
        };
        for _ in 0..SEARCHES {
            let lane = queue.next().map(|(_, lane)| lane);
            assert_eq!(lane, Some(Lane::Search), "a search with room didn't start");
        }
        let (asked, lane) = queue.next().expect("a search with no room wasn't asked");
        assert_eq!(lane, Lane::Ask, "a search with no room took another lane");
        queue.turn(lane, false);
        queue.asked.push(asked.test().to_owned());
        queue.waiting.push_front(asked);
        assert!(
            queue.next().is_none(),
            "a search asked already was asked again before its turn"
        );
        queue.turn(Lane::Search, false);
        let lane = queue
            .next()
            .map(|(job, lane)| (job.test().to_owned(), lane));
        assert_eq!(
            lane,
            Some((format!("vm-{SEARCHES}"), Lane::Search)),
            "the search asked didn't search first once a search ended"
        );
    }
}
