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
//! megabit to six. HTB lends Joel's two classes what the others leave
//! before it lends agents' any, and every class may borrow up to the whole
//! rate: the VPN's own CAKE is what holds it near V, and when that runs
//! ahead of a cut for a moment, its class borrows rather than dropping
//! what's inside the tunnel alike. But
//! while any agent has something waiting, a quarter of the rate is theirs
//! whatever Joel wants: agents are always usable. Agents get what Joel
//! leaves, split evenly between the lanes on which some wait, so a push
//! straight to a builder gets half rather than the sixteenth CAKE's bulk
//! tin gave it, which on 16 November was 346 kbit/s of 5.5 Mb/s. The
//! split isn't by flows: on 16 November at 20:25 agents held 117 flows to
//! the model inside the VPN and a few straight to the builders, so a
//! split by flows gave the builders' copies 8 kbit/s and they failed.
//!
//! A lane's agents wait while they hold a few packets, or while its CAKE
//! dropped or marked one in the last second: a copy starved to a window
//! of two packets holds too little to count, yet CAKE drops what waits
//! past its target, so a drop says the lane wants more than it gets.
//!
//! The VPN's share rises at once, and falls by at most a fifth a second,
//! so a copy's queue emptying between bursts doesn't swing it; a move
//! under a twentieth isn't made at all. It's held as a share, so a rate the
//! controller cuts cuts V with it at once. room.rs, which did this for the
//! two bulk tins alone, flipped its two lines about 2,200 times in two days.

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
/// The most the VPN's share falls in a second, as a share of itself.
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
    /// made, past its target: not those its ACK filter took.
    pub dropped: u64,
}

impl Agents {
    /// Whether the lane's agents wait, given whether their CAKE dropped
    /// or marked one in the last second, `pressed`.
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
}

/// The connection's three classes' rates, in kbit/s, which add up to the
/// controller's rate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Split {
    /// Joel straight on the connection.
    pub joel: f64,
    /// The VPN's outer flow, V.
    pub vpn: f64,
    /// Agents straight on the connection.
    pub agents: f64,
}

