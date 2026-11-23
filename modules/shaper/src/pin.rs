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

/// How many times the VPN's flows may come back to the pin's rule, each
/// within `REMEMBERED` of the one before, before the pin lets go. With other flows holding
/// both pinned ports toward the server, as a third socket in a burst of
/// reconnects might, NAT finds no port free and the kernel drops the new
/// flow, so the VPN stays down until conntrack frees one, two minutes for
/// a flow that was answered. NAT meets a flow it let through only at its
/// first packet, so a flow that comes back was dropped: the VPN sends again
/// within seconds, and letting go costs only the quick way. A new flow's
/// first two datagrams sent at once from two cores can both meet NAT before
/// conntrack confirms the first, and the second then counts as come back
/// (pin round 10's F3); a tunnel's handshake sends one and waits for its
/// answer, so that is rare, and one such race alone never lets go.
pub const RETRIED: u64 = 2;

/// How often the shaper reads the VPN's flows and the count of those that
/// came back to the pin's rule.
pub const LOOK: Duration = Duration::from_secs(5);

/// How long the pin's rule remembers a flow it gave a port, to know it
/// when it comes back: under conntrack's 30 s for unanswered UDP, so a flow
/// that went through and then idled until its entry went is never taken
/// for one dropped.
pub const REMEMBERED: Duration = Duration::from_secs(20);

/// How long a VPN flow missing from the set of its flows is still held, as
/// the set's own timeout for one idle (shaper.nft): a switch that changes
/// any table reloads the shaper's, which empties the set until each flow
/// sends again, and a flow forgotten then would be learned again as the
/// current pin's, its own pair probed under it (pin round 8's F2, F3 and
/// F5). One gone for good holds a pair out of rounds that much longer.
pub const FORGOTTEN: Duration = Duration::from_secs(300);

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
/// port it came from, with or without the timeout it was added with. A
/// listing that can't be read gives nothing, never no flows.
pub fn ends(json: &str) -> Option<Vec<End>> {
    let read = serde_json::from_str::<Value>(json).ok()?;
    let all = read["nftables"].as_array()?;
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
    Some(seen)
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
/// came back since, each within `REMEMBERED` of the last, and when the
/// last did, whether the last round kept a pin it
/// couldn't judge, the network the pin was measured on, and the VPN's
/// flows now, each with the first of the pair pinned when it was first
/// seen, the pin a move or let-go replaced and when, and each flow first
/// seen just after that, with the replaced pair it may hold instead.
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
    back_at: Option<Instant>,
    kept: bool,
    on: Option<(String, String)>,
    flows: Vec<(End, Option<u16>)>,
    missing: Vec<(End, Instant)>,
    replaced: Option<(u16, Instant)>,
    straddled: Vec<(End, u16)>,
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
pub const REFUSED: &str = "nftables didn't take its rule";

/// What the shaper keeps of the pin across a restart, so a switch neither
/// loses the pin nor has the next round probe a port a live flow holds:
/// the pin in force, the link and network it was measured on, whether a
/// round already kept it unjudged, when the last round was asked, in
/// seconds since the epoch, and each live flow with the pair it was given,
/// if any, so one from before any pin isn't given one after.
#[derive(Debug, Default, PartialEq)]
pub struct Restart {
    pub pin: Option<Pin>,
    pub on: Option<(String, String)>,
    pub kept: bool,
    pub asked: Option<u64>,
    pub flows: Vec<(End, Option<u16>)>,
}

/// What `saved` writes, line by line, after the boot it was written in,
/// since a boot starts the VPN's flows afresh.
pub fn saved(kept: &Restart, boot: &str) -> String {
    let mut out = format!("boot {boot}\n");
    if let Some((link, network)) = &kept.on {
        let _ = writeln!(out, "on {link} {network}");
    }
    if let Some(Pin {
        first,
        to,
        port,
        ms,
    }) = kept.pin
    {
        let _ = writeln!(out, "pin {first} {to} {port} {ms}");
    }
    if kept.kept {
        out.push_str("kept\n");
    }
    if let Some(asked) = kept.asked {
        let _ = writeln!(out, "asked {asked}");
    }
    for (
        End {
            to,
            port,
            from_port,
        },
        first,
    ) in &kept.flows
    {
        let first = first.map_or_else(|| "-".to_owned(), |f| f.to_string());
        let _ = writeln!(out, "flow {to} {port} {from_port} {first}");
    }
    out
}

/// What `saved` wrote in this boot, `boot`, or nothing where the text
/// was written in another boot or any line of it can't be read, as if
/// nothing were kept.
pub fn restored(text: &str, boot: &str) -> Option<Restart> {
    let mut lines = text.lines();
    if boot.is_empty() || lines.next()?.strip_prefix("boot ")? != boot {
        return None;
    }
    let pair = |first: &str| {
        let first: u16 = first.parse().ok()?;
        (FIRST_PORT..FIRST_PORT + PORTS - 1)
            .contains(&first)
            .then_some(first)
    };
    let mut kept = Restart::default();
    for line in lines {
        // A network a link names by itself, before its router answers, is
        // "link <name>", so the network is the rest of its line.
        if let Some(on) = line.strip_prefix("on ") {
            let (link, network) = on
                .split_once(' ')
                .filter(|(link, network)| !link.is_empty() && !network.is_empty())?;
            kept.on = Some((link.to_owned(), network.to_owned()));
            continue;
        }
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["pin", first, to, port, ms] => {
                kept.pin = Some(Pin {
                    first: pair(first)?,
                    to: to.parse().ok()?,
                    port: port.parse().ok()?,
                    ms: ms.parse().ok().filter(|ms: &f64| ms.is_finite())?,
                });
            }
            ["kept"] => kept.kept = true,
            ["asked", at] => kept.asked = Some(at.parse().ok()?),
            ["flow", to, port, from_port, first] => kept.flows.push((
                End {
                    to: to.parse().ok()?,
                    port: port.parse().ok()?,
                    from_port: from_port.parse().ok()?,
                },
                if *first == "-" {
                    None
                } else {
                    Some(pair(first)?)
                },
            )),
            _ => return None,
        }
    }
    Some(kept)
}

