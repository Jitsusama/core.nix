//! How fast a link may send and receive, found from what its delay does.
//!
//! The line beyond the router has a queue of its own, and it only stays
//! empty while this machine sends and receives a little slower than the
//! line can carry. How fast that is differs from network to network and
//! from minute to minute on Wi-Fi or a phone's tether, so it's found as it
//! goes, as cake-autorate does on routers: pings to a few distant hosts
//! each keep a baseline, the least round trip seen lately, and a ping that
//! comes back well over its baseline is delayed. While enough of the latest
//! pings are delayed and a direction is busy, that direction's rate drops
//! to a little under what it was carrying, since that is what the line
//! managed with a queue building, and that rate is held as the line's. The
//! queue then takes a while to drain, and the pings stay late meanwhile:
//! later cuts in the same spell lower the rate further, so it drains
//! sooner, but leave the held rate alone. Nothing climbs until a whole
//! window of fresh pings says the line is quiet. Then a busy direction
//! climbs quickly back to the held rate, and above it only slowly, so a
//! probe past what the line takes builds a few milliseconds of queue before
//! the pings see it, not a few hundred. While a direction rests, its rate
//! settles back towards the held rate, so the next burst starts there.

use std::collections::{HashMap, VecDeque};

/// Rates are in kbit/s, as tc takes them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: u64,
    pub max: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// A ping this far over its baseline is delayed.
    pub delay_ms: f64,
    /// How many of the latest pings are looked at, and how many of those
    /// must be delayed for the queue to be building.
    pub window: usize,
    pub delayed_needed: usize,
    /// A direction carrying this share of its rate is busy enough to climb.
    pub busy: f64,
    /// A direction carrying this share of its rate is loaded enough to be
    /// the one filling the queue.
    pub loaded: f64,
    /// What a cut leaves of what the direction carried.
    pub cut: f64,
    /// How much a climb adds each step, below the held rate and above it.
    pub climb_back: f64,
    pub climb: f64,
    /// Seconds without a cut before delay counts as a new spell.
    pub spell_s: f64,
    /// How far a resting rate moves towards the held rate each step.
    pub settle: f64,
    /// Seconds after a cut before another: the pings in flight still carry
    /// the queue the cut was for.
    pub after_cut_s: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            // About what a call notices; cake-autorate's own threshold for
            // round trips sits between 15 and 30 ms.
            delay_ms: 15.0,
            window: 6,
            delayed_needed: 3,
            busy: 0.75,
            loaded: 0.5,
            cut: 0.9,
            // About 10% a second at five steps a second, and about 2.5%:
            // a line that dipped to half is found again in about 30 s.
            climb_back: 1.02,
            climb: 1.005,
            spell_s: 2.0,
            settle: 0.05,
            after_cut_s: 0.6,
        }
    }
}

/// One direction of a link: the rate the shaper holds it to now, and the
/// rate it last held under load without delay, which it settles back to.
#[derive(Clone, Debug, PartialEq)]
pub struct Direction {
    pub bounds: Bounds,
    pub rate: u64,
    pub held: u64,
    /// Whether the held rate came from this link's own load since it was
    /// last remembered, rather than from memory: two links on one router
    /// share a memory, and an idle one mustn't write back what it was told.
    pub learned: bool,
}

impl Direction {
    pub fn new(bounds: Bounds, held: u64) -> Self {
        let held = held.clamp(bounds.min, bounds.max);
        Direction {
            bounds,
            rate: held,
            held,
            learned: false,
        }
    }

    fn set(&mut self, rate: f64) {
        self.rate = (rate.round() as u64).clamp(self.bounds.min, self.bounds.max);
    }
}

/// Each distant host's least round trip seen lately. It follows a faster
/// round trip at once and a slower one only over minutes, so a queue that
/// lasts a while isn't taken for the path's own delay.
#[derive(Clone, Debug, Default)]
struct Baseline {
    ms: Option<f64>,
    /// Whether the host's latest ping came back.
    answered: bool,
}

impl Baseline {
    fn update(&mut self, rtt: f64) -> f64 {
        let base = match self.ms {
            None => rtt,
            Some(base) if rtt < base => base * 0.1 + rtt * 0.9,
            Some(base) => base * 0.999 + rtt * 0.001,
        };
        self.ms = Some(base);
        self.answered = true;
        rtt - base
    }

    /// Whether a lost ping says anything: only the first loss after a reply
    /// does. A host that never answers, or stopped, as one that refuses
    /// pings from this network does, would otherwise read as a queue for
    /// as long as it stays silent, while a queue that overflows loses the
    /// next ping to every host at once.
    fn lost(&mut self) -> bool {
        std::mem::replace(&mut self.answered, false)
    }
}

