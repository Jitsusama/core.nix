//! The queues the shaper puts on an uplink, as tc and ip commands.
//!
//! Each uplink's root is an HTB with two classes that never limit each
//! other: 1:1 for what the link reaches without the router, under
//! `fq_codel` at a rate no link reaches, and 1:2 for everything else, at
//! the rate the controller finds, with the lanes beneath it (lanes.rs).
//! HTB puts a packet whose priority names one of its classes there, so
//! nftables sends local traffic to 1:1 by priority, and leaves every other
//! packet's priority naming CAKE's handle and a tin, which HTB doesn't know
//! and passes to its filters, then to its default, Joel's lane.
//!
//! What arrives is turned round through an IFB, a device of the shaper's
//! own, so a second HTB and CAKE can queue it as if it were leaving. Before
//! it goes, ctinfo copies the connection's class from its conntrack mark to
//! the packet's mark, where the IFB's CAKE reads the tin and its filter
//! finds local traffic. A packet's own mark is set only on the way in,
//! where nothing has marked it: on the way out, WARP and Tailscale route by
//! theirs. While nothing limits the download, what arrives goes straight
//! in instead, as `unturn` says why.

use serde_json::Value;

use crate::rate::{float, whole};

/// The conntrack mark's bits the shaper owns: the class, 1 to 4 in CAKE's
/// tin order (bulk, best effort, video, voice), and a bit for local
/// traffic. They sit clear of WARP's 0x100cf and Tailscale's 0xff0000.
pub const CLASS_MASK: u32 = 0x0700_0000;
pub const LOCAL_BIT: u32 = 0x0800_0000;
pub const OWN_MASK: u32 = CLASS_MASK | LOCAL_BIT;

/// A rate no link reaches, so HTB never holds either class back.
const UNLIMITED: &str = "100gbit";

/// The IFB that queues what one uplink receives, named by the uplink's
/// index in hex, since a name is at most 15 bytes and a link's can be
/// longer: "shaper" and eight hex digits fit any index.
pub fn ifb(index: u32) -> String {
    format!("shaper{index:x}")
}

fn cmd(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}

/// The root HTB, sending what nothing else claims to `default`, and its
/// class for local traffic.
fn htb(dev: &str, default: u32) -> Vec<Vec<String>> {
    vec![
        cmd(&format!(
            "tc qdisc replace dev {dev} root handle 1: htb default {default}"
        )),
        cmd(&format!(
            "tc class replace dev {dev} parent 1: classid 1:1 htb rate {UNLIMITED} quantum 60000"
        )),
        cmd(&format!(
            "tc qdisc replace dev {dev} parent 1:1 handle 10: fq_codel"
        )),
    ]
}

fn ingress_cake(kbit: u64) -> String {
    format!("cake bandwidth {kbit}kbit diffserv4 nat ingress fwmark {CLASS_MASK:#x}")
}

/// Everything an uplink needs, from nothing or from a half-made set, but
/// the turn of what arrives to the IFB, which `turn` adds: every command
/// replaces rather than adds, so it can be run again. The upload's rate is
/// `up`, split between its lanes as `lanes` says. HTB can't change the
/// class it sends what nothing claims to, so a root HTB from before the
/// lanes is taken away first, by whoever calls this.
pub fn install(
    link: &str,
    ifb: &str,
    (up, lanes): (u64, &crate::lanes::Split),
    down: u64,
) -> Vec<Vec<String>> {
    let mut all = htb(link, 3);
    all.extend(crate::lanes::tree(link, up, lanes));
    all.push(cmd(&format!("ip link add {ifb} type ifb")));
    all.push(cmd(&format!("ip link set {ifb} up")));
    all.extend(htb(ifb, 2));
    all.push(cmd(&format!(
        "tc class replace dev {ifb} parent 1: classid 1:2 htb rate {UNLIMITED} quantum 60000"
    )));
    all.push(cmd(&format!(
        "tc filter replace dev {ifb} parent 1: protocol all prio 1 \
         handle {LOCAL_BIT:#x}/{LOCAL_BIT:#x} fw classid 1:1"
    )));
    all.push(cmd(&format!(
        "tc qdisc replace dev {ifb} parent 1:2 handle 2: {}",
        ingress_cake(down)
    )));
    all
}

