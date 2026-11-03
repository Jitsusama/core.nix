//! The shaper's rebuilt controller: each direction's rate from that
//! direction's own delay, as ICMP timestamps measure it, rather than from
//! round trips, which mix both directions and the air.
//!
//! A direction is cut once its own delay has stood over `LATE_MS` for a
//! second while it carries a flow: to a tenth under what it carried, and
//! more the deeper the queue. It climbs while it's full with no delay, fast
//! until it nears where its queue last stood and gently after, and drifts
//! back toward the network's base while it's idle or low. The base
//! is learned from the line: what a direction carried each time its queue
//! stood with the rate well over it, the median of the last ten, less a
//! tenth. A new network starts with no limit at all, so nothing is held
//! down by a guess until the line shows where its queue is.
//!
//! Two things look like a queue and aren't. Wi-Fi's late spells delay
//! both directions at once and the gateway's round trip with them, so
//! whatever the gateway's rises past its jitter comes off both. And a
//! direction carrying nothing has no queue of its own, so its delay is the
//! path's, and the other direction's counts only past it.
//!
//! A live guard stands over it: a direction held under half its base for
//! a minute with no delay standing, or whose delay has stood unbroken for
//! half a minute however far it was cut, so that the queue is plainly
//! another machine's, goes back to its base; after three such returns in a
//! stay the controller gives up and leaves the line unlimited, as it does
//! when no timestamps are answered. Cuts alone don't trip it: a line
//! filling its buffer is rightly cut each second until it drains.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the replay and the shaper's setting take it up next"
    )
)]

use std::collections::VecDeque;

/// A direction's own delay past this, standing, is a queue.
pub const LATE_MS: f64 = 15.0;
/// How long the delay must stand before a cut.
const STANDING_S: f64 = 1.0;
/// Below this delay a full direction may climb.
const CLEAR_MS: f64 = 5.0;
/// A direction carrying under this many kbit/s has no flow to cut.
const FLOWING_KBIT: f64 = 500.0;
/// The least a direction is ever held to.
const LEAST_KBIT: f64 = 500.0;
/// How long after a cut before another.
const BETWEEN_CUTS_S: f64 = 1.0;
/// How fast a full, clear direction climbs, and an idle one drifts, each
/// second.
const CLIMB: f64 = 1.05;
/// How fast a rate climbs, per second, far from any queue it has met:
/// under this share of what got through when one last stood, or with none
/// met yet. A remembered rate far under the line is back in under ten
/// seconds.
const FAST_CLIMB: f64 = 1.25;
const FAR: f64 = 0.5;
const DRIFT: f64 = 0.01;
/// What share of its rate counts as full, and what as low.
const FULL: f64 = 0.75;
/// How many of the line's rates the base is the median of.
const LEARNED: usize = 10;
/// The gateway's jitter on Wi-Fi: its rises past this are the air's.
const AIR_MARGIN_MS: f64 = 8.0;
/// How fast the gateway's baseline may rise, each second, so a roam to a
/// farther access point is followed.
const AIR_RISE_MS: f64 = 0.05;
/// A direction carrying under this share of its rate is idle, and its
/// delay measures the path.
const IDLE: f64 = 0.1;
/// With no timestamps answered for this long, the line goes unlimited.
const UNANSWERED_S: f64 = 10.0;
/// The live guard's limits.
const GUARD_WINDOW_S: f64 = 60.0;
/// A delay standing this long, cuts and all, isn't this machine's queue.
const FUTILE_S: f64 = 30.0;
const GUARD_RETURNS: usize = 3;

/// What each direction is held to, in kbit/s; None is no limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub up: Option<u64>,
    pub down: Option<u64>,
}

/// What was seen at a moment: each direction's own delay above its
/// reflectors' least, the gateway's round trip, and what each direction
/// carried, in kbit/s.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seen {
    pub at_s: f64,
    pub up_delay_ms: Option<f64>,
    pub down_delay_ms: Option<f64>,
    pub gateway_ms: Option<f64>,
    pub up_kbit: f64,
    pub down_kbit: f64,
}

