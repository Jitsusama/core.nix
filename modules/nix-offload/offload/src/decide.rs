//! When a builder is lent and when it's given back, apart from doing
//! either, so each rule can be pinned by a test.

use crate::journal::Event;

/// How long a builder sits unused before it's given back.
pub const IDLE: u64 = 15 * 60;
/// How long after a builder fails, or goes on its own, before another is
/// asked for, so a helper that keeps failing doesn't make one a minute.
pub const AFTER_FAILING: u64 = 30 * 60;
/// How near its end a builder that goes on its own ended on time, rather
/// than being lost: one asked to live until 23:59 on 2 November was
/// deleted at 23:55:43, and counted lost, which lent none for 30 minutes.
pub const NEAR_ITS_END: u64 = 10 * 60;

/// The day's hours, in seconds into the local day: lends start from
/// `first` and before `last`, and every builder is gone by `deadline`,
/// unless each lives `longest` from its asking instead.
#[derive(Clone, Copy, Debug)]
pub struct Hours {
    pub first: u32,
    pub last: u32,
    pub deadline: u32,
    /// The longest a builder lives from its asking, in seconds, in place
    /// of the deadline, so one lent at any hour isn't ended mid-build at
    /// midnight, as two were on 2 November.
    pub longest: Option<u32>,
}

/// When a builder asked for at `asked` ends, as seen `now`, `into_day`
/// seconds into the local day.
pub fn ends(hours: Hours, asked: u64, now: u64, into_day: u32) -> u64 {
    match hours.longest {
        Some(longest) => asked + u64::from(longest),
        None => now + u64::from(hours.deadline.saturating_sub(into_day)),
    }
}

/// Whether the day's deadline has come for every builder.
pub fn past_deadline(hours: Hours, into_day: u32) -> bool {
    hours.longest.is_none() && into_day >= hours.deadline
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
    /// Whether the machine woke from sleep in the last few minutes, long
    /// enough for a helper to notice a builder that went while it slept.
    pub woke: bool,
}

/// With nothing lent: the seconds a builder asked for now should live, if
/// one should be. None is asked for while one this asked for earlier isn't
/// yet seen gone, so there's never a second beside it.
pub fn lend(hours: Hours, tick: Tick, not_before: u64, unreturned: usize) -> Option<u64> {
    let within = (hours.first..hours.last).contains(&tick.into_day);
    (tick.wants && within && tick.now >= not_before && unreturned == 0)
        .then(|| u64::from(hours.longest.unwrap_or(hours.deadline - tick.into_day)))
}

/// With one lent and last used at `used`, asked to live until `ends`: why
/// it's given back now, if it is. `gone` is the helper having ended, which
/// it does once the builder goes on its own. One whose time ran out ended
/// on time, gone or not, since a machine that slept past it sees it only
/// on waking, and the day's deadline alone can't say so once it's tomorrow.
/// One gone just after a sleep went unused, as its own idle delete takes
/// it, so it isn't lost either.
pub fn give_back(hours: Hours, tick: Tick, used: u64, ends: u64, gone: bool) -> Option<Event> {
    let over = tick.now >= ends || past_deadline(hours, tick.into_day);
    let near_its_end = tick.now + NEAR_ITS_END >= ends;
    if gone && !over && !near_its_end && !tick.woke {
        Some(Event::Lost)
    } else if over || gone || tick.now.saturating_sub(used) >= IDLE {
        Some(Event::Stopped)
    } else {
        None
    }
}

/// Whether a builder last used at `used`, asked to live until `ends`, is
/// asked whether a run outliving its caller still builds there: only once
/// it would otherwise be given back idle, so it's asked once each idle
/// spell rather than every tick, and never once its time is up, since
/// nothing holds a builder past that. With its caller gone, no ssh here
/// reaches it, and a prebuild's kernel would be given back mid-compile.
pub fn asks_outliving(hours: Hours, tick: Tick, used: u64, ends: u64) -> bool {
    let over = tick.now >= ends || past_deadline(hours, tick.into_day);
    !over && tick.now.saturating_sub(used) >= IDLE
}

