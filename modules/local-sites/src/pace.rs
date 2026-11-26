//! How often something costly may be done for whoever asks: a few at once,
//! then one each time a spell passes. The server asks the kernel who pressed
//! (peer.rs walks every descriptor the account holds) and journals why it
//! refused, both for any process on the loopback network, which can ask as
//! fast as it connects; paced, a flood costs a little and is said once.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// A few at once, then one each `every`, counting those turned away since
/// the last let through.
pub struct Pace {
    most: u32,
    every: Duration,
    held: Mutex<Held>,
}

struct Held {
    left: u32,
    since: Instant,
    turned: u64,
}

/// Whether one may go now, and how many were turned away before it.
#[derive(Debug, PartialEq, Eq)]
pub enum Turn {
    Go { turned: u64 },
    Wait,
}

impl Pace {
    /// `most` at once, one more each `every` up to that.
    pub fn new(most: u32, every: Duration) -> Self {
        Self {
            most,
            every,
            held: Mutex::new(Held {
                left: most,
                since: Instant::now(),
                turned: 0,
            }),
        }
    }

    /// Whether one may go now.
    pub fn take(&self) -> Turn {
        self.take_at(Instant::now())
    }

    fn take_at(&self, now: Instant) -> Turn {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        // Each whole spell since the last counted gives one back, the
        // spells' start kept so a part spell isn't lost.
        let spells =
            now.saturating_duration_since(held.since).as_nanos() / self.every.as_nanos().max(1);
        if spells > 0 {
            let back = u32::try_from(spells).unwrap_or(u32::MAX);
            held.left = held.left.saturating_add(back).min(self.most);
            held.since = if held.left == self.most {
                now
            } else {
                held.since + self.every * back
            };
        }
        if held.left == 0 {
            held.turned = held.turned.saturating_add(1);
            return Turn::Wait;
        }
        held.left -= 1;
        Turn::Go {
            turned: std::mem::take(&mut held.turned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPELL: Duration = Duration::from_millis(250);

    #[test]
    fn a_few_go_at_once_then_one_each_spell_saying_how_many_waited() {
        let pace = Pace::new(3, SPELL);
        let start = Instant::now();
        for _ in 0..3 {
            assert_eq!(
                pace.take_at(start),
                Turn::Go { turned: 0 },
                "one of the first few waited"
            );
        }
        assert_eq!(pace.take_at(start), Turn::Wait, "a fourth at once went");
        assert_eq!(
            pace.take_at(start + SPELL / 2),
            Turn::Wait,
            "one went inside a spell"
        );
        assert_eq!(
            pace.take_at(start + SPELL),
            Turn::Go { turned: 2 },
            "a spell on, none went, or the two turned away weren't said"
        );
        assert_eq!(
            pace.take_at(start + SPELL),
            Turn::Wait,
            "two went for one spell"
        );
    }

    #[test]
    fn a_long_quiet_gives_back_no_more_than_the_few() {
        let pace = Pace::new(2, SPELL);
        let start = Instant::now();
        let later = start + SPELL * 1000;
        assert_eq!(
            pace.take_at(start),
            Turn::Go { turned: 0 },
            "the first waited"
        );
        for _ in 0..2 {
            assert_eq!(
                pace.take_at(later),
                Turn::Go { turned: 0 },
                "the few weren't given back"
            );
        }
        assert_eq!(
            pace.take_at(later),
            Turn::Wait,
            "a quiet gave back more than the few"
        );
    }

    #[test]
    fn a_part_spell_while_refilling_counts_toward_the_next() {
        let pace = Pace::new(2, SPELL);
        let start = Instant::now();
        for _ in 0..2 {
            assert_eq!(
                pace.take_at(start),
                Turn::Go { turned: 0 },
                "one of the first two waited"
            );
        }
        assert_eq!(
            pace.take_at(start + SPELL * 3 / 2),
            Turn::Go { turned: 0 },
            "a spell and a half on, none went"
        );
        assert_eq!(
            pace.take_at(start + SPELL * 2),
            Turn::Go { turned: 0 },
            "the half spell was lost, so the next waited a whole one"
        );
    }
}
