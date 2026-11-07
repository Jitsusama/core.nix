//! When the feed reads next. It keeps to the pace spend-watch sets, so the
//! plan is read often only while pi's logs grow, reads at once when the
//! person asks, and reads further apart while the endpoint fails or asks
//! for a pause, since asking more often then only lengthens the pause.

use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

use crate::feed::Outcome;

/// How busy spend-watch says the account is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// pi's logs grew in the last ten minutes.
    Active,
    /// They haven't, or the watch said a pace this feed doesn't know.
    Idle,
}

impl Pace {
    fn interval(self) -> SignedDuration {
        match self {
            Pace::Active => SignedDuration::from_secs(60),
            Pace::Idle => SignedDuration::from_secs(600),
        }
    }
}

/// The longest the feed goes between reads, however long it has failed.
const LONGEST: SignedDuration = SignedDuration::from_secs(3600);

/// When the next read is due, from the pace and how the reads have gone.
#[derive(Debug, Clone)]
pub struct Schedule {
    pace: Pace,
    due: Timestamp,
    last: Option<Timestamp>,
    failures: u32,
}

impl Schedule {
    /// A schedule whose first read is due at `start`, before any pace is
    /// said, so a feed starting says how the plan stands at once.
    pub fn new(start: Timestamp) -> Schedule {
        Schedule {
            pace: Pace::Idle,
            due: start,
            last: None,
            failures: 0,
        }
    }

    pub fn due(&self) -> Timestamp {
        self.due
    }

    /// How long from `now` until the next read, none once it's due.
    pub fn wait(&self, now: Timestamp) -> Duration {
        Duration::try_from(self.due.duration_since(now)).unwrap_or(Duration::ZERO)
    }

    /// Takes a new pace. While backing off, the wait stands: a busier
    /// account doesn't make a failing endpoint any likelier to answer.
    pub fn pace(&mut self, pace: Pace) {
        self.pace = pace;
        if let (Some(last), 0) = (self.last, self.failures) {
            self.due = last + pace.interval();
        }
    }

    /// The person asked for the latest figures, so the next read is now.
    /// A pause the endpoint asked for is the read's to keep, which says so.
    pub fn refresh(&mut self, now: Timestamp) {
        self.due = now;
    }

    /// Notes how a read at `now` went, and when the next is due.
    pub fn read(&mut self, outcome: &Outcome, now: Timestamp) {
        self.last = Some(now);
        self.due = match outcome {
            Outcome::Answered | Outcome::Unasked => {
                self.failures = 0;
                now + self.pace.interval()
            }
            Outcome::Failed => now + self.backed_off(),
            Outcome::Paused(until) => (*until).max(now + self.backed_off()),
            // Nothing was asked, so nothing failed: a refresh pressed while
            // waiting leaves the backing off as it was.
            Outcome::Waiting(until) => *until,
        };
    }

    /// One more failure's wait: the pace's interval doubled for each
    /// failure in a row, up to the longest.
    fn backed_off(&mut self) -> SignedDuration {
        self.failures = self.failures.saturating_add(1);
        let doubled = 2_i32.saturating_pow(self.failures.min(30));
        self.pace
            .interval()
            .checked_mul(doubled)
            .map_or(LONGEST, |wait| wait.min(LONGEST))
    }
}
