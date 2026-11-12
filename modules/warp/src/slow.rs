//! Whether the way through the VPN is slow, from what waits on it as the
//! network's shaper hears it (waiting.rs), so the margin and Now can say
//! so while nothing shows the Work Network. It's slow once the wait holds
//! at 50 ms or more for a minute, and well again once it holds under
//! 30 ms for half of one (VKCF3Z, 2026-11-12), so one busy moment says
//! nothing, and a wait wavering about one bar doesn't flap.

use serde_json::Value;

/// The bars a wait is judged by, and for how long each must hold, each
/// in milliseconds.
#[derive(Clone, Copy, Debug)]
pub struct Bars {
    /// A wait at least this long, held for `slow_for`, is slow.
    pub slow_from: u64,
    pub slow_for: u64,
    /// While slow, a wait under this, held for `well_for`, is well.
    pub well_under: u64,
    pub well_for: u64,
}

impl Default for Bars {
    fn default() -> Self {
        Self {
            slow_from: 50,
            slow_for: 60_000,
            well_under: 30,
            well_for: 30_000,
        }
    }
}

/// The way while it's slow: what waits on it, and since when the wait
/// has held past the bar, so the drawer can say for how long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slow {
    pub waiting_ms: u64,
    pub since_ms: u64,
}

/// How the way stands as each reading of the shaper's is heard.
#[derive(Debug)]
pub struct Judge {
    bars: Bars,
    slow: bool,
    /// Since when the wait held past the bar, once that made it slow.
    began: u64,
    /// Since when the wait has held past the bar it must cross, if it has.
    crossing: Option<u64>,
    /// The last wait heard, said again while a question goes unanswered.
    last: Option<u64>,
}

impl Judge {
    pub const fn new(bars: Bars) -> Self {
        Self {
            bars,
            slow: false,
            began: 0,
            crossing: None,
            last: None,
        }
    }

    /// The way as judged, with `wait` said of it while it's slow.
    fn said(&self, wait: Option<u64>) -> Option<Slow> {
        wait.filter(|_| self.slow).map(|waiting_ms| Slow {
            waiting_ms,
            since_ms: self.began,
        })
    }

    /// Judges one reading, as waiting.rs makes it, heard at `at_ms`: the
    /// wait and since when while the way is slow, else nothing. A shaper that isn't
    /// running can't say the way is slow, so that ends it; a question
    /// that went unanswered says nothing either way, so it leaves the
    /// judgement, and the wait said, as they stand.
    pub fn heard(&mut self, reading: &Value, at_ms: u64) -> Option<Slow> {
        if reading["running"] != Value::Bool(true) {
            self.slow = false;
            self.crossing = None;
            self.last = None;
            return None;
        }
        let Some(wait) = waited(reading) else {
            return self.said(self.last);
        };
        self.last = Some(wait);
        let past = if self.slow {
            wait < self.bars.well_under
        } else {
            wait >= self.bars.slow_from
        };
        if past {
            let since = *self.crossing.get_or_insert(at_ms);
            let needed = if self.slow {
                self.bars.well_for
            } else {
                self.bars.slow_for
            };
            if at_ms.saturating_sub(since) >= needed {
                self.slow = !self.slow;
                self.began = since;
                self.crossing = None;
            }
        } else {
            self.crossing = None;
        }
        self.said(Some(wait))
    }
}

