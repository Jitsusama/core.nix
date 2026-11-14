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
//! and remembers for each network (src/memory.rs); with `--controller
//! one-way`, its rebuild finds it from each direction's own delay, as the
//! recorder measures it (src/live.rs). Which tin a connection
//! goes to, nftables decides from the cgroup of the socket that opened it
//! (the table in nixos.nix): the shaper keeps that table's sets of cgroups
//! current (src/classes.rs), and its queues on each uplink (src/qdisc.rs).
//!
//! Usage: shaper run [--hosts 1.1.1.1,...] [--vpns wg0,...] [--state DIR]
//!                   [--runtime DIR] [--controller round-trip|one-way]
//!                   [--record DIR] (`SHAPER_HOSTS` stands for --hosts)
//!        shaper status
//!        shaper stop
//!        shaper record [--state DIR]
//!        shaper replay [--base UP,DOWN] [--controller one-way|none] [--json] FILES...
//!        shaper day [--record DIR] [--out DIR] [YYYY-MM-DD]

mod call;
mod classes;
mod cost;
mod cpu;
mod day;
mod icmp;
mod links;
mod live;
mod memory;
mod oneway;
mod probe;
mod proof;
mod qdisc;
mod rate;
mod record;
mod replay;
mod room;
#[cfg(test)]
mod sim;
mod stamp;
mod vpndelay;
mod waiting;

use memory::{Held, Memory};
use probe::{Echo, Probes};
use rate::{Bounds, Controller, Direction, Settings, State, float};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

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
    /// Whether the one-way controller sets the rates (src/live.rs), from
    /// what the recorder writes in `record`, rather than the round trip's.
    one_way: bool,
    record: PathBuf,
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
        one_way: false,
        record: "/var/lib/shaper-record".into(),
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
            "--record" => o.record = value().into(),
            "--controller" => match value().as_str() {
                "one-way" => o.one_way = true,
                "round-trip" => o.one_way = false,
                other => eprintln!("ignoring --controller {other}"),
            },
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
        Some("record") => recording(&options(rest).state),
        Some("replay") => replay::command(rest),
        Some("day") => day::command(rest),
        Some("proof") => proof::command(rest),
        Some("proof-call") => call::command(rest),
        _ => {
            eprintln!("usage: shaper run | status | stop | record | replay | day | proof");
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
    /// What the upload's CAKE held when last read, and of the reads since
    /// the controller was last told, how many found a queue in it.
    holding: Option<(u64, u64)>,
    queued_reads: (u32, u32),
    /// What the upload's CAKE held, in bytes, over those reads together.
    held_bytes: u64,
    /// Of the reads of the VPNs riding the link since, how many found a
    /// queue in one: a VPN held under the link holds the queue in its own
    /// CAKE, and one the link's CAKE doesn't see is no router's.
    vpn_reads: (u32, u32),
    /// The packets the upload's CAKE had dropped when last read, and what
    /// it and the VPNs riding the link dropped since the controller was
    /// last told, and when that was.
    drops: Option<u64>,
    dropped: u64,
    told_at: Option<Instant>,
    /// Whether what the link receives goes straight in, past its IFB,
    /// because nothing limits the download.
    straight: bool,
    /// The bytes the upload's CAKE had sent when last read, and when the
    /// controller was last told: what crossed the line, apart from the
    /// home network's traffic, which the link sends as well.
    sent_bytes: u64,
    sent_told: Option<u64>,
}

