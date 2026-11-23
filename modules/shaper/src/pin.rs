//! The VPN's source-port pin. Past the router, the line sends each flow
//! one of its ways by the flow's ports, and a VPN is one flow: the port its
//! daemon opened its socket from decides, for as long as that socket
//! lives, which way every call and upload inside the tunnel goes. On 21
//! November each of 99 rounds over 18 hours sent each source port the same
//! way, one about 40 ms slower than the other at the farthest hop that
//! answers. So the shaper probes candidate ports toward the tunnel's
//! endpoint, finds two adjacent ports that both go the quick way, and has
//! nftables give new flows of the VPN's mark one of those two. NAT is
//! decided at a flow's first packet, so a live tunnel is never moved: the
//! daemon's next socket lands on the pin by itself. Two ports, since the
//! old socket's conntrack entry holds its port for a while after a new
//! socket opens.
//!
//! Which way is quick is told by delay alone, so nothing here names a
//! place or an address.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// The least time between rounds. The daemon opens a new socket on a
/// change of network and on its third reconnect in a row, about five
/// times in 17 hours on 21 November, and a round is about a hundred
/// packets, so one every ten minutes at most leaves a daemon that flaps
/// unprobed in between rather than probed at every flap.
pub const APART: Duration = Duration::from_secs(600);

/// The first of the ports tried, below the kernel's own range for
/// sockets that name none (32768 on), so a pin never takes a port some
/// program was handed.
pub const FIRST_PORT: u16 = 30_000;

/// How many ports are tried. With each port going either way at even
/// odds, the worst the 21 November trial allows (six of its eight ports
/// went the quick way), 32 ports hold no two adjacent quick ones about one
/// round in 750, against one in 25 for sixteen. Three probes a port make
/// about a hundred small packets a round.
pub const PORTS: u16 = 32;

/// How many probes each port is sent at the farthest hop. The least of
/// them is its delay: a queue anywhere only adds, so the least is the
/// way's own. A hop that answers one probe a second past a burst, as
/// Linux does by default, still answers one of three sent half a second
/// apart, so a round takes about forty seconds there, on its own thread.
pub const TRIES: usize = 3;

/// How much slower than the quickest port a port may read and still go
/// the quick way. The two ways read 22 to 26 ms and 65 to 67 ms on 22
/// November; a margin a quarter of that gap keeps a jittery quick port in
/// and every slow one out.
pub const MARGIN_MS: f64 = 10.0;

/// The farthest hop a probe is sent to. The way splits a few hops past the
/// router, and hops past the provider's edge stop answering.
pub const MOST_HOPS: u8 = 8;

/// One probe's answer: the hop that said its time ran out there, and how
/// long after the probe left it said so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Answer {
    pub hop: u8,
    pub ms: f64,
}

/// The farthest hop any of `answers` came from, and so the one every
/// port's delay is read at.
pub fn farthest(answers: &[Answer]) -> Option<u8> {
    answers.iter().map(|a| a.hop).max()
}

/// A port's delay at `hop`: the least of its answers from there, or none
/// where none came from there.
pub fn delay(answers: &[Answer], hop: u8) -> Option<f64> {
    answers
        .iter()
        .filter(|a| a.hop == hop)
        .map(|a| a.ms)
        .min_by(f64::total_cmp)
}

