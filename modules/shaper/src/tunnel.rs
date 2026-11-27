//! The hold on the VPN's own queue, for a wait inside its tunnel while the
//! line has room. The VPN's daemon sends what its CAKE hands it at the
//! pace its tunnel allows, so a CAKE set over that pace moves the queue
//! past it, where Joel's packets inside the VPN wait alike with agents'.
//! Held under that pace, the queue stays in the CAKE, which sends his
//! first. The link's own rate is left alone, so agents' direct lane
//! borrows what the VPN no longer takes.
//!
//! The same hold, a second one, holds agents' direct lane instead while
//! they send beside the VPN's wait (src/oneway.rs): there what they send
//! is what the VPN's flow may wait behind, and they yield first.

use std::collections::VecDeque;

use crate::oneway::{ELSEWHERE_S, FAR_CUTS, FAR_MOST_S, FAR_S, VPN_MARGIN_MS};

/// Each cut takes the VPN's share of its lane this far down, never under
/// `LEAST`, and it climbs back by `RISE` of its lane a second. On 16
/// November a proof's agents filled the tunnel past what it delivered, 7
/// to 9 Mbit/s against a CAKE at 9.1, and Joel's upload inside the VPN got
/// 2 to 3% of the link.
const CUT: f64 = 0.85;
const LEAST: f64 = 0.25;
const RISE: f64 = 0.02;

/// How many of a run's last judgements a wait is held against, twice
/// `FAR_CUTS`: long enough that a far path's waver comes back to its
/// height within them, short enough that one spike is forgotten.
const HEIGHTS: usize = 6;

/// How long back the waits judged with the share at `LEAST` are held
/// against one another, across runs, and how long back the highest wait
/// that stood is held against what stands once they're let go. A wait no
/// cut lowers stands there again between clear judgements. So may a
/// tunnel paced a little over `LEAST`, which saws through it, draining
/// there and refilling once the share climbs past its pace: letting it
/// go lets its queue climb past any wait it stood at here, and that
/// takes the hold back.
const AT_LEAST_S: f64 = 30.0;

/// The VPN's own queue's hold: the share of its lane it's held to, and
/// what its cuts found.
#[derive(Clone, Debug)]
pub struct Hold {
    share: f64,
    /// When the wait was last judged, at most every `ELSEWHERE_S`, and
    /// what it averaged since.
    judged_at: Option<f64>,
    sum_ms: f64,
    heard: u32,
    /// Whether the last judgement found no wait, so the share climbs.
    clear: bool,
    /// The wait's means at the last `HEIGHTS` cuts while a run lasts,
    /// oldest first.
    heights: VecDeque<f64>,
    /// Cuts while a run lasts.
    cuts: u32,
    /// The waits' means that stood with the share at `LEAST` in the last
    /// `AT_LEAST_S`, when, and whether each was lower than any before it
    /// there.
    at_least: VecDeque<(f64, f64, bool)>,
    /// The waits' means that stood in the last `AT_LEAST_S`, at any share,
    /// and when.
    stood: VecDeque<(f64, f64)>,
    /// Whether a wait at `LEAST` no cut lowers is let go. Agents' direct
    /// lane keeps its hold there: its wait is the VPN's flow's, which a
    /// let-go hold would send to the VPN's own queue to be cut for.
    lets_go_at_least: bool,
    /// While a wait let go at `LEAST` goes unheeded, the share it was let
    /// go from and the highest wait that stood before.
    let_go: Option<(f64, f64)>,
    /// Whether the last judgement while let go stood past the highest
    /// wait before, so the share waits on the next to say whether it's
    /// the tunnel's queue or a spike: one spike rarely spans two
    /// judgements, while a queue that climbs as the share does stands
    /// past it on each (tunnel hold at its least round 2's F19).
    over: bool,
    /// Until when no wait at `LEAST` is let go, once one let go there
    /// showed a tunnel's queue.
    queued_until: f64,
    /// The wait's mean that set the last cut, for saying.
    waited_ms: f64,
    unheeded_until: f64,
    unheeded_for: f64,
}

impl Default for Hold {
    fn default() -> Hold {
        Hold {
            share: 1.0,
            judged_at: None,
            sum_ms: 0.0,
            heard: 0,
            clear: true,
            heights: VecDeque::new(),
            cuts: 0,
            at_least: VecDeque::new(),
            stood: VecDeque::new(),
            lets_go_at_least: true,
            let_go: None,
            over: false,
            queued_until: f64::NEG_INFINITY,
            waited_ms: 0.0,
            unheeded_until: f64::NEG_INFINITY,
            unheeded_for: 0.0,
        }
    }
}

