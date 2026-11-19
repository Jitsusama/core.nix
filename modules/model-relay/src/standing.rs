//! When the relay stands down, sending pi direct for a while.
//!
//! On 18 November the relay's connections failed most of what they
//! carried for forty minutes while pi's own, straight to the proxy over
//! the same tunnel, failed almost nothing. Its long-lived connections
//! were what the path broke, so the relay judges itself by the requests
//! it finished lately: past a share failed, it stands down, answering
//! pi with its own 421 so each request goes direct, and takes requests
//! again once its path answers a trial, or after a while at most. One
//! more stand-down soon after the last doubles the while, up to a
//! ceiling, so a path that stays bad is tried with pi's requests less and
//! less often, while one that mends is back as soon as a trial says so. It's
//! all held in memory, so a relay that leaves once idle after a
//! stand-down, or is restarted, starts over at the first.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::STANDS_DOWN_FOR_AT_MOST;

/// How far back the requests it judges itself by go.
const LOOKS_BACK: Duration = Duration::from_secs(600);

/// The fewest finished requests it judges itself by, so a failure or two
/// in a quiet stretch isn't a share.
const JUDGED_ON_AT_LEAST: usize = 20;

/// The share failed, in percent, past which it stands down. Over four
/// days, every ten minutes past it fell in the 18 November spell.
const FAILED_PAST_PERCENT: usize = 15;

/// How many times its first stand-down its longest is, and how soon after
/// one ends, in the same times, another doubles it: from ten minutes, an
/// hour each.
const LONGEST_TIMES: u32 = 6;

/// Whether the relay is taking requests, and what it has finished lately.
pub(crate) struct Standing {
    /// How long its first stand-down lasts.
    first: Duration,
    held: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// When each request finished in the last `LOOKS_BACK`, oldest first,
    /// and whether it failed.
    finished: VecDeque<(Instant, bool)>,
    /// How many of `finished` failed.
    failed: usize,
    /// The stand-down it's in: until when, and how long it is.
    down: Option<(Instant, Duration)>,
    /// When the last stand-down ended, and how long it was.
    last: Option<(Instant, Duration)>,
    /// When it last forgot, or took requests again: a request sent before
    /// rode a connection it has since retired, and may break off when the
    /// path it was judged on is gone, so it isn't judged.
    since: Option<Instant>,
}

/// A stand-down just begun: what it was judged on, and how long it lasts.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StoodDown {
    pub failed: usize,
    pub finished: usize,
    pub lasting: Duration,
}

/// Whether it takes a request now.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Taking {
    /// It does.
    Yes,
    /// It doesn't, for this long more.
    No(Duration),
    /// It does again, its stand-down of this long having just ended; said
    /// to one asker alone.
    Again(Duration),
}

impl Standing {
    /// Taking requests, with a first stand-down of `first`, to
    /// `STANDS_DOWN_FOR_AT_MOST`, so the arithmetic on its doublings and
    /// its end can't overflow.
    pub(crate) fn new(first: Duration) -> Standing {
        Standing {
            first: first.min(STANDS_DOWN_FOR_AT_MOST),
            held: Mutex::new(State::default()),
        }
    }

    /// Hears a request `sent` and finished `at`, failed or answered, and
    /// says the stand-down it begins, if it does. One finishing while it
    /// stands down was taken before, and one sent before it last forgot
    /// or took requests again went down a connection since retired, so
    /// neither is judged.
    pub(crate) fn heard(&self, at: Instant, sent: Instant, failed: bool) -> Option<StoodDown> {
        let mut state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if state.down.is_some() || state.since.is_some_and(|since| sent < since) {
            return None;
        }
        state.finished.push_back((at, failed));
        state.failed += usize::from(failed);
        while let Some(&(when, failed)) = state.finished.front()
            && at.saturating_duration_since(when) > LOOKS_BACK
        {
            state.finished.pop_front();
            state.failed -= usize::from(failed);
        }
        let finished = state.finished.len();
        if finished < JUDGED_ON_AT_LEAST || state.failed * 100 <= finished * FAILED_PAST_PERCENT {
            return None;
        }
        let longest = self.first.saturating_mul(LONGEST_TIMES);
        let lasting = match state.last {
            Some((ended, was)) if at.saturating_duration_since(ended) < longest => {
                was.saturating_mul(2).min(longest)
            }
            _ => self.first,
        };
        let stood = StoodDown {
            failed: state.failed,
            finished,
            lasting,
        };
        state.down = Some((at + lasting, lasting));
        state.finished.clear();
        state.failed = 0;
        Some(stood)
    }