/// The pin: the first of two adjacent ports that both read within
/// `MARGIN_MS` of the quickest, the pair whose slower port is quickest.
/// None where no pair is quick, or where no port read slower than the
/// margin, since then the line shows one way and a pin would only narrow
/// what the daemon may open.
pub fn choose(delays: &BTreeMap<u16, f64>) -> Option<u16> {
    let quickest = delays.values().copied().min_by(f64::total_cmp)?;
    let slowest = delays.values().copied().max_by(f64::total_cmp)?;
    if slowest - quickest <= MARGIN_MS {
        return None;
    }
    let quick = |ms: f64| ms - quickest <= MARGIN_MS;
    delays
        .iter()
        .filter_map(|(&port, &ms)| {
            let next = *delays.get(&port.checked_add(1)?)?;
            (quick(ms) && quick(next)).then_some((port, ms.max(next)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(port, _)| port)
}

/// One flow of the VPN's mark the shaper saw leave: where it went and the
/// port its socket sent from, before any NAT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct End {
    pub to: Ipv4Addr,
    pub port: u16,
    pub from_port: u16,
}

/// The VPN's flows, from `nft -j list set inet <table> vpn_ends`: each
/// element is the concatenation of where a flow went, its port, and the
/// port it came from, with or without the timeout it was added with.
pub fn ends(json: &str) -> Vec<End> {
    let Ok(read) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(all) = read["nftables"].as_array() else {
        return Vec::new();
    };
    let mut seen: Vec<End> = all
        .iter()
        .filter_map(|item| item["set"]["elem"].as_array())
        .flatten()
        .filter_map(|element| {
            let value = element.get("elem").map_or(element, |e| &e["val"]);
            let [to, port, from_port] = value["concat"].as_array()?.as_slice() else {
                return None;
            };
            Some(End {
                to: to.as_str()?.parse().ok()?,
                port: u16::try_from(port.as_u64()?).ok()?,
                from_port: u16::try_from(from_port.as_u64()?).ok()?,
            })
        })
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen
}

/// The rule that gives new flows of the VPN's mark leaving by an uplink
/// the two ports from `first`; flows already open keep theirs.
pub fn rule(first: u16) -> String {
    format!(
        "oifname @uplinks meta mark @vpns meta l4proto udp masquerade to :{first}-{}",
        first.saturating_add(1)
    )
}

/// The pin chain as `table`'s transaction writes it: emptied, then the
/// pin's rule where there is one. Written with the sets every few
/// seconds, since a switch loads the table afresh, the chain empty.
pub fn chain(table: &str, pinned: Option<u16>) -> String {
    let mut out = format!("flush chain inet {table} pin\n");
    if let Some(first) = pinned {
        let _ = writeln!(out, "add rule inet {table} pin {}", rule(first));
    }
    out
}

/// What the journal says when the pin moves from `was` to `now`, from
/// what the round read of each port.
pub fn said(was: Option<u16>, now: Option<u16>, read: &BTreeMap<u16, f64>) -> String {
    let slowest = read.values().copied().max_by(f64::total_cmp).unwrap_or(0.0);
    match now {
        Some(first) => {
            let second = first.saturating_add(1);
            let ms = |port: u16| read.get(&port).copied().unwrap_or(0.0);
            format!(
                "The VPN's next connection leaves from port {first} or {second}, which read \
                 {:.0} and {:.0} ms at the farthest hop that answers, against {slowest:.0} ms \
                 for the slowest of {} ports",
                ms(first),
                ms(second),
                read.len()
            )
        }
        None => format!(
            "The VPN's next connection leaves from any port again, no longer {}: no two \
             adjacent ports of {} read the quick way, or every port read alike",
            was.map_or_else(String::new, |first| format!(
                "{first} or {}",
                first.saturating_add(1)
            )),
            read.len()
        ),
    }
}

/// How long a round's probes hold their ports in conntrack, from the
/// kernel's timeout for UDP that was never answered, in seconds, with a
/// few seconds more. Each probe went to the VPN's own server and port, so
/// while its entry lives a new VPN flow can't take the probe's port: with
/// both pinned ports held so, NAT finds no free port and the kernel drops
/// the flow. Read as its default, 30, when it can't be read.
pub fn settle(timeout: &str) -> Duration {
    Duration::from_secs(timeout.trim().parse::<u64>().unwrap_or(30) + 5)
}

/// What the pin knows between rounds: the VPN's flows it has probed for,
/// whether a round is out, when the last was asked, the pin a round found
/// and when it may take hold, and the pin in force.
#[derive(Debug, Default)]
pub struct Pinning {
    seen: Vec<End>,
    probing: bool,
    asked: Option<Instant>,
    coming: Option<(Option<u16>, Instant)>,
    pub pinned: Option<u16>,
}

impl Pinning {
    /// The flow to probe toward now, if any: one from a socket the VPN
    /// opened since the last round, at the start too, while no call is
    /// on, no round is out and none ran within `APART`. A flow left
    /// unprobed for a call or the spacing is asked for again later.
    pub fn ask(&mut self, ends: &[End], calling: bool, now: Instant) -> Option<End> {
        if calling
            || self.probing
            || self.coming.is_some()
            || self.asked.is_some_and(|at| now - at < APART)
        {
            return None;
        }
        let to = *ends.iter().find(|e| !self.seen.contains(e))?;
        self.seen = ends.to_vec();
        self.probing = true;
        self.asked = Some(now);
        Some(to)
    }

    /// The ports a round leaves out: the pinned pair, which the VPN's own
    /// flow may hold, so conntrack would give a probe from them another
    /// port and the probe would cross the line as some other flow.
    pub fn held(&self) -> Vec<u16> {
        self.pinned
            .map_or_else(Vec::new, |first| vec![first, first.saturating_add(1)])
    }

    /// Takes what a round read of each port, to take hold from `from`,
    /// once the round's probes have let their ports go (`settle`). A round
    /// that heard nothing, as with the network down, keeps the pin it had.
    pub fn found(&mut self, delays: &BTreeMap<u16, f64>, from: Instant) {
        self.probing = false;
        if !delays.is_empty() {
            self.coming = Some((choose(delays), from));
        }
    }

    /// Puts the pin a round found in force once its time comes, and says
    /// so when the pin moved.
    pub fn take_hold(&mut self, now: Instant) -> Option<Moved> {
        let (pin, from) = self.coming?;
        if now < from {
            return None;
        }
        self.coming = None;
        let was = std::mem::replace(&mut self.pinned, pin);
        (was != pin).then_some(Moved { was })
    }
}

/// A pin that moved, from the pin it replaced, none where new flows had
/// any port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moved {
    pub was: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answers(at: &[(u8, f64)]) -> Vec<Answer> {
        at.iter().map(|&(hop, ms)| Answer { hop, ms }).collect()
    }

    #[test]
    fn a_ports_delay_is_its_least_at_the_farthest_hop_that_answered() {
        // Hop 6 answers slowly or not at all on the quick way, as the
        // exchange's did on 22 November, so only hop 7 is read.
        let quick = answers(&[(5, 22.0), (6, 620.0), (7, 25.8), (7, 29.6), (7, 35.4)]);
        assert_eq!(farthest(&quick), Some(7), "the farthest hop wasn't 7");
        assert_eq!(
            delay(&quick, 7),
            Some(25.8),
            "the least at hop 7 wasn't kept"
        );
        assert_eq!(
            delay(&quick, 8),
            None,
            "a hop that never answered read a delay"
        );
        assert_eq!(farthest(&[]), None, "no answers named a hop");
    }

    fn delays(at: &[(u16, f64)]) -> BTreeMap<u16, f64> {
        at.iter().copied().collect()
    }

    #[test]
    fn the_pin_is_two_adjacent_ports_that_both_go_the_quick_way() {
        // As measured on 22 November: the slow way at 65 to 67 ms, the
        // quick one at 22 to 26.
        let read = delays(&[
            (30_000, 66.0),
            (30_001, 24.0),
            (30_002, 65.0),
            (30_003, 23.0),
            (30_004, 26.0),
            (30_005, 67.0),
        ]);
        assert_eq!(choose(&read), Some(30_003), "30003 and 30004 both go quick");
    }

    #[test]
    fn of_two_quick_pairs_the_one_whose_slower_port_is_quickest_is_pinned() {
        let read = delays(&[
            (30_000, 30.0),
            (30_001, 31.0),
            (30_002, 22.0),
            (30_003, 23.0),
            (30_004, 66.0),
        ]);
        assert_eq!(
            choose(&read),
            Some(30_002),
            "the quicker pair wasn't chosen"
        );
    }

    #[test]
    fn no_pin_where_no_two_adjacent_ports_go_the_quick_way() {
        let read = delays(&[
            (30_000, 24.0),
            (30_001, 66.0),
            (30_002, 23.0),
            (30_003, 65.0),
        ]);
        assert_eq!(choose(&read), None, "a pair with a slow port was pinned");
    }

    #[test]
    fn no_pin_where_every_port_goes_one_way() {
        let read = delays(&[(30_000, 24.0), (30_001, 31.0), (30_002, 23.0)]);
        assert_eq!(choose(&read), None, "a line showing one way was pinned");
        assert_eq!(choose(&BTreeMap::new()), None, "nothing read was pinned");
    }

    #[test]
    fn the_vpns_flows_are_read_from_its_set_with_or_without_timeouts() {
        let json = r#"{"nftables":[{"metainfo":{"version":"1.1.1"}},
            {"set":{"family":"inet","name":"vpn_ends","table":"shaper",
             "type":["ipv4_addr","inet_service","inet_service"],"flags":["timeout"],
             "elem":[{"elem":{"val":{"concat":["192.0.2.2",443,36284]},
                      "timeout":300,"expires":290}},
                     {"concat":["192.0.2.2",443,41000]},
                     {"concat":["192.0.2.2",443,36284]}]}}]}"#;
        assert_eq!(
            ends(json),
            vec![
                End {
                    to: Ipv4Addr::new(192, 0, 2, 2),
                    port: 443,
                    from_port: 36_284
                },
                End {
                    to: Ipv4Addr::new(192, 0, 2, 2),
                    port: 443,
                    from_port: 41_000
                },
            ],
            "each flow once, in order"
        );
        assert_eq!(ends(""), Vec::<End>::new(), "a refused listing named flows");
        assert_eq!(
            ends(r#"{"nftables":[{"set":{"name":"vpn_ends"}}]}"#),
            Vec::<End>::new(),
            "an empty set named flows"
        );
    }

    #[test]
    fn the_journal_says_where_the_pin_moved_and_what_it_read() {
        let read = delays(&[(30_000, 66.4), (30_001, 24.2), (30_002, 25.6)]);
        assert_eq!(
            said(None, Some(30_001), &read),
            "The VPN's next connection leaves from port 30001 or 30002, which read 24 and 26 \
             ms at the farthest hop that answers, against 66 ms for the slowest of 3 ports",
            "a new pin isn't said with its delays"
        );
        assert!(
            said(Some(30_001), None, &read).contains("no longer 30001 or 30002"),
            "a pin let go doesn't say which it was"
        );
    }

    #[test]
    fn the_chain_gives_new_flows_of_the_vpns_mark_the_two_pinned_ports() {
        assert_eq!(
            chain("shaper", Some(30_003)),
            "flush chain inet shaper pin\nadd rule inet shaper pin oifname @uplinks \
             meta mark @vpns meta l4proto udp masquerade to :30003-30004\n",
            "the chain isn't emptied and given the two ports from the pin"
        );
        assert_eq!(
            chain("shaper", None),
            "flush chain inet shaper pin\n",
            "with no pin the chain isn't left empty"
        );
    }

    fn end(from_port: u16) -> End {
        End {
            to: Ipv4Addr::new(192, 0, 2, 2),
            port: 443,
            from_port,
        }
    }

    #[test]
    fn a_round_is_asked_for_a_new_socket_outside_calls_and_spaced() {
        let start = Instant::now();
        let mut pinning = Pinning::default();
        assert_eq!(
            pinning.ask(&[], false, start),
            None,
            "no flow was probed toward"
        );
        assert_eq!(
            pinning.ask(&[end(36_284)], true, start),
            None,
            "a round was asked during a call"
        );
        assert_eq!(
            pinning.ask(&[end(36_284)], false, start),
            Some(end(36_284)),
            "the VPN's first flow wasn't probed toward"
        );
        let later = start + APART;
        assert_eq!(
            pinning.ask(&[end(41_000)], false, later),
            None,
            "a round was asked while one was out"
        );
        let settled = later + Duration::from_secs(35);
        pinning.found(
            &BTreeMap::from([(30_000, 24.0), (30_001, 25.0), (30_002, 66.0)]),
            settled,
        );
        assert_eq!(
            pinning.take_hold(later),
            None,
            "the pin took hold while the probes held its ports"
        );
        assert_eq!(
            pinning.ask(&[end(41_000)], false, settled + APART),
            None,
            "a round was asked while its last one's pin was still to take hold"
        );
        assert_eq!(
            pinning.take_hold(settled),
            Some(Moved { was: None }),
            "the pin didn't take hold once the probes let its ports go"
        );
        assert_eq!(
            pinning.pinned,
            Some(30_000),
            "the round's pair wasn't pinned"
        );
        assert_eq!(
            pinning.held(),
            vec![30_000, 30_001],
            "the pinned pair isn't held out"
        );
        assert_eq!(
            pinning.ask(&[end(36_284)], false, later),
            None,
            "a socket already probed for was probed again"
        );
        assert_eq!(
            pinning.ask(&[end(41_000)], false, start + Duration::from_secs(60)),
            None,
            "a new socket a minute on was probed within the spacing"
        );
        assert_eq!(
            pinning.ask(&[end(41_000)], false, later),
            Some(end(41_000)),
            "a new socket past the spacing wasn't probed toward"
        );
        pinning.found(&BTreeMap::new(), later);
        assert_eq!(
            pinning.take_hold(later),
            None,
            "a round that heard nothing moved the pin"
        );
        assert_eq!(
            pinning.pinned,
            Some(30_000),
            "a round that heard nothing lost the pin"
        );
        pinning.found(&BTreeMap::from([(30_004, 24.0), (30_005, 24.0)]), later);
        assert_eq!(
            pinning.take_hold(later),
            Some(Moved { was: Some(30_000) }),
            "a round that read every port alike didn't let the pin go"
        );
        assert_eq!(pinning.pinned, None, "a round that read alike kept a pin");
    }

    #[test]
    fn a_pin_waits_out_the_kernels_timeout_for_unanswered_udp() {
        assert_eq!(
            settle("30\n"),
            Duration::from_secs(35),
            "the default timeout isn't waited"
        );
        assert_eq!(
            settle("120"),
            Duration::from_secs(125),
            "a longer timeout isn't waited"
        );
        assert_eq!(
            settle(""),
            Duration::from_secs(35),
            "an unread timeout isn't the default"
        );
    }
}
