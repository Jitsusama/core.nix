//! The queues the shaper puts on an uplink, as tc and ip commands.
//!
//! Each uplink's root is an HTB with two classes that never limit each
//! other, at a rate no link reaches, so HTB only takes turns between them:
//! 1:1 for what the link reaches without the router, under `fq_codel` and
//! never slowed, and 1:2, the default, for everything else, under CAKE at
//! the rate the controller finds. HTB puts a packet whose priority names
//! one of its classes there, so nftables sends local traffic to 1:1 by
//! priority, and leaves every other packet's priority naming CAKE's handle
//! and a tin, which HTB doesn't know and passes to its default.
//!
//! What arrives is turned round through an IFB, a device of the shaper's
//! own, so a second HTB and CAKE can queue it as if it were leaving. Before
//! it goes, ctinfo copies the connection's class from its conntrack mark to
//! the packet's mark, where the IFB's CAKE reads the tin and its filter
//! finds local traffic. A packet's own mark is set only on the way in,
//! where nothing has marked it: on the way out, WARP and Tailscale route by
//! theirs.

use serde_json::Value;

use crate::rate::float;

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

fn htb(dev: &str) -> Vec<Vec<String>> {
    vec![
        cmd(&format!(
            "tc qdisc replace dev {dev} root handle 1: htb default 2"
        )),
        cmd(&format!(
            "tc class replace dev {dev} parent 1: classid 1:1 htb rate {UNLIMITED} quantum 60000"
        )),
        cmd(&format!(
            "tc class replace dev {dev} parent 1: classid 1:2 htb rate {UNLIMITED} quantum 60000"
        )),
        cmd(&format!(
            "tc qdisc replace dev {dev} parent 1:1 handle 10: fq_codel"
        )),
    ]
}

fn egress_cake(kbit: u64) -> String {
    // ack-filter thins a download's ACKs while they queue, so a full upload
    // doesn't hold a download back to the pace of its ACKs. nat has CAKE
    // read conntrack's addresses from before this machine's NAT, so each
    // virtual machine is a host of its own when CAKE shares between hosts.
    format!("cake bandwidth {kbit}kbit diffserv4 nat ack-filter")
}

fn ingress_cake(kbit: u64) -> String {
    format!("cake bandwidth {kbit}kbit diffserv4 nat ingress fwmark {CLASS_MASK:#x}")
}

/// Everything an uplink needs, from nothing or from a half-made set:
/// every command replaces rather than adds, so it can be run again.
pub fn install(link: &str, ifb: &str, up: u64, down: u64) -> Vec<Vec<String>> {
    let mut all = htb(link);
    all.push(cmd(&format!(
        "tc qdisc replace dev {link} parent 1:2 handle 2: {}",
        egress_cake(up)
    )));
    all.push(cmd(&format!("ip link add {ifb} type ifb")));
    all.push(cmd(&format!("ip link set {ifb} up")));
    all.extend(htb(ifb));
    all.push(cmd(&format!(
        "tc filter replace dev {ifb} parent 1: protocol all prio 1 \
         handle {LOCAL_BIT:#x}/{LOCAL_BIT:#x} fw classid 1:1"
    )));
    all.push(cmd(&format!(
        "tc qdisc replace dev {ifb} parent 1:2 handle 2: {}",
        ingress_cake(down)
    )));
    all.push(cmd(&format!(
        "tc qdisc replace dev {link} handle ffff: ingress"
    )));
    all.push(cmd(&format!(
        "tc filter replace dev {link} parent ffff: protocol all prio 1 matchall \
         action ctinfo cpmark {OWN_MASK:#x} pipe action mirred egress redirect dev {ifb}"
    )));
    all
}

/// A VPN's queue: CAKE alone at its root, under the same handle as a
/// connection's, so the priority nftables gives a packet names its tin there
/// too. A VPN reaches nothing local, so it needs no HTB.
pub fn vpn(dev: &str, kbit: u64) -> Vec<String> {
    cmd(&format!(
        "tc qdisc replace dev {dev} root handle 2: cake bandwidth {kbit}kbit diffserv4 ack-filter"
    ))
}

/// Whether a VPN's root is the shaper's CAKE.
pub fn vpn_ours(json: &str) -> bool {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return false;
    };
    all.iter()
        .any(|q| q["kind"] == "cake" && q["root"] == true && q["handle"] == "2:")
}

