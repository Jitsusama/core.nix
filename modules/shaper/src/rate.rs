//! How fast a link may send and receive, found from what its delay does.
//!
//! The line beyond the router has a queue of its own, and it only stays
//! empty while this machine sends and receives a little slower than the
//! line can carry. How fast that is differs from network to network and
//! from minute to minute on Wi-Fi or a phone's tether, so it's found as it
//! goes, as cake-autorate does on routers: pings to a few distant hosts
//! each keep a baseline, the least round trip seen lately, and a ping that
//! comes back well over its baseline is delayed. While enough of the latest
//! pings are delayed and a direction is busy, a little under what that
//! direction was carrying is held as the line's rate, since that is what
//! the line managed with a queue building. The rate itself goes further
//! under, by as much as the queue is deep, so the queue drains in about a
//! second rather than the ten a rate a tenth under the line would take
//! over a full modem buffer. The pings stay late while it drains, which is
//! no reason to cut again: a later cut in the same spell comes only while
//! the delay is still rising, as it does when the line is slower than the
//! cut, and the held rate follows it down. Cutting at every late ping took
//! an 8 Mbit/s line's rate to 3. Nothing climbs until a whole
//! window of fresh pings says the line is quiet. Then a busy direction
//! climbs quickly back to the held rate, and above it only slowly, so a
//! probe past what the line takes builds a few milliseconds of queue before
//! the pings see it, not a few hundred. While a direction rests, its rate
//! settles back towards the held rate, so the next burst starts there.
//! A cut long past says little about the line now, and climbing back from
//! it took a download most of a minute at half speed, so a direction that
//! rests long after its last cut holds its best rate again. Should the
//! line still be slow, the first burst's queue cuts it within a second or
//! two, and that cut is kept rather than forgotten again.
//! Only a direction busy for some seconds climbs past its held rate. pi's
//! requests, a megabyte or two each, kept the upload busy a second or two
//! at a time, and each such burst climbed past the line and was cut, so
//! every few seconds a queue of about 20 ms built that calls shared. A
//! burst now fits under the held rate, and a transfer that lasts still
//! looks for more.

use std::collections::{HashMap, VecDeque};

/// A count, a rate in kbit/s or a time in microseconds, as a float. Exact:
/// every one here is far below 2⁵³.
#[allow(clippy::cast_precision_loss)]
pub fn float(n: u64) -> f64 {
    n as f64
}

/// A rate worked out as a float, to the nearest whole kbit/s. A negative
/// rate is none, and every caller holds the result within its bounds.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn whole(kbit: f64) -> u64 {
    kbit.round().max(0.0) as u64
}

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
    /// The one of the two likeliest to fill the queue fills it from this
    /// share. What arrives is capped by the line, so a line fallen under
    /// half of the download's rate brings in under half of it while its
    /// queue grows, and at `loaded` alone would never be cut. The likeliest
    /// is the one whose share fell by this much since the line was last
    /// quiet, as a download's does when its line falls and a call's upload
    /// beside it doesn't, and otherwise the one with the larger share.
    pub fuller: f64,
    /// What a cut leaves of what the direction carried.
    pub cut: f64,
    /// How much a climb adds each step, below the held rate and above it.
    pub climb_back: f64,
    pub climb: f64,
    /// How far a resting rate moves towards the held rate each step.
    pub settle: f64,
    /// Seconds after a cut before another: the pings in flight still carry
    /// the queue the cut was for.
    pub after_cut_s: f64,
    /// A queue this deep, in ms, is drained at half the held rate; a
    /// shallower one at a share of it as much under as it is deep.
    pub drain_ms: f64,
    /// Seconds after a cut before a resting direction forgets it.
    pub forget_s: f64,
    /// Seconds a forgotten-back peak must carry a busy line without delay
    /// before it's the line's again, and a later cut is forgotten too.
    pub proven_s: f64,
    /// Seconds a direction must stay busy before it climbs past its held
    /// rate.
    pub probe_after_s: f64,
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
            fuller: 0.2,
            cut: 0.9,
            // About 10% a second at five steps a second, and about 2.5%:
            // a line that dipped to half is found again in about 30 s.
            climb_back: 1.02,
            climb: 1.005,
            settle: 0.05,
            after_cut_s: 0.6,
            drain_ms: 1000.0,
            // A line's dips, from a neighbour's upload or a busy evening,
            // come and go within minutes; ten of them quiet is long past.
            forget_s: 600.0,
            // Past a cable modem's burst allowance, which can carry a line
            // over its rate for its first several seconds. A peak not yet
            // proven costs little: one the line does carry is climbed to.
            proven_s: 15.0,
            // Longer than a request's burst, and short beside a download
            // or a push that a faster line would help.
            probe_after_s: 5.0,
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
    /// The highest rate it held under load without delay, which an old cut
    /// is forgotten back to.
    pub peak: u64,
    /// Whether the held rate is the peak put back since the last cut, so a
    /// cut that follows shows the peak is no longer the line's.
    forgot: bool,
    /// When the latest run of busy steps without delay began, while
    /// `forgot` holds.
    proving: Option<f64>,
    /// When this direction was last cut: the other's cuts say nothing of
    /// whether this one's is old.
    cut_at: Option<f64>,
    /// When its latest run of busy steps began, since it last rested or
    /// was cut.
    busy_since: Option<f64>,
}