impl Shaped {
    /// What the connection has carried each way, in bytes, as each
    /// direction's CAKE counts it. The link's own counts take in copies to
    /// this network's machines as well, which never cross the connection:
    /// on 29 October they took a home upload's rate from 8 Mbit/s to 458
    /// within three hours, on a connection carrying about 11, and a cut
    /// could go no further under that than Wi-Fi's floor, so the queue
    /// formed in the router behind every session's requests.
    ///
    /// The upload's CAKE is read for the queue it holds at the same time,
    /// five times a second, so a moment's burst through it isn't taken
    /// for a queue.
    fn bytes(&mut self) -> (u64, u64) {
        let read = |dev: &str| sh(&words(&format!("tc -s -j qdisc show dev {dev}"))).ok();
        let up = read(&self.uplink.name);
        if let Some(now) = up.as_deref().and_then(qdisc::holding) {
            let (queued, reads) = &mut self.queued_reads;
            *queued += u32::from(qdisc::queued(self.holding, now));
            *reads += 1;
            self.held_bytes += now.0;
            self.holding = Some(now);
        }
        if let Some(now) = up.as_deref().and_then(qdisc::drops) {
            self.dropped += since(self.drops.replace(now), now);
        }
        let sent = |json: Option<String>| {
            json.and_then(|json| qdisc::shaped_bytes(&json))
                .unwrap_or(0)
        };
        self.sent_bytes = sent(up);
        (self.sent_bytes, sent(read(&self.ifb)))
    }