    /// Whether it takes a request asked `at`.
    pub(crate) fn taking(&self, at: Instant) -> Taking {
        let mut state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        match state.down {
            None => Taking::Yes,
            Some((until, _)) if at < until => Taking::No(until - at),
            Some((until, lasting)) => {
                state.down = None;
                state.last = Some((until, lasting));
                state.since = Some(at);
                Taking::Again(lasting)
            }
        }
    }

    /// Whether it stands down `at`, so its path is to be tried.
    pub(crate) fn is_down(&self, at: Instant) -> bool {
        let state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        state.down.is_some_and(|(until, _)| at < until)
    }

    /// Takes requests again `at`, ahead of its stand-down's end, as its
    /// path answers again, and says how long it stood down; or none, when
    /// it wasn't standing down. A stand-down soon after still doubles the
    /// one this was to be, as the longest it may last.
    pub(crate) fn mend(&self, at: Instant) -> Option<Duration> {
        let mut state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let (until, lasting) = state.down.filter(|&(until, _)| at < until)?;
        state.down = None;
        state.last = Some((at, lasting));
        state.since = Some(at);
        Some(lasting.saturating_sub(until - at))
    }

    /// Whether it stands down `at`, or stood down within `after` before,
    /// so it mustn't leave: leaving, it would forget a stand-down, and one
    /// just ended is a time pi is sent back to it, as being asked is. Once
    /// a request has heard it take requests again, that request is what
    /// holds the relay its idle time, so only a stand-down nobody has
    /// asked past is held on for.
    pub(crate) fn holds_on(&self, at: Instant, after: Duration) -> bool {
        let state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .down
            .is_some_and(|(until, _)| until.checked_add(after).is_none_or(|ended| at < ended))
    }

    /// Forgets its stand-down and all it finished, `at` once the machine
    /// has slept: its clock stopped meanwhile, and the network it judged
    /// may be gone, so what was sent before isn't judged either. Says
    /// whether it was standing down.
    pub(crate) fn forget(&self, at: Instant) -> bool {
        let mut state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let was_down = state.down.is_some();
        *state = State {
            since: Some(at),
            ..State::default()
        };
        was_down
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{JUDGED_ON_AT_LEAST, Standing, StoodDown, Taking};
    use crate::STANDS_DOWN_FOR_AT_MOST;

    const TEN_MINUTES: Duration = Duration::from_secs(600);
    const A_SECOND: Duration = Duration::from_secs(1);

    /// Hears `answered` requests answered then `failed` failed, a second
    /// apart from `from`, each sent as it finished, and says the last
    /// stand-down heard, if any.
    fn hears(
        standing: &Standing,
        from: Instant,
        answered: usize,
        failed: usize,
    ) -> Option<StoodDown> {
        let mut stood = None;
        for at in 0..answered + failed {
            let when = from + A_SECOND * u32::try_from(at).expect("a few");
            stood = standing.heard(when, when, at >= answered).or(stood);
        }
        stood
    }

    #[test]
    fn it_stands_down_past_fifteen_in_a_hundred_failed_of_twenty_or_more() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();

        assert_eq!(
            hears(&standing, from, 17, 3),
            None,
            "3 of 20 is 15%, not past it"
        );
        assert_eq!(
            standing.heard(from + A_SECOND * 20, from + A_SECOND * 20, true),
            Some(StoodDown {
                failed: 4,
                finished: 21,
                lasting: TEN_MINUTES
            })
        );
        assert!(
            hears(&Standing::new(TEN_MINUTES), from, 22, 4).is_some(),
            "4 of 26 is past 15%, though not past 16"
        );
    }

    #[test]
    fn what_finished_ten_minutes_ago_to_the_moment_still_counts() {
        let from = Instant::now();
        for (since, stands_down) in [(TEN_MINUTES, true), (TEN_MINUTES + A_SECOND, false)] {
            let standing = Standing::new(TEN_MINUTES);
            for _ in 0..4 {
                assert_eq!(
                    standing.heard(from, from, true),
                    None,
                    "four alone are judged"
                );
            }
            let mut stood = None;
            for _ in 0..16 {
                stood = standing.heard(from + since, from + since, false).or(stood);
            }
            assert_eq!(
                stood.is_some(),
                stands_down,
                "four failed {} s before sixteen answered",
                since.as_secs()
            );
        }
    }