impl Hold {
    /// The hold on agents' direct lane, which keeps its hold at `LEAST`.
    pub fn beside_agents() -> Hold {
        Hold {
            lets_go_at_least: false,
            ..Hold::default()
        }
    }

    /// Whether a wait at `LEAST` no cut lowers is let go.
    #[cfg(test)]
    pub fn lets_go_at_least(&self) -> bool {
        self.lets_go_at_least
    }

    /// The share of its lane the VPN's own queue is held to.
    pub fn share(&self) -> f64 {
        self.share
    }

    /// The wait inside the tunnel that set the last cut, in ms.
    pub fn waited_ms(&self) -> f64 {
        self.waited_ms
    }

    /// Whether a wait at `at` is heeded: not while one its cuts didn't
    /// lower goes unheeded.
    pub fn heeding(&self, at: f64) -> bool {
        at >= self.unheeded_until
    }

    /// Steps the hold `dt` seconds on to `at`, with the VPN's wait past
    /// the line in ms when it's the tunnel's, or none, and the wait past
    /// which a queue counts as standing. A wait cuts the share every
    /// `ELSEWHERE_S` it averages past `late_ms`. Once `FAR_CUTS` cuts
    /// have passed, a wait back within the margin of the highest of the
    /// last `HEIGHTS` shows a far path no hold drains, which goes unheeded
    /// a while, as the controller's own far rule does. So does a wait
    /// standing `FAR_CUTS` times in `AT_LEAST_S` with the share at `LEAST`,
    /// which no cut takes lower, each time finding no new low since the
    /// last. That one is let go to climb back by `RISE`, still heard, and
    /// a wait standing past the highest of the last `AT_LEAST_S` on two
    /// judgements running takes the hold back, the share climbing no
    /// further between them.
    pub fn step(&mut self, at: f64, dt: f64, waiting: Option<f64>, late_ms: f64) {
        if at >= self.unheeded_until {
            self.let_go = None;
        }
        let waiting = waiting.filter(|_| at >= self.unheeded_until || self.let_go.is_some());
        if let Some(ms) = waiting {
            self.sum_ms += ms;
            self.heard += 1;
        }
        let judged_at = *self.judged_at.get_or_insert(at);
        if at - judged_at < ELSEWHERE_S {
            self.rise(dt);
            return;
        }
        let mean = self.sum_ms / f64::from(self.heard.max(1));
        let standing = self.heard > 0 && mean > late_ms;
        (self.judged_at, self.sum_ms, self.heard) = (Some(at), 0.0, 0);
        if standing {
            self.stood_at(at, mean);
        }
        if let Some((share, highest)) = self.let_go {
            let over = standing && mean > highest + VPN_MARGIN_MS;
            let again = over && self.over;
            self.over = over;
            if again {
                // The queue climbed as the share did: it's the tunnel's.
                self.let_go = None;
                self.over = false;
                (self.share, self.clear) = (share, false);
                (self.unheeded_until, self.unheeded_for) = (at, 0.0);
                self.queued_until = at + FAR_MOST_S;
                return;
            }
            self.clear = !over;
            self.rise(dt);
            return;
        }
        self.clear = !standing;
        if !standing {
            // Drained, or never there: a run of cuts ends, and a far path
            // found again later is unheeded for the first while again.
            if self.cuts > 0 {
                self.unheeded_for = 0.0;
            }
            self.end_run();
            self.rise(dt);
            return;
        }
        // A tunnel's queue, cut under its pace, drains and never comes back
        // to a height it had while the cuts go on, though it may refill
        // short of it. A far path's wait wavers about where it stands and
        // keeps coming back. Judged window against window, each dip of a
        // waver read as a drain: on 26 November a 68 ms wait held the VPN's
        // queue at 37% of its lane, while the link carried about 8 of its
        // 10.8 Mb/s and the agents inside the VPN waited three seconds on
        // average. A cut never ends the doubling of the time a far path
        // goes unheeded: only a run that drains does.
        let height = self
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        // At its least no cut is left to drain a wait, and one that keeps
        // standing there, finding no new low, isn't the tunnel's: from
        // 05:07 on 27 November, by air, the VPN's round trip spiked to 100
        // to 230 ms every 6 to 10 s, at a share of 0.99 as at 0.25, and
        // stood 13 to 26 ms over its least between. Each spike started a
        // run that a clear judgement ended before it was found far, so the
        // share sawed between its least and 0.41 for a minute, the VPN's
        // CAKE at 1 to 1.4 Mbit/s, under what bulk uploads got.
        // Let go, such a wait climbs back by `RISE`, not at once, so a
        // tunnel paced a little over the least that saws through it, and
        // so looks the same, overfills only a little before it's taken
        // back. On a model of a queue paced at 0.27 of the lane, one let go
        // at once stood over half its lane for 565 of 600 s.
        let floored = self.lets_go_at_least
            && self.share <= LEAST
            && at >= self.queued_until
            && self.unlowered_at_least(at, mean);
        if floored || (self.cuts >= FAR_CUTS && mean >= height - VPN_MARGIN_MS) {
            self.unheeded_for = (self.unheeded_for * 2.0).clamp(FAR_S, FAR_MOST_S);
            self.unheeded_until = at + self.unheeded_for;
            self.end_run();
            self.at_least.clear();
            if floored {
                let highest = self.stood.iter().map(|&(_, ms)| ms).fold(mean, f64::max);
                self.let_go = Some((self.share, highest));
                self.over = false;
            } else {
                self.share = 1.0;
            }
            self.clear = true;
            return;
        }
        if self.heights.len() == HEIGHTS {
            self.heights.pop_front();
        }
        self.heights.push_back(mean);
        self.cuts += 1;
        self.waited_ms = mean;
        self.share = (self.share * CUT).max(LEAST);
    }

