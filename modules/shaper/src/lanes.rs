//! One allocation of the upload between its two lanes: straight over the
//! connection, and inside the VPN.
//!
//! On the connection a VPN is one flow, so whatever its CAKE holds back
//! is sorted there, and nothing past it may queue. Under the connection's
//! rate its HTB gives three classes a rate each: Joel's traffic straight on
//! the connection, over a diffserv4 CAKE; the VPN's outer flow, at V, with
//! the VPN's own CAKE held just under it; and agents' straight on the
//! connection, over a CAKE that shares flow by flow. This says, each
//! second, how the controller's rate is shared between the three, and
//! makes the classes.
//!
//! Joel comes first: he gets what he carries on each lane, a fifth more
//! and a megabit, so his next burst finds room before the next second
//! moves the rates; a fifth alone would take ten seconds to grow from one
//! megabit to six. What his straight lane is given rises at once
//! and falls by a fifth a second, as the VPN's share, which carries his
//! part inside it, already does: a second in which something past the
//! shaper, a router's queue when the controller climbs past the line, gave
//! his upload little would otherwise leave him little the next, and beside
//! agents he'd take seconds to grow back. HTB lends Joel's two classes
//! what the others leave before it lends agents' any, and every class may
//! borrow up to the whole rate: the VPN's own CAKE is what holds it near
//! V, and when that runs ahead of a cut for a moment, its class borrows
//! rather than dropping what's inside the tunnel alike. But while any
//! agent has something waiting, a quarter of the rate is theirs whatever
//! Joel wants: agents are always usable.
//!
//! What Joel carries now on both lanes comes first, and the VPN's share
//! is set from it. What his straight lane was given is held past that, as
//! a share of the rate, from agents past their part of the quarter: from
//! those straight on the connection while any wait there, and HTB lends it
//! back to them while he leaves it; otherwise from the VPN's, while they
//! wait. Held from the VPN it sits unused while he's quiet, as the VPN's
//! own CAKE holds its class hard: after four megabits a second straight it
//! falls to his least in about eight seconds, leaving about a second of a
//! ten-megabit line unused, and more after more. Moving his upload into
//! the VPN then finds it kept only what he carried there the second before
//! and his headroom past it, so the lane he left holds back his growth on
//! the one he's on until his need there fills the room or the hold falls:
//! a second when he moves at speed, a few when he starts slowly. But
//! agents' uploads are mostly inside the VPN, where a dip beside them alone
//! would otherwise leave him little more than what he carried. With no
//! agent waiting nothing is held from the VPN, so a lane he left never
//! squeezes it.
//! While the VPN's share falls, it never takes the part of the quarter that
//! agents waiting straight are owed.
//!
//! Agents get what Joel leaves, split evenly between the lanes on which
//! some wait, with three exceptions. The VPN's agents may go short by up
//! to a twentieth of the VPN's whole share, the band it stays still inside,
//! which can be a good part of their half beside his. Agents straight go
//! short, down to their part of the quarter, while the VPN's share stands
//! or falls above what it's asked. And what's held of his straight lane for
//! a few seconds after he stops is only lent to agents straight, or held
//! from the VPN's, while it lasts. Even so a push straight to a builder
//! gets half rather than the sixteenth CAKE's bulk tin gave it, which on
//! 16 November was 346 kbit/s of 5.5 Mb/s. The split isn't by flows: on
//! 16 November at 20:25 agents held 117 flows to the model inside the VPN
//! and a few straight to the builders, so a split by flows gave the
//! builders' copies 8 kbit/s and they failed.
//!
//! While the VPN's flow waits past our queues beside what agents straight
//! send (src/oneway.rs), and they wait, they're held to a share of what
//! they'd get, never under their part of the quarter, and that is all
//! their class may borrow. The VPN's own CAKE keeps it from taking what
//! they give up, so only Joel straight may, who goes first; otherwise it
//! goes unsent and the queue past us drains. It's the one class whose
//! ceiling isn't the whole rate, and only while agents straight wait.
//!
//! A lane's agents wait while they hold a few packets, or for twenty
//! seconds after its CAKE last dropped or marked one: a copy starved to a
//! window of two packets holds too little to count, yet CAKE drops what
//! waits past its target, so a drop says the lane wants more than it gets.
//!
//! The VPN's share rises at once and falls by at most a fifth a second, so
//! a copy's queue emptying between bursts doesn't swing it. It gives up at
//! once only what Joel carries straight now and what agents straight are
//! owed, and a move under a twentieth isn't made at all. It's held as a
//! share, so a rate the controller cuts cuts V with it at once. A rate moved
//! and moved back between two seconds' splits leaves the share as it was,
//! but one still moved at a split stores what that rate gave, capped lower
//! or, as Joel's part is a larger share of less, raised, and it moves from
//! there once the rate comes back; most of the controller's cuts outlast a
//! split. A share raised so is kept as a share rather than in kbit/s: what
//! the VPN doesn't use of it HTB lends the other classes, Joel's first, and
//! it falls a fifth a second, where held in kbit/s a cut would come out of
//! what's held of Joel's straight lane, and agents straight past their part
//! of the quarter, before the VPN. room.rs, which did this for the two
//! bulk tins alone, flipped its two lines about 2,200 times in two days.
//!
//! Long because its tests, over half of it, walk each case of the split:
//! Joel alone, agents on either lane or both, a starved copy, a cut, and
//! the VPN's share falling and rising.

use serde::{Deserialize, Serialize};

use crate::rate::whole;

/// What Joel gets past what he carried, as a share of it, and in kbit/s,
/// so his next burst finds room.
const HEADROOM: f64 = 1.2;
const JOEL_LEAST: f64 = 1_000.0;
/// The least rate a class is given, in kbit/s: HTB takes none at nothing.
const LEAST_KBIT: u64 = 8;
/// How far under its class's rate the VPN's own CAKE is held, so its
/// queue forms inside the VPN, where its connections can be told apart:
/// the class counts each outer packet's link framing, which the CAKE
/// doesn't.
pub const VPN_HELD: f64 = 0.95;
/// What a class sends in its turn before its peers have theirs: one
/// full-sized packet, so a class at a small rate takes turns evenly.
const QUANTUM: u32 = 1514;
/// The share of the rate agents get, between them, while any waits.
pub const AGENTS_FLOOR: f64 = 0.25;
/// A lane's agents with more than this many bytes held are waiting.
const WAITING_BYTES: u64 = 3_000;
/// How long a lane's agents wait after their CAKE last dropped or marked
/// one, in seconds. A copy given its half drops seldom, so a lane that
/// waited only in the second of a drop would hand its half back at once
/// and be starved again; twenty seconds keeps the half through a copy's
/// quiet stretches, and costs the VPN at most that long after it ends.
const PRESSED_S: f64 = 20.0;
/// The most the VPN's share, or what Joel's straight lane is held at,
/// falls in a second, as a share of itself; but the VPN gives what Joel
/// carries straight now, agents waiting straight their part of the
/// quarter, and, while its agents alone wait, what's held of his straight
/// lane, at once. What's held of his lane
/// falls by it for each second of a gap, since he was quiet through them;
/// the VPN's share falls by it once however long the gap, as it always
/// has, since a loop that stalled read nothing of its agents meanwhile.
const FALL: f64 = 0.2;
/// A move of the VPN's share under this share of itself isn't made.
const STILL: f64 = 0.05;

/// One lane's agents, as its CAKE shows them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Agents {
    /// Bytes held in the agents' queue.
    pub backlog: u64,
    /// Flows with something held.
    pub flows: u32,
    /// The packets the agents' CAKE has dropped or marked since it was
    /// made, past its target: not those its ACK filter took. None where
    /// a CAKE couldn't be read, so the next count read starts afresh.
    pub dropped: Option<u64>,
}

impl Agents {
    /// Whether the lane's agents wait, given whether their CAKE dropped
    /// or marked one lately, `pressed`.
    fn waiting(self, pressed: bool) -> bool {
        pressed || self.backlog > WAITING_BYTES
    }

    /// The flows a lane's agents are waiting on: none while they don't
    /// wait, and at least one while they do, since CAKE may read none at
    /// the moment it holds a backlog.
    fn waiting_flows(self, pressed: bool) -> u32 {
        if self.waiting(pressed) {
            self.flows.max(1)
        } else {
            0
        }
    }
}

/// What the two lanes carried and hold, read each second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seen {
    /// The controller's rate for the upload, in kbit/s.
    pub rate: f64,
    /// What Joel carried straight on the connection, in kbit/s.
    pub joel_straight: f64,
    /// What Joel carried inside the VPN, in kbit/s.
    pub joel_vpn: f64,
    pub agents_straight: Agents,
    pub agents_vpn: Agents,
    /// The share of their split agents straight are held to while the
    /// VPN's flow waits beside what they send (src/tunnel.rs), or none.
    pub agents_held: Option<f64>,
}

/// What the controller's hold on agents straight says to the lanes: the
/// share they're held to, or none while it's whole.
pub fn held_to(share: f64) -> Option<f64> {
    (share < 1.0).then_some(share)
}

/// The connection's three classes' rates, in kbit/s, which add up to the
/// controller's rate, or to less while agents straight are held.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Split {
    /// Joel straight on the connection.
    pub joel: f64,
    /// The VPN's outer flow, V.
    pub vpn: f64,
    /// Agents straight on the connection.
    pub agents: f64,
    /// The most agents straight may borrow up to while they're held, in
    /// kbit/s: their own rate, which their class takes as its ceiling.
    /// None lets them borrow the whole rate.
    pub agents_most: Option<f64>,
    /// Whether a change of the classes to this split partly failed, so
    /// what they hold may differ from it in any way, a ceiling included.
    pub unsent: bool,
}

