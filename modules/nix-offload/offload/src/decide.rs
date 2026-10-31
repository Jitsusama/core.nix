//! When a builder is lent and when it's given back, apart from doing
//! either, so each rule can be pinned by a test.

use crate::journal::Event;

/// How long a builder sits unused before it's given back.
pub const IDLE: u64 = 15 * 60;
/// How long after a builder fails, or goes on its own, before another is
/// asked for, so a helper that keeps failing doesn't make one a minute.
pub const AFTER_FAILING: u64 = 30 * 60;

/// The day's hours, in seconds into the local day: lends start from
/// `first` and before `last`, and every builder is gone by `deadline`.
#[derive(Clone, Copy, Debug)]
pub struct Hours {
    pub first: u32,
    pub last: u32,
    pub deadline: u32,
}

/// What's known at one tick.
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    /// Seconds since the epoch.
    pub now: u64,
    /// Seconds into the local day.
    pub into_day: u32,
    /// Whether the queue holds back work a builder could take.
    pub wants: bool,
}

/// With nothing lent: the seconds a builder asked for now should live, if
/// one should be. None is asked for while one this asked for earlier isn't
/// yet seen gone, so there's never a second beside it.
pub fn lend(hours: Hours, tick: Tick, not_before: u64, unreturned: usize) -> Option<u64> {
    let within = (hours.first..hours.last).contains(&tick.into_day);
    (tick.wants && within && tick.now >= not_before && unreturned == 0)
        .then(|| u64::from(hours.deadline - tick.into_day))
}

/// With one lent and last used at `used`: why it's given back now, if it
/// is. `gone` is the helper having ended, which it does once the builder
/// goes on its own.
pub fn give_back(hours: Hours, tick: Tick, used: u64, gone: bool) -> Option<Event> {
    if gone {
        Some(Event::Lost)
    } else if tick.now.saturating_sub(used) >= IDLE || tick.into_day >= hours.deadline {
        Some(Event::Stopped)
    } else {
        None
    }
}

/// When the next builder may be asked for, after one last used at `used`
/// ends with `event`. One given back idle while work still `wants` it is
/// one that work couldn't use, so it waits as long as after a failure;
/// one given back in use, at the deadline, doesn't.
pub fn next_after(event: Event, wants: bool, used: u64, now: u64) -> u64 {
    match event {
        Event::Lost | Event::Failed => now + AFTER_FAILING,
        Event::Stopped if wants && now.saturating_sub(used) >= IDLE => now + AFTER_FAILING,
        _ => now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOURS: Hours = Hours {
        first: 8 * 3600,
        last: 19 * 3600 + 30 * 60,
        deadline: 20 * 3600,
    };

    fn at(into_day: u32, wants: bool) -> Tick {
        Tick {
            now: 1_000_000,
            into_day,
            wants,
        }
    }

    #[test]
    fn a_builder_is_lent_while_work_waits_within_the_days_hours() {
        assert_eq!(lend(HOURS, at(12 * 3600, true), 0, 0), Some(8 * 3600));
        assert_eq!(lend(HOURS, at(12 * 3600, false), 0, 0), None);
    }

    #[test]
    fn it_lives_to_the_deadline_to_the_second() {
        let late = 19 * 3600 + 29 * 60 + 15;
        assert_eq!(lend(HOURS, at(late, true), 0, 0), Some(45 + 30 * 60));
    }

    #[test]
    fn none_is_lent_before_the_first_start_or_from_the_last() {
        assert_eq!(lend(HOURS, at(60, true), 0, 0), None);
        assert_eq!(lend(HOURS, at(8 * 3600 - 1, true), 0, 0), None);
        assert_eq!(lend(HOURS, at(19 * 3600 + 30 * 60, true), 0, 0), None);
        assert_eq!(lend(HOURS, at(23 * 3600, true), 0, 0), None);
    }

    #[test]
    fn none_is_lent_beside_one_not_yet_seen_gone() {
        assert_eq!(lend(HOURS, at(12 * 3600, true), 0, 1), None);
    }

    #[test]
    fn none_is_lent_while_waiting_out_a_failure() {
        assert_eq!(lend(HOURS, at(12 * 3600, true), 1_000_001, 0), None);
        assert_eq!(next_after(Event::Failed, false, 5, 5), 5 + AFTER_FAILING);
        assert_eq!(next_after(Event::Lost, false, 5, 5), 5 + AFTER_FAILING);
        assert_eq!(next_after(Event::Stopped, false, 0, IDLE), IDLE);
    }

    #[test]
    fn one_given_back_unused_while_work_waits_is_not_asked_for_again_at_once() {
        let now = 100_000;
        assert_eq!(
            next_after(Event::Stopped, true, now - IDLE, now),
            now + AFTER_FAILING
        );
    }

    #[test]
    fn one_given_back_in_use_at_the_deadline_leaves_the_next_day_free_to_ask() {
        // Given back at 23:59 in use on 30 October, the next waited out
        // half an hour of the day after as if no work could use it.
        let now = 100_000;
        assert_eq!(next_after(Event::Stopped, true, now - 60, now), now);
    }

    #[test]
    fn a_builder_is_given_back_once_idle_at_the_deadline_or_once_gone() {
        let noon = at(12 * 3600, false);
        assert_eq!(give_back(HOURS, noon, noon.now - 60, false), None);
        assert_eq!(
            give_back(HOURS, noon, noon.now - IDLE, false),
            Some(Event::Stopped)
        );
        assert_eq!(
            give_back(HOURS, at(20 * 3600, true), 1_000_000, false),
            Some(Event::Stopped)
        );
        assert_eq!(give_back(HOURS, noon, noon.now, true), Some(Event::Lost));
    }
}