    fn end_run(&mut self) {
        self.heights.clear();
        self.cuts = 0;
    }

    /// Takes in a wait that stood at `at`, forgetting those past
    /// `AT_LEAST_S`.
    fn stood_at(&mut self, at: f64, mean: f64) {
        while self
            .stood
            .front()
            .is_some_and(|&(when, _)| at - when > AT_LEAST_S)
        {
            self.stood.pop_front();
        }
        self.stood.push_back((at, mean));
    }

    /// Takes in a wait standing at `at` with the share at `LEAST`, held
    /// against those there in the last `AT_LEAST_S`: it's a new low when
    /// under all of them by `VPN_MARGIN_MS`, and those before it are
    /// forgotten, since it lowered them. Answers whether `FAR_CUTS` of
    /// them found none since.
    fn unlowered_at_least(&mut self, at: f64, mean: f64) -> bool {
        while self
            .at_least
            .front()
            .is_some_and(|&(when, _, _)| at - when > AT_LEAST_S)
        {
            self.at_least.pop_front();
        }
        let lowest = self
            .at_least
            .iter()
            .map(|&(_, ms, _)| ms)
            .fold(f64::INFINITY, f64::min);
        let lower = mean < lowest - VPN_MARGIN_MS;
        if lower {
            self.at_least.clear();
        }
        self.at_least.push_back((at, mean, lower));
        let unlowered: u32 = self
            .at_least
            .iter()
            .map(|&(_, _, lower)| u32::from(!lower))
            .sum();
        unlowered >= FAR_CUTS
    }

