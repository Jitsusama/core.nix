//! When a builder is lent and when it's given back, apart from doing
//! either, so each rule can be pinned by a test.

use crate::journal::Event;

/// How long a builder sits unused before it's given back, unless its
/// lender says otherwise in [`Hours::idle`].
pub const IDLE: u64 = 15 * 60;
/// How long after a builder fails to stand before another is asked for,
/// so a helper that keeps failing doesn't make one a minute. One that
/// stood and then went is asked for again at once: on 5 November one
/// whose tunnel broke was lost, and with this hold after it none was lent
/// for half an hour, every build falling back to the laptop.
pub const AFTER_FAILING: u64 = 30 * 60;
/// How near its end a builder that goes on its own ended on time, rather
/// than being lost: one asked to live until 23:59 on 2 November was
/// deleted at 23:55:43, and counted lost, which lent none for 30 minutes.
pub const NEAR_ITS_END: u64 = 10 * 60;

/// How long before its end a builder still used or wanted hands over to a
/// successor, finishing what it has while every new build goes to the
/// next: its end cut 11 `there` runs at once at 17:18 on 3 November, 17
/// to 55 minutes in. Of 804 VM test runs on lent builders that passed, 95%
/// took under 37 minutes and 99% under 75, so an hour lets nearly every
/// run finish, and costs a second builder only while the first's last
/// runs do.
pub const HANDS_OVER: u64 = 60 * 60;

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
    /// How long a builder sits unused before it's given back, in seconds.
    /// A VM builder warms for about 17 minutes before it takes a test, so
    /// on 10 November four given back after [`IDLE`] were asked for again
    /// two to four minutes later, and VM runs waited a median 6.8 minutes
    /// for a builder while their successors warmed.
    pub idle: u64,
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
    /// Whether a `there` run waits for a builder, which it asks for only
    /// to build on it at once.
    pub asked: bool,
    /// Whether the machine woke from sleep in the last few minutes, long
    /// enough for a helper to notice a builder that went while it slept.
    pub woke: bool,
}

/// Until when no builder is asked for, as [`next_after`] says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// Seconds since the epoch.
    pub until: u64,
    /// Whether it holds because the last builder sat unused while work
    /// wanted it, which a `there` run asking for one lifts.
    pub unused: bool,
}

/// With nothing lent: the seconds a builder asked for now should live, if
/// one should be. None is asked for while one this asked for earlier isn't
/// yet seen gone, so there's never a second beside it, nor while `hold`
/// holds, unless a `there` run asks for one after one sat unused: one sat
/// unused at 23:16 on 4 November when its tunnel stopped answering, and
/// for half an hour after every `there` run gave up asking.
pub fn lend(hours: Hours, tick: Tick, hold: Hold, unreturned: usize) -> Option<u64> {
    let within = (hours.first..hours.last).contains(&tick.into_day);
    let waits = tick.now < hold.until && !(hold.unused && tick.asked);
    (tick.wants && within && !waits && unreturned == 0)
        .then(|| u64::from(hours.longest.unwrap_or(hours.deadline - tick.into_day)))
}

/// With one lent asked to live until `ends`, and `reached` by a build
/// now: the seconds a successor asked for now should live, if one should
/// be, since its time is nearly up while a build uses or wants it. Only
/// a builder that lives `longest` from its asking hands over: one asked
/// to live to the deadline would hand over to one ending with it. None is
/// asked while waiting out a successor that failed, until `not_before`.
pub fn hands_over(
    hours: Hours,
    tick: Tick,
    ends: u64,
    reached: bool,
    not_before: u64,
) -> Option<u64> {
    let longest = hours.longest?;
    let within = (hours.first..hours.last).contains(&tick.into_day);
    let near_its_end = tick.now < ends && tick.now + HANDS_OVER >= ends;
    (near_its_end && within && (reached || tick.wants) && tick.now >= not_before)
        .then_some(u64::from(longest))
}

/// With one handed over and asked to live until `ends`: whether it's given
/// back now, once its helper is `gone`, which it is once nothing reaches
/// it, or once its time is up. One holding a run that outlives its caller
/// stays until then, since nothing here reaches that run to say it ended.
pub fn leaves(now: u64, ends: u64, gone: bool, outliving: bool) -> bool {
    now >= ends || (gone && !outliving)
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
    } else if over || gone || tick.now.saturating_sub(used) >= hours.idle {
        Some(Event::Stopped)
    } else {
        None
    }
}

