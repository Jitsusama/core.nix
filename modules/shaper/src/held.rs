//! What the shaped queues on a link hold, read from `tc -s -j qdisc show
//! dev`, and how long the VPN's packets wait behind it: whether a queue
//! stands, what was lost, and the wait the controller is told. The queues
//! themselves, as tc commands, are qdisc.rs's.

use crate::qdisc::shaping;
use serde_json::Value;

/// What the shaped queues on a link hold now, as one read of them gives it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Holding {
    /// What every shaped queue holds, in bytes.
    pub held: u64,
    /// What the queues the VPN's flow waits behind hold, in bytes: every
    /// one but agents' lane's CAKE. Agents' lane is a class of its own,
    /// which borrows the line only after the others (src/lanes.rs), so
    /// what stands there while it is held waits apart from the VPN's
    /// packets: counted as the link's wait, it would read as the VPN's
    /// wait drained by the very hold that only moved it there. Joel's lane
    /// counts: his CAKE is paced at the connection's rate, never under his
    /// share, and his class takes turns with the VPN's at one priority, so
    /// what stands there slows the VPN's packets as the line drains both.
    pub before_the_vpn: u64,
    /// The packets every shaped queue has dropped or marked, every tin
    /// together.
    pub lost: u64,
    /// What agents' lane's CAKE has sent, in bytes, as it counts: read
    /// against the read before, the rate agents keep beside the VPN.
    pub agents_sent: u64,
}

/// What the shaped queues in `tc -s -j qdisc show dev` hold now, or none
/// without a CAKE.
pub fn holding(json: &str) -> Option<Holding> {
    let all = shaping(json)?;
    let n = |q: &Value, key: &str| q[key].as_u64().unwrap_or(0);
    let marks = |q: &Value| {
        q["tins"].as_array().map_or_else(
            || n(q, "ecn_mark"),
            |tins| tins.iter().map(|t| n(t, "ecn_mark")).sum(),
        )
    };
    Some(all.iter().fold(Holding::default(), |was, q| {
        let behind = q["parent"] == crate::lanes::AGENTS_CLASS;
        Holding {
            held: was.held + n(q, "backlog"),
            before_the_vpn: was.before_the_vpn + if behind { 0 } else { n(q, "backlog") },
            lost: was.lost + dropped(q) + marks(q),
            agents_sent: was.agents_sent + if behind { n(q, "bytes") } else { 0 },
        }
    }))
}

/// The packets the shaped queues in `tc -s -j qdisc show dev` have
/// dropped, marks aside: what TCP sends again of these never crossed the
/// link.
pub fn drops(json: &str) -> Option<u64> {
    shaping(json).map(|all| all.iter().map(dropped).sum())
}

/// What a queue dropped of its flows. CAKE's count takes in the ACKs its
/// ack-filter thinned, which nothing sends again and which say nothing
/// of a queue: beside a download on 11 November they were six in every
/// seven, so the link's CAKE read as queued nearly every second and hid
/// the VPN's queue, and what TCP resent read as ours.
fn dropped(cake: &Value) -> u64 {
    let thinned: u64 = cake["tins"].as_array().map_or(0, |tins| {
        tins.iter()
            .map(|t| t["ack_drops"].as_u64().unwrap_or(0))
            .sum()
    });
    cake["drops"].as_u64().unwrap_or(0).saturating_sub(thinned)
}

/// How long what a read found ahead of the VPN's packets takes to leave,
/// in microseconds. HTB sends a class under its assured rate before any
/// class borrows, so while agents' lane holds anything it takes its
/// assured `agents_kbit` beside the VPN's turn, whatever its priority, and
/// the bytes before the VPN drain at `kbit` less that. But the lane sends
/// only what it holds: once it empties, they drain at the whole `kbit`.
/// The wait is at most the bytes before the VPN at what agents leave: a
/// lane that doesn't empty keeps its assured rate throughout. It is at
/// least every byte held at the whole link, since a lane that empties
/// first sent all it held in the meantime. And it is at least the bytes
/// before the VPN at what agents leave going as they went, `going_kbit`,
/// what they sent since the read before, up to their assurance: CAKE
/// keeps a busy lane's queue to a few packets, refilled as fast as it
/// sends, so a lane holding little may still be taking its whole rate. No
/// rate keeps nothing.
pub fn ahead_us(now: Holding, kbit: u64, agents_kbit: u64, going_kbit: u64) -> u64 {
    if kbit == 0 {
        return 0;
    }
    let all = now.held.max(now.before_the_vpn) * 8_000 / kbit;
    let left = kbit.saturating_sub(agents_kbit);
    if left == 0 {
        return all;
    }
    let going = kbit - going_kbit.min(agents_kbit);
    let least = all.max(now.before_the_vpn * 8_000 / going);
    (now.before_the_vpn * 8_000 / left).min(least)
}

/// How long the queues kept what they held ahead of the VPN, in ms, on
/// average over `reads` reads whose `ahead_us` add to `ahead`. Nothing
/// read keeps nothing.
pub fn wait_ms(ahead: u64, reads: u32) -> u64 {
    if reads == 0 {
        return 0;
    }
    ahead / u64::from(reads) / 1_000
}

/// A queue CAKE holds, at least, rather than packets passing through it: a
/// couple of full-sized packets waiting.
pub const QUEUED_BYTES: u64 = 3_000;