/// Turns what a link or a VPN receives round through its IFB, the class
/// copied from the connection to the packet on the way.
pub fn turn(dev: &str, ifb: &str) -> Vec<Vec<String>> {
    vec![
        cmd(&format!("tc qdisc replace dev {dev} handle ffff: ingress")),
        cmd(&format!(
            "tc filter replace dev {dev} parent ffff: protocol all prio 1 matchall \
             action ctinfo cpmark {OWN_MASK:#x} pipe action mirred egress redirect dev {ifb}"
        )),
    ]
}

/// Lets what a link or a VPN receives go straight in, past its IFB, while
/// nothing limits the download. CAKE there shapes nothing at a rate the
/// connection never reaches, yet it drops: under a full connection, a call
/// sorted best effort lost 4,213 of its 5,200 packets coming down through
/// a CAKE at 2 Gbit/s, on a connection carrying 90 Mbit/s, and none with
/// the shaper stopped. The IFB stays, so a limit puts the turn back alone.
pub fn unturn(dev: &str) -> Vec<String> {
    cmd(&format!("tc qdisc del dev {dev} ingress"))
}

/// The most a VPN adds to each packet it carries: an outer IPv6 and UDP
/// header with MASQUE's framing and tag, or `WireGuard`'s, about 80 bytes.
/// CAKE on a VPN counts it, so the VPN is held to what its packets cost
/// the connection, however small they are. Counted as a share of the rate
/// instead, small packets, where it's most, would overrun the connection,
/// and the queue would form again in its CAKE, where the VPN is one flow.
const VPN_OVERHEAD: u32 = 80;

/// What a VPN may send and bring in, as a share of the connection it
/// rides. On the connection a VPN is one flow, in one tin, so its queue
/// has to form inside it, where its packets can still be told apart. CAKE's
/// diffserv4 keeps bulk at least a sixteenth of the connection, so with
/// bulk outside the VPN, as a build's upload to a builder is, a VPN held
/// any closer than that fills the connection's queue too, and every flow
/// inside it loses packets alike: Joel's upload got a thirty-third of the
/// line beside 32 jobs uploads that way. Under it, by a twentieth for what
/// goes round the VPN.
pub const VPN_SHARE: f64 = 0.95 * 15.0 / 16.0;

/// A VPN's rate, in kbit/s, against the connection's.
pub fn vpn_rate(link_kbit: u64) -> u64 {
    whole(float(link_kbit) * VPN_SHARE)
}

/// A VPN's queue: CAKE alone at its root, under the same handle as a
/// connection's, so the priority nftables gives a packet names its tin there
/// too, taking each of an agent's sessions inside as one flow. A VPN
/// reaches nothing local, so it needs no HTB.
pub fn vpn(dev: &str, kbit: u64) -> Vec<Vec<String>> {
    vec![
        cmd(&format!(
            "tc qdisc replace dev {dev} root handle 2: \
             cake bandwidth {kbit}kbit diffserv4 ack-filter overhead {VPN_OVERHEAD}"
        )),
        crate::lanes::sessions(dev, "2:"),
    ]
}

fn vpn_ingress_cake(kbit: u64) -> String {
    format!("{} overhead {VPN_OVERHEAD}", ingress_cake(kbit))
}

