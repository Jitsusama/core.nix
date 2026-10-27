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
//! Only this machine's own traffic is cut for a queue. A download that
//! arrives faster than its rate is held back here, so the line carries
//! more than that and its queue is the upload's or another machine's; on
//! Wi-Fi, cut for each such spell, a download fell from 85 Mbit/s to its
//! floor of 5 and stayed there. Pings come back late and early by turns
//! on Wi-Fi, so a queue back within moments of a cut is still the same
//! spell, cut again only if it grows. An upload has no arrivals to tell
//! by, so each cut is on trial: once the line is quiet the rate climbs
//! straight back to where it was cut from, and the cut stands only if the
//! queue comes back on the way. A cut near where one was confirmed isn't
//! tried again.

use std::collections::{HashMap, VecDeque};

/// A count, a rate in kbit/s or a time in microseconds, as a float. Exact:
/// every one here is far below 2⁵³.
#[expect(clippy::cast_precision_loss, reason = "every one is far below 2⁵³")]
pub fn float(n: u64) -> f64 {
    n as f64
}

/// A rate worked out as a float, to the nearest whole kbit/s. A negative
/// rate is none, and every caller holds the result within its bounds.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded, held at zero or more, and within its bounds by every caller"
)]
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
    /// A download arriving at more than this share of its rate is held
    /// back here, not on the line.
    pub held_here: f64,
    /// Seconds after a cut within which a queue back again is the same
    /// spell's, not a new one's.
    pub spell_gap_s: f64,
    /// Seconds a busy direction held at its floor may meet a queue that
    /// never drains before its best comes down a cut's share.
    pub floored_s: f64,
    /// The share of the best rate a direction has held that no cut goes
    /// under; none lets a cut follow the line down to its floor.
    pub least_of_best: f64,
    /// The same share while real-time traffic flows, when the line's
    /// queue has to be this machine's, where CAKE sends a call first.
    pub least_of_best_live: f64,
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
            // Past the twentieth over its rate that senders send while
            // they probe; a download arriving between its rate and that
            // may still be what fills the line.
            held_here: 1.05,
            // Over a few pings' worth: on Wi-Fi a queue's pings come back
            // late and early by turns, and each turn read as a new spell
            // cut the upload from 1.8 Mbit/s to 1.0 within three seconds.
            spell_gap_s: 2.0,
            // The air's late pings came a second of every three, so a
            // queue standing for six seconds running is the line's own.
            floored_s: 6.0,
            // A cable's line does fall, to half on a busy evening, and a
            // cut that follows it keeps its queue short.
            least_of_best: 0.0,
            least_of_best_live: 0.0,
        }
    }
}

impl Settings {
    /// What a link of this kind is shaped by. On Wi-Fi a ping comes back
    /// late for the air's sake, a neighbour's or the router's as often as
    /// for a queue of this machine's: three times on one day, cuts for such
    /// pings took its upload of about 10 Mbit/s to 1 or 2 and its download
    /// from 50 to 6 or 17. Joel wants the line's whole rate kept, and a
    /// shaper over the line only leaves it as unshaped, so a cut there
    /// never goes more than a twentieth under the best the line has
    /// carried, as a cable's ceiling sits. At a fifth, live under the
    /// agents' load, 104 of 115 cuts in 13 minutes ended at that floor,
    /// holding an upload of 10.5 Mbit/s at 8.
    ///
    /// While a call or a screen share runs, though, Joel wants it to work
    /// well above all, and a floor over the line leaves its packets behind
    /// the line's own queue: at 95% of a best learned in an upload's first
    /// seconds, a bulk upload took pings past the router from 27 ms to 44
    /// on average, and up to 134. Then a cut may go to three fifths of the
    /// best. Live, over four pairs beside a stand-in call, an upload ran
    /// 28% slower there, and the pings' slowest twentieth stayed under 54 ms
    /// where it reached 100 without the call. At four fifths the upload ran
    /// only 10% slower, but in a noisier hour that twentieth reached 145 ms
    /// beside the call. Joel chose the calls.
    pub fn for_link(wireless: bool) -> Self {
        Settings {
            least_of_best: if wireless { 0.95 } else { 0.0 },
            least_of_best_live: if wireless { 0.6 } else { 0.0 },
            ..Settings::default()
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
    /// The highest peak it has had, which a cut that lowers the peak
    /// leaves alone: the rate no cut goes under is a share of it.
    pub best: u64,
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
    /// The rate held before a cut still on trial, which it climbs straight
    /// back to once the line is quiet. Late pings say a queue, not whose:
    /// on Wi-Fi or beside a busy host they come late for no queue of ours,
    /// and each cut a tenth, with climbing past the held rate waiting five
    /// busy seconds, took an upload to its floor within two minutes. The
    /// cut stands only if the queue comes back on the way.
    on_trial: Option<u64>,
    /// The rate at which the line's queue last came back on a trial's way
    /// up: a cut from near it isn't tried again, since the line is there.
    confirmed: Option<u64>,
    /// Since when it has been busy at its floor with the line's queue
    /// standing, without a quiet window between.
    floored_since: Option<f64>,
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
            best: held,
            forgot: false,
            proving: None,
            cut_at: None,
            busy_since: None,
            on_trial: None,
            confirmed: None,
            floored_since: None,
        }
    }

