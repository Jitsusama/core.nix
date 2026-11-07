//! The shaper's rebuilt controller: each direction's rate from that
//! direction's own delay, as ICMP timestamps measure it, rather than from
//! round trips, which mix both directions and the air.
//!
//! A direction is cut once its own delay has stood over `LATE_MS` for a
//! second while it carries a flow: to a tenth under what it carried, and
//! more the deeper the queue; carrying a sliver of what it could, it's
//! halved instead, since the queue is mostly another machine's. It climbs
//! while it's full with its delay short of a queue: gently from half of
//! where its queue last stood, in the half minute before, to a quarter
//! past it, and fast anywhere else. It drifts back toward the network's
//! line while it's idle or low. The base
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
//! half a minute however far it was cut, or stood nearly all of it while
//! the direction carried a sliver of its line, so that the queue is plainly
//! another machine's, goes back to its base; after three such returns the
//! controller gives up and leaves the line unlimited for ten minutes, as it
//! does when no timestamps are answered. Cuts alone don't trip it: a line
//! filling its buffer is rightly cut each second until it drains.
//!
//! The line is also what the direction carried for five seconds running
//! with no queue, so a queue met while it carried a sliver can't teach it
//! a line under what it has already carried.

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
/// A direction carrying under this share of what it could carry, its rate
/// or its line, when its queue stands carries a sliver: the queue is
/// mostly another machine's or the air's, so a cut halves what it could
/// carry, and teaches neither the base nor where the queue stood.
const SLIVER: f64 = 0.25;
/// A queue standing this share of `FUTILE_S` while the direction carries a
/// sliver of its line is another machine's, however it breaks between.
const CROWDED: f64 = 0.5;
/// How long the line is left unlimited once the guard gives up.
const GIVEN_UP_S: f64 = 600.0;
/// How long a direction must carry a rate with no queue to prove the line
/// carries it.
const PROVEN_S: f64 = 5.0;
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
/// How long where a queue last stood slows the climb: past it, as CUBIC
/// does, the line has likely moved since.
const CEILING_S: f64 = 30.0;
/// A rate this far past where its queue last stood, with none standing, is
/// on a line that has grown, and climbs fast again.
const PAST: f64 = 1.25;
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
    /// stay, and when; None until it first does.
    ceiling: Option<(f64, f64)>,
    last_cut: f64,
    last_at: Option<f64>,
    under_half_since: Option<f64>,
    /// What it carried each moment of the clear spell it is in, for the
    /// last `PROVEN_S`, and when the spell began.
    clear: VecDeque<(f64, f64)>,
    clear_since: Option<f64>,
    /// The most it carried for `PROVEN_S` running with no queue, in this
    /// stay: the line carries at least that.
    proven: f64,
    /// Each moment of the last `FUTILE_S`, and whether its queue stood
    /// while it carried a sliver of its line.
    crowded: VecDeque<(f64, bool)>,
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
            clear: VecDeque::new(),
            clear_since: None,
            proven: 0.0,
            crowded: VecDeque::new(),
        }
    }

    /// What the line is known to carry: its base, or what it was proven to
    /// carry, whichever is more, less a tenth of the proof as the base is.
    fn line(&self) -> Option<f64> {
        let proven = (self.proven > 0.0).then_some(self.proven * 0.9);
        match (self.base, proven) {
            (Some(base), Some(proven)) => Some(base.max(proven)),
            (base, proven) => base.or(proven),
        }
    }

    /// Keep what a clear spell proves the line carries.
    fn prove(&mut self, at: f64, delay_ms: f64, carried: f64) {
        if delay_ms >= CLEAR_MS {
            self.clear.clear();
            self.clear_since = None;
            return;
        }
        let since = *self.clear_since.get_or_insert(at);
        self.clear.push_back((at, carried));
        while self.clear.front().is_some_and(|&(t, _)| at - t > PROVEN_S) {
            self.clear.pop_front();
        }
        if at - since >= PROVEN_S {
            let least = self
                .clear
                .iter()
                .map(|&(_, c)| c)
                .fold(f64::INFINITY, f64::min);
            self.proven = self.proven.max(least);
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

    /// Whether carrying `carried` leaves this direction idle: no flow, or
    /// a sliver of its rate.
    fn idle(&self, carried: f64) -> bool {
        carried < FLOWING_KBIT || (self.rate.is_finite() && carried < self.rate * IDLE)
    }

    /// Take in a moment: this direction's own delay, with the air and the
    /// other direction's path taken off, and what it carried.
    pub fn step(&mut self, at: f64, delay_ms: f64, carried: f64) {
        // A gap, while the line was left unlimited or asleep, is one
        // moment, not a climb all through it.
        let dt = self.last_at.map_or(0.0, |last| (at - last).clamp(0.0, 1.0));
        self.last_at = Some(at);
        self.prove(at, delay_ms, carried);
        let crowded = delay_ms > LATE_MS && self.line().is_some_and(|line| carried < line * SLIVER);
        self.crowded.push_back((at, crowded));
        while self
            .crowded
            .front()
            .is_some_and(|&(t, _)| at - t > FUTILE_S)
        {
            self.crowded.pop_front();
        }
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
        } else if carried >= self.rate * FULL && delay_ms <= LATE_MS {
            // As TCP's slow start does: far from any queue it has met, a
            // rate climbs fast, since nothing says the line is near, and
            // near the last one gently. A little delay short of a queue
            // doesn't stop it: the line carried more with no more than
            // that.
            let far = self.ceiling.is_none_or(|(ceiling, met)| {
                self.rate < ceiling * FAR || self.rate > ceiling * PAST || at - met >= CEILING_S
            });
            self.rate *= if far { FAST_CLIMB } else { CLIMB }.powf(dt);
        } else if let Some(base) = self.line()
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
        // Cut under a sliver, a queue this machine hardly feeds drains no
        // faster and the machine starves, so the reach is halved instead.
        // A line that really fell that far is met in a few halvings; a
        // queue no cut drains is the guard's.
        let reach = self.line().map_or(self.rate, |line| line.min(self.rate));
        if reach.is_finite() && carried < reach * SLIVER {
            self.rate = (reach * 0.5).max(LEAST_KBIT);
            self.last_cut = at;
            return;
        }
        // With the rate well over what got through, the line, not this
        // machine, set what it carried, and that is what the base learns.
        if carried * 1.15 <= self.rate {
            self.learned.push_back(carried);
            if self.learned.len() > LEARNED {
                self.learned.pop_front();
            }
            let mut sorted: Vec<f64> = self.learned.iter().copied().collect();
            sorted.sort_by(f64::total_cmp);
            self.base = Some((sorted[sorted.len() / 2] * 0.9).max(self.proven * 0.9));
        }
        let drain = 1.0 - (delay_ms / 1_000.0).min(0.5);
        self.rate = (carried.min(self.rate) * 0.9 * drain).max(LEAST_KBIT);
        self.last_cut = at;
        self.ceiling = Some((carried, at));
    }

    /// Whether the guard should send this direction back to its base: a
    /// delay standing unbroken past what cuts could drain, or standing
    /// nearly all that time while it carried a sliver of its line, or held
    /// under half its base for a minute with no queue standing.
    fn misbehaving(&mut self, at: f64) -> bool {
        let unbroken = self.late_since.is_some_and(|since| at - since >= FUTILE_S);
        let spanned = self
            .crowded
            .front()
            .is_some_and(|&(t, _)| at - t >= FUTILE_S - 1.0);
        let crowded = self.crowded.iter().filter(|&&(_, c)| c).count();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window of a few hundred moments"
        )]
        let share = crowded as f64 / self.crowded.len().max(1) as f64;
        // A direction held to nothing has nothing to go back from.
        let futile = self.rate.is_finite() && (unbroken || spanned && share >= CROWDED);
        let held_down = match self.line() {
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
        self.rate = self.line().unwrap_or(f64::INFINITY);
        self.under_half_since = None;
        self.late_since = None;
        self.crowded.clear();
        self.last_at = None;
    }

    /// Start over as on arriving, unlimited until a queue stands, keeping
    /// what was learned of the line, so a limit is never set without one.
    fn start_over(&mut self) {
        self.reset();
        self.rate = f64::INFINITY;
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
    gave_up_at: Option<f64>,
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
            gave_up_at: None,
        }
    }

    /// Whether the guard has given up, for `GIVEN_UP_S`.
    pub fn given_up(&self) -> bool {
        self.returns >= GUARD_RETURNS
    }

    /// Take in a moment, and give what each direction is held to.
    pub fn step(&mut self, seen: &Seen) -> Limits {
        let at = seen.at_s;
        let dt = self.last_at.map_or(0.0, |last| (at - last).max(0.0));
        self.last_at = Some(at);
        self.started_at.get_or_insert(at);
        if self
            .gave_up_at
            .is_some_and(|since| at - since >= GIVEN_UP_S)
        {
            self.gave_up_at = None;
            self.returns = 0;
            self.up.start_over();
            self.down.start_over();
        }
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
        if unanswered {
            self.up.start_over();
            self.down.start_over();
        }
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
            self.gave_up_at.get_or_insert(at);
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

/// Under replay, a recorded second is what the controller sees, and the
/// rates it answers hold through the next.
impl crate::replay::Controller for Controller {
    fn second(&mut self, s: &crate::replay::Second) -> crate::replay::Rates {
        let limits = self.step(&Seen {
            at_s: s.t_s,
            up_delay_ms: s.up_delay_ms,
            down_delay_ms: s.down_delay_ms,
            gateway_ms: s.gateway_ms,
            up_kbit: s.up_kbit,
            down_kbit: s.down_kbit,
        });
        crate::replay::Rates {
            up: limits.up,
            down: limits.down,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{self, Base, Stay};

    #[test]
    fn a_replayed_queue_is_cut_with_cause_and_an_idle_line_isnt_held_low() {
        let second = |t: u64, down_ms: f64, down_kbit: f64| replay::Second {
            t_s: crate::rate::float(t),
            up_kbit: 300.0,
            down_kbit,
            up_delay_ms: Some(0.0),
            down_delay_ms: Some(down_ms),
            gateway_ms: Some(3.0),
        };
        // Two seconds full with a queue standing, as a cut drains it, then
        // five minutes idle.
        let seconds = (0..2)
            .map(|t| second(t, 40.0, 200_000.0))
            .chain((2..302).map(|t| second(t, 1.0, 100.0)))
            .collect();
        let stay = Stay {
            network: "home".into(),
            began_ms: 0,
            seconds,
        };
        let base = Base {
            up: 10_000,
            down: 200_000,
        };
        let report = replay::replay(&[stay], Some(base), || Controller::new(None, None));
        assert_eq!(report.down.cuts, 1, "{:?}", report.down);
        assert_eq!(
            report.down.unfounded, 0,
            "{:?}",
            report.stays[0].unfounded_cuts
        );
        assert_eq!(report.up.cuts, 0);
        assert_eq!(
            report.down.held_idle_s, 0,
            "drifted back: {:?}",
            report.down
        );
    }

    #[test]
    fn a_queue_under_a_sliver_of_the_line_doesnt_hold_the_line_low() {
        // As the replay of 7 November found: a download carrying 100
        // Mbit/s with no queue, then a queue of 39 ms standing while it
        // carried 737 kbit/s, so the queue was another machine's or the
        // air's, then 15 Mbit/s with none. Cut to under the 737 and taught
        // it as the line, the controller held the download under half the
        // line for the next five minutes.
        let second = |t: u64, down_ms: f64, down_kbit: f64| replay::Second {
            t_s: crate::rate::float(t),
            up_kbit: 300.0,
            down_kbit,
            up_delay_ms: Some(0.0),
            down_delay_ms: Some(down_ms),
            gateway_ms: Some(3.0),
        };
        let seconds = (0..60)
            .map(|t| second(t, 1.0, 100_000.0))
            .chain((60..62).map(|t| second(t, 39.0, 737.0)))
            .chain((62..362).map(|t| second(t, 2.0, 15_000.0)))
            .collect();
        let stay = Stay {
            network: "home".into(),
            began_ms: 0,
            seconds,
        };
        let base = Base {
            up: 10_000,
            down: 100_000,
        };
        let report = replay::replay(&[stay], Some(base), || Controller::new(None, None));
        assert_eq!(report.down.starved_s, 0, "{:?}", report.down);
        assert_eq!(report.down.held_idle_s, 0, "{:?}", report.down);
    }

    #[test]
    fn another_machines_queue_breaking_now_and_then_goes_back_to_the_line() {
        // As the replay of 7 November found: with the line proven at 90
        // Mbit/s, the download's own delay stood at 16 to 30 ms for three
        // minutes, clearing a moment now and then, while this machine
        // carried 2 Mbit/s. Every break restarted the guard's half minute,
        // so the download sat at the least, which no call fits in.
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 10.0, quiet(300.0, 100_000.0));
        for n in 0..4 {
            let from = 10.0 + f64::from(n) * 10.0;
            run(&mut c, from, from + 9.5, queued(22.0, 2_000.0));
            run(&mut c, from + 9.5, from + 10.0, queued(4.0, 2_000.0));
        }
        assert_eq!(c.returns, 1, "{:?}", c.down.limit());
    }

    #[test]
    fn ten_minutes_after_giving_up_the_line_is_shaped_again() {
        let mut c = Controller::new(None, Some(200_000.0));
        run(&mut c, 0.0, 95.0, queued(40.0, 150_000.0));
        assert!(c.given_up());
        let limits = run(&mut c, 95.0, 700.0, quiet(300.0, 100_000.0));
        assert!(!c.given_up());
        assert_eq!(limits.down, None, "unlimited until a queue stands");
        let limits = run(&mut c, 700.0, 701.2, queued(40.0, 150_000.0));
        assert!(limits.down < Some(150_000), "shaped again: {limits:?}");
    }

    #[test]
    fn under_half_its_line_a_full_direction_climbs_through_a_little_delay() {
        // As the replay found of uploads on Wi-Fi: cut, then full with its
        // own delay between 5 and 15 ms, a rate under half the line was
        // held there for minutes while the line carried more.
        let mut c = Controller::new(None, Some(250_000.0));
        run(&mut c, 0.0, 2.2, queued(40.0, 100_000.0));
        assert!(c.down.limit() < Some(125_000), "cut: {:?}", c.down.limit());
        let mut at = 2.2;
        while at < 22.2 {
            let down_kbit = c.down.limit().map_or(1_000_000.0, crate::rate::float);
            c.step(&Seen {
                at_s: at,
                down_kbit,
                ..queued(8.0, 0.0)(at)
            });
            at += 0.1;
        }
        assert!(
            c.down.limit() >= Some(125_000),
            "climbed: {:?}",
            c.down.limit()
        );
    }

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
        assert_eq!(c.down.base.map(f64::round), Some(270_000.0));
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
            100.0, 900.0, 450.0, 800.0, 500.0, 700.0, 550.0, 600.0, 650.0, 1_000.0, 400.0,
        ] {
            way.rate = f64::INFINITY;
            way.cut(at, 20.0, carried * 1_000.0);
            at += 2.0;
        }
        // The first, 100, is gone; of the rest the middle is 650.
        assert_eq!(way.base.map(f64::round), Some(585_000.0));
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
        // Answered again, the old limits don't come back without a queue,
        // as the replay of 5 November found they did.
        let back = run(&mut c, 12.0, 15.0, quiet(5_000.0, 100_000.0));
        assert_eq!(back.down, None, "unlimited until a queue stands");
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
        let limits = wide(&mut c, 1.2, 8.2);
        let climbed = limits.down.map(crate::rate::float).expect("held") / cut;
        // Seven seconds at 1.05 a second is 1.41, still short of a quarter
        // past where the queue stood.
        assert!((1.38..1.43).contains(&climbed), "climbed {climbed}");
        let limits = wide(&mut c, 8.2, 11.2);
        let climbed = limits.down.map(crate::rate::float).expect("held") / cut;
        assert!(climbed > 2.0, "past it, fast again: {climbed}");
    }

    #[test]
    fn far_under_its_last_queue_a_little_delay_climbs_fast() {
        // As the replay found at a stay's start: held at 2.7 Mbit/s while
        // 315 got through with 6.5 ms of delay, it climbed 5% a second.
        let mut c = Controller::new(None, Some(250_000.0));
        run(&mut c, 0.0, 1.2, queued(40.0, 100_000.0));
        c.down.rate = 10_000.0;
        let mut at = 1.2;
        while at < 6.2 {
            let down_kbit = c.down.limit().map_or(1_000_000.0, crate::rate::float);
            c.step(&Seen {
                at_s: at,
                down_kbit,
                ..queued(8.0, 0.0)(at)
            });
            at += 0.1;
        }
        // Five seconds at 1.25 a second is 3.05; at 1.05, 1.28.
        assert!(
            c.down.limit() > Some(29_000),
            "climbed: {:?}",
            c.down.limit()
        );
    }

    #[test]
    fn a_queue_met_half_a_minute_ago_no_longer_slows_the_climb() {
        // As the replay found of downloads on Wi-Fi: cut where the air
        // once queued, a rate climbed gently for many seconds after the
        // air had cleared and the line carried twice that.
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.2, queued(40.0, 100_000.0));
        run(&mut c, 1.2, 32.0, quiet(300.0, 1_000.0));
        let held = c.down.limit().map(crate::rate::float).expect("held");
        let limits = wide(&mut c, 32.0, 37.0);
        let climbed = limits.down.map(crate::rate::float).expect("held") / held;
        // Five seconds at 1.25 a second is 3.05.
        assert!(climbed > 2.9, "climbed {climbed}");
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
        assert_eq!(c.down.base.map(f64::round), Some(200_000.0));
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
