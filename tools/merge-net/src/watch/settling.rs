//! What a round found, settled beside the rounds after it: the search for
//! the merge that broke a test, and the PR that lists a flaky one.
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

use serde_json::{Value, json};

use super::{
    AT_ONCE, SEARCHES, SPLITS, World, fixed_forward, list_flaky, listed_on_main, lock, search,
    settle,
};
use crate::state::State;

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
}

impl Job {
    fn test(&self) -> &str {
        match self {
            Self::Search { test, .. } | Self::Flaky { test, .. } => test,
        }
    }

    fn head(&self) -> &str {
        match self {
            Self::Search { head, .. } | Self::Flaky { head, .. } => head,
        }
    }

    fn is_search(&self) -> bool {
        matches!(self, Self::Search { .. })
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Search { base, head, test } => {
                json!({ "kind": "search", "base": base, "head": head, "test": test })
            }
            Self::Flaky { head, test, owner } => {
                json!({ "kind": "flaky", "head": head, "test": test, "owner": owner })
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
            _ => None,
        }
    }
}

/// The jobs waiting and under way.
#[derive(Default)]
struct Queue {
    waiting: VecDeque<Job>,
    /// The tests of every job waiting or under way, one job a test.
    held: Vec<String>,
    searching: usize,
    listing: bool,
    closing: bool,
}

impl Queue {
    /// The first job waiting that may start: up to `SEARCHES` searches,
    /// and one list at a time, since two lists' PRs change one file.
    fn next(&mut self) -> Option<Job> {
        let index = self.waiting.iter().position(|job| {
            if job.is_search() {
                self.searching < SEARCHES
            } else {
                !self.listing
            }
        })?;
        let job = self.waiting.remove(index)?;
        self.turn(&job, true);
        Some(job)
    }

    fn turn(&mut self, job: &Job, starts: bool) {
        if job.is_search() {
            if starts {
                self.searching += 1;
            } else {
                self.searching -= 1;
            }
        } else {
            self.listing = starts;
        }
    }

    fn searches(&self) -> usize {
        self.searching + self.waiting.iter().filter(|job| job.is_search()).count()
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

    /// Start the workers, one for each search that may run and one for
    /// lists, in `scope`, which ends once `close` lets them finish.
    pub fn start<'scope>(&'scope self, scope: &'scope Scope<'scope, '_>) {
        for _ in 0..=SEARCHES {
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
    /// left half written.
    pub fn leave<R>(&self, end: impl FnOnce() -> R) -> R {
        self.write(end)
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
        write()
    }

    /// How many merges a search's next step tries: the room `AT_ONCE` names,
    /// shared among the searches under way, and never fewer than `SPLITS`.
    /// A search alone tries twelve merges a step, and leaves a thirteenth.
    fn splits(&self) -> usize {
        (AT_ONCE / lock(&self.queue).searching.max(1)).max(SPLITS)
    }

    /// Settle `job` beside the rounds, unless one for its test is held.
    ///
    /// A failed test that finds no room asks whether main's newest head
    /// fixed it first, rather than waiting to. On 8 November six tests
    /// failed at once at 06fc027e, and their searches ran one after another
    /// from 10:30 to past 15:00, though five of the six were fixed forward
    /// and each was found so within one search step.
    pub fn hand(&self, job: Job) {
        if self.holds(job.test()) {
            return;
        }
        if let Job::Search { head, test, .. } = &job
            && lock(&self.queue).searches() >= SEARCHES
            // An error here is asked again beside the search's first step,
            // so it's left to that.
            && fixed_forward(self.world, self.state, head, test, None) == Ok(true)
        {
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
            if let Some(job) = queue.next() {
                drop(queue);
                let done = self.run(&job);
                queue = lock(&self.queue);
                queue.turn(&job, false);
                queue.held.retain(|held| held != job.test());
                match done {
                    Ok(()) => {
                        if let Err(why) = self.state.settled(job.test()) {
                            self.failed(why);
                        }
                    }
                    Err(why) => self.failed(why),
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
        if listed_on_main(world, job.head(), job.test()) || self.held_for(job.test()) {
            return Ok(());
        }
        match job {
            Job::Search { base, head, test } => {
                let found = search(world, state, base, head, test, &|| self.splits())?;
                self.write(|| match found {
                    Some(found) => settle(world, state, base, head, found),
                    None => Ok(()),
                })
            }
            Job::Flaky { head, test, owner } => {
                if state.found("flaky", test, head)
                    || world.quarantined(head).iter().any(|kept| kept == test)
                {
                    return Ok(());
                }
                list_flaky(world, state, head, test, *owner)
            }
        }
    }

    /// Whether the hold names `test` already.
    fn held_for(&self, test: &str) -> bool {
        self.state.hold().is_some_and(|hold| hold["test"] == test)
    }

    fn failed(&self, why: String) {
        self.state.say(&why);
        lock(&self.first_error).get_or_insert(why);
    }
}