#[derive(Clone, Debug)]
pub struct Controller {
    pub settings: Settings,
    pub up: Direction,
    pub down: Direction,
    baselines: HashMap<String, Baseline>,
    recent: VecDeque<bool>,
    last_cut: Option<f64>,
}

/// One step's climb, from a direction's rate and what it held.
fn climbed(s: &Settings, dir: &Direction) -> f64 {
    let rate = dir.rate as f64;
    if dir.rate < dir.held {
        (rate * s.climb_back).min(dir.held as f64)
    } else {
        rate * s.climb
    }
}

/// What the controller makes of a moment, for the status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Resting,
    Climbing,
    Holding,
    Cut,
}

impl Controller {
    pub fn new(settings: Settings, up: Direction, down: Direction) -> Self {
        Controller {
            settings,
            up,
            down,
            baselines: HashMap::new(),
            recent: VecDeque::new(),
            last_cut: None,
        }
    }

    /// A ping's round trip from one host, or None for one that never came
    /// back, which counts as delayed when the host's last ping answered.
    pub fn sample(&mut self, host: &str, rtt_ms: Option<f64>) {
        let baseline = self.baselines.entry(host.to_string()).or_default();
        let delayed = match rtt_ms {
            Some(rtt) => baseline.update(rtt) > self.settings.delay_ms,
            None if baseline.lost() => true,
            None => return,
        };
        self.recent.push_back(delayed);
        while self.recent.len() > self.settings.window {
            self.recent.pop_front();
        }
    }

    /// Whether the line's queue is building now.
    pub fn building(&self) -> bool {
        self.recent.iter().filter(|d| **d).count() >= self.settings.delayed_needed
    }

    /// Whether a whole window of pings has come since the last cut, too few
    /// of them late: only then may a rate climb.
    pub fn quiet(&self) -> bool {
        self.recent.len() >= self.settings.window && !self.building()
    }

    /// The baselines, for the status.
    pub fn baselines(&self) -> Vec<(String, f64)> {
        let mut all: Vec<(String, f64)> = self
            .baselines
            .iter()
            .filter_map(|(host, b)| b.ms.map(|ms| (host.clone(), ms)))
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    /// One step, given what each direction carried since the last, in
    /// kbit/s, at `now` seconds. Returns what it decided.
    pub fn step(&mut self, now: f64, sent: f64, received: f64) -> State {
        let s = self.settings.clone();
        if self.building() {
            let resting = self.last_cut.is_some_and(|at| now - at < s.after_cut_s);
            let new_spell = self.last_cut.is_none_or(|at| now - at >= s.spell_s);
            let mut cut = false;
            for (dir, carried) in [(&mut self.up, sent), (&mut self.down, received)] {
                if carried >= dir.rate as f64 * s.loaded && !resting {
                    let from = carried.min(dir.rate as f64);
                    dir.set(from * s.cut);
                    if new_spell {
                        dir.held = dir.rate;
                        dir.learned = true;
                    }
                    cut = true;
                }
            }
            if cut {
                self.last_cut = Some(now);
                // The pings that showed the queue are spent; the next cut
                // waits for pings sent at the new rate.
                self.recent.clear();
                return State::Cut;
            }
            return State::Holding;
        }
        if !self.quiet() {
            return State::Holding;
        }
        let mut climbing = false;
        for (dir, carried) in [(&mut self.up, sent), (&mut self.down, received)] {
            if carried >= dir.rate as f64 * s.busy {
                let rate = climbed(&s, dir);
                dir.set(rate);
                if dir.rate > dir.held {
                    dir.held = dir.rate;
                    dir.learned = true;
                }
                climbing = true;
            } else {
                let rate = dir.rate as f64 + (dir.held as f64 - dir.rate as f64) * s.settle;
                dir.set(rate);
            }
        }
        if climbing {
            State::Climbing
        } else {
            State::Resting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(up: u64, down: u64) -> Controller {
        Controller::new(
            Settings::default(),
            Direction::new(
                Bounds {
                    min: 1_000,
                    max: 1_000_000,
                },
                up,
            ),
            Direction::new(
                Bounds {
                    min: 5_000,
                    max: 1_000_000,
                },
                down,
            ),
        )
    }

    fn pings(c: &mut Controller, rtt: f64, n: usize) {
        for i in 0..n {
            c.sample(&format!("host{}", i % 3), Some(rtt));
        }
    }

    #[test]
    fn a_quiet_line_builds_no_queue() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 12);
        assert!(!c.building());
    }

    #[test]
    fn delayed_pings_mean_a_queue_is_building() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        assert!(c.building());
    }

