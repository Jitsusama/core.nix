//! How long a read spent on each kind it read, so health can say where a
//! slow read's time went rather than only that it was slow: a read of
//! Heat at a busy moment took 38 to 149 ms where most take 5 to 8, and
//! which of its chips, zones or fans held it was guessed, not seen.

use std::collections::BTreeMap;
use std::time::Instant;

/// Each kind's share of one read, in microseconds, timed from where the
/// kind before it ended, or from where the read began.
pub struct Spent {
    from: Instant,
    by_kind: BTreeMap<&'static str, u64>,
}

impl Spent {
    /// A read beginning at `began`.
    #[must_use]
    pub fn from(began: Instant) -> Self {
        Self {
            from: began,
            by_kind: BTreeMap::new(),
        }
    }

    /// Counts the time from where the last kind ended to `now` as this
    /// kind's, adding to what it spent already where it's read in parts.
    pub fn ended(&mut self, kind: &'static str, now: Instant) {
        let took = now.saturating_duration_since(self.from).as_micros();
        *self.by_kind.entry(kind).or_default() += u64::try_from(took).unwrap_or(u64::MAX);
        self.from = now;
    }

    /// What each kind read spent.
    #[must_use]
    pub fn by_kind(self) -> BTreeMap<&'static str, u64> {
        self.by_kind
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn each_kind_spends_from_where_the_one_before_ended() {
        let began = Instant::now();
        let at = |ms| began + Duration::from_millis(ms);
        let mut spent = Spent::from(began);
        spent.ended("sensor", at(3));
        spent.ended("zone", at(10));
        spent.ended("sensor", at(12));
        assert_eq!(
            spent.by_kind(),
            BTreeMap::from([("sensor", 5000), ("zone", 7000)])
        );
    }
}