/// Why a lent builder is given back now, as [`give_back`] says, unless
/// its last proofs all failed (`unanswered`): then it's lost, whatever
/// else holds it, so another is asked for at once. On 8 November a switch
/// hung 20 minutes on one that had run out of memory, its line standing.
pub fn give_back_proven(
    hours: Hours,
    tick: Tick,
    used: u64,
    ends: u64,
    gone: bool,
    unanswered: bool,
) -> Option<Event> {
    if unanswered {
        Some(Event::Lost)
    } else {
        give_back(hours, tick, used, ends, gone)
    }
}

/// How many proofs in a row a builder that builds still reach may fail
/// before it's given back. A proof waits for a build slot like any build,
/// so a builder every slot of which is busy can fail one while it answers:
/// on 8 November one carrying about 40 builds failed two in 76 s, and its
/// give-back cut them all. A dead builder's relay gives up on its own
/// within 3 minutes, which ends what reaches it.
pub const FAILS_WHILE_REACHED: u32 = 10;

/// Whether a builder whose proofs say it's dropped (`dropped`, after
/// `failures` in a row) is given back as lost, given whether a build
/// reaches it (`reached`). Dropped, it's named nowhere either way, so no
/// new build goes there.
pub fn proven_lost(dropped: bool, failures: u32, reached: bool) -> bool {
    dropped && (!reached || failures >= FAILS_WHILE_REACHED)
}

/// Whether a builder last used at `used`, asked to live until `ends`, is
/// asked whether a run outliving its caller still builds there: only once
/// it would otherwise be given back idle, so it's asked once each idle
/// spell rather than every tick, and never once its time is up, since
/// nothing holds a builder past that. With its caller gone, no ssh here
/// reaches it, and a prebuild's kernel would be given back mid-compile.
pub fn asks_outliving(hours: Hours, tick: Tick, used: u64, ends: u64) -> bool {
    let over = tick.now >= ends || past_deadline(hours, tick.into_day);
    !over && tick.now.saturating_sub(used) >= hours.idle
}

/// When the next builder may be asked for, after one last used at `used`
/// ends with `event`. One given back idle while work still `wants` it is
/// one that work couldn't use, so it waits as long as after a failure;
/// one given back in use at the deadline, or whose time ran out at `ends`
/// while it slept unused with the machine, doesn't; nor does one that sat
/// unused only because the machine slept, just before it `woke`.
pub fn next_after(event: Event, wants: bool, used: u64, ends: u64, woke: bool, now: u64) -> Hold {
    let (until, unused) = match event {
        Event::Failed => (now + AFTER_FAILING, false),
        Event::Stopped if wants && !woke && now < ends && now.saturating_sub(used) >= IDLE => {
            (now + AFTER_FAILING, true)
        }
        _ => (now, false),
    };
    Hold { until, unused }
}

/// Whether a builder is wanted now: while a `there` build waits for one,
/// or the queue holds work one could take, unless the lender lends only to
/// `there`, whose builder would never be offered the queue's work. The
/// queue is read only when it counts.
pub fn wants(only_there: bool, queued: impl FnOnce() -> bool, waiting: bool) -> bool {
    waiting || (!only_there && queued())
}

/// How a lender whose unit changed ends, for systemd to start the new one.
#[derive(Debug, PartialEq, Eq)]
pub enum Restart {
    /// No build reaches its builder, so its way ends with it.
    Ending,
    /// A build reaches its builder, so its way is told to take no new
    /// connection and finish those it has, outliving the lender, while the
    /// next start takes the builder back beside it.
    HandingOver,
}

