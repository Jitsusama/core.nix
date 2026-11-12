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
//! past it, and fast anywhere else. It drifts back toward what the
//! network's connection carries while it's idle or low. The base is
//! learned from the connection: what a direction carried each time its
//! queue stood with the rate well over it, the median of the last ten,
//! less a tenth. A new network starts with no limit at all, so nothing is
//! held down by a guess until the connection shows where its queue is.
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
//! the direction carried a sliver of its connection, so that the queue is
//! plainly another machine's, goes back to its base; after three such
//! returns the controller gives up and leaves the connection unlimited for
//! ten minutes, as it does when no timestamps are answered. Cuts alone
//! don't trip it: a connection filling its buffer is rightly cut each
//! second until it drains.
//!
//! What the connection carries is also what the direction carried for
//! five seconds running with no queue, so a queue met while it carried a
//! sliver can't teach it that the connection carries less than it already
//! has.
//!
//! What a direction learned of its connection, its base and the plateau a
//! router that queues flow by flow showed it, outlasts the stay: a network
//! met again starts from it, so the first upload after a restart or a
//! rejoin is held under that router at once rather than after it has been
//! found again.

use serde::{Deserialize, Serialize};
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
/// or its connection, when its queue stands carries a sliver: the queue is
/// mostly another machine's or the air's, so a cut halves what it could
/// carry, and teaches neither the base nor where the queue stood.
const SLIVER: f64 = 0.25;
/// A queue standing this share of `FUTILE_S` while the direction carries a
/// sliver of its connection is another machine's, however it breaks
/// between.
const CROWDED: f64 = 0.5;
/// How long the connection is left unlimited once the guard gives up.
const GIVEN_UP_S: f64 = 600.0;
/// How long a direction must carry a rate with no queue to prove the
/// connection carries it.
const PROVEN_S: f64 = 5.0;
/// How long after a cut before another.
const BETWEEN_CUTS_S: f64 = 1.0;
/// A queue only the VPN's round trip meets is heard a round trip and a
/// second late, so a cut for it waits this long for the last to show.
/// Cut each second on what was heard before, on 11 November the upload
/// went from 10,759 to 1,744 in 3 s, then to the floor.
const UNSEEN_DRAIN_S: f64 = 5.0;
/// The least share of what a direction reached in its stay that a queue
/// only the VPN's round trip meets cuts it to: the line carried that, and
/// a line that fell further is met by the probes on the link.
const UNSEEN_FLOOR: f64 = 0.5;
/// How fast a full, clear direction climbs, and an idle one drifts, each
/// second.
const CLIMB: f64 = 1.05;
/// How fast a rate climbs, per second, far from any queue it has met:
/// under this share of what got through when one last stood, or with none
/// met yet. A remembered rate far under the connection is back in under
/// ten seconds.
const FAST_CLIMB: f64 = 1.25;
const FAR: f64 = 0.5;
/// How long where a queue last stood slows the climb: past it, as CUBIC
/// does, the connection has likely moved since.
const CEILING_S: f64 = 30.0;
/// A rate this far past where its queue last stood, with none standing, is
/// on a connection that has grown, and climbs fast again.
const PAST: f64 = 1.25;
const DRIFT: f64 = 0.01;
/// What share of its rate counts as full, and what as low.
const FULL: f64 = 0.75;
/// How many of the connection's rates the base is the median of.
const LEARNED: usize = 10;
/// The gateway's jitter on Wi-Fi: its rises past this are the air's.
const AIR_MARGIN_MS: f64 = 8.0;
/// How fast the gateway's baseline may rise, each second, so a roam to a
/// farther access point is followed.
const AIR_RISE_MS: f64 = 0.05;
/// A call's round trip through the VPN jitters by this much, a server's
/// whole path away, with no queue standing.
const VPN_MARGIN_MS: f64 = 5.0;
/// How fast that round trip's baseline may rise, each second: faster than
/// the air's, since a queue for the VPN drains within a second or two of a
/// cut, and a round trip longer than that is the VPN's path moving.
const VPN_RISE_MS: f64 = 1.0;
/// A direction carrying under this share of its rate is idle, and its
/// delay measures the path.
const IDLE: f64 = 0.1;
/// With no timestamps answered for this long, the connection goes
/// unlimited.
const UNANSWERED_S: f64 = 10.0;
/// The live guard's limits.
const GUARD_WINDOW_S: f64 = 60.0;
/// A delay standing this long, cuts and all, isn't this machine's queue.
const FUTILE_S: f64 = 30.0;
const GUARD_RETURNS: usize = 3;
/// How long data must wait to go while our CAKE holds nothing and the link
/// carries under its rate before the queue is taken to be another's.
const ELSEWHERE_S: f64 = 2.0;
/// Under this share of its rate, a link with data waiting and our CAKE
/// holding nothing is held back by something past it.
const UNDER: f64 = 0.98;
/// With our CAKE holding a queue, a router past it dropping over this
/// share of the link's rate, in what TCP sends again past CAKE's own drops,
/// is over its line. Less is the air's, which a cut wouldn't mend.
const DROPPED_PAST: f64 = 0.05;
/// A queue held elsewhere is taken back only while the link carries at
/// least this share of the most it has carried: carrying less, what holds
/// it back is one slow server, or other machines' share of the router, and
/// cutting to it would hold everything else down with it.
const REACHED: f64 = 0.7;
/// What share of what got through, the plateau, a direction is held to
/// once it has found the queue elsewhere.
const HOLD: f64 = 0.95;
/// How often a held direction looks for more room, how far over its
/// plateau, and for how long.
const PROBE_EVERY_S: f64 = 60.0;
const PROBE_UP: f64 = 1.1;
const PROBE_S: f64 = 3.0;

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
    /// Whose the upload's queue is, when the live shaper can tell; a
    /// replay of the recorder's lines can't.
    pub up_ours: Option<Ours>,
    /// A call's round trip through the VPN riding the link, in ms, which
    /// waits in a queue the router keeps for the VPN's flow alone, where
    /// none of the probes' wait.
    pub vpn_ms: Option<f64>,
}

/// What the shaper sees of its own side of a direction: whether its CAKE
/// holds a queue, whether connections on the link have data waiting to
/// go, and what TCP sent again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is one reading of the queues, kept by name in status.json and replays"
)]
pub struct Ours {
    pub queued: bool,
    /// Whether the only queue that stood was a VPN's riding the link,
    /// which sends at most its share of the link's rate.
    pub in_vpn: bool,
    /// Whether a VPN's CAKE riding the link held a queue, whatever the
    /// link's did.
    #[serde(default)]
    pub vpn_queued: bool,
    /// How long the link's CAKE kept what it held, in ms: a wait of ours,
    /// which the VPN's flow takes too before it reaches the router.
    #[serde(default)]
    pub link_wait_ms: u64,
    pub waiting: bool,
    /// What TCP sent again over the last second, in kbit/s. A router held
    /// past what it carries drops the rest flow by flow, and what it
    /// dropped crosses our link twice, so it's no part of the line.
    pub resent: u64,
    /// What our own CAKE, and the VPNs' riding the link, dropped over the
    /// last second, in kbit/s: what TCP sends again of it never crossed
    /// our link the first time, so it's no router's.
    pub dropped: u64,
    /// What the link's CAKE sent over the last second, in kbit/s: what
    /// crossed the line, since traffic to the home network goes around it.
    /// None where the shaper didn't count it.
    #[serde(default)]
    pub sent: Option<u64>,
}

