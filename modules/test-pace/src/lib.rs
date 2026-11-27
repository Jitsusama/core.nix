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
//! The load average is the host's, a build sandbox not hiding it. Where
//! it can't be read, as on a Mac, the pace is one and the bound is the
//! idle one plus the slack.

use std::fs;
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
const LOAD_AVERAGE: &str = "/proc/loadavg";

/// A timed step of a test, begun when it was started.
#[derive(Debug, Clone, Copy)]
pub struct Timed {
    began: Instant,
    busy: f64,
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
        }
    }

    /// How long the step has taken so far.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.began.elapsed()
    }

    /// How long the step may take, given the bound an idle host meets: the
    /// bound stretched by the busier of the host's pace as the step
    /// started and its pace now, plus [`SLACK`].
    #[must_use]
    pub fn allowed(&self, idle: Duration) -> Duration {
        allowance(idle, self.busy.max(busyness()))
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
    fs::read_to_string(LOAD_AVERAGE)
        .ok()
        .and_then(|said| ready_each_minute(&said))
        .map_or(0.0, |ready| ready / f64::from(processors))
}

/// The tasks ready to run over the last minute, from what
/// `/proc/loadavg` says.
#[must_use]
pub fn ready_each_minute(said: &str) -> Option<f64> {
    said.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests;