    fn set(&mut self, rate: f64) {
        self.rate = whole(rate).clamp(self.bounds.min, self.bounds.max);
    }

    /// Holds this direction under a new most the link carries, as when a
    /// cable moves to a port of another speed, saying whether it moved. A
    /// lower one brings down whatever was held above it; a higher one only
    /// lets the rate climb there, as it learns the line.
    pub fn bound(&mut self, max: u64) -> bool {
        let max = max.max(self.bounds.min);
        if max == self.bounds.max {
            return false;
        }
        self.bounds.max = max;
        for kept in [
            &mut self.rate,
            &mut self.held,
            &mut self.peak,
            &mut self.best,
        ] {
            *kept = (*kept).min(max);
        }
        true
    }

    /// What the network's memory keeps of this direction: the best it held
    /// under load without delay, which a cut long past is forgotten back
    /// to, rather than the latest cut. A restart from the cut had no better
    /// rate to forget back to, and held a download on Wi-Fi at 5 Mbit/s
    /// where the line carried 85. Should the line be slower now, the first
    /// burst's queue cuts it within a second or two.
    ///
    /// A line no cut goes far under keeps its best instead. There a cut
    /// after the peak was put back lowers the peak to the floor, a share
    /// of the best; a restart made that the best, and so each restart
    /// took the floor down by that share again. Live, one restart moved a
    /// Wi-Fi download's floor from 40 Mbit/s to 38.
    pub fn remembered(&self, s: &Settings) -> u64 {
        if s.least_of_best > 0.0 {
            self.best
        } else {
            self.peak
        }
    }

    /// A busy step on a quiet line: climbs back to the held rate, past it
    /// once it has been busy a while, and proves a peak put back.
    fn climb(&mut self, s: &Settings, now: f64) {
        self.floored_since = None;
        // A peak put back is proved by carrying the line, whether or not
        // it may climb past it yet.
        if self.forgot {
            let since = *self.proving.get_or_insert(now);
            if now - since >= s.proven_s {
                self.forgot = false;
            }
        }
        let since = *self.busy_since.get_or_insert(now);
        if self.rate >= self.held && self.on_trial.is_none() && now - since < s.probe_after_s {
            return;
        }
        self.set(climbed(s, self));
        if self.on_trial.is_some_and(|before| self.rate >= before) {
            self.on_trial = None;
        }
        if self.rate > self.held {
            self.held = self.rate;
            // A trial's way up isn't yet the line's best.
            if self.on_trial.is_none() {
                self.peak = self.peak.max(self.held);
                self.best = self.best.max(self.peak);
            }
            self.learned = true;
        }
    }