    fn rise(&mut self, dt: f64) {
        if self.clear {
            self.share = (self.share + RISE * dt).min(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATE_MS: f64 = 15.0;
    const DT: f64 = 0.1;

    fn run(hold: &mut Hold, from: f64, seconds: f64, waiting: impl Fn(f64, f64) -> Option<f64>) {
        let mut at = from;
        while at < from + seconds {
            let ms = waiting(at, hold.share());
            hold.step(at, DT, ms, LATE_MS);
            at += DT;
        }
    }

    /// A tunnel whose pace is under 0.29 of the VPN's lane: past that, the
    /// wait grows with the share, and under it, it drains.
    fn slow_tunnel(share: f64) -> f64 {
        (400.0 * share - 115.0).max(0.0)
    }

    #[test]
    fn a_standing_wait_cuts_the_share_every_two_seconds_until_it_drains() {
        let mut hold = Hold::default();
        run(&mut hold, 0.0, 3.0, |_, share| Some(slow_tunnel(share)));
        assert!(
            (hold.share() - CUT).abs() < 0.01,
            "two seconds' wait took the share to {:.3}",
            hold.share()
        );
        run(&mut hold, 3.0, 120.0, |_, share| Some(slow_tunnel(share)));
        // The wait counts as late past 0.325 of the lane, and the share
        // climbs on for a judgement's two seconds, 0.04, before it's cut.
        assert!(
            (LEAST..0.37).contains(&hold.share()),
            "a tunnel paced at 0.29 of the lane left the share at {:.3}",
            hold.share()
        );
    }

    #[test]
    fn a_wait_no_cut_lowers_lets_the_share_go_and_goes_unheeded() {
        let mut hold = Hold::default();
        run(&mut hold, 0.0, 12.0, |_, _| Some(120.0));
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "a far path's wait held the share at {:.3}",
            hold.share()
        );
        run(&mut hold, 12.0, 60.0, |_, _| Some(120.0));
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "an unheeded far path cut the share to {:.3}",
            hold.share()
        );
    }

    /// Steps a hold through judgements whose two seconds each read the
    /// next of `waits`, in ms, from `from`, judging the last too. Its step
    /// is a power of two's part of a second, so each judgement falls on
    /// its window's end exactly and takes in that window's waits alone.
    /// Answers the least share it came to.
    fn judged(hold: &mut Hold, from: f64, waits: &[f64]) -> f64 {
        const STEP: f64 = 0.125;
        let mut least = hold.share();
        let mut at = from;
        let end = waits.iter().fold(from, |end, _| end + ELSEWHERE_S);
        while at <= end {
            let window = waits
                .iter()
                .zip(0_u32..)
                .skip(1)
                .take_while(|&(_, window)| from + ELSEWHERE_S * f64::from(window) < at)
                .last()
                .map_or(waits[0], |(&ms, _)| ms);
            hold.step(at, STEP, Some(window), LATE_MS);
            least = least.min(hold.share());
            at += STEP;
        }
        least
    }

    // A far path's wait wavers from one judgement to the next and comes
    // back to its height, so its cuts are let go, whichever wait the cycle
    // starts on and however far its peak is past its trough, rather than
    // taking the share to its least and holding it there.
    #[test]
    fn a_wavering_wait_no_cut_lowers_lets_the_share_go() {
        let cycles: [&[f64]; 4] = [
            &[68.0, 60.0, 74.0],
            &[18.0, 40.0, 25.0],
            &[20.0, 45.0, 30.0, 22.0],
            &[60.0, 62.0, 70.0, 64.0, 58.0],
        ];
        for cycle in cycles {
            for first in 0..cycle.len() {
                let waits: Vec<f64> = (first..first + 20)
                    .map(|window| cycle[window % cycle.len()])
                    .collect();
                let mut hold = Hold::default();
                let least = judged(&mut hold, 0.0, &waits);
                // Seven cuts at most, 0.321 of the lane, where the old
                // rule's went on to the least and stayed.
                assert!(
                    least > 0.3,
                    "a far path wavering through {cycle:?} from its {first}th wait cut the \
                     share to {least:.3}"
                );
                assert!(
                    (hold.share() - 1.0).abs() < f64::EPSILON,
                    "a far path wavering through {cycle:?} from its {first}th wait held the \
                     share at {:.3}",
                    hold.share()
                );
            }
        }
    }

    // One spike sets a height the waver never comes back to, but only
    // for `HEIGHTS` judgements: then it's forgotten, and the cuts are let
    // go all the same.
    #[test]
    fn a_spike_in_a_far_wait_is_forgotten() {
        let waits = [
            70.0, 150.0, 68.0, 72.0, 66.0, 70.0, 68.0, 74.0, 66.0, 70.0, 68.0, 74.0, 66.0,
        ];
        let mut hold = Hold::default();
        let least = judged(&mut hold, 0.0, &waits);
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "a far wait with one spike held the share at {:.3}, from a least of {least:.3}",
            hold.share()
        );
    }