impl Split {
    /// Whether a class's rate moved by more than a hundredth of `rate`
    /// from `was`'s, so the classes are worth changing; always, while
    /// `was` may not be what they hold.
    pub fn moved_from(&self, was: &Self, rate: f64) -> bool {
        let past = |a: f64, b: f64| (a - b).abs() * 100.0 > rate;
        was.unsent
            || past(self.joel, was.joel)
            || past(self.vpn, was.vpn)
            || past(self.agents, was.agents)
            || self.agents_most.is_some() != was.agents_most.is_some()
    }

    /// What stands once `tried` was sent in place of this split: all of it
    /// when every command took. Otherwise this split, marked unsent so the
    /// next is sent whatever it holds, with any cap either held, since a
    /// change that partly failed may have left one on agents' class, and
    /// letting agents go must then send the classes too.
    #[must_use]
    pub fn after(self, tried: Self, took: bool) -> Self {
        if took {
            tried
        } else {
            Self {
                agents_most: tried.agents_most.or(self.agents_most),
                unsent: true,
                ..self
            }
        }
    }
}

/// How the upload is shared, for `shaper status`: each class's rate, in
/// kbit/s, and the agents' flows waiting on each lane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Said {
    pub joel: u64,
    pub vpn: u64,
    pub agents: u64,
    pub waiting_vpn: u32,
    pub waiting_direct: u32,
}

/// Follows the VPN's share, and what Joel's straight lane is held at,
/// from second to second; a rate of zero forgets both.
#[derive(Clone, Debug, Default)]
pub struct Lanes {
    share: Option<f64>,
    /// Whether the share was last stored under what the VPN kept, for
    /// what's held of Joel's straight lane, so it rises back past the band
    /// it otherwise stays still inside.
    held_back: bool,
    /// What Joel's straight lane was given at the last real second, or with
    /// no agent waiting what's held of it as it falls, as a share of the
    /// rate, as the VPN's is, so a new rate takes it along.
    joel: f64,
    last: Seen,
    /// The seconds split over so far.
    clock: f64,
    /// When each lane's CAKE, straight and inside the VPN, last dropped
    /// or marked one, on `clock`.
    pressed_at: (Option<f64>, Option<f64>),
    /// What each had dropped and marked when last read.
    dropped: (Option<u64>, Option<u64>),
}

/// Whether a CAKE dropped or marked one between two counts of it. A count
/// that couldn't be read, or a CAKE made again, which counts from
/// nothing, reads as no drop.
fn pressed(was: Option<u64>, now: Option<u64>) -> bool {
    matches!((was, now), (Some(was), Some(now)) if now > was)
}

impl Lanes {
    /// What `split`, the split on the classes, gives each lane, beside the
    /// agents' flows waiting on each as last seen; none before a split.
    pub fn said(&self, split: &Split) -> Option<Said> {
        if split.joel + split.vpn + split.agents <= 0.0 {
            return None;
        }
        Some(Said {
            joel: whole(split.joel),
            vpn: whole(split.vpn),
            agents: whole(split.agents),
            waiting_vpn: self
                .last
                .agents_vpn
                .waiting_flows(self.lately(self.pressed_at.1)),
            waiting_direct: self
                .last
                .agents_straight
                .waiting_flows(self.lately(self.pressed_at.0)),
        })
    }

    /// Whether a lane last pressed at `at` did so within `PRESSED_S`.
    fn lately(&self, at: Option<f64>) -> bool {
        at.is_some_and(|at| self.clock - at < PRESSED_S)
    }

    /// The split at a new rate, from what was seen last: the controller
    /// moves its rate between the seconds this is told of the lanes.
    pub fn at(&mut self, rate: f64) -> Split {
        let seen = Seen { rate, ..self.last };
        self.split(&seen, 0.0)
    }

    /// Forget that agents straight were held, so the next split at a new
    /// rate lets them borrow the whole rate again: a link the controller
    /// no longer holds is split only so.
    pub fn let_agents_go(&mut self) {
        self.last.agents_held = None;
    }

    /// Whether agents straight on the connection wait, as the last split
    /// judged it: what lets the controller hold them for the VPN's wait.
    pub fn agents_wait_straight(&self) -> bool {
        self.last
            .agents_straight
            .waiting(self.lately(self.pressed_at.0))
    }

    /// How the rate is split now, given what was seen `dt` seconds after
    /// the last.
    pub fn split(&mut self, seen: &Seen, dt: f64) -> Split {
        self.last = *seen;
        let (straight, inside) = (seen.agents_straight, seen.agents_vpn);
        // A new rate between seconds reads nothing new, so what was
        // pressed stays.
        if dt > 0.0 {
            self.clock += dt;
            if pressed(self.dropped.0, straight.dropped) {
                self.pressed_at.0 = Some(self.clock);
            }
            if pressed(self.dropped.1, inside.dropped) {
                self.pressed_at.1 = Some(self.clock);
            }
            self.dropped = (straight.dropped, inside.dropped);
        }
        let rate = seen.rate;
        if rate <= 0.0 {
            self.share = None;
            self.held_back = false;
            self.joel = 0.0;
            return Split::default();
        }
        let (beside, within) = (
            straight.waiting(self.lately(self.pressed_at.0)),
            inside.waiting(self.lately(self.pressed_at.1)),
        );
        let floor = if beside || within {
            rate * AGENTS_FLOOR
        } else {
            0.0
        };
        // What Joel carries now on his two lanes shrinks alike when
        // together they'd pass what he may take, so neither is starved for
        // the other.
        let room = rate - floor;
        let now = (need(seen.joel_straight), need(seen.joel_vpn));
        let scale = (room / (now.0 + now.1)).min(1.0);
        let (straight_now, joel_vpn) = (now.0 * scale, now.1 * scale);
        // What's left is the agents', evenly between the lanes they wait
        // on. With none waiting it goes to the VPN, whose own CAKE is
        // held to V: the classes beside it borrow what it leaves.
        let left = rate - straight_now - joel_vpn;
        let (to_vpn, straight_floor) = match (beside, within) {
            (true, true) => (left / 2.0, floor / 2.0),
            (true, false) => (0.0, floor),
            (false, _) => (left, 0.0),
        };
        // What's held of Joel's straight lane past what he carries comes
        // from the VPN's agents while they wait and none waits straight,
        // never from Joel's part in it or the agents' quarter, and at once,
        // as what he carries straight does. With no agent waiting it isn't
        // held there: the lane he left would squeeze the one he's on. What's
        // held is what his straight lane was given, not what he'd have
        // wanted: busy on both lanes, the room shrank it, and holding more
        // took the VPN down by a third in a second's dip, his lane in it too.
        let extra = self.joel_held(straight_now, rate, dt) - straight_now;
        let from_vpn = if within && !beside {
            extra.min(room - straight_now - joel_vpn).max(0.0)
        } else {
            0.0
        };
        let kept = self
            .hold(joel_vpn + to_vpn, joel_vpn, rate, dt)
            .min(rate - straight_now - straight_floor);
        let vpn = kept.min(rate - straight_now - from_vpn);
        // What the caps left is what the VPN has, so it falls from there:
        // held above it, it would climb back the next second into the room
        // held for Joel, where its own CAKE would leave it unused. Only a
        // real second is stored: a new rate between seconds asks for more or
        // caps at that rate alone, and kept, a moment's cut would end the
        // VPN's slow fall, the only hold Joel's part in it gets, or come back
        // with the rate into the room held for him straight. A share stored
        // under what the VPN kept rises back past the band it otherwise
        // stays still inside, so it comes back to its own as his hold falls.
        if dt > 0.0 {
            self.share = Some(vpn / rate);
            self.held_back = vpn < kept;
        }
        // With agents waiting straight, what's held of his straight lane
        // comes from them, past their part of the floor: they borrow it back
        // while he leaves it. The first bound keeps agents their whole
        // quarter when the VPN's share stands up to a twentieth under what it
        // was asked, the second agents straight their part of it; neither is
        // under nothing, as Joel's two lanes fit in the room and the VPN is
        // held above.
        let spare =
            (room - straight_now - joel_vpn).min(rate - straight_now - vpn - straight_floor);
        let joel = straight_now + extra.min(spare);
        // Only a real second is stored, as the VPN's share is: a moment at a
        // higher rate gives his lane more, but holding it would let the next
        // dip take it from the VPN at the rate it falls back to. With no
        // agent waiting nothing is held from anyone, and the VPN fills what
        // he leaves, so the hold is stored as it falls rather than as what
        // he carried: agents who start waiting the second after find it.
        if dt > 0.0 {
            let held = if beside || within {
                joel
            } else {
                straight_now + extra
            };
            self.joel = held / rate;
        }
        // Held for the VPN's wait while they wait, agents straight keep a
        // share of their split, never under their part of the floor, and
        // borrow nothing past it. The VPN's own CAKE holds it to V, so
        // only Joel, who borrows first, may send what they give up; else
        // it goes unsent, and the queue past us the VPN's flow waits in
        // drains. A hold climbing back while they don't wait caps nothing,
        // or an upload starting then would get the least a class has.
        let agents = rate - joel - vpn;
        let agents_most = seen
            .agents_held
            .filter(|_| beside)
            .map(|share| (agents * share).max(straight_floor).min(agents));
        Split {
            joel,
            vpn,
            agents: agents_most.unwrap_or(agents),
            agents_most,
            unsent: false,
        }
    }

    /// What Joel's straight lane is held at, at `rate`, with `given` his
    /// due now: at once when that rises past what it was given at the last
    /// real second, and down by a fifth for each second after, so a gap of
    /// several seconds takes as much as those seconds would have.
    fn joel_held(&self, given: f64, rate: f64, dt: f64) -> f64 {
        let keep = (1.0 - FALL).powf(dt.max(0.0));
        given.max(self.joel * rate * keep)
    }