impl Direction {
    pub fn new(bounds: Bounds, held: u64) -> Self {
        let held = held.clamp(bounds.min, bounds.max);
        Direction {
            bounds,
            rate: held,
            held,
            learned: false,
            peak: held,
            forgot: false,
            proving: None,
            cut_at: None,
            busy_since: None,
        }
    }

    fn set(&mut self, rate: f64) {
        self.rate = whole(rate).clamp(self.bounds.min, self.bounds.max);
    }

    /// A busy step on a quiet line: climbs back to the held rate, past it
    /// once it has been busy a while, and proves a peak put back.
    fn climb(&mut self, s: &Settings, now: f64) {
        // A peak put back is proved by carrying the line, whether or not
        // it may climb past it yet.
        if self.forgot {
            let since = *self.proving.get_or_insert(now);
            if now - since >= s.proven_s {
                self.forgot = false;
            }
        }
        let since = *self.busy_since.get_or_insert(now);
        if self.rate >= self.held && now - since < s.probe_after_s {
            return;
        }
        self.set(climbed(s, self));
        if self.rate > self.held {
            self.held = self.rate;
            self.peak = self.peak.max(self.held);
            self.learned = true;
        }
    }

    /// A resting step: forgets a cut long past, and settles towards the
    /// held rate. Gives whether it forgot one.
    fn rest(&mut self, s: &Settings, now: f64) -> bool {
        self.proving = None;
        self.busy_since = None;
        let old = self.cut_at.is_some_and(|at| now - at >= s.forget_s);
        let forgot = old && self.held < self.peak;
        if forgot {
            self.held = self.peak;
            self.forgot = true;
        }
        self.set(float(self.rate) + (float(self.held) - float(self.rate)) * s.settle);
        forgot
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
    /// How far over its baseline each of the latest pings came back, in
    /// ms, beside `recent`; a lost one is unbounded.
    levels: VecDeque<f64>,
    last_cut: Option<f64>,
    /// The queue's delay when the rate was last cut.
    cut_level: f64,
    /// Whether the line has been delayed since a cut, without a whole
    /// quiet window between: a spell ends only when the queue has drained.
    in_spell: bool,
    /// What each direction carried of its rate, up and down, at the last
    /// step whose line wasn't building a queue.
    quiet_shares: [f64; 2],
    /// The direction this spell's latest cut took alone, 0 up and 1 down.
    cut_alone: Option<usize>,
}

/// One step's climb, from a direction's rate and what it held.
fn climbed(s: &Settings, dir: &Direction) -> f64 {
    let rate = float(dir.rate);
    if dir.rate < dir.held {
        (rate * s.climb_back).min(float(dir.held))
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
    /// Resting, and a direction put back its peak in place of an old cut.
    Forgot,
}

impl Controller {
    pub fn new(settings: Settings, up: Direction, down: Direction) -> Self {
        Controller {
            settings,
            up,
            down,
            baselines: HashMap::new(),
            recent: VecDeque::new(),
            levels: VecDeque::new(),
            last_cut: None,
            cut_level: 0.0,
            in_spell: false,
            quiet_shares: [0.0; 2],
            cut_alone: None,
        }
    }

    /// A ping's round trip from one host, or None for one that never came
    /// back, which counts as delayed when the host's last ping answered.
    pub fn sample(&mut self, host: &str, rtt_ms: Option<f64>) {
        let baseline = self.baselines.entry(host.to_string()).or_default();
        let over = match rtt_ms {
            Some(rtt) => baseline.update(rtt),
            None if baseline.lost() => f64::INFINITY,
            None => return,
        };
        self.recent.push_back(over > self.settings.delay_ms);
        self.levels.push_back(over);
        while self.recent.len() > self.settings.window {
            self.recent.pop_front();
            self.levels.pop_front();
        }
        if self.quiet() {
            self.in_spell = false;
        }
    }

    /// How far over their baselines the latest pings came back, by their
    /// median, in ms.
    fn level(&self) -> f64 {
        let mut all: Vec<f64> = self.levels.iter().copied().collect();
        all.sort_by(f64::total_cmp);
        all.get(all.len() / 2).copied().unwrap_or(0.0)
    }

    /// How far over their baselines the pings had come at the latest cut,
    /// in ms.
    pub fn cut_level(&self) -> f64 {
        self.cut_level
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
        let shares = [
            sent / float(self.up.rate.max(1)),
            received / float(self.down.rate.max(1)),
        ];
        if !self.building() {
            self.quiet_shares = shares;
        }
        // A proof is of steps without delay one after another. A delayed
        // step while busy is cut, which ends the forgetting, and one while
        // resting ends the run below; this says so for every other path.
        if !self.quiet() {
            self.up.proving = None;
            self.down.proving = None;
        }
        if self.building() {
            let resting = self.last_cut.is_some_and(|at| now - at < s.after_cut_s);
            let new_spell = !self.in_spell;
            let level = self.level();
            // A queue that drains needs no cut: only one still growing does.
            let rising = new_spell || level > self.cut_level + s.delay_ms / 3.0;
            // A rate a tenth under the line drains its queue at a tenth of
            // the line: a second of modem buffer would take ten seconds. So
            // a cut goes under the held rate by a share as large as the
            // queue's delay is of a second, up to half, which drains it in
            // about a second, and stays there until the line is quiet. A
            // rate let back up as the queue fell stood at the line with the
            // queue half drained.
            let drain = 1.0 - (level / s.drain_ms).min(0.5);
            let mut cuts = [false; 2];
            let fell = [
                self.quiet_shares[0] - shares[0],
                self.quiet_shares[1] - shares[1],
            ];
            // A queue still growing after one direction alone was cut is
            // the other's, when it carries enough to fill it.
            let likeliest = if let Some(cut) = self.cut_alone.filter(|_| !new_spell) {
                1 - cut
            } else if fell[0].max(fell[1]) >= s.fuller {
                usize::from(fell[1] > fell[0])
            } else {
                usize::from(shares[1] > shares[0])
            };
            for (i, ((dir, carried), share)) in [(&mut self.up, sent), (&mut self.down, received)]
                .into_iter()
                .zip(shares)
                .enumerate()
            {
                let filling = share >= s.loaded || (share >= s.fuller && i == likeliest);
                if filling && !resting && rising {
                    let from = carried.min(float(dir.rate));
                    // A queue still growing after a cut says that rate too
                    // was over the line, so each cut is the line's best
                    // guess.
                    dir.held = whole(from * s.cut).clamp(dir.bounds.min, dir.bounds.max);
                    dir.set(float(dir.held) * drain);
                    dir.learned = true;
                    if std::mem::replace(&mut dir.forgot, false) {
                        dir.peak = dir.held;
                    }
                    dir.proving = None;
                    dir.cut_at = Some(now);
                    dir.busy_since = None;
                    cuts[i] = true;
                }
            }
            if cuts[0] || cuts[1] {
                self.cut_alone = match cuts {
                    [true, false] => Some(0),
                    [false, true] => Some(1),
                    _ => None,
                };
                self.last_cut = Some(now);
                self.cut_level = level;
                self.in_spell = true;
                // The pings that showed the queue are spent; the next cut
                // waits for pings sent at the new rate.
                self.recent.clear();
                self.levels.clear();
                return State::Cut;
            }
            return State::Holding;
        }
        if !self.quiet() {
            return State::Holding;
        }
        let mut climbing = false;
        let mut forgot = false;
        for (dir, carried) in [(&mut self.up, sent), (&mut self.down, received)] {
            if carried >= float(dir.rate) * s.busy {
                dir.climb(&s, now);
                climbing = true;
            } else {
                forgot |= dir.rest(&s, now);
            }
        }
        if climbing {
            State::Climbing
        } else if forgot {
            State::Forgot
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
        // The line held a tenth under what it carried; the queue of about
        // 180 ms drains at about that much under that.
        assert_eq!(c.up.held, 9_900);
        assert!((8_000..8_300).contains(&c.up.rate), "{}", c.up.rate);
        // The download carried little, so it wasn't filling the queue.
        assert_eq!(c.down.rate, 100_000);
    }

    #[test]
    fn a_cut_waits_for_pings_sent_after_it() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        let draining = c.up.rate;
        assert_eq!(c.step(1.2, 0.0, 0.0), State::Holding);
        pings(&mut c, 300.0, 3);
        assert_eq!(c.step(1.4, 9_900.0, 0.0), State::Holding);
        assert_eq!(c.up.rate, draining);
        assert_eq!(c.step(1.7, 9_900.0, 0.0), State::Cut);
        assert!(c.up.rate < draining);
    }

    #[test]
    fn a_queue_still_growing_after_a_cut_lowers_the_held_rate_too() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        let draining = c.up.rate;
        pings(&mut c, 300.0, 3);
        c.step(1.7, 9_900.0, 0.0);
        // Still growing at the draining rate: the line is under even that.
        assert_eq!(c.up.held, whole(float(draining) * 0.9));
        assert!(c.up.rate < c.up.held);
    }

    #[test]
    fn a_queue_draining_after_a_cut_is_not_cut_again() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        let (draining, held) = (c.up.rate, c.up.held);
        // Still late, but less so each time: the cut already drains it.
        for (at, rtt) in [(1.7, 190.0), (2.4, 150.0), (3.1, 90.0)] {
            pings(&mut c, rtt, 3);
            assert_eq!(c.step(at, 9_900.0, 0.0), State::Holding, "at {at}");
        }
        assert_eq!((c.up.rate, c.up.held), (draining, held));
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
        assert_eq!(c.up.held, 7_200);
        assert!(c.up.rate < 7_200);
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
    fn a_busy_line_without_delay_climbs_once_it_has_been_busy_a_while() {
        let mut c = controller(10_000, 100_000);
        pings(&mut c, 20.0, 6);
        assert_eq!(c.step(1.0, 9_500.0, 1_000.0), State::Climbing);
        assert_eq!(c.up.rate, 10_000);
        assert!(!c.up.learned);
        c.step(6.0, 9_500.0, 1_000.0);
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
        // Back in about seven seconds, then probing past it only once busy
        // for five.
        let mut steps = 0;
        while c.up.rate < c.up.held && steps < 100 {
            c.step(1.0, float(c.up.rate), 0.0);
            steps += 1;
        }
        assert_eq!(c.up.rate, 10_000);
        assert!(steps <= 36, "{steps}");
        c.step(1.0, 10_000.0, 0.0);
        assert_eq!(c.up.rate, 10_000);
        c.step(6.0, 10_000.0, 0.0);
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
        assert!(c.up.rate.abs_diff(10_000) < 100, "{}", c.up.rate);
    }

    #[test]
    fn rates_stay_within_their_bounds() {
        let mut c = controller(1_000, 5_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 1_000.0, 5_000.0);
        assert_eq!((c.up.rate, c.down.rate), (1_000, 5_000));
    }

    /// A line beyond the router as the timed VM test makes it: `line`
    /// kbit/s, 20 ms away, with a buffer of a second at 8 Mbit/s, and
    /// uploads that always fill what the shaper lets out. Steps it for
    /// `seconds`, three pings a step, and gives the queue's delay in ms.
    struct Line {
        queue_kbit: f64,
        at: f64,
    }

    impl Line {
        fn run(&mut self, c: &mut Controller, line: f64, seconds: f64) -> f64 {
            let dt = 0.2;
            let end = self.at + seconds;
            while self.at < end {
                let sent = float(c.up.rate);
                self.queue_kbit = (self.queue_kbit + (sent - line) * dt).clamp(0.0, 8_000.0);
                let delay = self.queue_kbit / line * 1000.0;
                pings(c, 20.0 + delay, 3);
                self.at += dt;
                c.step(self.at, sent, 0.0);
            }
            self.queue_kbit / line * 1000.0
        }

        /// Uploads `on` seconds of every `on + off`, as requests do, for
        /// `seconds`, and gives how many times the shaper cut.
        fn bursts(
            &mut self,
            c: &mut Controller,
            line: f64,
            seconds: f64,
            on: f64,
            off: f64,
        ) -> usize {
            let dt = 0.2;
            let start = self.at;
            let mut cuts = 0;
            while self.at < start + seconds {
                let sending = (self.at - start) % (on + off) < on;
                let sent = if sending { float(c.up.rate) } else { 0.0 };
                self.queue_kbit = (self.queue_kbit + (sent - line) * dt).clamp(0.0, 8_000.0);
                let delay = self.queue_kbit / line * 1000.0;
                pings(c, 20.0 + delay, 3);
                self.at += dt;
                if c.step(self.at, sent.min(line), 0.0) == State::Cut {
                    cuts += 1;
                }
            }
            cuts
        }
    }

    #[test]
    fn requests_sent_in_bursts_fit_under_the_line_without_probing_past_it() {
        let mut c = controller(12_000, 50_000);
        pings(&mut c, 20.0, 6);
        let mut line = Line {
            queue_kbit: 0.0,
            at: 0.0,
        };
        // A long upload finds the line.
        line.run(&mut c, 10_000.0, 30.0);
        // Then two seconds of every five: each fits under what was held,
        // and none climbs past the line to fill its queue.
        let cuts = line.bursts(&mut c, 10_000.0, 120.0, 2.0, 3.0);
        assert!(cuts <= 1, "{cuts}");
    }

    #[test]
    fn a_full_modem_buffer_is_drained_without_cutting_far_under_the_line() {
        let mut c = controller(10_000, 50_000);
        pings(&mut c, 20.0, 6);
        let mut line = Line {
            queue_kbit: 0.0,
            at: 0.0,
        };
        // It probes a little past the line and cuts at the queue that
        // builds, so it sits within a tenth of the line either way.
        line.run(&mut c, 8_000.0, 20.0);
        assert!((5_000..8_800).contains(&c.up.rate), "{}", c.up.rate);
        let delay = line.run(&mut c, 8_000.0, 10.0);
        assert!(delay < 60.0, "{delay}");
        assert!((5_000..8_800).contains(&c.up.rate), "{}", c.up.rate);
        // The line halves: followed within seconds.
        line.run(&mut c, 4_000.0, 30.0);
        assert!(c.up.rate < 4_400, "{}", c.up.rate);
        let delay = line.run(&mut c, 4_000.0, 10.0);
        assert!(delay < 100.0, "{delay}");
        // And back: found again within two minutes.
        line.run(&mut c, 8_000.0, 120.0);
        assert!(c.up.rate > 6_000, "{}", c.up.rate);
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

    /// A download over a line beyond the router: `line` kbit/s, with an
    /// ISP's buffer, and senders that fill what the shaper lets through and
    /// a little more, as TCP probing does. What arrives is what the line
    /// carries while its queue holds anything. Steps it for `seconds`,
    /// three pings a step, and gives the seconds the queue was over 30 ms.
    struct Download {
        queue_kbit: f64,
        at: f64,
    }

    impl Download {
        fn run(&mut self, c: &mut Controller, line: f64, seconds: f64, acks: f64) -> f64 {
            let dt = 0.2;
            let end = self.at + seconds;
            let mut late = 0.0;
            while self.at < end {
                let offered = float(c.down.rate) * 1.05;
                self.queue_kbit = (self.queue_kbit + (offered - line) * dt).clamp(0.0, 100_000.0);
                let received = if self.queue_kbit > 0.0 { line } else { offered };
                let delay = self.queue_kbit / line * 1000.0;
                if delay > 30.0 {
                    late += dt;
                }
                pings(c, 20.0 + delay, 3);
                self.at += dt;
                c.step(self.at, acks.min(float(c.up.rate)), received);
            }
            late
        }

        /// Nothing sent or received for `seconds`, the line quiet.
        fn rest(&mut self, c: &mut Controller, seconds: f64) {
            let end = self.at + seconds;
            self.queue_kbit = 0.0;
            while self.at < end {
                pings(c, 20.0, 3);
                self.at += 0.2;
                c.step(self.at, 0.0, 0.0);
            }
        }
    }

    /// A download whose line dipped to 50 of its 95 Mbit/s, and was cut.
    fn cut_by_a_dip() -> (Controller, Download) {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let mut line = Download {
            queue_kbit: 0.0,
            at: 0.0,
        };
        line.run(&mut c, 100_000.0, 2.0, 1_000.0);
        line.run(&mut c, 50_000.0, 10.0, 1_000.0);
        assert!(c.down.held < 50_000, "{}", c.down.held);
        (c, line)
    }

    #[test]
    fn a_cut_long_past_is_forgotten_by_the_next_download() {
        let (mut c, mut line) = cut_by_a_dip();
        // Said once, on the step it's forgotten.
        let mut told = 0;
        for _ in 0..3_500 {
            pings(&mut c, 20.0, 3);
            line.at += 0.2;
            if c.step(line.at, 0.0, 0.0) == State::Forgot {
                told += 1;
            }
        }
        assert_eq!(told, 1);
        // The line is back: the download starts near what it carried
        // before the dip, not half way up a half-minute climb.
        line.run(&mut c, 100_000.0, 2.0, 1_000.0);
        assert!(c.down.rate > 85_000, "{}", c.down.rate);
    }

    #[test]
    fn a_peak_the_line_carried_again_outlasts_the_next_dip() {
        let (mut c, mut line) = cut_by_a_dip();
        line.rest(&mut c, 700.0);
        // The peak put back carries a download for a while: it's the
        // line's again, and a later dip's cut is forgotten back to it. The
        // line is faster than the climb reaches, so no probe past it is
        // cut on the way.
        line.run(&mut c, 200_000.0, 20.0, 1_000.0);
        line.rest(&mut c, 60.0);
        line.run(&mut c, 50_000.0, 10.0, 1_000.0);
        line.rest(&mut c, 700.0);
        assert!(c.down.held > 90_000, "{}", c.down.held);
    }

    #[test]
    fn an_upload_cut_now_and_then_leaves_an_old_download_cut_to_be_forgotten() {
        let (mut c, mut line) = cut_by_a_dip();
        for _ in 0..4 {
            line.rest(&mut c, 300.0);
            let mut upload = Line {
                queue_kbit: 0.0,
                at: line.at,
            };
            upload.run(&mut c, 5_000.0, 5.0);
            line.at = upload.at;
        }
        // The upload was cut by its last run, from the peak put back after
        // 600 s, rather than left at its own: an upload that fits under
        // its line isn't cut at all.
        assert!(c.up.peak < 10_000, "{}", c.up.peak);
        assert!(c.down.held > 90_000, "{}", c.down.held);
    }

    #[test]
    fn a_recent_cut_is_still_held() {
        let (mut c, mut line) = cut_by_a_dip();
        line.rest(&mut c, 60.0);
        assert_ne!(c.step(line.at + 0.2, 0.0, 0.0), State::Forgot);
        assert!(c.down.held < 50_000, "{}", c.down.held);
    }

    #[test]
    fn a_line_that_stayed_slow_is_cut_once_and_kept() {
        let (mut c, mut line) = cut_by_a_dip();
        line.rest(&mut c, 700.0);
        // Forgotten, but the line is still slow: the first download's
        // queue is drained within a couple of seconds, and its cut is
        // kept rather than forgotten again.
        let late = line.run(&mut c, 50_000.0, 10.0, 1_000.0);
        assert!(late < 2.0, "{late}");
        line.rest(&mut c, 700.0);
        assert!(c.down.held < 50_000, "{}", c.down.held);
    }

    #[test]
    fn a_line_fallen_under_half_the_download_is_followed_down() {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let mut line = Download {
            queue_kbit: 0.0,
            at: 0.0,
        };
        line.run(&mut c, 100_000.0, 2.0, 1_000.0);
        // The line falls to 40: what arrives is then 42% of the shaped
        // rate, under `loaded`, but the download is still what fills it.
        let late = line.run(&mut c, 40_000.0, 10.0, 1_000.0);
        assert!(c.down.rate < 40_000, "{}", c.down.rate);
        assert!(late < 2.0, "{late}");
        let late = line.run(&mut c, 40_000.0, 10.0, 1_000.0);
        assert!(late < 0.5, "{late}");
    }

    #[test]
    fn a_call_s_upload_isnt_cut_for_a_fallen_download_line() {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let mut line = Download {
            queue_kbit: 0.0,
            at: 0.0,
        };
        line.run(&mut c, 100_000.0, 2.0, 3_000.0);
        // A call sends 3 of the upload's 10 (0.30) while the download's
        // line falls to 25 of its 95 (0.26): the larger share is the
        // upload's, but it is the download's that fell.
        let late = line.run(&mut c, 25_000.0, 20.0, 3_000.0);
        assert_eq!(c.up.rate, 10_000);
        assert!(c.down.rate < 25_000, "{}", c.down.rate);
        assert!(late < 3.0, "{late}");
    }

    #[test]
    fn a_line_falling_twice_is_followed_while_the_upload_carries_acks() {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let mut line = Download {
            queue_kbit: 0.0,
            at: 0.0,
        };
        line.run(&mut c, 100_000.0, 2.0, 1_000.0);
        // The line falls, then falls again within the spell: handing the
        // next cut to the upload's acknowledgements mustn't stall it.
        line.run(&mut c, 25_000.0, 3.0, 1_000.0);
        let late = line.run(&mut c, 12_000.0, 20.0, 1_000.0);
        assert!(c.down.rate < 12_000, "{}", c.down.rate);
        assert!(late < 4.0, "{late}");
    }

    #[test]
    fn a_queue_a_cut_did_not_stop_is_put_to_the_other_direction() {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let (mut queue_kbit, mut at, mut late) = (0.0_f64, 0.0, 0.0);
        // A download fills the line, then ends as an app's upload of 4 of
        // 10 meets an uplink fallen to 3: the download's share fell the
        // more, so it is cut first, but the queue is the upload's.
        while at < 30.0 {
            let (line, sending, received) = if at < 2.0 {
                (10_000.0, 1_000.0_f64, float(c.down.rate))
            } else {
                (3_000.0, 4_000.0, 30_000.0_f64.min(float(c.down.rate)))
            };
            let sent = sending.min(float(c.up.rate));
            queue_kbit = (queue_kbit + (sent - line) * 0.2).clamp(0.0, 8_000.0);
            let delay = queue_kbit / line * 1000.0;
            if at > 20.0 && delay > 30.0 {
                late += 0.2;
            }
            pings(&mut c, 20.0 + delay, 3);
            at += 0.2;
            c.step(at, sent, received);
        }
        // It probes a little past the line once the queue is gone.
        assert!(c.up.rate < 3_300, "{}", c.up.rate);
        assert!(late < 1.0, "{late}");
    }

    #[test]
    fn a_download_s_acks_are_not_cut_for_its_queue() {
        let mut c = controller(10_000, 95_000);
        pings(&mut c, 20.0, 6);
        let mut line = Download {
            queue_kbit: 0.0,
            at: 0.0,
        };
        // A quarter of the upload is the download's acknowledgements.
        line.run(&mut c, 40_000.0, 10.0, 2_500.0);
        assert_eq!(c.up.rate, 10_000);
        assert!(c.down.rate < 40_000, "{}", c.down.rate);
    }
}
