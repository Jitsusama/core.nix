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
//!
//! Long because its tests, over half of it, walk each way a pin is kept,
//! moved or let go: a round that can't judge it, a network changed under
//! it, flows that hold its ports, and flows its rule can't get through.

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

/// The fewest ports a round must read to move the pin: half the 32 tried.
/// A round that heard fewer, as with a hop that stopped answering most
/// probes, would choose from too few to say the line no longer splits.
pub const HEARD: usize = 16;

/// How many times the VPN's flows may come back to the pin's rule, one
/// look after another, before the pin lets go. With other flows holding
/// both pinned ports toward the server, as a third socket in a burst of
/// reconnects might, NAT finds no port free and the kernel drops the new
/// flow, so the VPN stays down until conntrack frees one, two minutes for
/// a flow that was answered. NAT meets a flow it let through only at its
/// first packet, so a flow that comes back was dropped: the VPN sends again
/// within seconds, and letting go costs only the quick way.
pub const RETRIED: u64 = 2;

/// How long the pin's rule remembers a flow it gave a port, to know it
/// when it comes back: under conntrack's 30 s for unanswered UDP, so a flow
/// that went through and then idled until its entry went is never taken
/// for one dropped.
pub const REMEMBERED: Duration = Duration::from_secs(20);

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

/// A pin in force: the first of its two ports, the VPN's server and port
/// it was measured toward, to which alone its rules give the pair, and the
/// slower of the pair's delays when a round chose it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub first: u16,
    pub to: Ipv4Addr,
    pub port: u16,
    pub ms: f64,
}

/// The rules that give new flows of the VPN's mark leaving by an uplink
/// for the server the pin was measured toward the two ports from its
/// first, flows already open keeping theirs: one counts a flow that comes
/// back, the other remembers each and gives it the pair.
pub fn rules(pin: &Pin) -> [String; 2] {
    let Pin {
        first, to, port, ..
    } = *pin;
    let vpn = format!(
        "oifname @uplinks meta mark @vpns meta nfproto ipv4 meta l4proto udp \
         ip daddr {to} udp dport {port}"
    );
    let flow = "ip daddr . udp dport . udp sport";
    [
        format!("{vpn} {flow} @pintries counter name \"pinretried\""),
        format!(
            "{vpn} add @pintries {{ {flow} timeout {}s }} masquerade to :{first}-{}",
            REMEMBERED.as_secs(),
            first.saturating_add(1)
        ),
    ]
}

