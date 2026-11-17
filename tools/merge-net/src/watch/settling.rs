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

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::thread::Scope;
use std::time::Duration;

use serde_json::{Value, json};

use super::{
    AT_ONCE, Bad, Outcome, SEARCHES, SPLITS, World, ask_before_settling, attempt, clear_hold_of,
    fixed_forward, list_flaky_unless_mended, listed_on_main, lock, pr_of, revert_and_tell,
    same_drv, search, settle,
};
use crate::state::{State, now, short};

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

/// How often a net leaving looks again whether the write it waits on only
/// builds or has ended.
const LOOK_AGAIN: Duration = Duration::from_millis(200);

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
    /// A hold whose revert it cuts is marked cut, so that job settles again
    /// in full, telling the owner, rather than being dropped as held; when
    /// the mark can't be written, the leave waits as for any write. The
    /// guard on the write is held while `end` runs, so an `end` that
    /// returns, as a test's does, leaves the write stopped at its build only
    /// until the guard goes; the watch's own `end` exits.
    pub fn leave<R>(&self, end: impl FnOnce() -> R) -> R {
        let mut said = false;
        loop {
            let _writing = match self.writing.try_lock() {
                Ok(writing) => writing,
                Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => {
                    if let Some(_building) = self.state.only_building()
                        && mark_a_cut_revert(self.state)
                    {
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
                    std::thread::sleep(LOOK_AGAIN);
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

    /// Run `write` while nothing else writes the hold.
    pub fn write<R>(&self, write: impl FnOnce() -> R) -> R {
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
    fn end(&self, test: &str, write: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
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
            return self.end(test, || Ok(()));
        }
        let passes = format!("passes at main's newest head {}", short(newest));
        self.end(test, || clear_hold_of(state, test, &passes))
    }

    /// Compare a Mac's checks at `merge` as `check`, and revert the merge
    /// if they fail twice there. One no Mac took is kept, and the next
    /// round takes it up again, until it was handed `UNRUN` ago: then it's
    /// told as a finding naming its PR, and let go.
    fn mac(&self, merge: &str, check: &str, since: u64) -> Result<(), String> {
        let (world, state) = (self.world, self.state);
        match attempt(world, state, merge, check) {
            Outcome::Unsent if now().saturating_sub(since) >= UNRUN => {
                state.say(&format!(
                    "no Mac took {check} at {} in a day: told, and let go",
                    short(merge)
                ));
                let unrun = json!({
                    "kind": "mac-unrun", "test": check, "commit": merge,
                    "owner_pr": pr_of(&world.subject(merge)), "since": since, "at": now(),
                });
                self.end(check, || state.find(&unrun).map(|_| ()))
            }
            Outcome::Unsent => {
                state.say(&format!(
                    "no Mac took {check} at {}: it's tried again next round",
                    short(merge)
                ));
                Ok(())
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
            Outcome::Passed | Outcome::Flaky | Outcome::Stalled => self.end(check, || Ok(())),
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

    /// Whether the hold names `test` already.
    /// Whether the hold is on `test` already, so its job has nothing left
    /// to settle; a hold a leave cut short of its revert isn't.
    fn held_for(&self, test: &str) -> bool {
        self.state
            .hold()
            .is_some_and(|hold| hold["test"] == test && hold["cut"] != true)
    }

    fn failed(&self, why: String) {
        self.state.say(&why);
        lock(&self.first_error).get_or_insert(why);
    }
}

/// Mark the hold cut when a leave cuts its revert short, before its outcome
/// is kept, so the job kept for it settles again in full at the next build,
/// telling the owner or quarantining as a first revert does. Whether the
/// leave may end: not while a mark it needs can't be written.
fn mark_a_cut_revert(state: &State) -> bool {
    let Some(mut hold) = state.hold() else {
        return true;
    };
    if !hold["revert_commit"].is_null() || hold.get("revert_again").is_some() {
        return true;
    }
    hold["cut"] = json!(true);
    match state.set_hold(&hold) {
        Ok(()) => true,
        Err(why) => {
            state.say(&format!(
                "the cut revert's hold isn't marked, so this waits: {why}"
            ));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