    #[test]
    fn one_late_ping_is_not_a_queue() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 1);
        pings(&mut c, 20.0, 2);
        assert!(!c.building());
    }

    #[test]
    fn lost_pings_count_as_delayed() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        for host in ["host0", "host1", "host2"] {
            c.sample(host, None);
        }
        assert!(c.building());
    }

    #[test]
    fn a_host_that_stays_silent_counts_once() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        for _ in 0..6 {
            c.sample("host0", None);
            c.sample("never", None);
        }
        assert!(!c.building());
    }

    #[test]
    fn a_queue_under_upload_cuts_the_upload_to_under_what_it_carried() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        assert_eq!(c.step(1.0, 11_000.0, 2_000.0), State::Cut);
        assert_eq!(c.up.rate, 9_900);
        assert_eq!(c.up.held, 9_900);
        // The download carried little, so it wasn't filling the queue.
        assert_eq!(c.down.rate, 100_000);
    }

    #[test]
    fn a_cut_waits_for_pings_sent_after_it() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        assert_eq!(c.step(1.2, 0.0, 0.0), State::Holding);
        pings(&mut c, 200.0, 3);
        assert_eq!(c.step(1.4, 9_900.0, 0.0), State::Holding);
        assert_eq!(c.up.rate, 9_900);
        assert_eq!(c.step(1.7, 9_900.0, 0.0), State::Cut);
        assert_eq!(c.up.rate, 8_910);
    }

    #[test]
    fn cuts_while_the_queue_drains_leave_the_held_rate() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        pings(&mut c, 200.0, 3);
        c.step(1.7, 9_900.0, 0.0);
        assert_eq!((c.up.rate, c.up.held), (8_910, 9_900));
    }

    #[test]
    fn delay_after_a_quiet_spell_sets_the_held_rate_again() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(4.0, 8_000.0, 0.0);
        assert_eq!((c.up.rate, c.up.held), (7_200, 7_200));
    }

    #[test]
    fn nothing_climbs_until_a_window_of_fresh_pings_is_quiet() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        pings(&mut c, 20.0, 5);
        assert_eq!(c.step(2.0, 9_900.0, 0.0), State::Holding);
        pings(&mut c, 20.0, 1);
        assert_eq!(c.step(2.2, 9_900.0, 0.0), State::Climbing);
    }

    #[test]
    fn a_queue_nobody_here_fills_changes_nothing() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        assert_eq!(c.step(1.0, 500.0, 1_000.0), State::Holding);
        assert_eq!((c.up.rate, c.down.rate), (20_000, 100_000));
    }

    #[test]
    fn a_busy_line_without_delay_climbs() {
        let mut c = controller(10_000, 100_000);
        pings(&mut c, 20.0, 6);
        assert_eq!(c.step(1.0, 9_500.0, 1_000.0), State::Climbing);
        assert_eq!(c.up.rate, 10_050);
        assert_eq!(c.up.held, 10_050);
        assert!(c.up.learned);
        assert!(!c.down.learned);
    }

    #[test]
    fn a_rate_under_what_the_line_held_climbs_back_quickly() {
        let mut c = controller(10_000, 100_000);
        c.up.rate = 5_000;
        pings(&mut c, 20.0, 6);
        c.step(1.0, 5_000.0, 0.0);
        assert_eq!((c.up.rate, c.up.held), (5_100, 10_000));
        // Back in about seven seconds, then only probing past it.
        let mut steps = 0;
        while c.up.rate < c.up.held && steps < 100 {
            c.step(1.0, c.up.rate as f64, 0.0);
            steps += 1;
        }
        assert_eq!(c.up.rate, 10_000);
        assert!(steps <= 36, "{steps}");
        c.step(1.0, 10_000.0, 0.0);
        assert_eq!(c.up.rate, 10_050);
    }

    #[test]
    fn a_resting_rate_settles_back_to_what_the_line_held() {
        let mut c = controller(10_000, 100_000);
        c.up.rate = 20_000;
        pings(&mut c, 20.0, 6);
        for _ in 0..100 {
            c.step(1.0, 0.0, 0.0);
        }
        assert!((c.up.rate as i64 - 10_000).abs() < 100, "{}", c.up.rate);
    }

    #[test]
    fn rates_stay_within_their_bounds() {
        let mut c = controller(1_000, 5_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 1_000.0, 5_000.0);
        assert_eq!((c.up.rate, c.down.rate), (1_000, 5_000));
    }

    #[test]
    fn a_baseline_follows_a_faster_path_at_once_and_a_slower_one_slowly() {
        let mut b = Baseline::default();
        b.update(40.0);
        b.update(20.0);
        assert!(b.ms.unwrap() < 23.0);
        for _ in 0..100 {
            b.update(200.0);
        }
        assert!(b.ms.unwrap() < 40.0, "{:?}", b.ms);
    }
}