    /// The VPN's rate, moving towards `wanted` kbit/s: at once when it
    /// rises past a twentieth or Joel's own part, `joel`, passes what's
    /// held; by at most a fifth a second when it falls past a twentieth.
    /// What it keeps is split's to store, after the cap.
    fn hold(&self, wanted: f64, joel: f64, rate: f64, dt: f64) -> f64 {
        let wanted = wanted / rate;
        let share = match self.share {
            Some(held) if joel / rate <= held => {
                let back = self.held_back && wanted > held;
                if (wanted - held).abs() < held * STILL && !back {
                    held
                } else if wanted > held {
                    wanted
                } else {
                    wanted.max(held * (1.0 - FALL * dt.clamp(0.0, 1.0)))
                }
            }
            _ => wanted,
        };
        share * rate
    }
}

/// What Joel needs on a lane past what he carried there, in kbit/s.
fn need(carried: f64) -> f64 {
    carried * HEADROOM + JOEL_LEAST
}

/// Agents' lane's class, under which their CAKE stands.
pub const AGENTS_CLASS: &str = "1:5";

fn cmd(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}

/// The connection's rate on 1:2, and the three classes under it at
/// `split`, as tc commands that `verb` each class: replace or change.
fn classes(verb: &str, dev: &str, rate: u64, split: &Split) -> Vec<Vec<String>> {
    let class = |parent: &str, id: &str, (kbit, ceil): (u64, u64), prio: u8| {
        cmd(&format!(
            "tc class {verb} dev {dev} parent {parent} classid {id} htb \
             rate {kbit}kbit ceil {ceil}kbit prio {prio} quantum {QUANTUM}"
        ))
    };
    let kbit = |share: f64| whole(share).max(LEAST_KBIT);
    let agents = kbit(split.agents);
    vec![
        class("1:", "1:2", (rate, rate), 0),
        class("1:2", "1:3", (kbit(split.joel), rate), 0),
        class("1:2", "1:4", (kbit(split.vpn), rate), 0),
        class(
            "1:2",
            AGENTS_CLASS,
            (agents, split.agents_most.map_or(rate, kbit)),
            1,
        ),
    ]
}

/// The lanes under a connection's root HTB, whose default is 1:3: Joel's
/// CAKE keeps CAKE's handle, 2:, so the priority nftables gives a packet
/// names its tin there, as on a VPN, whose CAKE is its root. Agents' bulk,
/// 2:1, is sent to theirs by a filter. HTB puts the VPN's packets, given
/// 1:4, in its class itself. Every command replaces, and the filter has a
/// handle, so running them again makes nothing twice.
pub fn tree(dev: &str, rate: u64, split: &Split) -> Vec<Vec<String>> {
    let mut all = classes("replace", dev, rate, split);
    all.extend([
        cmd(&format!(
            "tc qdisc replace dev {dev} parent 1:3 handle 2: cake diffserv4 nat ack-filter"
        )),
        cmd(&format!(
            "tc qdisc replace dev {dev} parent 1:4 handle 4: fq_codel"
        )),
        cmd(&format!(
            "tc qdisc replace dev {dev} parent 1:5 handle 5: cake besteffort nat ack-filter"
        )),
        cmd(&format!(
            "tc filter replace dev {dev} parent 1: protocol all prio 1 handle 1 \
             basic match meta(priority eq 0x20001) classid 1:5"
        )),
    ]);
    all
}

/// New rates for the connection and its lanes.
pub fn change(dev: &str, rate: u64, split: &Split) -> Vec<Vec<String>> {
    classes("change", dev, rate, split)
}

/// What Joel's CAKE is told: a rate, or none at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cake {
    Unlimited,
    At(u64),
}

/// Tells Joel's CAKE the rate `joel_cake` gives it, or that it has none.
///
/// A CAKE with a rate paces what it sends, so HTB, which may burst past
/// it, now and then finds it holding packets it won't yet give up. The
/// kernel then logs once per queue `htb: cake qdisc 2: is
/// non-work-conserving?`, which is expected: CAKE's own timer sends them.
///
/// While a limit holds the upload, it's the most HTB lets his class
/// borrow, so CAKE keeps diffserv4's priorities while HTB still holds the
/// class to what the agents and the VPN leave. CAKE with no rate of its
/// own can't order its tins by when each is due, and takes turns by
/// weight instead, best effort's twice
/// video's and four times voice's: whenever Joel's class was full, his own
/// uploads over TCP, a file or a push, took two thirds of it from his call.
/// In the call test, with the upload held at 6 Mb/s and an app uploading a
/// file beside the call, his CAKE dropped 1,548 of the video call's
/// datagrams.
///
/// Video is due up to half the connection's rate and voice up to a
/// quarter, so while the others hold over half of it, what goes as video
/// can take all his class has, and what goes best effort gets only what it
/// leaves: an app's UDP other than to 443, and while a call is on all of
/// it, QUIC's too (shaper.nft). His call ahead of his pages is what this is
/// for. A switch restarts the shaper with its queues, and a shaper from
/// before this change, which never tells the CAKE a rate, would keep this
/// one, since a replace changes only what it names. So a shaper stopping
/// on SIGTERM tells each CAKE it has none first (`unshape_joel` in
/// main.rs); only one killed outright leaves the rate, and stopping the
/// shaper and starting it again then builds the queues anew.
pub fn joel_rate(dev: &str, cake: Cake) -> Vec<String> {
    let rate = match cake {
        Cake::Unlimited => "unlimited".to_owned(),
        Cake::At(rate) => format!("bandwidth {rate}kbit"),
    };
    cmd(&format!(
        "tc qdisc change dev {dev} parent 1:3 handle 2: cake {rate}"
    ))
}

/// The rate Joel's CAKE is given: the `classes`' while the upload is held
/// `up` under its ceiling, `most`, and none while nothing limits it. Only
/// a limit fills his class, which is when its tins need ordering; at a
/// rate the connection never reaches, CAKE shapes nothing, yet on the
/// download one at the ceiling dropped most of a call's packets (see
/// `qdisc::unturn`), so a resting upload's CAKE stays unlimited, as it
/// was before it had a rate at all.
pub fn joel_cake(up: u64, most: u64, classes: u64) -> Cake {
    if up < most {
        Cake::At(classes)
    } else {
        Cake::Unlimited
    }
}

/// Takes back a link's lanes and its IFB as a restart left them: every
/// class, queue and filter `qdisc::install` puts on is replaced in place,
/// so a new version's layout lands, but not the two root HTBs. HTB
/// refuses a change to its root, and making one afresh drops everything
/// under it, so a root's own default stays as the old shaper made it.
pub fn adopt(link: &str, ifb: &str, (rate, split): (u64, &Split), down: u64) -> Vec<Vec<String>> {
    let root =
        |c: &Vec<String>| c.get(1).is_some_and(|w| w == "qdisc") && c.iter().any(|w| w == "root");
    crate::qdisc::install(link, ifb, (rate, split), down)
        .into_iter()
        .filter(|c| !root(c))
        .collect()
}

/// Whether a connection's queues, from `tc -j qdisc show dev`, hold the
/// lanes: Joel's CAKE, the VPN's class's queue and agents' CAKE.
pub fn laned(json: &str) -> bool {
    let Ok(serde_json::Value::Array(all)) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };
    let has = |kind: &str, parent: &str| {
        all.iter()
            .any(|q| q["kind"] == kind && q["parent"] == parent)
    };
    all.iter().any(|q| q["kind"] == "htb" && q["root"] == true)
        && has("cake", "1:3")
        && has("fq_codel", "1:4")
        && has("cake", AGENTS_CLASS)
}