/// Whether CAKE held a queue since it was last read: something waiting in
/// it now, or a packet it dropped or marked since, from what `holding`
/// read then and now. A count that starts over, as a link's queues made
/// again do, says nothing of the drops.
pub fn queued(before: Option<Holding>, now: Holding) -> bool {
    now.held >= QUEUED_BYTES || before.is_some_and(|was| now.lost > was.lost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_cake_holds_is_its_backlog_and_every_tins_drops_and_marks() {
        let json = r#"[{"kind":"htb","backlog":99999,"drops":7},
                      {"kind":"cake","backlog":4542,"drops":10,"tins":[
            {"ecn_mark":1},{"ecn_mark":2},{"ecn_mark":0},{"ecn_mark":0}]}]"#;
        assert_eq!(
            holding(json),
            Some(Holding {
                held: 4542,
                before_the_vpn: 4542,
                lost: 13,
                agents_sent: 0,
            }),
            "a lone CAKE at the root, with no lanes, holds all the VPN's flow waits behind"
        );
        assert_eq!(holding(r#"[{"kind":"fq_codel","backlog":9000}]"#), None);
        assert_eq!(drops(json), Some(10));
        assert_eq!(drops(r#"[{"kind":"fq_codel","drops":9}]"#), None);
    }

    /// CAKE counts the ACKs its ack-filter thins among its drops, a
    /// hundred a second beside a download on 11 November against about
    /// fifteen it dropped of a flow, and nothing sends those again.
    #[test]
    fn the_acks_cake_thins_are_no_drop_and_no_queue() {
        let json = r#"[{"kind":"cake","backlog":0,"drops":110,"tins":[
            {"drops":4,"ack_drops":60,"ecn_mark":1},{"drops":6,"ack_drops":40,"ecn_mark":0},
            {"drops":0,"ack_drops":0,"ecn_mark":0},{"drops":0,"ack_drops":0,"ecn_mark":0}]}]"#;
        assert_eq!(drops(json), Some(10), "only what CAKE dropped of a flow");
        assert_eq!(
            holding(json).map(|h| (h.held, h.lost)),
            Some((0, 11)),
            "thinned ACKs aren't a queue"
        );
        let thinned = r#"[{"kind":"cake","backlog":0,"drops":160,"tins":[
            {"drops":4,"ack_drops":110,"ecn_mark":1},{"drops":6,"ack_drops":40,"ecn_mark":0},
            {"drops":0,"ack_drops":0,"ecn_mark":0},{"drops":0,"ack_drops":0,"ecn_mark":0}]}]"#;
        assert!(
            !queued(holding(json), holding(thinned).unwrap()),
            "more ACKs thinned, with nothing waiting, is no queue"
        );
    }

    #[test]
    fn what_cake_held_waits_as_long_as_its_rate_takes_to_send_it() {
        // 12 500 bytes ahead of the VPN, at 10 Mbit/s.
        let alone = Holding {
            held: 12_500,
            before_the_vpn: 12_500,
            ..Holding::default()
        };
        assert_eq!(
            ahead_us(alone, 10_000, 2_500, 0),
            10_000,
            "100 kbit at 10 kbit a ms, with agents' lane empty"
        );
        let busy = Holding {
            held: 42_500,
            ..alone
        };
        assert_eq!(
            ahead_us(busy, 10_000, 2_500, 2_500),
            13_333,
            "agents' 30 000 bytes, outlasting the VPN's wait, took their 2.5 Mbit/s throughout"
        );
        let trickle = Holding {
            held: 12_600,
            ..alone
        };
        assert_eq!(
            ahead_us(trickle, 10_000, 2_500, 0),
            10_080,
            "agents' 100 bytes took only themselves, not their assured rate throughout"
        );
        assert_eq!(
            ahead_us(trickle, 10_000, 2_500, 1_000),
            11_111,
            "agents still sending at 1 Mbit/s took that beside the VPN, holding only 100 bytes"
        );
        assert_eq!(
            ahead_us(alone, 10_000, 2_500, 5_000),
            13_333,
            "agents sending past their assurance, all borrowed, took only it ahead of the VPN"
        );
        let beside = Holding {
            held: 16_500,
            ..alone
        };
        assert_eq!(
            ahead_us(beside, 10_000, 2_500, 0),
            13_200,
            "agents' 4 000 bytes, sent in 12.8 ms, emptied before the VPN's turn came"
        );
        assert_eq!(
            ahead_us(beside, 10_000, 2_500, 2_500),
            13_333,
            "agents' 4 000 bytes, refilled as fast as they went, kept 2.5 Mbit/s throughout"
        );
        assert_eq!(
            ahead_us(beside, 10_000, 10_000, 0),
            13_200,
            "an assurance of the whole link, which no split gives, sends all of it first"
        );
        assert_eq!(ahead_us(alone, 0, 0, 0), 0, "no rate is no CAKE to wait in");
        assert_eq!(
            wait_ms(50_000, 5),
            10,
            "five reads of 10 ms each wait 10 ms"
        );
        assert_eq!(wait_ms(50_000, 0), 0, "nothing read keeps nothing");
    }

    /// What a link's queues hold and have lost, with none of it before the VPN.
    fn holds(held: u64, lost: u64) -> Holding {
        Holding {
            held,
            lost,
            ..Holding::default()
        }
    }

    #[test]
    fn a_queue_is_something_waiting_in_cake_or_a_packet_it_dropped() {
        assert!(
            queued(Some(holds(0, 13)), holds(4542, 13)),
            "a backlog wasn't a queue"
        );
        assert!(
            queued(Some(holds(0, 13)), holds(0, 14)),
            "a drop wasn't a queue"
        );
        assert!(
            !queued(Some(holds(0, 13)), holds(1514, 13)),
            "one packet passing was a queue"
        );
        // Queues made again count from nothing.
        assert!(
            !queued(None, holds(0, 14)),
            "fresh queues' first count was a queue"
        );
    }
}