/// One direction's rate.
#[derive(Clone, Debug)]
pub struct Way {
    /// The rate it's held to; infinite is no limit.
    rate: f64,
    base: Option<f64>,
    learned: VecDeque<f64>,
    late_since: Option<f64>,
    /// What got through when this direction's queue last stood, in this
    /// stay; None until it first does.
    ceiling: Option<f64>,
    last_cut: f64,
    last_at: Option<f64>,
    under_half_since: Option<f64>,
}

impl Way {
    /// A direction on a network whose base is `base`, if one was learned,
    /// starting there; with none, it starts with no limit. A known base
    /// counts as one of the line's rates, so a single cut under another
    /// machine's flow doesn't replace it.
    pub fn new(base: Option<f64>) -> Way {
        Way {
            rate: base.unwrap_or(f64::INFINITY),
            base,
            learned: base.map(|b| b / 0.9).into_iter().collect(),
            late_since: None,
            ceiling: None,
            last_cut: f64::NEG_INFINITY,
            last_at: None,
            under_half_since: None,
        }
    }

    pub fn limit(&self) -> Option<u64> {
        self.rate.is_finite().then(|| {
            // A rate is a few hundred thousand kbit/s at most.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive rate far under u64's range"
            )]
            let kbit = self.rate.round() as u64;
            kbit
        })
    }

    pub fn base(&self) -> Option<f64> {
        self.base
    }

    /// Whether carrying `carried` leaves this direction idle: no flow, or
    /// a sliver of its rate.
    fn idle(&self, carried: f64) -> bool {
        carried < FLOWING_KBIT || (self.rate.is_finite() && carried < self.rate * IDLE)
    }

    /// Take in a moment: this direction's own delay, with the air and the
    /// other direction's path taken off, and what it carried.
    pub fn step(&mut self, at: f64, delay_ms: f64, carried: f64) {
        let dt = self.last_at.map_or(0.0, |last| (at - last).max(0.0));
        self.last_at = Some(at);
        if delay_ms > LATE_MS {
            self.late_since.get_or_insert(at);
        } else {
            self.late_since = None;
        }
        let standing = self
            .late_since
            .is_some_and(|since| at - since >= STANDING_S);
        let flowing = carried > FLOWING_KBIT;
        if standing && flowing && at - self.last_cut >= BETWEEN_CUTS_S {
            self.cut(at, delay_ms, carried);
        } else if delay_ms < CLEAR_MS && carried >= self.rate * FULL {
            // As TCP's slow start does: far from any queue it has met, a
            // rate climbs fast, since nothing says the line is near, and
            // near the last one gently.
            let far = self.ceiling.is_none_or(|ceiling| self.rate < ceiling * FAR);
            self.rate *= if far { FAST_CLIMB } else { CLIMB }.powf(dt);
        } else if let Some(base) = self.base
            && carried < self.rate * FULL
            && !standing
        {
            let toward = (1.0 + DRIFT).powf(dt);
            self.rate = if self.rate > base {
                (self.rate / toward).max(base)
            } else {
                (self.rate * toward).min(base)
            };
        }
    }

    fn cut(&mut self, at: f64, delay_ms: f64, carried: f64) {
        // With the rate well over what got through, the line, not this
        // machine, set what it carried, and that is what the base learns.
        if carried * 1.15 <= self.rate {
            self.learned.push_back(carried);
            if self.learned.len() > LEARNED {
                self.learned.pop_front();
            }
            let mut sorted: Vec<f64> = self.learned.iter().copied().collect();
            sorted.sort_by(f64::total_cmp);
            self.base = Some(sorted[sorted.len() / 2] * 0.9);
        }
        let drain = 1.0 - (delay_ms / 1_000.0).min(0.5);
        self.rate = (carried.min(self.rate) * 0.9 * drain).max(LEAST_KBIT);
        self.last_cut = at;
        self.ceiling = Some(carried);
    }

    /// Whether the guard should send this direction back to its base: a
    /// delay standing unbroken past what cuts could drain, or held under
    /// half its base for a minute with no queue standing.
    fn misbehaving(&mut self, at: f64) -> bool {
        let futile = self.late_since.is_some_and(|since| at - since >= FUTILE_S);
        let held_down = match self.base {
            Some(base) if self.rate < base * 0.5 && self.late_since.is_none() => {
                at - *self.under_half_since.get_or_insert(at) >= GUARD_WINDOW_S
            }
            _ => {
                self.under_half_since = None;
                false
            }
        };
        futile || held_down
    }

    fn reset(&mut self) {
        self.rate = self.base.unwrap_or(f64::INFINITY);
        self.under_half_since = None;
        self.late_since = None;
    }
}