/// Takes every filter off the queues `json` lists on `dev` but its ingress,
/// so adopting leaves only the filters this version puts on: replacing a
/// queue with one of its own kind keeps whatever filters were under it.
pub fn unfiltered(dev: &str, json: &str) -> Vec<Vec<String>> {
    let Ok(serde_json::Value::Array(all)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    all.iter()
        .filter_map(|q| q["handle"].as_str())
        .filter(|handle| *handle != "ffff:")
        .map(|handle| cmd(&format!("tc filter del dev {dev} parent {handle}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 10_000.0;

    fn waiting(flows: u32) -> Agents {
        Agents {
            backlog: 30_000,
            flows,
            dropped: None,
        }
    }

    /// A copy starved to a window of two packets: it holds too little to
    /// count as waiting, two segments of 1,448 bytes, and its CAKE has
    /// dropped `dropped` so far.
    fn starved(dropped: u64) -> Agents {
        Agents {
            backlog: 2 * 1_448,
            flows: 2,
            dropped: Some(dropped),
        }
    }

    fn seen(joel: (f64, f64), agents: (Agents, Agents)) -> Seen {
        Seen {
            rate: RATE,
            joel_straight: joel.0,
            joel_vpn: joel.1,
            agents_straight: agents.0,
            agents_vpn: agents.1,
            agents_held: None,
        }
    }

    fn near(got: f64, want: f64) -> bool {
        (got - want).abs() < 1.0
    }

    fn adds_up(split: Split, rate: f64) -> bool {
        near(split.joel + split.vpn + split.agents, rate)
            && split.joel >= 0.0
            && split.vpn >= 0.0
            && split.agents >= 0.0
    }

    #[test]
    fn a_split_says_each_lane_s_rate_and_the_agents_flows_waiting_on_each() {
        let mut lanes = Lanes::default();
        let split = lanes.split(&seen((200.0, 0.0), (waiting(3), waiting(12))), 1.0);
        let said = lanes.said(&split);
        assert_eq!(
            said,
            Some(Said {
                joel: whole(split.joel),
                vpn: whole(split.vpn),
                agents: whole(split.agents),
                waiting_vpn: 12,
                waiting_direct: 3,
            }),
            "from {split:?}"
        );
    }

    #[test]
    fn agents_holding_too_little_to_wait_are_said_to_wait_on_nothing() {
        let mut lanes = Lanes::default();
        let few = Agents {
            backlog: WAITING_BYTES,
            flows: 4,
            dropped: None,
        };
        let split = lanes.split(&seen((0.0, 0.0), (few, few)), 1.0);
        let said = lanes.said(&split).expect("a split was said");
        assert_eq!(
            (said.waiting_vpn, said.waiting_direct),
            (0, 0),
            "agents under {WAITING_BYTES} bytes were said to wait"
        );
    }

    #[test]
    fn nothing_is_said_before_the_lanes_are_split() {
        assert_eq!(
            Lanes::default().said(&Split::default()),
            None,
            "an unsplit upload was said to have lanes"
        );
    }

    #[test]
    fn joel_alone_leaves_the_vpn_all_but_his_straight_room() {
        let split = Lanes::default().split(&seen((200.0, 500.0), Default::default()), 1.0);
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        assert!(
            near(split.joel, need(200.0)),
            "Joel's straight class held {}",
            split.joel
        );
        assert!(
            near(split.vpn, RATE - need(200.0)),
            "the VPN held {}",
            split.vpn
        );
    }

    #[test]
    fn agents_waiting_inside_the_vpn_alone_get_all_of_it_joel_leaves() {
        let split =
            Lanes::default().split(&seen((0.0, 0.0), (Agents::default(), waiting(30))), 1.0);
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        assert!(
            near(split.agents, 0.0),
            "straight agents held {}",
            split.agents
        );
        assert!(
            near(split.vpn, RATE - JOEL_LEAST),
            "the VPN held {}",
            split.vpn
        );
    }

    #[test]
    fn agents_waiting_straight_alone_get_all_of_it_joel_leaves() {
        let split = Lanes::default().split(&seen((0.0, 0.0), (waiting(4), Agents::default())), 1.0);
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        assert!(
            near(split.agents, RATE - 2.0 * JOEL_LEAST),
            "straight agents held {}",
            split.agents
        );
        assert!(near(split.vpn, JOEL_LEAST), "the VPN held {}", split.vpn);
    }

    #[test]
    fn agents_waiting_on_both_lanes_share_them_evenly_whatever_their_flows() {
        // As on 16 November at 20:25: 117 flows to the model inside the
        // VPN, and two copies straight to the builders.
        let split = Lanes::default().split(&seen((0.0, 0.0), (waiting(2), waiting(117))), 1.0);
        let agents = RATE - 2.0 * JOEL_LEAST;
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        assert!(
            near(split.agents, agents / 2.0),
            "straight agents with 2 flows of 119 held {}",
            split.agents
        );
        assert!(
            near(split.vpn, JOEL_LEAST + agents / 2.0),
            "the VPN held {}",
            split.vpn
        );
    }

    #[test]
    fn a_starved_copy_whose_cake_drops_waits_though_it_holds_little() {
        let mut lanes = Lanes::default();
        let inside = waiting(117);
        lanes.split(&seen((0.0, 0.0), (starved(100), inside)), 1.0);
        let split = lanes.split(&seen((0.0, 0.0), (starved(103), inside)), 1.0);
        let agents = RATE - 2.0 * JOEL_LEAST;
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        // The VPN's share falls a fifth a second, so its half comes later;
        // agents straight get at least what it gives up at once.
        assert!(
            split.agents >= (RATE - JOEL_LEAST) * FALL - 1.0,
            "a dropping copy held {} of {agents}",
            split.agents
        );
        for dropped in 104..120 {
            lanes.split(&seen((0.0, 0.0), (starved(dropped), inside)), 1.0);
        }
        let settled = lanes.split(&seen((0.0, 0.0), (starved(121), inside)), 1.0);
        assert!(
            near(settled.agents, agents / 2.0),
            "a dropping copy settled at {} of {agents}",
            settled.agents
        );
        assert_eq!(
            lanes.said(&settled).map(|s| s.waiting_direct),
            Some(2),
            "the dropping copy's flows weren't said to wait"
        );
    }

    #[test]
    fn a_copy_keeps_its_half_through_a_quiet_stretch_and_then_gives_it_back() {
        let mut lanes = Lanes::default();
        let inside = waiting(117);
        lanes.split(&seen((0.0, 0.0), (starved(100), inside)), 1.0);
        let mut split = Split::default();
        // It drops once, then holds its half without another drop for 19 s.
        for _ in 0..20 {
            split = lanes.split(&seen((0.0, 0.0), (starved(101), inside)), 1.0);
        }
        let agents = RATE - 2.0 * JOEL_LEAST;
        assert!(
            near(split.agents, agents / 2.0),
            "a copy quiet for {} s of {PRESSED_S} held {}",
            19,
            split.agents
        );
        let after = lanes.split(&seen((0.0, 0.0), (starved(101), inside)), 1.0);
        assert!(
            near(after.agents, 0.0),
            "a copy quiet for {PRESSED_S} s still held {}",
            after.agents
        );
    }

    #[test]
    fn agents_inside_the_vpn_whose_cake_drops_wait_though_they_hold_little() {
        let mut lanes = Lanes::default();
        let straight = waiting(4);
        lanes.split(&seen((0.0, 0.0), (straight, starved(100))), 1.0);
        let split = lanes.split(&seen((0.0, 0.0), (straight, starved(101))), 1.0);
        let agents = RATE - 2.0 * JOEL_LEAST;
        assert!(
            near(split.vpn, JOEL_LEAST + agents / 2.0),
            "dropping agents inside the VPN held {}",
            split.vpn
        );
    }

    #[test]
    fn a_count_after_one_that_couldn_t_be_read_is_no_drop() {
        let mut lanes = Lanes::default();
        let inside = waiting(30);
        let unread = Agents {
            dropped: None,
            ..starved(0)
        };
        lanes.split(&seen((0.0, 0.0), (starved(100), inside)), 1.0);
        lanes.split(&seen((0.0, 0.0), (unread, inside)), 1.0);
        let split = lanes.split(&seen((0.0, 0.0), (starved(70_000), inside)), 1.0);
        assert!(
            near(split.agents, 0.0),
            "a count after a missed read held {}",
            split.agents
        );
    }

    #[test]
    fn a_cake_made_again_counting_from_nothing_is_no_drop() {
        let mut lanes = Lanes::default();
        let inside = waiting(30);
        lanes.split(&seen((0.0, 0.0), (starved(70_000), inside)), 1.0);
        let split = lanes.split(&seen((0.0, 0.0), (starved(3), inside)), 1.0);
        assert!(
            near(split.agents, 0.0),
            "a fresh CAKE's count held {}",
            split.agents
        );
    }

    #[test]
    fn a_copy_holding_little_whose_cake_drops_nothing_doesn_t_wait() {
        let mut lanes = Lanes::default();
        lanes.split(&seen((0.0, 0.0), (starved(100), waiting(30))), 1.0);
        let split = lanes.split(&seen((0.0, 0.0), (starved(100), waiting(30))), 1.0);
        assert!(
            near(split.agents, 0.0),
            "an easy copy held {}",
            split.agents
        );
    }

    #[test]
    fn the_first_count_of_drops_is_no_drop() {
        let split = Lanes::default().split(&seen((0.0, 0.0), (starved(70_000), waiting(30))), 1.0);
        assert!(
            near(split.agents, 0.0),
            "drops before the shaper read any held {}",
            split.agents
        );
    }

    #[test]
    fn a_new_rate_between_seconds_keeps_what_was_pressed() {
        let mut lanes = Lanes::default();
        let inside = waiting(30);
        lanes.split(&seen((0.0, 0.0), (starved(100), inside)), 1.0);
        lanes.split(&seen((0.0, 0.0), (starved(105), inside)), 1.0);
        let half = lanes.at(RATE / 2.0);
        assert_eq!(
            lanes.said(&half).map(|s| s.waiting_direct),
            Some(2),
            "a new rate forgot the dropping copy: {half:?}"
        );
        assert!(
            half.agents > 0.0,
            "a new rate gave a dropping copy nothing: {half:?}"
        );
    }

    #[test]
    fn joel_busy_beside_waiting_agents_leaves_them_a_quarter() {
        let split =
            Lanes::default().split(&seen((3_000.0, 8_000.0), (waiting(10), waiting(10))), 1.0);
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        // Joel needs more than the three quarters he may take, so both his
        // parts shrink alike; agents keep their quarter, split evenly by
        // their flows.
        let (joel, wants) = (RATE * (1.0 - AGENTS_FLOOR), need(3_000.0) + need(8_000.0));
        assert!(
            near(split.joel, joel * need(3_000.0) / wants),
            "Joel's straight class held {}",
            split.joel
        );
        assert!(
            near(split.agents, RATE * AGENTS_FLOOR / 2.0),
            "straight agents held {}",
            split.agents
        );
        assert!(
            near(
                split.vpn,
                joel * need(8_000.0) / wants + RATE * AGENTS_FLOOR / 2.0
            ),
            "the VPN held {}",
            split.vpn
        );
    }

    // On 19 November the hidden-queue test's controller climbed to 8,557
    // kbit/s past its router's 8,000, whose queue gave Joel's upload a
    // 33rd of the line for a second. His lane, following that second, fell
    // to 1,259 and took about four seconds to grow back beside 32 agents.
    #[test]
    fn joel_s_lane_falls_by_a_fifth_a_second_and_rises_at_once() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), Agents::default());
        // Under the three quarters he may take beside waiting agents.
        let busy = lanes.split(&seen((4_000.0, 0.0), agents), 1.0).joel;
        assert!(near(busy, need(4_000.0)), "Joel busy held {busy}");
        let dipped = lanes.split(&seen((200.0, 0.0), agents), 1.0).joel;
        assert!(
            near(dipped, busy * (1.0 - FALL)),
            "a second's dip took Joel's lane from {busy} to {dipped}"
        );
        // Held as what a real second gave him: a smaller rate between
        // seconds stores nothing.
        lanes.at(RATE / 2.0);
        assert!(
            near(lanes.at(RATE).joel, dipped),
            "a smaller rate between seconds kept less of Joel's lane"
        );
        let back = lanes.split(&seen((4_000.0, 0.0), agents), 1.0).joel;
        assert!(
            near(back, need(4_000.0)),
            "didn't rise back at once: {back}"
        );
        let mut joel = back;
        for _ in 0..20 {
            let next = lanes.split(&seen((0.0, 0.0), agents), 1.0).joel;
            let fell = (joel * (1.0 - FALL)).max(need(0.0));
            assert!(near(next, fell), "fell from {joel} to {next}, not {fell}");
            joel = next;
        }
        assert!(
            near(joel, need(0.0)),
            "settled at {joel}, not {}",
            need(0.0)
        );
    }

    #[test]
    fn joel_s_lane_falls_a_fifth_for_each_second_of_a_gap() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), Agents::default());
        let busy = lanes.split(&seen((4_000.0, 0.0), agents), 1.0).joel;
        let after = lanes.split(&seen((0.0, 0.0), agents), 3.0).joel;
        assert!(
            near(after, busy * (1.0 - FALL).powi(3)),
            "three seconds took Joel's lane from {busy} to {after}"
        );
    }

    // The running shaper never takes its rate under its floor, so this is
    // the type's own promise, as it always was for the VPN's share.
    #[test]
    fn a_rate_of_zero_forgets_what_joel_held() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), Agents::default());
        lanes.split(&seen((4_000.0, 0.0), agents), 1.0);
        let gone = Seen {
            rate: 0.0,
            ..seen((4_000.0, 0.0), agents)
        };
        lanes.split(&gone, 1.0);
        let after = lanes.split(&seen((0.0, 0.0), agents), 1.0).joel;
        assert!(
            near(after, need(0.0)),
            "a rate of zero still held {after} of Joel's lane"
        );
    }

    #[test]
    fn a_lane_joel_left_doesn_t_squeeze_the_one_he_moved_to() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), Agents::default());
        lanes.split(&seen((4_000.0, 0.0), agents), 1.0);
        let moved = lanes.split(&seen((0.0, 4_000.0), agents), 1.0);
        assert!(
            adds_up(moved, RATE),
            "the classes don't add up to the rate: {moved:?}"
        );
        assert!(
            near(moved.vpn, need(4_000.0)),
            "the lane he moved to got {} of the {} he needs",
            moved.vpn,
            need(4_000.0)
        );
        // What's held of the lane he left past need(0) comes only from
        // agents straight past their floor.
        let spare = RATE * (1.0 - AGENTS_FLOOR) - need(0.0) - need(4_000.0);
        assert!(
            near(moved.joel, need(0.0) + spare),
            "the lane he left held {}",
            moved.joel
        );
    }

    // Agents' uploads are mostly inside the VPN: on 16 November at 20:25
    // they held 117 flows to the model there. A second's dip beside them
    // alone must leave Joel's straight lane held as beside agents straight,
    // with the VPN's agents keeping their quarter, and the VPN rising back
    // as his hold falls.
    #[test]
    fn joel_s_hold_beside_agents_only_inside_the_vpn_comes_from_them() {
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        let busy = lanes.split(&seen((4_000.0, 0.0), agents), 1.0);
        assert!(
            near(busy.joel, need(4_000.0)),
            "Joel busy held {}",
            busy.joel
        );
        let dipped = lanes.split(&seen((200.0, 0.0), agents), 1.0);
        assert!(
            adds_up(dipped, RATE),
            "the classes don't add up to the rate: {dipped:?}"
        );
        assert!(
            near(dipped.joel, busy.joel * (1.0 - FALL)),
            "a second's dip beside agents only inside the VPN took Joel's lane from {} to {}",
            busy.joel,
            dipped.joel
        );
        assert!(
            near(dipped.vpn, RATE - dipped.joel),
            "Joel's hold came from elsewhere than the VPN: {dipped:?}"
        );
        // Past the band its share otherwise stays still inside, a twentieth.
        let mut quiet = dipped;
        for _ in 0..20 {
            quiet = lanes.split(&seen((0.0, 0.0), agents), 1.0);
        }
        assert!(
            near(quiet.vpn, RATE - need(0.0)),
            "the VPN came back to {}, not {}, once Joel's hold fell",
            quiet.vpn,
            RATE - need(0.0)
        );
    }

    // A rate moved between seconds, or a gap of several, holds Joel's lane
    // beside agents only inside the VPN as it does beside agents straight.
    #[test]
    fn joel_s_hold_from_the_vpn_keeps_through_a_new_rate_and_a_gap() {
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        let busy = lanes.split(&seen((4_000.0, 0.0), agents), 1.0).joel;
        let dipped = lanes.split(&seen((200.0, 0.0), agents), 1.0).joel;
        let half = lanes.at(RATE / 2.0);
        assert!(
            adds_up(half, RATE / 2.0) && half.vpn - need(0.0) >= RATE / 2.0 * AGENTS_FLOOR - 1.0,
            "at half the rate Joel's hold left the VPN's agents less than their quarter: {half:?}"
        );
        let back = lanes.at(RATE);
        assert!(
            near(back.joel, dipped),
            "a smaller rate between seconds left Joel's lane {}, not {dipped}",
            back.joel
        );
        let after = lanes.split(&seen((0.0, 0.0), agents), 3.0).joel;
        assert!(
            near(after, busy * (1.0 - FALL).powi(4)),
            "three seconds took Joel's lane from {dipped} to {after}"
        );
    }

    // What's held of his straight lane can pass what the VPN's agents have
    // past their quarter, as when he moves his upload into the VPN, and then
    // they keep the quarter.
    #[test]
    fn joel_s_hold_from_the_vpn_leaves_its_agents_their_quarter() {
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        lanes.split(&seen((20_000.0, 0.0), agents), 1.0);
        let moved = lanes.split(&seen((0.0, 4_000.0), agents), 1.0);
        assert!(
            adds_up(moved, RATE),
            "the classes don't add up to the rate: {moved:?}"
        );
        assert!(
            near(moved.vpn - need(4_000.0), RATE * AGENTS_FLOOR),
            "Joel's hold left the VPN's agents {}, not their {}",
            moved.vpn - need(4_000.0),
            RATE * AGENTS_FLOOR
        );
    }

    // Busy on both lanes, Joel's straight lane gets what's left of the room
    // beside his part in the VPN; a dip holds that, not what he'd have
    // wanted, so the VPN falls no faster than a fifth a second.
    #[test]
    fn a_dip_on_both_lanes_holds_what_joel_s_straight_lane_was_given() {
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        let busy = lanes.split(&seen((6_000.0, 4_000.0), agents), 1.0);
        let dipped = lanes.split(&seen((200.0, 200.0), agents), 1.0);
        assert!(
            adds_up(dipped, RATE),
            "the classes don't add up to the rate: {dipped:?}"
        );
        assert!(
            dipped.vpn >= busy.vpn * (1.0 - FALL) - 1.0,
            "a dip took the VPN, Joel's own lane too, from {} to {}",
            busy.vpn,
            dipped.vpn
        );
        assert!(
            near(dipped.joel, busy.joel * (1.0 - FALL)),
            "a dip held Joel's straight lane at {}, not a fifth under the {} it was given",
            dipped.joel,
            busy.joel
        );
    }

    // The controller moves its rate between seconds; a moment at a higher
    // rate gives his straight lane more, but holds none of it.
    // With no agent waiting nothing is held from anyone, so a dip then
    // leaves the hold falling by a fifth, for agents who start waiting the
    // second after to find.
    #[test]
    fn a_second_with_no_agent_waiting_leaves_joel_s_hold_falling() {
        let mut lanes = Lanes::default();
        let none = (Agents::default(), Agents::default());
        lanes.split(&seen((4_000.0, 0.0), none), 1.0);
        lanes.split(&seen((200.0, 0.0), none), 1.0);
        let fell = need(4_000.0) * (1.0 - FALL);
        assert!(
            near(lanes.joel * RATE, fell),
            "a second with no agent waiting took Joel's hold from {fell} to {}",
            lanes.joel * RATE
        );
    }

    // Moved into the VPN, Joel's straight lane is given only his least, so
    // that's what's held of it: a stall in his VPN upload the second after
    // finds the VPN falling no faster than a fifth.
    #[test]
    fn a_dip_after_moving_into_the_vpn_cuts_it_by_at_most_a_fifth() {
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        lanes.split(&seen((20_000.0, 0.0), agents), 1.0);
        let moved = lanes.split(&seen((0.0, 8_000.0), agents), 1.0);
        let stalled = lanes.split(&seen((0.0, 2_000.0), agents), 1.0);
        assert!(
            stalled.vpn >= moved.vpn * (1.0 - FALL) - 1.0,
            "a stall the second after Joel moved into the VPN took it from {} to {}",
            moved.vpn,
            stalled.vpn
        );
    }

    // What's held of his straight lane is a share of the rate, as the VPN's
    // is, so a cut takes both down alike rather than the VPN first.
    #[test]
    fn a_cut_takes_joel_s_hold_down_with_the_rate() {
        let fast = 10.0 * RATE;
        let mut lanes = Lanes::default();
        let agents = (Agents::default(), waiting(30));
        let at = |joel| Seen {
            rate: fast,
            ..seen(joel, agents)
        };
        lanes.split(&at((50_000.0, 50_000.0)), 1.0);
        let dipped = lanes.split(&at((200.0, 200.0)), 1.0);
        let cut = lanes.at(fast / 2.0);
        assert!(
            cut.vpn / (fast / 2.0) >= dipped.vpn / fast * (1.0 - FALL),
            "halving the rate took the VPN's share from {} to {}",
            dipped.vpn / fast,
            cut.vpn / (fast / 2.0)
        );
    }

    // Once agents wait straight too, what's held of his straight lane comes
    // from them, who borrow it back, and the VPN keeps its half.
    #[test]
    fn with_agents_straight_too_joel_s_hold_leaves_the_vpn_its_half() {
        let mut lanes = Lanes::default();
        lanes.split(
            &seen((20_000.0, 0.0), (Agents::default(), waiting(30))),
            1.0,
        );
        let both = lanes.split(&seen((0.0, 0.0), (waiting(32), waiting(30))), 1.0);
        let half = (RATE - 2.0 * need(0.0)) / 2.0 + need(0.0);
        assert!(
            near(both.vpn, half),
            "the VPN got {}, not its {half}, with agents waiting straight to hold Joel's lane",
            both.vpn
        );
    }

    // Agents beginning to wait straight take the hold over from the VPN,
    // whose share falls from what it had, not from what it kept for itself.
    #[test]
    fn joel_s_hold_from_the_vpn_passes_to_agents_who_wait_straight() {
        let mut lanes = Lanes::default();
        let inside = (Agents::default(), waiting(30));
        let busy = lanes.split(&seen((4_000.0, 0.0), inside), 1.0).joel;
        lanes.split(&seen((200.0, 0.0), inside), 1.0);
        let both = lanes.split(&seen((0.0, 0.0), (waiting(32), waiting(30))), 1.0);
        let held = busy * (1.0 - FALL).powi(2);
        assert!(
            adds_up(both, RATE),
            "the classes don't add up to the rate: {both:?}"
        );
        assert!(
            near(both.joel, held),
            "agents beginning to wait straight took Joel's lane to {}, not {held}",
            both.joel
        );
    }

    // With no agent waiting, nothing is held from the VPN: the lane Joel
    // left mustn't squeeze the one he moved to.
    #[test]
    fn with_no_agent_waiting_joel_s_hold_takes_nothing_from_the_vpn() {
        let mut lanes = Lanes::default();
        lanes.split(&seen((4_000.0, 0.0), Default::default()), 1.0);
        let moved = lanes.split(&seen((0.0, 4_000.0), Default::default()), 1.0);
        assert!(
            near(moved.vpn, RATE - need(0.0)),
            "the lane he moved to got {}, not the {} his straight lane leaves",
            moved.vpn,
            RATE - need(0.0)
        );
    }

    // The VPN's share falls slowly too, so what both hold could leave the
    // agents waiting straight less than their quarter.
    #[test]
    fn both_holds_leave_agents_straight_their_floor() {
        let mut lanes = Lanes::default();
        lanes.split(&seen((4_000.0, 0.0), (Agents::default(), waiting(30))), 1.0);
        let moved = lanes.split(&seen((0.0, 0.0), (waiting(32), Agents::default())), 1.0);
        assert!(
            adds_up(moved, RATE),
            "the classes don't add up to the rate: {moved:?}"
        );
        assert!(
            moved.agents >= RATE * AGENTS_FLOOR - 1.0,
            "agents straight got {} of their {} with both holds on",
            moved.agents,
            RATE * AGENTS_FLOOR
        );
        assert!(
            moved.joel > need(0.0) + 1.0,
            "Joel's lane kept nothing of its hold: {}",
            moved.joel
        );
    }

    // Joel moving from the VPN to straight leaves the VPN's share falling
    // from what he carried there: it mustn't take agents straight their
    // quarter meanwhile, nor leave Joel under what he carries.
    #[test]
    fn the_vpn_s_fall_leaves_agents_straight_their_floor() {
        let agents = (waiting(32), Agents::default());
        let mut lanes = Lanes::default();
        lanes.split(&seen((0.0, 5_000.0), agents), 1.0);
        let moved = lanes.split(&seen((5_000.0, 0.0), agents), 1.0);
        assert!(
            adds_up(moved, RATE),
            "the classes don't add up to the rate: {moved:?}"
        );
        assert!(
            moved.agents >= RATE * AGENTS_FLOOR - 1.0,
            "Joel moving straight left agents straight {} of their {}",
            moved.agents,
            RATE * AGENTS_FLOOR
        );
        let room = RATE * (1.0 - AGENTS_FLOOR);
        let carries = need(5_000.0) * room / (need(5_000.0) + need(0.0));
        assert!(
            near(moved.joel, carries),
            "Joel moving straight got {} of the {carries} he carries",
            moved.joel
        );
        // The VPN's share falls from what the cap left it, so a dip the
        // second after doesn't hand it back the room held for Joel, which
        // falls from what his lane was given.
        let dip = lanes.split(&seen((0.0, 0.0), agents), 1.0);
        let held = carries * (1.0 - FALL);
        assert!(
            near(dip.joel, held),
            "a dip after moving from the VPN took Joel's lane to {}, not {held}",
            dip.joel
        );
        let mut lanes = Lanes::default();
        lanes.split(&seen((0.0, 0.0), (Agents::default(), waiting(30))), 1.0);
        let quiet = lanes.split(&seen((0.0, 0.0), agents), 1.0);
        assert!(
            near(quiet.joel, need(0.0)),
            "the VPN's agents giving way left quiet Joel {}, not {}",
            quiet.joel,
            need(0.0)
        );
        assert!(
            quiet.agents >= RATE * AGENTS_FLOOR - 1.0,
            "the VPN's agents giving way left agents straight {} of their {}",
            quiet.agents,
            RATE * AGENTS_FLOOR
        );
    }

    // The VPN's share stays put inside a twentieth of what it's asked, so
    // the room past Joel's two lanes can be all there is: his hold mustn't
    // take the agents' quarter from it. The quarter is theirs between both
    // lanes: the still share leaves the VPN's agents a little under half of
    // it here, 1,050 of 1,250, and those straight the rest.
    #[test]
    fn a_still_vpn_share_leaves_agents_their_quarter() {
        let agents = (waiting(32), waiting(10));
        let mut lanes = Lanes::default();
        lanes.split(&seen((4_000.0, 8_000.0), agents), 1.0);
        let next = lanes.split(&seen((2_000.0, 5_000.0), agents), 1.0);
        let room = RATE * (1.0 - AGENTS_FLOOR);
        let joel_vpn = need(5_000.0) * room / (need(2_000.0) + need(5_000.0));
        let theirs = next.agents + next.vpn - joel_vpn;
        assert!(
            theirs >= RATE * AGENTS_FLOOR - 1.0,
            "agents on both lanes got {theirs} of their {}",
            RATE * AGENTS_FLOOR
        );
        let half = RATE * AGENTS_FLOOR / 2.0;
        assert!(
            next.agents >= half - 1.0,
            "agents straight got {} of their {half}",
            next.agents
        );
        assert!(
            next.vpn - joel_vpn >= half - next.vpn * STILL - 1.0,
            "the VPN's agents got {} of their {half}, short by more than the still share's \
             twentieth",
            next.vpn - joel_vpn
        );
    }

    // When the VPN's agents wait again after the cap trimmed its share, it
    // rises at once to give them their half, and agents straight keep theirs
    // beside it. That the trimmed share is the one stored is pinned by the
    // dip in the_vpn_s_fall_leaves_agents_straight_their_floor.
    #[test]
    fn a_capped_vpn_share_rises_when_its_agents_wait() {
        let straight = (waiting(32), Agents::default());
        let mut lanes = Lanes::default();
        lanes.split(&seen((0.0, 5_000.0), straight), 1.0);
        lanes.split(&seen((5_000.0, 0.0), straight), 1.0);
        let both = lanes.split(&seen((5_000.0, 0.0), (waiting(32), waiting(10))), 1.0);
        let room = RATE * (1.0 - AGENTS_FLOOR);
        let joel_vpn = need(0.0) * room / (need(5_000.0) + need(0.0));
        let half = RATE * AGENTS_FLOOR / 2.0;
        assert!(
            adds_up(both, RATE),
            "the classes don't add up to the rate: {both:?}"
        );
        assert!(
            both.vpn - joel_vpn >= half - 1.0,
            "the VPN's agents got {} of their {half} from a capped share",
            both.vpn - joel_vpn
        );
        assert!(
            both.agents >= half - 1.0,
            "agents straight got {} of their {half} beside a capped share",
            both.agents
        );
    }

    // The controller moves its rate between seconds, and the cap at that
    // rate alone mustn't end the VPN's slow fall, the only hold Joel's part
    // inside it gets.
    #[test]
    fn a_cut_between_seconds_leaves_the_vpn_s_held_share() {
        let straight = (waiting(32), Agents::default());
        let mut lanes = Lanes::default();
        lanes.split(&seen((0.0, 2_000.0), straight), 1.0);
        let held = lanes.split(&seen((3_000.0, 0.0), straight), 1.0);
        lanes.at(RATE * 0.8);
        let back = lanes.at(RATE);
        assert!(
            near(back.vpn, held.vpn),
            "a cut between seconds left the VPN {} of the {} it held",
            back.vpn,
            held.vpn
        );
    }

    // A cut still in place at a second's split stores the share that rate
    // gave, here raised, as Joel's part inside the VPN is a larger share of
    // less, and it comes back with the rate. This pins the share stored, as
    // the module doc says, where it says why it isn't kept in kbit/s.
    #[test]
    fn a_cut_held_across_a_second_keeps_the_share_it_stored() {
        let straight = (waiting(32), Agents::default());
        let mut lanes = Lanes::default();
        lanes.split(&seen((4_000.0, 0.0), straight), 1.0);
        lanes.split(&seen((200.0, 0.0), straight), 1.0);
        let cut_rate = RATE * 0.3;
        lanes.at(cut_rate);
        let cut = lanes.split(
            &Seen {
                rate: cut_rate,
                ..seen((200.0, 0.0), straight)
            },
            1.0,
        );
        let back = lanes.at(RATE);
        assert!(
            near(back.vpn, cut.vpn / cut_rate * RATE),
            "a cut held across a second came back with the VPN at {}, not the {} its share \
             at the cut gives",
            back.vpn,
            cut.vpn / cut_rate * RATE
        );
    }

    // Joel's part inside the VPN is a larger share of a smaller rate, so a
    // cut between seconds asks the VPN for more; kept, it would come back
    // with the rate and take the room held for him straight.
    #[test]
    fn a_cut_between_seconds_gives_back_what_it_raised() {
        let straight = (waiting(32), Agents::default());
        let mut lanes = Lanes::default();
        lanes.split(&seen((4_000.0, 0.0), straight), 1.0);
        let held = lanes.split(&seen((200.0, 0.0), straight), 1.0);
        lanes.at(RATE * 0.3);
        let back = lanes.at(RATE);
        assert!(
            near(back.vpn, held.vpn),
            "a cut between seconds came back with the VPN at {}, not {}",
            back.vpn,
            held.vpn
        );
        assert!(
            near(back.joel, held.joel),
            "a cut between seconds came back with Joel at {}, not {}",
            back.joel,
            held.joel
        );
    }

    // With agents on both lanes, those straight keep their half of the
    // quarter, and Joel's hold gets what they'd have past it.
    #[test]
    fn with_agents_on_both_lanes_joel_s_hold_leaves_those_straight_their_half() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), waiting(10));
        lanes.split(&seen((4_000.0, 0.0), agents), 1.0);
        let quiet = lanes.split(&seen((0.0, 0.0), agents), 1.0);
        let mut fresh = Lanes::default();
        fresh.split(&seen((0.0, 0.0), agents), 1.0);
        let unheld = fresh.split(&seen((0.0, 0.0), agents), 1.0);
        assert!(
            adds_up(quiet, RATE),
            "the classes don't add up to the rate: {quiet:?}"
        );
        assert!(
            near(quiet.vpn, unheld.vpn),
            "Joel's hold left the VPN {} of its {}",
            quiet.vpn,
            unheld.vpn
        );
        assert!(
            near(quiet.agents, RATE * AGENTS_FLOOR / 2.0),
            "agents straight got {} beside the VPN's agents, not half the quarter",
            quiet.agents
        );
        assert!(
            quiet.joel > need(0.0) + 1.0,
            "Joel's lane kept nothing of its hold: {}",
            quiet.joel
        );
    }

    // Joel's lane falls a fifth for each second of a gap, but the VPN's
    // share only once, as FALL says.
    #[test]
    fn the_vpn_s_share_falls_once_over_a_gap() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), waiting(10));
        let busy = lanes.split(&seen((0.0, 5_000.0), agents), 1.0);
        let after = lanes.split(&seen((0.0, 200.0), agents), 3.0);
        assert!(
            near(after.vpn, busy.vpn * (1.0 - FALL)),
            "three seconds took the VPN from {} to {}",
            busy.vpn,
            after.vpn
        );
    }

    #[test]
    fn joel_s_part_inside_the_vpn_falls_with_the_vpn_s_share_alone() {
        let mut lanes = Lanes::default();
        let agents = (waiting(32), waiting(10));
        let busy = lanes.split(&seen((0.0, 5_000.0), agents), 1.0);
        let room = RATE * (1.0 - AGENTS_FLOOR);
        let wants = need(0.0) + need(5_000.0);
        assert!(
            near(
                busy.vpn,
                room * need(5_000.0) / wants + RATE * AGENTS_FLOOR / 2.0
            ),
            "the VPN held {} with Joel busy inside it",
            busy.vpn
        );
        // The VPN's share already falls by a fifth a second, so his part
        // inside it isn't held a second time, which would keep it from
        // agents straight on the connection.
        let dipped = lanes.split(&seen((0.0, 200.0), agents), 1.0);
        assert!(
            near(dipped.vpn, busy.vpn * (1.0 - FALL)),
            "a second's dip took the VPN from {} to {}",
            busy.vpn,
            dipped.vpn
        );
    }

    #[test]
    fn joel_busy_with_no_agent_waiting_gets_everything_he_needs() {
        let split = Lanes::default().split(&seen((0.0, 8_000.0), Default::default()), 1.0);
        assert!(
            adds_up(split, RATE),
            "the classes don't add up to the rate: {split:?}"
        );
        // What he needs on both lanes passes the whole rate.
        assert!(
            near(
                split.vpn,
                RATE * need(8_000.0) / (need(0.0) + need(8_000.0))
            ),
            "the VPN held {}",
            split.vpn
        );
    }

    #[test]
    fn the_vpn_s_share_falls_by_a_fifth_a_second_and_rises_at_once() {
        let mut lanes = Lanes::default();
        let alone = seen((0.0, 0.0), (Agents::default(), waiting(30)));
        let both = seen((0.0, 0.0), (waiting(30), waiting(10)));
        let first = lanes.split(&alone, 1.0).vpn;
        let next = lanes.split(&both, 1.0);
        assert!(
            adds_up(next, RATE),
            "the classes don't add up to the rate: {next:?}"
        );
        assert!(
            near(next.vpn, first * (1.0 - FALL)),
            "fell from {first} to {}",
            next.vpn
        );
        let mut vpn = next.vpn;
        for _ in 0..10 {
            vpn = lanes.split(&both, 1.0).vpn;
        }
        let settled = JOEL_LEAST + (RATE - 2.0 * JOEL_LEAST) / 2.0;
        assert!(near(vpn, settled), "settled at {vpn}, not {settled}");
        assert!(
            near(lanes.split(&alone, 1.0).vpn, first),
            "didn't rise back at once"
        );
    }

    #[test]
    fn a_small_move_of_the_vpn_s_share_isn_t_made() {
        let mut lanes = Lanes::default();
        let first = lanes
            .split(&seen((0.0, 0.0), (waiting(30), waiting(30))), 1.0)
            .vpn;
        let next = lanes.split(&seen((0.0, 100.0), (waiting(30), waiting(30))), 1.0);
        assert!(
            adds_up(next, RATE),
            "the classes don't add up to the rate: {next:?}"
        );
        assert!(near(next.vpn, first), "moved from {first} to {}", next.vpn);
    }

    #[test]
    fn a_rate_cut_cuts_the_vpn_at_once() {
        let mut lanes = Lanes::default();
        let busy = seen((0.0, 0.0), (Agents::default(), waiting(30)));
        let first = lanes.split(&busy, 1.0).vpn;
        let cut = lanes.split(
            &Seen {
                rate: RATE / 2.0,
                ..busy
            },
            1.0,
        );
        assert!(
            adds_up(cut, RATE / 2.0),
            "the classes don't add up to the rate: {cut:?}"
        );
        assert!(
            cut.vpn <= first / 2.0 + 1.0,
            "the VPN held {} after the cut",
            cut.vpn
        );
    }

    #[test]
    fn the_classes_add_up_to_the_rate_on_a_line_too_small_for_joel_s_least() {
        let split = Lanes::default().split(
            &Seen {
                rate: 1_500.0,
                ..seen((0.0, 0.0), (waiting(1), waiting(1)))
            },
            1.0,
        );
        assert!(
            adds_up(split, 1_500.0),
            "the classes don't add up to the rate: {split:?}"
        );
    }

    #[test]
    fn joel_s_burst_inside_the_vpn_raises_it_at_once() {
        let mut lanes = Lanes::default();
        let low = lanes
            .split(&seen((0.0, 0.0), (waiting(30), waiting(1))), 1.0)
            .vpn;
        let burst = lanes.split(&seen((0.0, 4_000.0), (waiting(30), waiting(1))), 1.0);
        assert!(
            burst.vpn >= need(4_000.0),
            "the VPN held {} from {low}",
            burst.vpn
        );
    }

    #[test]
    fn a_new_rate_keeps_the_vpn_s_share_of_it() {
        let mut lanes = Lanes::default();
        let first = lanes.split(&seen((0.0, 0.0), (waiting(10), waiting(30))), 1.0);
        let half = lanes.at(RATE / 2.0);
        assert!(
            adds_up(half, RATE / 2.0),
            "the classes don't add up to the rate: {half:?}"
        );
        assert!(
            near(half.vpn, first.vpn / 2.0),
            "the VPN held {} of {}",
            half.vpn,
            first.vpn
        );
    }

    #[test]
    fn a_split_moves_past_a_hundredth_of_the_rate() {
        let was = Split {
            joel: 1_000.0,
            vpn: 8_000.0,
            agents: 1_000.0,
            agents_most: None,
            unsent: false,
        };
        let little = Split {
            vpn: 8_050.0,
            agents: 950.0,
            ..was
        };
        let more = Split {
            vpn: 8_200.0,
            agents: 800.0,
            ..was
        };
        assert!(
            !little.moved_from(&was, RATE),
            "half a hundredth was a move"
        );
        assert!(more.moved_from(&was, RATE), "two hundredths weren't a move");
        let held = Split {
            agents_most: Some(1_000.0),
            ..was
        };
        assert!(
            held.moved_from(&was, RATE),
            "holding agents at the same rate wasn't a move"
        );
    }

    // A change that partly failed may have capped agents' class, or left
    // its cap, so the cap is kept as standing, and both a split without
    // one and letting agents go send the classes again.
    #[test]
    fn a_cap_a_failed_change_may_have_left_is_kept_as_standing() {
        let free = Split {
            joel: 1_000.0,
            vpn: 8_000.0,
            agents: 1_000.0,
            agents_most: None,
            unsent: false,
        };
        // Under their share, and at their floor, where the cap moves only
        // the ceiling.
        let under = Split {
            agents: 500.0,
            agents_most: Some(500.0),
            ..free
        };
        let floor = Split {
            agents_most: Some(1_000.0),
            ..free
        };
        for held in [under, floor] {
            assert_eq!(
                free.after(held, true),
                held,
                "a change that took wasn't kept"
            );
            assert_eq!(held.after(free, true), free, "a cap taken off was kept");
            let tried = free.after(held, false);
            assert_eq!(
                tried.agents_most, held.agents_most,
                "a cap tried was forgotten"
            );
            assert!(
                held.moved_from(&tried, RATE),
                "a cap that failed isn't sent again"
            );
            assert!(free.moved_from(&tried, RATE), "no change would take it off");
            let left = held.after(free, false);
            assert_eq!(
                left.agents_most, held.agents_most,
                "a cap left was forgotten"
            );
            assert!(
                free.moved_from(&left, RATE),
                "a cap left isn't taken off again"
            );
        }
        let again = free.after(free, false);
        assert_eq!(again.agents_most, None, "nothing tried was a cap");
        assert!(
            free.moved_from(&again, RATE),
            "a failed change isn't sent again"
        );
    }

    // Held for the VPN's wait, agents straight waiting get less than they
    // would, never under their part of the floor (half the quarter beside
    // the VPN's agents, all of it alone), and borrow nothing past it. The
    // other lanes keep what they had, so the three add up to less.
    #[test]
    fn a_held_share_caps_agents_straight_never_under_their_floor() {
        for (agents, floor) in [
            ((waiting(4), waiting(4)), RATE * AGENTS_FLOOR / 2.0),
            ((waiting(4), Agents::default()), RATE * AGENTS_FLOOR),
        ] {
            let free = Lanes::default().split(&seen((1_000.0, 2_000.0), agents), 1.0);
            assert_eq!(free.agents_most, None, "agents not held were capped");
            for share in [0.5, 0.1] {
                let held = Lanes::default().split(
                    &Seen {
                        agents_held: held_to(share),
                        ..seen((1_000.0, 2_000.0), agents)
                    },
                    1.0,
                );
                assert!(
                    held.agents < free.agents - 1.0 || held.agents <= floor + 1.0,
                    "held to {share}, agents straight kept {} of {}",
                    held.agents,
                    free.agents
                );
                assert!(
                    held.agents >= floor - 1.0,
                    "held to {share}, agents straight got {} under their floor {floor}",
                    held.agents
                );
                assert_eq!(
                    held.agents_most,
                    Some(held.agents),
                    "held agents may borrow past their rate"
                );
                assert!(
                    near(held.joel, free.joel) && near(held.vpn, free.vpn),
                    "holding agents straight moved Joel {} or the VPN {}",
                    held.joel,
                    held.vpn
                );
                assert!(
                    held.joel + held.vpn + held.agents <= RATE + 1.0,
                    "held to {share}, the lanes promised more than the rate: {held:?}"
                );
            }
        }
    }

    // Where their share of the split is over the floor, held agents keep
    // just that share.
    #[test]
    fn a_held_share_over_the_floor_is_what_agents_keep() {
        let agents = (waiting(4), waiting(4));
        let free = Lanes::default().split(&seen((0.0, 0.0), agents), 1.0);
        let held = Lanes::default().split(
            &Seen {
                agents_held: held_to(0.5),
                ..seen((0.0, 0.0), agents)
            },
            1.0,
        );
        assert!(
            free.agents * 0.5 > RATE * AGENTS_FLOOR / 2.0 + 1.0,
            "the case lands on the floor: agents free had {}",
            free.agents
        );
        assert!(
            near(held.agents, free.agents * 0.5),
            "held to half, agents straight had {} of {}",
            held.agents,
            free.agents
        );
    }

    // A hold climbing back once agents straight stop waiting caps nothing:
    // capped there, at the little the split leaves them, an upload starting
    // then would get the least a class has until it queued.
    #[test]
    fn a_held_share_caps_nothing_while_agents_straight_don_t_wait() {
        let agents = (Agents::default(), waiting(4));
        let free = Lanes::default().split(&seen((1_000.0, 2_000.0), agents), 1.0);
        let mut lanes = Lanes::default();
        let held = lanes.split(
            &Seen {
                agents_held: held_to(0.5),
                ..seen((1_000.0, 2_000.0), agents)
            },
            1.0,
        );
        assert_eq!(held, free, "agents not waiting were held");
        assert!(!lanes.agents_wait_straight(), "idle agents read as waiting");
        lanes.split(&seen((1_000.0, 2_000.0), (waiting(4), waiting(4))), 1.0);
        assert!(lanes.agents_wait_straight(), "waiting agents read as idle");
        assert_eq!(held_to(1.0), None, "a whole share held agents");
    }

    // A link the controller no longer holds keeps its last reading for each
    // new rate, so a hold it carried is let go before one comes.
    #[test]
    fn agents_let_go_borrow_the_whole_rate_at_the_next_rate() {
        let mut lanes = Lanes::default();
        let seen = Seen {
            agents_held: held_to(0.5),
            ..seen((0.0, 0.0), (waiting(4), waiting(4)))
        };
        assert!(
            lanes.split(&seen, 1.0).agents_most.is_some(),
            "agents held weren't capped"
        );
        assert!(
            lanes.at(9_000.0).agents_most.is_some(),
            "a new rate let agents go"
        );
        lanes.let_agents_go();
        assert_eq!(
            lanes.at(9_000.0).agents_most,
            None,
            "agents let go stayed held"
        );
    }

    fn words(commands: &[Vec<String>]) -> Vec<String> {
        commands.iter().map(|c| c.join(" ")).collect()
    }

    #[test]
    fn the_tree_puts_each_lane_under_the_rate_and_lets_each_borrow_it_all() {
        let split = Split {
            joel: 1_240.4,
            vpn: 8_759.6,
            agents: 0.0,
            agents_most: None,
            unsent: false,
        };
        let all = words(&tree("wlan0", 10_000, &split));
        let has = |line: &str| all.iter().any(|c| c == line);
        for line in [
            "tc class replace dev wlan0 parent 1: classid 1:2 htb \
             rate 10000kbit ceil 10000kbit prio 0 quantum 1514",
            "tc class replace dev wlan0 parent 1:2 classid 1:3 htb \
             rate 1240kbit ceil 10000kbit prio 0 quantum 1514",
            "tc class replace dev wlan0 parent 1:2 classid 1:4 htb \
             rate 8760kbit ceil 10000kbit prio 0 quantum 1514",
            "tc class replace dev wlan0 parent 1:2 classid 1:5 htb \
             rate 8kbit ceil 10000kbit prio 1 quantum 1514",
            "tc qdisc replace dev wlan0 parent 1:3 handle 2: cake diffserv4 nat ack-filter",
            "tc qdisc replace dev wlan0 parent 1:5 handle 5: cake besteffort nat ack-filter",
            "tc filter replace dev wlan0 parent 1: protocol all prio 1 handle 1 \
             basic match meta(priority eq 0x20001) classid 1:5",
        ] {
            assert!(has(line), "no `{line}` in {all:#?}");
        }
        let held = Split {
            joel: 1_240.4,
            vpn: 7_000.0,
            agents: 1_759.6,
            agents_most: Some(1_759.6),
            unsent: false,
        };
        let line = "tc class change dev wlan0 parent 1:2 classid 1:5 htb \
                    rate 1760kbit ceil 1760kbit prio 1 quantum 1514";
        assert!(
            words(&change("wlan0", 10_000, &held))
                .iter()
                .any(|c| c == line),
            "agents held for the VPN's wait still borrow past their rate"
        );
        let changed = words(&change("wlan0", 10_000, &split));
        assert!(
            changed.len() == 4 && changed.iter().all(|c| c.starts_with("tc class change")),
            "a change moves only the four classes: {changed:#?}"
        );
        assert_eq!(
            joel_rate("wlan0", Cake::At(12_000)).join(" "),
            "tc qdisc change dev wlan0 parent 1:3 handle 2: cake bandwidth 12000kbit",
            "Joel's CAKE doesn't follow the connection's rate, so it shares its tins by weight"
        );
        assert_eq!(
            joel_rate("wlan0", Cake::Unlimited).join(" "),
            "tc qdisc change dev wlan0 parent 1:3 handle 2: cake unlimited",
            "Joel's CAKE keeps a rate once nothing limits the upload"
        );
        assert_eq!(
            joel_cake(6_000, 2_000_000, 6_000),
            Cake::At(6_000),
            "held, it's shaped"
        );
        assert_eq!(
            joel_cake(1_999_999, 2_000_000, 1_990_000),
            Cake::At(1_990_000),
            "held just under the ceiling, it takes the classes' rate, not the one asked"
        );
        assert_eq!(
            joel_cake(2_000_000, 2_000_000, 2_000_000),
            Cake::Unlimited,
            "at rest, Joel's CAKE is shaped at a rate the connection never reaches"
        );
        let adopted = words(&adopt("wlan0", "shaper3", (10_000, &split), 80_000));
        let installed = words(&crate::qdisc::install(
            "wlan0",
            "shaper3",
            (10_000, &split),
            80_000,
        ));
        let roots: Vec<&String> = installed.iter().filter(|c| !adopted.contains(c)).collect();
        assert!(
            roots.len() == 2 && roots.iter().all(|c| c.contains(" root handle 1: htb")),
            "adopting puts on all but the two root HTBs, which drop what they hold: {roots:#?}"
        );
    }

    #[test]
    fn the_lanes_are_told_from_the_one_cake_they_replace() {
        let lanes = r#"[{"kind":"htb","handle":"1:","root":true},
            {"kind":"cake","handle":"2:","parent":"1:3"},
            {"kind":"fq_codel","handle":"4:","parent":"1:4"},
            {"kind":"fq_codel","handle":"10:","parent":"1:1"},
            {"kind":"cake","handle":"5:","parent":"1:5"}]"#;
        let one = r#"[{"kind":"htb","handle":"1:","root":true},
            {"kind":"cake","handle":"2:","parent":"1:2"},
            {"kind":"fq_codel","handle":"10:","parent":"1:1"}]"#;
        assert!(laned(lanes), "the lanes weren't seen");
        assert!(!laned(one), "the one CAKE was taken for the lanes");
    }

    #[test]
    fn adopting_clears_every_filter_but_the_ingress() {
        let held = r#"[{"kind":"htb","handle":"1:","root":true},
            {"kind":"cake","handle":"5:","parent":"1:5"},
            {"kind":"ingress","handle":"ffff:","parent":"ffff:fff1"}]"#;
        let cleared: Vec<String> = unfiltered("wlan0", held)
            .iter()
            .map(|c| c.join(" "))
            .collect();
        assert_eq!(
            cleared,
            [
                "tc filter del dev wlan0 parent 1:",
                "tc filter del dev wlan0 parent 5:"
            ],
            "a kept queue's old filters would outlive the version that made them"
        );
    }
}
