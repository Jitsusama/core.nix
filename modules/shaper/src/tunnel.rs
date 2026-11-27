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

use crate::oneway::{ELSEWHERE_S, FAR_CUTS, FAR_MOST_S, FAR_S, VPN_MARGIN_MS};

/// Each cut takes the VPN's share of its lane this far down, never under
/// `LEAST`, and it climbs back by `RISE` of its lane a second. On 16
/// November a proof's agents filled the tunnel past what it delivered, 7
/// to 9 Mbit/s against a CAKE at 9.1, and Joel's upload inside the VPN got
/// 2 to 3% of the link.
const CUT: f64 = 0.85;
const LEAST: f64 = 0.25;
const RISE: f64 = 0.02;

/// A wait past this many times the lowest its run's cuts took it to has
/// refilled, as a tunnel that slows does, rather than wavered, as a far
/// path's does within a few ms either way.
const REFILLED: f64 = 2.0;

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
    /// The wait's mean before the last cut, while a run of cuts lasts.
    before_ms: Option<f64>,
    /// The lowest the wait's mean has come to in a run of cuts, once a
    /// cut first lowered it.
    lowest_ms: Option<f64>,
    /// The wait's mean that set the last cut, for saying.
    waited_ms: f64,
    /// Futile cuts in a row.
    futile: u32,
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
            before_ms: None,
            lowest_ms: None,
            waited_ms: 0.0,
            futile: 0,
            unheeded_until: f64::NEG_INFINITY,
            unheeded_for: 0.0,
        }
    }
}

impl Hold {
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
    /// `ELSEWHERE_S` it averages past `late_ms`. A cut is futile when the
    /// wait is no lower after it, or, once a cut has lowered it, comes to
    /// no new low. `FAR_CUTS` futile cuts in a row show a far path no hold
    /// drains, which goes unheeded a while, as the controller's own far
    /// rule does.
    pub fn step(&mut self, at: f64, dt: f64, waiting: Option<f64>, late_ms: f64) {
        let waiting = waiting.filter(|_| at >= self.unheeded_until);
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
        self.clear = !standing;
        if !standing {
            // Drained, or never there: a run of cuts ends, and a far path
            // found again later is unheeded for the first while again.
            self.lowest_ms = None;
            if self.before_ms.take().is_some() {
                self.unheeded_for = 0.0;
            }
            self.futile = 0;
            self.rise(dt);
            return;
        }
        // Once a cut has lowered the wait, a cut works only if it takes the
        // wait to a new low: a far path's wait wavers, and one window's dip
        // under the last would otherwise count as one drained, so a run of
        // futile cuts may never come to `FAR_CUTS`. On 26 November a 68 ms
        // wait held the VPN's queue at 37% of its lane, while the link
        // carried about 8 of its 10.8 Mb/s and the agents inside the VPN
        // waited three seconds on average. Until a cut first lowers it,
        // each is judged against the window before, as a tunnel's queue
        // still grows for the first cuts over its pace; and a wait past
        // `REFILLED` times its low has refilled, as when the tunnel slows,
        // so it's judged so again rather than against a low from before.
        if self
            .lowest_ms
            .is_some_and(|lowest| mean > lowest * REFILLED)
        {
            (self.lowest_ms, self.futile) = (None, 0);
        }
        match self.lowest_ms.or(self.before_ms) {
            Some(than) if mean >= than - VPN_MARGIN_MS => {
                self.futile += 1;
                self.lowest_ms = self.lowest_ms.map(|lowest| lowest.min(mean));
            }
            // Only a run that drains ends the doubling of the time a far
            // path goes unheeded: a wavering one's first dip would.
            Some(_) => (self.futile, self.lowest_ms) = (0, Some(mean)),
            None => {}
        }
        if self.futile >= FAR_CUTS {
            self.unheeded_for = (self.unheeded_for * 2.0).clamp(FAR_S, FAR_MOST_S);
            self.unheeded_until = at + self.unheeded_for;
            (self.share, self.before_ms, self.lowest_ms, self.futile) = (1.0, None, None, 0);
            self.clear = true;
            return;
        }
        (self.before_ms, self.waited_ms) = (Some(mean), mean);
        self.share = (self.share * CUT).max(LEAST);
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
    /// next of `waits`, in ms, from `from`. Answers the least share it
    /// came to, and where it ended.
    fn judged(hold: &mut Hold, from: f64, waits: &[f64]) -> (f64, f64) {
        let mut least = hold.share();
        let mut at = from;
        let end = waits.iter().fold(from, |end, _| end + ELSEWHERE_S);
        while at < end {
            let window = waits
                .iter()
                .zip(0_u32..)
                .take_while(|&(_, at_window)| from + ELSEWHERE_S * f64::from(at_window) <= at)
                .last()
                .map_or(0.0, |(&ms, _)| ms);
            hold.step(at, DT, Some(window), LATE_MS);
            least = least.min(hold.share());
            at += DT;
        }
        (least, at)
    }

    // A far path's wait wavers from one judgement to the next: a dip under
    // the last judgement's isn't a drain, so the cuts come to no new low
    // and are let go, whichever wait the cycle starts on, rather than
    // taking the share to its least and holding it there.
    #[test]
    fn a_wavering_wait_no_cut_lowers_lets_the_share_go() {
        for first in 0..3 {
            let waits: Vec<f64> = (first..first + 15)
                .map(|window| [68.0, 60.0, 74.0][window % 3])
                .collect();
            let mut hold = Hold::default();
            let (least, _) = judged(&mut hold, 0.0, &waits);
            // Six cuts at most, 0.377 of the lane, where the old rule's
            // went on to the least and stayed.
            assert!(
                least > 0.35,
                "a far path wavering from its {first}th wait cut the share to {least:.3}"
            );
            assert!(
                (hold.share() - 1.0).abs() < f64::EPSILON,
                "a far path wavering from its {first}th wait held the share at {:.3}",
                hold.share()
            );
        }
    }

    // A tunnel that slows mid-run refills: its wait climbs past twice the
    // low the cuts took it to, and drains again under cuts, but not under
    // that old low at once. Its cuts are judged against the window before
    // again, so a drain is still a drain, rather than three cuts that
    // came to no new low.
    #[test]
    fn a_refilled_tunnel_drains_again_under_its_cuts() {
        let mut hold = Hold::default();
        let (least, _) = judged(&mut hold, 0.0, &[100.0, 40.0, 20.0, 60.0, 45.0, 30.0, 30.0]);
        assert!(
            hold.share() < 0.5,
            "a tunnel draining again after a refill was let go, at {:.3} from a least of \
             {least:.3}",
            hold.share()
        );
    }

    // Each far path found again is unheeded twice as long as the last, up
    // to `FAR_MOST_S`: a wavering wait's first dip doesn't start it over.
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