/// Both directions on one network, and what stands over them.
#[derive(Clone, Debug)]
pub struct Controller {
    pub up: Way,
    pub down: Way,
    air_base: Option<f64>,
    last_at: Option<f64>,
    answered_at: Option<f64>,
    started_at: Option<f64>,
    returns: usize,
}

impl Controller {
    /// A controller for a network whose bases, up and down, were learned
    /// before, or not.
    pub fn new(up: Option<f64>, down: Option<f64>) -> Controller {
        Controller {
            up: Way::new(up),
            down: Way::new(down),
            air_base: None,
            last_at: None,
            answered_at: None,
            started_at: None,
            returns: 0,
        }
    }

    /// Whether the guard has given up on this stay.
    pub fn given_up(&self) -> bool {
        self.returns >= GUARD_RETURNS
    }

    /// Take in a moment, and give what each direction is held to.
    pub fn step(&mut self, seen: &Seen) -> Limits {
        let at = seen.at_s;
        let dt = self.last_at.map_or(0.0, |last| (at - last).max(0.0));
        self.last_at = Some(at);
        self.started_at.get_or_insert(at);
        let air = seen.gateway_ms.map_or(0.0, |gateway| {
            let base = self
                .air_base
                .map_or(gateway, |b| gateway.min(b + AIR_RISE_MS * dt));
            self.air_base = Some(base);
            (gateway - base - AIR_MARGIN_MS).max(0.0)
        });
        if seen.up_delay_ms.is_some() || seen.down_delay_ms.is_some() {
            self.answered_at = Some(at);
        }
        let unanswered = self
            .answered_at
            .or(self.started_at)
            .is_some_and(|since| at - since >= UNANSWERED_S);
        if self.given_up() || unanswered {
            return Limits {
                up: None,
                down: None,
            };
        }
        let up = seen.up_delay_ms.map(|d| (d - air).max(0.0));
        let down = seen.down_delay_ms.map(|d| (d - air).max(0.0));
        let idle_up = self.up.idle(seen.up_kbit);
        let idle_down = self.down.idle(seen.down_kbit);
        if let Some(delay) = up {
            let beside = if idle_down { down.unwrap_or(0.0) } else { 0.0 };
            self.up.step(at, (delay - beside).max(0.0), seen.up_kbit);
        }
        if let Some(delay) = down {
            let beside = if idle_up { up.unwrap_or(0.0) } else { 0.0 };
            self.down
                .step(at, (delay - beside).max(0.0), seen.down_kbit);
        }
        for way in [&mut self.up, &mut self.down] {
            if way.misbehaving(at) {
                way.reset();
                self.returns += 1;
            }
        }
        if self.given_up() {
            return Limits {
                up: None,
                down: None,
            };
        }
        Limits {
            up: self.up.limit(),
            down: self.down.limit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tenth of a second at a time from `from` to `to`, each moment as
    /// `seen` makes it.
    fn run(c: &mut Controller, from: f64, to: f64, seen: impl Fn(f64) -> Seen) -> Limits {
        let mut limits = Limits {
            up: None,
            down: None,
        };
        let mut at = from;
        while at < to {
            limits = c.step(&Seen {
                at_s: at,
                ..seen(at)
            });
            at += 0.1;
        }
        limits
    }

    /// A wide line from `from` to `to` carrying the whole of whatever the
    /// download is held to, with nothing waiting.
    fn wide(c: &mut Controller, from: f64, to: f64) -> Limits {
        let mut limits = Limits {
            up: c.up.limit(),
            down: c.down.limit(),
        };
        let mut at = from;
        while at < to {
            let down_kbit = limits.down.map_or(1_000_000.0, crate::rate::float);
            limits = c.step(&Seen {
                at_s: at,
                down_kbit,
                ..quiet(300.0, 0.0)(at)
            });
            at += 0.1;
        }
        limits
    }

    fn quiet(up_kbit: f64, down_kbit: f64) -> impl Fn(f64) -> Seen {
        move |_| Seen {
            up_delay_ms: Some(1.0),
            down_delay_ms: Some(1.0),
            gateway_ms: Some(3.0),
            up_kbit,
            down_kbit,
            ..Seen::default()
        }
    }

    fn queued(down_ms: f64, down_kbit: f64) -> impl Fn(f64) -> Seen {
        move |_| Seen {
            up_delay_ms: Some(0.0),
            down_delay_ms: Some(down_ms),
            gateway_ms: Some(3.0),
            up_kbit: 300.0,
            down_kbit,
            ..Seen::default()
        }
    }

    #[test]
    fn a_new_network_is_left_unlimited_until_its_queue_stands() {
        let mut c = Controller::new(None, None);
        let limits = run(&mut c, 0.0, 30.0, quiet(5_000.0, 200_000.0));
        assert_eq!(
            limits,
            Limits {
                up: None,
                down: None
            }
        );
    }

    #[test]
    fn a_standing_queue_cuts_a_tenth_under_what_got_through_and_learns_the_line() {
        let mut c = Controller::new(None, None);
        let limits = run(&mut c, 0.0, 1.15, queued(40.0, 300_000.0));
        // 300,000 × 0.9 × (1 − 0.04).
        assert_eq!(limits.down, Some(259_200));
        assert_eq!(limits.up, None, "the upload's own delay never rose");
        assert_eq!(c.down.base().map(f64::round), Some(270_000.0));
    }

    #[test]
    fn a_late_spell_under_a_second_isnt_a_queue() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 0.8, queued(40.0, 300_000.0));
        let limits = run(&mut c, 0.8, 5.0, queued(1.0, 300_000.0));
        assert_eq!(limits.down, None);
    }