    /// Cuts to `held`, putting the cut on trial unless the line has shown
    /// it's there: its queue came back on this trial's way up, a trial found
    /// it near the rate cut from, or the peak was just put back.
    fn cut_to(&mut self, held: u64) {
        let before = self.held;
        let on_the_way = self.on_trial.is_some() && self.rate >= self.held;
        let near = |rate: u64| (float(rate) * 0.9..=float(rate) * 1.1).contains(&float(before));
        let shown = on_the_way || self.forgot || self.confirmed.is_some_and(near);
        if on_the_way {
            self.confirmed = Some(self.rate);
        } else if shown && !self.forgot {
            self.confirmed = Some(before);
        }
        self.on_trial = if shown {
            None
        } else {
            Some(self.on_trial.unwrap_or(before))
        };
        self.held = held;
    }

    /// A resting step: forgets a cut long past, and settles towards the
    /// held rate. Gives whether it forgot one.
    fn rest(&mut self, s: &Settings, now: f64) -> bool {
        self.floored_since = None;
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

    /// A step of the line's queue standing: lowers the best by a cut's
    /// share, and the held rate with it, when this direction has been busy
    /// at its floor, the queue never draining, for `floored_s`. A queue
    /// that stands full no longer grows, so no cut would come of it. The
    /// floor keeps cuts off a line whose late pings are the air's, which come and go; a queue that
    /// stands for as long as this direction fills its rate is its own, and
    /// live the floor held an upload at 11.7 Mbit/s over a line of 9.3,
    /// its queue at 60 to 70 ms. A climb puts the best back once the line
    /// carries more.
    ///
    /// Only a direction that may be filling the line counts: a download
    /// arriving faster than its rate is held back here, and lowering its
    /// best for a queue elsewhere took one from 50 Mbit/s to 8 in two
    /// minutes.
    fn pinned(&mut self, s: &Settings, now: f64, carried: f64, suspect: bool) {
        let least = whole(float(self.best) * s.least_of_best).max(self.bounds.min);
        if !suspect || carried < float(self.rate) * s.busy || self.held > least {
            self.floored_since = None;
            return;
        }
        let since = *self.floored_since.get_or_insert(now);
        if now - since >= s.floored_s {
            self.best = whole(float(self.best) * s.cut).max(self.bounds.min);
            self.peak = self.peak.min(self.best);
            let floor = whole(float(self.best) * s.least_of_best).max(self.bounds.min);
            self.held = whole(float(self.held) * s.cut).max(floor);
            self.set(float(self.held));
            self.learned = true;
            self.floored_since = Some(now);
        }
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
    /// Whether real-time traffic, a call's or a screen share's, crosses the
    /// line now, as the shaper's video tin says.
    pub live: bool,
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
    /// Whether this spell has cut each direction, up and down.
    cut_in_spell: [bool; 2],
    /// When a download last arrived faster than its rate while the line
    /// built a queue.
    held_at: Option<f64>,
}

/// One step's climb, from a direction's rate and what it held.
fn climbed(s: &Settings, dir: &Direction) -> f64 {
    let rate = float(dir.rate);
    if dir.rate < dir.held {
        (rate * s.climb_back).min(float(dir.held))
    } else if let Some(before) = dir.on_trial.filter(|before| dir.rate < *before) {
        (rate * s.climb_back).min(float(before))
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
            live: false,
            baselines: HashMap::new(),
            recent: VecDeque::new(),
            levels: VecDeque::new(),
            last_cut: None,
            cut_level: 0.0,
            in_spell: false,
            quiet_shares: [0.0; 2],
            cut_alone: None,
            cut_in_spell: [false; 2],
            held_at: None,
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
        let mut s = self.settings.clone();
        if self.live {
            s.least_of_best = s.least_of_best.min(s.least_of_best_live);
        }
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
            let new_spell =
                !self.in_spell && self.last_cut.is_none_or(|at| now - at >= s.spell_gap_s);
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
            if new_spell {
                self.cut_in_spell = [false; 2];
            }
            let filling = self.suspects(now, [sent, received], shares, new_spell);
            for (i, (dir, carried)) in [(&mut self.up, sent), (&mut self.down, received)]
                .into_iter()
                .enumerate()
            {
                if filling[i] && !resting && rising {
                    let from = carried.min(float(dir.rate));
                    // A queue still growing after a cut says that rate too
                    // was over the line, so each cut is the line's best
                    // guess.
                    let least = whole(float(dir.best) * s.least_of_best).max(dir.bounds.min);
                    dir.cut_to(whole(from * s.cut).clamp(least, dir.bounds.max.max(least)));
                    dir.set((float(dir.held) * drain).max(float(least)));
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
            for (i, (dir, carried)) in [(&mut self.up, sent), (&mut self.down, received)]
                .into_iter()
                .enumerate()
            {
                dir.pinned(&s, now, carried, filling[i]);
            }
            if cuts[0] || cuts[1] {
                self.cut_in_spell[0] |= cuts[0];
                self.cut_in_spell[1] |= cuts[1];
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

    /// Which directions, up and down, may be filling the line's building
    /// queue, given what each `carried` and its share of its rate.
    fn suspects(
        &mut self,
        now: f64,
        carried: [f64; 2],
        shares: [f64; 2],
        new_spell: bool,
    ) -> [bool; 2] {
        let s = &self.settings;
        let fell = [
            self.quiet_shares[0] - shares[0],
            self.quiet_shares[1] - shares[1],
        ];
        // A download arriving faster than its rate is held back by the
        // shaping here, so the line carries more than that rate and
        // the download can't be what queues on it: the queue is the
        // upload's, or another machine's. Cut for every spell while a
        // video took its rate, a download on Wi-Fi fell from 85 Mbit/s
        // to the floor of 5 within ten minutes and never climbed back.
        // Once this spell has cut it, though, the line's queue drains
        // at the line's speed, over the new rate, so it says nothing.
        let held_here = [false, shares[1] > s.held_here && !self.cut_in_spell[1]];
        // That leaves the upload, or another machine, but not the upload by
        // elimination: it's cut only when it carries enough to fill the
        // queue on its own. A video's bursts arrive faster than the
        // download's rate and queue on the line as they do, and put to the
        // upload, the agents' requests beside it took Wi-Fi's upload of
        // about 10 Mbit/s to its floor within three minutes. The pings stay
        // late a moment after a burst ends, while its queue drains, so a
        // spell gap after it is still the burst's.
        if held_here[1] {
            self.held_at = Some(now);
        }
        let held_lately = self.held_at.is_some_and(|at| now - at < s.spell_gap_s);
        // A cut leaves a direction under what it's asked to carry, so
        // weighed against its rate it reads as full at every burst after,
        // and requests asking 3 Mbit/s beside a video were held at 2.3.
        // Beside a burst, each is weighed against the best the line has
        // carried for it instead.
        let weighed = if held_lately {
            [
                carried[0] / float(self.up.peak.max(self.up.rate).max(1)),
                carried[1] / float(self.down.peak.max(self.down.rate).max(1)),
            ]
        } else {
            shares
        };
        // A queue still growing after one direction alone was cut is the
        // other's, when it carries enough to fill it.
        let likeliest = if held_lately {
            None
        } else if let Some(cut) = self.cut_alone.filter(|_| !new_spell) {
            Some(1 - cut)
        } else if fell[0].max(fell[1]) >= s.fuller {
            Some(usize::from(fell[1] > fell[0]))
        } else {
            Some(usize::from(shares[1] > shares[0]))
        };
        [0, 1].map(|i| {
            !held_here[i]
                && (weighed[i] >= s.loaded || (weighed[i] >= s.fuller && likeliest == Some(i)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(up: u64, down: u64) -> Controller {
        Controller::new(
            Settings::for_link(false),
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

    #[test]
    fn a_link_slowed_brings_down_what_was_held_over_its_new_most() {
        let mut c = controller(20_000, 900_000);
        assert!(c.down.bound(93_000));
        assert_eq!(
            (c.down.rate, c.down.held, c.down.peak, c.down.best),
            (93_000, 93_000, 93_000, 93_000)
        );
        assert!(!c.down.bound(93_000), "the same most moves nothing");
        assert!(c.down.bound(930_000));
        assert_eq!(c.down.bounds.max, 930_000);
        assert_eq!(c.down.rate, 93_000, "a faster link is learned, not assumed");
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
    fn a_download_arriving_faster_than_its_rate_isnt_cut_for_the_line_s_queue() {
        let mut c = controller(10_000, 20_000);
        pings(&mut c, 20.0, 6);
        // A video takes all the download's rate and more arrives: the
        // shaping here holds it back, so the line carries more than the
        // download's rate and the download can't be what queues on it.
        // The queue is the upload's, or another machine's.
        pings(&mut c, 60.0, 3);
        assert_eq!(c.step(1.0, 9_500.0, 22_000.0), State::Cut);
        assert!(c.up.rate < 10_000, "{}", c.up.rate);
        assert_eq!((c.down.rate, c.down.held), (20_000, 20_000));
    }

    #[test]
    fn a_queue_nobody_here_fills_leaves_a_download_held_back_here_alone() {
        let mut c = controller(10_000, 85_000);
        pings(&mut c, 20.0, 6);
        // Another machine keeps the line's queue at 20 to 40 ms over its
        // baseline, with quiet moments between, while a video here takes
        // all of the download's rate and more arrives. Cut at every spell,
        // the download fell from 85 to the floor of 5 within ten minutes
        // and stayed there.
        let mut at = 0.0;
        for spell in 0..200 {
            let over = if spell % 2 == 0 { 40.0 } else { 20.0 };
            for _ in 0..5 {
                pings(&mut c, 20.0 + over, 3);
                at += 0.2;
                c.step(at, 500.0, float(c.down.rate) * 1.1);
            }
            for _ in 0..5 {
                pings(&mut c, 20.0, 3);
                at += 0.2;
                c.step(at, 500.0, float(c.down.rate) * 1.1);
            }
        }
        assert!(c.down.rate >= 85_000, "{}", c.down.rate);
    }

    #[test]
    fn an_upload_carrying_little_isnt_cut_for_a_download_held_back_here() {
        let mut c = controller(10_000, 50_000);
        pings(&mut c, 20.0, 6);
        // A 4K video fetches a second of every four at more than the
        // download's rate, and the line's queue builds 40 ms while it does,
        // as the agents' requests keep a third of the upload busy. Put to
        // the upload as the only other likeliest, every burst cut it, and
        // Wi-Fi's upload of about 10 Mbit/s was at its floor of 1 within
        // three minutes.
        let (mut at, mut lowest) = (0.0, u64::MAX);
        while at < 180.0 {
            let fetching = at % 4.0 < 1.0;
            let late = if fetching { 40.0 } else { 0.0 };
            let received = if fetching { 1.3 } else { 0.05 } * float(c.down.rate);
            pings(&mut c, 20.0 + late, 3);
            at += 0.2;
            c.step(at, 0.3 * float(c.up.rate), received);
            lowest = lowest.min(c.up.rate);
        }
        assert!(lowest >= 9_000, "{lowest}");
        assert!(c.down.rate >= 50_000, "{}", c.down.rate);
    }

    #[test]
    fn requests_beside_a_video_get_what_they_ask_for() {
        let mut c = controller(10_000, 50_000);
        pings(&mut c, 20.0, 6);
        // The same video, beside requests asking 3 Mbit/s and 8 for a
        // second of every seven. When a request's burst meets a video's,
        // the upload carries enough to be cut; each video burst after that
        // confirmed the cut, and held the requests at 2.3 Mbit/s.
        let (mut at, mut lowest) = (0.0, u64::MAX);
        while at < 180.0 {
            let fetching = at % 4.0 < 1.0;
            let late = if fetching { 40.0 } else { 0.0 };
            let received = if fetching { 1.3 } else { 0.05 } * float(c.down.rate);
            let asked: f64 = if at % 7.0 < 1.0 { 8_000.0 } else { 3_000.0 };
            pings(&mut c, 20.0 + late, 3);
            at += 0.2;
            c.step(at, asked.min(float(c.up.rate)), received);
            if at > 60.0 {
                lowest = lowest.min(c.up.rate);
            }
        }
        assert!(lowest >= 3_000, "{lowest}");
    }

    #[test]
    fn a_wi_fi_line_is_never_cut_far_under_its_best() {
        let bounds = |min| Bounds {
            min,
            max: 1_000_000,
        };
        let mut c = Controller::new(
            Settings::for_link(true),
            Direction::new(bounds(1_000), 10_000),
            Direction::new(bounds(5_000), 50_000),
        );
        pings(&mut c, 20.0, 6);
        // As the agents came back after a reboot, the line's queue grew
        // from 29 ms to 92 while each cut took the upload and then the
        // download, from 10 Mbit/s to 2.4 and from 50 to 16 within a
        // second: the queue was neither's. Here pings grow late for a
        // second of every three whatever either direction carries.
        let (mut at, mut lowest) = (0.0, [u64::MAX; 2]);
        while at < 180.0 {
            let late = if at % 3.0 < 1.0 {
                30.0 + 60.0 * (at % 1.0)
            } else {
                0.0
            };
            pings(&mut c, 20.0 + late, 3);
            at += 0.2;
            c.step(at, 0.6 * float(c.up.rate), 0.6 * float(c.down.rate));
            lowest = [lowest[0].min(c.up.rate), lowest[1].min(c.down.rate)];
        }
        assert!(lowest[0] >= 9_500, "{lowest:?}");
        assert!(lowest[1] >= 47_500, "{lowest:?}");
    }

    #[test]
    fn a_wi_fi_line_s_best_is_the_most_it_has_carried() {
        let bounds = |min| Bounds {
            min,
            max: 1_000_000,
        };
        let mut c = Controller::new(
            Settings::for_link(true),
            Direction::new(bounds(1_000), 10_000),
            Direction::new(bounds(5_000), 50_000),
        );
        pings(&mut c, 20.0, 6);
        // A push finds the line faster than where it started.
        let mut at = 0.0;
        while at < 30.0 {
            pings(&mut c, 20.0, 3);
            at += 0.2;
            c.step(at, float(c.up.rate), 0.0);
        }
        let climbed = c.up.rate;
        assert!(climbed > 15_000, "{climbed}");
        let mut lowest = u64::MAX;
        while at < 120.0 {
            let late = if at % 3.0 < 1.0 { 60.0 } else { 0.0 };
            pings(&mut c, 20.0 + late, 3);
            at += 0.2;
            c.step(at, 0.6 * float(c.up.rate), 0.0);
            lowest = lowest.min(c.up.rate);
        }
        assert!(
            float(lowest) >= 0.95 * float(climbed) - 1.0,
            "{lowest} of {climbed}"
        );
    }

    #[test]
    fn a_wi_fi_line_remembers_its_best_past_a_cut_at_its_floor() {
        let bounds = |min| Bounds {
            min,
            max: 1_000_000,
        };
        let mut c = Controller::new(
            Settings::for_link(true),
            Direction::new(bounds(1_000), 10_000),
            Direction::new(bounds(5_000), 50_000),
        );
        pings(&mut c, 20.0, 6);
        // Live, late pings cut the download to its floor, a quiet spell
        // put its best back, and the next late pings cut that too: the
        // memory kept the floor, and the restart after made it the best,
        // so its floor fell from 40 Mbit/s to 38.
        let mut at = 0.0;
        let late_for = |c: &mut Controller, at: &mut f64, until: f64| {
            while *at < until {
                let late = if *at % 3.0 < 1.0 { 60.0 } else { 0.0 };
                pings(c, 20.0 + late, 3);
                *at += 0.2;
                c.step(*at, 0.6 * float(c.up.rate), 0.6 * float(c.down.rate));
            }
        };
        late_for(&mut c, &mut at, 60.0);
        while at < 760.0 {
            pings(&mut c, 20.0, 3);
            at += 0.2;
            c.step(at, 0.0, 0.0);
        }
        late_for(&mut c, &mut at, 820.0);
        let s = c.settings.clone();
        assert!(c.down.peak < 50_000, "{}", c.down.peak);
        assert_eq!(c.down.remembered(&s), 50_000);
        assert_eq!(c.up.remembered(&s), 10_000);
    }

    fn wi_fi(up: u64) -> Controller {
        Controller::new(
            Settings::for_link(true),
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
                50_000,
            ),
        )
    }

    #[test]
    fn a_wi_fi_line_slower_than_its_floor_is_followed_down() {
        // Live, an upload learned 12.3 Mbit/s, so its floor held it at 11.7
        // when the line carried 9.3: the queue stood at 60 to 70 ms in the
        // router, ahead of every ping and call. The queue was this
        // machine's: it stayed for as long as the upload ran.
        let mut c = wi_fi(12_000);
        pings(&mut c, 20.0, 6);
        let mut line = Line {
            queue_kbit: 0.0,
            at: 0.0,
        };
        line.run(&mut c, 9_000.0, 90.0);
        // Then it holds just under the line, losing little of it.
        let (mut lowest, mut worst) = (u64::MAX, 0.0_f64);
        for _ in 0..30 {
            worst = worst.max(line.run(&mut c, 9_000.0, 1.0));
            lowest = lowest.min(c.up.rate);
        }
        assert!(c.up.rate < 9_000, "{}", c.up.rate);
        assert!(lowest >= 8_100, "{lowest}");
        assert!(worst < 50.0, "{worst} ms");
    }

    #[test]
    fn a_busy_wi_fi_upload_keeps_its_best_through_the_air_s_late_pings() {
        // The floor's reason: pings late for the air's sake, a second of
        // every three, while the upload fills its rate on a faster line.
        let mut c = wi_fi(10_000);
        pings(&mut c, 20.0, 6);
        let (mut at, mut lowest) = (0.0, u64::MAX);
        while at < 180.0 {
            let late = if at % 3.0 < 1.0 { 60.0 } else { 0.0 };
            pings(&mut c, 20.0 + late, 3);
            at += 0.2;
            c.step(at, float(c.up.rate), 0.0);
            lowest = lowest.min(c.up.rate);
        }
        assert!(lowest >= 9_500, "{lowest}");
        assert!(c.up.best >= 10_000, "{}", c.up.best);
    }

    #[test]
    fn a_download_held_back_here_keeps_its_best_through_a_standing_queue() {
        // Binks's case on core #616: a Wi-Fi download at its floor, arriving
        // at 1.1 times its rate, is held back here and fills no line; with
        // pings standing 60 ms late, its rate fell to 18 Mbit/s in a minute
        // and a best of 19 was kept.
        let mut c = wi_fi(10_000);
        // Cut to its floor earlier.
        c.down.held = 47_500;
        c.down.rate = 47_500;
        pings(&mut c, 20.0, 6);
        let mut at = 0.0;
        while at < 120.0 {
            pings(&mut c, 80.0, 3);
            at += 0.2;
            c.step(at, 0.0, 1.1 * float(c.down.rate));
        }
        assert!(c.down.best >= 50_000, "{}", c.down.best);
        assert!(c.down.rate >= 47_500, "{}", c.down.rate);
    }

    /// Uploads four seconds of every five into a line of 11 Mbit/s, under a
    /// best of 12.3 whose floor is over the line, and gives the worst
    /// queue of the last minute, in ms, and the least rate held in it: the
    /// queue rises and drains by turns, as it did live at 20:21, never
    /// standing long.
    fn bursts_over_the_floor(live: bool) -> (f64, u64) {
        let mut c = wi_fi(12_300);
        c.live = live;
        pings(&mut c, 20.0, 6);
        let (mut at, mut queue_kbit, mut worst, mut least) = (0.0, 0.0_f64, 0.0_f64, u64::MAX);
        while at < 240.0 {
            let sent = if at % 5.0 < 4.0 {
                float(c.up.rate)
            } else {
                0.0
            };
            queue_kbit = (queue_kbit + (sent - 11_000.0) * 0.2).clamp(0.0, 8_000.0);
            let delay = queue_kbit / 11_000.0 * 1000.0;
            pings(&mut c, 20.0 + delay, 3);
            at += 0.2;
            c.step(at, sent.min(11_000.0), 0.0);
            if at > 180.0 {
                worst = worst.max(delay);
                least = least.min(c.up.rate);
            }
        }
        (worst, least)
    }

    #[test]
    fn while_real_time_flows_a_wi_fi_line_keeps_its_queue_here() {
        // Live, a bulk upload took pings past the router from 27 ms to 44
        // on average, up to 134, while the floor held it at 11.7 Mbit/s:
        // its queue was the line's, where a call's packets wait behind it.
        let (held, _) = bursts_over_the_floor(false);
        let (live, rate) = bursts_over_the_floor(true);
        assert!(held > 60.0, "{held} ms at the floor");
        assert!(live < 30.0, "{live} ms while live, against {held}");
        // It gives up little of the line for it.
        assert!(rate >= 8_800, "{rate}");
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
    fn a_queue_back_within_moments_of_a_cut_is_the_same_spell() {
        let mut c = controller(20_000, 100_000);
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        c.step(1.0, 11_000.0, 0.0);
        let (rate, held) = (c.up.rate, c.up.held);
        // On Wi-Fi a draining queue's pings come back early, then late.
        pings(&mut c, 20.0, 6);
        pings(&mut c, 200.0, 3);
        assert_eq!(c.step(2.0, 9_000.0, 0.0), State::Holding);
        assert_eq!((c.up.rate, c.up.held), (rate, held));
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

    /// An upload filling all it may on a `line` kbit/s line 20 ms away, as
    /// `Line` does, with pings `late` ms late for 0.8 s of every `every`
    /// that no queue of its made, as on Wi-Fi or a busy host. Steps for
    /// `seconds`, and gives the lowest rate after the first ten and how many
    /// times it cut.
    fn upload_beside_noise(
        c: &mut Controller,
        line: f64,
        seconds: f64,
        every: f64,
        late: f64,
    ) -> (u64, usize) {
        let dt = 0.2;
        let (mut queue_kbit, mut at, mut lowest, mut cuts) = (0.0_f64, 0.0, u64::MAX, 0);
        while at < seconds {
            let sent = float(c.up.rate);
            queue_kbit = (queue_kbit + (sent - line) * dt).clamp(0.0, 8_000.0);
            let noise = if at % every < 0.8 { late } else { 0.0 };
            pings(c, 20.0 + queue_kbit / line * 1000.0 + noise, 3);
            at += dt;
            if c.step(at, sent.min(line), 0.0) == State::Cut {
                cuts += 1;
            }
            if at > 10.0 {
                lowest = lowest.min(c.up.rate);
            }
        }
        (lowest, cuts)
    }

    #[test]
    fn pings_late_for_no_queue_of_ours_leave_the_upload_near_its_rate() {
        // Cut a tenth each time, and slow to climb past what it held, an
        // upload fell to its floor within two minutes of this.
        let mut c = controller(3_300, 50_000);
        pings(&mut c, 20.0, 6);
        let (lowest, _) = upload_beside_noise(&mut c, 8_000.0, 120.0, 6.0, 18.0);
        assert!(lowest > 2_800, "{lowest}");
        assert!(c.up.rate >= 3_300, "{}", c.up.rate);
    }

    #[test]
    fn a_line_a_retry_confirmed_is_cut_no_more_often() {
        // Retrying every cut cut a steady line 50 times in two minutes,
        // where it took 11.
        let mut c = controller(10_000, 50_000);
        pings(&mut c, 20.0, 6);
        upload_beside_noise(&mut c, 8_000.0, 60.0, f64::INFINITY, 0.0);
        let (_, cuts) = upload_beside_noise(&mut c, 8_000.0, 120.0, f64::INFINITY, 0.0);
        assert!(cuts <= 12, "{cuts}");
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
    fn a_dip_leaves_the_line_s_best_to_be_remembered() {
        // A restart starts from what is remembered: a cut remembered in
        // place of the best held a download on Wi-Fi at 5 Mbit/s with no
        // peak to forget back to, where the line carried 85.
        let (c, _) = cut_by_a_dip();
        assert_eq!(c.down.remembered(&c.settings), 95_000);
    }

    #[test]
    fn a_peak_the_line_no_longer_carries_is_remembered_no_more() {
        let (mut c, mut line) = cut_by_a_dip();
        line.rest(&mut c, 600.0);
        // The line stayed at 50: the peak put back is cut again, and that
        // cut is what the line now holds.
        line.run(&mut c, 50_000.0, 10.0, 1_000.0);
        let remembered = c.down.remembered(&c.settings);
        assert!(remembered < 50_000, "{remembered}");
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
