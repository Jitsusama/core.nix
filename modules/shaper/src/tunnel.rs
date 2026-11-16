//! The hold on the VPN's own queue, for a wait inside its tunnel while the
//! line has room. The VPN's daemon sends what its CAKE hands it at the
//! pace its tunnel allows, so a CAKE set over that pace moves the queue
//! past it, where Joel's packets inside the VPN wait alike with agents'.
//! Held under that pace, the queue stays in the CAKE, which sends his
//! first. The link's own rate is left alone, so agents' direct lane
//! borrows what the VPN no longer takes.

use crate::oneway::{ELSEWHERE_S, FAR_CUTS, FAR_MOST_S, FAR_S, VPN_MARGIN_MS};

/// Each cut takes the VPN's share of its lane this far down, never under
/// `LEAST`, and it climbs back by `RISE` of its lane a second. On 16
/// November a proof's agents filled the tunnel past what it delivered, 7
/// to 9 Mbit/s against a CAKE at 9.1, and Joel's upload inside the VPN got
/// 2 to 3% of the link.
const CUT: f64 = 0.85;
const LEAST: f64 = 0.25;
const RISE: f64 = 0.02;

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
    /// The wait's mean that set the last cut, for saying.
    waited_ms: f64,
    /// Cuts in a row after which the wait was no lower.
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

    /// Steps the hold `dt` seconds on to `at`, with the VPN's wait past
    /// the line in ms when it's the tunnel's, or none, and the wait past
    /// which a queue counts as standing. A wait cuts the share every
    /// `ELSEWHERE_S` it averages past `late_ms`; a cut after which it was
    /// no lower is futile, and `FAR_CUTS` of those in a row show a far
    /// path no hold drains, which goes unheeded a while, as the
    /// controller's own far rule does.
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
            if self.before_ms.take().is_some() {
                self.unheeded_for = 0.0;
            }
            self.futile = 0;
            self.rise(dt);
            return;
        }
        match self.before_ms {
            Some(before) if mean >= before - VPN_MARGIN_MS => self.futile += 1,
            Some(_) => {
                self.futile = 0;
                self.unheeded_for = 0.0;
            }
            None => {}
        }
        if self.futile >= FAR_CUTS {
            self.unheeded_for = (self.unheeded_for * 2.0).clamp(FAR_S, FAR_MOST_S);
            self.unheeded_until = at + self.unheeded_for;
            (self.share, self.before_ms, self.futile) = (1.0, None, 0);
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