    #[test]
    fn another_an_hour_after_to_the_moment_starts_over() {
        let an_hour = TEN_MINUTES * 6;
        let all_but_a_second = Duration::from_secs(3599);
        for (after, lasting) in [(all_but_a_second, TEN_MINUTES * 2), (an_hour, TEN_MINUTES)] {
            let standing = Standing::new(TEN_MINUTES);
            let from = Instant::now();
            hears(&standing, from, 0, 20).expect("it stands down");
            let back = from + A_SECOND * 19 + TEN_MINUTES;
            assert_eq!(standing.taking(back), Taking::Again(TEN_MINUTES));

            // Twenty heard a second apart stand it down at the last.
            let first_heard = (back + after)
                .checked_sub(A_SECOND * 19)
                .expect("after the stand-down");
            let stood = hears(&standing, first_heard, 0, 20);
            assert_eq!(
                stood.map(|stood| stood.lasting),
                Some(lasting),
                "another {} s after the last ended",
                after.as_secs()
            );
        }
    }

    #[test]
    fn a_sleep_forgets_a_stand_down_and_the_one_before() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();
        hears(&standing, from, 0, 20).expect("it stands down");
        let back = from + A_SECOND * 19 + TEN_MINUTES;
        assert_eq!(standing.taking(back), Taking::Again(TEN_MINUTES));
        hears(&standing, back, 0, 20).expect("it stands down again");