impl Ours {
    /// Whose the upload's queue was, from how many reads of the link's
    /// CAKE and of the VPNs' riding it found a queue, of how many read:
    /// one held in most reads stood, how long the link's kept what it held,
    /// in ms, and what TCP sent again and our queues dropped, in kbit/s.
    /// None before anything was read.
    pub fn from_reads(
        link: (u32, u32),
        vpn: (u32, u32),
        link_wait_ms: u64,
        waiting: bool,
        (resent, dropped): (u64, u64),
    ) -> Option<Self> {
        let stood = |(queued, reads): (u32, u32)| reads > 0 && queued * 2 >= reads;
        (link.1 > 0).then_some(Self {
            queued: stood(link) || stood(vpn),
            in_vpn: stood(vpn) && !stood(link),
            vpn_queued: stood(vpn),
            link_wait_ms,
            waiting,
            resent,
            dropped,
            sent: None,
        })
    }
}

/// What a direction learned of its connection that outlasts a stay, in
/// kbit/s: its base, and the plateau a router that queues flow by flow
/// showed it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Learned {
    /// Kept as `line` since 11 November: a base kept before counted what
    /// the link sent, a transfer across the home network included, and
    /// held 282 Mbit/s of a 12 Mbit/s upload, so each network learns its
    /// bases once more and keeps its plateaus.
    #[serde(rename = "line", default)]
    pub base: Option<f64>,
    /// Kept as `held` since 11 November: cuts a second apart for a queue
    /// only the VPN met saved a plateau of 1,424 on a 12 Mbit/s upload,
    /// which held every later start there, so each network finds its
    /// plateau once more.
    #[serde(rename = "held", default)]
    pub plateau: Option<f64>,
}

impl Learned {
    /// What this says, and what `older` said where this says nothing: a
    /// stay that learned no plateau, a replayed one among them, forgets
    /// none.
    pub fn over(self, older: Learned) -> Learned {
        Learned {
            base: self.base.or(older.base),
            plateau: self.plateau.or(older.plateau),
        }
    }

    /// Whether either rate moved by more than a twentieth from `from`, or
    /// came or went.
    pub fn moved(self, from: Learned) -> bool {
        let far = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => (a - b).abs() * 20.0 > a.max(b),
            (a, b) => a.is_some() != b.is_some(),
        };
        far(self.base, from.base) || far(self.plateau, from.plateau)
    }
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
    /// stay: the connection carries at least that.
    proven: f64,
    /// Each moment of the last `FUTILE_S`, and whether its queue stood
    /// while it carried a sliver of its connection.
    crowded: VecDeque<(f64, bool)>,
    /// What it carried each moment of the last `PROVEN_S`, and the most it
    /// carried for that long, queue or not, in this stay.
    carrying: VecDeque<(f64, f64)>,
    reached: f64,
    /// Since when data has waited with our CAKE holding nothing, and what
    /// got through each moment of it.
    elsewhere: Option<(f64, f64, u32)>,
    /// What got through when the queue was last found past our CAKE, which
    /// the rate is held under, and when it next looks for more room, or
    /// until when it is looking.
    plateau: Option<f64>,
    next_probe: f64,
    probing_until: Option<f64>,
}

