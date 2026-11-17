//! The shaper's rebuilt controller: each direction's rate from that
//! direction's own delay, as ICMP timestamps measure it, rather than from
//! round trips, which mix both directions and the air.
//!
//! A direction is cut once its own delay has stood over `LATE_MS` for a
//! second while it carries a flow: to a tenth under what it carried, and
//! more the deeper the queue; carrying a sliver of what it could, it's
//! halved instead, since the queue is mostly another machine's. A
//! download held under what arrives whose queue still doesn't drain goes
//! straight in for a while, since its cut only queued what arrived here
//! too. It climbs
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
//!
//! A network whose line is declared, its anchor, starts at the anchor's
//! hold instead, since what it learned may be the cuts' own doing. There a
//! wait only the VPN's round trip meets is cut for only while the link
//! carries near that hold: under it, the line has room, the router queues
//! nothing, and the wait is the VPN's own, said in the why line and left
//! alone.

use crate::why::Why;
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
/// The lowest ceiling a trial may set the upload under, in kbit/s.
/// A ceiling under it is refused rather than raised to it, since it's
/// most likely written in Mb/s, and one this low starves agents' lane.
const MOST_KBIT_LEAST: f64 = 4_000.0;
/// A direction carrying under this share of what it could carry, its rate
/// or its connection, when its queue stands carries a sliver: the queue is
/// mostly another machine's or the air's, so a cut halves what it could
/// carry, and teaches neither the base nor where the queue stood.
const SLIVER: f64 = 0.25;
/// A queue standing this share of `FUTILE_S` while the upload carries a
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
/// How long a start counts the plateau it learned before as reached: the
/// seconds every flow takes to start again, after which a line that fell
/// under what was learned is found from what the stay carries.
const STARTING_S: f64 = 30.0;
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
pub(crate) const VPN_MARGIN_MS: f64 = 5.0;
/// How long back that round trip's least is taken from. A least that rose
/// a millisecond a second whenever the VPN's CAKE held nothing went from
/// 25 to 62 ms on 11 November, under a queue of 100 to 350 ms standing for
/// minutes, and counted none of it; a path that moved is learned within
/// this, or at once when the guard lets go.
const VPN_LEAST_S: f64 = 300.0;
/// How many cuts of a run for a queue only the VPN's round trip meets
/// must find it back where the run's first found it, less the round
/// trip's own jitter, before it's taken to be the VPN's far path. A queue
/// the router buffers for the VPN's flow drains under a cut, cut deeper
/// the longer it is, and stays down: in `behind_a_buffering_router_beside`
/// from 119 ms to 15 in one, then held there by cuts that each find it
/// no lower. The far path's delay is no lower for a cut, and its jitter
/// keeps coming back to where it was.
pub(crate) const FAR_CUTS: u32 = 3;
/// How long the far path's delay then goes unheeded, at first and at
/// most: it doubles each time it's found again, and a cut that drains a
/// queue brings it back to the first.
pub(crate) const FAR_S: f64 = VPN_LEAST_S;
pub(crate) const FAR_MOST_S: f64 = 1_800.0;
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
pub(crate) const ELSEWHERE_S: f64 = 2.0;
/// How long a download carrying a sliver of its connection may be held
/// under what arrives, with its queue standing, before the queue is taken
/// to be another's, beyond twice how deep the queue then was: cut under
/// the line, a queue of ours drains faster than it rose.
const DRAIN_S: f64 = 2.0;
/// How long a download that found its queue another's leaves that queue
/// alone, before trying a cut again in case the line itself fell.
const ANOTHERS_S: f64 = 30.0;
/// How far a held download's queue must move past the least or the most
/// it has been, since it was first held, to count as moving.
const MOVED: f64 = 1.25;
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
/// The share of a network's declared upload rate, its anchor, that the
/// guard sends the upload back to: within the 85% to 95% of a cable
/// modem's rate that `LibreQoS` holds one at.
pub const ANCHOR_HOLD: f64 = 0.9;
/// An upload held under this share of its anchor for `ANCHOR_LOW_S`, with
/// no cut in that time that drained the queue it was made for, goes back
/// to the anchor's hold. On 15 November it sat at a third of a 12 Mbit/s
/// line for two hours, behind cuts for a delay no cut here reached.
pub const ANCHOR_LOW: f64 = 0.8;
const ANCHOR_LOW_S: f64 = 60.0;
/// How long after a cut the delay it was made for has to fall under a
/// queue's for the cut to count as having drained it.
const DRAINED_S: f64 = UNSEEN_DRAIN_S;
/// On a network with an anchor, the share of the anchor's hold the link
/// must carry for a queue past our CAKE to be the router's: a router holds
/// back nothing while the line has room. At 02:05 on 16 November the
/// VPN's own sending dipped under its CAKE's rate while the link carried
/// 5.6 of 12 Mbit/s, the reflectors 5 to 9 ms, and each 2 s dip was taken
/// for the line, holding the upload at 4.
const NEAR_THE_HOLD: f64 = 0.9;

/// How the upload tells a queue and finds its line, which a trial may set
/// in place of the constants above to learn how near the line it may hold.
/// Each is kept within bounds that leave Joel's delay short: past 30 ms a
/// standing queue is one a call hears, and a hold at the plateau leaves
/// CAKE no queue to order.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tuning {
    /// The delay, in ms, past which a queue stands.
    pub late_ms: f64,
    /// How long it must stand, in seconds, before a cut.
    pub standing_s: f64,
    /// What share of the plateau the upload is held to.
    pub hold: f64,
    /// How far over the plateau a probe looks.
    pub probe_up: f64,
    /// The most, in kbit/s, the upload is given, whatever it learns, or
    /// none. Held under a second shaper past ours, it learns whether the
    /// loss past our queues is that shaper's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_kbit: Option<f64>,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            late_ms: LATE_MS,
            standing_s: STANDING_S,
            hold: HOLD,
            probe_up: PROBE_UP,
            most_kbit: None,
        }
    }
}

impl Tuning {
    /// This, each value held within its bounds, and one that is no number
    /// at all taken as its default.
    #[must_use]
    pub fn bounded(self) -> Tuning {
        let today = Tuning::default();
        let within = |value: f64, least: f64, most: f64, default: f64| {
            if value.is_nan() {
                default
            } else {
                value.clamp(least, most)
            }
        };
        Tuning {
            late_ms: within(self.late_ms, 10.0, 30.0, today.late_ms),
            standing_s: within(self.standing_s, 1.0, 3.0, today.standing_s),
            hold: within(self.hold, 0.9, 1.0, today.hold),
            probe_up: within(self.probe_up, 1.02, 1.2, today.probe_up),
            most_kbit: self
                .most_kbit
                .filter(|most| most.is_finite() && *most >= MOST_KBIT_LEAST),
        }
    }
}

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

/// What the round trip through the VPN riding the link says of the
/// upload's queue past our CAKE: it waits in the router's queue for the
/// VPN's flow, which no probe meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tunnel {
    /// Not heard, or heard while nothing of ours queued for the VPN, so
    /// it says nothing of the queue.
    Unheard,
    /// Heard no further past its least, beyond its margin, than the
    /// probes' own delay, while the VPN's CAKE held a queue: the VPN's flow
    /// waits nowhere the probes don't.
    Clear,
    /// Heard waiting past CAKE longer than the probes did: the queue is
    /// the router's, and only the VPN's flow meets it.
    Past,
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

