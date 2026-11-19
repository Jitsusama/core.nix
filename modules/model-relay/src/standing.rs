//! When the relay stands down, sending pi direct for a while.
//!
//! On 18 November the relay's connections failed most of what they
//! carried for forty minutes while pi's own, straight to the proxy over
//! the same tunnel, failed almost nothing. Its long-lived connections
//! were what the path broke, so the relay judges itself by the requests
//! it finished lately: past a share failed, it stands down, answering
//! pi with its own 421 so each request goes direct, and takes requests
//! again after a while. One more stand-down soon after the last doubles
//! the while, up to a ceiling, so a path that stays bad is tried again
//! less and less often, while one that mends is back in minutes.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

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
    /// Taking requests, with a first stand-down of `first`.
    pub(crate) fn new(first: Duration) -> Standing {
        Standing {
            first,
            held: Mutex::new(State::default()),
        }
    }

    /// Hears a request finished `at`, failed or answered, and says the
    /// stand-down it begins, if it does. One finishing while it stands
    /// down was taken before, so it isn't judged.
    pub(crate) fn heard(&self, at: Instant, failed: bool) -> Option<StoodDown> {
        let mut state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if state.down.is_some() {
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
        let longest = self.first * LONGEST_TIMES;
        let lasting = match state.last {
            Some((ended, was)) if at.saturating_duration_since(ended) < longest => {
                (was * 2).min(longest)
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
                Taking::Again(lasting)
            }
        }
    }

    /// Whether it stands down `at`, or stood down within `after` before,
    /// so it mustn't leave: leaving, it would forget a stand-down, and one
    /// just ended is a time pi is sent back to it, as being asked is.
    pub(crate) fn holds_on(&self, at: Instant, after: Duration) -> bool {
        let state = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .down
            .or(state.last)
            .is_some_and(|(ended, _)| at < ended + after)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{JUDGED_ON_AT_LEAST, Standing, StoodDown, Taking};

    const TEN_MINUTES: Duration = Duration::from_secs(600);
    const A_SECOND: Duration = Duration::from_secs(1);

    /// Hears `answered` requests answered then `failed` failed, a second
    /// apart from `from`, and says the last stand-down heard, if any.
    fn hears(
        standing: &Standing,
        from: Instant,
        answered: usize,
        failed: usize,
    ) -> Option<StoodDown> {
        let mut stood = None;
        for at in 0..answered + failed {
            let when = from + A_SECOND * u32::try_from(at).expect("a few");
            stood = standing.heard(when, at >= answered).or(stood);
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
            standing.heard(from + A_SECOND * 20, true),
            Some(StoodDown {
                failed: 4,
                finished: 21,
                lasting: TEN_MINUTES
            })
        );
    }

    #[test]
    fn never_down_it_holds_nothing_on() {
        let standing = Standing::new(TEN_MINUTES);

        assert!(!standing.holds_on(Instant::now(), TEN_MINUTES));
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
        assert!(standing.holds_on(down_at + A_SECOND, Duration::ZERO));
        assert!(!standing.holds_on(down_at + TEN_MINUTES, Duration::ZERO));
        assert!(standing.holds_on(down_at + TEN_MINUTES, A_SECOND));
        assert!(!standing.holds_on(down_at + TEN_MINUTES + A_SECOND, A_SECOND));
        assert_eq!(
            standing.taking(down_at + TEN_MINUTES),
            Taking::Again(TEN_MINUTES)
        );
        assert_eq!(standing.taking(down_at + TEN_MINUTES), Taking::Yes);
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
            assert!(matches!(standing.taking(back), Taking::Again(_)));
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
