//! An instant on the kernel's boot clock, which counts the machine's
//! sleep, for every judgement of how long ago something in the air
//! happened: a point kept off an hour, a give-way held a settle, an
//! access point last heard so long ago. tokio's `Instant` is of the
//! monotonic clock, which stops while the machine sleeps, so a night
//! asleep would read as no time at all, and a point kept off an hour
//! before a suspend would still be kept off for most of that hour after
//! waking. Timers that only drive an action (a sleep, an arrival's
//! deadline) stay on tokio's clock, which is the one its timers wake by:
//! a deadline on this clock would wake late by as long as the machine
//! slept.
//!
//! A type of its own rather than a `Duration`, so the compiler refuses a
//! boot instant where tokio's is meant, and the other way round.

use std::ops::{Add, Sub};

use tokio::time::Duration;

use crate::learn;

/// A time on the boot clock, as how long the machine had been up when it
/// was read, its sleep counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(Duration);

impl Instant {
    /// The boot clock now (`learn::booted`).
    pub fn now() -> Instant {
        Instant(learn::booted())
    }

    /// How long the machine had been up at this instant, its sleep
    /// counted, as what's learned is timed by (learn.rs).
    pub fn since_boot(self) -> Duration {
        self.0
    }

    /// How long after `earlier` this is, or none where it's before, as
    /// tokio's `Instant` saturates.
    pub fn duration_since(self, earlier: Instant) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    /// The same as `duration_since`, named as tokio names it, so a reader
    /// sees it saturates.
    pub fn saturating_duration_since(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }

    /// The instant `back` before this one, unless that's before boot.
    pub fn checked_sub(self, back: Duration) -> Option<Instant> {
        self.0.checked_sub(back).map(Instant)
    }

    /// How long ago this was, by the boot clock now.
    pub fn elapsed(self) -> Duration {
        Instant::now().duration_since(self)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, later: Duration) -> Instant {
        Instant(self.0 + later)
    }
}

/// Panics where the result would be before boot, as tokio's `Instant`
/// panics before its clock's start: no instant wifid reads is that old.
impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, back: Duration) -> Instant {
        self.checked_sub(back)
            .unwrap_or_else(|| panic!("{back:?} before {self:?} is before boot"))
    }
}

/// How long after `earlier` this is, or none where it's before, as
/// `duration_since`.
impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instant_is_the_boot_clock_read_and_goes_on_from_it() {
        let before = learn::booted();
        let now = Instant::now();
        assert!(
            before <= now.since_boot() && now.since_boot() <= learn::booted(),
            "now read {now:?}, not the boot clock between {before:?} and after"
        );
        assert!(Instant::now() >= now, "the boot clock went back");
    }

    #[test]
    fn its_arithmetic_saturates_as_tokio_s_does_and_refuses_before_boot() {
        let now = Instant::now();
        let later = now + Duration::from_mins(5);
        assert_eq!(
            later - now,
            Duration::from_mins(5),
            "five minutes on read otherwise"
        );
        assert_eq!(
            now - later,
            Duration::ZERO,
            "an earlier instant read as later"
        );
        assert_eq!(
            now.saturating_duration_since(later),
            Duration::ZERO,
            "saturating read an earlier instant as later"
        );
        assert_eq!(
            later - Duration::from_mins(5),
            now,
            "five minutes back read otherwise"
        );
        assert_eq!(
            now.checked_sub(now.since_boot() + Duration::from_secs(1)),
            None,
            "an instant before boot was made"
        );
    }
}