/// The pin chain as `table`'s transaction writes it: emptied, then the
/// pin's rules where there is one. Written with the sets every few
/// seconds, since a switch loads the table afresh, the chain empty.
pub fn chain(table: &str, pinned: Option<&Pin>) -> String {
    let mut out = format!("flush chain inet {table} pin\n");
    for rule in pinned.map(rules).into_iter().flatten() {
        let _ = writeln!(out, "add rule inet {table} pin {rule}");
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
/// whether a round is out and whether its network changed since, when the
/// last was asked, the pin a round found and when it may take hold, the
/// pin in force, the count last read of its flows that came back, how many
/// came back look after look since, whether the last round kept a pin it
/// couldn't judge, the network the pin was measured on, and the VPN's
/// flows now, each with the first of the pair pinned when it was first
/// seen.
#[derive(Debug, Default)]
pub struct Pinning {
    seen: Vec<End>,
    probing: bool,
    stale: bool,
    asked: Option<Instant>,
    coming: Option<(Option<Pin>, Instant)>,
    pub pinned: Option<Pin>,
    retried: Option<u64>,
    failing: u64,
    kept: bool,
    on: Option<String>,
    flows: Vec<(End, Option<u16>)>,
}

/// How many of the VPN's flows came back to the pin's rule, in packets,
/// from `nft -j list counter`, or none unread.
pub fn retried(json: &str) -> Option<u64> {
    let all: Value = serde_json::from_str(json).ok()?;
    all["nftables"].as_array()?.iter().find_map(|item| {
        let counter = &item["counter"];
        (counter["name"] == "pinretried").then(|| counter["packets"].as_u64())?
    })
}

/// What the journal says when a pin lets go because the VPN's flows under
/// it were dropped.
pub fn dropped(first: u16, times: u64) -> String {
    format!(
        "The VPN's next connection leaves from any port again, no longer {first} or {}: its \
         new connections came back unsent {times} times, as when other flows hold both ports \
         toward its server",
        first.saturating_add(1)
    )
}

/// What the journal says when a pin lets go because `why`: two rounds in
/// a row couldn't judge it, or the uplink moved to another network.
pub fn let_go(first: u16, why: &str) -> String {
    format!(
        "The VPN's next connection leaves from any port again, no longer {first} or {}: {why}",
        first.saturating_add(1)
    )
}

/// Why a pin two rounds couldn't judge lets go.
pub const UNJUDGED: &str = "two rounds in a row couldn't measure it, so it isn't kept on a \
                            delay read long ago";

/// Why a pin whose rule nftables refused lets go.
pub const REFUSED: &str = "nftables refused its rule";

/// The pin in force and the network it was measured on, as the shaper
/// keeps them across a restart, on one line: the boot it was made in,
/// since a boot starts the VPN's flows afresh, the first port, the
/// server, its port, the pair's delay and the network.
pub fn saved(pin: &Pin, network: &str, boot: &str) -> String {
    let Pin {
        first,
        to,
        port,
        ms,
    } = pin;
    format!("{boot} {first} {to} {port} {ms} {network}\n")
}

/// A pin and its network as `saved` wrote them in this boot, `boot`, or
/// nothing where the text isn't one or was written in another boot.
pub fn restored(text: &str, boot: &str) -> Option<(Pin, String)> {
    let mut words = text.trim().splitn(6, ' ');
    if boot.is_empty() || words.next()? != boot {
        return None;
    }
    let pin = Pin {
        first: words.next()?.parse().ok()?,
        to: words.next()?.parse().ok()?,
        port: words.next()?.parse().ok()?,
        ms: words
            .next()?
            .parse()
            .ok()
            .filter(|ms: &f64| ms.is_finite())?,
    };
    let network = words.next().filter(|n| !n.is_empty())?;
    (FIRST_PORT..FIRST_PORT + PORTS - 1)
        .contains(&pin.first)
        .then(|| (pin, network.to_owned()))
}

/// Why a pin measured on another network lets go.
pub const ELSEWHERE: &str = "it was measured on another network";

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

    /// Takes the VPN's flows now: each new one is remembered with the pair
    /// pinned toward its server when it was first seen, since NAT gave it
    /// one of that pair, and a flow gone is forgotten.
    pub fn flows(&mut self, ends: &[End]) {
        self.flows.retain(|(flow, _)| ends.contains(flow));
        for &end in ends {
            if self.flows.iter().all(|&(flow, _)| flow != end) {
                let pinned = self.pinned.filter(|p| (p.to, p.port) == (end.to, end.port));
                self.flows.push((end, pinned.map(|p| p.first)));
            }
        }
    }

    /// The ports a round leaves out, since a flow of the VPN's may hold
    /// them, so conntrack would give a probe from one another port and the
    /// probe would cross the line as some other flow: the pinned pair, the
    /// pair each live flow was pinned to, as one pinned before a move or a
    /// let-go still is, and each flow's own port, which it keeps unpinned.
    pub fn held(&self) -> Vec<u16> {
        let pairs = self
            .pinned
            .map(|p| p.first)
            .into_iter()
            .chain(self.flows.iter().filter_map(|&(_, first)| first))
            .flat_map(|first| [first, first.saturating_add(1)]);
        let own = self.flows.iter().map(|(flow, _)| flow.from_port);
        let mut held: Vec<u16> = pairs.chain(own).collect();
        held.sort_unstable();
        held.dedup();
        held
    }

    /// Takes the network the primary uplink is on: a pin measured on
    /// another lets go, since the line it split isn't this one's, a round
    /// out reads the old one, and the VPN's flows are probed for afresh.
    /// The first network named only names where the pin already is, and
    /// a link the router hasn't answered on yet, named by itself, learns
    /// its network a moment later: the same network, not another. A round
    /// may go at once, since the VPN opens a new socket on a new network.
    /// Gives the first of the pair let go.
    pub fn network(&mut self, network: &str) -> Option<u16> {
        if network.starts_with("link ") {
            return None;
        }
        let was = self.on.replace(network.to_owned())?;
        if was == network {
            return None;
        }
        self.stale = self.probing;
        self.coming = None;
        self.kept = false;
        self.asked = None;
        self.seen.clear();
        self.pinned.take().map(|p| p.first)
    }

    /// Takes up a pin kept across a restart, measured on `network`, so its
    /// rule is put back and a round leaves out the pair the VPN's live
    /// flow was given before the restart.
    pub fn resume(&mut self, pin: Pin, network: String) {
        self.pinned = Some(pin);
        self.on = Some(network);
    }

    /// Lets the pin go because its rule was refused, giving its first.
    pub fn refused(&mut self) -> Option<u16> {
        self.coming = None;
        self.pinned.take().map(|p| p.first)
    }

    /// The network the pin in force was measured on.
    pub fn on(&self) -> Option<&str> {
        self.on.as_deref()
    }

    /// Whether a round toward `end` that read `delays` can judge the pin
    /// in force. It can't when it heard too few ports, as with the network
    /// down, or, toward the pin's own server, when it read every port
    /// slower than the pin read, or read the line splitting but no two
    /// adjacent ports quick: the round leaves the pin's pair out (`held`),
    /// so it can't tell a pair still quick from one that went slow.
    fn judges(&self, end: End, delays: &BTreeMap<u16, f64>) -> bool {
        let Some(pin) = self.pinned else {
            return true;
        };
        if delays.len() < HEARD {
            return false;
        }
        if (pin.to, pin.port) != (end.to, end.port) {
            return true;
        }
        let quickest = delays.values().copied().fold(f64::INFINITY, f64::min);
        let slowest = delays.values().copied().fold(0.0, f64::max);
        let splits = slowest > quickest + MARGIN_MS;
        quickest <= pin.ms + MARGIN_MS && !(splits && choose(delays).is_none())
    }

    /// Takes what a round toward `end` read of each port, to take hold
    /// from `from`, once the round's probes have let their ports go
    /// (`settle`). A round that can't judge the pin in force (`judges`)
    /// keeps it, but only once: the next such round lets it go, so a pair
    /// whose way went slow or began dropping isn't kept on a delay read
    /// long ago, and the round after, the pair no longer held out, reads
    /// every port afresh. A round that read fewer than `HEARD` ports
    /// chooses no pin, and one whose network changed while it was out
    /// counts for nothing. Gives the first of a pair let go at once, since
    /// no probe holds a port the VPN would then take.
    pub fn found(&mut self, end: End, delays: &BTreeMap<u16, f64>, from: Instant) -> Option<u16> {
        self.probing = false;
        if std::mem::take(&mut self.stale) {
            return None;
        }
        if !self.judges(end, delays) {
            if !std::mem::replace(&mut self.kept, true) {
                return None;
            }
            self.kept = false;
            self.coming = None;
            return self.pinned.take().map(|p| p.first);
        }
        self.kept = false;
        if delays.len() < HEARD {
            return None;
        }
        let pin = choose(delays).map(|first| {
            let ms = |port: u16| delays.get(&port).copied().unwrap_or(f64::INFINITY);
            Pin {
                first,
                to: end.to,
                port: end.port,
                ms: ms(first).max(ms(first.saturating_add(1))),
            }
        });
        self.coming = Some((pin, from));
        None
    }

    /// Puts the pin a round found in force once its time comes, and says
    /// so when the pin moved.
    pub fn take_hold(&mut self, now: Instant) -> Option<Moved> {
        let (pin, from) = self.coming?;
        if now < from {
            return None;
        }
        self.coming = None;
        let was = std::mem::replace(&mut self.pinned, pin).map(|p| p.first);
        (was != pin.map(|p| p.first)).then_some(Moved { was })
    }

    /// Takes how many flows have come back to the pin's rule, read now,
    /// and lets the pin go once `RETRIED` have, one look after another: a
    /// look with none back starts the count over, as does a count that
    /// fell, as a switch's does. Gives the pin let go and how many came
    /// back.
    pub fn heed(&mut self, retried: u64) -> Option<(Moved, u64)> {
        let was = self.retried.replace(retried);
        let first = self.pinned?.first;
        let back = retried.saturating_sub(was?);
        if back == 0 {
            self.failing = 0;
            return None;
        }
        self.failing += back;
        if self.failing < RETRIED {
            return None;
        }
        self.pinned = None;
        Some((
            Moved { was: Some(first) },
            std::mem::take(&mut self.failing),
        ))
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
        let pin = Pin {
            first: 30_003,
            ..pin_at(30_006)
        };
        assert_eq!(
            chain("shaper", Some(&pin)),
            "flush chain inet shaper pin\n\
             add rule inet shaper pin oifname @uplinks meta mark @vpns meta nfproto ipv4 \
             meta l4proto udp ip daddr 192.0.2.2 udp dport 443 \
             ip daddr . udp dport . udp sport @pintries counter name \"pinretried\"\n\
             add rule inet shaper pin oifname @uplinks meta mark @vpns meta nfproto ipv4 \
             meta l4proto udp ip daddr 192.0.2.2 udp dport 443 \
             add @pintries { ip daddr . udp dport . udp sport timeout 20s } \
             masquerade to :30003-30004\n",
            "the chain isn't emptied, counting flows to the pin's server that come back, \
             and given the pin's ports"
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

    /// A pin of `first` toward `end`'s server, read at 24 ms.
    fn pin_at(first: u16) -> Pin {
        Pin {
            first,
            to: Ipv4Addr::new(192, 0, 2, 2),
            port: 443,
            ms: 24.0,
        }
    }

    /// What a round reads of every port tried but `held`: the `quick` ones
    /// at 24 ms, the rest at `rest`.
    fn read(quick: &[u16], rest: f64, held: &[u16]) -> BTreeMap<u16, f64> {
        (FIRST_PORT..FIRST_PORT + PORTS)
            .filter(|port| !held.contains(port))
            .map(|port| (port, if quick.contains(&port) { 24.0 } else { rest }))
            .collect()
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
        pinning.found(end(36_284), &read(&[30_000, 30_001], 66.0, &[]), settled);
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
            Some(pin_at(30_000)),
            "the round's pair wasn't pinned toward its server, at its slower port's delay"
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
    }

    #[test]
    fn a_round_keeps_a_pin_it_cant_judge_once_and_lets_it_go_the_next_time() {
        let later = Instant::now();
        let held = [30_000, 30_001];
        for (heard, why) in [
            (BTreeMap::new(), "a round that heard nothing"),
            (
                read(&[30_004, 30_005], 66.0, &held)
                    .into_iter()
                    .take(HEARD - 1)
                    .collect(),
                "a round that heard too few ports",
            ),
            (
                read(&[], 66.0, &held),
                "a round that read every other port slower than the pin",
            ),
            (
                read(&[30_004, 30_008], 66.0, &held),
                "a round that read the line split with no two adjacent ports quick",
            ),
        ] {
            let mut pinning = Pinning {
                pinned: Some(pin_at(30_000)),
                ..Pinning::default()
            };
            assert_eq!(
                pinning.found(end(41_000), &heard, later),
                None,
                "{why} let go"
            );
            assert_eq!(pinning.take_hold(later), None, "{why} moved the pin");
            assert_eq!(pinning.pinned, Some(pin_at(30_000)), "{why} lost the pin");
            assert_eq!(
                pinning.found(end(41_000), &heard, later),
                Some(30_000),
                "{why}, twice running, kept a pin read long ago"
            );
            assert_eq!(pinning.pinned, None, "{why}, twice running, left the pin");
        }
    }

    #[test]
    fn a_round_moves_a_pin_it_can_judge() {
        let later = Instant::now();
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_000)),
            ..Pinning::default()
        };
        let held = pinning.held();
        assert_eq!(
            pinning.found(end(41_000), &read(&[], 66.0, &held), later),
            None,
            "a pin a round couldn't judge wasn't kept once"
        );
        let elsewhere = End {
            to: Ipv4Addr::new(198, 51, 100, 7),
            ..end(41_000)
        };
        pinning.found(elsewhere, &read(&[], 66.0, &held), later);
        assert_eq!(
            pinning.take_hold(later),
            Some(Moved { was: Some(30_000) }),
            "a round toward another server, reading one way, kept the old server's pin"
        );
        pinning.pinned = Some(pin_at(30_000));
        assert_eq!(
            pinning.found(end(41_000), &read(&[], 66.0, &held), later),
            None,
            "a round that judged between two that couldn't didn't start the count over"
        );
        pinning.found(end(41_000), &read(&[30_010, 30_011], 66.0, &held), later);
        assert_eq!(
            pinning.take_hold(later),
            Some(Moved { was: Some(30_000) }),
            "a round that read another quick pair didn't move the pin"
        );
        pinning.found(end(41_000), &read(&[], 24.0, &pinning.held()), later);
        assert_eq!(
            pinning.take_hold(later),
            Some(Moved { was: Some(30_010) }),
            "a round that read every port as quick as the pin didn't let it go"
        );
        assert_eq!(pinning.pinned, None, "a round that read alike kept a pin");
    }

    #[test]
    fn a_round_leaves_out_every_port_the_vpns_flows_hold() {
        let mut pinning = Pinning::default();
        pinning.flows(&[end(30_020)]);
        pinning.pinned = Some(pin_at(30_006));
        pinning.flows(&[end(30_020), end(41_000)]);
        pinning.pinned = Some(pin_at(30_010));
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 30_010, 30_011, 30_020, 41_000],
            "a port a live flow holds, by its pin then or its own, was probed"
        );
        pinning.flows(&[end(41_000)]);
        pinning.pinned = None;
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 41_000],
            "a pair pinned before a let-go, still held by its flow, was probed"
        );
        pinning.flows(&[]);
        assert_eq!(
            pinning.held(),
            Vec::<u16>::new(),
            "a flow gone still held its ports"
        );
    }

    #[test]
    fn a_pin_lets_go_on_another_network_but_not_on_the_first_named_or_a_links_own() {
        let mut pinning = Pinning::default();
        let earlier = Instant::now();
        assert_eq!(
            pinning.ask(&[end(36_284)], false, earlier),
            Some(end(36_284)),
            "the VPN's flow wasn't probed for"
        );
        pinning.found(end(36_284), &read(&[30_006, 30_007], 66.0, &[]), earlier);
        pinning.take_hold(earlier);
        pinning.probing = true;
        assert_eq!(
            pinning.network("link eth1"),
            None,
            "a link named by itself let go"
        );
        assert_eq!(
            pinning.network("aa:bb"),
            None,
            "the first network named let go"
        );
        assert_eq!(
            pinning.network("link eth1"),
            None,
            "a link named by itself, after its router answered, let go"
        );
        assert_eq!(pinning.network("aa:bb"), None, "the same network let go");
        assert_eq!(
            pinning.network("cc:dd"),
            Some(30_006),
            "a pin measured on another network stayed"
        );
        assert_eq!(pinning.pinned, None, "the pin was said let go but kept");
        let later = Instant::now();
        pinning.found(end(41_000), &read(&[30_000, 30_001], 66.0, &[]), later);
        assert_eq!(
            pinning.take_hold(later),
            None,
            "a round out when the network changed pinned what it read of the old one"
        );
        assert_eq!(
            pinning.ask(&[end(36_284)], false, later),
            Some(end(36_284)),
            "the VPN's flow, probed for minutes ago, wasn't probed for afresh on the new network"
        );
    }

    #[test]
    fn a_pin_kept_across_a_restart_reads_as_it_was_saved() {
        let pin = Pin {
            ms: 24.5,
            ..pin_at(30_006)
        };
        let text = saved(&pin, "00:00:5e:00:53:01", "b0");
        assert_eq!(
            restored(&text, "b0"),
            Some((pin, "00:00:5e:00:53:01".to_owned())),
            "a saved pin read back as something else"
        );
        assert_eq!(
            restored(&text, "b1"),
            None,
            "a pin from another boot read back"
        );
        assert_eq!(
            restored(&text, ""),
            None,
            "a pin read back with the boot unknown"
        );
        for bad in [
            "",
            "b0 30006 192.0.2.2 443 24",
            "b0 40000 192.0.2.2 443 24 aa",
            "b0 30031 192.0.2.2 443 24 aa",
            "b0 x y z w v",
        ] {
            assert_eq!(restored(bad, "b0"), None, "{bad:?} read as a pin");
        }
        let mut pinning = Pinning::default();
        pinning.resume(pin, "aa:bb".to_owned());
        pinning.flows(&[end(41_000)]);
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 41_000],
            "a flow from before the restart, given the kept pin's pair, had a port probed"
        );
        assert_eq!(
            pinning.network("cc:dd"),
            Some(30_006),
            "a kept pin measured on another network stayed"
        );
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

    /// A pin in force at 30 006, the count of flows back already read once.
    fn pinned_at(retried: u64) -> Pinning {
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_006)),
            ..Pinning::default()
        };
        assert_eq!(pinning.heed(retried), None, "a first read let go");
        pinning
    }

    const LET_GO: Option<(Moved, u64)> = Some((Moved { was: Some(30_006) }, 2));

    #[test]
    fn a_pin_lets_go_once_the_vpns_flows_keep_coming_back() {
        let mut pinning = pinned_at(0);
        assert_eq!(pinning.heed(1), None, "one flow back let go at once");
        assert_eq!(
            pinning.heed(2),
            LET_GO,
            "a second look with one back kept the pin"
        );
        assert_eq!(pinning.pinned, None, "the pin was said let go but kept");
        assert_eq!(pinning.heed(9), None, "no pin let go again");

        let mut burst = pinned_at(0);
        assert_eq!(burst.heed(2), LET_GO, "two back in a look kept the pin");

        // A flow that went through never comes back, so looks between
        // single returns, as a VPN's few reconnects give, start over.
        let mut spaced = pinned_at(0);
        for (look, retried) in [1, 1, 2, 2, 3, 3].into_iter().enumerate() {
            assert_eq!(
                spaced.heed(retried),
                None,
                "look {look}, one back after none, let go"
            );
        }

        // A switch loads the table afresh, its count from nought.
        let mut switched = pinned_at(40);
        assert_eq!(switched.heed(0), None, "a count that fell let go");
        assert_eq!(switched.heed(1), None, "one back after a switch let go");

        let mut unpinned = Pinning::default();
        for retried in [0, 5, 10] {
            assert_eq!(unpinned.heed(retried), None, "no pin let go");
        }
    }

    #[test]
    fn the_count_is_read_by_name_and_the_journal_says_why_a_pin_let_go() {
        let json = r#"{"nftables":[{"metainfo":{}},
            {"counter":{"family":"inet","name":"pinretried","table":"shaper",
             "packets":4,"bytes":512}}]}"#;
        assert_eq!(retried(json), Some(4), "the count wasn't read");
        assert_eq!(retried(""), None, "an unread counter read nought");
        assert_eq!(
            dropped(30_006, 3),
            "The VPN's next connection leaves from any port again, no longer 30006 or 30007: \
             its new connections came back unsent 3 times, as when other flows hold both \
             ports toward its server",
            "the journal doesn't say why the pin let go"
        );
        assert!(
            REMEMBERED + Duration::from_secs(5) < settle(""),
            "a flow is remembered past conntrack's timeout for unanswered UDP"
        );
    }
}