/// Why a pin measured on another network lets go.
pub const ELSEWHERE: &str = "it was measured on another network";

/// Why a pin whose link no router has named for `APART` lets go.
pub const UNNAMED: &str = "no router has said which network its link is on for ten minutes";

/// Whether `network` names no network: a link the router hasn't answered
/// on is named by itself.
pub fn unnamed(network: &str) -> bool {
    network.starts_with("link ")
}

impl Pinning {
    /// The flow to probe toward now, if any: one from a socket the VPN
    /// opened since the last round, at the start too, while no call is
    /// on, no round is out and none ran within `APART`. A flow left
    /// unprobed for a call or the spacing is asked for again later. While
    /// the VPN has a flow to the server it's pinned toward, only a flow to
    /// that server is: any packet of the VPN's mark joins its flows, and a
    /// round toward another server, as a stray datagram given the mark
    /// would ask, would take the tunnel's pin to pin a server it doesn't
    /// ride (pin round 10's F5). Once none goes there, as when the VPN
    /// moves to another server, a flow to any is probed toward.
    pub fn ask(&mut self, ends: &[End], calling: bool, now: Instant) -> Option<End> {
        if calling
            || self.probing
            || self.coming.is_some()
            || self.asked.is_some_and(|at| now - at < APART)
        {
            return None;
        }
        let server = |e: &End| (e.to, e.port);
        let ridden = self
            .pinned
            .map(|p| (p.to, p.port))
            .filter(|&pinned| ends.iter().any(|e| server(e) == pinned));
        let to = *ends
            .iter()
            .find(|e| !self.seen.contains(e) && ridden.is_none_or(|pinned| server(e) == pinned))?;
        self.seen = ends.to_vec();
        self.probing = true;
        self.asked = Some(now);
        Some(to)
    }