impl Split {
    /// Whether a class's rate moved by more than a hundredth of `rate`
    /// from `was`'s, so the classes are worth changing.
    pub fn moved_from(&self, was: &Self, rate: f64) -> bool {
        let past = |a: f64, b: f64| (a - b).abs() * 100.0 > rate;
        past(self.joel, was.joel) || past(self.vpn, was.vpn) || past(self.agents, was.agents)
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

/// Follows the VPN's share from second to second.
#[derive(Clone, Debug, Default)]
pub struct Lanes {
    share: Option<f64>,
    last: Seen,
    /// Whether each lane's CAKE, straight and inside the VPN, dropped
    /// or marked one between the last two seconds read.
    pressed: (bool, bool),
    /// What each had dropped and marked when last read, once read.
    dropped: Option<(u64, u64)>,
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
            waiting_vpn: self.last.agents_vpn.waiting_flows(self.pressed.1),
            waiting_direct: self.last.agents_straight.waiting_flows(self.pressed.0),
        })
    }

    /// The split at a new rate, from what was seen last: the controller
    /// moves its rate between the seconds this is told of the lanes.
    pub fn at(&mut self, rate: f64) -> Split {
        let seen = Seen { rate, ..self.last };
        self.split(&seen, 0.0)
    }

    /// How the rate is split now, given what was seen `dt` seconds after
    /// the last.
    pub fn split(&mut self, seen: &Seen, dt: f64) -> Split {
        self.last = *seen;
        let (straight, inside) = (seen.agents_straight, seen.agents_vpn);
        // A new rate between seconds reads nothing new, so what was
        // pressed stays. A CAKE made again counts from nothing, which
        // reads as no drop.
        if dt > 0.0 {
            let now = (straight.dropped, inside.dropped);
            self.pressed = self
                .dropped
                .map_or((false, false), |was| (now.0 > was.0, now.1 > was.1));
            self.dropped = Some(now);
        }
        let rate = seen.rate;
        if rate <= 0.0 {
            self.share = None;
            return Split::default();
        }
        let (beside, within) = (
            straight.waiting(self.pressed.0),
            inside.waiting(self.pressed.1),
        );
        let floor = if beside || within {
            rate * AGENTS_FLOOR
        } else {
            0.0
        };
        // Joel's two parts shrink alike when together they'd pass what he
        // may take, so neither of his lanes is starved for the other.
        let need = |carried: f64| carried * HEADROOM + JOEL_LEAST;
        let wants = (need(seen.joel_straight), need(seen.joel_vpn));
        let scale = ((rate - floor) / (wants.0 + wants.1)).min(1.0);
        let (joel, joel_vpn) = (wants.0 * scale, wants.1 * scale);
        // What's left is the agents', evenly between the lanes they wait
        // on. With none waiting it goes to the VPN, whose own CAKE is
        // held to V: the classes beside it borrow what it leaves.
        let left = rate - joel - joel_vpn;
        let to_vpn = match (beside, within) {
            (true, true) => left / 2.0,
            (true, false) => 0.0,
            (false, _) => left,
        };
        let vpn = self
            .hold(joel_vpn + to_vpn, joel_vpn, rate, dt)
            .min(rate - joel);
        Split {
            joel,
            vpn,
            agents: rate - joel - vpn,
        }
    }

    /// The VPN's rate, moving towards `wanted` kbit/s: at once when it
    /// rises past a twentieth or Joel's own part, `joel`, passes what's
    /// held; by at most a fifth a second when it falls past a twentieth.
    fn hold(&mut self, wanted: f64, joel: f64, rate: f64, dt: f64) -> f64 {
        let wanted = wanted / rate;
        let share = match self.share {
            Some(held) if joel / rate <= held => {
                if (wanted - held).abs() < held * STILL {
                    held
                } else if wanted > held {
                    wanted
                } else {
                    wanted.max(held * (1.0 - FALL * dt.clamp(0.0, 1.0)))
                }
            }
            _ => wanted,
        };
        self.share = Some(share);
        share * rate
    }
}

fn cmd(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}

/// The connection's rate on 1:2, and the three classes under it at
/// `split`, as tc commands that `verb` each class: replace or change.
fn classes(verb: &str, dev: &str, rate: u64, split: &Split) -> Vec<Vec<String>> {
    let class = |parent: &str, id: &str, kbit: u64, prio: u8| {
        cmd(&format!(
            "tc class {verb} dev {dev} parent {parent} classid {id} htb \
             rate {kbit}kbit ceil {rate}kbit prio {prio} quantum {QUANTUM}"
        ))
    };
    let kbit = |share: f64| whole(share).max(LEAST_KBIT);
    vec![
        class("1:", "1:2", rate, 0),
        class("1:2", "1:3", kbit(split.joel), 0),
        class("1:2", "1:4", kbit(split.vpn), 0),
        class("1:2", "1:5", kbit(split.agents), 1),
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
        && has("cake", "1:5")
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

    fn need(carried: f64) -> f64 {
        carried * HEADROOM + JOEL_LEAST
    }

    fn waiting(flows: u32) -> Agents {
        Agents {
            backlog: 30_000,
            flows,
            dropped: 0,
        }
    }

    /// A copy starved to a window of two packets: it holds too little to
    /// count as waiting, two segments of 1,448 bytes, and its CAKE has
    /// dropped `dropped` so far.
    fn starved(dropped: u64) -> Agents {
        Agents {
            backlog: 2 * 1_448,
            flows: 2,
            dropped,
        }
    }

    fn seen(joel: (f64, f64), agents: (Agents, Agents)) -> Seen {
        Seen {
            rate: RATE,
            joel_straight: joel.0,
            joel_vpn: joel.1,
            agents_straight: agents.0,
            agents_vpn: agents.1,
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
            dropped: 0,
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
        let changed = words(&change("wlan0", 10_000, &split));
        assert!(
            changed.len() == 4 && changed.iter().all(|c| c.starts_with("tc class change")),
            "a change moves only the four classes: {changed:#?}"
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
