//! Keeps the network's queue on this machine, sorted by the slices'
//! classes, so a call, music and what Joel waits on never queue behind the
//! work he started.
//!
//! The queue that fills when this machine uploads isn't its own: it's the
//! modem's or the router's, first come first served, where a call's packets
//! wait behind a build's. So on every link with a default route, the shaper
//! sends and receives a little slower than the line can take, through CAKE,
//! and the queue forms here instead, short and sorted into CAKE's four
//! tins. How fast "a little slower" is, it finds as it goes (src/rate.rs),
//! and remembers for each network (src/memory.rs). Which tin a connection
//! goes to, nftables decides from the cgroup of the socket that opened it
//! (the table in nixos.nix): the shaper keeps that table's sets of cgroups
//! current (src/classes.rs), and its queues on each uplink (src/qdisc.rs).
//!
//! Usage: shaper run [--hosts 1.1.1.1,...] [--tunnels wg0,...] [--state DIR]
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
/// uplinks and downlinks, so its first seconds of load don't queue at the
/// modem, and climbing from there within seconds when the line takes more.
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
    tunnels: Vec<String>,
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
        tunnels: Vec::new(),
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
            "--tunnels" => {
                o.tunnels = value()
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
    /// Whether the journal has been told of the line's queue building now.
    told: bool,
}

