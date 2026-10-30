//! Keeps the network's queue on this machine, sorted by the slices'
//! classes, so a call, music and what Joel waits on never queue behind the
//! work he started.
//!
//! The queue that fills when this machine uploads isn't its own: it's the
//! modem's or the router's, first come first served, where a call's packets
//! wait behind a build's. So on every link with a default route, the shaper
//! sends and receives a little slower than the connection can take, through CAKE,
//! and the queue forms here instead, short and sorted into CAKE's four
//! tins. How fast "a little slower" is, it finds as it goes (src/rate.rs),
//! and remembers for each network (src/memory.rs). Which tin a connection
//! goes to, nftables decides from the cgroup of the socket that opened it
//! (the table in nixos.nix): the shaper keeps that table's sets of cgroups
//! current (src/classes.rs), and its queues on each uplink (src/qdisc.rs).
//!
//! Usage: shaper run [--hosts 1.1.1.1,...] [--vpns wg0,...] [--state DIR]
//!                   [--runtime DIR] (`SHAPER_HOSTS` stands for --hosts)
//!        shaper status
//!        shaper stop

mod classes;
mod links;
mod memory;
mod probe;
mod qdisc;
mod rate;

use memory::{Held, Memory};
use probe::{Echo, Probes};
use rate::{Bounds, Controller, Direction, Settings, State, float, whole};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

const TABLE: &str = "shaper";
const STEP: Duration = Duration::from_millis(200);

/// Where a network nobody has measured starts, in kbit/s: under most home
/// connections both ways, so its first seconds of load don't queue at the
/// modem, and climbing from there within seconds when the connection takes
/// more.
const FIRST: Held = Held {
    up: 10_000,
    down: 50_000,
};
const FLOOR: Held = Held {
    up: 1_000,
    down: 5_000,
};

struct Options {
    hosts: Vec<String>,
    state: PathBuf,
    runtime: PathBuf,
    cgroups: PathBuf,
    /// Where processes are listed, and the accounts' runtime folders,
    /// where the harness says which pi sessions run a turn a person typed.
    proc: PathBuf,
    users: PathBuf,
    vpns: Vec<String>,
}

fn options(args: &[String]) -> Options {
    let mut o = Options {
        // Four operators' anycast resolvers, near anywhere, answering ping,
        // and outside the ranges WARP takes.
        hosts: ["1.1.1.1", "8.8.8.8", "9.9.9.9", "1.0.0.1"]
            .map(String::from)
            .to_vec(),
        state: "/var/lib/shaper".into(),
        runtime: "/run/shaper".into(),
        cgroups: "/sys/fs/cgroup".into(),
        proc: "/proc".into(),
        users: "/run/user".into(),
        vpns: Vec::new(),
    };
    // A network of its own, a test's among them, names its own distant hosts.
    if let Ok(hosts) = std::env::var("SHAPER_HOSTS") {
        o.hosts = hosts.split(',').map(str::to_string).collect();
    }
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().unwrap_or_default();
        match arg.as_str() {
            "--hosts" => o.hosts = value().split(',').map(str::to_string).collect(),
            "--state" => o.state = value().into(),
            "--runtime" => o.runtime = value().into(),
            "--cgroups" => o.cgroups = value().into(),
            "--proc" => o.proc = value().into(),
            "--users" => o.users = value().into(),
            "--vpns" => {
                o.vpns = value()
                    .split(',')
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            other => eprintln!("ignoring {other}"),
        }
    }
    o
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    match args.first().map(String::as_str) {
        Some("run") => run(&options(rest)),
        Some("status") => status(&options(rest).runtime),
        Some("stop") => {
            stop();
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: shaper run | status | stop");
            ExitCode::from(2)
        }
    }
}