    /// Takes the VPN's flows at `now`: each new one is remembered with the
    /// pair pinned toward its server when it was first seen, since NAT
    /// gave it one of that pair, and one missing since `FORGOTTEN` ago is
    /// forgotten. One first seen within two looks of the pin moving or
    /// letting go may have opened before, under the pair replaced, so it's
    /// remembered with that pair too (pin round 10's F1); the two looks
    /// allow for a look late by a step. That isn't kept across a restart,
    /// which costs at most a round's probe misread on the pair.
    pub fn flows(&mut self, ends: &[End], now: Instant) {
        self.missing.retain(|(flow, _)| !ends.contains(flow));
        for &(flow, _) in &self.flows {
            if !ends.contains(&flow) && self.missing.iter().all(|&(gone, _)| gone != flow) {
                self.missing.push((flow, now));
            }
        }
        let gone: Vec<End> = self
            .missing
            .iter()
            .filter(|&&(_, since)| now.saturating_duration_since(since) >= FORGOTTEN)
            .map(|&(flow, _)| flow)
            .collect();
        self.missing.retain(|(flow, _)| !gone.contains(flow));
        self.flows.retain(|(flow, _)| !gone.contains(flow));
        self.straddled.retain(|(flow, _)| !gone.contains(flow));
        let replaced = self
            .replaced
            .filter(|&(_, at)| now.saturating_duration_since(at) <= LOOK * 2)
            .map(|(first, _)| first);
        for &end in ends {
            if self.flows.iter().all(|&(flow, _)| flow != end) {
                let pinned = self.pinned.filter(|p| (p.to, p.port) == (end.to, end.port));
                let first = pinned.map(|p| p.first);
                self.flows.push((end, first));
                if let Some(old) = replaced.filter(|&old| Some(old) != first) {
                    self.straddled.push((end, old));
                }
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
            .chain(self.straddled.iter().map(|&(_, first)| first))
            .flat_map(|first| [first, first.saturating_add(1)]);
        let own = self.flows.iter().map(|(flow, _)| flow.from_port);
        let mut held: Vec<u16> = pairs.chain(own).collect();
        held.sort_unstable();
        held.dedup();
        held
    }

    /// Takes the primary uplink, `link`, and the network it's on: a pin
    /// measured on another lets go, since the line it split isn't this
    /// one's, a round out reads the old one, and the VPN's flows are
    /// probed for afresh. The first named only names where the pin already
    /// is. A link the router hasn't answered on is named by itself, which
    /// names no network yet, on that link or another: a link plugged in
    /// beside or roamed back to is unnamed for its first moments, so the
    /// pin waits for the router's name to judge it, keeping a good pin
    /// through docking onto the same network. A round may go at once,
    /// since the VPN opens a new socket on a new network. Gives the first
    /// of the pair let go.
    pub fn network(&mut self, link: &str, network: &str) -> Option<u16> {
        let now = (link.to_owned(), network.to_owned());
        let Some((was_link, was)) = self.on.clone() else {
            self.on = Some(now);
            return None;
        };
        // A router that hasn't answered, or stopped, leaves the network
        // the pin was measured on as it was, so a move through an unnamed
        // moment to another router still lets go, and one to the same
        // router keeps it (pin round 8's F1 and F4, round 7's F1).
        if unnamed(network) {
            return None;
        }
        // The same network by another link is where the pin is, and the
        // link it's now on is the one whose unnamed moments are its own.
        if was == network {
            self.on = Some(now);
            return None;
        }
        self.on = Some(now);
        if was_link == link && unnamed(&was) {
            return None;
        }
        self.stale = self.probing;
        self.coming = None;
        self.kept = false;
        self.asked = None;
        self.seen.clear();
        self.pinned.take().map(|p| p.first)
    }

    /// Lets the pin go at `now` where its link has gone `APART` with no
    /// router naming its network (pin round 9's F3 and F5): a pin kept
    /// through a moment unnamed would otherwise be kept as long as the
    /// router stays silent, on a line nothing says is the one it split.
    /// The spacing and the flows probed for are left as they are, since a
    /// silent router names no new network to probe on. Gives the first of
    /// the pair let go.
    pub fn unheard(&mut self, now: Instant) -> Option<u16> {
        self.coming = None;
        self.kept = false;
        let first = self.pinned.take()?.first;
        self.replaced = Some((first, now));
        Some(first)
    }

    /// Takes up what was kept across a restart (`Restart`), the last round
    /// asked at `asked`: the pin's rule is put back, a round still waits
    /// out the spacing and its kept-once mark, and leaves out every pair
    /// the VPN's live flows were given.
    pub fn resume(&mut self, kept: Restart, asked: Option<Instant>) {
        self.pinned = kept.pin;
        self.on = kept.on;
        self.kept = kept.kept;
        self.asked = asked;
        self.flows = kept.flows;
    }

    /// What to keep across a restart, the last round asked at `asked`, in
    /// seconds since the epoch.
    pub fn kept(&self, asked: Option<u64>) -> Restart {
        Restart {
            pin: self.pinned,
            on: self.on.clone(),
            kept: self.kept,
            asked,
            flows: self.flows.clone(),
        }
    }

    /// When the last round was asked, if one was.
    pub fn asked(&self) -> Option<Instant> {
        self.asked
    }

    /// Lets the pin go because its rule was refused, giving its first.
    pub fn refused(&mut self) -> Option<u16> {
        self.coming = None;
        self.pinned.take().map(|p| p.first)
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
    /// long ago, and the round after reads the pair afresh once no live
    /// flow of the VPN's holds it (pin round 10's F2). A pair let go is
    /// counted replaced until `from`, when this round's pin would have
    /// taken hold, since that's the instant `found` knows: flows first seen
    /// until then are held to it too, a few more than opened under it,
    /// which costs a later round at most those two ports. A round that
    /// read fewer than `HEARD` ports
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
            let first = self.pinned.take().map(|p| p.first)?;
            self.replaced = Some((first, from));
            return Some(first);
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
        let moved = was != pin.map(|p| p.first);
        if let Some(first) = was.filter(|_| moved) {
            self.replaced = Some((first, now));
        }
        moved.then_some(Moved { was })
    }

    /// Takes how many flows have come back to the pin's rule, read at
    /// `now`, and lets the pin go once `RETRIED` have, each within
    /// `REMEMBERED` of the one before: a VPN that sends again more slowly
    /// than the shaper looks still counts (pin round 10's F4), while a
    /// return longer after the last, as a VPN's few reconnects in a day
    /// give, starts the count over, as does a count that fell, as a
    /// switch's does. Gives the pin let go and how many came back.
    pub fn heed(&mut self, retried: u64, now: Instant) -> Option<(Moved, u64)> {
        let was = self.retried.replace(retried);
        let first = self.pinned?.first;
        let was = was?;
        if retried < was {
            self.failing = 0;
            self.back_at = None;
            return None;
        }
        let back = retried - was;
        if back == 0 {
            return None;
        }
        let apart = self
            .back_at
            .replace(now)
            .is_none_or(|at| now.saturating_duration_since(at) > REMEMBERED);
        if apart {
            self.failing = 0;
        }
        self.failing += back;
        if self.failing < RETRIED {
            return None;
        }
        self.pinned = None;
        self.back_at = None;
        self.replaced = Some((first, now));
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
    fn a_quick_way_spread_past_the_margin_still_pins_a_pair_of_its_own() {
        // The quick way can read from 22 to 35 ms in one round, wider than
        // `MARGIN_MS` (pin round 1's F17). A quick port past the margin is
        // left out as a slow one would be, which costs the pairs it is in,
        // and the pin is still a pair of the quick way's, never one touching
        // the slow way's 65 ms.
        let read = delays(&[
            (30_000, 35.0),
            (30_001, 22.0),
            (30_002, 66.0),
            (30_003, 34.0),
            (30_004, 29.0),
            (30_005, 31.0),
            (30_006, 65.0),
        ]);
        assert_eq!(
            choose(&read),
            Some(30_004),
            "the pair within the margin wasn't pinned"
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
            Some(vec![
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
            ]),
            "each flow once, in order"
        );
        assert_eq!(ends(""), None, "a refused listing read as no flows");
        assert_eq!(
            ends("{}"),
            None,
            "a listing of something else read as no flows"
        );
        assert_eq!(
            ends(r#"{"nftables":[{"set":{"name":"vpn_ends"}}]}"#),
            Some(Vec::new()),
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
    fn a_flow_first_seen_just_after_a_move_holds_the_pair_it_replaced_too() {
        let start = Instant::now();
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_006)),
            coming: Some((Some(pin_at(30_000)), start)),
            ..Pinning::default()
        };
        assert_eq!(
            pinning.take_hold(start),
            Some(Moved { was: Some(30_006) }),
            "the pin didn't move"
        );
        pinning.flows(&[end(41_000)], start + LOOK);
        assert_eq!(
            pinning.held(),
            vec![30_000, 30_001, 30_006, 30_007, 41_000],
            "a flow first seen a look after the move isn't held to the pair it replaced"
        );
        pinning.flows(&[end(41_000), end(42_000)], start + LOOK * 3);
        assert_eq!(
            pinning.held(),
            vec![30_000, 30_001, 30_006, 30_007, 41_000, 42_000],
            "a flow first seen three looks on was held to the pair replaced"
        );
        let later = start + LOOK * 4 + FORGOTTEN;
        pinning.flows(&[end(42_000)], start + LOOK * 4);
        pinning.flows(&[end(42_000)], later);
        assert_eq!(
            pinning.held(),
            vec![30_000, 30_001, 42_000],
            "a flow forgotten still holds the pair it straddled"
        );

        // A pin let go for flows coming back replaces its pair too.
        let mut dropped = pinned_at(0, start);
        assert_eq!(
            dropped.heed(2, look(start, 1)),
            LET_GO,
            "two flows back didn't let the pin go"
        );
        dropped.flows(&[end(43_000)], look(start, 2));
        assert_eq!(
            dropped.held(),
            vec![30_006, 30_007, 43_000],
            "a flow first seen a look after a let-go isn't held to the pair let go"
        );
    }

    #[test]
    fn a_pin_on_a_link_no_router_names_lets_go_but_keeps_the_spacing() {
        let start = Instant::now();
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_006)),
            asked: Some(start),
            seen: vec![end(36_284)],
            ..Pinning::default()
        };
        assert!(
            unnamed("link eth1"),
            "a link named by itself named a network"
        );
        assert!(!unnamed("aa:bb"), "a router's name named no network");
        assert_eq!(
            pinning.unheard(start + APART),
            Some(30_006),
            "the pin wasn't let go"
        );
        assert_eq!(pinning.unheard(start + APART), None, "no pin let go twice");
        assert_eq!(
            pinning.ask(&[end(36_284)], false, start + APART / 2),
            None,
            "a silent router set off a round within the spacing"
        );
        assert_eq!(
            pinning.held(),
            Vec::<u16>::new(),
            "the pair let go is held with no flow first seen after it"
        );
    }