        assert!(
            standing.forget(back + A_SECOND * 20),
            "it didn't say it was standing down"
        );
        assert_eq!(standing.taking(back + A_SECOND * 20), Taking::Yes);
        assert!(
            !standing.forget(back + A_SECOND * 20),
            "it said it stood down after forgetting"
        );
        assert_eq!(
            hears(&standing, back + A_SECOND * 20, 0, 20).map(|stood| stood.lasting),
            Some(TEN_MINUTES),
            "a stand-down after a sleep doubled the one before it"
        );
    }

    #[test]
    fn what_was_sent_before_a_sleep_or_its_return_isnt_judged() {
        let from = Instant::now();
        let woke = from + TEN_MINUTES;
        let standing = Standing::new(TEN_MINUTES);
        assert!(!standing.forget(woke), "it said it stood down");
        for at in 0..20 {
            let ended = woke + A_SECOND * at;
            assert_eq!(
                standing.heard(ended, from, true),
                None,
                "a request sent before a sleep was judged as it broke off after"
            );
        }
        assert!(
            hears(&standing, woke, 0, 20).is_some(),
            "what was sent after the sleep wasn't judged"
        );

        let standing = Standing::new(TEN_MINUTES);
        hears(&standing, from, 0, 20).expect("it stands down");
        let back = from + A_SECOND * 19 + TEN_MINUTES;
        assert_eq!(standing.taking(back), Taking::Again(TEN_MINUTES));
        for at in 1..=20 {
            assert_eq!(
                standing.heard(back + A_SECOND * at, from, true),
                None,
                "a request sent before it stood down was judged after it took requests again"
            );
            // Taken before it stood down and sent once more as it stood
            // down, as on a connection opened meanwhile.
            assert_eq!(
                standing.heard(back + A_SECOND * at, from + A_SECOND * 20, true),
                None,
                "a request sent as it stood down was judged after it took requests again"
            );
        }
        assert!(
            hears(&standing, back + A_SECOND * 21, 0, 20).is_some(),
            "what was sent after it took requests again wasn't judged"
        );
    }

    #[test]
    fn a_first_stand_down_past_a_day_is_a_day_and_its_doublings_dont_overflow() {
        let standing = Standing::new(Duration::MAX);
        let from = Instant::now();

        let stood = hears(&standing, from, 0, 20).expect("it stands down");
        assert_eq!(stood.lasting, STANDS_DOWN_FOR_AT_MOST);
        assert!(
            standing.holds_on(from + STANDS_DOWN_FOR_AT_MOST, Duration::MAX),
            "an idle time too long to add wasn't held on for"
        );
        let back = from + A_SECOND * 19 + STANDS_DOWN_FOR_AT_MOST;
        assert_eq!(
            standing.taking(back),
            Taking::Again(STANDS_DOWN_FOR_AT_MOST)
        );
        assert_eq!(
            hears(&standing, back, 0, 20).map(|stood| stood.lasting),
            Some(STANDS_DOWN_FOR_AT_MOST * 2),
            "another soon after a day's didn't double it"
        );
    }

    #[test]
    fn never_down_it_holds_nothing_on() {
        let standing = Standing::new(TEN_MINUTES);

        assert!(
            !standing.holds_on(Instant::now(), TEN_MINUTES),
            "a relay that never stood down held itself on"
        );
    }

    #[test]
    fn it_judges_nothing_on_fewer_than_twenty() {
        let standing = Standing::new(TEN_MINUTES);

        assert_eq!(
            hears(&standing, Instant::now(), 0, JUDGED_ON_AT_LEAST - 1),
            None
        );
    }

    #[test]
    fn it_forgets_what_finished_over_ten_minutes_ago() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();

        assert_eq!(hears(&standing, from, 0, 10), None);
        assert_eq!(
            hears(&standing, from + TEN_MINUTES * 2, 15, 5),
            Some(StoodDown {
                failed: 5,
                finished: 20,
                lasting: TEN_MINUTES
            }),
            "five of twenty since is past the share"
        );
        let standing = Standing::new(TEN_MINUTES);
        assert_eq!(hears(&standing, from, 0, 10), None);
        assert_eq!(
            hears(&standing, from + TEN_MINUTES * 2, 30, 0),
            None,
            "the ten failed long ago were counted"
        );
    }

    #[test]
    fn standing_down_it_takes_nothing_until_its_while_is_over_then_says_so_once() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();
        hears(&standing, from, 0, 20).expect("it stands down");
        let down_at = from + A_SECOND * 19;

        assert_eq!(
            standing.taking(down_at + A_SECOND),
            Taking::No(Duration::from_secs(599))
        );
        assert!(
            standing.holds_on(down_at + A_SECOND, Duration::ZERO),
            "it didn't hold on while standing down"
        );
        assert!(
            !standing.holds_on(down_at + TEN_MINUTES, Duration::ZERO),
            "it held on past its stand-down, with no time after"
        );
        assert!(
            standing.holds_on(down_at + TEN_MINUTES, A_SECOND),
            "it didn't hold on within its time after the stand-down"
        );
        assert!(
            !standing.holds_on(down_at + TEN_MINUTES + A_SECOND, A_SECOND),
            "it held on past its time after the stand-down"
        );
        assert_eq!(
            standing.taking(down_at + TEN_MINUTES),
            Taking::Again(TEN_MINUTES)
        );
        assert_eq!(standing.taking(down_at + TEN_MINUTES), Taking::Yes);
    }

    #[test]
    fn its_path_answering_it_takes_requests_again_early_and_the_next_still_doubles() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();
        hears(&standing, from, 0, 20).expect("it stands down");
        let down_at = from + A_SECOND * 19;
        let mended_at = down_at + A_SECOND * 90;

        assert!(standing.is_down(mended_at), "it wasn't down to be tried");
        assert_eq!(
            standing.mend(mended_at),
            Some(A_SECOND * 90),
            "it didn't say how long it stood down before its path answered"
        );
        assert!(!standing.is_down(mended_at), "it stood down still");
        assert_eq!(
            standing.taking(mended_at),
            Taking::Yes,
            "it didn't take requests once its path answered"
        );
        assert!(
            !standing.holds_on(mended_at, Duration::ZERO),
            "it held on for a stand-down it ended"
        );
        assert_eq!(
            standing.mend(mended_at),
            None,
            "it mended a stand-down it wasn't in"
        );
        assert_eq!(
            hears(&standing, mended_at + A_SECOND, 0, 20),
            Some(StoodDown {
                failed: 20,
                finished: 20,
                lasting: TEN_MINUTES * 2,
            }),
            "one soon after a stand-down its path ended didn't double"
        );
    }

    #[test]
    fn what_finishes_while_it_stands_down_isnt_judged() {
        let standing = Standing::new(TEN_MINUTES);
        let from = Instant::now();
        hears(&standing, from, 0, 20).expect("it stands down");
        let back = from + A_SECOND * 19 + TEN_MINUTES;

        assert_eq!(hears(&standing, from + A_SECOND * 20, 0, 30), None);
        assert_eq!(standing.taking(back), Taking::Again(TEN_MINUTES));
        assert_eq!(
            hears(&standing, back, 0, JUDGED_ON_AT_LEAST - 1),
            None,
            "what failed while it stood down was judged after"
        );
    }

    #[test]
    fn another_soon_after_doubles_it_to_an_hour_at_most_and_one_later_starts_over() {
        let standing = Standing::new(TEN_MINUTES);
        let mut from = Instant::now();
        let mut lasted = Vec::new();
        for _ in 0..5 {
            let stood = hears(&standing, from, 0, 20).expect("it stands down");
            let back = from + A_SECOND * 19 + stood.lasting;
            assert_eq!(
                standing.taking(back),
                Taking::Again(stood.lasting),
                "it didn't take requests again once its stand-down was over"
            );
            lasted.push(stood.lasting.as_secs() / 60);
            from = back;
        }
        assert_eq!(lasted, [10, 20, 40, 60, 60]);

        let stood = hears(&standing, from + TEN_MINUTES * 6, 0, 20).expect("it stands down");
        assert_eq!(
            stood.lasting, TEN_MINUTES,
            "an hour after the last, it starts over"
        );
    }
}