/// A new rate for one direction's CAKE.
pub fn change(dev: &str, kbit: u64, ingress: bool) -> Vec<String> {
    let cake = if ingress {
        ingress_cake(kbit)
    } else {
        egress_cake(kbit)
    };
    cmd(&format!(
        "tc qdisc change dev {dev} parent 1:2 handle 2: {cake}"
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

/// Whether a device's queues, from `tc -j qdisc show dev`, are the
/// shaper's: its HTB at the root with CAKE under 1:2.
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
#[derive(Clone, Debug, PartialEq)]
pub struct Tin {
    pub packets: u64,
    pub drops: u64,
    pub marks: u64,
    pub average_us: u64,
    pub peak_us: u64,
}

/// The tins of the CAKE in `tc -s -j qdisc show dev`, in its order.
pub fn tins(json: &str) -> Vec<Tin> {
    let Ok(Value::Array(all)) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(cake) = all.iter().find(|q| q["kind"] == "cake") else {
        return Vec::new();
    };
    let n = |t: &Value, key: &str| t[key].as_u64().unwrap_or(0);
    cake["tins"]
        .as_array()
        .map(|tins| {
            tins.iter()
                .map(|t| Tin {
                    packets: n(t, "sent_packets"),
                    drops: n(t, "drops"),
                    marks: n(t, "ecn_mark"),
                    average_us: n(t, "avg_delay_us"),
                    peak_us: n(t, "peak_delay_us"),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Where real-time traffic goes: an app's calls and everything Tuple's
/// daemon sends, in CAKE's tin order (bulk, best effort, video, voice).
const VIDEO: usize = 2;
/// Packets a second in the video tins, both ways together, that say a call
/// or a screen share runs: a call's audio alone is about 50 a second each
/// way, where nothing else is classed there.
const LIVE_PACKETS: f64 = 20.0;
/// How long after its last such second real-time traffic still counts as
/// flowing, so a pause in talk doesn't put the floor back mid-call.
const LIVE_HOLD_S: f64 = 30.0;

/// The packets the video tin of the CAKE in `tc -s -j qdisc show dev` has
/// sent.
pub fn video_packets(json: &str) -> u64 {
    tins(json).get(VIDEO).map_or(0, |tin| tin.packets)
}

/// Whether real-time traffic crosses a link, from its video tins' counts.
#[derive(Clone, Debug, Default)]
pub struct RealTime {
    /// The packets counted last, and when, in seconds.
    seen: Option<(u64, f64)>,
    /// Until when it counts as flowing.
    until: f64,
}

impl RealTime {
    /// Takes the video tins' packets, sent and received together, at `now`
    /// seconds, and gives whether real-time traffic flows. A count that
    /// starts over, as a link's queues made again do, reads as none.
    pub fn count(&mut self, packets: u64, now: f64) -> bool {
        if let Some((before, at)) = self.seen {
            let seconds = (now - at).max(0.001);
            if float(packets.saturating_sub(before)) / seconds >= LIVE_PACKETS {
                self.until = now + LIVE_HOLD_S;
            }
        }
        self.seen = Some((packets, now));
        now < self.until
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_s_packets_count_as_real_time_and_last_through_a_pause() {
        let mut live = RealTime::default();
        assert!(!live.count(0, 0.0));
        // A call's audio, 50 packets a second each way.
        assert!(live.count(100, 1.0));
        // Silence for a while keeps it, as talk pauses.
        assert!(live.count(100, 20.0));
        assert!(!live.count(100, 32.0));
    }

    #[test]
    fn a_trickle_isn_t_real_time_and_a_count_started_over_isn_t_a_burst() {
        let mut live = RealTime::default();
        live.count(1_000, 0.0);
        assert!(!live.count(1_010, 1.0));
        assert!(!live.count(3, 2.0));
    }

    #[test]
    fn the_video_tin_s_packets_are_read() {
        let json = r#"[{"kind":"htb"},{"kind":"cake","tins":[
            {"sent_packets":1},{"sent_packets":2},{"sent_packets":3},{"sent_packets":4}]}]"#;
        assert_eq!(video_packets(json), 3);
        assert_eq!(video_packets("[]"), 0);
    }

    #[test]
    fn an_ifbs_name_fits_the_kernels_limit() {
        assert!(ifb(4_000_000_000).len() <= 15);
    }

    #[test]
    fn installing_puts_cake_under_the_default_class_both_ways() {
        let all: Vec<String> = install("enp1", "shaper3", 9_000, 80_000)
            .iter()
            .map(|c| c.join(" "))
            .collect();
        let out = "tc qdisc replace dev enp1 parent 1:2 handle 2: \
                   cake bandwidth 9000kbit diffserv4 nat ack-filter";
        let back = "tc qdisc replace dev shaper3 parent 1:2 handle 2: \
                    cake bandwidth 80000kbit diffserv4 nat ingress fwmark 0x7000000";
        assert!(all.contains(&out.to_string()));
        assert!(all.contains(&back.to_string()));
        assert!(all.iter().any(|c| {
            c.contains("ctinfo cpmark 0xf000000 pipe action mirred egress redirect dev shaper3")
        }));
        assert!(
            all.iter()
                .any(|c| c.contains("handle 0x8000000/0x8000000 fw classid 1:1"))
        );
    }

    #[test]
    fn a_change_keeps_each_directions_options() {
        assert_eq!(
            change("enp1", 8_000, false).join(" "),
            "tc qdisc change dev enp1 parent 1:2 handle 2: \
             cake bandwidth 8000kbit diffserv4 nat ack-filter"
        );
        assert!(
            change("shaper3", 8_000, true)
                .join(" ")
                .ends_with("ingress fwmark 0x7000000")
        );
    }

    #[test]
    fn a_vpns_cake_sits_at_its_root_under_the_connections_handle() {
        assert_eq!(
            vpn("wg0", 9_000).join(" "),
            "tc qdisc replace dev wg0 root handle 2: cake bandwidth 9000kbit diffserv4 ack-filter"
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
            {"sent_packets":5,"drops":1,"ecn_mark":0,"avg_delay_us":300,"peak_delay_us":900},
            {"sent_packets":7,"drops":0,"ecn_mark":2,"avg_delay_us":100,"peak_delay_us":400}]}]"#;
        let t = tins(json);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].packets, 5);
        assert_eq!(t[1].marks, 2);
    }
}