    #[test]
    fn while_the_vpn_rides_its_pinned_server_no_other_is_probed_toward() {
        let start = Instant::now();
        let stray = End {
            to: Ipv4Addr::new(198, 51, 100, 7),
            port: 53,
            from_port: 50_000,
        };
        // The tunnel's socket, probed for by the round that pinned it.
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_006)),
            seen: vec![end(36_284)],
            ..Pinning::default()
        };
        assert_eq!(
            pinning.ask(&[end(36_284), stray], false, start),
            None,
            "a stray flow of the VPN's mark was probed toward while it rode its pin"
        );
        assert_eq!(
            pinning.ask(&[end(36_284), stray, end(41_000)], false, start),
            Some(end(41_000)),
            "a new socket to the pinned server wasn't probed toward beside a stray"
        );

        let mut moved = Pinning {
            pinned: Some(pin_at(30_006)),
            ..Pinning::default()
        };
        assert_eq!(
            moved.ask(&[stray], false, start),
            Some(stray),
            "a VPN gone from its pinned server wasn't probed toward where it went"
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
        let now = Instant::now();
        pinning.flows(&[end(30_020)], now);
        pinning.pinned = Some(pin_at(30_006));
        pinning.flows(&[end(30_020), end(41_000)], now);
        pinning.pinned = Some(pin_at(30_010));
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 30_010, 30_011, 30_020, 41_000],
            "a port a live flow holds, by its pin then or its own, was probed"
        );
        let later = now + FORGOTTEN;
        pinning.flows(&[end(41_000)], now);
        pinning.flows(&[end(41_000)], later);
        pinning.pinned = None;
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 41_000],
            "a pair pinned before a let-go, still held by its flow, was probed"
        );
        // A reloaded table lists no flow until each sends again, which
        // takes back the pair it was given then, not the pin's now.
        pinning.pinned = Some(pin_at(30_014));
        pinning.flows(&[], later);
        pinning.flows(&[end(41_000)], later + Duration::from_secs(1));
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 30_014, 30_015, 41_000],
            "a flow missing from one listing lost the pair it was given"
        );
        pinning.pinned = None;
        pinning.flows(&[], later + FORGOTTEN);
        assert_eq!(
            pinning.held(),
            vec![30_006, 30_007, 41_000],
            "a flow missing for less than the set's timeout was forgotten"
        );
        pinning.flows(&[], later + FORGOTTEN * 2);
        assert_eq!(
            pinning.held(),
            Vec::<u16>::new(),
            "a flow gone still held its ports"
        );
    }

    #[test]
    fn a_pin_lets_go_on_another_network_but_not_on_the_first_named_or_an_unnamed_one() {
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
        let named = [
            ("eth1", "link eth1", "the first named"),
            ("eth1", "aa:bb", "its router learned on the same link"),
            ("eth1", "link eth1", "its router forgotten on the same link"),
            ("eth1", "aa:bb", "the same network again"),
        ];
        for (link, network, why) in named {
            assert_eq!(
                pinning.network(link, network),
                None,
                "the pin let go on {why}"
            );
        }
        assert_eq!(
            pinning.network("wlan0", "link wlan0"),
            None,
            "a pin let go on another link before its router answered, naming no network"
        );
        assert_eq!(
            pinning.network("wlan0", "cc:dd"),
            Some(30_006),
            "a pin measured on one network stayed on another, by another link"
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
        // Docking onto the same network, a new link is unnamed until its
        // router answers, and undocking goes back through the old link's
        // own unnamed moment (pin round 8's F1 and F4, round 7's F1).
        pinning.pinned = Some(pin_at(30_010));
        for (link, network, why) in [
            ("eth0", "link eth0", "docking, before the router answered"),
            ("eth0", "cc:dd", "docking onto the same network"),
            (
                "wlan0",
                "link wlan0",
                "undocking, before the router answered",
            ),
            ("wlan0", "cc:dd", "undocking onto the same network"),
        ] {
            assert_eq!(pinning.network(link, network), None, "the pin let go {why}");
        }
        assert_eq!(
            pinning.pinned,
            Some(pin_at(30_010)),
            "the docked pin wasn't kept"
        );
        assert_eq!(
            pinning.network("wlan0", "ee:ff"),
            Some(30_010),
            "a pin measured on another network stayed"
        );
        pinning.pinned = Some(pin_at(30_020));
        assert_eq!(
            pinning.network("wlan0", "link wlan0"),
            None,
            "a pin let go when its router stopped answering"
        );
        assert_eq!(
            pinning.network("wlan0", "gg:hh"),
            Some(30_020),
            "a pin carried through a moment with no router onto another network"
        );
        // The same network by another link, then that link's own unnamed
        // moment, is still where the pin was measured (pin round 6's F1).
        pinning.pinned = Some(pin_at(30_030));
        for (link, network) in [("eth2", "gg:hh"), ("eth2", "link eth2"), ("eth2", "gg:hh")] {
            assert_eq!(
                pinning.network(link, network),
                None,
                "the pin let go on {network} by {link}, its own network by another link"
            );
        }
        assert_eq!(pinning.pinned, Some(pin_at(30_030)), "the pin wasn't kept");
    }

    #[test]
    fn a_pin_kept_across_a_restart_reads_as_it_was_saved() {
        let kept = Restart {
            pin: Some(Pin {
                ms: 24.5,
                ..pin_at(30_006)
            }),
            on: Some(("eth1".to_owned(), "00:00:5e:00:53:01".to_owned())),
            kept: true,
            asked: Some(1_794_000_000),
            flows: vec![
                (end(41_000), Some(30_010)),
                (end(41_001), Some(30_006)),
                (end(41_002), None),
            ],
        };
        let text = saved(&kept, "b0");
        assert_eq!(
            restored(&text, "b0"),
            Some(kept),
            "what was kept read back as something else"
        );
        let unnamed = Restart {
            on: Some(("eth1".to_owned(), "link eth1".to_owned())),
            ..Restart::default()
        };
        assert_eq!(
            restored(&saved(&unnamed, "b0"), "b0"),
            Some(unnamed),
            "a network a link names by itself didn't read back"
        );
        assert_eq!(
            restored(&text, "b1"),
            None,
            "what was kept in another boot read back"
        );
        assert_eq!(
            restored(&text, ""),
            None,
            "what was kept read back with the boot unknown"
        );
        assert_eq!(
            restored("boot b0\n", "b0"),
            Some(Restart::default()),
            "nothing kept"
        );
        for bad in [
            "",
            "boot b0\npin 30006 192.0.2.2 443",
            "boot b0\npin 40000 192.0.2.2 443 24",
            "boot b0\npin 30031 192.0.2.2 443 24",
            "boot b0\nflow 192.0.2.2 443 41000 29000",
            "boot b0\non eth1",
            "boot b0\non eth1 ",
            "boot b0\non  aa:bb",
            "boot b0\nwhat",
        ] {
            assert_eq!(restored(bad, "b0"), None, "{bad:?} read as something kept");
        }
    }

    #[test]
    fn a_restart_takes_back_the_pin_its_mark_its_spacing_and_every_flows_pair() {
        let mut before = Pinning::default();
        let start = Instant::now();
        before.network("eth1", "aa:bb");
        before.flows(&[end(41_000)], start);
        before.pinned = Some(pin_at(30_006));
        before.flows(&[end(41_000), end(41_001)], start);
        before.pinned = Some(pin_at(30_010));
        before.kept = true;
        let text = saved(&before.kept(Some(7)), "b0");
        let resumed = || {
            let mut after = Pinning::default();
            after.resume(restored(&text, "b0").expect("kept"), Some(start));
            after
        };
        let mut moved = resumed();
        assert_eq!(
            moved.network("eth1", "aa:bb"),
            None,
            "a kept pin let go on its own network"
        );
        assert_eq!(
            moved.network("eth1", "cc:dd"),
            Some(30_010),
            "a kept pin stayed on another network"
        );
        let mut after = resumed();
        after.flows(&[end(41_000), end(41_001), end(41_002)], start);
        assert_eq!(
            after.kept(None).flows[0],
            (end(41_000), None),
            "a flow from before any pin was given the pin's pair after the restart"
        );
        assert_eq!(
            after.held(),
            vec![30_006, 30_007, 30_010, 30_011, 41_000, 41_001, 41_002],
            "a pair a live flow was given before the restart was probed after it"
        );
        assert_eq!(
            after.pinned,
            Some(pin_at(30_010)),
            "the pin in force wasn't taken back"
        );
        assert_eq!(
            after.ask(&[end(41_003)], false, start + Duration::from_secs(60)),
            None,
            "a round went within the spacing of the last before the restart"
        );
        let unjudged = read(&[], 66.0, &after.held());
        assert_eq!(
            after.found(end(41_000), &unjudged, start),
            Some(30_010),
            "a pin kept once unjudged before the restart was kept again after it"
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

    /// A pin in force at 30 006, the count of flows back already read once
    /// at `at`.
    fn pinned_at(retried: u64, at: Instant) -> Pinning {
        let mut pinning = Pinning {
            pinned: Some(pin_at(30_006)),
            ..Pinning::default()
        };
        assert_eq!(pinning.heed(retried, at), None, "a first read let go");
        pinning
    }

    /// The shaper's looks at the count, 5 s apart, from `start`.
    fn look(start: Instant, n: u64) -> Instant {
        start + Duration::from_secs(5 * n)
    }

    const LET_GO: Option<(Moved, u64)> = Some((Moved { was: Some(30_006) }, 2));

    #[test]
    fn a_pin_lets_go_once_the_vpns_flows_keep_coming_back() {
        let start = Instant::now();
        let mut pinning = pinned_at(0, start);
        assert_eq!(
            pinning.heed(1, look(start, 1)),
            None,
            "one flow back let go at once"
        );
        assert_eq!(
            pinning.heed(2, look(start, 2)),
            LET_GO,
            "a second look with one back kept the pin"
        );
        assert_eq!(pinning.pinned, None, "the pin was said let go but kept");
        assert_eq!(pinning.heed(9, look(start, 3)), None, "no pin let go again");

        let mut burst = pinned_at(0, start);
        assert_eq!(
            burst.heed(2, look(start, 1)),
            LET_GO,
            "two back in a look kept the pin"
        );

        // A VPN that sends again every 10 s, more slowly than the shaper
        // looks, still has each return counted with the last.
        let mut slow = pinned_at(0, start);
        let looks = [1, 1, 2];
        for (n, retried) in (1..).zip(looks) {
            let said = slow.heed(retried, look(start, n));
            let want = if n == 3 { LET_GO } else { None };
            assert_eq!(said, want, "look {n}, a VPN sending again every 10 s");
        }

        // A flow that went through never comes back, so returns further
        // apart than the rule remembers, as a VPN's few reconnects give,
        // start over.
        let mut spaced = pinned_at(0, start);
        for (n, retried) in (1..).zip([1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3]) {
            assert_eq!(
                spaced.heed(retried, look(start, n)),
                None,
                "look {n}, one back 25 s after the last, let go"
            );
        }

        // A switch loads the table afresh, its count from nought.
        let mut switched = pinned_at(40, start);
        assert_eq!(
            switched.heed(41, look(start, 1)),
            None,
            "one back let go at once"
        );
        assert_eq!(
            switched.heed(0, look(start, 2)),
            None,
            "a count that fell let go"
        );
        assert_eq!(
            switched.heed(1, look(start, 3)),
            None,
            "one back after a switch was counted with one before it"
        );

        let mut unpinned = Pinning::default();
        for (n, retried) in (1..).zip([0, 5, 10]) {
            assert_eq!(
                unpinned.heed(retried, look(start, n)),
                None,
                "no pin let go"
            );
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