impl Way {
    /// A direction on a network whose base is `base`, if one was learned,
    /// starting there; with none, it starts with no limit. A known base
    /// counts as one of the connection's rates, so a single cut under
    /// another machine's flow doesn't replace it.
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
            carrying: VecDeque::new(),
            reached: 0.0,
            elsewhere: None,
            plateau: None,
            next_probe: 0.0,
            probing_until: None,
        }
    }

    /// A direction starting from what it learned before: at its base, and
    /// with a plateau, held under it and looking for more room as soon as
    /// data waits, so a plateau gone stale either way is soon set right.
    pub fn from_learned(learned: Learned) -> Way {
        let mut way = Way::new(learned.base);
        // A plateau under half the base would hold the stay where the
        // guard lets go of it a minute in, and a stay shorter than that,
        // as a proof's are, saves it again: proof 7 on 11 November left
        // 2,620 on a base of 11,148. So a start lets go of it at once.
        let plateau = learned
            .plateau
            .filter(|&plateau| learned.base.is_none_or(|base| plateau >= base * 0.5));
        if let Some(plateau) = plateau {
            way.plateau = Some(plateau);
            way.rate = way.rate.min((plateau * HOLD).max(LEAST_KBIT));
        }
        way
    }

    /// What this direction has learned that outlasts the stay.
    pub fn learned(&self) -> Learned {
        Learned {
            // What a clear spell proved outlasts a lower base, so a base
            // learned wrong is put right by the first fast, clear stay. It
            // makes none where none was learned, which would hold a start
            // the plateau sets.
            base: self.base.map(|base| base.max(self.proven * 0.9)),
            plateau: self.plateau,
        }
    }

    /// Keep the most it carried for `PROVEN_S` running, queue or not.
    fn reach(&mut self, at: f64, carried: f64) {
        self.carrying.push_back((at, carried));
        while self
            .carrying
            .front()
            .is_some_and(|&(t, _)| at - t > PROVEN_S)
        {
            self.carrying.pop_front();
        }
        if self
            .carrying
            .front()
            .is_some_and(|&(t, _)| at - t >= PROVEN_S - 0.05)
        {
            let least = self
                .carrying
                .iter()
                .map(|&(_, c)| c)
                .fold(f64::INFINITY, f64::min);
            self.reached = self.reached.max(least);
        }
    }

    /// Follows a queue our probes can't see: a router that queues flow by
    /// flow, as the eero does, delays no probe however full the link is,
    /// and shares what it carries out by flow, so Joel's upload gets one
    /// share among every agent's. With data waiting, our CAKE holding
    /// nothing and the link carrying under its rate, the queue is past
    /// CAKE: the rate goes a little under what got through, so the queue
    /// is CAKE's again and its tins say who goes first. Held there, it
    /// looks a little higher now and then, and keeps what the link carries
    /// while CAKE still holds a queue, and cut for a queue the probes saw,
    /// climbs back to where it held. Answers whether it set the rate.
    fn own(&mut self, at: f64, dt: f64, ours: Ours, carried: f64) -> bool {
        // What got through the router once: a router dropping what it
        // can't carry has our link carry more than its line. What TCP sent
        // again of what our CAKE dropped crossed the link once, and got
        // through.
        let lost = ours.resent.saturating_sub(ours.dropped);
        let through = (carried - crate::rate::float(lost)).max(0.0);
        // A VPN riding the link sends at most its share of the link's rate,
        // so what it holds is judged by that, and a probe lifts the VPN past
        // the line held rather than only the link.
        let share = if ours.in_vpn {
            crate::qdisc::VPN_SHARE
        } else {
            1.0
        };
        // With our CAKE holding nothing, the link carrying under its rate
        // is held back past it. With CAKE holding a queue, the link carries
        // the rate, and a router past it dropping a share of that is over
        // its line: as when a plateau was found while the router's buffer
        // still took more than its line, or remembered from a faster day.
        let held_back = if ours.queued {
            through < self.rate * share * (1.0 - DROPPED_PAST)
        } else {
            through < self.rate * UNDER
        };
        let elsewhere = ours.waiting
            && held_back
            && through > FLOWING_KBIT
            && self.reached > 0.0
            && through >= self.reached * REACHED;
        if elsewhere {
            let (since, sum, n) = self.elsewhere.get_or_insert((at, 0.0, 0));
            *sum += through;
            *n += 1;
            if at - *since >= ELSEWHERE_S - 0.05 {
                let plateau = *sum / f64::from(*n);
                self.elsewhere = None;
                self.plateau = Some(plateau);
                self.rate = (plateau * HOLD).max(LEAST_KBIT);
                self.probing_until = None;
                self.next_probe = at + PROBE_EVERY_S;
                return true;
            }
        } else {
            self.elsewhere = None;
        }
        let Some(plateau) = self.plateau else {
            return false;
        };
        match self.probing_until {
            Some(until) if at >= until => {
                self.probing_until = None;
                if ours.queued && through >= self.rate * share * UNDER {
                    // The link carried the higher rate with CAKE still
                    // holding a queue, so there is room past it: keep it,
                    // and look higher at once.
                    self.plateau = Some(through);
                    self.next_probe = at;
                } else {
                    self.rate = plateau * HOLD;
                    self.next_probe = at + PROBE_EVERY_S;
                }
            }
            None if at >= self.next_probe && ours.waiting => {
                self.rate = plateau * PROBE_UP / share;
                self.probing_until = Some(at + PROBE_S);
                self.elsewhere = None;
            }
            // The queue the probes saw has gone, and what the link carried
            // before it still stands for the line: climbing past it would
            // fill the router's hidden queue again, so it climbs back to
            // where it held, gently, as near a queue it met, and no further.
            None if self.rate < plateau * HOLD => {
                self.rate = (self.rate * CLIMB.powf(dt)).min(plateau * HOLD);
            }
            // A probe under way, or none due yet.
            Some(_) | None => {}
        }
        true
    }

    /// What the connection is known to carry: its base, or what it was
    /// proven to carry, whichever is more, less a tenth of the proof as the
    /// base is.
    fn connection(&self) -> Option<f64> {
        let proven = (self.proven > 0.0).then_some(self.proven * 0.9);
        match (self.base, proven) {
            (Some(base), Some(proven)) => Some(base.max(proven)),
            (base, proven) => base.or(proven),
        }
    }

    /// Keep what a clear spell proves the connection carries.
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
    ///
    /// `unseen` says the delay is the round trip through a VPN alone, a
    /// queue the link's probes don't meet.
    pub fn step(&mut self, at: f64, delay_ms: f64, carried: f64, ours: Option<Ours>, unseen: bool) {
        // A gap, while the connection was left unlimited or asleep, is one
        // moment, not a climb all through it.
        let dt = self.last_at.map_or(0.0, |last| (at - last).clamp(0.0, 1.0));
        self.last_at = Some(at);
        // What crossed the line is what CAKE sent, where the shaper
        // counted it: the link sends the home network's traffic too.
        let line = ours
            .and_then(|o| o.sent)
            .map_or(carried, |sent| carried.min(crate::rate::float(sent)));
        self.prove(at, delay_ms, line);
        self.reach(at, line);
        let crowded = delay_ms > LATE_MS
            && self
                .connection()
                .is_some_and(|connection| carried < connection * SLIVER);
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
        // A queue standing while nothing of ours waits to go, and the link
        // carries well under our rate, is the rest of the house's: cutting
        // frees nothing of it and starves this machine, so the rate holds.
        // A line that fell under our rate leaves our sockets waiting, and
        // is cut to as before.
        let theirs = ours.is_some_and(|ours| !ours.waiting) && carried < self.rate * FULL;
        if let (Some(_), Some(plateau)) = (self.probing_until, self.plateau)
            && delay_ms > LATE_MS
        {
            // A probe that raises a queue found no room past the hold, and
            // Joel's call waits in that queue: it goes back at once, rather
            // than once the queue has stood.
            self.probing_until = None;
            self.rate = plateau * HOLD;
            self.next_probe = at + PROBE_EVERY_S;
        } else if standing
            && flowing
            && !theirs
            && at - self.last_cut
                >= if unseen {
                    UNSEEN_DRAIN_S
                } else {
                    BETWEEN_CUTS_S
                }
        {
            // A queue the probes see is first come, first served, and the
            // delay says where it is better than any plateau, which is
            // kept only as where to climb back to once it has gone. A
            // queue only the VPN's round trip meets, standing while our
            // CAKE holds one and no probe runs, stands for a hold over a
            // router that buffers the VPN's flow, so the line is no more
            // than what got through, and the hold climbs back under it. A
            // queue the probes meet says nothing of the line: the air or
            // another machine took the moment, and what got through in it
            // would ratchet the plateau down with every cut.
            if self.probing_until.take().is_some() {
                self.next_probe = at + PROBE_EVERY_S;
            } else if unseen
                && ours.is_some_and(|ours| ours.queued)
                && let Some(plateau) = self.plateau
            {
                // The line is under the plateau only if, over the seconds
                // the queue built, it took less than the hold: the cut's
                // own second is already slowed by a cut before it, and a
                // burst at the hold says nothing of the line. Taken from
                // that second, bursts on 11 November ratcheted the
                // plateau from 11,371 to 5,992 in 130 s.
                let most = self
                    .carrying
                    .iter()
                    .map(|&(_, c)| c)
                    .fold(carried, f64::max);
                self.plateau = Some(plateau.min(most / HOLD).max(self.reached * UNSEEN_FLOOR));
            }
            self.elsewhere = None;
            // Only the line sets what got through while something of ours
            // waited to go; otherwise it is what was wanted. Filling a
            // rate says no more: a cut holds a flow just under what it
            // wants, which it then fills, and on 8 November that taught
            // the download a line of a few Mbit/s again and again. A
            // download can't hear its senders wait, so it learns its line
            // from clear spells alone.
            let filled = ours.is_some_and(|ours| ours.waiting);
            self.cut(at, delay_ms, carried, filled);
            if unseen {
                self.rate = self.rate.max(self.reached * UNSEEN_FLOOR);
            }
        } else if let Some(ours) = ours
            && !standing
            && self.own(at, dt, ours, carried)
        {
        } else if carried >= self.rate * FULL && delay_ms <= LATE_MS {
            // As TCP's slow start does: far from any queue it has met, a
            // rate climbs fast, since nothing says the connection is near,
            // and near the last one gently. A little delay short of a queue
            // doesn't stop it: the connection carried more with no more
            // than that.
            let far = self.ceiling.is_none_or(|(ceiling, met)| {
                self.rate < ceiling * FAR || self.rate > ceiling * PAST || at - met >= CEILING_S
            });
            self.rate *= if far { FAST_CLIMB } else { CLIMB }.powf(dt);
        } else if self.base.is_none()
            && carried < self.rate * FULL
            && !standing
            && at - self.last_cut >= CEILING_S
        {
            // With no line learned, a cut held what was wanted, and once
            // the line has likely moved there is nothing left to hold to:
            // the direction goes back to unlimited until a queue stands.
            // What an idle spell proved is no line: it proves the idle
            // rate.
            self.rate = f64::INFINITY;
        } else if let Some(base) = self.connection()
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

    fn cut(&mut self, at: f64, delay_ms: f64, carried: f64, filled: bool) {
        // Cut under a sliver, a queue this machine hardly feeds drains no
        // faster and the machine starves, so the reach is halved instead.
        // A connection that really fell that far is met in a few halvings;
        // a queue no cut drains is the guard's.
        let reach = self
            .connection()
            .map_or(self.rate, |connection| connection.min(self.rate));
        if reach.is_finite() && carried < reach * SLIVER {
            self.rate = (reach * 0.5).max(LEAST_KBIT);
            self.last_cut = at;
            return;
        }
        // With the rate well over what got through, and the line setting
        // it, the connection, not this machine, set what it carried, and
        // that is what the base learns. A download left unlimited carries
        // what it wanted: on 10 November one carrying 26 Mbit/s of a 370
        // Mbit/s line taught a base of 23.9.
        if filled && carried * 1.15 <= self.rate {
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
    /// nearly all that time while it carried a sliver of its connection, or
    /// held under half its base for a minute with no queue standing.
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
        let held_down = match self.connection() {
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
        self.rate = self.connection().unwrap_or(f64::INFINITY);
        self.plateau = None;
        self.probing_until = None;
        self.elsewhere = None;
        self.under_half_since = None;
        self.late_since = None;
        self.crowded.clear();
        self.last_at = None;
    }

    /// Start over as on arriving, unlimited until a queue stands, keeping
    /// what was learned of the connection, so a limit is never set without
    /// one.
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
    vpn_base: Option<f64>,
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
            vpn_base: None,
            last_at: None,
            answered_at: None,
            started_at: None,
            returns: 0,
            gave_up_at: None,
        }
    }

    /// A controller for a network met before, from what each direction
    /// learned of it.
    pub fn from_learned(up: Learned, down: Learned) -> Controller {
        Controller {
            up: Way::from_learned(up),
            down: Way::from_learned(down),
            ..Controller::new(None, None)
        }
    }

    /// The least a call's round trip through the VPN is taken to be, in
    /// ms, once one was heard.
    pub fn vpn_base(&self) -> Option<f64> {
        self.vpn_base
    }

    /// Whether the guard has given up, for `GIVEN_UP_S`.
    pub fn given_up(&self) -> bool {
        self.returns >= GUARD_RETURNS
    }

    /// The upload's queue in the router that only the VPN's flow waits in,
    /// in ms.
    /// It's what the VPN's round trip took over its least, past what the
    /// probes on the link found each way, while the VPN's CAKE held a
    /// queue: then the VPN sends as fast as it's let, and what it fills is
    /// the upload's. With our CAKE holding nothing, the VPN's flow is
    /// filled by what the shaper doesn't hold, and cutting drains none of
    /// it. With the link's CAKE holding one too, the VPN's flow waits in
    /// it, ours, which the link's own rate settles, so that wait comes
    /// off: it was a few ms in the trace on 11 November, beside a median
    /// 33 ms in the router, and counting nothing while the link's CAKE
    /// held any left that queue to grow.
    fn hidden(&mut self, seen: &Seen, dt: f64) -> f64 {
        let Some(vpn) = seen.vpn_ms else {
            return 0.0;
        };
        let queued = seen.up_ours.filter(|ours| ours.in_vpn || ours.vpn_queued);
        // The base follows the path up only while the VPN's CAKE holds
        // nothing: rising under load, it rose from 29 to 55 ms on 11
        // November, and took a standing queue for the path. A path that
        // moved under load is learned again once the guard lets go.
        let rise = if queued.is_some() {
            0.0
        } else {
            VPN_RISE_MS * dt
        };
        let base = self.vpn_base.map_or(vpn, |b| vpn.min(b + rise));
        self.vpn_base = Some(base);
        let Some(ours) = queued else {
            return 0.0;
        };
        let probes = seen.up_delay_ms.unwrap_or(0.0) + seen.down_delay_ms.unwrap_or(0.0);
        let link = crate::rate::float(ours.link_wait_ms);
        (vpn - base - probes - link - VPN_MARGIN_MS).max(0.0)
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
        let hidden = self.hidden(seen, dt);
        let up = seen.up_delay_ms.map(|d| (d - air).max(0.0));
        let down = seen.down_delay_ms.map(|d| (d - air).max(0.0));
        let idle_up = self.up.idle(seen.up_kbit);
        let idle_down = self.down.idle(seen.down_kbit);
        if let Some(delay) = up {
            let beside = if idle_down { down.unwrap_or(0.0) } else { 0.0 };
            let seen_delay = (delay - beside).max(0.0);
            let unseen = hidden > seen_delay;
            self.up.step(
                at,
                seen_delay.max(hidden),
                seen.up_kbit,
                seen.up_ours,
                unseen,
            );
        }
        if let Some(delay) = down {
            let beside = if idle_up { up.unwrap_or(0.0) } else { 0.0 };
            // The download's queue at home is the cable's, first come,
            // first served, which the delay finds.
            self.down
                .step(at, (delay - beside).max(0.0), seen.down_kbit, None, false);
        }
        for way in [&mut self.up, &mut self.down] {
            if way.misbehaving(at) {
                way.reset();
                self.returns += 1;
                // A queue no cut drains may be a VPN's path that moved,
                // which its base, held under load, would not follow.
                self.vpn_base = None;
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
            up_ours: s.up_ours,
            vpn_ms: s.vpn_ms,
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
    fn a_plateau_learned_before_holds_under_the_router_at_once() {
        let at = |held: &[(f64, Option<u64>)], s: f64| {
            held.iter().find(|&&(at, _)| at >= s).and_then(|&(_, l)| l)
        };
        let learned = Learned {
            base: None,
            plateau: Some(12_000.0),
        };
        let mut met = Controller::from_learned(learned, Learned::default());
        let held = flow_queued(&mut met, 0.0, 5.0, |_| 12_000.0);
        assert!(near(at(&held, 4.0), 11_400.0), "{:?}", at(&held, 4.0));
        let mut cold = Controller::new(None, None);
        let held = flow_queued(&mut cold, 0.0, 5.0, |_| 12_000.0);
        assert!(
            !near(at(&held, 4.0), 11_400.0),
            "a network never met found its plateau as soon, so this shows nothing"
        );
    }

    #[test]
    fn what_a_direction_learned_outlasts_a_stay_that_learned_less() {
        let old = Learned {
            base: Some(10_000.0),
            plateau: Some(12_000.0),
        };
        let replayed = Learned {
            base: Some(10_200.0),
            plateau: None,
        };
        let kept = replayed.over(old);
        assert_eq!(kept.plateau, Some(12_000.0));
        assert_eq!(kept.base, Some(10_200.0));
        assert!(
            !kept.moved(old),
            "a base a fiftieth off was taken for a move"
        );
        assert!(
            Learned::default().moved(old),
            "forgetting both wasn't a move"
        );
    }

    #[test]
    fn a_vpn_s_queue_is_ours_and_said_to_be_its() {
        let vpn = Ours::from_reads((0, 5), (4, 5), 0, true, (0, 0));
        assert_eq!(vpn.map(|o| (o.queued, o.in_vpn)), Some((true, true)));
        let link = Ours::from_reads((3, 5), (4, 5), 6, true, (90, 40));
        assert_eq!(link.map(|o| (o.queued, o.in_vpn)), Some((true, false)));
        assert_eq!(
            link.map(|o| (o.vpn_queued, o.link_wait_ms)),
            Some((true, 6)),
            "the VPN's queue stood beside the link's, which kept it 6 ms"
        );
        assert_eq!(link.map(|o| (o.resent, o.dropped)), Some((90, 40)));
        let none = Ours::from_reads((1, 5), (0, 0), 0, true, (0, 0));
        assert_eq!(none.map(|o| (o.queued, o.in_vpn)), Some((false, false)));
        assert_eq!(Ours::from_reads((0, 0), (5, 5), 0, true, (0, 0)), None);
    }

    #[test]
    fn a_replayed_queue_is_cut_with_cause_and_an_idle_connection_isnt_held_low() {
        let second = |t: u64, down_ms: f64, down_kbit: f64| replay::Second {
            t_s: crate::rate::float(t),
            up_kbit: 300.0,
            down_kbit,
            up_delay_ms: Some(0.0),
            down_delay_ms: Some(down_ms),
            gateway_ms: Some(3.0),
            up_ours: None,
            vpn_ms: None,
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
    fn a_queue_under_a_sliver_of_the_connection_doesnt_hold_the_connection_low() {
        // As the replay of 7 November found: a download carrying 100
        // Mbit/s with no queue, then a queue of 39 ms standing while it
        // carried 737 kbit/s, so the queue was another machine's or the
        // air's, then 15 Mbit/s with none. Cut to under the 737 and taught
        // it as the connection, the controller held the download under half
        // the connection for the next five minutes.
        let second = |t: u64, down_ms: f64, down_kbit: f64| replay::Second {
            t_s: crate::rate::float(t),
            up_kbit: 300.0,
            down_kbit,
            up_delay_ms: Some(0.0),
            down_delay_ms: Some(down_ms),
            gateway_ms: Some(3.0),
            up_ours: None,
            vpn_ms: None,
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
    fn another_machines_queue_breaking_now_and_then_goes_back_to_the_connection() {
        // As the replay of 7 November found: with the connection proven at
        // 90 Mbit/s, the download's own delay stood at 16 to 30 ms for three
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
    fn ten_minutes_after_giving_up_the_connection_is_shaped_again() {
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
    fn under_half_its_connection_a_full_direction_climbs_through_a_little_delay() {
        // As the replay found of uploads on Wi-Fi: cut, then full with its
        // own delay between 5 and 15 ms, a rate under half the connection
        // was held there for minutes while the connection carried more.
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

    /// A wide connection from `from` to `to` carrying the whole of whatever
    /// the download is held to, with nothing waiting.
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
    fn a_standing_queue_cuts_a_tenth_under_what_got_through() {
        let mut c = Controller::new(None, None);
        let limits = run(&mut c, 0.0, 1.15, queued(40.0, 300_000.0));
        // 300,000 × 0.9 × (1 − 0.04).
        assert_eq!(limits.down, Some(259_200));
        assert_eq!(limits.up, None, "the upload's own delay never rose");
    }

    #[test]
    fn an_unlimited_download_s_cut_learns_no_line() {
        // On 10 November a download carrying 26 Mbit/s of a 370 Mbit/s
        // line met a spike, and the base it learned held every later
        // stay's download to 23.9: it carried what it wanted, not what the
        // line could.
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.15, queued(40.0, 26_000.0));
        assert!(c.down.limit().is_some(), "the queue is still cut");
        assert_eq!(c.down.base, None, "learned {:?}", c.down.base);
    }

    #[test]
    fn a_download_filling_what_its_own_cut_held_it_to_learns_no_line() {
        // As on 8 November: cut just under what it wanted, a flow fills
        // the rate, and the next spike would teach that as the line.
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.15, queued(40.0, 26_000.0));
        let held = c.down.limit().map(crate::rate::float).expect("cut");
        run(&mut c, 1.15, 3.3, queued(40.0, held));
        assert!(
            c.down.limit() < Some(23_000),
            "cut again: {:?}",
            c.down.limit()
        );
        assert_eq!(c.down.base, None, "learned {:?}", c.down.base);
    }

    #[test]
    fn what_a_clear_spell_proved_is_kept_over_a_lower_base() {
        let mut c = Controller::from_learned(
            Learned::default(),
            Learned {
                base: Some(23_907.0),
                plateau: None,
            },
        );
        let mut at = 0.0;
        while at < 30.0 {
            let down_kbit = c
                .down
                .limit()
                .map_or(200_000.0, |r| crate::rate::float(r).min(200_000.0));
            c.step(&Seen {
                at_s: at,
                ..quiet(300.0, down_kbit)(at)
            });
            at += 0.1;
        }
        let kept = c.down.learned().base.expect("a base");
        assert!(
            kept >= 170_000.0,
            "kept {kept}, after carrying 200,000 clear"
        );
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
            way.step(at, 1.0, 0.0, None, false);
            at += 0.1;
        }
        let rate = way.limit().expect("held");
        assert!(
            (120_000..=125_000).contains(&rate),
            "1% a second toward base: {rate}"
        );
        while at < 60.0 {
            way.step(at, 1.0, 0.0, None, false);
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
            way.cut(at, 20.0, carried * 1_000.0, true);
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
    fn no_timestamps_for_ten_seconds_leaves_the_connection_unlimited() {
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
        // air had cleared and the connection carried twice that. A line
        // learned keeps it held while idle.
        let mut c = Controller::new(None, Some(400_000.0));
        run(&mut c, 0.0, 1.2, queued(40.0, 100_000.0));
        run(&mut c, 1.2, 32.0, quiet(300.0, 1_000.0));
        let held = c.down.limit().map(crate::rate::float).expect("held");
        let limits = wide(&mut c, 32.0, 37.0);
        let climbed = limits.down.map(crate::rate::float).expect("held") / held;
        // Five seconds at 1.25 a second is 3.05.
        assert!(climbed > 2.9, "climbed {climbed}");
    }

    #[test]
    fn with_no_line_learned_a_cut_is_let_go_half_a_minute_on() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.2, queued(40.0, 26_000.0));
        let early = run(&mut c, 1.2, 30.0, quiet(300.0, 1_000.0));
        assert!(early.down.is_some(), "held until the line has likely moved");
        let limits = run(&mut c, 30.0, 32.0, quiet(300.0, 1_000.0));
        assert_eq!(limits.down, None, "unlimited until a queue stands");
    }

    #[test]
    fn a_connection_filling_its_buffer_is_cut_each_second_without_the_guard() {
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

    // On 11 November a transfer across the home network sent 300 to 757
    // Mbit/s on the link, which never reached the 12 Mbit/s line, and a
    // clear spell took it for the line: the upload kept a base of 282
    // Mbit/s.
    #[test]
    fn a_clear_spell_proves_only_what_cake_sent() {
        let mut way = Way::from_learned(Learned {
            base: Some(11_148.0),
            plateau: None,
        });
        let ours = Ours {
            sent: Some(11_000),
            ..Ours::default()
        };
        for i in 0..100_u32 {
            way.step(f64::from(i) * 0.1, 0.0, 300_000.0, Some(ours), false);
        }
        let base = way.learned().base.expect("a base");
        assert!(base < 12_000.0, "proved {base} of a home transfer");
        let plateau = way.reached;
        assert!(plateau < 12_000.0, "reached {plateau} of a home transfer");
    }

    #[test]
    fn a_base_or_plateau_kept_before_is_learned_again() {
        let old = r#"{"base": 282090.8, "line": null, "plateau": 1423.7}"#;
        let learned: Learned = serde_json::from_str(old).expect("a learned");
        assert_eq!(learned.base, None, "kept a base a home transfer taught");
        assert_eq!(learned.plateau, None, "kept a plateau hasty cuts taught");
        let kept = Learned {
            base: Some(11_148.0),
            plateau: Some(10_759.0),
        };
        let again: Learned =
            serde_json::from_str(&serde_json::to_string(&kept).expect("json")).expect("back");
        assert_eq!(again, kept, "a base kept now is read back");
    }

    #[test]
    fn a_start_lets_go_of_a_plateau_under_half_its_base() {
        let way = Way::from_learned(Learned {
            base: Some(11_148.0),
            plateau: Some(2_620.0),
        });
        assert_eq!(way.plateau, None, "kept a plateau the guard lets go of");
        assert_eq!(way.limit(), Some(11_148));
        let way = Way::from_learned(Learned {
            base: Some(16_373.0),
            plateau: Some(10_020.0),
        });
        assert_eq!(way.limit(), Some(9_519), "a plateau over half is kept");
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
    fn three_returns_and_the_connection_is_left_unlimited() {
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

    /// An upload behind a router that queues by flow, as the eero does,
    /// from `from` to `to`: the router carries `line(at)` and its queue,
    /// shared flow by flow, delays no probe. Held under the line, the
    /// queue is our CAKE's and the link carries the rate; held over it, the
    /// link carries the line and our CAKE holds nothing.
    fn flow_queued(
        c: &mut Controller,
        from: f64,
        to: f64,
        line: impl Fn(f64) -> f64,
    ) -> Vec<(f64, Option<u64>)> {
        let mut held = Vec::new();
        let mut at = from;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let line = line(at);
            let limits = c.step(&Seen {
                at_s: at,
                up_kbit: rate.min(line),
                up_ours: Some(Ours {
                    queued: rate < line,
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            held.push((at, limits.up));
            at += 0.1;
        }
        held
    }

    fn near(limit: Option<u64>, kbit: f64) -> bool {
        limit.is_some_and(|l| (crate::rate::float(l) - kbit).abs() <= kbit * 0.02)
    }

    #[test]
    fn a_queue_a_flow_queued_router_hides_is_taken_back_under_it() {
        let mut c = Controller::new(None, None);
        let limits = run_flow_queued(&mut c, 0.0, 10.0, 12_000.0);
        assert!(near(limits, 11_400.0), "held under the router: {limits:?}");
    }

    fn run_flow_queued(c: &mut Controller, from: f64, to: f64, line: f64) -> Option<u64> {
        flow_queued(c, from, to, |_| line)
            .last()
            .and_then(|&(_, l)| l)
    }

    #[test]
    fn under_the_router_the_rate_holds_and_a_probe_over_it_comes_back() {
        let mut c = Controller::new(None, None);
        let held = flow_queued(&mut c, 0.0, 120.0, |_| 12_000.0);
        let over: Vec<f64> = held
            .iter()
            .filter(|&&(at, l)| at > 10.0 && !near(l, 11_400.0))
            .map(|&(at, _)| at)
            .collect();
        // A probe each minute, over in a few seconds.
        assert!(!over.is_empty(), "it never looked for more room");
        assert!(over.len() <= 2 * 60, "over the router too long: {over:?}");
        let last = held.last().and_then(|&(_, l)| l);
        assert!(
            near(last, 11_400.0),
            "didn't come back under the router: {last:?}"
        );
    }

    #[test]
    fn a_probe_that_finds_more_room_climbs_to_it() {
        let mut c = Controller::new(None, None);
        let held = flow_queued(&mut c, 0.0, 150.0, |at| {
            if at < 30.0 { 12_000.0 } else { 20_000.0 }
        });
        assert!(
            near(held.last().and_then(|&(_, l)| l), 19_000.0),
            "{:?}",
            held.last()
        );
    }

    /// Steps a controller whose link carries everything through a VPN, as
    /// WARP does: the VPN sends at its share of the link's rate, the link
    /// carries that up to the line, and the queue is the VPN's, ours, only
    /// while the VPN's rate is under the line. Answers the VPN's median
    /// rate over the last minute, probes and all.
    fn behind_a_vpn(c: &mut Controller, to: f64, line: impl Fn(f64) -> f64) -> f64 {
        let mut sent = Vec::new();
        let mut at = 0.0;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let vpn = rate * crate::qdisc::VPN_SHARE;
            if at >= to - 60.0 {
                sent.push(vpn);
            }
            let line = line(at);
            c.step(&Seen {
                at_s: at,
                up_kbit: vpn.min(line),
                up_ours: Some(Ours {
                    queued: vpn < line,
                    in_vpn: vpn < line,
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        sent.sort_by(f64::total_cmp);
        sent[sent.len() / 2]
    }

    // Held under the line, the link leaves what goes round the VPN its
    // room, so the VPN sends its share of the hold: about 0.85 of the line.
    #[test]
    fn behind_a_vpn_its_queue_is_held_under_the_line_and_no_lower() {
        let mut c = Controller::new(None, None);
        let vpn = behind_a_vpn(&mut c, 150.0, |_| 12_000.0);
        assert!(
            (9_800.0..=11_000.0).contains(&vpn),
            "the VPN sent at {vpn} on a 12000 line"
        );
    }

    #[test]
    fn behind_a_vpn_a_probe_that_finds_more_room_climbs_to_it() {
        let mut c = Controller::new(None, None);
        let vpn = behind_a_vpn(
            &mut c,
            300.0,
            |at| if at < 30.0 { 12_000.0 } else { 20_000.0 },
        );
        assert!(
            (16_000.0..=17_500.0).contains(&vpn),
            "the VPN sent at {vpn} on a 20000 line"
        );
    }

    /// Steps a controller as `behind_a_vpn` does, on a 12000 line that
    /// the air takes a third of for two seconds in every twenty, as the
    /// Wi-Fi does when a neighbour talks: the probes meet a queue, and
    /// what the link carries falls. Answers the VPN's median rate over
    /// the last minute.
    fn behind_a_vpn_on_busy_air(c: &mut Controller, to: f64) -> f64 {
        let mut sent = Vec::new();
        let mut at = 0.0;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let vpn = rate * crate::qdisc::VPN_SHARE;
            if at >= to - 60.0 {
                sent.push(vpn);
            }
            let busy = at % 20.0 >= 18.0;
            let line = if busy { 8_000.0 } else { 12_000.0 };
            c.step(&Seen {
                at_s: at,
                up_kbit: vpn.min(line),
                up_delay_ms: Some(if busy { 40.0 } else { 1.0 }),
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(30.0 + if busy { 40.0 } else { 0.0 }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        replay::median(sent.into_iter()).unwrap_or(0.0)
    }

    // A queue the probes see while the air is busy is cut, as any is, but
    // what got through in that moment says nothing of the line, so the
    // plateau the probes found stays, and the hold climbs back to it. Each
    // cut lowering the plateau to what got through ratcheted the VPN down
    // to the floor in a minute and a half of the live proof.
    #[test]
    fn a_busy_moment_on_the_air_doesn_t_lower_the_plateau() {
        let learned = Learned {
            base: None,
            plateau: Some(11_843.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let vpn = behind_a_vpn_on_busy_air(&mut c, 300.0);
        assert!(
            vpn >= 9_000.0,
            "the VPN sent at {vpn} on a 12000 line busy a tenth of the time"
        );
    }

    /// What the VPN sent and how long the router's queue for it was, in
    /// ms, over the last minute behind a buffering router: their middles,
    /// and the longest queue.
    #[derive(Debug)]
    struct Buffered {
        sent: f64,
        queue_ms: f64,
        longest_ms: f64,
    }

    /// Steps a controller whose link carries everything through a VPN,
    /// behind a router that keeps the VPN's one flow in a queue of its own
    /// and buffers up to five seconds of it, as the eero does: the link
    /// carries everything the VPN sends until the buffer is full, the
    /// probes on the link meet no queue, and only a round trip through the
    /// VPN waits in it. The agents offer far more than the line, so our
    /// CAKE holds a queue whenever the VPN is held under what they offer.
    fn behind_a_buffering_router(c: &mut Controller, to: f64, line: f64) -> Buffered {
        behind_a_buffering_router_beside(c, to, line, None)
    }

    /// As `behind_a_buffering_router`, with the link's CAKE holding a
    /// queue of `link_ms` too, when it's given, as it did in most seconds
    /// of the trace on 11 November: the link's own wait, a few ms, which
    /// the VPN's flow waits in before it reaches the router. Then the link
    /// carries its whole rate, the VPN's flow and what goes round it, and
    /// the router queues all of it in the VPN's flow, as there the VPN
    /// carries nearly everything.
    fn behind_a_buffering_router_beside(
        c: &mut Controller,
        to: f64,
        line: f64,
        link_ms: Option<f64>,
    ) -> Buffered {
        const OFFERED: f64 = 40_000.0;
        let buffer = line * 5.0;
        let (mut backlog, mut sent, mut queue) = (0.0_f64, Vec::new(), Vec::new());
        let mut at = 0.0;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let vpn = (rate * crate::qdisc::VPN_SHARE).min(OFFERED);
            let carried = if link_ms.is_some() {
                rate.min(OFFERED)
            } else {
                vpn
            };
            let filled = backlog + (carried - line) * 0.1;
            // What the full buffer can't take is dropped, and TCP sends it
            // again.
            let overflow = (filled - buffer).max(0.0) / 0.1;
            backlog = filled.clamp(0.0, buffer);
            let queue_ms = backlog / line * 1_000.0;
            if at >= to - 60.0 {
                sent.push(carried);
                queue.push(queue_ms);
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive rate far under u64's range"
            )]
            let (resent, link_wait_ms) = (overflow as u64, link_ms.map_or(0, |ms| ms as u64));
            let vpn_queued = vpn < OFFERED;
            c.step(&Seen {
                at_s: at,
                up_kbit: carried,
                up_ours: Some(Ours {
                    queued: vpn_queued || link_ms.is_some(),
                    in_vpn: vpn_queued && link_ms.is_none(),
                    vpn_queued,
                    link_wait_ms,
                    waiting: true,
                    resent,
                    ..Ours::default()
                }),
                vpn_ms: Some(30.0 + link_ms.unwrap_or(0.0) + queue_ms),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        let longest_ms = queue.iter().copied().fold(0.0, f64::max);
        let middle = |values: Vec<f64>| replay::median(values.into_iter()).unwrap_or(0.0);
        Buffered {
            sent: middle(sent),
            queue_ms: middle(queue),
            longest_ms,
        }
    }

    // A plateau over the line, remembered from a faster hour or kept from
    // a probe while the router's buffer took in the difference, holds the
    // VPN over a line the probes can't see: the call's round trip through
    // the VPN can, and the VPN is brought under the line, and a probe
    // past it comes back before Joel's call waits long.
    #[test]
    fn a_queue_only_the_vpn_s_round_trip_meets_is_cut_and_kept_under() {
        let learned = Learned {
            base: None,
            plateau: Some(12_500.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let seen = behind_a_buffering_router(&mut c, 180.0, 10_000.0);
        assert!(
            (8_000.0..=10_000.0).contains(&seen.sent),
            "the VPN sent at {} on a 10000 line: {seen:?}",
            seen.sent
        );
        assert!(
            seen.queue_ms < 15.0,
            "Joel's call waited {} ms in the router: {seen:?}",
            seen.queue_ms
        );
        assert!(
            seen.longest_ms < 250.0,
            "a probe kept Joel's call waiting {} ms: {seen:?}",
            seen.longest_ms
        );
    }

    // The link's CAKE holding a few ms of its own doesn't hide the router's
    // queue for the VPN's flow: in the trace on 11 November it did in 99
    // of 119 seconds, the VPN's round trip ran a median 33 ms over its
    // least, and the controller counted none of it.
    #[test]
    fn the_vpn_s_queue_in_the_router_is_cut_while_the_link_s_cake_holds_one_too() {
        let learned = Learned {
            base: None,
            plateau: Some(12_500.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let seen = behind_a_buffering_router_beside(&mut c, 180.0, 10_000.0, Some(5.0));
        assert!(
            (8_000.0..=10_000.0).contains(&seen.sent),
            "the VPN sent at {} on a 10000 line: {seen:?}",
            seen.sent
        );
        assert!(
            seen.queue_ms < 15.0,
            "Joel's call waited {} ms in the router: {seen:?}",
            seen.queue_ms
        );
    }

    // The link's own wait is ours, and the link's rate settles it: cutting
    // for it would only fill our CAKE further.
    #[test]
    fn the_link_s_own_wait_isn_t_taken_for_the_router_s() {
        let mut c = Controller::new(None, None);
        flow_queued(&mut c, 0.0, 20.0, |_| 12_000.0);
        let before = c.up.limit();
        let limits = run(&mut c, 20.0, 30.0, |at| Seen {
            up_ours: Some(Ours {
                queued: true,
                vpn_queued: true,
                link_wait_ms: 40,
                waiting: true,
                ..Ours::default()
            }),
            vpn_ms: Some(if at < 21.0 { 30.0 } else { 70.0 }),
            ..quiet(11_400.0, 300.0)(at)
        });
        assert!(
            near(limits.up, 11_400.0),
            "the upload was cut for the link's own wait: {limits:?}, from {before:?}"
        );
    }

    // With our CAKE holding nothing, a queue for the VPN's flow is filled
    // by what the VPN wasn't held back from, such as the download's
    // answers, and cutting the upload drains none of it.
    #[test]
    fn a_vpn_s_round_trip_rising_while_our_cake_holds_nothing_isnt_cut() {
        let mut c = Controller::new(None, None);
        flow_queued(&mut c, 0.0, 20.0, |_| 12_000.0);
        let limits = run(&mut c, 20.0, 30.0, |at| Seen {
            up_ours: Some(Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            }),
            vpn_ms: Some(if at < 21.0 { 30.0 } else { 150.0 }),
            ..quiet(11_400.0, 300.0)(at)
        });
        assert!(near(limits.up, 11_400.0), "the upload was cut: {limits:?}");
    }

    // As in the trace on 11 November: held over a router that queues the
    // VPN's flow, a burst fills it now and then for a few seconds. Each
    // cut lowered the plateau to what got through in the cut's second,
    // from 11,371 to 5,992 in 130 s, and the stay saved 2,620.
    #[test]
    fn a_burst_in_the_router_doesn_t_ratchet_the_plateau_down() {
        let mut c = Controller::from_learned(
            Learned {
                base: Some(12_000.0),
                plateau: Some(10_000.0),
            },
            Learned::default(),
        );
        let mut at = 0.0;
        while at < 300.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let burst = (20.0..23.0).contains(&(at % 30.0));
            c.step(&Seen {
                at_s: at,
                up_kbit: rate.min(10_000.0),
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    vpn_queued: true,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(if burst { 150.0 } else { 30.0 }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        let plateau = c.up.plateau.expect("a plateau");
        assert!(plateau >= 9_900.0, "ratcheted to {plateau}");
    }

    // As at 17:50 on 11 November: held over a router that buffers the
    // VPN's flow, the round trip through the VPN is heard a round trip and
    // a second late. Cut again each second on what was heard before the
    // last cut showed, the upload went from 10,759 to 1,744 in 3 s, then
    // to the floor, and the stay saved a plateau of 1,424.
    #[test]
    fn a_queue_only_the_vpn_meets_is_let_drain_before_another_cut() {
        const LINE: f64 = 10_000.0;
        let mut c = Controller::from_learned(
            Learned {
                base: None,
                plateau: Some(11_325.0),
            },
            Learned::default(),
        );
        let (mut backlog, mut heard, mut least) = (0.0_f64, VecDeque::new(), f64::INFINITY);
        let mut at = 0.0;
        while at < 120.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let carried = rate.min(40_000.0);
            backlog = (backlog + (carried - LINE) * 0.1).clamp(0.0, LINE * 5.0);
            heard.push_back(30.0 + backlog / LINE * 1_000.0);
            let vpn_ms = if heard.len() > 15 {
                heard.pop_front()
            } else {
                None
            };
            c.step(&Seen {
                at_s: at,
                up_kbit: carried,
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    vpn_queued: true,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms,
                ..quiet(0.0, 300.0)(at)
            });
            if at >= 10.0 {
                least = least.min(rate);
            }
            at += 0.1;
        }
        assert!(
            least >= LINE * 0.5,
            "cut to {least} on a {LINE} line before its queue could drain"
        );
        let plateau = c.up.plateau.expect("a plateau");
        assert!(plateau >= LINE * 0.5, "kept a plateau of {plateau}");
    }

    /// A second over a router behind the VPN, its CAKE holding a queue or
    /// not, and the VPN's round trip.
    fn through_the_vpn(vpn_queued: bool, vpn_ms: f64) -> impl Fn(f64) -> Seen {
        move |at| Seen {
            up_ours: Some(Ours {
                queued: vpn_queued,
                in_vpn: vpn_queued,
                vpn_queued,
                waiting: true,
                ..Ours::default()
            }),
            vpn_ms: Some(vpn_ms),
            ..quiet(11_400.0, 300.0)(at)
        }
    }

    // In the same trace the base rose a millisecond a second under load,
    // from 29 to 55, though the least heard kept coming back to 29: a
    // standing queue of 20 ms was counted as the path.
    #[test]
    fn the_vpn_s_base_rises_only_while_its_cake_holds_nothing() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.0, through_the_vpn(true, 30.0));
        run(&mut c, 1.0, 21.0, through_the_vpn(true, 50.0));
        assert_eq!(
            c.vpn_base().map(f64::round),
            Some(30.0),
            "a standing queue was taken for the path"
        );
        run(&mut c, 21.0, 41.0, through_the_vpn(false, 50.0));
        assert_eq!(
            c.vpn_base().map(f64::round),
            Some(50.0),
            "the path itself moved"
        );
    }

    #[test]
    fn a_vpn_path_that_moved_is_learned_again_once_the_guard_lets_go() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.0, through_the_vpn(true, 30.0));
        run(&mut c, 1.0, 80.0, through_the_vpn(true, 90.0));
        assert_eq!(c.vpn_base().map(f64::round), Some(90.0));
    }

    /// Steps a controller whose link carries many uploads, enough that our
    /// CAKE always holds a queue, behind a router that queues flow by flow
    /// and drops what it can't carry: the link carries the rate, and TCP
    /// sends again what the router dropped, and `cake_drops` of the rate
    /// CAKE dropped itself. Answers the rate held at the end.
    fn many_flows(c: &mut Controller, to: f64, line: f64, cake_drops: f64) -> Option<u64> {
        let mut at = 0.0;
        let mut limits = Limits {
            up: None,
            down: None,
        };
        while at < to {
            let rate = c.up.limit().map_or(line * 2.0, crate::rate::float);
            let dropped = rate * cake_drops;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive rate far under u64's range"
            )]
            let kbit = |k: f64| k as u64;
            limits = c.step(&Seen {
                at_s: at,
                up_kbit: rate,
                up_ours: Some(Ours {
                    queued: true,
                    waiting: true,
                    resent: kbit((rate - line).max(0.0) + dropped),
                    dropped: kbit(dropped),
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        limits.up
    }

    // A plateau learned while the router's buffer still took more than its
    // line, at a flood's start, or remembered from a faster day, holds the
    // link over the line, where the router drops what it can't carry while
    // our CAKE queues: what TCP sends again past CAKE's own drops says so.
    #[test]
    fn a_plateau_over_the_line_is_brought_under_it_by_what_the_router_drops() {
        let learned = Learned {
            base: None,
            plateau: Some(10_300.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let held = many_flows(&mut c, 10.0, 8_000.0, 0.03);
        assert!(near(held, 7_600.0), "held over the line: {held:?}");
    }

    #[test]
    fn what_our_own_cake_drops_is_no_router_s_and_moves_no_hold() {
        let learned = Learned {
            base: None,
            plateau: Some(8_000.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let held = many_flows(&mut c, 50.0, 8_000.0, 0.08);
        assert!(near(held, 7_600.0), "cut for CAKE's own drops: {held:?}");
    }

    #[test]
    fn a_line_falling_under_what_it_held_is_followed_down() {
        let mut c = Controller::new(None, None);
        let held = flow_queued(&mut c, 0.0, 40.0, |at| {
            if at < 30.0 { 12_000.0 } else { 9_000.0 }
        });
        assert!(
            near(held.last().and_then(|&(_, l)| l), 8_550.0),
            "{:?}",
            held.last()
        );
    }

    #[test]
    fn what_a_router_drops_and_tcp_sends_again_isnt_the_line() {
        // A router sharing 12 Mbit/s flow by flow drops what it can't
        // carry, and TCP sends it again: held over the line, the link
        // carries a quarter more than the line, and that quarter is sent
        // twice.
        let mut c = Controller::new(None, None);
        let mut at = 0.0;
        let mut last = None;
        while at < 20.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let line = 12_000.0;
            let carried = rate.min(line * 1.25);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a few thousand kbit/s, never negative"
            )]
            let resent = (carried - line).max(0.0) as u64;
            last = c
                .step(&Seen {
                    at_s: at,
                    up_kbit: carried,
                    up_ours: Some(Ours {
                        queued: rate < line,
                        waiting: true,
                        resent,
                        ..Ours::default()
                    }),
                    ..quiet(0.0, 300.0)(at)
                })
                .up;
            at += 0.1;
        }
        assert!(
            near(last, 11_400.0),
            "held over the router's line: {last:?}"
        );
    }

    #[test]
    fn a_delay_cut_while_held_climbs_back_to_the_hold_and_no_further() {
        let mut c = Controller::new(None, None);
        flow_queued(&mut c, 0.0, 20.0, |_| 12_000.0);
        // Two seconds of a queue the probes see, as the air raises one.
        run(&mut c, 20.0, 22.0, |_| Seen {
            up_delay_ms: Some(60.0),
            up_ours: Some(Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            }),
            ..quiet(12_000.0, 300.0)(0.0)
        });
        let held = flow_queued(&mut c, 22.0, 50.0, |_| 12_000.0);
        let most = held.iter().filter_map(|&(_, l)| l).max();
        assert!(
            most.is_some_and(|m| crate::rate::float(m) <= 11_400.0 * 1.02),
            "climbed past the router's line again: {most:?}"
        );
        let last = held.last().and_then(|&(_, l)| l);
        assert!(
            near(last, 11_400.0),
            "didn't come back to the hold: {last:?}"
        );
    }

    #[test]
    fn one_slow_far_server_doesnt_hold_the_upload_down() {
        let mut c = Controller::new(None, None);
        // The connection carried 12 Mbit/s once, with nothing waiting;
        // then a single upload to a server that takes 2.
        run(&mut c, 0.0, 10.0, |_| Seen {
            up_ours: Some(Ours::default()),
            ..quiet(12_000.0, 300.0)(0.0)
        });
        let limits = run(&mut c, 10.0, 30.0, |_| Seen {
            up_ours: Some(Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            }),
            ..quiet(2_000.0, 300.0)(0.0)
        });
        assert_eq!(limits.up, None);
    }

    /// The upload held under a flow-queued router at 12 Mbit/s, then a
    /// queue standing while it carries 3 Mbit/s, with data of ours waiting
    /// or not: the rate it holds after ten seconds of it.
    fn under_a_queue_carrying_little(waiting: bool) -> Option<u64> {
        let mut c = Controller::new(None, None);
        flow_queued(&mut c, 0.0, 20.0, |_| 12_000.0);
        run(&mut c, 20.0, 30.0, |_| Seen {
            up_delay_ms: Some(60.0),
            up_ours: Some(Ours {
                queued: false,
                waiting,
                ..Ours::default()
            }),
            ..quiet(3_000.0, 300.0)(0.0)
        })
        .up
    }

    #[test]
    fn a_queue_the_house_raised_while_nothing_of_ours_waits_isnt_cut() {
        let held = under_a_queue_carrying_little(false);
        assert!(near(held, 11_400.0), "cut for the house's queue: {held:?}");
    }

    #[test]
    fn a_queue_standing_while_ours_waits_is_cut() {
        let held = under_a_queue_carrying_little(true);
        assert!(
            held.is_some_and(|l| l < 11_000),
            "held over a queue of ours: {held:?}"
        );
    }

    #[test]
    fn nothing_waiting_is_no_hidden_queue() {
        let mut c = Controller::new(None, None);
        let limits = run(&mut c, 0.0, 30.0, |_| Seen {
            up_ours: Some(Ours::default()),
            ..quiet(12_000.0, 300.0)(0.0)
        });
        assert_eq!(limits.up, None);
    }
}
