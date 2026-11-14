//! How many calls a session makes to GitHub in an hour, and when that's
//! too many.
//!
//! A session's work makes a few hundred calls in a busy hour; a command
//! looping on the API makes thousands, and spends the person's own rate
//! limit, which every other tool of theirs shares. So past [`LOUD`] the
//! door warns once an hour and carries on, and past [`MOST`] it refuses
//! until the hour resets, saying when, or until Joel lifts it
//! (`pi-github-door lift`). An hour starts with a session's first call
//! after the last one ended.

use std::time::{Duration, Instant};

/// The calls in an hour past which the door warns.
pub const LOUD: u32 = 600;
/// The calls in an hour past which it refuses.
pub const MOST: u32 = 2_000;
/// How long an hour is.
pub const HOUR: Duration = Duration::from_hours(1);

/// One session's count.
#[derive(Debug)]
pub struct Count {
    /// When the hour began.
    began: Instant,
    calls: u32,
    /// Whether Joel lifted the cap for this hour.
    lifted: bool,
}

/// What a call does to the count.
#[derive(Debug, PartialEq, Eq)]
pub enum Counted {
    /// Under the cap.
    Fine,
    /// The call that crossed [`LOUD`], to be warned of.
    Loud(u32),
    /// Past [`MOST`]; the hour resets after this long.
    Refused(Duration),
}

impl Count {
    /// A count beginning now.
    pub fn new(now: Instant) -> Self {
        Count {
            began: now,
            calls: 0,
            lifted: false,
        }
    }

    /// One more call at `now`.
    pub fn call(&mut self, now: Instant) -> Counted {
        if now.duration_since(self.began) >= HOUR {
            *self = Count::new(now);
        }
        if self.calls >= MOST && !self.lifted {
            return Counted::Refused(HOUR.saturating_sub(now.duration_since(self.began)));
        }
        self.calls += 1;
        if self.calls == LOUD + 1 {
            Counted::Loud(self.calls)
        } else {
            Counted::Fine
        }
    }

    /// Lift the cap for the rest of this hour.
    pub fn lift(&mut self) {
        self.lifted = true;
    }
}

/// What a refusal past the cap says, the hour resetting after `left`.
pub fn refused_detail(left: Duration) -> String {
    let minutes = left.as_secs().div_ceil(60).max(1);
    format!(
        "{MOST} calls to GitHub this hour from this session, the most it makes; the hour \
         resets in {minutes} min, and Joel can lift it with pi-github-door lift"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_warns_once_past_loud_and_refuses_past_most() {
        let start = Instant::now();
        let mut count = Count::new(start);
        let mut loud = 0;
        for _ in 0..MOST {
            match count.call(start) {
                Counted::Fine => {}
                Counted::Loud(calls) => {
                    loud += 1;
                    assert_eq!(calls, LOUD + 1);
                }
                Counted::Refused(_) => panic!("refused under the cap"),
            }
        }
        assert_eq!(loud, 1);
        let later = start + Duration::from_mins(20);
        assert_eq!(count.call(later), Counted::Refused(Duration::from_mins(40)));
    }

    #[test]
    fn the_hour_resets() {
        let start = Instant::now();
        let mut count = Count::new(start);
        for _ in 0..MOST {
            count.call(start);
        }
        assert!(
            matches!(count.call(start), Counted::Refused(_)),
            "a call past the most is refused"
        );
        assert_eq!(count.call(start + HOUR), Counted::Fine);
    }

    #[test]
    fn joel_can_lift_it() {
        let start = Instant::now();
        let mut count = Count::new(start);
        for _ in 0..MOST {
            count.call(start);
        }
        count.lift();
        assert_eq!(count.call(start), Counted::Fine);
    }

    #[test]
    fn a_refusal_says_when_it_resets_and_who_lifts_it() {
        let detail = refused_detail(Duration::from_secs(61));
        assert!(detail.contains("resets in 2 min"), "{detail}");
        assert!(detail.contains("Joel can lift it"), "{detail}");
    }
}