    // By air a wait that spikes and falls away by itself, as on 27
    // November, ends each run with a clear judgement before it's found
    // far, and the cuts outrun the climbs between. At the least share,
    // where no cut is left, it stands again and finds no new low, so the
    // cuts are let go rather than holding the VPN's CAKE at its least.
    #[test]
    fn a_spiking_wait_no_cut_lowers_is_let_go_at_the_least() {
        let cycle = [150.0, 60.0, 40.0, 30.0, 10.0, 10.0];
        let waits: Vec<f64> = (0..48).map(|window| cycle[window % cycle.len()]).collect();
        let mut hold = Hold::default();
        let least = judged(&mut hold, 0.0, &waits);
        assert!(
            (least - LEAST).abs() < f64::EPSILON,
            "the spiking wait's cuts came only to {least:.3}, never to the least"
        );
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "a spiking wait no cut lowered held the share at {:.3}",
            hold.share()
        );
        assert!(
            !hold.heeding(48.0 * ELSEWHERE_S),
            "the spiking wait let go was heeded again at once"
        );
    }

    /// Steps a hold for `seconds` over a tunnel with a queue of its own,
    /// paced at `pace` of the lane: what the share sends past the pace
    /// fills it, and what the pace sends past the share drains it. Answers
    /// the seconds the share stood over half its lane, the highest wait,
    /// in ms, and whether its hold was let go at any step.
    fn queued(hold: &mut Hold, pace: f64, seconds: f64) -> (f64, f64, bool) {
        let (mut queue, mut over, mut most) = (0.0_f64, 0.0, 0.0_f64);
        let mut let_go = false;
        let mut at = 0.0;
        while at < seconds {
            queue = (queue + (hold.share() - pace) * DT).max(0.0);
            let ms = queue / pace * 1_000.0;
            most = most.max(ms);
            hold.step(at, DT, Some(ms), LATE_MS);
            let_go |= hold.let_go.is_some();
            if hold.share() > 0.5 {
                over += DT;
            }
            at += DT;
        }
        (over, most, let_go)
    }

    // A tunnel paced a little over the least saws through it, draining
    // there and refilling as the share climbs past its pace, and comes
    // back to much the same waits each time, so it may be let go as a
    // wait no cut lowers is. Its queue then climbs past any wait it stood
    // at on two judgements running, which takes the hold back before the
    // share is far over its pace. The second judgement, which tells it
    // from a spike, lets its wait climb about half a second more, to
    // about 1.6 s at most on a model at 0.27. Let go at once, each of
    // these stood over half its lane for most of the run.
    #[test]
    fn a_tunnel_paced_a_little_over_the_least_keeps_its_hold() {
        for pace in [0.255, 0.27, 0.29] {
            let mut hold = Hold {
                share: pace + 0.03,
                ..Hold::default()
            };
            let (over, most, let_go) = queued(&mut hold, pace, 600.0);
            assert!(
                let_go,
                "a tunnel paced at {pace} of the lane was never let go, so its taking back \
                 went untried"
            );
            assert!(
                over < 12.0,
                "a tunnel paced at {pace} of the lane was let over half of it for {over:.1} s"
            );
            assert!(
                most < 2_000.0,
                "a tunnel paced at {pace} of the lane waited {most:.0} ms"
            );
            assert!(
                hold.heeding(600.0) && hold.let_go.is_none(),
                "a tunnel paced at {pace} of the lane was let go at the run's end"
            );
        }
    }

    // A wait let go at the least spikes by air higher than any spike in
    // the 30 s before, from time to time. Each spike stands one
    // judgement, so none takes the hold back, rather than barring the
    // let-go for `FAR_MOST_S` from the first spike taller than the rest
    // (tunnel hold at its least round 2's F19 and F24).
    #[test]
    fn an_uneven_spike_doesn_t_take_a_let_go_hold_back() {
        let heights = [150.0; 8]
            .into_iter()
            .chain([230.0, 110.0, 190.0, 130.0, 210.0, 170.0]);
        let waits: Vec<f64> = heights
            .flat_map(|height| [height, 60.0, 40.0, 30.0, 10.0, 10.0])
            .collect();
        let end = ELSEWHERE_S * f64::from(u32::try_from(waits.len()).expect("few waits"));
        let mut hold = Hold::default();
        judged(&mut hold, 0.0, &waits);
        assert!(
            hold.let_go.is_some() && !hold.heeding(end),
            "a spiking wait let go at the least was taken back by a taller spike"
        );
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "a spiking wait let go at the least, its spikes uneven, held the share at {:.3}",
            hold.share()
        );
    }

    // A wait at the least that a judgement lowers shows the cuts draining
    // something, so the waits it lowered aren't counted against the next
    // ones: only those found since a new low are.
    #[test]
    fn a_new_low_at_the_least_forgets_the_waits_it_lowered() {
        let mut hold = Hold {
            share: LEAST,
            ..Hold::default()
        };
        judged(&mut hold, 0.0, &[90.0, 80.0, 77.0, 74.0, 40.0, 37.0]);
        assert!(
            hold.heeding(6.0 * ELSEWHERE_S),
            "a wait lowered to a new low at the least was let go"
        );
        assert!(
            (hold.share() - LEAST).abs() < f64::EPSILON,
            "a wait lowered to a new low at the least left the share at {:.3}",
            hold.share()
        );
    }

    // Agents' direct lane keeps its hold at the least, since its wait is
    // the VPN's flow's, which a let-go hold would send to the VPN's own
    // queue to be cut for.
    #[test]
    fn agents_lane_keeps_its_hold_at_the_least() {
        let cycle = [150.0, 60.0, 40.0, 30.0, 10.0, 10.0];
        let waits: Vec<f64> = (0..48).map(|window| cycle[window % cycle.len()]).collect();
        let mut hold = Hold::beside_agents();
        let least = judged(&mut hold, 0.0, &waits);
        assert!(
            (least - LEAST).abs() < f64::EPSILON,
            "the spiking wait's cuts came only to {least:.3}, never to the least"
        );
        assert!(
            hold.heeding(48.0 * ELSEWHERE_S),
            "agents' lane was let go at the least"
        );
    }

    // A tunnel that slows mid-run refills, its wait climbing over the low
    // the cuts took it to, and drains again under cuts, short of a height
    // it had. The queue first grows over the tunnel's pace, too. However
    // often it refills, the drain keeps its hold.
    #[test]
    fn a_refilled_tunnel_drains_again_under_its_cuts() {
        let refills: [&[f64]; 4] = [
            &[100.0, 40.0, 20.0, 60.0, 45.0, 30.0, 30.0, 30.0],
            &[100.0, 40.0, 20.0, 35.0, 28.0, 22.0, 22.0, 22.0],
            &[50.0, 80.0, 100.0, 90.0, 70.0, 55.0, 58.0, 52.0, 50.0, 48.0],
            &[
                100.0, 40.0, 20.0, 60.0, 45.0, 30.0, 50.0, 35.0, 25.0, 45.0, 30.0, 20.0,
            ],
        ];
        for waits in refills {
            let mut hold = Hold::default();
            let least = judged(&mut hold, 0.0, waits);
            assert!(
                hold.share() < 0.5,
                "a tunnel draining again after a refill through {waits:?} was let go, at \
                 {:.3} from a least of {least:.3}",
                hold.share()
            );
        }
    }

    // Each far path found again is unheeded twice as long as the last, up
    // to `FAR_MOST_S`: a wavering wait's cuts don't start it over.
    #[test]
    fn a_far_path_found_again_goes_unheeded_longer() {
        let mut hold = Hold::default();
        let waits: Vec<f64> = (0..350)
            .map(|window| [68.0, 60.0, 74.0][window % 3])
            .collect();
        judged(&mut hold, 0.0, &waits);
        assert!(
            hold.unheeded_for > FAR_S,
            "a far path found a second time went unheeded only {:.0} s",
            hold.unheeded_for
        );
    }

    // A run that drains is a tunnel's own queue, not a far path, so the
    // next far path found is unheeded for the first while again.
    #[test]
    fn a_run_that_drains_starts_the_unheeded_time_over() {
        let mut hold = Hold {
            unheeded_for: FAR_MOST_S,
            ..Hold::default()
        };
        judged(&mut hold, 0.0, &[100.0, 60.0, 10.0]);
        assert!(
            hold.unheeded_for.abs() < f64::EPSILON,
            "a run that drained left the unheeded time at {:.0} s",
            hold.unheeded_for
        );
    }

    #[test]
    fn the_share_climbs_back_once_nothing_waits() {
        let mut hold = Hold::default();
        run(&mut hold, 0.0, 30.0, |_, share| Some(slow_tunnel(share)));
        let low = hold.share();
        assert!(low < 0.5, "thirty seconds' wait held the share at {low:.3}");
        run(&mut hold, 30.0, 45.0, |_, _| None);
        assert!(
            (hold.share() - 1.0).abs() < f64::EPSILON,
            "forty-five clear seconds left the share at {:.3}, from {low:.3}",
            hold.share()
        );
    }
}
