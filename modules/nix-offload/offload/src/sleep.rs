//! Whether the machine slept lately, so a builder that went while it did
//! isn't taken for one that failed.
//!
//! The kernel's time since boot, in /proc/uptime, counts a suspend, and the
//! monotonic clock behind `Instant` doesn't, so a sleep shows as the first
//! gaining on the second between two readings. Where /proc/uptime can't be
//! read, no sleep is ever seen, which is how it was before.

use std::fs;
use std::time::Instant;

/// How much the clocks may drift apart between readings without a sleep.
const DRIFT: f64 = 30.0;

/// How long after waking it counts as just woken: a helper looks at its
/// builder once a minute, and asking the cloud can take as long again.
const WAKING: u64 = 5 * 60;

/// Reads the two clocks at each tick and keeps when the machine last woke.
#[derive(Default)]
pub struct Sleep {
    last: Option<(f64, Instant)>,
    woke: Option<u64>,
}

impl Sleep {
    /// Whether the machine woke within the last few minutes, as of `now` in
    /// seconds since the epoch.
    pub fn woke(&mut self, now: u64) -> bool {
        let reading = uptime().map(|up| (up, Instant::now()));
        if let (Some((before, then)), Some((up, at))) = (self.last, reading)
            && slept(up - before, at.duration_since(then).as_secs_f64())
        {
            self.woke = Some(now);
        }
        self.last = reading;
        just_woke(self.woke, now)
    }
}

/// Whether a reading's time since boot gained `since_boot` seconds against
/// `monotonic` by more than drift explains.
fn slept(since_boot: f64, monotonic: f64) -> bool {
    since_boot - monotonic > DRIFT
}

/// Whether `woke`, if it did, was recent enough at `now`.
fn just_woke(woke: Option<u64>, now: u64) -> bool {
    woke.is_some_and(|at| now.saturating_sub(at) <= WAKING)
}

/// Seconds since boot, suspends included.
fn uptime() -> Option<f64> {
    fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gain_past_drift_is_a_sleep_and_drift_is_not() {
        // Asleep from 23:31 to 09:35, the boot clock gained ten hours.
        assert!(slept(36_283.0 + 20.0, 20.0));
        assert!(!slept(20.4, 20.0));
        assert!(!slept(20.0, 20.0 + 900.0));
    }

    #[test]
    fn just_woken_lasts_a_few_minutes() {
        assert!(just_woke(Some(1_000), 1_000 + WAKING));
        assert!(!just_woke(Some(1_000), 1_001 + WAKING));
        assert!(!just_woke(None, 1_000));
    }

    #[test]
    fn a_machine_that_never_slept_never_woke() {
        let mut sleep = Sleep::default();
        assert!(!sleep.woke(1_000));
        assert!(!sleep.woke(1_020));
    }
}