/// When the next builder may be asked for, after one last used at `used`
/// ends with `event`. One given back idle while work still `wants` it is
/// one that work couldn't use, so it waits as long as after a failure;
/// one given back in use at the deadline, or whose time ran out at `ends`
/// while it slept unused with the machine, doesn't; nor does one that sat
/// unused only because the machine slept, just before it `woke`.
pub fn next_after(event: Event, wants: bool, used: u64, ends: u64, woke: bool, now: u64) -> u64 {
    match event {
        Event::Lost | Event::Failed => now + AFTER_FAILING,
        Event::Stopped if wants && !woke && now < ends && now.saturating_sub(used) >= IDLE => {
            now + AFTER_FAILING
        }
        _ => now,
    }
}

/// Whether a builder is wanted now: while a `there` build waits for one,
/// or the queue holds work one could take, unless the lender lends only to
/// `there`, whose builder would never be offered the queue's work. The
/// queue is read only when it counts.
pub fn wants(only_there: bool, queued: impl FnOnce() -> bool, waiting: bool) -> bool {
    waiting || (!only_there && queued())
}

/// Whether a lender whose unit a switch has changed (`superseded`) ends
/// now, for systemd to start the new one: only once no build reaches its
/// builder (`in_use`) and none `wants` one, since its end takes the way to
/// its builder with it. Three switches on 3 November cut 8.1 hours of
/// builds that way, restarting every lender at once.
pub fn restarts(superseded: bool, wants: bool, in_use: bool) -> bool {
    superseded && !wants && !in_use
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_superseded_lender_ends_only_once_nothing_uses_or_wants_its_builder() {
        assert!(restarts(true, false, false));
        assert!(!restarts(true, true, false), "a build waits on one");
        assert!(!restarts(true, false, true), "a build reaches it");
        assert!(!restarts(false, false, false), "nothing changed it");
    }

    #[test]
    fn a_lender_only_for_there_wants_nothing_the_queue_holds() {
        assert!(!wants(true, || true, false));
        assert!(wants(true, || panic!("the queue isn't read"), true));
        assert!(wants(false, || true, false));
        assert!(!wants(false, || false, false));
    }

    const HOURS: Hours = Hours {
        first: 8 * 3600,
        last: 19 * 3600 + 30 * 60,
        deadline: 20 * 3600,
        longest: None,
    };

    /// Lent at any hour, each builder living at most 12 hours.
    const ANY_HOUR: Hours = Hours {
        first: 0,
        last: 23 * 3600 + 59 * 60,
        deadline: 23 * 3600 + 59 * 60,
        longest: Some(12 * 3600),
    };

    #[test]
    fn lent_for_a_span_it_lives_that_long_whatever_the_hour() {
        let late = 23 * 3600 + 50 * 60;
        assert_eq!(lend(ANY_HOUR, at(late, true), 0, 0), Some(12 * 3600));
        // Past the deadline it's still in use, so it isn't given back.
        let past = at(23 * 3600 + 59 * 60 + 30, false);
        assert_eq!(give_back(ANY_HOUR, past, past.now, LATER, false), None);
        let now = 1_000_000;
        assert_eq!(ends(ANY_HOUR, now - 600, now, 0), now - 600 + 12 * 3600);
        assert_eq!(ends(HOURS, now - 600, now, 12 * 3600), now + 8 * 3600);
    }

    #[test]
    fn one_gone_just_before_its_end_ended_on_time() {
        // Asked to live until 23:59 on 2 November, a builder was deleted at
        // 23:55:43, and counted lost, lending none for 30 minutes.
        let tick = at(12 * 3600, false);
        let ends = tick.now + 200;
        assert_eq!(
            give_back(HOURS, tick, tick.now, ends, true),
            Some(Event::Stopped)
        );
        let ends = tick.now + NEAR_ITS_END + 1;
        assert_eq!(
            give_back(HOURS, tick, tick.now, ends, true),
            Some(Event::Lost)
        );
    }

    /// An end past every tick these tests make, for a builder whose time
    /// hasn't run out.
    const LATER: u64 = 10_000_000;

    fn at(into_day: u32, wants: bool) -> Tick {
        Tick {
            now: 1_000_000,
            into_day,
            wants,
            woke: false,
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
        assert_eq!(
            next_after(Event::Failed, false, 5, LATER, false, 5),
            5 + AFTER_FAILING
        );
        assert_eq!(
            next_after(Event::Lost, false, 5, LATER, false, 5),
            5 + AFTER_FAILING
        );
        assert_eq!(
            next_after(Event::Stopped, false, 0, LATER, false, IDLE),
            IDLE
        );
    }

    #[test]
    fn one_given_back_unused_while_work_waits_is_not_asked_for_again_at_once() {
        let now = 100_000;
        assert_eq!(
            next_after(Event::Stopped, true, now - IDLE, LATER, false, now),
            now + AFTER_FAILING
        );
    }

    #[test]
    fn one_given_back_in_use_at_the_deadline_leaves_the_next_day_free_to_ask() {
        // Given back at 23:59 in use on 30 October, the next waited out
        // half an hour of the day after as if no work could use it.
        let now = 100_000;
        assert_eq!(
            next_after(Event::Stopped, true, now - 60, LATER, false, now),
            now
        );
    }

    #[test]
    fn one_that_ran_out_its_time_while_the_machine_slept_is_given_back_on_time() {
        // Lent at 18:30 on 31 October to live to 23:59, it ended while the
        // machine slept from 23:31 to 09:35. Counted lost on waking, the next
        // waited half an hour with every build and VM test held for it.
        let ends = 1_000_000;
        let morning = Tick {
            now: ends + 10 * 3600,
            into_day: 9 * 3600 + 36 * 60,
            wants: true,
            woke: false,
        };
        assert_eq!(
            give_back(HOURS, morning, ends - 1800, ends, true),
            Some(Event::Stopped)
        );
        assert_eq!(
            next_after(Event::Stopped, true, ends - 1800, ends, false, morning.now),
            morning.now
        );
    }

    #[test]
    fn one_gone_on_its_own_while_the_machine_slept_is_not_counted_lost() {
        // A builder deletes itself after 15 idle minutes, so a lid shut over
        // lunch leaves it gone on waking, though nothing failed.
        let woken = Tick {
            woke: true,
            ..at(13 * 3600, true)
        };
        let used = woken.now - 3600;
        assert_eq!(
            give_back(HOURS, woken, used, LATER, true),
            Some(Event::Stopped)
        );
        assert_eq!(
            next_after(Event::Stopped, true, used, LATER, true, woken.now),
            woken.now
        );
    }

    #[test]
    fn a_builder_is_asked_about_runs_outliving_their_callers_only_when_idle_within_its_time() {
        let noon = at(12 * 3600, false);
        assert!(!asks_outliving(HOURS, noon, noon.now - 60, LATER));
        assert!(asks_outliving(HOURS, noon, noon.now - IDLE, LATER));
        assert!(!asks_outliving(HOURS, noon, noon.now - IDLE, noon.now));
        let late = at(20 * 3600, false);
        assert!(!asks_outliving(HOURS, late, late.now - IDLE, LATER));
    }

    #[test]
    fn a_builder_is_given_back_once_idle_at_the_deadline_or_once_gone() {
        let noon = at(12 * 3600, false);
        assert_eq!(give_back(HOURS, noon, noon.now - 60, LATER, false), None);
        assert_eq!(
            give_back(HOURS, noon, noon.now - IDLE, LATER, false),
            Some(Event::Stopped)
        );
        assert_eq!(
            give_back(HOURS, at(20 * 3600, true), 1_000_000, LATER, false),
            Some(Event::Stopped)
        );
        assert_eq!(
            give_back(HOURS, noon, noon.now, LATER, true),
            Some(Event::Lost)
        );
    }
}