/// Whether a lender whose unit a switch has changed (`superseded`) ends
/// now, and how, given whether a build reaches its builder (`in_use`):
/// at once, handing its builder over when one does, unless a builder it
/// handed to a successor still finishes (`finishing`), which nothing would
/// give back. Waiting for no build to reach it, as it did, never came
/// under steady load: on 8 November the lenders ran the old binary for
/// hours after a switch.
pub fn restarts(superseded: bool, in_use: bool, finishing: bool) -> Option<Restart> {
    match (superseded && !finishing, in_use) {
        (false, _) => None,
        (true, false) => Some(Restart::Ending),
        (true, true) => Some(Restart::HandingOver),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_builder_near_its_end_hands_over_while_a_build_uses_or_wants_it() {
        let tick = at(12 * 3600, false);
        let ends = tick.now + HANDS_OVER;
        assert_eq!(hands_over(ANY_HOUR, tick, ends, true, 0), Some(12 * 3600));
        let wanted = at(12 * 3600, true);
        assert_eq!(
            hands_over(ANY_HOUR, wanted, ends, false, 0),
            Some(12 * 3600)
        );
        assert_eq!(
            hands_over(ANY_HOUR, tick, ends, false, 0),
            None,
            "nothing uses or wants it"
        );
        assert_eq!(
            hands_over(ANY_HOUR, tick, ends + 1, true, 0),
            None,
            "it has more than an hour left"
        );
        assert_eq!(
            hands_over(ANY_HOUR, tick, tick.now, true, 0),
            None,
            "its time is up"
        );
    }

    #[test]
    fn no_successor_is_asked_for_to_the_deadline_outside_the_hours_or_after_one_failed() {
        let tick = at(12 * 3600, true);
        let ends = tick.now + 60;
        assert_eq!(hands_over(HOURS, tick, ends, true, 0), None);
        assert_eq!(hands_over(ANY_HOUR, tick, ends, true, tick.now + 1), None);
        let late = at(23 * 3600 + 59 * 60 + 30, true);
        assert_eq!(hands_over(ANY_HOUR, late, late.now + 60, true, 0), None);
    }

    #[test]
    fn one_handed_over_is_given_back_once_nothing_reaches_it_or_its_time_is_up() {
        assert!(!leaves(100, 200, false, false));
        assert!(leaves(100, 200, true, false));
        assert!(
            !leaves(100, 200, true, true),
            "a run outliving its caller holds it"
        );
        assert!(leaves(200, 200, true, true));
        assert!(leaves(200, 200, false, false));
    }

    #[test]
    fn a_builder_builds_still_reach_is_given_back_only_after_ten_failed_proofs() {
        assert!(proven_lost(true, 2, false), "nothing reaches it");
        assert!(!proven_lost(true, 2, true), "a build reaches it");
        assert!(!proven_lost(true, FAILS_WHILE_REACHED - 1, true));
        assert!(proven_lost(true, FAILS_WHILE_REACHED, true));
        assert!(!proven_lost(false, 0, false), "it answers");
    }

    #[test]
    fn a_superseded_lender_ends_at_once_unless_a_builder_it_handed_over_finishes() {
        assert_eq!(restarts(true, false, false), Some(Restart::Ending));
        assert_eq!(
            restarts(true, true, false),
            Some(Restart::HandingOver),
            "a build reaches it"
        );
        assert_eq!(
            restarts(true, true, true),
            None,
            "one it handed over finishes"
        );
        assert_eq!(restarts(false, true, false), None, "nothing changed it");
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
        idle: IDLE,
    };

    /// Lent at any hour, each builder living at most 12 hours.
    const ANY_HOUR: Hours = Hours {
        first: 0,
        last: 23 * 3600 + 59 * 60,
        deadline: 23 * 3600 + 59 * 60,
        longest: Some(12 * 3600),
        idle: IDLE,
    };

    #[test]
    fn lent_for_a_span_it_lives_that_long_whatever_the_hour() {
        let late = 23 * 3600 + 50 * 60;
        assert_eq!(lend(ANY_HOUR, at(late, true), FREE, 0), Some(12 * 3600));
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

    #[test]
    fn one_in_use_that_fails_its_proofs_is_lost_and_another_asked_for_at_once() {
        let tick = at(12 * 3600, true);
        assert_eq!(
            give_back_proven(ANY_HOUR, tick, tick.now, LATER, false, false),
            None,
            "in use and answering"
        );
        assert_eq!(
            give_back_proven(ANY_HOUR, tick, tick.now, LATER, false, true),
            Some(Event::Lost),
            "in use, but not answering"
        );
        let idle = tick.now - IDLE;
        assert_eq!(
            give_back_proven(ANY_HOUR, tick, idle, LATER, false, false),
            Some(Event::Stopped)
        );
        let hold = next_after(Event::Lost, true, tick.now, LATER, false, tick.now);
        assert_eq!(lend(ANY_HOUR, tick, hold, 0), Some(12 * 3600));
    }

    fn at(into_day: u32, wants: bool) -> Tick {
        Tick {
            now: 1_000_000,
            into_day,
            wants,
            asked: false,
            woke: false,
        }
    }

    /// No hold at all.
    const FREE: Hold = Hold {
        until: 0,
        unused: false,
    };

    #[test]
    fn a_builder_is_lent_while_work_waits_within_the_days_hours() {
        assert_eq!(lend(HOURS, at(12 * 3600, true), FREE, 0), Some(8 * 3600));
        assert_eq!(lend(HOURS, at(12 * 3600, false), FREE, 0), None);
    }

    #[test]
    fn it_lives_to_the_deadline_to_the_second() {
        let late = 19 * 3600 + 29 * 60 + 15;
        assert_eq!(lend(HOURS, at(late, true), FREE, 0), Some(45 + 30 * 60));
    }

    #[test]
    fn none_is_lent_before_the_first_start_or_from_the_last() {
        assert_eq!(lend(HOURS, at(60, true), FREE, 0), None);
        assert_eq!(lend(HOURS, at(8 * 3600 - 1, true), FREE, 0), None);
        assert_eq!(lend(HOURS, at(19 * 3600 + 30 * 60, true), FREE, 0), None);
        assert_eq!(lend(HOURS, at(23 * 3600, true), FREE, 0), None);
    }

    #[test]
    fn none_is_lent_beside_one_not_yet_seen_gone() {
        assert_eq!(lend(HOURS, at(12 * 3600, true), FREE, 1), None);
    }

    #[test]
    fn none_is_lent_while_waiting_out_a_failure() {
        let failed = next_after(Event::Failed, false, 5, LATER, false, 5);
        assert_eq!(failed.until, 5 + AFTER_FAILING);
        // One that stood and then went is asked for again at once: on 5
        // November one whose tunnel broke was lost, and none was lent for
        // the 30 minutes after, everything building on the laptop.
        assert_eq!(next_after(Event::Lost, true, 5, LATER, false, 5).until, 5);
        assert_eq!(
            next_after(Event::Stopped, false, 0, LATER, false, IDLE).until,
            IDLE
        );
        let failing = Hold {
            until: 1_000_001,
            ..failed
        };
        assert_eq!(lend(HOURS, at(12 * 3600, true), failing, 0), None);
        // A `there` run asking doesn't lift it: the next would fail too.
        let asked = Tick {
            asked: true,
            ..at(12 * 3600, true)
        };
        assert_eq!(lend(HOURS, asked, failing, 0), None);
    }

    #[test]
    fn one_given_back_unused_while_work_waits_is_not_asked_for_again_at_once() {
        let now = 100_000;
        assert_eq!(
            next_after(Event::Stopped, true, now - IDLE, LATER, false, now).until,
            now + AFTER_FAILING
        );
    }

    #[test]
    fn a_there_run_asking_lifts_the_hold_after_one_sat_unused() {
        // At 23:16 on 4 November a builder whose tunnel stopped answering
        // sat unused and was given back, and for half an hour every `there`
        // run gave up asking for one.
        let noon = at(12 * 3600, true);
        let unused = next_after(
            Event::Stopped,
            true,
            noon.now - IDLE,
            LATER,
            false,
            noon.now,
        );
        assert_eq!(lend(HOURS, noon, unused, 0), None);
        let asked = Tick {
            asked: true,
            ..noon
        };
        assert_eq!(lend(HOURS, asked, unused, 0), Some(8 * 3600));
    }

    #[test]
    fn one_given_back_in_use_at_the_deadline_leaves_the_next_day_free_to_ask() {
        // Given back at 23:59 in use on 30 October, the next waited out
        // half an hour of the day after as if no work could use it.
        let now = 100_000;
        assert_eq!(
            next_after(Event::Stopped, true, now - 60, LATER, false, now).until,
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
            asked: false,
            woke: false,
        };
        assert_eq!(
            give_back(HOURS, morning, ends - 1800, ends, true),
            Some(Event::Stopped)
        );
        assert_eq!(
            next_after(Event::Stopped, true, ends - 1800, ends, false, morning.now).until,
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
            next_after(Event::Stopped, true, used, LATER, true, woken.now).until,
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
    fn a_builder_slow_to_warm_stays_through_the_idle_its_lender_names() {
        let warming = Hours {
            idle: 45 * 60,
            ..HOURS
        };
        let noon = at(12 * 3600, false);
        assert_eq!(
            give_back(warming, noon, noon.now - IDLE, LATER, false),
            None
        );
        assert!(
            !asks_outliving(warming, noon, noon.now - IDLE, LATER),
            "asked about an outliving run before the idle its lender names"
        );
        assert_eq!(
            give_back(warming, noon, noon.now - 45 * 60, LATER, false),
            Some(Event::Stopped)
        );
        assert!(
            asks_outliving(warming, noon, noon.now - 45 * 60, LATER),
            "not asked about an outliving run once idle as long as its lender names"
        );
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