/// Runs a command, saying what it said when it fails.
fn sh(words: &[String]) -> Result<String, String> {
    let out = Command::new(&words[0])
        .args(&words[1..])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{}: {e}", words[0]))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "{}: {}",
            words.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn read_number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// One uplink being shaped.
struct Shaped {
    uplink: links::Uplink,
    ifb: String,
    network: String,
    controller: Controller,
    probes: Probes,
    applied: (u64, u64),
    counted: (u64, u64, Instant),
    state: State,
    /// The link whose rate this one takes, when both reach the same router.
    follows: Option<String>,
    /// Whether the journal has been told of the connection's queue building now.
    told: bool,
    /// Whether a call's or a screen share's traffic crosses the link.
    real_time: qdisc::RealTime,
}

impl Shaped {
    /// What the connection has carried each way, in bytes, as each
    /// direction's CAKE counts it. The link's own counts take in copies to
    /// this network's machines as well, which never cross the connection:
    /// on 29 October they took a home upload's rate from 8 Mbit/s to 458
    /// within three hours, on a connection carrying about 11, and a cut
    /// could go no further under that than Wi-Fi's floor, so the queue
    /// formed in the router behind every session's requests.
    fn bytes(&self) -> (u64, u64) {
        let sent = |dev: &str| {
            sh(&words(&format!("tc -s -j qdisc show dev {dev}")))
                .ok()
                .and_then(|json| qdisc::shaped_bytes(&json))
                .unwrap_or(0)
        };
        (sent(&self.uplink.name), sent(&self.ifb))
    }

    /// What the link sent and received since it was last counted, in
    /// kbit/s.
    fn carried(&mut self) -> (f64, f64) {
        let (tx, rx) = self.bytes();
        let (tx0, rx0, at) = self.counted;
        let seconds = at.elapsed().as_secs_f64().max(0.001);
        self.counted = (tx, rx, Instant::now());
        let kbit = |now: u64, then: u64| float(now.saturating_sub(then)) * 8.0 / 1000.0 / seconds;
        (kbit(tx, tx0), kbit(rx, rx0))
    }
}

/// Names the network an uplink is on: its router's hardware address, or
/// the link's own name until the router has answered.
fn network_of(uplink: &links::Uplink) -> String {
    uplink
        .gateway
        .as_ref()
        .and_then(|gw| {
            sh(&words(&format!(
                "ip -j neigh show dev {} {gw}",
                uplink.name
            )))
            .ok()
        })
        .and_then(|json| links::gateway_address(&json))
        .unwrap_or_else(|| format!("link {}", uplink.name))
}

/// The most a link can carry each way, in kbit/s: Ethernet says, Wi-Fi
/// doesn't.
fn ceiling(link: &str) -> links::Ceiling {
    links::ceiling(read_number(
        &Path::new("/sys/class/net").join(link).join("speed"),
    ))
}

fn start(
    uplink: links::Uplink,
    memory: &Memory,
    hosts: &[String],
    echoes: &std::sync::mpsc::Sender<Echo>,
) -> Option<Shaped> {
    let index = u32::try_from(read_number(
        &Path::new("/sys/class/net")
            .join(&uplink.name)
            .join("ifindex"),
    )?)
    .ok()?;
    let ifb = qdisc::ifb(index);
    let network = network_of(&uplink);
    let held = memory.get(&network).unwrap_or(FIRST);
    let max = ceiling(&uplink.name);
    // The kernel gives a Wi-Fi link a wireless folder of its own.
    let wireless = Path::new("/sys/class/net")
        .join(&uplink.name)
        .join("wireless")
        .is_dir();
    let controller = Controller::new(
        Settings::for_link(wireless),
        Direction::new(
            Bounds {
                min: FLOOR.up,
                max: max.up,
            },
            held.up,
        ),
        Direction::new(
            Bounds {
                min: FLOOR.down,
                max: max.down,
            },
            held.down,
        ),
    );
    let (up, down) = (controller.up.rate, controller.down.rate);
    install(&uplink.name, &ifb, up, down);
    eprintln!(
        "shaping {} on {network} at {up} kbit/s up, {down} down",
        uplink.name
    );
    let probes = Probes::start(&uplink.name, hosts, 0.3, echoes);
    let mut shaped = Shaped {
        uplink,
        ifb,
        network,
        controller,
        probes,
        applied: (up, down),
        counted: (0, 0, Instant::now()),
        state: State::Resting,
        follows: None,
        told: false,
        real_time: qdisc::RealTime::default(),
    };
    let (tx, rx) = shaped.bytes();
    shaped.counted = (tx, rx, Instant::now());
    Some(shaped)
}

fn install(link: &str, ifb: &str, up: u64, down: u64) {
    run_all(qdisc::install(link, ifb, up, down));
}

/// Runs each command, saying what failed, and gives whether all went.
fn run_all(commands: Vec<Vec<String>>) -> bool {
    let mut fine = true;
    for command in commands {
        match sh(&command) {
            // The IFB is left from before when the shaper restarts.
            Err(e) if command[..3] == ["ip", "link", "add"] && e.contains("exists") => {}
            Err(e) => {
                eprintln!("{e}");
                fine = false;
            }
            Ok(_) => {}
        }
    }
    fine
}

fn uplinks() -> Vec<links::Uplink> {
    let v4 = sh(&words("ip -4 -j route show default")).unwrap_or_default();
    let v6 = sh(&words("ip -6 -j route show default")).unwrap_or_default();
    links::uplinks(&v4, &v6)
}

/// The links the shaper holds and what each reaches without the router.
fn local(shaped: &BTreeMap<String, Shaped>, vpns: &BTreeMap<String, Vpn>) -> classes::Local {
    let mut local = classes::Local::default();
    local.links.extend(vpns.keys().cloned());
    for name in shaped.keys() {
        local.links.push(name.clone());
        let json = sh(&words(&format!("ip -j addr show dev {name}"))).unwrap_or_default();
        let (v4, v6) = links::local_networks(&json);
        local.v4.extend(v4);
        local.v6.extend(v6);
    }
    local.v4.sort();
    local.v4.dedup();
    local.v6.sort();
    local.v6.dedup();
    local
}

fn refill(text: &str) {
    let child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return eprintln!("couldn't run nft");
    };
    let _ = child
        .stdin
        .take()
        .expect("piped")
        .write_all(text.as_bytes());
    if let Ok(out) = child.wait_with_output()
        && !out.status.success()
    {
        eprintln!("nft: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
}

fn run(options: &Options) -> ExitCode {
    let _ = fs::create_dir_all(&options.state);
    let _ = fs::create_dir_all(&options.runtime);
    let mut memory = Memory::open(&options.state);
    let (send, echoes): (_, Receiver<Echo>) = channel();
    let mut shaped: BTreeMap<String, Shaped> = BTreeMap::new();
    let began = Instant::now();
    let mut last_look: Option<Instant> = None;
    let mut last_fill: Option<Instant> = None;
    let mut filled = String::new();
    let mut reach = classes::Local::default();
    let mut vpns: BTreeMap<String, Vpn> = BTreeMap::new();
    let mut last_kept = Instant::now();
    // Listing every process costs more than a step can spend, so the pi
    // sessions are found every few seconds, and whether each runs a typed
    // turn is read at every step.
    let mut sessions = Vec::new();
    let mut last_sessions: Option<Instant> = None;
    loop {
        let now = Instant::now();
        if last_sessions.is_none_or(|at| now - at >= Duration::from_secs(5)) {
            last_sessions = Some(now);
            sessions = classes::sessions(&options.proc, &options.users);
        }
        if last_look.is_none_or(|at| now - at >= Duration::from_secs(1)) {
            last_look = Some(now);
            let order = look(&mut shaped, &mut memory, options, &send);
            let networks: Vec<(&str, &str)> = order
                .iter()
                .filter_map(|n| shaped.get(n).map(|s| (n.as_str(), s.network.as_str())))
                .collect();
            let follows = links::leaders(&networks);
            for (name, s) in &mut shaped {
                let leader = follows.get(name).cloned();
                if leader != s.follows {
                    match &leader {
                        Some(l) => {
                            eprintln!("{name} reaches the same router as {l}; taking its rate");
                        }
                        None if s.follows.is_some() => eprintln!("{name} finds its own rate again"),
                        None => {}
                    }
                    s.follows = leader;
                }
            }
            let base = order.first().and_then(|p| shaped.get(p)).map(|s| s.applied);
            follow_vpns(&mut vpns, &options.vpns, base);
            reach = local(&shaped, &vpns);
            watch_real_time(&mut shaped, (now - began).as_secs_f64());
        }
        // A slice or unit made a moment ago reaches its set at the next
        // step. nixos-rebuild loads the table afresh on a switch, empty, so
        // the sets are also written every few seconds, changed or not.
        let mut sets = classes::read(&options.cgroups);
        sets.bulk5 = classes::untyped(&options.cgroups, &sessions);
        let text = classes::transaction(TABLE, &sets, &reach);
        if text != filled || last_fill.is_none_or(|at| now - at >= Duration::from_secs(5)) {
            last_fill = Some(now);
            refill(&text);
            filled = text;
        }
        while let Ok(echo) = echoes.try_recv() {
            if let Some(s) = shaped.get_mut(&echo.link) {
                s.controller.sample(&echo.host, echo.rtt_ms);
            }
        }
        let t = (now - began).as_secs_f64();
        // Every link is counted, followers too: their traffic crosses the
        // leader's connection, and a leader blind to it reads the queue a
        // download over Wi-Fi builds against an idle download, and cuts
        // its upload.
        let carried: BTreeMap<String, (f64, f64)> = shaped
            .iter_mut()
            .map(|(n, s)| (n.clone(), s.carried()))
            .collect();
        let follows: BTreeMap<String, String> = shaped
            .iter()
            .filter_map(|(n, s)| s.follows.clone().map(|l| (n.clone(), l)))
            .collect();
        for (name, (sent, received)) in links::connection_loads(&carried, &follows) {
            if let Some(s) = shaped.get_mut(&name) {
                step(s, t, sent, received);
            }
        }
        // A follower holds its leader's rates, so the two together stay
        // under the one connection; its own controller only keeps its pings.
        let rates: BTreeMap<String, (u64, u64)> = shaped
            .iter()
            .map(|(n, s)| (n.clone(), (s.controller.up.rate, s.controller.down.rate)))
            .collect();
        for s in shaped.values_mut() {
            if let Some((up, down)) = s.follows.as_ref().and_then(|l| rates.get(l)) {
                s.controller.up.rate = *up;
                s.controller.down.rate = *down;
                apply(s);
            }
        }
        if now - last_kept >= Duration::from_secs(60) {
            last_kept = now;
            for s in shaped.values_mut() {
                keep(&mut memory, s);
            }
        }
        write_status(&options.runtime, &shaped);
        thread::sleep(STEP);
    }
}

/// Remembers what a link learned of its network's rates, each direction
/// only if its own load taught it, keeping what memory holds for the other.
fn keep(memory: &mut Memory, s: &mut Shaped) {
    let c = &mut s.controller;
    let (settings, up, down) = (&c.settings, &mut c.up, &mut c.down);
    if !up.learned && !down.learned {
        return;
    }
    let known = memory.get(&s.network).unwrap_or(Held {
        up: up.remembered(settings),
        down: down.remembered(settings),
    });
    let held = Held {
        up: if up.learned {
            up.remembered(settings)
        } else {
            known.up
        },
        down: if down.learned {
            down.remembered(settings)
        } else {
            known.down
        },
    };
    memory.put(&s.network, held);
    up.learned = false;
    down.learned = false;
}

/// What a VPN may send and bring in, against the connection it rides: a
/// little under, so its own queue forms inside the VPN, where its packets
/// can still be told apart, before the connection's CAKE sees them as one
/// flow. Its CAKE counts the VPN's own headers on each packet, so the
/// share left over is for what goes round the VPN.
const VPN_SHARE: f64 = 0.95;

/// What the shaper holds a VPN to each way, in kbit/s, and the IFB its
/// downloads are turned round through.
struct Vpn {
    up: u64,
    down: u64,
    ifb: String,
}

/// Whether a rate moved by more than a hundredth from what was applied.
fn moved(applied: Option<u64>, kbit: u64) -> bool {
    applied.is_none_or(|a| a.abs_diff(kbit) * 100 > a)
}

/// Keeps a CAKE on each VPN that's there, each way, at its share of the
/// primary uplink's rates, and lets go of one that's gone or has no uplink
/// to ride.
fn follow_vpns(applied: &mut BTreeMap<String, Vpn>, names: &[String], base: Option<(u64, u64)>) {
    for name in names {
        let dir = Path::new("/sys/class/net").join(name);
        let index = read_number(&dir.join("ifindex")).and_then(|i| u32::try_from(i).ok());
        let (Some((up, down)), Some(index)) = (base, index) else {
            if let Some(gone) = applied.remove(name) {
                if dir.exists() {
                    let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
                    let _ = sh(&words(&format!("tc qdisc del dev {name} ingress")));
                }
                let _ = sh(&words(&format!("ip link del {}", gone.ifb)));
            }
            continue;
        };
        let ifb = qdisc::ifb(index);
        let up = whole(float(up) * VPN_SHARE).max(FLOOR.up);
        let down = whole(float(down) * VPN_SHARE).max(FLOOR.down);
        let was = applied.get(name);
        // A VPN brought back up is a new device, with a new index.
        if let Some(old) = was.filter(|v| v.ifb != ifb) {
            let _ = sh(&words(&format!("ip link del {}", old.ifb)));
        }
        let same_ifb = was.is_some_and(|v| v.ifb == ifb);
        let (was_up, was_down) = (was.map(|v| v.up), was.map(|v| v.down).filter(|_| same_ifb));
        let json = sh(&words(&format!("tc -j qdisc show dev {name}"))).unwrap_or_default();
        let back = sh(&words(&format!("tc -j qdisc show dev {ifb}"))).unwrap_or_default();
        let mut kept = Vpn {
            up: was_up.unwrap_or(0),
            down: was_down.unwrap_or(0),
            ifb: ifb.clone(),
        };
        if !qdisc::vpn_ours(&json) || moved(was_up, up) {
            match sh(&qdisc::vpn(name, up)) {
                Ok(_) => kept.up = up,
                Err(e) => eprintln!("{e}"),
            }
        }
        if !qdisc::turned(&json) || !qdisc::vpn_ours(&back) {
            if run_all(qdisc::vpn_ingress(name, &ifb, down)) {
                kept.down = down;
            }
        } else if moved(was_down, down) {
            match sh(&qdisc::vpn_ingress_change(&ifb, down)) {
                Ok(_) => kept.down = down,
                Err(e) => eprintln!("{e}"),
            }
        }
        if was.is_none() {
            eprintln!("shaping VPN {name} at {up} kbit/s up and {down} down");
        }
        applied.insert(name.clone(), kept);
    }
}

/// Follows the uplinks: shapes a new one, lets a gone one go, and puts back
/// queues somebody took away or a link that came back without. Returns the
/// uplinks in the routes' order, the first being the one VPNs ride.
fn look(
    shaped: &mut BTreeMap<String, Shaped>,
    memory: &mut Memory,
    options: &Options,
    send: &std::sync::mpsc::Sender<Echo>,
) -> Vec<String> {
    let now: Vec<links::Uplink> = uplinks();
    let order: Vec<String> = now.iter().map(|u| u.name.clone()).collect();
    let gone: Vec<String> = shaped
        .keys()
        .filter(|k| !now.iter().any(|u| &u.name == *k))
        .cloned()
        .collect();
    for name in gone {
        if let Some(mut s) = shaped.remove(&name) {
            keep(memory, &mut s);
            for command in qdisc::remove(&name, &s.ifb) {
                let _ = sh(&command);
            }
            eprintln!("{name} no longer routes; left unshaped");
        }
    }
    for uplink in now {
        match shaped.get_mut(&uplink.name) {
            None => {
                let name = uplink.name.clone();
                if let Some(s) = start(uplink, memory, &options.hosts, send) {
                    shaped.insert(name, s);
                }
            }
            Some(s) => {
                let json = sh(&words(&format!("tc -j qdisc show dev {}", s.uplink.name)))
                    .unwrap_or_default();
                let ifb =
                    sh(&words(&format!("tc -j qdisc show dev {}", s.ifb))).unwrap_or_default();
                if !qdisc::ours(&json) || !qdisc::turned(&json) || !qdisc::ours(&ifb) {
                    eprintln!("{}'s queues were gone; putting them back", s.uplink.name);
                    install(
                        &s.uplink.name,
                        &s.ifb,
                        s.controller.up.rate,
                        s.controller.down.rate,
                    );
                    s.applied = (s.controller.up.rate, s.controller.down.rate);
                }
                if !s.probes.running() {
                    s.probes = Probes::start(&s.uplink.name, &options.hosts, 0.3, send);
                }
                // A cable moved to another port can link at another speed
                // without the link going away: a desk's went from 100 Mbit/s
                // to gigabit, and its rates stayed held under the old wire.
                let max = ceiling(&s.uplink.name);
                let up = s.controller.up.bound(max.up);
                if s.controller.down.bound(max.down) || up {
                    eprintln!(
                        "{}: the link's speed changed, so its rates are held under {} up and {} \
                         down",
                        s.uplink.name,
                        mbit(max.up),
                        mbit(max.down)
                    );
                }
                // The router answers ARP a moment after the link comes up,
                // so a link first named by itself learns its network later.
                if s.network.starts_with("link ") || s.uplink.gateway != uplink.gateway {
                    s.uplink.gateway = uplink.gateway;
                    let network = network_of(&s.uplink);
                    if network != s.network {
                        if let Some(held) = memory.get(&network) {
                            s.controller.up = Direction::new(s.controller.up.bounds, held.up);
                            s.controller.down = Direction::new(s.controller.down.bounds, held.down);
                        }
                        s.network = network;
                    }
                }
            }
        }
    }
    order
}

/// Tells each link's controller whether real-time traffic crosses it, from
/// the packets its video tins sent each way, and the journal when that
/// changes.
fn watch_real_time(shaped: &mut BTreeMap<String, Shaped>, t: f64) {
    for (name, s) in shaped {
        let packets = [&s.uplink.name, &s.ifb]
            .iter()
            .map(|dev| {
                sh(&words(&format!("tc -s -j qdisc show dev {dev}")))
                    .map_or(0, |json| qdisc::video_packets(&json))
            })
            .sum();
        let live = s.real_time.count(packets, t);
        if live != s.controller.live {
            if live {
                eprintln!(
                    "{name}: real-time traffic flows, so the connection's queue is kept here"
                );
            } else {
                eprintln!(
                    "{name}: no real-time traffic now, so the connection's whole rate is kept"
                );
            }
            s.controller.live = live;
        }
    }
}

/// One step of an uplink's controller on what its connection carried, in
/// kbit/s, applying its rates when they moved.
fn step(s: &mut Shaped, t: f64, sent: f64, received: f64) {
    let was = (s.controller.up.rate, s.controller.down.rate);
    s.state = s.controller.step(t, sent, received);
    tell(s, was, sent, received);
    apply(s);
}

/// Tells the journal of each cut, and of a queue on the connection that
/// builds while the shaper holds its rates, once a spell: a spike in a ping
/// with neither in the journal was never the connection's queue, and a
/// spike with only the second was one this machine doesn't fill, or
/// couldn't cut yet. It
/// tells of an old cut forgotten too, so a rate that jumps back is
/// explained.
fn tell(s: &mut Shaped, was: (u64, u64), sent: f64, received: f64) {
    let name = &s.uplink.name;
    let (up, down) = (s.controller.up.rate, s.controller.down.rate);
    match s.state {
        State::Cut => {
            eprintln!(
                "{name}: the connection's queue reached {:.0} ms; up {} to {}, down {} to {}",
                s.controller.cut_level(),
                mbit(was.0),
                mbit(up),
                mbit(was.1),
                mbit(down)
            );
            s.told = true;
        }
        State::Holding if s.controller.building() && !s.told => {
            eprintln!(
                "{name}: the connection's queue is building and the rates hold: \
                 up carries {:.0}% \
                 of {}, down {:.0}% of {}",
                sent / float(up.max(1)) * 100.0,
                mbit(up),
                received / float(down.max(1)) * 100.0,
                mbit(down)
            );
            s.told = true;
        }
        State::Forgot => {
            eprintln!(
                "{name}: no cut for {:.0} minutes, so the connection's best is held again: \
                 up {}, down {}",
                s.controller.settings.forget_s / 60.0,
                mbit(s.controller.up.held),
                mbit(s.controller.down.held)
            );
        }
        _ if !s.controller.building() => s.told = false,
        _ => {}
    }
}

/// Puts a link's rates on its queues when they moved by more than 1%.
fn apply(s: &mut Shaped) {
    let moved = |a: u64, b: u64| a.abs_diff(b) * 100 > b;
    let (up, down) = (s.controller.up.rate, s.controller.down.rate);
    if moved(up, s.applied.0) {
        match sh(&qdisc::change(&s.uplink.name, up, false)) {
            Ok(_) => s.applied.0 = up,
            Err(e) => eprintln!("{e}"),
        }
    }
    if moved(down, s.applied.1) {
        match sh(&qdisc::change(&s.ifb, down, true)) {
            Ok(_) => s.applied.1 = down,
            Err(e) => eprintln!("{e}"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LinkStatus {
    name: String,
    ifb: String,
    network: String,
    up: u64,
    down: u64,
    held_up: u64,
    held_down: u64,
    state: String,
    building: bool,
    baselines: Vec<(String, f64)>,
    #[serde(default)]
    follows: Option<String>,
    /// Whether real-time traffic crosses the link now.
    #[serde(default)]
    live: bool,
}

fn write_status(runtime: &Path, shaped: &BTreeMap<String, Shaped>) {
    let all: Vec<LinkStatus> = shaped
        .values()
        .map(|s| LinkStatus {
            name: s.uplink.name.clone(),
            ifb: s.ifb.clone(),
            network: s.network.clone(),
            up: s.applied.0,
            down: s.applied.1,
            held_up: s.controller.up.held,
            held_down: s.controller.down.held,
            state: format!("{:?}", s.state).to_lowercase(),
            building: s.controller.building(),
            baselines: s.controller.baselines(),
            follows: s.follows.clone(),
            live: s.controller.live,
        })
        .collect();
    let path = runtime.join("status.json");
    let fresh = runtime.join("status.json.new");
    let text = serde_json::to_string(&all).expect("status serialises");
    if fs::write(&fresh, text)
        .and_then(|()| fs::rename(&fresh, &path))
        .is_err()
    {
        eprintln!("couldn't write {}", path.display());
    }
}

fn mbit(kbit: u64) -> String {
    format!("{:.1} Mbit/s", float(kbit) / 1000.0)
}

const TINS: [&str; 4] = ["bulk", "best effort", "video", "voice"];

/// What the shaper holds each uplink to, and what each tin carried.
fn status(runtime: &Path) -> ExitCode {
    use std::fmt::Write as _;
    let Ok(text) = fs::read_to_string(runtime.join("status.json")) else {
        eprintln!("the shaper isn't running");
        return ExitCode::FAILURE;
    };
    // Written whole at the end, so a reader that stops early, as
    // `grep -q` or `head` does, ends the report rather than panicking it.
    let mut out = String::new();
    let all: Vec<LinkStatus> = serde_json::from_str(&text).unwrap_or_default();
    if all.is_empty() {
        let _ = writeln!(out, "nothing to shape: no link has a default route");
    }
    for link in all {
        let _ = writeln!(out, "{}  on {}", link.name, link.network);
        let _ = writeln!(out, "  up {}, down {}", mbit(link.up), mbit(link.down));
        if let Some(leader) = &link.follows {
            let _ = writeln!(out, "  at {leader}'s rate: both reach the same router");
        }
        // Said only while it holds: a quiet connection needs no line saying so.
        if link.building && link.follows.is_none() {
            let _ = writeln!(
                out,
                "  the connection's queue is building, so the shaper is slowing"
            );
        }
        if link.live {
            let _ = writeln!(
                out,
                "  real-time traffic flows, so the connection's queue is kept here"
            );
        }
        if link.follows.is_none() {
            let _ = writeln!(
                out,
                "  held under load: up {}, down {}",
                mbit(link.held_up),
                mbit(link.held_down)
            );
        }
        let pings: Vec<String> = link
            .baselines
            .iter()
            .map(|(h, ms)| format!("{h} {ms:.0} ms"))
            .collect();
        let _ = writeln!(out, "  quiet round trips: {}", pings.join(", "));
        for (way, dev) in [("sent", &link.name), ("received", &link.ifb)] {
            let json = sh(&words(&format!("tc -s -j qdisc show dev {dev}"))).unwrap_or_default();
            let _ = writeln!(
                out,
                "  {way:<9} {:>12} {:>8} {:>8} {:>16}",
                "packets", "dropped", "marked", "delay (av, peak)"
            );
            for (name, tin) in TINS.iter().zip(qdisc::tins(&json)) {
                let _ = writeln!(
                    out,
                    "    {name:<11} {:>10} {:>8} {:>8} {:>8.1} {:>6.1} ms",
                    tin.packets,
                    tin.drops,
                    tin.marks,
                    float(tin.average_us) / 1000.0,
                    float(tin.peak_us) / 1000.0
                );
            }
        }
    }
    let _ = std::io::stdout().write_all(out.as_bytes());
    ExitCode::SUCCESS
}

/// Takes every queue the shaper made away, for when it stops.
fn stop() {
    let Ok(devices) = fs::read_dir("/sys/class/net") else {
        return;
    };
    for device in devices.flatten() {
        let name = device.file_name().to_string_lossy().to_string();
        if name.starts_with("shaper") {
            let _ = sh(&words(&format!("ip link del {name}")));
            continue;
        }
        let json = sh(&words(&format!("tc -j qdisc show dev {name}"))).unwrap_or_default();
        if qdisc::ours(&json) || qdisc::vpn_ours(&json) {
            let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
            let _ = sh(&words(&format!("tc qdisc del dev {name} ingress")));
        }
    }
}