/// What a VPN brings in, turned round through an IFB of its own to a CAKE
/// sorted by the same classes. The connection's IFB sees a VPN as one flow
/// of the VPN's daemon, so there a model's reply inside it waits behind a
/// git fetch inside it; held a little under the connection's rate here,
/// the VPN's queue forms where its connections can be told apart. Every
/// command replaces rather than adds, but for the IFB's, which says it
/// exists when run again.
pub fn vpn_ingress(dev: &str, ifb: &str, kbit: u64) -> Vec<Vec<String>> {
    let mut all = vec![
        cmd(&format!("ip link add {ifb} type ifb")),
        cmd(&format!("ip link set {ifb} up")),
        cmd(&format!(
            "tc qdisc replace dev {ifb} root handle 2: {}",
            vpn_ingress_cake(kbit)
        )),
    ];
    all.extend(turn(dev, ifb));
    all
}

/// A new rate for what a VPN brings in.
pub fn vpn_ingress_change(ifb: &str, kbit: u64) -> Vec<String> {
    cmd(&format!(
        "tc qdisc change dev {ifb} root handle 2: {}",
        vpn_ingress_cake(kbit)
    ))
}

/// Whether a VPN's root, or its IFB's, is the shaper's CAKE.
pub fn vpn_ours(json: &str) -> bool {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return false;
    };
    all.iter()
        .any(|q| q["kind"] == "cake" && q["root"] == true && q["handle"] == "2:")
}

/// A new rate for the download's CAKE, on the IFB; lanes.rs changes the
/// upload's.
pub fn change_down(ifb: &str, kbit: u64) -> Vec<String> {
    cmd(&format!(
        "tc qdisc change dev {ifb} parent 1:2 handle 2: {}",
        ingress_cake(kbit)
    ))
}

/// Takes an uplink's queues away, leaving the kernel's default.
pub fn remove(link: &str, ifb: &str) -> Vec<Vec<String>> {
    vec![
        cmd(&format!("tc qdisc del dev {link} root")),
        cmd(&format!("tc qdisc del dev {link} ingress")),
        cmd(&format!("ip link del {ifb}")),
    ]
}

/// Whether an IFB's queues, from `tc -j qdisc show dev`, are the
/// shaper's: its HTB at the root with CAKE under 1:2. A link's are its
/// lanes (`lanes::laned`).
pub fn ours(json: &str) -> bool {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return false;
    };
    let has = |kind: &str, key: &str, value: &str| {
        all.iter().any(|q| q["kind"] == kind && q[key] == value)
    };
    has("htb", "handle", "1:")
        && all.iter().any(|q| q["kind"] == "htb" && q["root"] == true)
        && has("cake", "parent", "1:2")
}

/// Whether a link's ingress turns its traffic to the IFB.
pub fn turned(json: &str) -> bool {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return false;
    };
    all.iter().any(|q| q["kind"] == "ingress")
}

/// One tin's counts, from CAKE's statistics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tin {
    pub packets: u64,
    pub bytes: u64,
    pub backlog: u64,
    pub drops: u64,
    pub marks: u64,
    pub average_us: u64,
    pub peak_us: u64,
    /// Flows with something held.
    pub flows: u32,
}

impl Tin {
    /// Takes another CAKE's tin in: agents' bulk, beside Joel's.
    fn add(&mut self, other: &Self) {
        self.packets += other.packets;
        self.bytes += other.bytes;
        self.backlog += other.backlog;
        self.drops += other.drops;
        self.marks += other.marks;
        self.average_us = self.average_us.max(other.average_us);
        self.peak_us = self.peak_us.max(other.peak_us);
        self.flows += other.flows;
    }
}