    #[test]
    fn a_direction_with_no_flow_isnt_cut() {
        let mut c = Controller::new(None, None);
        let limits = run(&mut c, 0.0, 5.0, queued(40.0, 100.0));
        assert_eq!(limits.down, None);
    }

    #[test]
    fn an_idle_direction_drifts_back_to_its_base_and_no_further() {
        let mut way = Way::new(Some(100_000.0));
        way.rate = 150_000.0;
        let mut at = 0.0;
        while at < 20.0 {
            way.step(at, 1.0, 0.0);
            at += 0.1;
        }
        let rate = way.limit().expect("held");
        assert!(
            (120_000..=125_000).contains(&rate),
            "1% a second toward base: {rate}"
        );
        while at < 60.0 {
            way.step(at, 1.0, 0.0);
            at += 0.1;
        }
        assert_eq!(way.limit(), Some(100_000));
    }

    #[test]
    fn the_base_is_the_middle_of_the_last_ten_less_a_tenth() {
        let mut way = Way::new(None);
        let mut at = 0.0;
        for carried in [
            100.0, 900.0, 200.0, 800.0, 300.0, 700.0, 400.0, 600.0, 500.0, 1_000.0, 50.0,
        ] {
            way.rate = f64::INFINITY;
            way.cut(at, 20.0, carried * 1_000.0);
            at += 2.0;
        }
        // The first, 100, is gone; of the rest the middle is 600.
        assert_eq!(way.base().map(f64::round), Some(540_000.0));
    }