/// A held download's queue since it last moved: since when, and the
/// least and the most its delay has been since it was first held.
#[derive(Clone, Copy, Debug)]
struct Draining {
    since: f64,
    least_ms: f64,
    most_ms: f64,
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
    /// The plateau a start took from what it learned before, which counts
    /// as reached until `STARTING_S` into the stay, and until when.
    started_from: f64,
    starting_until: Option<f64>,
    /// Since when data has waited with our CAKE holding nothing, and what
    /// got through each moment of it.
    elsewhere: Option<(f64, f64, u32)>,
    /// What got through when the queue was last found past our CAKE, which
    /// the rate is held under, and when it next looks for more room, or
    /// until when it is looking.
    plateau: Option<f64>,
    next_probe: f64,
    probing_until: Option<f64>,
    /// Whether this is the download, whose cuts hold what has already
    /// crossed the line: its senders hear of a cut only from what CAKE
    /// drops, a round trip later, and what they send meanwhile waits here.
    download: bool,
    /// How a download held under what arrives, with its queue standing,
    /// has seen that queue since it last moved.
    draining: Option<Draining>,
    /// Until when a download leaves a queue alone that its cut didn't
    /// drain.
    anothers_until: f64,
    /// How it tells a queue and finds its line.
    tuning: Tuning,
    /// The network's declared rate, in kbit/s, where it has one.
    anchor: Option<f64>,
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
            started_from: 0.0,
            starting_until: None,
            elsewhere: None,
            plateau: None,
            next_probe: 0.0,
            probing_until: None,
            download: false,
            draining: None,
            anothers_until: f64::NEG_INFINITY,
            tuning: Tuning::default(),
            anchor: None,
        }
    }

    /// How it tells a queue and finds its line.
    #[cfg(test)]
    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    /// Tell a queue and find the line as `tuning` says, within its bounds.
    pub fn tune(&mut self, tuning: Tuning) {
        let tuning = tuning.bounded();
        // The shaper tunes it each second, by the same trial or by none,
        // which leaves it as it was.
        if tuning == self.tuning {
            return;
        }
        // A hold lowered comes down at once, as a probe that met a queue
        // does; one raised is climbed to gently, as after a cut.
        if let Some(plateau) = self.plateau
            && self.probing_until.is_none()
        {
            self.rate = self.rate.min((plateau * tuning.hold).max(LEAST_KBIT));
        }
        self.tuning = tuning;
        self.under_the_ceiling();
    }

    /// Holds the rate under a trial's ceiling. It bounds the rate itself,
    /// not only what's given, so the queue it leaves in our CAKE is known
    /// for ours: held only at the output, a link carrying the ceiling
    /// with our queue full read as the line, and was learned as its
    /// plateau. A ceiling lifted is climbed from gently, as after a cut.
    fn under_the_ceiling(&mut self) {
        if let Some(most) = self.tuning.most_kbit {
            self.rate = self.rate.min(most);
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
            // The link carried the plateau on this network, so the start
            // counts it as reached. With nothing reached, the seconds after
            // a start, while every flow starts again, passed for the line:
            // at 03:41 on 14 November they taught a plateau of 1,404 over
            // one of about 7,600.
            way.started_from = plateau;
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

    /// The most the line is known to carry in this stay: what it carried,
    /// or through a start's own seconds, the plateau it started from.
    fn reached(&self, at: f64) -> f64 {
        if self.starting_until.is_some_and(|until| at < until) {
            self.reached.max(self.started_from)
        } else {
            self.reached
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
    fn own(&mut self, at: f64, dt: f64, ours: Ours, carried: f64, tunnel: Tunnel) -> bool {
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
        // With only the VPN's CAKE holding a queue, the VPN's round trip
        // waits in any queue the router keeps for its flow. Heard clear,
        // the link carrying under the VPN's share is the tunnel's flows
        // backing off after a loss, not the line: on 15 November that
        // dip, taken for the line, held the upload at half of it for 20
        // minutes, with the round trip within 6 ms of its least.
        let backing_off = ours.in_vpn && tunnel == Tunnel::Clear;
        let line_full = self
            .anchor
            .is_none_or(|anchor| through >= anchor * ANCHOR_HOLD * NEAR_THE_HOLD);
        let elsewhere = ours.waiting
            && held_back
            && !backing_off
            && line_full
            && through > FLOWING_KBIT
            && self.reached(at) > 0.0
            && through >= self.reached(at) * REACHED;
        if elsewhere {
            let (since, sum, n) = self.elsewhere.get_or_insert((at, 0.0, 0));
            *sum += through;
            *n += 1;
            if at - *since >= ELSEWHERE_S - 0.05 {
                let plateau = *sum / f64::from(*n);
                self.elsewhere = None;
                self.plateau = Some(plateau);
                self.rate = (plateau * self.tuning.hold).max(LEAST_KBIT);
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
                    self.rate = plateau * self.tuning.hold;
                    self.next_probe = at + PROBE_EVERY_S;
                }
            }
            None if at >= self.next_probe && ours.waiting => {
                self.rate = plateau * self.tuning.probe_up / share;
                self.probing_until = Some(at + PROBE_S);
                self.elsewhere = None;
            }
            // The queue the probes saw has gone, and what the link carried
            // before it still stands for the line: climbing past it would
            // fill the router's hidden queue again, so it climbs back to
            // where it held, gently, as near a queue it met, and no further.
            None if self.rate < plateau * self.tuning.hold => {
                self.rate = (self.rate * CLIMB.powf(dt)).min(plateau * self.tuning.hold);
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

    /// Takes back what cuts for a delay they didn't drain did: the plateau
    /// goes back to where it stood before the first of them, and the rate
    /// to its hold, as a probe finds next.
    fn restore(&mut self, at: f64, plateau: Option<f64>) {
        if let Some(before) = plateau {
            let plateau = self.plateau.map_or(before, |now| now.max(before));
            self.plateau = Some(plateau);
            self.rate = self.rate.max(plateau * self.tuning.hold);
        }
        self.ceiling = None;
        self.late_since = None;
        self.probing_until = None;
        self.next_probe = at + PROBE_EVERY_S;
    }

    /// Holds at `kbit` from now, as a start from a plateau would, and
    /// climbs back to at least that after a cut or a probe.
    fn hold_at(&mut self, at: f64, kbit: f64) {
        self.rate = kbit;
        let plateau = kbit / self.tuning.hold;
        self.plateau = Some(self.plateau.map_or(plateau, |now| now.max(plateau)));
        self.ceiling = None;
        self.late_since = None;
        self.elsewhere = None;
        self.under_half_since = None;
        self.probing_until = None;
        self.next_probe = at + PROBE_EVERY_S;
    }

    pub fn limit(&self) -> Option<u64> {
        self.rate.is_finite().then(|| crate::rate::whole(self.rate))
    }

    /// What a trial's ceiling holds the upload to while the controller
    /// lets it go, which a trial still means to hold.
    fn let_go(&self) -> Option<u64> {
        self.tuning.most_kbit.map(crate::rate::whole)
    }

    /// Whether carrying `carried` leaves this direction idle: no flow, or
    /// a sliver of its rate.
    fn idle(&self, carried: f64) -> bool {
        carried < FLOWING_KBIT || (self.rate.is_finite() && carried < self.rate * IDLE)
    }

    /// Take in a moment: this direction's own delay, with the air and the
    /// other direction's path taken off, and what it carried.
    ///
    /// `tunnel` says what the round trip through a VPN found; past the
    /// probes', the delay is that round trip alone, a
    /// queue the link's probes don't meet.
    pub fn step(
        &mut self,
        at: f64,
        delay_ms: f64,
        carried: f64,
        ours: Option<Ours>,
        tunnel: Tunnel,
    ) {
        let unseen = tunnel == Tunnel::Past;
        // A gap, while the connection was left unlimited or asleep, is one
        // moment, not a climb all through it.
        let dt = self.last_at.map_or(0.0, |last| (at - last).clamp(0.0, 1.0));
        self.last_at = Some(at);
        self.starting_until.get_or_insert(at + STARTING_S);
        // What crossed the line is what CAKE sent, where the shaper
        // counted it: the link sends the home network's traffic too.
        let line = ours
            .and_then(|o| o.sent)
            .map_or(carried, |sent| carried.min(crate::rate::float(sent)));
        self.prove(at, delay_ms, line);
        self.reach(at, line);
        let late = self.tuning.late_ms;
        let crowded = delay_ms > late
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
        if delay_ms > late {
            self.late_since.get_or_insert(at);
        } else {
            self.late_since = None;
        }
        let standing = self
            .late_since
            .is_some_and(|since| at - since >= self.tuning.standing_s);
        let flowing = carried > FLOWING_KBIT;
        // A queue standing while nothing of ours waits to go, and the link
        // carries well under our rate, is the rest of the house's: cutting
        // frees nothing of it and starves this machine, so the rate holds.
        // A line that fell under our rate leaves our sockets waiting, and
        // is cut to as before.
        let theirs = ours.is_some_and(|ours| !ours.waiting) && carried < self.rate * FULL;
        let theirs =
            theirs || self.download && self.anothers(at, delay_ms, carried, delay_ms > late);
        if let (Some(_), Some(plateau)) = (self.probing_until, self.plateau)
            && delay_ms > late
        {
            // A probe that raises a queue found no room past the hold, and
            // Joel's call waits in that queue: it goes back at once, rather
            // than once the queue has stood.
            self.probing_until = None;
            self.rate = plateau * self.tuning.hold;
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
                self.plateau = Some(self.unseen_plateau(at, carried, plateau));
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
                self.rate = self.rate.max(self.reached(at) * UNSEEN_FLOOR);
            }
        } else if let Some(ours) = ours
            && !standing
            && self.own(at, dt, ours, carried, tunnel)
        {
        } else if carried >= self.rate * FULL && delay_ms <= late {
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

    /// Whether a download's queue is another machine's or the air's: held
    /// under what arrives, a queue of ours drains within a moment or two,
    /// however much of its connection the download carries, and one that
    /// doesn't sends the download straight in. On 12 November a queue of
    /// 20 to 110 ms stood for a quarter hour while the download carried 5
    /// to 12 Mbit/s of 85, and cut to 5 Mbit/s under it, what Joel's apps
    /// received waited 100 to 300 ms more in our own CAKE, behind the queue
    /// it couldn't drain. On 13 November the same held of downloads
    /// carrying far more than a sliver, under the access point's air.
    fn anothers(&mut self, at: f64, delay_ms: f64, carried: f64, late: bool) -> bool {
        let held = self.rate.is_finite() && carried >= self.rate * FULL;
        if !(late && held) {
            self.draining = None;
            return late && at < self.anothers_until;
        }
        let draining = self.draining.get_or_insert(Draining {
            since: at,
            least_ms: delay_ms,
            most_ms: delay_ms,
        });
        // A queue a cut drains falls, and one rising is a line falling
        // under the cut, as a roam's does: either is ours, and the clock
        // starts again. Only a queue standing still is another's.
        if delay_ms < draining.least_ms / MOVED {
            draining.least_ms = delay_ms;
            draining.since = at;
        } else if delay_ms > draining.most_ms * MOVED {
            draining.most_ms = delay_ms;
            draining.since = at;
        }
        if at - draining.since >= DRAIN_S + 2.0 * draining.most_ms / 1_000.0 {
            self.rate = f64::INFINITY;
            self.draining = None;
            self.anothers_until = at + ANOTHERS_S;
        }
        late && at < self.anothers_until
    }

    /// The plateau a cut for a queue only the VPN meets leaves, from
    /// `plateau` before it, with the link carrying `carried`.
    fn unseen_plateau(&self, at: f64, carried: f64, plateau: f64) -> f64 {
        // The line is under the plateau only if, over the seconds the
        // queue built, it took less than the hold: the cut's own second is
        // already slowed by a cut before it, and a burst at the hold says
        // nothing of the line. Taken from that second, bursts on 11
        // November ratcheted the plateau from 11,371 to 5,992 in 130 s.
        let most = self
            .carrying
            .iter()
            .map(|&(_, c)| c)
            .fold(carried, f64::max);
        // A declared line carries at least its hold, so a wait under it is
        // the tunnel's: the rate is still cut for it, but the plateau stays.
        // On 16 November one such cut held the upload under the anchor's
        // hold for an hour.
        let declared = self
            .anchor
            .map_or(0.0, |anchor| anchor * ANCHOR_HOLD / self.tuning.hold);
        plateau
            .min(most / self.tuning.hold)
            .max(self.reached(at) * UNSEEN_FLOOR)
            .max(declared.min(plateau))
    }

    fn cut(&mut self, at: f64, delay_ms: f64, carried: f64, filled: bool) {
        // Cut under a sliver, a queue this machine hardly feeds drains no
        // faster and the machine starves, so the reach is halved instead.
        // A connection that really fell that far is met in a few halvings;
        // a queue no cut drains is the guard's, or for a download, sent
        // straight in once a cut under what arrives drains nothing.
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
    /// nearly all that time while the upload carried a sliver of its
    /// connection, or
    /// held under half its base for a minute with no queue standing.
    fn misbehaving(&mut self, at: f64) -> bool {
        // A download judges a crowded queue itself, and the time it left
        // one alone is no cut's: its guard waits as long again after.
        let unbroken = self
            .late_since
            .is_some_and(|since| at - since.max(self.anothers_until) >= FUTILE_S);
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
        let futile =
            self.rate.is_finite() && (unbroken || !self.download && spanned && share >= CROWDED);
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

/// The Wi-Fi hop's own queue, read from the gateway's round trip: its rise
/// past a baseline that falls at once and rises slowly, less its jitter.
/// A cut of ours doesn't drain it, so it comes off what each direction's
/// probes met, here and in a day's account of them (src/day.rs).
#[derive(Clone, Debug, Default)]
pub struct Air {
    base: Option<f64>,
}

impl Air {
    /// Take in the gateway's round trip, if it answered, `dt` seconds on
    /// from the last, and give the air's delay in ms.
    pub fn delay(&mut self, gateway_ms: Option<f64>, dt: f64) -> f64 {
        gateway_ms.map_or(0.0, |gateway| {
            let base = self
                .base
                .map_or(gateway, |b| gateway.min(b + AIR_RISE_MS * dt));
            self.base = Some(base);
            (gateway - base - AIR_MARGIN_MS).max(0.0)
        })
    }
}

/// What the cuts made for a queue only the VPN's round trip meets did to
/// it. That round trip goes to a resolver a whole path away, through the
/// VPN's far end, and waits in whatever queues there too: on the evening
/// of 15 November it took 40 to 490 ms, matching none of what the VPN
/// sent, while the link carried 11 to 13 Mbit/s with the reflectors'
/// delay unmoved, and cuts for it held the upload at a third of the line
/// for two hours. A queue that is ours drains under a cut; one that
/// several cuts in a row leave where it was is the far path's, and goes
/// unheeded a while.
#[derive(Clone, Debug, Default)]
struct Far {
    /// The delay heard over the last `UNSEEN_DRAIN_S`, and when.
    heard: VecDeque<(f64, f64)>,
    /// The first cut of a run that drained none of it: the delay it met,
    /// and the plateau before it.
    first: Option<(f64, Option<f64>)>,
    futile: u32,
    last_cut: f64,
    unheeded_until: f64,
    unheeded_for: f64,
}

impl Far {
    /// Take in the delay only the VPN's round trip meets, and give it, or
    /// none while it goes unheeded.
    fn heed(&mut self, at: f64, hidden: f64) -> Option<f64> {
        self.heard.push_back((at, hidden));
        while self
            .heard
            .front()
            .is_some_and(|&(t, _)| at - t > UNSEEN_DRAIN_S)
        {
            self.heard.pop_front();
        }
        if at - self.last_cut > FUTILE_S {
            self.first = None;
            self.futile = 0;
        }
        (at >= self.unheeded_until).then_some(hidden)
    }

    /// Take in a cut for it, made with the plateau at `plateau` before:
    /// once enough cuts of the run found it where it was, it's let go.
    fn cut(&mut self, at: f64, plateau: Option<f64>) -> Option<Released> {
        self.last_cut = at;
        #[expect(clippy::cast_precision_loss, reason = "a few seconds of round trips")]
        let met =
            self.heard.iter().map(|&(_, ms)| ms).sum::<f64>() / self.heard.len().max(1) as f64;
        let Some((first, before)) = self.first else {
            self.first = Some((met, plateau));
            return None;
        };
        if met < first - VPN_MARGIN_MS {
            // Lower than the run's first found it: drained, by this cut
            // or the ones before.
            return None;
        }
        self.futile += 1;
        if self.futile < FAR_CUTS {
            return None;
        }
        self.unheeded_for = (self.unheeded_for * 2.0).clamp(FAR_S, FAR_MOST_S);
        self.unheeded_until = at + self.unheeded_for;
        self.first = None;
        self.futile = 0;
        self.heard.clear();
        Some(Released { plateau: before })
    }
}

/// A far path's delay let go, and the plateau before the cuts for it.
struct Released {
    plateau: Option<f64>,
}

/// The anchor's guard: an upload held under `ANCHOR_LOW` of its network's
/// anchor for `ANCHOR_LOW_S`, with no cut in that time that drained the
/// queue it was made for, is held too low by a guess, whatever made it.
#[derive(Clone, Debug, Default)]
struct Guard {
    /// Since when the upload has been held under the guard's share.
    low_since: Option<f64>,
    /// When the latest cut not yet seen to drain was made.
    cut_at: Option<f64>,
    /// When a cut was last seen to drain its queue.
    drained_at: Option<f64>,
}

impl Guard {
    /// Takes a moment: the upload's rate, the delay it was stepped on,
    /// whether it was cut, and the anchor, in kbit/s, and answers whether
    /// it has been held low for longer than a drained cut excuses.
    fn breached(
        &mut self,
        at: f64,
        rate: f64,
        delay_ms: Option<f64>,
        late_ms: f64,
        cut: bool,
        anchor: f64,
    ) -> bool {
        if cut {
            self.cut_at = Some(at);
        } else if let Some(cut_at) = self.cut_at {
            if delay_ms.is_some_and(|delay| delay <= late_ms) {
                self.drained_at = Some(at);
                self.cut_at = None;
            } else if at - cut_at > DRAINED_S {
                self.cut_at = None;
            }
        }
        if rate >= anchor * ANCHOR_LOW {
            self.low_since = None;
            return false;
        }
        let since = *self.low_since.get_or_insert(at);
        let from = self.drained_at.map_or(since, |drained| drained.max(since));
        if at - from < ANCHOR_LOW_S {
            return false;
        }
        self.low_since = None;
        self.cut_at = None;
        true
    }

    /// Whether a cut made at `cut_at` has been seen to drain its queue.
    fn eased(&self, cut_at: f64) -> bool {
        self.drained_at.is_some_and(|drained| drained >= cut_at)
    }
}

/// The delay the upload's latest cut was made on, in ms, and whether it
/// was the VPN's alone.
#[derive(Clone, Copy, Debug, Default)]
struct LastCut {
    delay_ms: f64,
    vpn: bool,
}

/// Both directions on one network, and what stands over them.
#[derive(Clone, Debug)]
pub struct Controller {
    pub up: Way,
    pub down: Way,
    air: Air,
    far: Far,
    vpn_base: Option<f64>,
    /// Each round trip through the VPN heard over the last `VPN_LEAST_S`,
    /// and when.
    vpn_heard: VecDeque<(f64, f64)>,
    last_at: Option<f64>,
    answered_at: Option<f64>,
    started_at: Option<f64>,
    returns: usize,
    gave_up_at: Option<f64>,
    /// The network's declared upload rate, its anchor, in kbit/s.
    anchor: Option<f64>,
    guard: Guard,
    /// How many times the guard has sent the upload back to the anchor's
    /// hold, for the shaper to say each.
    pub guarded: u32,
    /// Why the upload holds where it does, and since when.
    why: Why,
    why_since: Option<f64>,
    last_cut: LastCut,
    /// The hold on the VPN's own queue, for a wait inside its tunnel.
    tunnel: crate::tunnel::Hold,
}

impl Controller {
    /// A controller for a network whose bases, up and down, were learned
    /// before, or not.
    pub fn new(up: Option<f64>, down: Option<f64>) -> Controller {
        Controller {
            up: Way::new(up),
            down: Way {
                download: true,
                ..Way::new(down)
            },
            air: Air::default(),
            far: Far::default(),
            vpn_base: None,
            vpn_heard: VecDeque::new(),
            last_at: None,
            answered_at: None,
            started_at: None,
            returns: 0,
            gave_up_at: None,
            anchor: None,
            guard: Guard::default(),
            guarded: 0,
            why: Why::Unmeasured,
            why_since: None,
            last_cut: LastCut::default(),
            tunnel: crate::tunnel::Hold::default(),
        }
    }

    /// The share of its lane the VPN's own queue is held to: under one
    /// while it waits inside its tunnel with the line to spare.
    pub fn tunnel_share(&self) -> f64 {
        self.tunnel.share()
    }

    /// Anchors the upload to its network's declared rate, in kbit/s, or to
    /// none, which leaves it unguarded.
    pub fn anchor(&mut self, up: Option<f64>) {
        self.anchor = up.filter(|kbit| kbit.is_finite() && *kbit > 0.0);
        self.up.anchor = self.anchor;
    }

    /// Starts a stay on an anchored network at the anchor's hold, whatever
    /// the network learned: what it learned may be the cuts' own doing, as
    /// home's 4,586 was on 16 November, and the guard takes a minute to
    /// undo it. A controller already stepping is left as it is.
    pub fn start_at_the_anchor(&mut self) {
        let Some(anchor) = self.anchor else {
            return;
        };
        if self.last_at.is_some() {
            return;
        }
        let hold = anchor * ANCHOR_HOLD;
        let plateau = hold / self.up.tuning.hold;
        self.up.rate = hold;
        self.up.plateau = Some(plateau);
        self.up.started_from = plateau;
    }

    /// Whether the link carries enough for a queue past it to be the
    /// router's: on an anchored network, near the anchor's hold, since a
    /// router holds back nothing while the line has room.
    fn may_fill_the_line(&self, carried: f64) -> bool {
        self.anchor
            .is_none_or(|anchor| carried >= anchor * ANCHOR_HOLD * NEAR_THE_HOLD)
    }

    /// Why the upload holds where it does, in one sentence.
    pub fn why(&self) -> String {
        let at = self.last_at.unwrap_or(0.0);
        let since = self.why_since.unwrap_or(at);
        crate::why::say(self.why, self.up.limit(), self.anchor, at - since)
    }

    /// A controller for a network met before, from what each direction
    /// learned of it.
    pub fn from_learned(up: Learned, down: Learned) -> Controller {
        Controller {
            up: Way::from_learned(up),
            down: Way {
                download: true,
                ..Way::from_learned(down)
            },
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
    /// It's what the VPN's round trip took over the least heard in the last
    /// `VPN_LEAST_S`, whatever its CAKE held then, past what the
    /// probes on the link found each way, while the VPN's CAKE held a
    /// queue: then the VPN sends as fast as it's let, and what it fills is
    /// the upload's. With our CAKE holding nothing, the VPN's flow is
    /// filled by what the shaper doesn't hold, and cutting drains none of
    /// it. With the link's CAKE holding one too, the VPN's flow waits in
    /// it, ours, which the link's own rate settles, so that wait comes
    /// off: it was a few ms in the trace on 11 November, beside a median
    /// 33 ms in the router, and counting nothing while the link's CAKE
    /// held any left that queue to grow.
    fn hidden(&mut self, seen: &Seen) -> f64 {
        let Some(vpn) = seen.vpn_ms else {
            return 0.0;
        };
        let queued = seen.up_ours.filter(|ours| ours.in_vpn || ours.vpn_queued);
        self.vpn_heard.push_back((seen.at_s, vpn));
        while self
            .vpn_heard
            .front()
            .is_some_and(|&(at, _)| seen.at_s - at > VPN_LEAST_S)
        {
            self.vpn_heard.pop_front();
        }
        let base = self.vpn_heard.iter().map(|&(_, ms)| ms).fold(vpn, f64::min);
        self.vpn_base = Some(base);
        let Some(ours) = queued else {
            return 0.0;
        };
        let probes = seen.up_delay_ms.unwrap_or(0.0) + seen.down_delay_ms.unwrap_or(0.0);
        let link = crate::rate::float(ours.link_wait_ms);
        (vpn - base - probes - link - VPN_MARGIN_MS).max(0.0)
    }

    /// Take in a cut for a queue only the VPN's round trip met, made with
    /// the upload's plateau at `plateau` before, and take back what the
    /// cuts for it did once they show it's the far path's.
    fn cut_for_the_vpn(&mut self, at: f64, plateau: Option<f64>) {
        if let Some(released) = self.far.cut(at, plateau) {
            self.up.restore(at, released.plateau);
            // A delay no cut drains may be a path that moved too, as the
            // guard takes it, so its least is learned again.
            self.vpn_base = None;
            self.vpn_heard.clear();
        }
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
        let air = self.air.delay(seen.gateway_ms, dt);
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
            self.judge(
                at,
                if unanswered {
                    Why::Unanswered
                } else {
                    Why::GaveUp
                },
            );
            return Limits {
                up: self.up.let_go(),
                down: None,
            };
        }
        let met = self.hidden(seen);
        // Under the line, a wait only the VPN's round trip meets is the
        // VPN's own, or its far path's: at 02:05 on 16 November it read
        // over 100 ms while the link carried half its line. It's said,
        // and no cut is made for it.
        // That wait the VPN's own queue is held for instead, so it
        // stands in the VPN's CAKE, where Joel's packets go first, rather
        // than inside its tunnel, where they wait alike with agents'.
        let far = self.far.heed(at, met);
        let heeded = far.filter(|_| self.may_fill_the_line(seen.up_kbit));
        self.tunnel.step(
            at,
            dt,
            far.filter(|_| heeded.is_none()),
            self.up.tuning.late_ms,
        );
        let up = seen.up_delay_ms.map(|d| (d - air).max(0.0));
        let down = seen.down_delay_ms.map(|d| (d - air).max(0.0));
        let idle_up = self.up.idle(seen.up_kbit);
        let idle_down = self.down.idle(seen.down_kbit);
        let cuts = self.up.last_cut;
        let fed = up.map(|delay| {
            let beside = if idle_down { down.unwrap_or(0.0) } else { 0.0 };
            self.step_up(seen, (delay - beside).max(0.0), heeded)
        });
        if let Some(delay) = down {
            let beside = if idle_up { up.unwrap_or(0.0) } else { 0.0 };
            // The download's queue at home is the cable's, first come,
            // first served, which the delay finds.
            self.down.step(
                at,
                (delay - beside).max(0.0),
                seen.down_kbit,
                None,
                Tunnel::Unheard,
            );
        }
        for way in [&mut self.up, &mut self.down] {
            if way.misbehaving(at) {
                way.reset();
                self.returns += 1;
                // A queue no cut drains may be a VPN's path that moved,
                // which its base, held under load, would not follow.
                self.vpn_base = None;
                self.vpn_heard.clear();
            }
        }
        if self.given_up() {
            self.gave_up_at.get_or_insert(at);
            self.judge(at, Why::GaveUp);
            return Limits {
                up: self.up.let_go(),
                down: None,
            };
        }
        let cut = self.up.last_cut > cuts;
        let guarded = self.guard_the_anchor(at, fed, cut);
        self.up.under_the_ceiling();
        let why = self.reason(at, guarded, met, heeded);
        self.judge(at, why);
        Limits {
            up: self.up.limit(),
            down: self.down.limit(),
        }
    }

    /// Steps the upload on its own delay, `seen_delay`, and on the queue
    /// only the VPN's flow meets, while it's `heeded`, and gives the delay
    /// it was stepped on.
    fn step_up(&mut self, seen: &Seen, seen_delay: f64, heeded: Option<f64>) -> f64 {
        let at = seen.at_s;
        let hidden = heeded.unwrap_or(0.0);
        let tunnel = if hidden > seen_delay {
            Tunnel::Past
        } else if heeded.is_some()
            && seen.vpn_ms.is_some()
            && seen
                .up_ours
                .is_some_and(|ours| ours.in_vpn || ours.vpn_queued)
        {
            Tunnel::Clear
        } else {
            Tunnel::Unheard
        };
        let (cuts, plateau) = (self.up.last_cut, self.up.plateau);
        let fed = seen_delay.max(hidden);
        self.up.step(at, fed, seen.up_kbit, seen.up_ours, tunnel);
        if self.up.last_cut > cuts {
            self.last_cut = LastCut {
                delay_ms: fed,
                vpn: tunnel == Tunnel::Past,
            };
            if tunnel == Tunnel::Past {
                self.cut_for_the_vpn(at, plateau);
            }
        }
        fed
    }

    /// Sends the upload back to its anchor's hold once the guard finds it
    /// held low past what a drained cut excuses, answering whether it did.
    fn guard_the_anchor(&mut self, at: f64, fed: Option<f64>, cut: bool) -> bool {
        let Some(anchor) = self.anchor else {
            return false;
        };
        let late = self.up.tuning.late_ms;
        // A probe's few seconds over the plateau aren't where the upload is
        // held, so the guard judges the hold it falls back to: judging the
        // probe's rate started the guard's minute again with every probe.
        let up = &self.up;
        let held = match (up.probing_until, up.plateau) {
            (Some(_), Some(plateau)) => plateau * up.tuning.hold,
            _ => up.rate,
        };
        // Held at a trial's ceiling, the upload is where the trial put it,
        // which no return to the anchor would mend.
        if up.tuning.most_kbit.is_some_and(|most| held >= most) {
            return false;
        }
        if !self.guard.breached(at, held, fed, late, cut, anchor) {
            return false;
        }
        self.up.hold_at(at, anchor * ANCHOR_HOLD);
        self.guarded += 1;
        true
    }

    /// What set the upload's rate this moment: `met` is the queue only the
    /// VPN's flow meets, and `heeded` it while cuts still heed it.
    fn reason(&self, at: f64, guarded: bool, met: f64, heeded: Option<f64>) -> Why {
        let up = &self.up;
        let cut_lately = at - up.last_cut < CEILING_S;
        let since = self.why_since.unwrap_or(at);
        // The guard's return is the reason until a cut or a minute passes.
        if guarded || self.why == Why::Guarded && up.last_cut < since && at - since < ANCHOR_LOW_S {
            return Why::Guarded;
        }
        if up.tuning.most_kbit.is_some_and(|most| up.rate >= most) {
            return Why::Ceiling;
        }
        if !up.rate.is_finite() {
            return Why::Unmeasured;
        }
        let share = self.tunnel.share();
        if heeded.is_none() && share < 1.0 {
            return Why::Tunnel {
                delay_ms: self.tunnel.waited_ms(),
                share,
            };
        }
        if heeded.is_none() && met > 0.0 {
            return Why::Far { delay_ms: met };
        }
        if cut_lately {
            let delay_ms = self.last_cut.delay_ms;
            return if self.last_cut.vpn {
                Why::Vpn { delay_ms }
            } else {
                Why::Line {
                    delay_ms,
                    eased: self.guard.eased(up.last_cut),
                }
            };
        }
        let at_the_hold = self
            .anchor
            .is_some_and(|anchor| up.rate >= anchor * ANCHOR_HOLD - 1.0);
        match up.plateau {
            Some(plateau) if !at_the_hold && up.rate <= plateau * up.tuning.hold + 1.0 => {
                Why::Router { plateau }
            }
            _ => Why::Clear,
        }
    }

    /// Takes `why` as the reason now, counting how long from when its kind
    /// last changed.
    fn judge(&mut self, at: f64, why: Why) {
        if self.why_since.is_none() || !why.same_kind(self.why) {
            self.why_since = Some(at);
        }
        self.why = why;
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

    // As at 03:41 on 14 November: a switch restarted the shaper with a
    // plateau of about 7,600 learned, and in the seconds after, the
    // agents' flows, starting again, sent 1.4 Mbit/s with data waiting.
    // With nothing yet carried in the stay, that was taken for a queue
    // past CAKE: the plateau fell to 1,404, the upload was held at 1.3
    // Mbit/s where it had carried 9.9 just before, and the stay saved it.
    #[test]
    fn the_slow_seconds_after_a_start_don_t_teach_a_small_plateau() {
        let learned = Learned {
            base: None,
            plateau: Some(7_600.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        run(&mut c, 0.0, 20.0, |at| Seen {
            up_ours: Some(Ours {
                waiting: true,
                ..Ours::default()
            }),
            ..quiet(1_400.0, 300.0)(at)
        });
        let plateau = c.up.plateau.expect("a plateau");
        assert!(
            plateau >= 3_800.0,
            "a start's slow seconds taught {plateau}"
        );
    }

    // The same seconds behind a VPN whose round trip stands over what it
    // was, with our CAKE holding a queue for it: a queue only the VPN
    // meets lowers the plateau to what got through, and with nothing yet
    // carried in the stay, nothing held it over that.
    #[test]
    fn a_start_behind_a_slow_vpn_keeps_half_its_plateau() {
        let learned = Learned {
            base: None,
            plateau: Some(7_600.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        run(&mut c, 0.0, 20.0, |at| Seen {
            up_ours: Some(Ours {
                queued: true,
                in_vpn: true,
                vpn_queued: true,
                waiting: true,
                ..Ours::default()
            }),
            vpn_ms: Some(if at < 5.0 { 80.0 } else { 130.0 }),
            ..quiet(1_400.0, 300.0)(at)
        });
        let plateau = c.up.plateau.expect("a plateau");
        assert!(
            plateau >= 3_800.0,
            "a start behind a slow VPN taught {plateau}"
        );
    }

    // A network whose line fell to half what was learned of it: the
    // router's hidden queue is still taken back under the line, once the
    // start's own seconds have passed.
    #[test]
    fn a_line_fallen_under_its_learned_plateau_is_still_found() {
        let learned = Learned {
            base: None,
            plateau: Some(12_000.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let limits = run_flow_queued(&mut c, 0.0, 90.0, 6_000.0);
        assert!(
            near(limits, 5_700.0),
            "held under the fallen line: {limits:?}"
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
    fn a_download_carrying_a_sliver_under_another_s_queue_is_left_alone() {
        // As on 12 November from 20:53: the download's own delay stood at
        // 20 to 110 ms for a quarter hour while it carried 5 to 12 Mbit/s
        // of an 85 Mbit/s connection. Halved under it to 5 Mbit/s, the
        // senders kept sending, and what arrived waited 100 to 300 ms in
        // our own CAKE, behind the queue it couldn't drain.
        let mut c = Controller::from_learned(
            Learned::default(),
            Learned {
                base: Some(85_000.0),
                plateau: None,
            },
        );
        let mut under_s = 0.0;
        let mut at = 0.0;
        while at < 120.0 {
            let limits = c.step(&Seen {
                at_s: at,
                ..queued(40.0, 10_000.0)(at)
            });
            if limits.down.is_some_and(|l| l < 10_000) {
                under_s += 0.1;
            }
            at += 0.1;
        }
        assert!(
            under_s <= 12.0,
            "held under what arrived for {under_s:.1} s of 120, under a queue it hardly fed"
        );
        assert_eq!(c.returns, 0, "the guard sent it back: {:?}", c.down.limit());
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
        let mut at_least_s = 0.0;
        for n in 0..4 {
            let from = 10.0 + f64::from(n) * 10.0;
            let mut at = from;
            while at < from + 10.0 {
                let down_ms = if at < from + 9.5 { 22.0 } else { 4.0 };
                let limits = c.step(&Seen {
                    at_s: at,
                    ..queued(down_ms, 2_000.0)(at)
                });
                if limits.down.is_some_and(|l| l < 2_000) {
                    at_least_s += 0.1;
                }
                at += 0.1;
            }
        }
        assert!(
            at_least_s <= 5.0,
            "held under what arrived for {at_least_s:.1} s of 40"
        );
    }

    #[test]
    fn a_download_s_line_falling_to_a_sliver_is_still_shaped() {
        // A line that really falls to a sliver of what the download
        // learned looks at first like another machine's queue, but a cut
        // under it drains its queue, so the download stays held.
        held_under_a_falling_line(40_000.0);
    }

    #[test]
    fn a_download_s_line_falling_to_most_of_it_is_still_shaped() {
        // A line falling to 60% of what the download learned carries more
        // than a sliver, and a cut under it drains its queue as surely.
        held_under_a_falling_line(150_000.0);
    }

    /// Runs a download learned at 250 Mbit/s for a minute over a line
    /// fallen to `line_kbit`, whose buffer a greedy flow fills while the
    /// rate is over it, and asserts the download is held under it.
    fn held_under_a_falling_line(line_kbit: f64) {
        let mut c = Controller::new(None, Some(250_000.0));
        let held = c.down.limit();
        let mut at = 0.0;
        let mut delay_ms: f64 = 0.0;
        while at < 60.0 {
            let rate = c.down.limit().map_or(f64::INFINITY, crate::rate::float);
            let carried = rate.min(line_kbit);
            // The buffer fills as fast as the rate is over the line, to
            // twice it, and drains as fast as the rate is under.
            delay_ms = (delay_ms + (rate.min(line_kbit * 2.0) - line_kbit) / line_kbit * 100.0)
                .clamp(0.0, 600.0);
            c.step(&Seen {
                at_s: at,
                ..queued(delay_ms, carried)(at)
            });
            at += 0.1;
        }
        assert_ne!(c.down.limit(), held, "never cut");
        let over = crate::rate::whole(line_kbit * 1.125);
        assert!(
            c.down.limit() < Some(over),
            "left over a {line_kbit} kbit/s line: {:?}, {delay_ms:.0} ms",
            c.down.limit()
        );
    }

    #[test]
    fn a_download_carrying_much_under_a_queue_its_cut_cant_drain_is_left_alone() {
        // As on 13 November from 20:43: 75 times the download's own delay
        // stood while it carried more than a sliver, and each time it was
        // cut for 30 s until the guard sent it back. Over the 75, the
        // recorder's one-way delay coming down had a median of 35 ms
        // before the cut, 35 during it and 34 after, so the queue was the
        // access point's air, and what the cut held waited up to a second
        // more in our own CAKE.
        let mut c = Controller::from_learned(
            Learned::default(),
            Learned {
                base: Some(85_000.0),
                plateau: None,
            },
        );
        let mut under_s = 0.0;
        let mut at = 0.0;
        while at < 120.0 {
            let limits = c.step(&Seen {
                at_s: at,
                ..queued(40.0, 40_000.0)(at)
            });
            if limits.down.is_some_and(|l| l < 40_000) {
                under_s += 0.1;
            }
            at += 0.1;
        }
        assert!(
            under_s <= 12.0,
            "held under what arrived for {under_s:.1} s of 120, under a queue no cut drained"
        );
        assert_eq!(c.returns, 0, "the guard sent it back: {:?}", c.down.limit());
    }

    #[test]
    fn ten_minutes_after_giving_up_the_connection_is_shaped_again() {
        let mut c = Controller::new(Some(200_000.0), None);
        run(&mut c, 0.0, 95.0, queued_up(40.0, 150_000.0));
        assert!(c.given_up());
        let limits = run(&mut c, 95.0, 700.0, quiet(100_000.0, 300.0));
        assert!(!c.given_up());
        assert_eq!(limits.up, None, "unlimited until a queue stands");
        let limits = run(&mut c, 700.0, 701.2, queued_up(40.0, 150_000.0));
        assert!(limits.up < Some(150_000), "shaped again: {limits:?}");
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

    fn queued_up(up_ms: f64, up_kbit: f64) -> impl Fn(f64) -> Seen {
        move |_| Seen {
            up_delay_ms: Some(up_ms),
            down_delay_ms: Some(0.0),
            gateway_ms: Some(3.0),
            up_kbit,
            down_kbit: 300.0,
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
            way.step(at, 1.0, 0.0, None, Tunnel::Unheard);
            at += 0.1;
        }
        let rate = way.limit().expect("held");
        assert!(
            (120_000..=125_000).contains(&rate),
            "1% a second toward base: {rate}"
        );
        while at < 60.0 {
            way.step(at, 1.0, 0.0, None, Tunnel::Unheard);
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

    // The guard's own cases are the upload's: a download its cut holds
    // under a queue standing still goes straight in within seconds, long
    // before the guard's half minute.
    #[test]
    fn a_queue_standing_half_a_minute_whatever_the_cuts_goes_back_to_its_base() {
        let mut c = Controller::new(Some(200_000.0), None);
        let early = run(&mut c, 0.0, 29.0, queued_up(40.0, 150_000.0));
        assert!(early.up < Some(10_000), "cut to the least: {early:?}");
        let limits = run(&mut c, 29.0, 30.2, queued_up(40.0, 150_000.0));
        assert_eq!(limits.up, Some(200_000));
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
            way.step(
                f64::from(i) * 0.1,
                0.0,
                300_000.0,
                Some(ours),
                Tunnel::Unheard,
            );
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
        let mut c = Controller::new(Some(200_000.0), None);
        run(&mut c, 0.0, 95.0, queued_up(40.0, 150_000.0));
        assert!(c.given_up());
        let limits = run(&mut c, 95.0, 100.0, queued_up(40.0, 150_000.0));
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

    // At 20:48 on 14 November the upload, left unshaped, carried 11.26
    // Mbit/s with its delay at most 16 ms over the floor, where the shaper
    // held 8 to 9: a trial learns whether a queue counted later, longer or
    // held nearer the plateau keeps Joel's delay short.
    #[test]
    fn a_trial_s_later_queue_leaves_a_delay_under_it_uncut() {
        let mut today = Controller::new(None, None);
        let cut = run(&mut today, 0.0, 1.5, queued_up(18.0, 10_000.0));
        assert!(
            cut.up.is_some(),
            "today's 15 ms cuts it, so this shows nothing"
        );
        let mut trial = Controller::new(None, None);
        trial.up.tune(Tuning {
            late_ms: 22.0,
            ..Tuning::default()
        });
        let limits = run(&mut trial, 0.0, 5.0, queued_up(18.0, 10_000.0));
        assert_eq!(limits.up, None, "cut under the trial's 22 ms");
        let limits = run(&mut trial, 5.0, 6.5, queued_up(25.0, 10_000.0));
        assert!(limits.up.is_some(), "a queue past 22 ms is still cut");
    }

    #[test]
    fn a_trial_s_longer_standing_leaves_a_short_queue_uncut() {
        let mut trial = Controller::new(None, None);
        trial.up.tune(Tuning {
            standing_s: 2.0,
            ..Tuning::default()
        });
        let limits = run(&mut trial, 0.0, 1.5, queued_up(18.0, 10_000.0));
        assert_eq!(limits.up, None, "cut before 2 s stood");
        let limits = run(&mut trial, 1.5, 2.5, queued_up(18.0, 10_000.0));
        assert!(limits.up.is_some(), "a queue standing 2 s is still cut");
    }

    #[test]
    fn a_trial_holds_the_upload_at_its_share_of_the_plateau() {
        let mut c = Controller::new(None, None);
        c.up.tune(Tuning {
            hold: 0.98,
            ..Tuning::default()
        });
        let limits = run_flow_queued(&mut c, 0.0, 10.0, 12_000.0);
        assert!(near(limits, 11_760.0), "held under the router: {limits:?}");
    }

    // On 16 November heavy uploads lost a fifth of their packets past our
    // queues while light pings lost none, as a second shaper past ours
    // would drop them. A trial's ceiling holds the upload under it to
    // learn whether the loss is that shaper's, and ends with the trial.
    #[test]
    fn a_trial_s_ceiling_holds_the_upload_under_it_until_it_ends() {
        let mut c = Controller::new(None, None);
        c.up.tune(Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        });
        run_flow_queued(&mut c, 0.0, 20.0, 12_000.0);
        let learned = c.up.plateau;
        let held = flow_queued(&mut c, 20.0, 80.0, |_| 12_000.0);
        assert!(
            held.iter().all(|&(_, limit)| limit == Some(8_500)),
            "held over or under its ceiling: {held:?}"
        );
        assert_eq!(c.up.plateau, learned, "the ceiling was learned as the line");
        assert!(c.why().contains("a trial's ceiling"), "{}", c.why());
        c.up.tune(Tuning::default());
        let limits = run_flow_queued(&mut c, 80.0, 140.0, 12_000.0);
        assert!(near(limits, 11_400.0), "not back at the line: {limits:?}");
    }

    // A ceiling holds an upload nothing measures yet, and otherwise only
    // what passes it.
    #[test]
    fn a_ceiling_over_what_the_upload_is_given_changes_nothing_else() {
        let mut today = Controller::new(None, None);
        let given = flow_queued(&mut today, 0.0, 30.0, |_| 12_000.0);
        let mut trial = Controller::new(None, None);
        trial.up.tune(Tuning {
            most_kbit: Some(50_000.0),
            ..Tuning::default()
        });
        let held = flow_queued(&mut trial, 0.0, 30.0, |_| 12_000.0);
        for (&(at, today), &(_, trial)) in given.iter().zip(&held) {
            let meant = Some(today.map_or(50_000, |limit| limit.min(50_000)));
            assert_eq!(trial, meant, "at {at:.1} s");
        }
    }

    // A trial means to hold the upload while the controller lets it go,
    // as it does when no reflector answers.
    #[test]
    fn a_ceiling_holds_when_the_controller_lets_the_upload_go() {
        let mut c = Controller::new(None, None);
        c.up.tune(Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        });
        let unanswered = |at_s| Seen {
            at_s,
            ..Seen::default()
        };
        c.step(&unanswered(0.0));
        let limits = c.step(&unanswered(UNANSWERED_S + 1.0));
        assert_eq!(limits.up, Some(8_500), "let go past the ceiling");
        assert_eq!(limits.down, None, "the download is no part of it");
    }

    #[test]
    fn a_trial_file_naming_only_a_ceiling_keeps_today_s_tuning() {
        let trial: Tuning = serde_json::from_str(r#"{"most_kbit": 8500}"#).expect("a trial");
        assert_eq!(
            trial,
            Tuning {
                most_kbit: Some(8_500.0),
                ..Tuning::default()
            },
            "the ceiling, and today's for the rest"
        );
        assert!(
            !serde_json::to_string(&Tuning::default())
                .expect("written")
                .contains("most_kbit"),
            "no ceiling is said as none, so records read as before"
        );
    }

    #[test]
    fn a_trial_s_probe_looks_as_far_over_the_plateau_as_it_says() {
        let mut c = Controller::new(None, None);
        c.up.tune(Tuning {
            probe_up: 1.05,
            ..Tuning::default()
        });
        run_flow_queued(&mut c, 0.0, 10.0, 12_000.0);
        let plateau = c.up.plateau.expect("a plateau");
        let held = flow_queued(&mut c, 10.0, 71.0, |_| 12_000.0);
        let most = held.iter().filter_map(|&(_, limit)| limit).max();
        assert!(
            near(most, plateau * 1.05),
            "probed at {most:?} over {plateau}"
        );
    }

    // The shaper tunes the upload each second, trial or none. A cut for a
    // queue only the VPN meets holds the rate at no less than half what
    // the stay reached, which may be over the hold of the plateau it
    // lowered, and being tuned the same must leave it there.
    #[test]
    fn being_tuned_the_same_each_second_changes_nothing() {
        let mut c = Controller::new(None, None);
        c.up.plateau = Some(10_000.0);
        c.up.rate = 11_000.0;
        c.up.tune(Tuning::default());
        assert_eq!(c.up.limit(), Some(11_000), "tuned the same");
        c.up.tune(Tuning {
            hold: 0.9,
            ..Tuning::default()
        });
        assert_eq!(
            c.up.limit(),
            Some(9_000),
            "a hold lowered comes down at once"
        );
    }

    #[test]
    fn a_trial_is_held_within_its_bounds() {
        let wild = Tuning {
            late_ms: 100.0,
            standing_s: 0.0,
            hold: 2.0,
            probe_up: f64::NAN,
            most_kbit: Some(8.5),
        };
        assert_eq!(
            wild.bounded(),
            Tuning {
                late_ms: 30.0,
                standing_s: 1.0,
                hold: 1.0,
                probe_up: PROBE_UP,
                most_kbit: None,
            }
        );
        for most in [f64::NAN, f64::INFINITY, -8_500.0, MOST_KBIT_LEAST - 1.0] {
            let lost = Tuning {
                most_kbit: Some(most),
                ..Tuning::default()
            };
            assert_eq!(lost.bounded().most_kbit, None, "{most} is no ceiling");
        }
        let least = Tuning {
            most_kbit: Some(MOST_KBIT_LEAST),
            ..Tuning::default()
        };
        assert_eq!(least.bounded(), least, "the least ceiling holds");
        let mut c = Controller::new(None, None);
        c.up.tune(wild);
        assert_eq!(c.up.tuning(), wild.bounded());
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

    /// Steps a controller as `behind_a_vpn` does, on a 12000 line with no
    /// queue past our CAKE, while the VPN's round trip waits on its far
    /// path from a minute in: 40 to 135 ms, whatever the VPN sends, as on
    /// the evening of 15 November, when the link carried 11 to 13 Mbit/s
    /// with the reflectors' delay unmoved and the round trip matched none
    /// of what the VPN sent. Answers the VPN's median rate over the last
    /// minute.
    fn behind_a_far_path(c: &mut Controller, to: f64) -> f64 {
        const LINE: f64 = 12_000.0;
        let mut sent = Vec::new();
        let mut at = 0.0;
        let mut noise: u64 = 0x9e37_79b9;
        let mut far = 0.0;
        let mut second = -1.0;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let vpn = rate * crate::qdisc::VPN_SHARE;
            if at >= to - 60.0 {
                sent.push(vpn);
            }
            // The round trip is heard once a second, as live.
            if at.floor() > second {
                second = at.floor();
                noise = noise
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                far = crate::rate::float((noise >> 33) % 1_000) / 1_000.0;
            }
            let queued = vpn < LINE;
            c.step(&Seen {
                at_s: at,
                up_kbit: vpn.min(LINE),
                up_ours: Some(Ours {
                    queued,
                    in_vpn: queued,
                    vpn_queued: queued,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(if at < 60.0 { 27.0 } else { 40.0 + 95.0 * far }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        replay::median(sent.into_iter()).unwrap_or(0.0)
    }

    // A delay on the VPN's far path is no queue of ours: cuts drain none
    // of it, and on 15 November they held the upload at a third of a
    // clear line for two hours. Cuts that leave it where it was let it
    // go, and the upload climbs back to the line.
    #[test]
    fn a_delay_on_the_vpn_s_far_path_that_cuts_don_t_drain_is_let_go() {
        let learned = Learned {
            base: None,
            plateau: Some(11_843.0),
        };
        let mut c = Controller::from_learned(learned, Learned::default());
        let vpn = behind_a_far_path(&mut c, 600.0);
        assert!(
            vpn >= 9_000.0,
            "the VPN sent at {vpn} on a clear 12000 line"
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

    // As at 09:35 on 16 November: at home, held at the anchor's hold, a
    // wait of four seconds only the VPN's round trip met was cut for, as
    // it may be near the line. The cut took the plateau from 11,368 to
    // 10,428, and the hold stayed under the anchor's for an hour, though
    // the VPN went on waiting 40 to 200 ms past its least with nothing of
    // ours queued: the wait was the tunnel's, and said nothing of the line.
    #[test]
    fn a_wait_only_the_vpn_meets_keeps_an_anchored_plateau_at_the_anchor_s_hold() {
        let mut c = anchored(12_000.0);
        c.start_at_the_anchor();
        let at_the_hold = c.up.plateau.expect("a plateau");
        let mut at = 0.0;
        while at < 120.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            // Then the agents wanted less than the hold, and the tunnel
            // kept its own wait, as a VPN's far edge does.
            let (queued, vpn_ms, wanted) = match at {
                t if t < 12.0 => (true, 62.0, 9_852.0),
                t if t < 16.0 => (true, 150.0, 9_852.0),
                t => (false, if t % 10.0 < 5.0 { 100.0 } else { 200.0 }, 6_000.0),
            };
            c.step(&Seen {
                at_s: at,
                up_kbit: rate.min(wanted),
                up_ours: Some(Ours {
                    queued,
                    in_vpn: queued,
                    vpn_queued: queued,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(vpn_ms),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        let plateau = c.up.plateau.expect("a plateau");
        assert!(
            plateau >= at_the_hold - 1.0,
            "the plateau fell to {plateau} under the anchor's {at_the_hold}"
        );
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

    // As live on 15 November from 02:26: with only the VPN's CAKE holding
    // a queue, what the tunnel's flows sent dipped for a few seconds
    // after each loss, as TCP backs off, while the round trip through the
    // VPN stayed within a few ms of its least. Each dip was taken for a
    // router's queue past CAKE, and the plateau went from 11,129 to 8,218
    // in two minutes, and from 9,946 to 5,190 in 75 s at 00:36.
    #[test]
    fn a_vpn_s_flows_backing_off_with_its_round_trip_clear_keep_the_plateau() {
        const PLATEAU: f64 = 10_000.0;
        const LINE: f64 = 10_500.0;
        let mut c = Controller::from_learned(
            Learned {
                base: None,
                plateau: Some(PLATEAU),
            },
            Learned::default(),
        );
        let (mut least, mut at) = (f64::INFINITY, 0.0);
        while at < 180.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let wanted = rate * crate::qdisc::VPN_SHARE;
            // Past the line, the queue is the router's, and the VPN's
            // round trip meets it.
            let ours = wanted < LINE;
            let backing_off = at % 20.0 >= 16.0;
            let vpn = wanted.min(LINE) * if backing_off { 0.8 } else { 1.0 };
            c.step(&Seen {
                at_s: at,
                up_kbit: vpn,
                up_ours: Some(Ours {
                    queued: ours,
                    in_vpn: ours,
                    vpn_queued: ours,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(if ours { 30.0 } else { 60.0 }),
                ..quiet(0.0, 300.0)(at)
            });
            if at >= 10.0 {
                least = least.min(c.up.plateau.unwrap_or(0.0));
            }
            at += 0.1;
        }
        assert!(
            least >= PLATEAU * 0.95,
            "backing off taught a plateau of {least} with the VPN's round trip clear"
        );
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
    fn the_vpn_s_least_is_the_least_heard_in_five_minutes() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.0, through_the_vpn(true, 30.0));
        run(&mut c, 1.0, 21.0, through_the_vpn(true, 50.0));
        assert_eq!(
            c.vpn_base().map(f64::round),
            Some(30.0),
            "a standing queue was taken for the path"
        );
        run(&mut c, 21.0, 330.0, through_the_vpn(false, 50.0));
        assert_eq!(
            c.vpn_base().map(f64::round),
            Some(50.0),
            "the path itself moved, and five minutes passed"
        );
    }

    // As live on 11 November from 20:44: in spells while the VPN's CAKE
    // held nothing, its least crept up a millisecond a second, from 25 to
    // 62 ms, and a queue of 100 to 350 ms past CAKE went uncounted.
    #[test]
    fn the_vpn_s_least_holds_through_minutes_of_a_queue() {
        let mut c = Controller::new(None, None);
        run(&mut c, 0.0, 1.0, through_the_vpn(false, 25.0));
        run(&mut c, 1.0, 241.0, through_the_vpn(false, 125.0));
        assert_eq!(
            c.vpn_base().map(f64::round),
            Some(25.0),
            "a queue of minutes was taken for the path"
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

    /// Steps a controller on a 12000 line with room to spare, behind a
    /// VPN whose own sending falls to four fifths of its CAKE's rate for
    /// three seconds in every ten, as at 02:05 on 16 November: its CAKE
    /// holds a queue, the link's holds none, and the reflectors read the
    /// line clear. Answers the upload's lowest limit.
    fn behind_a_wobbling_vpn(c: &mut Controller, to: f64) -> f64 {
        let mut lowest = f64::INFINITY;
        let mut at = 0.0;
        while at < to {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let cake = rate * crate::qdisc::VPN_SHARE;
            let vpn = if at % 10.0 >= 7.0 { cake * 0.8 } else { cake };
            let limits = c.step(&Seen {
                at_s: at,
                up_kbit: vpn.min(12_000.0),
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    vpn_queued: true,
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            lowest = lowest.min(limits.up.map_or(f64::INFINITY, crate::rate::float));
            at += 0.1;
        }
        lowest
    }

    fn anchored(plateau: f64) -> Controller {
        let mut c = Controller::from_learned(
            Learned {
                base: None,
                plateau: Some(plateau),
            },
            Learned::default(),
        );
        c.anchor(Some(12_000.0));
        c
    }

    #[test]
    fn a_vpn_s_own_dips_don_t_hold_an_anchored_upload_under_the_line() {
        let held = behind_a_wobbling_vpn(&mut anchored(12_000.0), 300.0);
        assert!(
            held >= 10_000.0,
            "held down to {held} on a clear 12000 line"
        );
        let mut unanchored = anchored(12_000.0);
        unanchored.anchor(None);
        let fell = behind_a_wobbling_vpn(&mut unanchored, 300.0);
        assert!(
            fell < 8_000.0,
            "with no anchor it held at least {fell}, so the wobble shows nothing"
        );
    }

    /// Steps a controller whose upload fills what it's held to, with no
    /// queue anywhere, and answers its limit at each moment named.
    fn filling(c: &mut Controller, to: f64, at_moments: &[f64]) -> Vec<Option<u64>> {
        let mut said = vec![None; at_moments.len()];
        let mut at = 0.0;
        while at < to {
            let rate = c.up.limit().map_or(12_000.0, crate::rate::float);
            let limits = c.step(&Seen {
                at_s: at,
                up_kbit: rate.min(12_000.0),
                up_ours: Some(Ours {
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            for (moment, limit) in at_moments.iter().zip(said.iter_mut()) {
                if (at - moment).abs() < 0.05 {
                    *limit = limits.up;
                }
            }
            at += 0.1;
        }
        said
    }

    #[test]
    fn an_anchored_upload_held_low_with_no_cut_that_drained_goes_back_to_its_hold() {
        let mut c = anchored(4_069.0);
        let said = filling(&mut c, 70.0, &[50.0, 65.0]);
        assert!(near(said[0], 3_866.0), "before a minute: {:?}", said[0]);
        assert!(near(said[1], 10_800.0), "after a minute: {:?}", said[1]);
        assert_eq!(c.guarded, 1);
        assert!(c.why().contains("back to the line's hold"), "{}", c.why());
        let mut unanchored = anchored(4_069.0);
        unanchored.anchor(None);
        let said = filling(&mut unanchored, 70.0, &[65.0]);
        assert!(
            said[0].is_some_and(|l| l < 6_000),
            "with no anchor: {:?}",
            said[0]
        );
    }

    // A ceiling under the guard's share of the anchor is the trial's, so
    // the guard leaves it, and what the upload learned stays as it was.
    #[test]
    fn an_anchored_upload_held_under_a_trial_s_ceiling_stays_there() {
        let mut c = anchored(12_000.0);
        c.up.tune(Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        });
        let said = filling(&mut c, 130.0, &[65.0, 125.0]);
        assert_eq!(said, [Some(8_500), Some(8_500)], "held at the ceiling");
        assert_eq!(c.guarded, 0, "the guard returned it to the anchor");
        assert_eq!(c.up.plateau, Some(12_000.0), "the line learned under it");
    }

    /// At 07:34 on 16 November a hidden queue held the plateau at 10,040,
    /// so the upload held at 9,538, under the guard's 9,600. Agents sent
    /// about 9 Mbit/s, under what a probe looks at, so no probe raised the
    /// plateau, and each probe's 3 s over the guard's share started its
    /// minute again: it held there for good.
    #[test]
    fn probes_over_the_guard_s_share_don_t_keep_a_low_hold_from_it() {
        let mut c = anchored(10_040.0);
        let mut at = 0.0;
        while at < 200.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            c.step(&Seen {
                at_s: at,
                up_kbit: rate.min(9_000.0),
                up_ours: Some(Ours {
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        assert_eq!(c.guarded, 1, "the guard never saw the hold under its share");
        assert!(
            c.up.limit().is_some_and(|l| l >= 10_800),
            "held at {:?}",
            c.up.limit()
        );
    }

    #[test]
    fn cuts_that_drain_a_line_fallen_under_its_anchor_keep_the_guard_off() {
        let mut c = anchored(12_000.0);
        let mut at = 0.0;
        while at < 200.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            c.step(&Seen {
                at_s: at,
                up_delay_ms: Some(if rate > 6_000.0 { 40.0 } else { 1.0 }),
                up_kbit: rate.min(6_000.0),
                up_ours: Some(Ours {
                    queued: rate < 6_000.0,
                    waiting: true,
                    ..Ours::default()
                }),
                ..quiet(0.0, 300.0)(at)
            });
            at += 0.1;
        }
        assert_eq!(c.guarded, 0, "the guard undid cuts that drained");
        assert!(
            c.up.limit().is_some_and(|l| l < 7_000),
            "{:?}",
            c.up.limit()
        );
    }

    #[test]
    fn a_clear_anchored_upload_says_so() {
        let mut c = anchored(12_000.0);
        filling(&mut c, 30.0, &[]);
        assert!(c.why().starts_with("Upload "), "{}", c.why());
        assert!(c.why().contains("clear for"), "{}", c.why());
    }

    /// Steps an anchored controller for `seconds` whose agents fill the
    /// VPN, its CAKE holding a queue, through a tunnel that delivers
    /// `tunnel(at)` kbit/s: what the VPN's CAKE sends past that waits in
    /// the tunnel, where the round trip through it meets it. Answers the
    /// upload's lowest limit, and the tunnel's mean wait over the last
    /// half of the run, in ms.
    fn filling_a_tunnel(
        c: &mut Controller,
        from: f64,
        seconds: f64,
        tunnel: impl Fn(f64) -> f64,
        queued_bits: &mut f64,
    ) -> (f64, f64) {
        const BASE_MS: f64 = 30.0;
        const DT: f64 = 0.1;
        let (mut lowest, mut waited, mut heard) = (f64::INFINITY, 0.0, 0.0_f64);
        let mut at = from;
        while at < from + seconds {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let cake = rate * crate::qdisc::VPN_SHARE * c.tunnel_share();
            let delivered = tunnel(at);
            *queued_bits = (*queued_bits + (cake - delivered) * 1_000.0 * DT).max(0.0);
            let wait_ms = *queued_bits / delivered;
            let limits = c.step(&Seen {
                at_s: at,
                up_kbit: cake.min(delivered),
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    vpn_queued: true,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(BASE_MS + wait_ms),
                ..quiet(0.0, 300.0)(at)
            });
            lowest = lowest.min(limits.up.map_or(f64::INFINITY, crate::rate::float));
            if at >= from + seconds / 2.0 {
                waited += wait_ms;
                heard += 1.0;
            }
            at += DT;
        }
        (lowest, waited / heard.max(1.0))
    }

    // As in a proof at 08:58 on 16 November: agents filled the tunnel past
    // what it delivered, 7 to 9 Mbit/s under a line with room, and the
    // queue formed past the VPN's CAKE, where Joel's upload inside the VPN
    // got 2 to 3% of the link. Held under the tunnel, the queue is the
    // CAKE's, where his packets go first.
    #[test]
    fn a_wait_inside_the_tunnel_holds_the_vpn_s_own_queue_not_the_upload() {
        let mut c = anchored(12_000.0);
        let mut queued = 0.0;
        let (lowest, waited) = filling_a_tunnel(&mut c, 0.0, 120.0, |_| 7_500.0, &mut queued);
        assert!(
            lowest >= 10_799.0,
            "the upload was cut to {lowest} for the tunnel's own queue"
        );
        assert!(
            waited < 30.0,
            "the tunnel held {waited:.0} ms on average past the VPN's CAKE, at a share of {:.2}",
            c.tunnel_share()
        );
        assert!(c.why().contains("inside its tunnel"), "{}", c.why());
    }

    #[test]
    fn the_vpn_s_own_queue_climbs_back_once_the_tunnel_delivers_again() {
        let mut c = anchored(12_000.0);
        let mut queued = 0.0;
        filling_a_tunnel(&mut c, 0.0, 60.0, |_| 7_500.0, &mut queued);
        let held = c.tunnel_share();
        assert!(
            held < 0.95,
            "a full tunnel left the VPN's share at {held:.2}"
        );
        filling_a_tunnel(&mut c, 60.0, 60.0, |_| 20_000.0, &mut queued);
        assert!(
            c.tunnel_share() > 0.99,
            "a clear tunnel left the VPN's share at {:.2}, from {held:.2}",
            c.tunnel_share()
        );
    }

    // A wait no hold drains is the far path's, as on 15 November, and
    // holding the VPN for it would only slow everything inside.
    #[test]
    fn a_far_wait_the_vpn_s_hold_doesn_t_drain_lets_the_hold_go() {
        let mut c = anchored(12_000.0);
        sending_through_a_slow_vpn(&mut c, 6_000.0, 150.0);
        assert!(
            c.tunnel_share() > 0.99,
            "a far path's wait held the VPN to {:.2} of its lane",
            c.tunnel_share()
        );
    }

    /// Steps an anchored controller whose agents send `sending` through
    /// the VPN, its CAKE holding a queue, while the round trip through it
    /// reads `vpn_ms` once the first ten seconds have set its least.
    /// Answers the upload's lowest limit after those ten seconds.
    fn sending_through_a_slow_vpn(c: &mut Controller, sending: f64, vpn_ms: f64) -> f64 {
        let mut lowest = f64::INFINITY;
        let mut at = 0.0;
        while at < 120.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let limits = c.step(&Seen {
                at_s: at,
                up_kbit: sending.min(rate),
                up_ours: Some(Ours {
                    queued: true,
                    in_vpn: true,
                    vpn_queued: true,
                    waiting: true,
                    ..Ours::default()
                }),
                vpn_ms: Some(if at < 10.0 { 30.0 } else { vpn_ms }),
                ..quiet(0.0, 300.0)(at)
            });
            if at >= 10.0 {
                lowest = lowest.min(limits.up.map_or(f64::INFINITY, crate::rate::float));
            }
            at += 0.1;
        }
        lowest
    }

    // As at 02:05 on 16 November: the agents' upload through the VPN ran
    // at half the line while the round trip through it rose past 100 ms.
    // A router queues nothing while the line has room, so that wait is
    // the VPN's own, and cuts here only held the upload low.
    #[test]
    fn an_anchored_upload_under_the_line_isnt_cut_for_the_vpn_s_own_delay() {
        let mut c = anchored(12_000.0);
        let lowest = sending_through_a_slow_vpn(&mut c, 6_000.0, 150.0);
        assert!(
            lowest >= 10_799.0,
            "cut to {lowest} while the link carried half its line"
        );
        assert!(c.why().contains("no cut here reaches"), "{}", c.why());
    }

    // Near the line, the router may hold a queue only the VPN's flow
    // meets, and a cut that drains it is still made there.
    #[test]
    fn an_anchored_upload_near_the_line_is_cut_for_a_queue_only_the_vpn_meets() {
        let mut c = anchored(12_000.0);
        let seen = behind_a_buffering_router(&mut c, 180.0, 11_000.0);
        assert!(
            seen.queue_ms < 15.0,
            "Joel's call waited {} ms in the router: {seen:?}",
            seen.queue_ms
        );
    }

    // A round trip through the VPN a few ms over its least, less than the
    // probes' own delay, shows no queue only the VPN's flow meets, so its
    // flows backing off after a loss aren't the line either.
    #[test]
    fn a_vpn_s_flows_backing_off_with_its_round_trip_near_the_probes_keep_the_plateau() {
        const PLATEAU: f64 = 10_000.0;
        const LINE: f64 = 10_500.0;
        let mut c = Controller::from_learned(
            Learned {
                base: None,
                plateau: Some(PLATEAU),
            },
            Learned::default(),
        );
        let (mut least, mut at) = (f64::INFINITY, 0.0);
        while at < 180.0 {
            let rate = c.up.limit().map_or(f64::INFINITY, crate::rate::float);
            let wanted = rate * crate::qdisc::VPN_SHARE;
            // Past the line, the queue is the router's, as in the test above.
            let ours = wanted < LINE;
            let backing_off = at % 20.0 >= 16.0;
            let vpn = wanted.min(LINE) * if backing_off { 0.8 } else { 1.0 };
            c.step(&Seen {
                at_s: at,
                up_delay_ms: Some(10.0),
                up_kbit: vpn,
                up_ours: Some(Ours {
                    queued: ours,
                    in_vpn: ours,
                    vpn_queued: ours,
                    waiting: true,
                    ..Ours::default()
                }),
                // The least is 30, and 50 is 4 ms past the probes' 11 and
                // the margin: less than the upload's own 10.
                vpn_ms: Some(if at < 10.0 {
                    30.0
                } else if ours {
                    50.0
                } else {
                    60.0
                }),
                ..quiet(0.0, 300.0)(at)
            });
            if at >= 10.0 {
                least = least.min(c.up.plateau.unwrap_or(0.0));
            }
            at += 0.1;
        }
        assert!(
            least >= PLATEAU * 0.95,
            "backing off taught a plateau of {least} with the VPN's round trip near the probes'"
        );
    }

    // On 16 November home's learned plateau had fallen to 4,586 under
    // cuts for the VPN's own delay, and every restart began there, a
    // minute under the line before the guard sent it back.
    #[test]
    fn an_anchored_start_begins_at_the_anchor_s_hold() {
        let mut c = anchored(4_586.0);
        c.start_at_the_anchor();
        assert!(near(c.up.limit(), 10_800.0), "{:?}", c.up.limit());
        let mut higher = anchored(12_500.0);
        higher.start_at_the_anchor();
        assert!(near(higher.up.limit(), 10_800.0), "{:?}", higher.up.limit());
        let mut unanchored = anchored(4_586.0);
        unanchored.anchor(None);
        unanchored.start_at_the_anchor();
        assert!(
            unanchored.up.limit().is_some_and(|l| l < 5_000),
            "with no anchor: {:?}",
            unanchored.up.limit()
        );
    }
}
