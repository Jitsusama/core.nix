//! How long a timed step of a test may take on a host as busy as this one.
//!
//! A test that bounds how long something took, "the desktop was asked, so
//! it ended within two seconds", states the time an idle host takes. On a
//! builder running sixteen builds, or a laptop with forty tasks ready for
//! twelve processors, every process waits its turn, the one under test
//! and the test itself, and the same step takes several times as long.
//! A bound stated at idle then fails on a change that never touched it.
//!
//! So a timed step is started with [`Timed::start`], and judged with
//! [`Timed::within`] against the bound an idle host meets. The bound is
//! stretched by the host's pace, as `tests/lib/vm_helpers.py` stretches a
//! VM test's waits: how many tasks were ready to run each minute, per
//! processor, read as the step starts and again as it's judged, the
//! busier of the two counting, never below one and at most
//! [`MOST_PACE`]. [`SLACK`] is added, since a step of a few milliseconds
//! can lose that much to one turn of the scheduler that no average sees.
//!
//! Many timed steps tell right behaviour from wrong by time alone: asked,
//! the desktop ends at once, where one that ignores being asked waits out
//! a fixed five seconds' grace. Stretching such a bound past five seconds
//! would pass the wrong behaviour too, so a step like that is started with
//! [`Timed::short_of`], naming the fixed time the wrong behaviour takes,
//! and its bound stretches only to [`SLACK`] short of it.
//!
//! The load average is the host's. Linux says it in `/proc/loadavg`, which
//! a build sandbox doesn't hide; a Mac says it to `getloadavg(3)`, read in
//! the process, as Python's `os.getloadavg`, which `vm_helpers` reads,
//! takes it on each. Where it can't be read the pace is one and the bound
//! is the idle one plus the slack.

use std::thread;
use std::time::{Duration, Instant};

/// The most times longer than its idle bound a step may take on a busy
/// host, as `MOST_PACE` in `tests/lib/vm_helpers.py`. Past this the host
/// is so oversubscribed that a test should fail, in a time still bounded,
/// rather than pass whatever the step took.
pub const MOST_PACE: f64 = 4.0;

/// What every bound allows on top of its paced time: a turn or two of the
/// scheduler, which a step of milliseconds can lose on any host.
pub const SLACK: Duration = Duration::from_millis(250);

/// Where Linux says how many tasks were ready to run, over the last
/// minute first.
#[cfg(not(target_os = "macos"))]
const LOAD_AVERAGE: &str = "/proc/loadavg";

/// A timed step of a test, begun when it was started.
#[derive(Debug, Clone, Copy)]
pub struct Timed {
    began: Instant,
    busy: f64,
    wrong: Option<Duration>,
}

impl Timed {
    /// Start timing a step, reading how busy the host is as it starts.
    #[must_use]
    pub fn start() -> Self {
        // Read before the clock starts, so the reading isn't part of the
        // step it judges.
        Self {
            busy: busyness(),
            began: Instant::now(),
            wrong: None,
        }
    }

    /// Hold the step's bound short of the fixed time the behaviour it
    /// tells apart from takes, however busy the host: a stretched bound
    /// stops [`SLACK`] short of `wrong`, so the wrong behaviour fails on
    /// any host. An idle bound already past that is left as it is.
    #[must_use]
    pub fn short_of(self, wrong: Duration) -> Self {
        Self {
            wrong: Some(wrong),
            ..self
        }
    }

    /// How long the step has taken so far.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.began.elapsed()
    }

    /// How long the step may take, given the bound an idle host meets: the
    /// bound stretched by the busier of the host's pace as the step
    /// started and its pace now, plus [`SLACK`], and held short of the
    /// wrong behaviour's time where [`Timed::short_of`] named one.
    #[must_use]
    pub fn allowed(&self, idle: Duration) -> Duration {
        held_short(allowance(idle, self.busy.max(busyness())), idle, self.wrong)
    }

    /// Whether the step took no longer than it may on this host, given the
    /// bound an idle host meets.
    #[must_use]
    pub fn within(&self, idle: Duration) -> bool {
        let allowed = self.allowed(idle);
        self.elapsed() <= allowed
    }

    /// Say how long the step took against how long it may, for a failed
    /// assert's message.
    #[must_use]
    pub fn said(&self, idle: Duration) -> String {
        format!(
            "took {:?}, where {idle:?} is allowed at idle and {:?} on this machine",
            self.elapsed(),
            self.allowed(idle)
        )
    }
}

/// How long a step may take on a host this busy, given the bound an idle
/// host meets.
#[must_use]
pub fn allowance(idle: Duration, busy: f64) -> Duration {
    idle.mul_f64(pace(busy)) + SLACK
}

/// A stretched bound held [`SLACK`] short of the time the wrong behaviour
/// takes, where there is one, but never below the idle bound.
#[must_use]
pub fn held_short(allowed: Duration, idle: Duration, wrong: Option<Duration>) -> Duration {
    wrong.map_or(allowed, |wrong| {
        allowed.min(wrong.saturating_sub(SLACK)).max(idle)
    })
}

/// How many times longer than its idle bound a step may take on a host
/// this busy. On a host with a processor to spare for each task a step
/// takes its idle time; past that each task waits its turn behind the
/// others, so its time grows with them, up to [`MOST_PACE`].
#[must_use]
pub fn pace(busy: f64) -> f64 {
    if busy.is_nan() {
        return 1.0;
    }
    busy.clamp(1.0, MOST_PACE)
}

/// How many tasks the host has had ready to run each minute, per
/// processor, or none where that can't be read.
#[must_use]
pub fn busyness() -> f64 {
    let processors = thread::available_parallelism()
        .ok()
        .and_then(|count| u32::try_from(count.get()).ok())
        .unwrap_or(1);
    ready_last_minute().map_or(0.0, |ready| ready / f64::from(processors))
}

/// How many tasks were ready to run over the last minute, or none where
/// the host won't say.
#[cfg(not(target_os = "macos"))]
fn ready_last_minute() -> Option<f64> {
    // A host with no load average to read, as a sandbox without /proc,
    // says nothing of its load, so the bound keeps its idle value.
    ready_each_minute(&std::fs::read_to_string(LOAD_AVERAGE).ok()?)
}

/// How many tasks were ready to run over the last minute, or none where
/// the host won't say. A Mac says it to `getloadavg(3)`, from inside the
/// process, so a build sandbox that can't run `sysctl` still reads it and
/// a step judged every few milliseconds starts no process to do so.
#[cfg(target_os = "macos")]
fn ready_last_minute() -> Option<f64> {
    unsafe extern "C" {
        fn getloadavg(averages: *mut f64, count: std::ffi::c_int) -> std::ffi::c_int;
    }
    let mut averages = [0.0_f64; 1];
    // SAFETY: getloadavg writes at most `count` doubles into `averages`,
    // which holds exactly one, and keeps no pointer past the call.
    let said = unsafe { getloadavg(averages.as_mut_ptr(), 1) };
    (said == 1).then_some(averages[0])
}

/// The tasks ready to run over the last minute, from what Linux's
/// `/proc/loadavg` says, the first of its words.
#[must_use]
pub fn ready_each_minute(said: &str) -> Option<f64> {
    said.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests;