/// How long past the way at rest a reading waited, in whole milliseconds,
/// as the Work Network's Waiting says it (WarpWaiting.js); nothing where
/// either figure went unheard.
fn waited(reading: &Value) -> Option<u64> {
    let through = reading["through_ms"].as_f64()?.round();
    let rest = reading["at_rest_ms"].as_f64()?.round();
    // Whole and short, as round trips are.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a wait is a few hundred whole milliseconds, never negative once floored at 0"
    )]
    Some((through - rest).max(0.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn waiting(ms: f64) -> Value {
        json!({"running": true, "through_ms": 30.0 + ms, "at_rest_ms": 30.0})
    }

    /// The waits the judge says for each reading, heard at its time.
    fn judged(judge: &mut Judge, waits: &[(u64, Value)]) -> Vec<Option<u64>> {
        waits
            .iter()
            .map(|(at, reading)| judge.heard(reading, *at).map(|slow| slow.waiting_ms))
            .collect()
    }

    #[test]
    fn slow_says_since_the_wait_first_held_past_the_bar() {
        let mut judge = Judge::new(Bars::default());
        judged(
            &mut judge,
            &[
                (0, waiting(120.0)),
                (40_000, waiting(10.0)),
                (50_000, waiting(90.0)),
            ],
        );
        assert_eq!(
            judge.heard(&waiting(90.0), 110_000),
            Some(Slow {
                waiting_ms: 90,
                since_ms: 50_000
            })
        );
        assert_eq!(
            judge
                .heard(&waiting(40.0), 200_000)
                .map(|slow| slow.since_ms),
            Some(50_000),
            "and keeps saying it while it's slow"
        );
    }

    #[test]
    fn a_wait_held_past_the_bar_for_a_minute_is_slow_and_not_before() {
        let mut judge = Judge::new(Bars::default());
        let heard = judged(
            &mut judge,
            &[
                (0, waiting(120.0)),
                (59_999, waiting(80.0)),
                (60_000, waiting(90.0)),
            ],
        );
        assert_eq!(heard, vec![None, None, Some(90)]);
    }

    #[test]
    fn a_busy_moment_under_a_minute_says_nothing_and_starts_the_minute_again() {
        let mut judge = Judge::new(Bars::default());
        let heard = judged(
            &mut judge,
            &[
                (0, waiting(120.0)),
                (40_000, waiting(10.0)),
                (50_000, waiting(120.0)),
                (100_000, waiting(120.0)),
                (110_000, waiting(120.0)),
            ],
        );
        assert_eq!(heard, vec![None, None, None, None, Some(120)]);
    }

    #[test]
    fn slow_holds_while_the_wait_wavers_and_ends_after_half_a_minute_under_30() {
        let mut judge = Judge::new(Bars::default());
        judge.heard(&waiting(60.0), 0);
        assert_eq!(
            judge
                .heard(&waiting(60.0), 60_000)
                .map(|slow| slow.waiting_ms),
            Some(60)
        );
        let heard = judged(
            &mut judge,
            &[
                (61_000, waiting(40.0)),
                (62_000, waiting(20.0)),
                (80_000, waiting(35.0)),
                (81_000, waiting(20.0)),
                (110_999, waiting(20.0)),
                (111_000, waiting(20.0)),
            ],
        );
        assert_eq!(
            heard,
            vec![Some(40), Some(20), Some(35), Some(20), Some(20), None],
            "a wait between the bars is still slow, and one back over 30 starts the half again"
        );
    }

    #[test]
    fn a_question_unanswered_leaves_it_as_it_stood_and_the_shaper_stopping_ends_it() {
        let mut judge = Judge::new(Bars::default());
        judge.heard(&waiting(60.0), 0);
        judge.heard(&waiting(60.0), 60_000);
        let unheard = json!({"running": true, "through_ms": null, "at_rest_ms": 30.0});
        assert_eq!(
            judge.heard(&unheard, 61_000),
            Some(Slow {
                waiting_ms: 60,
                since_ms: 0
            }),
            "unheard says the last wait again"
        );
        assert_eq!(
            judged(&mut judge, &[(62_000, waiting(55.0))]),
            vec![Some(55)],
            "and leaves it slow"
        );
        assert_eq!(judge.heard(&json!({"running": false}), 63_000), None);
        assert_eq!(
            judge.heard(&waiting(55.0), 64_000),
            None,
            "once the shaper stopped, a minute must pass again"
        );
    }

    #[test]
    fn the_wait_is_the_round_trip_past_the_way_at_rest_as_waiting_says_it() {
        let reading = json!({"running": true, "through_ms": 103.12, "at_rest_ms": 27.67});
        assert_eq!(waited(&reading), Some(75));
        let quicker = json!({"running": true, "through_ms": 20.0, "at_rest_ms": 27.67});
        assert_eq!(waited(&quicker), Some(0));
    }
}