    #[test]
    fn the_airs_lateness_comes_off_both_directions() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 5.0, quiet(5_000.0, 200_000.0));
        let late_air = |_| Seen {
            up_delay_ms: Some(31.0),
            down_delay_ms: Some(31.0),
            gateway_ms: Some(41.0),
            up_kbit: 5_000.0,
            down_kbit: 200_000.0,
            ..Seen::default()
        };
        let limits = run(&mut c, 5.0, 10.0, late_air);
        assert_eq!(
            limits,
            Limits {
                up: None,
                down: None
            }
        );
    }

    #[test]
    fn an_idle_directions_delay_is_the_paths_and_comes_off_the_other() {
        let mut c = Controller::new(Some(10_000.0), Some(200_000.0));
        let path = |_| Seen {
            up_delay_ms: Some(30.0),
            down_delay_ms: Some(35.0),
            gateway_ms: Some(3.0),
            up_kbit: 0.0,
            down_kbit: 199_000.0,
            ..Seen::default()
        };
        let limits = run(&mut c, 0.0, 5.0, path);
        assert!(
            limits.down >= Some(200_000),
            "only 5 ms past the path's: {limits:?}"
        );
    }

    #[test]
    fn no_timestamps_for_ten_seconds_leaves_the_line_unlimited() {
        let mut c = Controller::new(Some(10_000.0), Some(200_000.0));
        let unanswered = |_| Seen {
            gateway_ms: Some(3.0),
            up_kbit: 5_000.0,
            down_kbit: 100_000.0,
            ..Seen::default()
        };
        let early = run(&mut c, 0.0, 5.0, unanswered);
        assert_eq!(early.down, Some(200_000), "not yet ten seconds");
        let limits = run(&mut c, 5.0, 12.0, unanswered);
        assert_eq!(
            limits,
            Limits {
                up: None,
                down: None
            }
        );
    }

    #[test]
    fn a_rate_that_has_never_met_its_queue_climbs_a_quarter_a_second() {
        let mut c = Controller::new(None, Some(16_000.0));
        let limits = wide(&mut c, 0.0, 10.0);
        // 16 Mbit/s carried whole for ten seconds at 1.25 a second is 149.
        assert_eq!(limits.down.map(|kbit| kbit / 1_000), Some(149));
    }

    #[test]
    fn near_where_its_queue_last_stood_a_rate_climbs_gently() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.2, queued(40.0, 100_000.0));
        let cut = c.down.limit().map(crate::rate::float).expect("cut");
        let limits = wide(&mut c, 1.2, 11.2);
        let climbed = limits.down.map(crate::rate::float).expect("held") / cut;
        // Ten seconds at 1.05 a second is 1.63.
        assert!((1.6..1.66).contains(&climbed), "climbed {climbed}");
    }

    #[test]
    fn a_line_filling_its_buffer_is_cut_each_second_without_the_guard() {
        let mut c = Controller::new(None, Some(200_000.0));
        for n in 0..20 {
            let from = f64::from(n) * 3.0;
            run(&mut c, from, from + 2.5, queued(40.0, 150_000.0));
            run(&mut c, from + 2.5, from + 3.0, quiet(300.0, 150_000.0));
        }
        assert!(!c.given_up());
        assert!(
            c.down.limit() < Some(150_000),
            "still cut: {:?}",
            c.down.limit()
        );
    }

    #[test]
    fn a_queue_standing_half_a_minute_whatever_the_cuts_goes_back_to_its_base() {
        let mut c = Controller::new(None, Some(200_000.0));
        let early = run(&mut c, 0.0, 29.0, queued(40.0, 150_000.0));
        assert!(early.down < Some(10_000), "cut to the least: {early:?}");
        let limits = run(&mut c, 29.0, 30.2, queued(40.0, 150_000.0));
        assert_eq!(limits.down, Some(200_000));
    }

    #[test]
    fn held_under_half_its_base_a_minute_with_no_queue_goes_back() {
        let mut c = Controller::new(None, Some(200_000.0));
        run(&mut c, 0.0, 1.2, queued(40.0, 50_000.0));
        assert!(c.down.limit() < Some(100_000));
        let limits = run(&mut c, 1.2, 62.5, |_| Seen {
            up_delay_ms: Some(1.0),
            down_delay_ms: Some(10.0),
            gateway_ms: Some(3.0),
            up_kbit: 300.0,
            down_kbit: 40_000.0,
            ..Seen::default()
        });
        assert_eq!(limits.down, Some(200_000));
    }

    #[test]
    fn a_single_cut_under_another_machines_flow_keeps_a_known_base() {
        let mut c = Controller::new(None, Some(200_000.0));
        run(&mut c, 0.0, 1.2, queued(40.0, 50_000.0));
        assert_eq!(c.down.base().map(f64::round), Some(200_000.0));
    }

    #[test]
    fn three_returns_and_the_line_is_left_unlimited() {
        let mut c = Controller::new(None, Some(200_000.0));
        run(&mut c, 0.0, 95.0, queued(40.0, 150_000.0));
        assert!(c.given_up());
        let limits = run(&mut c, 95.0, 100.0, queued(40.0, 150_000.0));
        assert_eq!(
            limits,
            Limits {
                up: None,
                down: None
            }
        );
    }
}