    /// Whose the upload's queue has been since last asked: whether CAKE
    /// held one in most reads, how long it kept what it held, whether data
    /// waits to go, and what TCP sent again and our queues dropped, in
    /// kbit/s.
    fn ours(&mut self, waiting: bool, resent: u64, now: Instant) -> Option<oneway::Ours> {
        let link = std::mem::take(&mut self.queued_reads);
        let held = std::mem::take(&mut self.held_bytes);
        let wait = qdisc::wait_ms(held, link.1, self.applied.0);
        let vpn = std::mem::take(&mut self.vpn_reads);
        let dropped = std::mem::take(&mut self.dropped);
        let seconds = self.told_at.replace(now).map(|at| (now - at).as_secs_f64());
        let dropped = seconds.map_or(0, |s| segments_kbit(dropped, s));
        let sent = seconds
            .zip(self.sent_told.replace(self.sent_bytes))
            .map(|(s, then)| bytes_kbit(self.sent_bytes.saturating_sub(then), s));
        oneway::Ours::from_reads(link, vpn, wait, waiting, (resent, dropped))
            .map(|ours| oneway::Ours { sent, ..ours })
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

/// Record the signal of the first uplink with an IPv4 gateway, and of the
/// next when it changes, until stopped.
fn recording(folder: &Path) -> ExitCode {
    loop {
        let first = uplinks().into_iter().find(|u| {
            u.gateway
                .as_ref()
                .is_some_and(|g| g.parse::<std::net::Ipv4Addr>().is_ok())
        });
        let Some(uplink) = first else {
            thread::sleep(Duration::from_secs(5));
            continue;
        };
        let network = asked_network_of(&uplink);
        let gateway = uplink.gateway.as_ref().and_then(|g| g.parse().ok());
        // The same uplink on the same network: a router that hadn't yet
        // answered names its network once it does, and the pool and the
        // record start over under that name. A router whose entry has gone
        // stale for a moment is still the same router.
        let still = || {
            uplinks().iter().find(|u| {
                u.gateway
                    .as_ref()
                    .is_some_and(|g| g.parse::<std::net::Ipv4Addr>().is_ok())
            }) == Some(&uplink)
                && router_of(&uplink).is_none_or(|router| router == network)
        };
        record::record(&uplink.name, &network, gateway, folder, &still);
        thread::sleep(Duration::from_secs(2));
    }
}

/// Names the network an uplink is on: its router's hardware address, or
/// the link's own name until the router has answered.
fn network_of(uplink: &links::Uplink) -> String {
    router_of(uplink).unwrap_or_else(|| format!("link {}", uplink.name))
}

/// How many times, and how often, the recorder asks a router that hasn't
/// answered yet: a router answers ARP within a second of being asked.
const ASKS: u32 = 10;
const ASKING: Duration = Duration::from_millis(300);

/// Names the network as `network_of` does, first asking a router that
/// hasn't answered. A link that has just come up sends nothing to its
/// router until something needs it, so the recorder, which starts with
/// the link, named its network by the link and started over under the
/// router's name a moment later (19:18:42 on 9 November).
fn asked_network_of(uplink: &links::Uplink) -> String {
    let gateway = uplink
        .gateway
        .as_ref()
        .and_then(|g| g.parse::<std::net::Ipv4Addr>().ok());
    for _ in 0..ASKS {
        if let Some(router) = router_of(uplink) {
            return router;
        }
        // Any datagram to the router makes the kernel ask its address;
        // the discard port takes it, or the router drops it.
        if let Some(gateway) = gateway
            && let Ok(socket) = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
        {
            let _ = socket.send_to(&[0], (gateway, 9));
        }
        thread::sleep(ASKING);
    }
    network_of(uplink)
}

/// The hardware address of an uplink's router, once it has answered.
fn router_of(uplink: &links::Uplink) -> Option<String> {
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
    install(&uplink.name, &ifb, up, down, false);
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
        holding: None,
        queued_reads: (0, 0),
        held_bytes: 0,
        vpn_reads: (0, 0),
        drops: None,
        dropped: 0,
        told_at: None,
        straight: false,
        sent_bytes: 0,
        sent_told: None,
    };
    let (tx, rx) = shaped.bytes();
    shaped.counted = (tx, rx, Instant::now());
    Some(shaped)
}

/// Puts a link's queues on, turning what it receives through its IFB
/// unless it goes `straight` in.
fn install(link: &str, ifb: &str, up: u64, down: u64, straight: bool) {
    run_all(qdisc::install(link, ifb, up, down));
    if !straight {
        run_all(qdisc::turn(link, ifb));
    }
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
    local
        .marks
        .extend(vpns.values().filter_map(|v| v.mark.clone()));
    local.marks.sort();
    local.marks.dedup();
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

/// How often the shaper asks whether a call is on: a-call-is-on reads each
/// account's PipeWire in about 40 ms, so asking costs about 2% of a core,
/// and a call goes as best effort for at most this long after it starts.
const CALL_EVERY: Duration = Duration::from_secs(2);

/// Whether a call is on in any account, as a-call-is-on last said, and when
/// it was last asked.
#[derive(Default)]
struct Call {
    on: bool,
    asked: Option<Instant>,
}

impl Call {
    /// Asks a-call-is-on again once `CALL_EVERY` has passed, says so in the
    /// journal when the answer changes, and gives the `calling` set's
    /// members. When it can't tell, what was last known stands.
    fn heed(&mut self, now: Instant, users: &Path) -> Vec<String> {
        if self.asked.is_none_or(|at| now - at >= CALL_EVERY) {
            self.asked = Some(now);
            let asked = Command::new("a-call-is-on")
                .arg(users)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let on = match asked.map(|s| s.code()) {
                Ok(Some(0)) => Some(true),
                Ok(Some(1)) => Some(false),
                _ => None,
            };
            if let Some(on) = on
                && on != self.on
            {
                if on {
                    eprintln!("a call is on, so what apps send by UDP goes as video");
                } else {
                    eprintln!("no call is on, so apps' UDP to 443 goes as a page's again");
                }
                self.on = on;
            }
        }
        classes::calling(self.on)
    }
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
    let mut vpns = Vpns::default();
    let mut last_kept = Instant::now();
    // Listing every process costs more than a step can spend, so the pi
    // sessions are found every few seconds, and whether each runs a typed
    // turn is read at every step.
    let mut sessions = Vec::new();
    let mut last_sessions: Option<Instant> = None;
    let mut last_resent: Option<(u64, Instant)> = None;
    let mut heard: BTreeMap<String, Heard> = BTreeMap::new();
    let mut primary: Option<String> = None;
    let mut call = Call::default();
    let mut live = options.one_way.then(|| {
        let learned = options.state.join("learned.json");
        live::Live::new(&options.record, SystemTime::now(), &learned)
    });
    loop {
        let now = Instant::now();
        let t = (now - began).as_secs_f64();
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
            primary = order.first().cloned();
            let base = order
                .first()
                .and_then(|p| shaped.get(p))
                .map(|s| (s.applied.0, s.applied.1, s.straight));
            follow_vpns(&mut vpns, &options.vpns, (base, order.first()), t);
            reach = local(&shaped, &vpns.0);
            watch_real_time(&mut shaped, (now - began).as_secs_f64());
            if let Some(l) = live.as_mut() {
                tell_live(l, options, &mut shaped, &vpns.0, (&mut last_resent, now));
            }
        }
        // A slice or unit made a moment ago reaches its set at the next
        // step. nixos-rebuild loads the table afresh on a switch, empty, so
        // the sets are also written every few seconds, changed or not.
        let mut sets = classes::read(&options.cgroups);
        sets.bulk5 = classes::untyped(&options.cgroups, &sessions);
        sets.calling = call.heed(now, &options.users);
        let rides = primary.as_ref().and_then(|p| shaped.get_mut(p));
        read_vpns(&vpns.0, &mut heard, rides);
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
        step_links(&mut shaped, live.as_mut(), t);
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
        let heard = live.as_ref().and_then(live::Live::heard);
        write_status(&options.runtime, &shaped, heard.as_ref());
        thread::sleep(STEP);
    }
}

/// Tells the one-way controller, each second, whose each upload's queue
/// is, and has it hear a call's round trip through the VPN riding the link:
/// behind a router that queues flow by flow, its probes see neither.
fn tell_live(
    l: &mut live::Live,
    options: &Options,
    shaped: &mut BTreeMap<String, Shaped>,
    vpns: &BTreeMap<String, Vpn>,
    (last_resent, now): (&mut Option<(u64, Instant)>, Instant),
) {
    let waiting = waiting::read(&options.proc);
    let resent = resent_kbit(last_resent, waiting::resent(&options.proc), now);
    for (name, s) in shaped {
        if let Some(ours) = s.ours(waiting, resent, now) {
            l.see(name, ours);
        }
    }
    l.hear_vpn(vpns.keys().next().map(String::as_str));
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

/// What the shaper holds a VPN to each way, in kbit/s, the IFB its
/// downloads are turned round through, and the firewall mark its own
/// packets carry, when it routes by one.
struct Vpn {
    up: u64,
    down: u64,
    ifb: String,
    mark: Option<String>,
}

/// What the shaper has heard of a VPN's CAKE: what it held and dropped
/// when last read.
#[derive(Default)]
struct Heard {
    held: Option<(u64, u64)>,
    drops: Option<u64>,
}

/// What a count read `now` rose by since it read `before`: nothing at a
/// count's first read, or once it starts over, as a link's queues made
/// again do.
fn since(before: Option<u64>, now: u64) -> u64 {
    before.map_or(0, |b| now.saturating_sub(b))
}

/// Counts each VPN's CAKE at every step: whether any held a queue, and
/// what they dropped, are counted to the primary uplink they ride, whose
/// queue it is.
fn read_vpns(
    vpns: &BTreeMap<String, Vpn>,
    heard: &mut BTreeMap<String, Heard>,
    primary: Option<&mut Shaped>,
) {
    heard.retain(|name, _| vpns.contains_key(name));
    let mut read = (0, 0);
    let mut dropped = 0;
    for name in vpns.keys() {
        let json = sh(&words(&format!("tc -s -j qdisc show dev {name}"))).unwrap_or_default();
        let h = heard.entry(name.clone()).or_default();
        if let Some(now) = qdisc::holding(&json) {
            let queued = qdisc::queued(h.held.replace(now), now);
            read = (read.0.max(u32::from(queued)), 1);
        }
        if let Some(now) = qdisc::drops(&json) {
            dropped += since(h.drops.replace(now), now);
        }
    }
    if let Some(s) = primary {
        s.vpn_reads.0 += read.0;
        s.vpn_reads.1 += read.1;
        s.dropped += dropped;
    }
}

/// The mark a VPN's own packets carry, read from the routing rules.
fn vpn_mark(name: &str) -> Option<String> {
    let rules = sh(&words("ip -j rule show")).unwrap_or_default();
    cost::mark_of(&rules, name, |table| {
        sh(&words(&format!("ip -j route show table {table}"))).unwrap_or_default()
    })
}

/// Whether a rate moved by more than a hundredth from what was applied.
fn moved(applied: Option<u64>, kbit: u64) -> bool {
    applied.is_none_or(|a| a.abs_diff(kbit) * 100 > a)
}

/// The VPNs the shaper holds, with what their packets cost the uplink past
/// their count and what they give up for its bulk.
type Vpns = (BTreeMap<String, Vpn>, cost::Cost, room::Room);

/// What the VPNs give up of their upload for the uplink's own bulk, in
/// kbit/s, from their bulk tins, read in `read`, the uplink's, `beside`,
/// and its rate, `line` (room.rs says why), at `t` seconds.
fn give_room(
    room: &mut room::Room,
    read: &[String],
    (beside, line): (Option<&qdisc::Tin>, Option<u64>),
    t: f64,
) -> u64 {
    let (Some(beside), Some(line)) = (beside, line) else {
        return 0;
    };
    let bulk: Vec<qdisc::Tin> = read.iter().filter_map(|json| qdisc::bulk(json)).collect();
    if bulk.is_empty() {
        return 0;
    }
    let inside = qdisc::Tin {
        bytes: bulk.iter().map(|tin| tin.bytes).sum(),
        backlog: bulk.iter().map(|tin| tin.backlog).sum(),
        ..qdisc::Tin::default()
    };
    let was = room.given();
    let given = room.sample(&inside, beside, line, t);
    if was == 0 && given > 0 {
        eprintln!("the uplink's bulk waits behind the VPNs' bulk; the VPNs make room");
    } else if was > 0 && given == 0 {
        eprintln!("the VPNs' bulk has its room back");
    }
    given
}

/// Keeps a CAKE on each VPN that's there, each way, at its share of the
/// primary uplink's rates, less what its own packets cost the uplink past
/// what its CAKE counts and the room its bulk makes for the uplink's, and
/// lets go of one that's gone or has no uplink to ride. What a VPN receives
/// goes straight in, past its IFB, while the uplink's does. Read at `t`
/// seconds.
fn follow_vpns(
    (applied, spent, room): &mut Vpns,
    names: &[String],
    (base, primary): (Option<(u64, u64, bool)>, Option<&String>),
    t: f64,
) {
    // The primary uplink's bulk tin, beside the VPNs riding it.
    let beside = primary.and_then(|p| {
        qdisc::bulk(&sh(&words(&format!("tc -s -j qdisc show dev {p}"))).unwrap_or_default())
    });
    let read: Vec<String> = applied
        .keys()
        .map(|name| sh(&words(&format!("tc -s -j qdisc show dev {name}"))).unwrap_or_default())
        .collect();
    let counted: Option<u64> = read.iter().map(|json| qdisc::counted_bytes(json)).sum();
    let given = give_room(room, &read, (beside.as_ref(), base.map(|b| b.0)), t);
    let carried = sh(&words(&format!(
        "nft -j list counter inet {TABLE} {}",
        cost::COUNTER
    )));
    let extra = match (
        carried.ok().as_deref().and_then(cost::counter_bytes),
        counted,
    ) {
        (Some(carried), Some(counted)) => spent.sample(carried, counted, t),
        _ => 0,
    };
    for name in names {
        let dir = Path::new("/sys/class/net").join(name);
        let index = read_number(&dir.join("ifindex")).and_then(|i| u32::try_from(i).ok());
        let (Some((up, down, straight)), Some(index)) = (base, index) else {
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
        let up = qdisc::vpn_rate(up)
            .saturating_sub(extra + given)
            .max(FLOOR.up);
        let down = qdisc::vpn_rate(down).max(FLOOR.down);
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
            mark: vpn_mark(name),
        };
        if !qdisc::vpn_ours(&json) || moved(was_up, up) {
            match sh(&qdisc::vpn(name, up)) {
                Ok(_) => kept.up = up,
                Err(e) => eprintln!("{e}"),
            }
        }
        if straight {
            if qdisc::turned(&json)
                && let Err(e) = sh(&qdisc::unturn(name))
            {
                eprintln!("{e}");
            }
        } else if !qdisc::turned(&json) || !qdisc::vpn_ours(&back) {
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
                let turned = s.straight || qdisc::turned(&json);
                if !qdisc::ours(&json) || !turned || !qdisc::ours(&ifb) {
                    eprintln!("{}'s queues were gone; putting them back", s.uplink.name);
                    s.holding = None;
                    install(
                        &s.uplink.name,
                        &s.ifb,
                        s.controller.up.rate,
                        s.controller.down.rate,
                        s.straight,
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
    // The round trip's controller reads what came down from the IFB's
    // CAKE, so its downloads always go through it.
    go_straight(s, false);
}

/// One step of every uplink's controller, on what its connection carried:
/// the round trip's, or with `live`, the one-way controller's.
fn step_links(shaped: &mut BTreeMap<String, Shaped>, live: Option<&mut live::Live>, t: f64) {
    // Every link is counted, followers too: their traffic crosses the
    // leader's connection, and a leader blind to it reads the queue a
    // download over Wi-Fi builds against an idle download, and cuts its
    // upload.
    let carried: BTreeMap<String, (f64, f64)> = shaped
        .iter_mut()
        .map(|(n, s)| (n.clone(), s.carried()))
        .collect();
    let follows: BTreeMap<String, String> = shaped
        .iter()
        .filter_map(|(n, s)| s.follows.clone().map(|l| (n.clone(), l)))
        .collect();
    // The one-way controller holds the link the recorder follows; any
    // other goes at its whole rate, its traffic still sorted by CAKE.
    let held = live.map(|l| match l.poll(SystemTime::now()) {
        Some((link, rates)) => (Some(link.to_string()), rates),
        None => (None, replay::Rates::default()),
    });
    for (name, (sent, received)) in links::connection_loads(&carried, &follows) {
        if let Some(s) = shaped.get_mut(&name) {
            match &held {
                None => step(s, t, sent, received),
                Some((link, rates)) if link.as_deref() == Some(name.as_str()) => hold(s, *rates),
                Some(_) => hold(s, replay::Rates::default()),
            }
        }
    }
}

/// Holds a link to the rates the one-way controller answers, at its
/// ceiling where it sets none, and tells the journal when a spell of limits
/// begins and when it ends, rather than of each second's cut.
fn hold(s: &mut Shaped, rates: replay::Rates) {
    let held =
        |rate: Option<u64>, floor: u64, max: u64| rate.map_or(max, |r| r.max(floor).min(max));
    let (up_max, down_max) = (s.controller.up.bounds.max, s.controller.down.bounds.max);
    let (up, down) = (
        held(rates.up, FLOOR.up, up_max),
        held(rates.down, FLOOR.down, down_max),
    );
    s.controller.up.rate = up;
    s.controller.down.rate = down;
    let limited = rates.up.is_some() || rates.down.is_some();
    let name = &s.uplink.name;
    if limited && !s.told {
        eprintln!(
            "{name}: a queue past CAKE was taken back, so the link is held to {} up and {} down",
            mbit(up),
            mbit(down)
        );
        s.told = true;
    } else if !limited && s.told {
        eprintln!("{name}: neither direction queues now, so the link's whole rate is kept");
        s.told = false;
    }
    s.state = if limited {
        State::Holding
    } else {
        State::Resting
    };
    apply(s);
    go_straight(s, rates.down.is_none());
}

/// Lets what a link receives go straight in, past its IFB, or turns it
/// back through once a limit holds, after `apply` gave the IFB's CAKE its
/// rate. `qdisc::unturn` says why.
fn go_straight(s: &mut Shaped, straight: bool) {
    if straight == s.straight {
        return;
    }
    let name = s.uplink.name.clone();
    let done = if straight {
        match sh(&qdisc::unturn(&name)) {
            // Queues put back while it went straight came without a turn.
            Err(e) if !e.contains("Cannot find") => {
                eprintln!("{e}");
                false
            }
            _ => true,
        }
    } else {
        run_all(qdisc::turn(&name, &s.ifb))
    };
    if done {
        s.straight = straight;
        if straight {
            eprintln!("{name}: nothing limits the download, so it goes straight in");
        } else {
            eprintln!("{name}: the download is limited, so it queues in CAKE again");
        }
    }
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
    /// Whether what the link receives goes straight in, past its IFB.
    #[serde(default)]
    straight: bool,
    /// What the one-way controller last heard of the link it holds, and
    /// made of it, so a trace can say why its rate held where it did.
    #[serde(default)]
    heard: Option<live::Heard>,
}

fn write_status(
    runtime: &Path,
    shaped: &BTreeMap<String, Shaped>,
    heard: Option<&(&str, live::Heard)>,
) {
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
            straight: s.straight,
            heard: heard
                .filter(|(link, _)| *link == s.uplink.name)
                .map(|(_, heard)| heard.clone()),
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

/// A segment TCP sends again, in bytes: a bulk upload's are full, an
/// Ethernet frame's worth, and Wi-Fi's are the same size.
const SEGMENT_BYTES: u64 = 1_500;

/// What TCP sent again since `last`, in kbit/s, from the kernel's count of
/// segments. The count is the machine's, not a link's: the upload that
/// fills one link is what resends, and the rest is a trickle.
fn resent_kbit(last: &mut Option<(u64, Instant)>, now_count: Option<u64>, now: Instant) -> u64 {
    let Some(count) = now_count else {
        return 0;
    };
    let kbit = last.map_or(0, |(then, at)| {
        segments_kbit(count.saturating_sub(then), (now - at).as_secs_f64())
    });
    *last = Some((count, now));
    kbit
}

/// What `segments` full-sized packets over `seconds` come to, in kbit/s.
fn segments_kbit(segments: u64, seconds: f64) -> u64 {
    bytes_kbit(segments * SEGMENT_BYTES, seconds)
}

/// What `bytes` over `seconds` come to, in kbit/s.
fn bytes_kbit(bytes: u64, seconds: f64) -> u64 {
    let bits = bytes * 8;
    // A few hundred thousand kbit/s at most.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive rate far under u64's range"
    )]
    let kbit = (float(bits) / 1_000.0 / seconds.max(0.001)).round() as u64;
    kbit
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
    let first = Spent::now();
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
    // A share of time needs an interval: a second is long enough to hold
    // hundreds of the shaper's wakes and short enough to wait for.
    if let Some(left) = Duration::from_secs(1).checked_sub(first.at.elapsed()) {
        std::thread::sleep(left);
    }
    first.say(&Spent::now(), &mut out);
    let _ = std::io::stdout().write_all(out.as_bytes());
    ExitCode::SUCCESS
}

/// The services whose CPU status shows, by their cgroups.
const SPENDERS: [(&str, &str); 2] = [
    (
        "the shaper",
        "/sys/fs/cgroup/system.slice/shaper.service/cpu.stat",
    ),
    (
        "its recorder",
        "/sys/fs/cgroup/system.slice/shaper-record.service/cpu.stat",
    ),
];

/// What the processor had spent at one moment, for `shaper status`.
struct Spent {
    at: Instant,
    cores: Vec<cpu::Ticks>,
    services: Vec<Option<u64>>,
}

impl Spent {
    fn now() -> Self {
        let stat = fs::read_to_string("/proc/stat").unwrap_or_default();
        let services = SPENDERS
            .iter()
            .map(|(_, path)| cpu::used_usec(&fs::read_to_string(path).unwrap_or_default()))
            .collect();
        Self {
            at: Instant::now(),
            cores: cpu::ticks(&stat),
            services,
        }
    }

    /// What was spent between this and `later`, as lines of status.
    fn say(&self, later: &Self, out: &mut String) {
        use std::fmt::Write as _;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a second or two, far under u64's microseconds"
        )]
        let elapsed = (later.at - self.at).as_micros() as u64;
        let spent: Vec<String> = SPENDERS
            .iter()
            .zip(self.services.iter().zip(&later.services))
            .filter_map(|((name, _), pair)| match pair {
                (Some(b), Some(a)) => Some(format!(
                    "{name} {:.1}%",
                    cpu::of_a_core(*b, *a, elapsed) * 100.0
                )),
                _ => None,
            })
            .collect();
        if !spent.is_empty() {
            let _ = writeln!(out, "cpu: {} of a core", spent.join(", "));
        }
        if let Some((all, busiest)) = cpu::softirq(&self.cores, &later.cores) {
            let _ = writeln!(
                out,
                "softirq, where CAKE runs: {:.1}% of the machine, {:.1}% of its busiest core",
                all * 100.0,
                busiest * 100.0
            );
        }
    }
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