impl Shaped {
    fn bytes(&self) -> (u64, u64) {
        let stat = |name: &str| {
            read_number(
                &Path::new("/sys/class/net")
                    .join(&self.uplink.name)
                    .join("statistics")
                    .join(name),
            )
            .unwrap_or(0)
        };
        (stat("tx_bytes"), stat("rx_bytes"))
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

/// The most a link can carry, in kbit/s: Ethernet says, Wi-Fi doesn't.
fn ceiling(link: &str) -> u64 {
    match read_number(&Path::new("/sys/class/net").join(link).join("speed")) {
        Some(mbit) if mbit > 0 && mbit < 1_000_000 => mbit * 1000,
        _ => 2_000_000,
    }
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
    let controller = Controller::new(
        Settings::default(),
        Direction::new(Bounds { min: FLOOR.up, max }, held.up),
        Direction::new(
            Bounds {
                min: FLOOR.down,
                max,
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
    };
    let (tx, rx) = shaped.bytes();
    shaped.counted = (tx, rx, Instant::now());
    Some(shaped)
}

fn install(link: &str, ifb: &str, up: u64, down: u64) {
    for command in qdisc::install(link, ifb, up, down) {
        if let Err(e) = sh(&command) {
            // The IFB is left from before when the shaper restarts.
            if !(command[..3] == ["ip", "link", "add"] && e.contains("exists")) {
                eprintln!("{e}");
            }
        }
    }
}

fn uplinks() -> Vec<links::Uplink> {
    let v4 = sh(&words("ip -4 -j route show default")).unwrap_or_default();
    let v6 = sh(&words("ip -6 -j route show default")).unwrap_or_default();
    links::uplinks(&v4, &v6)
}

/// The links the shaper holds and what each reaches without the router.
fn local(shaped: &BTreeMap<String, Shaped>, tunnels: &BTreeMap<String, u64>) -> classes::Local {
    let mut local = classes::Local::default();
    local.links.extend(tunnels.keys().cloned());
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
    let mut tunnels: BTreeMap<String, u64> = BTreeMap::new();
    let mut last_kept = Instant::now();
    loop {
        let now = Instant::now();
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
            let base = order
                .first()
                .and_then(|p| shaped.get(p))
                .map(|s| s.applied.0);
            follow_tunnels(&mut tunnels, &options.tunnels, base);
            reach = local(&shaped, &tunnels);
        }
        // A slice or unit made a moment ago reaches its set at the next
        // step. nixos-rebuild loads the table afresh on a switch, empty, so
        // the sets are also written every few seconds, changed or not.
        let text = classes::transaction(TABLE, &classes::read(&options.cgroups), &reach);
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
        for s in shaped.values_mut().filter(|s| s.follows.is_none()) {
            step(s, t);
        }
        // A follower holds its leader's rates, so the two together stay
        // under the one line; its own controller only keeps its pings.
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
    let (up, down) = (&mut s.controller.up, &mut s.controller.down);
    if !up.learned && !down.learned {
        return;
    }
    let known = memory.get(&s.network).unwrap_or(Held {
        up: up.held,
        down: down.held,
    });
    let held = Held {
        up: if up.learned { up.held } else { known.up },
        down: if down.learned { down.held } else { known.down },
    };
    memory.put(&s.network, held);
    up.learned = false;
    down.learned = false;
}

/// What a tunnel may send, against the uplink it rides: a little under, so
/// its own queue forms inside the tunnel, where its packets can still be
/// told apart, before the uplink's CAKE sees them as one flow. A tunnel's
/// own header takes about 4% of a packet (MASQUE's or `WireGuard`'s on
/// 1300 bytes).
const TUNNEL_SHARE: f64 = 0.95;

/// Keeps a CAKE on each tunnel that's there, at its share of the primary
/// uplink's rate, and lets go of one that's gone or has no uplink to ride.
fn follow_tunnels(applied: &mut BTreeMap<String, u64>, names: &[String], base: Option<u64>) {
    for name in names {
        let there = Path::new("/sys/class/net").join(name).exists();
        let Some(base) = base.filter(|_| there) else {
            if applied.remove(name).is_some() && there {
                let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
            }
            continue;
        };
        let kbit = whole(float(base) * TUNNEL_SHARE).max(FLOOR.up);
        let json = sh(&words(&format!("tc -j qdisc show dev {name}"))).unwrap_or_default();
        let moved = applied
            .get(name)
            .is_none_or(|a| a.abs_diff(kbit) * 100 > *a);
        if !qdisc::tunnel_ours(&json) || moved {
            match sh(&qdisc::tunnel(name, kbit)) {
                Ok(_) => {
                    if !applied.contains_key(name) {
                        eprintln!("shaping tunnel {name} at {kbit} kbit/s");
                    }
                    applied.insert(name.clone(), kbit);
                }
                Err(e) => eprintln!("{e}"),
            }
        }
    }
}

/// Follows the uplinks: shapes a new one, lets a gone one go, and puts back
/// queues somebody took away or a link that came back without. Returns the
/// uplinks in the routes' order, the first being the one tunnels ride.
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

/// One step of an uplink's controller, applying its rates when they moved.
fn step(s: &mut Shaped, t: f64) {
    let (tx, rx) = s.bytes();
    let (tx0, rx0, at) = s.counted;
    let seconds = at.elapsed().as_secs_f64().max(0.001);
    s.counted = (tx, rx, Instant::now());
    let kbit = |now: u64, then: u64| float(now.saturating_sub(then)) * 8.0 / 1000.0 / seconds;
    let was = (s.controller.up.rate, s.controller.down.rate);
    let (sent, received) = (kbit(tx, tx0), kbit(rx, rx0));
    s.state = s.controller.step(t, sent, received);
    tell(s, was, sent, received);
    apply(s);
}

/// Tells the journal of each cut, and of a queue on the line that builds
/// while the shaper holds its rates, once a spell: a spike in a ping with
/// neither in the journal was never the line's queue, and a spike with only
/// the second was one this machine doesn't fill, or couldn't cut yet. It
/// tells of an old cut forgotten too, so a rate that jumps back is
/// explained.
fn tell(s: &mut Shaped, was: (u64, u64), sent: f64, received: f64) {
    let name = &s.uplink.name;
    let (up, down) = (s.controller.up.rate, s.controller.down.rate);
    match s.state {
        State::Cut => {
            eprintln!(
                "{name}: the line's queue reached {:.0} ms; up {} to {}, down {} to {}",
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
                "{name}: the line's queue is building and the rates hold: up carries {:.0}% \
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
                "{name}: no cut for {:.0} minutes, so the line's best is held again: up {}, down {}",
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
    let Ok(text) = fs::read_to_string(runtime.join("status.json")) else {
        eprintln!("the shaper isn't running");
        return ExitCode::FAILURE;
    };
    let all: Vec<LinkStatus> = serde_json::from_str(&text).unwrap_or_default();
    if all.is_empty() {
        println!("nothing to shape: no link has a default route");
    }
    for link in all {
        println!("{}  on {}", link.name, link.network);
        println!("  up {}, down {}", mbit(link.up), mbit(link.down));
        if let Some(leader) = &link.follows {
            println!("  at {leader}'s rate: both reach the same router");
        }
        // Said only while it holds: a quiet line needs no line saying so.
        if link.building && link.follows.is_none() {
            println!("  the line's queue is building, so the shaper is slowing");
        }
        if link.follows.is_none() {
            println!(
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
        println!("  quiet round trips: {}", pings.join(", "));
        for (way, dev) in [("sent", &link.name), ("received", &link.ifb)] {
            let json = sh(&words(&format!("tc -s -j qdisc show dev {dev}"))).unwrap_or_default();
            println!(
                "  {way:<9} {:>12} {:>8} {:>8} {:>16}",
                "packets", "dropped", "marked", "delay (av, peak)"
            );
            for (name, tin) in TINS.iter().zip(qdisc::tins(&json)) {
                println!(
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
        if qdisc::ours(&json) {
            let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
            let _ = sh(&words(&format!("tc qdisc del dev {name} ingress")));
        } else if qdisc::tunnel_ours(&json) {
            let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
        }
    }
}