fn tins_of(cake: &Value) -> Vec<Tin> {
    let n = |t: &Value, key: &str| t[key].as_u64().unwrap_or(0);
    cake["tins"]
        .as_array()
        .map(|tins| {
            tins.iter()
                .map(|t| Tin {
                    packets: n(t, "sent_packets"),
                    bytes: n(t, "sent_bytes"),
                    backlog: n(t, "backlog_bytes"),
                    drops: n(t, "drops"),
                    marks: n(t, "ecn_mark"),
                    average_us: n(t, "avg_delay_us"),
                    peak_us: n(t, "peak_delay_us"),
                    flows: u32::try_from(n(t, "sparse_flows") + n(t, "bulk_flows"))
                        .unwrap_or(u32::MAX),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The tins of the CAKE in `tc -s -j qdisc show dev`, in its order: on a
/// link, Joel's, handle 2:, with agents' one tin taken into its bulk.
pub fn tins(json: &str) -> Vec<Tin> {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let cakes: Vec<&Value> = all.iter().filter(|q| q["kind"] == "cake").collect();
    let Some(main) = cakes
        .iter()
        .position(|q| q["handle"] == "2:")
        .or((!cakes.is_empty()).then_some(0))
    else {
        return Vec::new();
    };
    let mut tins = tins_of(cakes[main]);
    for (_, other) in cakes.iter().enumerate().filter(|(i, _)| *i != main) {
        if let (Some(bulk), Some(theirs)) = (tins.first_mut(), tins_of(other).first()) {
            bulk.add(theirs);
        }
    }
    tins
}

/// Where real-time traffic goes: an app's calls and everything Tuple's
/// daemon sends, in CAKE's tin order (bulk, best effort, video, voice).
const VIDEO: usize = 2;

/// What each of CAKE's four tins sent a second, in kbit/s, in diffserv4's
/// order: each class's share of what a device sends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tins {
    pub bulk: u64,
    pub best: u64,
    pub video: u64,
    pub voice: u64,
}

/// Follows what CAKE's tins have sent, for each tin's rate over the last
/// second or more: a step comes every 200 ms, too short to read a rate by.
#[derive(Debug, Default)]
pub struct TinRates {
    then: Option<(Vec<u64>, f64)>,
    kbit: Option<Tins>,
}

impl TinRates {
    /// Takes the bytes each tin has sent, `sent`, at `at` seconds.
    pub fn take(&mut self, sent: Vec<u64>, at: f64) {
        if sent.len() != 4 {
            *self = Self::default();
            return;
        }
        let Some((was, then)) = &self.then else {
            self.then = Some((sent, at));
            return;
        };
        let seconds = at - then;
        if seconds < 1.0 {
            return;
        }
        // A CAKE made again counts from nothing, so what it sent since
        // reads as nothing for that second rather than a rate past u64.
        let kbit = |tin: usize| {
            let bits = float(sent[tin].saturating_sub(was[tin]) * 8);
            whole(bits / 1_000.0 / seconds)
        };
        self.kbit = Some(Tins {
            bulk: kbit(0),
            best: kbit(1),
            video: kbit(2),
            voice: kbit(3),
        });
        self.then = Some((sent, at));
    }

    /// Each tin's rate over the last second read, once there is one.
    pub fn kbit(&self) -> Option<Tins> {
        self.kbit
    }
}

/// The bulk tin of the CAKE in `tc -s -j qdisc show dev`, the first in
/// its order.
pub fn bulk(json: &str) -> Option<Tin> {
    tins(json).into_iter().next()
}

/// The queues in `tc -s -j qdisc show dev` that hold what's shaped: each
/// CAKE, and the VPN's lane's `fq_codel`, but not the local class's, or
/// none without a CAKE.
fn shaping(json: &str) -> Option<Vec<Value>> {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return None;
    };
    all.iter().any(|q| q["kind"] == "cake").then(|| {
        all.into_iter()
            .filter(|q| q["kind"] == "cake" || (q["kind"] == "fq_codel" && q["parent"] != "1:1"))
            .collect()
    })
}

/// The bytes the shaped queues in `tc -s -j qdisc show dev` have sent,
/// which is what the connection carried: traffic to this network's own
/// machines goes to a class of its own and never reaches them.
pub fn shaped_bytes(json: &str) -> Option<u64> {
    shaping(json).map(|all| all.iter().filter_map(|q| q["bytes"].as_u64()).sum())
}

/// What a VPN's CAKE, in `tc -s -j qdisc show dev`, has counted sending:
/// its bytes, and the header it counts each packet with.
pub fn counted_bytes(json: &str) -> Option<u64> {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return None;
    };
    let cake = all.iter().find(|q| q["kind"] == "cake")?;
    Some(cake["bytes"].as_u64()? + u64::from(VPN_OVERHEAD) * cake["packets"].as_u64()?)
}

/// What the shaped queues in `tc -s -j qdisc show dev` hold now, in bytes,
/// and the packets they have dropped or marked, every tin together, or
/// none without a CAKE.
pub fn holding(json: &str) -> Option<(u64, u64)> {
    let all = shaping(json)?;
    let n = |q: &Value, key: &str| q[key].as_u64().unwrap_or(0);
    let marks = |q: &Value| {
        q["tins"].as_array().map_or_else(
            || n(q, "ecn_mark"),
            |tins| tins.iter().map(|t| n(t, "ecn_mark")).sum(),
        )
    };
    Some(all.iter().fold((0, 0), |(held, lost), q| {
        (held + n(q, "backlog"), lost + dropped(q) + marks(q))
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

/// How long a CAKE sending at `kbit` keeps what it holds, in ms, from the
/// bytes it held over `reads` reads together: what it held on average,
/// sent at its rate. Nothing read, or no rate, keeps nothing.
pub fn wait_ms(held: u64, reads: u32, kbit: u64) -> u64 {
    if reads == 0 || kbit == 0 {
        return 0;
    }
    held * 8 / u64::from(reads) / kbit
}

/// A queue CAKE holds, at least, rather than packets passing through it: a
/// couple of full-sized packets waiting.
pub const QUEUED_BYTES: u64 = 3_000;

/// Whether CAKE held a queue since it was last read: something waiting in
/// it now, or a packet it dropped or marked since, from what `holding`
/// read then and now. A count that starts over, as a link's queues made
/// again do, says nothing of the drops.
pub fn queued(before: Option<(u64, u64)>, now: (u64, u64)) -> bool {
    now.0 >= QUEUED_BYTES || before.is_some_and(|(_, dropped)| now.1 > dropped)
}

/// The packets the video tin of the CAKE in `tc -s -j qdisc show dev` has
/// sent.
pub fn video_packets(json: &str) -> u64 {
    tins(json).get(VIDEO).map_or(0, |tin| tin.packets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tin_s_rate_is_what_it_sent_over_a_second() {
        let mut rates = TinRates::default();
        rates.take(vec![0, 1_000, 2_000, 3_000], 10.0);
        assert_eq!(rates.kbit(), None, "one read has no rate");
        rates.take(vec![500_000, 251_000, 2_000, 15_500], 10.6);
        assert_eq!(rates.kbit(), None, "under a second has no rate yet");
        rates.take(vec![500_000, 251_000, 2_000, 15_500], 12.0);
        assert_eq!(
            rates.kbit(),
            Some(Tins {
                bulk: 2_000,
                best: 1_000,
                video: 0,
                voice: 50,
            })
        );
    }

    #[test]
    fn a_cake_made_again_counts_nothing_rather_than_a_wrapped_rate() {
        let mut rates = TinRates::default();
        rates.take(vec![9_000_000; 4], 0.0);
        rates.take(vec![125_000, 0, 0, 0], 1.0);
        assert_eq!(rates.kbit(), Some(Tins::default()));
        rates.take(vec![250_000, 0, 0, 0], 2.0);
        assert_eq!(rates.kbit().map(|t| t.bulk), Some(1_000));
    }

    #[test]
    fn no_cake_with_four_tins_says_no_rate() {
        let mut rates = TinRates::default();
        rates.take(vec![0; 4], 0.0);
        rates.take(vec![125_000; 4], 1.0);
        rates.take(Vec::new(), 2.0);
        assert_eq!(rates.kbit(), None);
    }

    #[test]
    fn a_vpn_leaves_the_connection_s_bulk_its_sixteenth() {
        // CAKE on the VPN counts its headers, so its rate is what its
        // packets cost the connection: under what the connection's best
        // effort gets while bulk outside the VPN takes its sixteenth, the
        // VPN's queue stays its own.
        let link = 8_800;
        let vpn = vpn_rate(link);
        assert!(
            vpn * 16 <= link * 15,
            "a VPN at {vpn} of {link} fills the connection"
        );
        assert!(
            vpn * 100 >= link * 85,
            "a VPN at {vpn} of {link} leaves too much unused"
        );
    }

    #[test]
    fn the_video_tin_s_packets_are_read() {
        let json = r#"[{"kind":"htb"},{"kind":"cake","tins":[
            {"sent_packets":1},{"sent_packets":2},{"sent_packets":3},{"sent_packets":4}]}]"#;
        assert_eq!(video_packets(json), 3);
        assert_eq!(video_packets("[]"), 0);
    }

    #[test]
    fn what_the_connection_carried_is_its_cake_s_bytes_not_the_local_class_s() {
        let json = r#"[{"kind":"htb","handle":"1:","bytes":9000},
                      {"kind":"cake","handle":"2:","parent":"1:2","bytes":6000},
                      {"kind":"fq_codel","handle":"10:","parent":"1:1","bytes":3000}]"#;
        assert_eq!(shaped_bytes(json), Some(6000));
        assert_eq!(shaped_bytes(r#"[{"kind":"fq_codel","bytes":3000}]"#), None);
        assert_eq!(shaped_bytes(""), None);
    }

    #[test]
    fn what_a_vpns_cake_counted_is_its_bytes_and_a_header_a_packet() {
        let json = r#"[{"kind":"cake","handle":"2:","bytes":13000,"packets":10}]"#;
        assert_eq!(counted_bytes(json), Some(13_800));
        assert_eq!(counted_bytes(r#"[{"kind":"fq_codel","bytes":3000}]"#), None);
        assert_eq!(counted_bytes(""), None);
    }

    #[test]
    fn what_cake_holds_is_its_backlog_and_every_tins_drops_and_marks() {
        let json = r#"[{"kind":"htb","backlog":99999,"drops":7},
                      {"kind":"cake","backlog":4542,"drops":10,"tins":[
            {"ecn_mark":1},{"ecn_mark":2},{"ecn_mark":0},{"ecn_mark":0}]}]"#;
        assert_eq!(holding(json), Some((4542, 13)));
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
        assert_eq!(holding(json), Some((0, 11)), "thinned ACKs aren't a queue");
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
        // Five reads holding 12 500 bytes on average, at 10 Mbit/s.
        assert_eq!(wait_ms(62_500, 5, 10_000), 10, "100 kbit at 10 kbit a ms");
        assert_eq!(wait_ms(62_500, 0, 10_000), 0, "nothing read keeps nothing");
        assert_eq!(wait_ms(62_500, 5, 0), 0, "no rate is no CAKE to wait in");
    }

    #[test]
    fn a_queue_is_something_waiting_in_cake_or_a_packet_it_dropped() {
        assert!(
            queued(Some((0, 13)), (4542, 13)),
            "a backlog wasn't a queue"
        );
        assert!(queued(Some((0, 13)), (0, 14)), "a drop wasn't a queue");
        assert!(
            !queued(Some((0, 13)), (1514, 13)),
            "one packet passing was a queue"
        );
        // Queues made again count from nothing.
        assert!(
            !queued(None, (0, 14)),
            "fresh queues' first count was a queue"
        );
    }

    #[test]
    fn an_ifbs_name_fits_the_kernels_limit() {
        assert!(ifb(4_000_000_000).len() <= 15);
    }

    #[test]
    fn installing_puts_the_lanes_up_and_cake_under_the_default_class_down() {
        let split = crate::lanes::Split::default();
        let all: Vec<String> = install("enp1", "shaper3", (9_000, &split), 80_000)
            .iter()
            .map(|c| c.join(" "))
            .collect();
        let out = "tc class replace dev enp1 parent 1: classid 1:2 htb \
                   rate 9000kbit ceil 9000kbit prio 0 quantum 1514";
        assert!(
            all.contains(&"tc qdisc replace dev enp1 root handle 1: htb default 3".to_string()),
            "what nothing claims doesn't go to Joel's lane in {all:?}"
        );
        let back = "tc qdisc replace dev shaper3 parent 1:2 handle 2: \
                    cake bandwidth 80000kbit diffserv4 nat ingress fwmark 0x7000000";
        assert!(all.contains(&out.to_string()), "no upload CAKE in {all:?}");
        assert!(
            all.contains(&back.to_string()),
            "no download CAKE in {all:?}"
        );
        assert!(
            all.iter()
                .any(|c| c.contains("handle 0x8000000/0x8000000 fw classid 1:1")),
            "local traffic isn't sent past the IFB's CAKE in {all:?}"
        );
        assert!(
            !all.iter().any(|c| c.contains("ffff:")),
            "installing turned downloads to the IFB before a limit asked: {all:?}"
        );
    }

    #[test]
    fn a_turn_copies_the_class_and_sends_what_arrives_to_the_ifb() {
        let all: Vec<String> = turn("enp1", "shaper3")
            .iter()
            .map(|c| c.join(" "))
            .collect();
        assert_eq!(
            all,
            [
                "tc qdisc replace dev enp1 handle ffff: ingress",
                "tc filter replace dev enp1 parent ffff: protocol all prio 1 matchall \
                 action ctinfo cpmark 0xf000000 pipe action mirred egress redirect dev shaper3",
            ],
            "the turn's commands"
        );
    }

    #[test]
    fn going_straight_leaves_the_ifb_in_place() {
        assert_eq!(
            unturn("enp1").join(" "),
            "tc qdisc del dev enp1 ingress",
            "going straight took more than the link's ingress"
        );
    }

    #[test]
    fn a_change_down_keeps_the_ifb_s_options() {
        assert_eq!(
            change_down("shaper3", 8_000).join(" "),
            "tc qdisc change dev shaper3 parent 1:2 handle 2: \
             cake bandwidth 8000kbit diffserv4 nat ingress fwmark 0x7000000"
        );
    }

    const LANES: &str = r#"[{"kind":"htb","handle":"1:","root":true,"bytes":99,"backlog":99},
        {"kind":"cake","handle":"2:","parent":"1:3","bytes":6000,"backlog":100,"drops":1,
         "tins":[{"sent_packets":0,"sent_bytes":0},{"sent_packets":4,"sent_bytes":6000,
         "ecn_mark":1},{},{}]},
        {"kind":"fq_codel","handle":"4:","parent":"1:4","bytes":8000,"backlog":3000,
         "drops":2,"ecn_mark":3},
        {"kind":"fq_codel","handle":"10:","parent":"1:1","bytes":5000,"backlog":7000,"drops":9},
        {"kind":"cake","handle":"5:","parent":"1:5","bytes":2000,"backlog":4000,"drops":4,
         "tins":[{"sent_packets":2,"sent_bytes":2000,"backlog_bytes":4000,"drops":4,
         "sparse_flows":1,"bulk_flows":6,"avg_delay_us":9000}]}]"#;

    #[test]
    fn the_lanes_together_are_what_the_connection_carried_and_held() {
        assert_eq!(
            shaped_bytes(LANES),
            Some(16_000),
            "all three lanes, not the local class"
        );
        assert_eq!(
            holding(LANES),
            Some((7_100, 11)),
            "every lane's backlog, drops and marks"
        );
        assert_eq!(drops(LANES), Some(7), "every lane's drops");
    }

    #[test]
    fn joel_s_tins_take_in_agents_bulk() {
        let t = tins(LANES);
        assert_eq!(t.len(), 4, "Joel's CAKE's four tins");
        assert_eq!(
            (t[0].bytes, t[0].backlog, t[0].flows, t[0].average_us),
            (2_000, 4_000, 7, 9_000),
            "agents' bulk"
        );
        assert_eq!(t[1].bytes, 6_000, "Joel's best effort");
    }

    #[test]
    fn what_a_vpn_brings_in_is_sorted_by_class_in_an_ifb_of_its_own() {
        let all: Vec<String> = vpn_ingress("wg0", "shaper9", 45_000)
            .iter()
            .map(|c| c.join(" "))
            .collect();
        assert!(
            all.contains(
                &"tc qdisc replace dev shaper9 root handle 2: \
              cake bandwidth 45000kbit diffserv4 nat ingress fwmark 0x7000000 overhead 80"
                    .to_string()
            )
        );
        assert!(
            all.iter()
                .any(|c| c.starts_with("tc filter replace dev wg0 parent ffff:")
                    && c.ends_with(
                        "ctinfo cpmark 0xf000000 pipe action mirred egress redirect dev shaper9"
                    ))
        );
        assert_eq!(
            vpn_ingress_change("shaper9", 40_000).join(" "),
            "tc qdisc change dev shaper9 root handle 2: \
             cake bandwidth 40000kbit diffserv4 nat ingress fwmark 0x7000000 overhead 80"
        );
    }

    #[test]
    fn a_vpns_cake_sits_at_its_root_under_the_connections_handle() {
        let cake = "tc qdisc replace dev wg0 root handle 2: \
                    cake bandwidth 9000kbit diffserv4 ack-filter overhead 80";
        let sessions = crate::lanes::sessions("wg0", "2:");
        assert_eq!(
            vpn("wg0", 9_000),
            [cmd(cake), sessions],
            "each session one flow"
        );
        assert!(vpn_ours(r#"[{"kind":"cake","handle":"2:","root":true}]"#));
        assert!(!vpn_ours(
            r#"[{"kind":"mq","handle":"0:","root":true},{"kind":"fq_codel","parent":":1"}]"#
        ));
        assert!(!vpn_ours(
            r#"[{"kind":"cake","handle":"2:","parent":"1:2"}]"#
        ));
    }

    #[test]
    fn the_kernels_own_queue_isnt_ours() {
        assert!(!ours(r#"[{"kind":"fq_codel","handle":"0:","root":true}]"#));
        assert!(!ours(r#"[{"kind":"noqueue","handle":"0:","root":true}]"#));
    }

    #[test]
    fn our_htb_with_cake_beneath_is_ours() {
        let json = r#"[{"kind":"htb","handle":"1:","root":true},
                      {"kind":"fq_codel","handle":"10:","parent":"1:1"},
                      {"kind":"cake","handle":"2:","parent":"1:2"},
                      {"kind":"ingress","handle":"ffff:","parent":"ffff:fff1"}]"#;
        assert!(ours(json));
        assert!(turned(json));
    }

    #[test]
    fn cakes_tins_are_read_in_order() {
        let json = r#"[{"kind":"htb"},{"kind":"cake","tins":[
            {"sent_packets":5,"sent_bytes":900,"backlog_bytes":40,"drops":1,"ecn_mark":0,
             "avg_delay_us":300,"peak_delay_us":900},
            {"sent_packets":7,"drops":0,"ecn_mark":2,"avg_delay_us":100,"peak_delay_us":400}]}]"#;
        let t = tins(json);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].packets, 5);
        assert_eq!((t[0].bytes, t[0].backlog), (900, 40));
        assert_eq!(bulk(json), Some(t[0].clone()));
        assert_eq!(t[1].marks, 2);
    }
}
