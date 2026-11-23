//! Keeps the network's queue on this machine, sorted by the slices'
//! classes, so a call, music and what Joel waits on never queue behind the
//! work he started.
//!
//! The queue that fills when this machine uploads isn't its own: it's the
//! modem's or the router's, first come first served, where a call's packets
//! wait behind a build's. So on every link with a default route, the shaper
//! sends and receives a little slower than the connection can take, through CAKE,
//! and the queue forms here instead, short and sorted into CAKE's four
//! tins. How fast "a little slower" is, the one-way controller finds as it
//! goes, from each direction's own delay, as the recorder measures it
//! (src/live.rs); its pings to distant hosts only say each one's quiet
//! round trip (src/trips.rs). Which tin a connection
//! goes to, nftables decides from the cgroup of the socket that opened it
//! (the table in nixos.nix): the shaper keeps that table's sets of cgroups
//! current (src/classes.rs), and its queues on each uplink (src/qdisc.rs).
//!
//! `shaper --help`, and each command's, say what each takes
//! (src/commands.rs); `SHAPER_HOSTS` stands for `--hosts`, and
//! `SHAPER_TURN_DOWNLOAD`, set to `1`, keeps every download turned
//! through its IFB, so a test can count what each class received.

mod bars;
mod call;
mod classes;
mod commands;
mod cost;
mod cpu;
mod day;
mod first;
mod held;
mod hops;
mod house;
mod icmp;
mod lanes;
mod links;
mod live;
mod networks;
mod oneway;
mod pin;
mod probe;
mod proof;
mod qdisc;
mod rate;
mod realtime;
mod record;
mod reflectors;
mod replay;
mod restart;
#[cfg(test)]
mod sim;
mod stamp;
mod status;
mod trace;
mod trips;
mod tunnel;
mod vpndelay;
mod waiting;
mod why;

use probe::{Echo, Probes};
use rate::{Bounds, Direction, float};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use terminal_kit::{Context, Exit, Failure};

const TABLE: &str = "shaper";
const STEP: Duration = Duration::from_millis(200);

/// A rate each way, in kbit/s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    up: u64,
    down: u64,
}

/// Whether the controller holds a link under its most either way now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Resting,
    Holding,
}

/// Where a network with no anchor starts each stay, in kbit/s: under most home
/// connections both ways, so its first seconds of load don't queue at the
/// modem, until the controller's first second leaves it unlimited while
/// no queue stands.
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
    /// Where the recorder writes what the controller sets the rates from
    /// (src/live.rs).
    record: PathBuf,
    /// Where a trial of how the upload finds its line may be written, by
    /// an account that may stop the shaper anyway (src/live.rs).
    trial: PathBuf,
    /// Where each network's declared upload rate is written, its anchor,
    /// which the one-way controller's guard holds the upload near.
    networks: Option<PathBuf>,
    /// Whether what each link receives is turned through its IFB even
    /// while nothing limits the download, so a test can count what each
    /// class received.
    turn_download: bool,
}

/// Whether `SHAPER_TURN_DOWNLOAD`'s value turns every download through its
/// IFB: only `1` does, so `0` or an empty value reads as off.
fn turns_download(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Reads the flags `run`, `status` and `record` take, refusing any other
/// as `shaper {command}` asked wrongly.
fn options(command: &str, args: &[String]) -> Result<Options, Failure> {
    let wrongly =
        |what: String| Failure::new(Exit::Usage, what).try_this(format!("shaper {command} --help"));
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
        record: "/var/lib/shaper-record".into(),
        trial: "/run/shaper-trial/upload.json".into(),
        networks: None,
        turn_download: turns_download(std::env::var("SHAPER_TURN_DOWNLOAD").ok().as_deref()),
    };
    // A network of its own, a test's among them, names its own distant hosts.
    if let Ok(hosts) = std::env::var("SHAPER_HOSTS") {
        o.hosts = hosts.split(',').map(str::to_string).collect();
    }
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| wrongly(format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--hosts" => o.hosts = value()?.split(',').map(str::to_string).collect(),
            "--state" => o.state = value()?.into(),
            "--runtime" => o.runtime = value()?.into(),
            "--cgroups" => o.cgroups = value()?.into(),
            "--proc" => o.proc = value()?.into(),
            "--users" => o.users = value()?.into(),
            "--record" => o.record = value()?.into(),
            "--trial" => o.trial = value()?.into(),
            "--networks" => o.networks = Some(value()?.into()),
            "--vpns" => {
                o.vpns = value()?
                    .split(',')
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            other => return Err(wrongly(format!("{other} isn't a flag it takes"))),
        }
    }
    Ok(o)
}

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), asked)
}

/// Runs the command its first word names; the kit has refused any other.
fn asked(context: &Context) -> Result<(), Failure> {
    let Some((command, rest)) = context.args.split_first() else {
        return Err(Failure::new(Exit::Usage, "shaper takes a command")
            .because("it does only what one asks")
            .try_this("shaper --help"));
    };
    match command.as_str() {
        "run" => run(&options("run", rest)?),
        "status" => status::status(&options("status", rest)?.runtime, context),
        "stop" => {
            // The kit refuses a stray word; a stray flag is refused here.
            let unit = match rest {
                [] => None,
                [flag, unit] if flag == "--unless-restarting" => Some(unit),
                [flag] if flag == "--unless-restarting" => {
                    return Err(wrongly("stop", format!("{flag} takes the unit's name")));
                }
                [flag, ..] => {
                    return Err(wrongly("stop", format!("{flag} isn't a flag it takes")));
                }
            };
            if let Some(unit) = unit.filter(|unit| restarting(unit)) {
                eprintln!("{unit} is restarting, so its queues stay for it to take back");
                return Ok(());
            }
            stop();
            Ok(())
        }
        "record" => recording(&options("record", rest)?.state),
        "replay" => replay::command(rest, context.shared.format.is_json()),
        "day" => day::command(rest),
        "proof" => proof::command(rest),
        "proof-call" => call::command(rest),
        _ => Err(Failure::new(Exit::Usage, "shaper takes its command first")
            .because(format!("{command} came before it"))
            .try_this("shaper --help")),
    }
}

/// A command asked wrongly, pointing at its own help.
fn wrongly(command: &str, what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this(format!("shaper {command} --help"))
}

/// Runs a command, saying what it said when it fails, on one line: the
/// journal splits at each newline, and a line of the command's own apart
/// from what the shaper made of it reads as a failure of its own.
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
            String::from_utf8_lossy(&out.stderr)
                .trim()
                .replace('\n', " ")
        ))
    }
}

/// A device's queues, as `tc -j qdisc show` gives them, or nothing when
/// the read says nothing of them, which is said, since a read failing
/// every time would otherwise leave a queue unrepaired without a word.
fn queues_of(device: &str) -> Option<String> {
    let read = sh(&words(&format!("tc -j qdisc show dev {device}")));
    let seen = seen_queues(read.clone());
    if let (None, Err(e)) = (&seen, &read) {
        eprintln!("{e}: {device}'s queues are left as they are");
    }
    seen
}

/// A device that isn't there has no queues, which reads as none. Any other
/// failure, a `tc` killed partway or one that couldn't start, says nothing
/// of them: taken for none, it put the VPN's turn on again over the one
/// there, and matchall refused it.
fn seen_queues(read: Result<String, String>) -> Option<String> {
    match read {
        Ok(json) => Some(json),
        Err(e) if e.contains("Cannot find device") => Some(String::new()),
        Err(_) => None,
    }
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn read_number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// How what a link receives comes in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Download {
    /// Turned through its IFB, because a limit holds it.
    Turned,
    /// Straight in, past its IFB, because nothing limits it.
    Straight,
    /// Turned through its IFB always, as `Options::turn_download` asks.
    Kept,
}

impl Download {
    fn straight(self) -> bool {
        self == Self::Straight
    }
}

/// One uplink being shaped.
struct Shaped {
    uplink: links::Uplink,
    ifb: String,
    network: String,
    /// What the link is held to each way.
    up: Direction,
    down: Direction,
    /// The pings to distant hosts, and each one's quiet round trip.
    probes: Probes,
    trips: trips::Trips,
    applied: (u64, u64),
    /// What Joel's CAKE was last told, a rate or none (`lanes::joel_cake`),
    /// which waits for the classes' and is told again until it takes, apart
    /// from them (`lanes::joel_rate`). It's untold after an install, which
    /// names no rate: one adopting another shaper's CAKE keeps the rate it
    /// had, with nothing else to change it.
    joel_told: Option<lanes::Cake>,
    state: State,
    /// The link whose rate this one takes, when both reach the same router.
    follows: Option<String>,
    /// Whether the journal has been told of a spell of limits now.
    told: bool,
    /// Whether the spell told is held by a ceiling alone, a trial's, the
    /// network's or its house's, with nothing the controller judged holding it.
    ceiling: bool,
    /// Whether a call's or a screen share's traffic crosses the link, and
    /// whether it did at the last look.
    real_time: realtime::RealTime,
    /// What the upload's CAKE held when last read, and when, and of the
    /// reads since the controller was last told, how many found a queue
    /// in it.
    holding: Option<(held::Holding, Instant)>,
    queued_reads: (u32, u32),
    /// How long what the upload's queues held before the VPN's packets
    /// takes to leave, in microseconds, over those reads together
    /// (`held::ahead_us`).
    ahead_us: u64,
    /// Of the reads of the VPNs riding the link since, how many found a
    /// queue in one: a VPN held under the link holds the queue in its own
    /// CAKE, and one the link's CAKE doesn't see is no router's.
    vpn_reads: (u32, u32),
    /// The share of its lane the VPN riding the link is held to, while it
    /// waits inside its tunnel (src/tunnel.rs).
    vpn_share: f64,
    /// The share of their split agents straight on the link are held to,
    /// while the VPN's flow waits beside what they send.
    agents_share: f64,
    /// The packets the upload's CAKE had dropped when last read, and what
    /// it and the VPNs riding the link dropped since the controller was
    /// last told, and when that was.
    drops: Option<u64>,
    dropped: u64,
    told_at: Option<Instant>,
    /// How what the link receives comes in.
    download: Download,
    /// The bytes the upload's CAKE had sent at its last good read, and at
    /// the last good read before the controller was last told, each with
    /// when it was read: what crossed the line, apart from the home
    /// network's traffic, which the link sends as well.
    sent_read: Option<(u64, Instant)>,
    sent_told: Option<(u64, Instant)>,
    /// How the upload is split between its lanes, what was put on its
    /// classes, when it was last split, and what Joel's CAKE's tins sent.
    lanes: lanes::Lanes,
    split: lanes::Split,
    split_at: Option<f64>,
    straight_tins: qdisc::TinRates,
}

impl Shaped {
    /// Splits the upload between its lanes, at `t` seconds, from what
    /// Joel's and agents' straight on the link carried and hold, and what
    /// Joel sent inside the VPN riding it and its agents hold, `inside`;
    /// and puts the split on the link's classes when one moved. Answers
    /// the VPN's class's rate, in kbit/s.
    fn share_lanes(&mut self, inside: (f64, lanes::Agents), t: f64) -> f64 {
        let json = sh(&words(&format!(
            "tc -s -j qdisc show dev {}",
            self.uplink.name
        )))
        .unwrap_or_default();
        let tins = qdisc::tins(&json);
        self.straight_tins
            .take(tins.iter().map(|tin| tin.bytes).collect(), t);
        let joel = self
            .straight_tins
            .kbit()
            .map_or(0.0, |k| float(k.best + k.video + k.voice));
        let bulk = tins
            .first()
            .map_or_else(lanes::Agents::default, |tin| lanes::Agents {
                backlog: tin.backlog,
                flows: tin.flows,
                dropped: Some(tin.drops + tin.marks),
            });
        let rate = float(self.applied.0);
        let seen = lanes::Seen {
            rate,
            joel_straight: joel,
            joel_vpn: inside.0,
            agents_straight: bulk,
            agents_vpn: inside.1,
            agents_held: lanes::held_to(self.agents_share),
        };
        let dt = self.split_at.replace(t).map_or(1.0, |at| t - at);
        let split = self.lanes.split(&seen, dt);
        if split.moved_from(&self.split, rate) {
            let took = run_all(lanes::change(&self.uplink.name, self.applied.0, &split));
            self.split = self.split.after(split, took);
        }
        self.split.vpn
    }

    /// Lets agents straight borrow the whole rate again once the
    /// controller no longer holds them. Each link's lanes are split once a
    /// second, which reads the share again, but a rate moved in the steps
    /// between splits would put the cap back from the last one.
    fn let_agents_go(&mut self) {
        if self.split.agents_most.is_none() || self.applied.0 == 0 {
            return;
        }
        self.lanes.let_agents_go();
        let split = self.lanes.at(float(self.applied.0));
        let took = run_all(lanes::change(&self.uplink.name, self.applied.0, &split));
        self.split = self.split.after(split, took);
    }

    /// The shares this link's queues are held to now, as a trace says them
    /// (`applied_held`).
    fn held(&self) -> trace::Held {
        applied_held((self.vpn_share, self.agents_share), &self.split)
    }

    /// Reads the upload's CAKE: what it has sent, as CAKE counts it, since
    /// the link's own counts take in copies to this network's machines,
    /// which never cross the connection, and the queue it holds and what it
    /// dropped. It is read five times a second, so a moment's burst through
    /// it isn't taken for a queue.
    fn read_upload(&mut self) {
        let read = |dev: &str| sh(&words(&format!("tc -s -j qdisc show dev {dev}"))).ok();
        let up = read(&self.uplink.name);
        if let Some(now) = up.as_deref().and_then(held::holding) {
            let read = (now, Instant::now());
            let (queued, reads) = &mut self.queued_reads;
            *queued += u32::from(held::queued(self.holding.map(|(was, _)| was), now));
            *reads += 1;
            let sent = |(h, at): (held::Holding, Instant)| (h.agents_sent, at);
            let going = sent_kbit(self.holding.map(sent), Some(sent(read))).unwrap_or(0);
            let assured = lanes::agents_assured(&self.split);
            self.ahead_us += held::ahead_us(now, self.applied.0, assured, going);
            self.holding = Some(read);
        }
        if let Some(now) = up.as_deref().and_then(held::drops) {
            self.dropped += since(self.drops.replace(now), now);
        }
        if let Some(sent) = up.and_then(|json| qdisc::shaped_bytes(&json)) {
            self.sent_read = Some((sent, Instant::now()));
        }
    }

    /// Whose the upload's queue has been since last asked: whether CAKE
    /// held one in most reads, how long it kept what it held, whether data
    /// waits to go, and what TCP sent again and our queues dropped, in
    /// kbit/s.
    fn ours(&mut self, waiting: bool, resent: u64, now: Instant) -> Option<oneway::Ours> {
        let link = std::mem::take(&mut self.queued_reads);
        let wait = held::wait_ms(std::mem::take(&mut self.ahead_us), link.1);
        let vpn = std::mem::take(&mut self.vpn_reads);
        let dropped = std::mem::take(&mut self.dropped);
        let seconds = self.told_at.replace(now).map(|at| (now - at).as_secs_f64());
        let dropped = seconds.map_or(0, |s| segments_kbit(dropped, s));
        let then = std::mem::replace(&mut self.sent_told, self.sent_read);
        let sent = sent_kbit(then, self.sent_read);
        let agents_straight = self.lanes.agents_wait_straight();
        oneway::Ours::from_reads(link, vpn, wait, waiting, (resent, dropped)).map(|ours| {
            oneway::Ours {
                agents_straight,
                sent,
                ..ours
            }
        })
    }
}

/// What the upload's CAKE sent between two good reads, in kbit/s, over
/// the time between them, so one failed read costs nothing while another
/// that second worked. None where it can't be told: no read before, none
/// new since, or counts that went back as CAKE was put in again. The
/// controller then judges by the rate, rather than take a CAKE that sent
/// nothing for one starved.
fn sent_kbit(then: Option<(u64, Instant)>, now: Option<(u64, Instant)>) -> Option<u64> {
    let ((before, read_before), (after, read_after)) = then.zip(now)?;
    (after >= before && read_after > read_before)
        .then(|| bytes_kbit(after - before, (read_after - read_before).as_secs_f64()))
}

/// Record the signal of the first uplink with an IPv4 gateway, and of the
/// next when it changes, until stopped.
fn recording(folder: &Path) -> Result<(), Failure> {
    loop {
        let first = uplinks().unwrap_or_default().into_iter().find(|u| {
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
            uplinks().unwrap_or_default().iter().find(|u| {
                u.gateway
                    .as_ref()
                    .is_some_and(|g| g.parse::<std::net::Ipv4Addr>().is_ok())
            }) == Some(&uplink)
                && router_of(&uplink).is_none_or(|router| router == network)
        };
        // The other links to the same router send over the same
        // connection, so what they carry counts with this one's.
        let beside = || {
            uplinks()
                .unwrap_or_default()
                .iter()
                .filter(|u| u.name != uplink.name)
                .filter(|u| router_of(u).is_some_and(|router| router == network))
                .map(|u| u.name.clone())
                .collect()
        };
        record::record(&uplink.name, &network, gateway, folder, (&still, &beside));
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

/// Starts shaping an uplink: at the upload's rate its network's declared
/// line holds it to, or else at `FIRST`, and at `FIRST`'s download, until
/// the controller says more.
fn start(
    uplink: links::Uplink,
    networks: Option<&Path>,
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
    let mut held = FIRST;
    // A network whose line is declared starts at its hold, as its stay in
    // the one-way controller does (src/oneway.rs), under its ceiling.
    let declared = |path: &Path| {
        networks::hold(
            &networks::anchors(path),
            &networks::ceilings(path),
            &network,
        )
    };
    if let Some(up) = networks.and_then(declared) {
        held.up = up;
    }
    let max = ceiling(&uplink.name);
    let held_up = Direction::new(
        Bounds {
            min: FLOOR.up,
            max: max.up,
        },
        held.up,
    );
    let held_down = Direction::new(
        Bounds {
            min: FLOOR.down,
            max: max.down,
        },
        held.down,
    );
    let (up, down) = (held_up.rate, held_down.rate);
    let mut lanes = lanes::Lanes::default();
    let split = lanes.at(float(up));
    // A read that failed leaves the link as it is, to be taken up once one
    // answers.
    if !install(&uplink.name, &ifb, (up, &split), down, false) {
        return None;
    }
    eprintln!(
        "shaping {} on {network} at {up} kbit/s up, {down} down",
        uplink.name
    );
    let probes = Probes::start(&uplink.name, hosts, 0.3, echoes);
    let mut shaped = Shaped {
        uplink,
        ifb,
        network,
        up: held_up,
        down: held_down,
        probes,
        trips: trips::Trips::default(),
        applied: (up, down),
        joel_told: None,
        state: State::Resting,
        follows: None,
        told: false,
        ceiling: false,
        real_time: realtime::RealTime::default(),
        holding: None,
        queued_reads: (0, 0),
        ahead_us: 0,
        vpn_reads: (0, 0),
        vpn_share: 1.0,
        agents_share: 1.0,
        drops: None,
        dropped: 0,
        told_at: None,
        download: Download::Turned,
        sent_read: None,
        sent_told: None,
        lanes,
        split,
        split_at: None,
        straight_tins: qdisc::TinRates::default(),
    };
    shaped.read_upload();
    Some(shaped)
}

/// Puts a link's queues on, its upload at `up` split as `split`, turning
/// what it receives through its IFB unless it goes `straight` in. Lanes
/// and an IFB already there, as a restart leaves them, are taken back at
/// these rates, so the link is never unshaped and nothing queued is lost;
/// a turn already there stays, since matchall refuses to replace its one
/// filter. Otherwise what was there goes first, since HTB can't change
/// where it sends what nothing claims: a link's own queue from the kernel
/// can't be taken away, and says so, which is nothing to tell. A read that
/// failed says nothing of what is there, so nothing is changed, and it
/// gives false.
fn install(
    link: &str,
    ifb: &str,
    (up, split): (u64, &lanes::Split),
    down: u64,
    straight: bool,
) -> bool {
    let (Some(held), Some(back)) = (queues_of(link), queues_of(ifb)) else {
        return false;
    };
    let turned = if lanes::laned(&held) && qdisc::ours(&back) {
        eprintln!("taking back the queues {link} kept through a restart");
        // tc complains of a queue with no filters to take off, which is
        // nothing to tell.
        for clear in lanes::unfiltered(link, &held)
            .iter()
            .chain(&lanes::unfiltered(ifb, &back))
        {
            let _ = sh(clear);
        }
        run_all(lanes::adopt(link, ifb, (up, split), down));
        qdisc::turned(&held)
    } else {
        let _ = sh(&words(&format!("tc qdisc del dev {link} root")));
        let _ = sh(&qdisc::unturn(link));
        run_all(qdisc::install(link, ifb, (up, split), down));
        false
    };
    if !straight && !turned {
        run_all(qdisc::turn(link, ifb));
    } else if straight && turned {
        let _ = sh(&qdisc::unturn(link));
    }
    true
}

/// Gives each link the round trips its probes heard since the last step.
fn hear_echoes(echoes: &Receiver<Echo>, shaped: &mut BTreeMap<String, Shaped>) {
    while let Ok(echo) = echoes.try_recv() {
        if let Some(s) = shaped.get_mut(&echo.link) {
            s.trips.heard(&echo.host, echo.rtt_ms);
        }
    }
}

/// The VPN's source-port pin as the shaper keeps it (src/pin.rs): its
/// rounds run on a thread of their own, since one takes seconds, and what
/// each read comes back here, kept for the journal.
struct Pinner {
    pinning: pin::Pinning,
    send: std::sync::mpsc::Sender<(pin::End, BTreeMap<u16, f64>)>,
    read: Receiver<(pin::End, BTreeMap<u16, f64>)>,
    last: BTreeMap<u16, f64>,
    looked: Option<Instant>,
    /// Whether the journal said the count of flows back can't be read,
    /// so it says so once rather than every look.
    unread: bool,
    /// Where the pin is kept across a restart (`pin::Restart`), so a switch
    /// doesn't lose it and a round after leaves out every pair the VPN's
    /// live flows were given; what was last written there; this boot,
    /// which what's kept there has to be from; and when the last round
    /// was asked, as an instant and in seconds since the epoch.
    kept_at: PathBuf,
    written: Option<String>,
    boot: String,
    asked_at: Option<(Instant, u64)>,
}

impl Pinner {
    fn new(state: &Path) -> Pinner {
        let (send, read) = channel();
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap_or_default();
        let kept_at = state.join("pin");
        let written = fs::read_to_string(&kept_at).ok();
        let mut pinning = pin::Pinning::default();
        let mut asked_at = None;
        if let Some(kept) = written
            .as_deref()
            .and_then(|t| pin::restored(t, boot.trim()))
        {
            if let Some(pin) = kept.pin {
                eprintln!(
                    "the VPN's pin to ports {} and {} is kept through the restart",
                    pin.first,
                    pin.first + 1
                );
            }
            asked_at = kept.asked.and_then(|at| Some((instant_of(at)?, at)));
            pinning.resume(kept, asked_at.map(|(at, _)| at));
        }
        Pinner {
            pinning,
            send,
            read,
            last: BTreeMap::new(),
            looked: None,
            unread: false,
            kept_at,
            written,
            boot: boot.trim().to_owned(),
            asked_at,
        }
    }

    /// Takes whether the pin's rule went in: one nftables refused lets go,
    /// so the journal never says the VPN leaves from a pair it doesn't.
    fn went(&mut self, applied: bool) {
        if !applied && let Some(first) = self.pinning.refused() {
            eprintln!("{}", pin::let_go(first, pin::REFUSED));
        }
    }

    /// Keeps what a restart needs of the pin, written whole or not at all,
    /// each time it changes. A write that fails is said once and tried
    /// again only at the next change, so it can't fill the journal.
    fn keep(&mut self) {
        let asked = self.pinning.asked();
        if self.asked_at.map(|(at, _)| at) != asked {
            self.asked_at = asked.and_then(|at| Some((at, epoch_of(at)?)));
        }
        let text = pin::saved(
            &self.pinning.kept(self.asked_at.map(|(_, at)| at)),
            &self.boot,
        );
        if self.written.as_ref() == Some(&text) {
            return;
        }
        let fresh = self.kept_at.with_extension("new");
        let wrote = fs::write(&fresh, &text).and_then(|()| fs::rename(&fresh, &self.kept_at));
        self.written = Some(text);
        // What was kept before is stale now, so it goes rather than come
        // back at a restart as a pin that may have let go.
        if let Err(e) = wrote {
            match fs::remove_file(&self.kept_at) {
                Err(stuck) if stuck.kind() != std::io::ErrorKind::NotFound => eprintln!(
                    "the VPN's pin can't be kept across a restart ({e}), nor what was kept \
                     before taken away ({stuck}), so a restart may take back a stale pin"
                ),
                _ => eprintln!(
                    "the VPN's pin can't be kept across a restart, so a restart forgets it: {e}"
                ),
            }
        }
    }

    /// Takes what a round read, puts its pin in force once the round's
    /// probes have let their ports go and says so when the pin moves, and
    /// every few seconds lets a pin go that the VPN's new flows can't get
    /// through, and looks for a new socket of the VPN's to probe for, out
    /// of the primary uplink, on its network. Gives the pin chain's
    /// transaction.
    fn step(&mut self, primary: Option<&Shaped>, calling: bool, now: Instant) -> String {
        let uplink = primary.map(|s| (s.uplink.name.as_str(), s.network.as_str()));
        if let Some((link, network)) = uplink
            && let Some(first) = self.pinning.network(link, network)
        {
            eprintln!("{}", pin::let_go(first, pin::ELSEWHERE));
        }
        while let Ok((end, read)) = self.read.try_recv() {
            let timeout = fs::read_to_string("/proc/sys/net/netfilter/nf_conntrack_udp_timeout");
            let from = now + pin::settle(&timeout.unwrap_or_default());
            if read.len() < pin::HEARD {
                eprintln!(
                    "a round toward the VPN's server heard {} of {} ports, too few to choose a pair",
                    read.len(),
                    pin::PORTS
                );
            }
            if let Some(first) = self.pinning.found(end, &read, from) {
                eprintln!("{}", pin::let_go(first, pin::UNJUDGED));
            }
            self.last = read;
        }
        if let Some(moved) = self.pinning.take_hold(now) {
            let first = self.pinning.pinned.map(|p| p.first);
            eprintln!("{}", pin::said(moved.was, first, &self.last));
        }
        if let Some((link, _)) = uplink
            && self
                .looked
                .is_none_or(|at| now - at >= Duration::from_secs(5))
        {
            self.looked = Some(now);
            let counted = sh(&words(&format!(
                "nft -j list counter inet {TABLE} pinretried"
            )));
            match counted.ok().as_deref().and_then(pin::retried) {
                Some(retried) => {
                    self.unread = false;
                    if let Some((pin::Moved { was: Some(first) }, times)) =
                        self.pinning.heed(retried)
                    {
                        eprintln!("{}", pin::dropped(first, times));
                    }
                }
                None if self.pinning.pinned.is_some() && !self.unread => {
                    self.unread = true;
                    eprintln!(
                        "the count of the VPN's flows that came back to its pin can't be read, \
                         so a pin they can't get through isn't let go"
                    );
                }
                None => {}
            }
            // A listing that failed says nothing of the VPN's flows, so the
            // pairs they hold are kept and no round goes until one answers.
            let listed = sh(&words(&format!("nft -j list set inet {TABLE} vpn_ends")));
            if let Some(ends) = listed.ok().as_deref().and_then(pin::ends) {
                self.pinning.flows(&ends, now);
                if let Some(end) = self.pinning.ask(&ends, calling, now) {
                    let (link, held) = (link.to_string(), self.pinning.held());
                    let (send, to) = (self.send.clone(), SocketAddrV4::new(end.to, end.port));
                    thread::spawn(move || send.send((end, hops::round(&link, to, &held))));
                }
            }
        }
        self.keep();
        pin::chain(TABLE, self.pinning.pinned.as_ref())
    }
}

/// The seconds since the epoch at `at`, or nothing where the clock can't
/// say.
fn epoch_of(at: Instant) -> Option<u64> {
    let ago = Instant::now().checked_duration_since(at)?;
    let then = SystemTime::now().checked_sub(ago)?;
    Some(then.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs())
}

/// The instant at `at` seconds since the epoch, or nothing where it's to
/// come or before this machine's clock began.
fn instant_of(at: u64) -> Option<Instant> {
    let then = SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(at))?;
    Instant::now().checked_sub(SystemTime::now().duration_since(then).ok()?)
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

/// The uplinks the default routes name, or what failed when a read of
/// them did.
fn uplinks() -> Result<Vec<links::Uplink>, String> {
    let routes = |family: &str| sh(&words(&format!("ip {family} -j route show default")));
    routes_seen(routes("-4"), routes("-6"))
}

/// The uplinks two reads of the default routes name, IPv4's and IPv6's. A
/// read that failed says nothing of the links: taken for no routes, it let
/// every link go, and a restart's sweep took their queues.
fn routes_seen(
    v4: Result<String, String>,
    v6: Result<String, String>,
) -> Result<Vec<links::Uplink>, String> {
    Ok(links::uplinks(&v4?, &v6?))
}

/// Whether a restart's sweep of the old shaper's queues may run: once the
/// routes were read, `order`, every VPN there was taken up, `read`, and so
/// was every link the routes name. Anything not taken up yet would go with
/// the rest, as would every link while none is.
fn may_sweep<V>(read: bool, order: Option<&[String]>, shaped: &BTreeMap<String, V>) -> bool {
    read && !shaped.is_empty() && order.is_some_and(|o| o.iter().all(|n| shaped.contains_key(n)))
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
    /// members. When it can't tell, what was last known stands. It asks
    /// about this machine alone (`--here`), since the set holds this
    /// machine's own UDP: a call on a laptop beside it would only put this
    /// machine's traffic ahead of the laptop's call on the same uplink.
    fn heed(&mut self, now: Instant, users: &Path) -> Vec<String> {
        if self.asked.is_none_or(|at| now - at >= CALL_EVERY) {
            self.asked = Some(now);
            let asked = Command::new("a-call-is-on")
                .arg("--here")
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

/// Each set's members at this step: the slices as they stand, the sessions
/// running a turn, whether a call is on and the work put first.
fn sorted(
    options: &Options,
    sessions: &[classes::Session],
    call: &mut Call,
    first: &mut first::Heed,
    now: Instant,
) -> classes::Sets {
    let mut sets = classes::read(&options.cgroups);
    (sets.bulk5, sets.typed5) = classes::by_typing(&options.cgroups, sessions);
    sets.calling = call.heed(now, &options.users);
    sets.first = first.heed(&options.cgroups, &options.users);
    sets
}

/// Runs each of `texts` through nft as a transaction of its own, so one
/// that fails, as a pin's rule the kernel refuses, leaves the others.
/// Loads each transaction, giving whether the last, the pin's, went.
fn refill(texts: &[String]) -> bool {
    let mut went = true;
    for text in texts {
        went = transact(text);
    }
    went
}

/// Loads one transaction, saying what nftables refused, and gives
/// whether it went.
fn transact(text: &str) -> bool {
    let child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        eprintln!("couldn't run nft");
        return false;
    };
    let _ = child
        .stdin
        .take()
        .expect("piped")
        .write_all(text.as_bytes());
    match child.wait_with_output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            eprintln!("nft: {}", String::from_utf8_lossy(&out.stderr).trim());
            false
        }
        Err(e) => {
            eprintln!("nft: {e}");
            false
        }
    }
}

/// The one-way controller, guarded by each network's anchor.
fn started_live(options: &Options) -> live::Live {
    let mut live = live::Live::new(&options.record, SystemTime::now());
    live.anchor(
        options
            .networks
            .as_deref()
            .map(networks::anchors)
            .unwrap_or_default(),
    );
    live.ceil(
        options
            .networks
            .as_deref()
            .map(networks::ceilings)
            .unwrap_or_default(),
    );
    live
}

fn run(options: &Options) -> Result<(), Failure> {
    let _ = fs::create_dir_all(&options.state);
    let _ = fs::create_dir_all(&options.runtime);
    // Where each network's rates were kept from one stay to the next, by
    // the round trip's controller and then the one-way one, which nothing
    // reads now, with the half-written copy its writer could leave.
    for kept in ["connections.json", "learned.json", "learned.json.new"] {
        let _ = fs::remove_file(options.state.join(kept));
    }
    let (send, echoes): (_, Receiver<Echo>) = channel();
    let mut shaped: BTreeMap<String, Shaped> = BTreeMap::new();
    let began = Instant::now();
    let mut last_look: Option<Instant> = None;
    let mut last_fill: Option<Instant> = None;
    let mut filled = [String::new(), String::new()];
    let mut first = first::Heed::default();
    let mut reach = classes::Local::default();
    let mut vpns = Vpns::default();
    // Listing every process costs more than a step can spend, so the pi
    // sessions are found every few seconds, and whether each runs a typed
    // turn is read at every step.
    let mut sessions = Vec::new();
    let mut last_sessions: Option<Instant> = None;
    let mut last_resent: Option<(u64, Instant)> = None;
    let mut heard: BTreeMap<String, Heard> = BTreeMap::new();
    let mut up_tins = qdisc::TinRates::default();
    let mut primary: Option<String> = None;
    let mut call = Call::default();
    let mut live = started_live(options);
    // Each network's ceiling holds its links whatever the controller asks,
    // or before it asks anything (`led`), and so does its house's.
    let mut most = NetworkCeilings::of(options);
    // What the controller heard and held each second, which the
    // recorder's lines can't say, kept for a replay (src/trace.rs).
    let mut traces = trace::Writer::new(&options.state.join("heard"));
    let mut swept = false;
    let mut pinner = Pinner::new(&options.state);
    heed_stopping();
    loop {
        if STOPPING.load(Ordering::Relaxed) {
            unshape_joel(&shaped);
            return Ok(());
        }
        let now = Instant::now();
        let t = (now - began).as_secs_f64();
        if last_sessions.is_none_or(|at| now - at >= Duration::from_secs(5)) {
            last_sessions = Some(now);
            sessions = classes::sessions(&options.proc, &options.users);
        }
        if last_look.is_none_or(|at| now - at >= Duration::from_secs(1)) {
            last_look = Some(now);
            let order = look(&mut shaped, options, &send);
            // A read of the routes that failed leaves the links as they
            // are, and the VPNs on the link they rode, as does a first link
            // not taken up yet, since letting them go would tear them down.
            if let Some(order) = &order {
                follow_leaders(&mut shaped, order);
                primary = order
                    .first()
                    .filter(|f| shaped.contains_key(*f))
                    .cloned()
                    .or(primary);
            }
            let rides = primary.as_ref().and_then(|p| shaped.get_mut(p));
            let read = follow_vpns(&mut vpns, &options.vpns, rides, (up_tins.kbit(), t));
            // A restart left the old shaper's queues; once this one has
            // taken up the links and VPNs it shapes, the rest go.
            if !swept && may_sweep(read, order.as_deref(), &shaped) {
                swept = true;
                sweep(&held(&shaped, &vpns.0));
            }
            for (name, s) in &mut shaped {
                if primary.as_ref() != Some(name) {
                    s.share_lanes((0.0, lanes::Agents::default()), t);
                }
            }
            reach = local(&shaped, &vpns.0);
            watch_real_time(&mut shaped, (now - began).as_secs_f64());
            live.tune(live::trial(&options.trial));
            tell_live(
                &mut live,
                options,
                &mut shaped,
                &vpns.0,
                (&mut last_resent, now),
            );
        }
        // A slice or unit made a moment ago reaches its set at the next
        // step. nixos-rebuild loads the table afresh on a switch, empty, so
        // the sets are also written every few seconds, changed or not.
        let sets = sorted(options, &sessions, &mut call, &mut first, now);
        let rides = primary.as_ref().and_then(|p| shaped.get_mut(p));
        read_vpns(&vpns.0, (&mut heard, &mut up_tins), rides, t);
        let text = [
            classes::transaction(TABLE, &sets, &reach),
            pinner.step(primary.as_ref().and_then(|p| shaped.get(p)), call.on, now),
        ];
        if text != filled || last_fill.is_none_or(|at| now - at >= Duration::from_secs(5)) {
            last_fill = Some(now);
            pinner.went(refill(&text));
            filled = text;
        }
        hear_echoes(&echoes, &mut shaped);
        let (house, steady) = most.house(Instant::now());
        step_links(&mut shaped, &mut live, (&most.written, &house, steady));
        hold_followers(&mut shaped);
        let heard = live.heard().map(|(link, mut h)| {
            h.up_tins_kbit = up_tins.kbit();
            (link, h)
        });
        write_status(&options.runtime, &shaped, heard.as_ref());
        if let Some(second) = live.traced() {
            traces.write(&traced(second, &shaped, &house));
        }
        thread::sleep(STEP);
    }
}

/// Has each link that reaches the same router as one before it in `order`
/// follow that one's rates, saying so when a link starts or stops.
fn follow_leaders(shaped: &mut BTreeMap<String, Shaped>, order: &[String]) {
    let networks: Vec<(&str, &str)> = order
        .iter()
        .filter_map(|n| shaped.get(n).map(|s| (n.as_str(), s.network.as_str())))
        .collect();
    let follows = links::leaders(&networks);
    for (name, s) in shaped {
        let leader = follows.get(name).cloned();
        if leader != s.follows {
            match &leader {
                Some(l) => eprintln!("{name} reaches the same router as {l}; taking its rate"),
                None if s.follows.is_some() => eprintln!("{name} finds its own rate again"),
                None => {}
            }
            s.follows = leader;
        }
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
    let mut seen = BTreeMap::new();
    for (name, s) in shaped.iter_mut() {
        if let Some(ours) = s.ours(waiting, resent, now) {
            seen.insert(name.clone(), ours);
        }
    }
    let follows: BTreeMap<String, String> = shaped
        .iter()
        .filter_map(|(name, s)| s.follows.clone().map(|leader| (name.clone(), leader)))
        .collect();
    crossing(&mut seen, &follows);
    for (name, ours) in seen {
        l.see(&name, ours);
    }
    l.hear_vpn(vpns.keys().next().map(String::as_str));
}

/// Adds what each follower's CAKE sent, dropped and held to its leader's:
/// the two send over one connection, so what crossed it is what both sent,
/// and TCP's resends, the machine's, are of what both dropped. A leader
/// whose follower's count can't be told, or none of whose reads worked
/// that second, can't tell what crossed either, so is judged by its rate,
/// rather than by a sum leaving the follower out.
fn crossing(seen: &mut BTreeMap<String, oneway::Ours>, follows: &BTreeMap<String, String>) {
    for (follower, leader) in follows {
        let theirs = seen.get(follower).copied();
        let Some(ours) = seen.get_mut(leader) else {
            continue;
        };
        let Some(theirs) = theirs else {
            ours.sent = None;
            continue;
        };
        ours.sent = ours.sent.zip(theirs.sent).map(|(own, sent)| own + sent);
        ours.dropped += theirs.dropped;
        // Only a VPN's queue stood on the line while none stood of a link's
        // own, on either.
        if theirs.queued {
            ours.in_vpn = theirs.in_vpn && (ours.in_vpn || !ours.queued);
            ours.queued = true;
        }
        ours.vpn_queued |= theirs.vpn_queued;
        // Agents straight on a follower aren't the leader's to hold: the
        // hold lands on the leader's lanes alone, so only its own agents
        // yield.
    }
}

/// Takes a VPN's queues away, and its IFB, once it's gone or has no link
/// to ride; a device that's gone took its own queues with it.
fn let_vpn_go(applied: &mut BTreeMap<String, Vpn>, name: &str, dir: &Path) {
    if let Some(gone) = applied.remove(name) {
        if dir.exists() {
            let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
            let _ = sh(&words(&format!("tc qdisc del dev {name} ingress")));
        }
        let _ = sh(&words(&format!("ip link del {}", gone.ifb)));
    }
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
    held: Option<held::Holding>,
    drops: Option<u64>,
}

/// What a count read `now` rose by since it read `before`: nothing at a
/// count's first read, or once it starts over, as a link's queues made
/// again do.
fn since(before: Option<u64>, now: u64) -> u64 {
    before.map_or(0, |b| now.saturating_sub(b))
}

/// Counts each VPN's CAKE at every step, at `t` seconds: whether any held
/// a queue, and what they dropped, are counted to the primary uplink they
/// ride, whose queue it is, and what each of their tins sent to `tins`.
/// The VPN's tins are read rather than the uplink's, where the tunnel's
/// packets all wait in best effort, whatever class they carry.
fn read_vpns(
    vpns: &BTreeMap<String, Vpn>,
    (heard, tins): (&mut BTreeMap<String, Heard>, &mut qdisc::TinRates),
    primary: Option<&mut Shaped>,
    t: f64,
) {
    heard.retain(|name, _| vpns.contains_key(name));
    let mut read = (0, 0);
    let mut dropped = 0;
    let mut sent: Vec<u64> = Vec::new();
    for name in vpns.keys() {
        let json = sh(&words(&format!("tc -s -j qdisc show dev {name}"))).unwrap_or_default();
        let these: Vec<u64> = qdisc::tins(&json).iter().map(|tin| tin.bytes).collect();
        if sent.is_empty() {
            sent = these;
        } else if these.len() == sent.len() {
            sent.iter_mut()
                .zip(these)
                .for_each(|(all, one)| *all += one);
        }
        let h = heard.entry(name.clone()).or_default();
        if let Some(now) = held::holding(&json) {
            let queued = held::queued(h.held.replace(now), now);
            read = (read.0.max(u32::from(queued)), 1);
        }
        if let Some(now) = held::drops(&json) {
            dropped += since(h.drops.replace(now), now);
        }
    }
    tins.take(sent, t);
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
/// their count.
type Vpns = (BTreeMap<String, Vpn>, cost::Cost);

/// Keeps a CAKE on each VPN that's there, each way: up, under the class
/// the primary uplink's lanes give the VPNs riding it, less what their own
/// packets cost the uplink past what their CAKE counts; down, at a share
/// of the uplink's rate. Splits the primary uplink's lanes on the way,
/// from what was sent inside the VPNs, `inside`, and what their bulk
/// holds. Lets go of one that's gone or has no uplink to ride. What a VPN
/// receives goes straight in, past its IFB, while the uplink's does. Read
/// at `t` seconds. Gives whether every VPN there was taken up: one whose
/// read failed is left as it was, and one with no link to ride is let go.
fn follow_vpns(
    (applied, spent): &mut Vpns,
    names: &[String],
    primary: Option<&mut Shaped>,
    (inside, t): (Option<qdisc::Tins>, f64),
) -> bool {
    let mut every_read = true;
    let read: Vec<String> = applied
        .keys()
        .map(|name| sh(&words(&format!("tc -s -j qdisc show dev {name}"))).unwrap_or_default())
        .collect();
    let counted: Option<u64> = read.iter().map(|json| qdisc::counted_bytes(json)).sum();
    // The drops are counted only while every VPN's CAKE was read, so one
    // missed reads as a fresh count rather than a press.
    let agents = read.iter().map(|json| qdisc::bulk(json)).fold(
        lanes::Agents {
            dropped: Some(0),
            ..lanes::Agents::default()
        },
        |all, tin| match tin {
            Some(tin) => lanes::Agents {
                backlog: all.backlog + tin.backlog,
                flows: all.flows + tin.flows,
                dropped: all.dropped.map(|d| d + tin.drops + tin.marks),
            },
            None => lanes::Agents {
                dropped: None,
                ..all
            },
        },
    );
    let joel = inside.map_or(0.0, |k| float(k.best + k.video + k.voice));
    let base = primary.map(|s| {
        let vpn = s.share_lanes((joel, agents), t);
        // Held under its tunnel's pace while it waits inside it, so the
        // queue stands in this CAKE; agents' direct lane borrows the rest.
        let held = vpn * lanes::VPN_HELD * s.vpn_share;
        (rate::whole(held), s.applied.1, s.download.straight())
    });
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
            // A VPN that is there isn't taken up, with no link to ride or
            // its index unread.
            every_read &= !dir.exists();
            let_vpn_go(applied, name, &dir);
            continue;
        };
        let ifb = qdisc::ifb(index);
        let up = up.saturating_sub(extra).max(FLOOR.up);
        let down = qdisc::vpn_rate(down).max(FLOOR.down);
        let was = applied.get(name);
        // A VPN brought back up is a new device, with a new index.
        if let Some(old) = was.filter(|v| v.ifb != ifb) {
            let _ = sh(&words(&format!("ip link del {}", old.ifb)));
        }
        let same_ifb = was.is_some_and(|v| v.ifb == ifb);
        let (was_up, was_down) = (was.map(|v| v.up), was.map(|v| v.down).filter(|_| same_ifb));
        // A read that failed leaves the VPN's queues as they are until one
        // answers.
        let (Some(json), Some(back)) = (queues_of(name), queues_of(&ifb)) else {
            every_read = false;
            continue;
        };
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
    every_read
}

/// Follows the uplinks: shapes a new one, lets a gone one go, and puts back
/// queues somebody took away or a link that came back without. Returns the
/// uplinks in the routes' order, the first being the one VPNs ride, or
/// nothing when the routes couldn't be read, which leaves every link as it
/// is rather than taking each for gone.
fn look(
    shaped: &mut BTreeMap<String, Shaped>,
    options: &Options,
    send: &std::sync::mpsc::Sender<Echo>,
) -> Option<Vec<String>> {
    let now = match uplinks() {
        Ok(now) => now,
        Err(e) => {
            eprintln!("{e}: the links are left as they are until a read answers");
            return None;
        }
    };
    let order: Vec<String> = now.iter().map(|u| u.name.clone()).collect();
    let gone: Vec<String> = shaped
        .keys()
        .filter(|k| !now.iter().any(|u| &u.name == *k))
        .cloned()
        .collect();
    for name in gone {
        if let Some(s) = shaped.remove(&name) {
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
                if let Some(mut s) =
                    start(uplink, options.networks.as_deref(), &options.hosts, send)
                {
                    if options.turn_download {
                        s.download = Download::Kept;
                    }
                    shaped.insert(name, s);
                }
            }
            Some(s) => {
                // A read that failed leaves the link's queues as they are
                // until one answers, rather than taking them down to put
                // them back.
                let seen = queues_of(&s.uplink.name).zip(queues_of(&s.ifb));
                if let Some((json, ifb)) = seen
                    && (!lanes::laned(&json)
                        || !(s.download.straight() || qdisc::turned(&json))
                        || !qdisc::ours(&ifb))
                {
                    eprintln!("{}'s queues were gone; putting them back", s.uplink.name);
                    s.holding = None;
                    s.split = s.lanes.at(float(s.up.rate));
                    if install(
                        &s.uplink.name,
                        &s.ifb,
                        (s.up.rate, &s.split),
                        s.down.rate,
                        s.download.straight(),
                    ) {
                        s.applied = (s.up.rate, s.down.rate);
                        s.joel_told = None;
                    }
                }
                if !s.probes.running() {
                    s.probes = Probes::start(&s.uplink.name, &options.hosts, 0.3, send);
                }
                // A cable moved to another port can link at another speed
                // without the link going away: a desk's went from 100 Mbit/s
                // to gigabit, and its rates stayed held under the old wire.
                let max = ceiling(&s.uplink.name);
                let up = s.up.bound(max.up);
                if s.down.bound(max.down) || up {
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
                    s.network = network_of(&s.uplink);
                }
            }
        }
    }
    Some(order)
}

/// Follows whether real-time traffic crosses each link, from the packets
/// its video tins sent each way, for its status, and tells the journal
/// when that changes.
fn watch_real_time(shaped: &mut BTreeMap<String, Shaped>, t: f64) {
    for (name, s) in shaped {
        let packets = [&s.uplink.name, &s.ifb]
            .iter()
            .map(|dev| {
                sh(&words(&format!("tc -s -j qdisc show dev {dev}")))
                    .map_or(0, |json| qdisc::video_packets(&json))
            })
            .sum();
        let was = s.real_time.flows();
        let live = s.real_time.count(packets, t);
        if live != was {
            if live {
                eprintln!("{name}: real-time traffic flows now");
            } else {
                // Worded apart from the line it ends, which the VM tests
                // look for.
                eprintln!("{name}: the real-time traffic stopped");
            }
        }
    }
}

/// Holds each follower to its leader's rates, so the two together stay
/// under the one connection; its own pings still say its round trips. Its
/// download goes straight in as its leader's does, since nothing limits
/// the one connection's. Whether a limit holds its upload is judged
/// against the lower of its leader's ceiling and its own, so its CAKE is
/// never told a rate its own link can't carry.
fn hold_followers(shaped: &mut BTreeMap<String, Shaped>) {
    let rates: BTreeMap<String, (u64, u64, bool, u64)> = shaped
        .iter()
        .map(|(n, s)| {
            let held = (
                s.up.rate,
                s.down.rate,
                s.download.straight(),
                s.up.bounds.max,
            );
            (n.clone(), held)
        })
        .collect();
    for s in shaped.values_mut() {
        if let Some(&(up, down, straight, most)) = s.follows.as_ref().and_then(|l| rates.get(l)) {
            s.up.rate = up;
            s.down.rate = down;
            apply(s, most.min(s.up.bounds.max));
            go_straight(s, straight);
        }
    }
}

/// One step of the controller, `live`, over every uplink that leads its
/// router: a follower takes its leader's rates after.
fn step_links(
    shaped: &mut BTreeMap<String, Shaped>,
    live: &mut live::Live,
    (ceilings, house, steady): (&networks::Ceilings, &HouseHolds, networks::Ceilings),
) {
    // Every link's upload CAKE is read, followers' too, for whose queue
    // each is when the controller is next told (`Shaped::ours`).
    for s in shaped.values_mut() {
        s.read_upload();
    }
    // The controller holds its own rate under what the house leaves, as
    // under a trial's ceiling, the highest of the last few seconds' so it
    // holds still; the link is held to each second's below.
    live.house(steady);
    // The one-way controller holds the link the recorder follows; any
    // other goes at its whole rate, its traffic still sorted by CAKE.
    // One moment for all three, after the poll reads what's new, so the
    // rates, the reason and the VPN's share judge the recorder's silence
    // alike.
    let now = SystemTime::now();
    let (link, rates) = match live.poll(now) {
        Some((link, rates)) => (Some(link.to_string()), rates),
        None => (None, replay::Rates::default()),
    };
    let why = live.why(now);
    let share = live.vpn_share(now);
    let agents = live.agents_share(now);
    for (name, s) in shaped.iter_mut() {
        let held = link.as_deref() == Some(name.as_str());
        // The VPN is held only while the upload is: a recorder gone
        // silent lets both go, unless a ceiling holds the upload, when
        // the VPN has its whole share of it. A follower isn't held at
        // all, so a link held when it led is let go once it follows.
        let link_is = (held, rates.up.is_some(), s.follows.is_some());
        (s.vpn_share, s.agents_share) = held_shares(link_is, (share, agents));
        if s.agents_share >= 1.0 {
            s.let_agents_go();
        }
        if s.follows.is_some() {
            continue;
        }
        let most = lower(
            ceilings.get(&s.network).copied().map(rate::whole),
            house.get(&s.network).map(|&(_, most)| most),
        );
        if held {
            hold(s, led(rates, true, most), &why);
        } else {
            hold(s, led(rates, false, most), &why::Judged::Nothing);
        }
    }
}

/// The shares of their lanes the VPN and agents straight are held to on a
/// link, `(vpn, agents)` from the controller's holds: those while it holds
/// the link's upload, which takes the link being the one it follows, its
/// upload limited and the link not following another, and whole otherwise.
fn held_shares(
    (followed, limited, follows): (bool, bool, bool),
    (vpn, agents): (f64, f64),
) -> (f64, f64) {
    if followed && limited && !follows {
        (vpn, agents)
    } else {
        (1.0, 1.0)
    }
}

/// What a link's queues are held to, from the shares `held_shares` gave it,
/// `(vpn, agents)`, and the split its classes hold: the VPN's share as
/// given, and agents' only while the split caps their lane, which it does
/// only while they wait straight (src/lanes.rs). A hold climbing back while
/// they don't caps nothing, so a trace saying it would read as a hold that
/// wasn't there.
fn applied_held((vpn, agents): (f64, f64), split: &lanes::Split) -> trace::Held {
    trace::Held {
        vpn,
        agents: if split.agents_most.is_some() {
            agents
        } else {
            1.0
        },
    }
}

/// What each network's house holds its links to this step, by network:
/// what the other machine last said it sent and the most the ceiling
/// leaves this one, in kbit/s.
type HouseHolds = BTreeMap<String, (u64, u64)>;

/// Each network's ceilings: the one it's written with, and its house's,
/// which holds this machine under what another on the line leaves while
/// that one says what it sends (src/house.rs).
struct NetworkCeilings {
    written: networks::Ceilings,
    houses: networks::Houses,
    ear: Option<house::Ear>,
    /// Whether a house held anything at the last step, so the journal
    /// says only when that changes.
    held: bool,
    steady: house::Steady,
}

impl NetworkCeilings {
    fn of(options: &Options) -> Self {
        let file = options.networks.as_deref();
        let houses = file.map(networks::houses).unwrap_or_default();
        let from = || house::Ear::start(houses.values().map(|h| h.from).collect());
        Self {
            written: file.map(networks::ceilings).unwrap_or_default(),
            ear: (!houses.is_empty()).then(from),
            houses,
            held: false,
            steady: house::Steady::default(),
        }
    }

    /// What each network's house holds its links to at `now`, said in the
    /// journal as it starts or stops holding, and the ceiling it gives
    /// the controller there (`house::Steady`).
    fn house(&mut self, now: Instant) -> (HouseHolds, networks::Ceilings) {
        let house = house_holds(&self.houses, self.ear.as_ref(), now);
        if house.is_empty() == self.held {
            self.held = !self.held;
            eprintln!("{}", house_said(&house));
        }
        let held = house
            .iter()
            .map(|(network, &(_, most))| (network.clone(), most))
            .collect();
        let steady = self
            .steady
            .of(&held, now)
            .into_iter()
            .map(|(network, most)| (network, rate::float(most)));
        (house, steady.collect())
    }
}

/// The traced `second` as its link held it: the shares main put on the
/// link's queues, and what the link's house heard and held.
fn traced(
    second: trace::Line,
    shaped: &BTreeMap<String, Shaped>,
    house: &HouseHolds,
) -> trace::Line {
    let its = shaped.get(&second.link);
    let network = its.map(|s| s.network.clone());
    let mut line = second.held(its.map_or(trace::Held::WHOLE, Shaped::held));
    house_traced(&mut line, network.as_deref(), house);
    line
}

/// Puts in the traced `line` what its link's house heard and held, when
/// one holds it: the house of `network`, the link's. The house held the
/// link to this second's, under the steady ceiling the controller was
/// given, so it is the second's ceiling where lower: the bars and a held
/// replay set a rate held there apart as the ceiling's.
fn house_traced(line: &mut trace::Line, network: Option<&str>, house: &HouseHolds) {
    if let Some(&(sent, most)) = network.and_then(|n| house.get(n)) {
        (line.house_sent, line.house_most) = (Some(sent), Some(most));
        line.ceiling = Some(line.ceiling.map_or(most, |ceiling| ceiling.min(most)));
    }
}

/// What each network's house holds its links to at `now`: none where the
/// other machine hasn't said lately what it sends, or none is heard, and
/// never under the least a trial may set, where a call still fits. The
/// address alone says a report is the other machine's, so this bounds what
/// one forged on the line can do to what a trial may.
fn house_holds(houses: &networks::Houses, ear: Option<&house::Ear>, now: Instant) -> HouseHolds {
    let Some(ear) = ear else {
        return HouseHolds::new();
    };
    let least = rate::whole(oneway::MOST_KBIT_LEAST);
    houses
        .iter()
        .filter_map(|(network, h)| {
            let sent = ear.sent(h.from, now);
            let most = house::held(h.most, sent, least)?;
            Some((network.clone(), (rate::whole(sent?.0), most)))
        })
        .collect()
}

/// The journal's line as the house's ceiling starts holding, `house`, or
/// stops, with no report fresh.
fn house_said(house: &HouseHolds) -> String {
    if house.is_empty() {
        return "the house's other machine went quiet, so its ceiling holds nothing".to_string();
    }
    house
        .iter()
        .map(|(network, (sent, most))| {
            format!(
                "the house's ceiling holds {network}'s upload under {most} kbit/s, the other \
                 machine sending {sent}"
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The lower of two ceilings, either of which may be unset.
fn lower(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// What a link is held to at a step: the controller's `rates` on the
/// link it holds, `held`, and none on another, under the ceiling of the
/// link's network, `most`, in kbit/s, either way. So a network's ceiling
/// holds before the controller has stepped on a second, as when the
/// shaper starts, and on a link the controller doesn't hold.
fn led(rates: replay::Rates, held: bool, most: Option<u64>) -> replay::Rates {
    let rates = if held {
        rates
    } else {
        replay::Rates::default()
    };
    replay::Rates {
        up: rates
            .up
            .map_or(most, |up| Some(most.map_or(up, |most| up.min(most)))),
        ..rates
    }
}

/// Holds a link to the rates the one-way controller answers, at its
/// ceiling where it sets none, and tells the journal when a spell of limits
/// begins, with the controller's reason `why`, and when it ends, rather
/// than of each second's cut.
fn hold(s: &mut Shaped, rates: replay::Rates, why: &why::Judged) {
    let most = Held {
        up: s.up.bounds.max,
        down: s.down.bounds.max,
    };
    let spell = spell(&s.uplink.name, (s.told, s.ceiling), rates, most, why);
    s.up.rate = spell.held.up;
    s.down.rate = spell.held.down;
    if let Some(line) = &spell.said {
        eprintln!("{line}");
    }
    (s.told, s.ceiling) = spell.state();
    // A spell waiting to be told holds the link as one told does.
    s.state = if rates.up.is_some() || rates.down.is_some() {
        State::Holding
    } else {
        State::Resting
    };
    apply(s, s.up.bounds.max);
    go_straight(s, rates.down.is_none());
}

/// What a second's rates come to on a link: what each way is held to,
/// whether a spell of limits is told, and whether nothing the controller
/// judged holds it, a ceiling alone, with the journal's line as
/// one begins or ends, or as the controller's judgement stops holding it
/// or holds it again.
#[derive(Debug, PartialEq)]
struct Spell {
    held: Held,
    told: bool,
    ceiling: bool,
    said: Option<String>,
}

impl Spell {
    /// What the next second's spell is handed: whether it's told, and
    /// whether a ceiling alone holds it.
    fn state(&self) -> (bool, bool) {
        (self.told, self.ceiling)
    }
}

/// Holds each way to the rate asked, kept between the floor and `most`, or
/// to `most` where none is asked, and says a spell beginning, with the
/// controller's reason for the rate it held, or ending, a spell `told`
/// already or not, and held then by a ceiling alone, `silent`, or not.
/// `why` says what the controller's judgement holds: where nothing it
/// judged does, a ceiling is credited only with the rate it asked.
fn spell(
    name: &str,
    (told, silent): (bool, bool),
    rates: replay::Rates,
    most: Held,
    why: &why::Judged,
) -> Spell {
    let held =
        |rate: Option<u64>, floor: u64, max: u64| rate.map_or(max, |r| r.max(floor).min(max));
    let held = Held {
        up: held(rates.up, FLOOR.up, most.up),
        down: held(rates.down, FLOOR.down, most.down),
    };
    let limited = rates.up.is_some() || rates.down.is_some();
    if limited && *why == why::Judged::Waiting {
        // The controller's rate from before a silence holds while its
        // reason waits for a second heard since: no ceiling holds it, so
        // the spell stays as told, or waits to be told with the reason.
        return Spell {
            held,
            told,
            ceiling: silent,
            said: None,
        };
    }
    let quiet = why.reason().is_none();
    let said = if limited && (!told || quiet != silent) {
        let up = rates.up.map(|_| held.up);
        let down = rates.down.map(|_| held.down);
        Some(if quiet && rates.up == Some(held.up) && down.is_none() {
            why::ceiling(name, held.up)
        } else {
            why::holding(name, up, down, why.reason())
        })
    } else if !limited && told {
        Some(format!(
            "{name}: neither direction is held now, so the link's whole rate is kept"
        ))
    } else {
        None
    };
    Spell {
        held,
        told: limited,
        ceiling: limited && quiet,
        said,
    }
}

/// Lets what a link receives go straight in, past its IFB, or turns it
/// back through once a limit holds, after `apply` gave the IFB's CAKE its
/// rate. `qdisc::unturn` says why.
fn go_straight(s: &mut Shaped, straight: bool) {
    let straight = straight && s.download != Download::Kept;
    if straight == s.download.straight() {
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
        s.download = if straight {
            Download::Straight
        } else {
            Download::Turned
        };
        if straight {
            eprintln!("{name}: nothing limits the download, so it goes straight in");
        } else {
            eprintln!("{name}: the download is limited, so it queues in CAKE again");
        }
    }
}

/// Puts a link's rates on its queues when they moved by more than 1%, and
/// tells Joel's CAKE a rate while the upload's is under `most`, the
/// ceiling that holds it: its own, or for a follower the lower of its
/// leader's and its own.
fn apply(s: &mut Shaped, most: u64) {
    let moved = |a: u64, b: u64| a.abs_diff(b) * 100 > b;
    let (up, down) = (s.up.rate, s.down.rate);
    if moved(up, s.applied.0) {
        // The lanes move with the rate at once, so the classes never
        // promise more than it.
        let split = s.lanes.at(float(up));
        let took = run_all(lanes::change(&s.uplink.name, up, &split));
        if took {
            s.applied.0 = up;
        }
        s.split = s.split.after(split, took);
    }
    // Apart from the classes, so a CAKE that refuses its rate never makes
    // them read as unmoved, and asked again each step until it takes.
    let joel = lanes::joel_cake(s.up.rate, most, s.applied.0);
    if s.joel_told != Some(joel) {
        match sh(&lanes::joel_rate(&s.uplink.name, joel)) {
            Ok(_) => s.joel_told = Some(joel),
            Err(e) => eprintln!("{e}"),
        }
    }
    if moved(down, s.applied.1) {
        match sh(&qdisc::change_down(&s.ifb, down)) {
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
    state: String,
    /// Each distant host's quiet round trip, in ms.
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
    /// How the upload is shared between its lanes, and the agents'
    /// flows waiting on each.
    #[serde(default)]
    lanes: Option<lanes::Said>,
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
            state: format!("{:?}", s.state).to_lowercase(),
            baselines: s.trips.quiet(),
            follows: s.follows.clone(),
            live: s.real_time.flows(),
            straight: s.download.straight(),
            heard: heard
                .filter(|(link, _)| *link == s.uplink.name)
                .map(|(_, heard)| heard.clone()),
            lanes: s.lanes.said(&s.split),
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
    format!("{:.1} Mb/s", float(kbit) / 1000.0)
}

/// Whether systemd is restarting `unit`. When it can't say, it isn't, so
/// the queues go as on any stop.
fn restarting(unit: &str) -> bool {
    let mut ask = words("systemctl list-jobs --no-legend --plain --full");
    ask.push(unit.to_string());
    match sh(&ask) {
        Ok(jobs) => restart::pending(&jobs, unit),
        Err(why) => {
            eprintln!("systemd couldn't say whether {unit} restarts, so its queues go: {why}");
            false
        }
    }
}

/// Set once systemd asks the running shaper to stop.
static STOPPING: AtomicBool = AtomicBool::new(false);

/// Only marks the stop, since a signal's handler may do little else; the
/// loop sees it within a step.
extern "C" fn stopping(_: libc::c_int) {
    STOPPING.store(true, Ordering::Relaxed);
}

/// Has a SIGTERM end the loop rather than the process, so the shaper can
/// leave its queues as any version of it takes them back.
fn heed_stopping() {
    let handler = stopping as extern "C" fn(libc::c_int);
    // SAFETY: the handler only stores to an atomic, which is safe to do in
    // a signal's handler, and it lives as long as the process.
    unsafe {
        libc::signal(libc::SIGTERM, handler as libc::sighandler_t);
    }
}

/// Tells each link's CAKE in Joel's lane it has no rate, as it is built
/// before this shaper gives it one. A restart keeps the queues for the
/// next shaper to take back, and one that never tells the CAKE a rate, as
/// before this, would keep the last one held for as long as it ran. Every
/// link is told, whatever `joel_told` says, since telling a CAKE with no
/// rate costs nothing, and a rate set by a `tc` whose answer was lost
/// would otherwise stay.
fn unshape_joel(shaped: &BTreeMap<String, Shaped>) {
    for s in shaped.values() {
        if let Err(why) = sh(&lanes::joel_rate(&s.uplink.name, lanes::Cake::Unlimited)) {
            eprintln!(
                "{}: Joel's CAKE kept its rate as the shaper stopped: {why}",
                s.uplink.name
            );
        }
    }
}

/// Takes away, for when the shaper stops, every queue it made, and the
/// VPN's pin, which would
/// otherwise stay in force with nothing to let it go until the table is
/// loaded again.
/// Lets the pin go first, since a stop cut short by its timeout would
/// otherwise leave the VPN pinned with no shaper to let it go, then takes
/// every queue away. The pin kept for a restart stays where it's kept.
fn stop() {
    transact(&pin::chain(TABLE, None));
    sweep(&BTreeSet::new());
}

/// The links, VPNs and IFBs the shaper holds now.
fn held(shaped: &BTreeMap<String, Shaped>, vpns: &BTreeMap<String, Vpn>) -> BTreeSet<String> {
    let links = shaped.iter().map(|(name, s)| (name, &s.ifb));
    let tunnels = vpns.iter().map(|(name, v)| (name, &v.ifb));
    links
        .chain(tunnels)
        .flat_map(|(name, ifb)| [name.clone(), ifb.clone()])
        .collect()
}

/// Takes away every queue the shaper made but on the devices `kept`.
fn sweep(kept: &BTreeSet<String>) {
    let Ok(devices) = fs::read_dir("/sys/class/net") else {
        return;
    };
    for device in devices.flatten() {
        let name = device.file_name().to_string_lossy().to_string();
        if kept.contains(&name) {
            continue;
        }
        if name.starts_with("shaper") {
            let _ = sh(&words(&format!("ip link del {name}")));
            continue;
        }
        // A read that failed leaves the device's queues, and says so.
        let Some(json) = queues_of(&name) else {
            continue;
        };
        if qdisc::ours(&json) || lanes::laned(&json) || qdisc::vpn_ours(&json) {
            let _ = sh(&words(&format!("tc qdisc del dev {name} root")));
            let _ = sh(&words(&format!("tc qdisc del dev {name} ingress")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The controller's holds land on a link only while it holds the
    // link's upload, each on its own lane.
    #[test]
    fn a_link_s_lanes_are_held_only_while_its_upload_is() {
        let shares = (0.6, 0.4);
        assert_eq!(held_shares((true, true, false), shares), shares);
        for (link_is, what) in [
            ((false, true, false), "a link the controller doesn't follow"),
            ((true, false, false), "an upload with no limit"),
            ((true, true, true), "a follower"),
        ] {
            assert_eq!(held_shares(link_is, shares), (1.0, 1.0), "{what} was held");
        }
    }

    // A trace says the holds the link's queues took, not the controller's
    // shares: agents' climbing back while they don't wait caps nothing.
    #[test]
    fn a_trace_says_agents_hold_only_while_their_lane_is_capped() {
        let split = |agents_most| lanes::Split {
            joel: 3_000.0,
            vpn: 6_000.0,
            agents: 3_000.0,
            agents_most,
            unsent: false,
        };
        // Agents' cap is the floor's, not their share of their lane, so
        // the share the trace records can't be taken for the cap.
        assert_eq!(
            applied_held((0.6, 0.4), &split(Some(900.0))),
            trace::Held {
                vpn: 0.6,
                agents: 0.4
            },
            "each share where it's taken, by name, and not the cap"
        );
        assert_eq!(
            applied_held((0.6, 0.4), &split(None)),
            trace::Held {
                vpn: 0.6,
                agents: 1.0
            },
            "agents' share with their lane uncapped held nothing"
        );
        let (vpn, agents) = held_shares((true, false, false), (0.6, 0.4));
        assert_eq!(
            applied_held((vpn, agents), &split(Some(1_200.0))),
            trace::Held::WHOLE,
            "an upload with no limit holds neither"
        );
    }

    #[test]
    fn only_1_turns_every_download() {
        for (value, turns) in [
            (Some("1"), true),
            (Some("0"), false),
            (Some(""), false),
            (Some("true"), false),
            (None, false),
        ] {
            assert_eq!(
                turns_download(value),
                turns,
                "SHAPER_TURN_DOWNLOAD={value:?}"
            );
        }
    }

    // On 18 November the rate test's uploads over a second link to the
    // router crossed the connection without the first link's line hearing
    // of them, so its controller never cut.
    #[test]
    fn a_follower_s_sending_crosses_its_leader_s_line() {
        let sent = |kbit| oneway::Ours {
            sent: Some(kbit),
            ..oneway::Ours::default()
        };
        let mut seen = BTreeMap::from([
            ("eth1".to_owned(), sent(300)),
            ("eth2".to_owned(), sent(7_000)),
            ("wlan0".to_owned(), sent(50)),
        ]);
        let follows = BTreeMap::from([("eth2".to_owned(), "eth1".to_owned())]);
        seen.get_mut("eth2").expect("eth2").dropped = 400;
        crossing(&mut seen, &follows);
        assert_eq!(seen["eth1"].sent, Some(7_300), "{seen:?}");
        // TCP's resends are the machine's, so what the follower's CAKE
        // dropped is no router's either.
        assert_eq!(seen["eth1"].dropped, 400, "{seen:?}");
        assert_eq!(seen["eth2"].sent, Some(7_000), "a follower keeps its own");
        assert_eq!(
            seen["wlan0"].sent,
            Some(50),
            "a link nobody follows keeps its own"
        );
        let mut unread = seen.clone();
        unread.get_mut("eth1").expect("eth1").sent = Some(300);
        unread.get_mut("eth2").expect("eth2").sent = None;
        crossing(&mut unread, &follows);
        assert_eq!(unread["eth1"].sent, None, "a sum left the follower out");
        let mut unseen = seen;
        unseen.get_mut("eth1").expect("eth1").sent = Some(300);
        unseen.remove("eth2");
        crossing(&mut unseen, &follows);
        assert_eq!(unseen["eth1"].sent, None, "a sum left the follower out");
    }

    #[test]
    fn what_cake_sent_is_counted_between_good_reads() {
        let start = Instant::now();
        let read = |bytes, ms| Some((bytes, start + Duration::from_millis(ms)));
        // What was read before and now, and what CAKE sent between them:
        // 125,000 bytes is 1000 kbit over a second.
        let cases = [
            (None, read(125_000, 1_000), None),
            (read(0, 0), None, None),
            (read(0, 0), read(125_000, 1_000), Some(1_000)),
            // The last read of a second failed, so the count runs to the
            // one before it, over the time between them.
            (read(0, 0), read(100_000, 800), Some(1_000)),
            // Every read since failed: nothing new to tell.
            (read(125_000, 1_000), read(125_000, 1_000), None),
            // A read that worked with nothing sent: an idle second.
            (read(125_000, 1_000), read(125_000, 2_000), Some(0)),
            // CAKE was put in again and counts from nothing.
            (read(125_000, 1_000), read(5_000, 2_000), None),
        ];
        for (then, now, sent) in cases {
            assert_eq!(sent_kbit(then, now), sent, "{then:?} to {now:?}");
        }
    }

    #[test]
    fn a_follower_s_queue_is_its_leader_s_and_only_a_vpn_s_stays_the_vpn_s() {
        // Whether a queue stood, whether only a VPN's did and whether a
        // VPN's did beside the link's own: the leader's and the follower's,
        // and the first two as the leader reads them after.
        let cases = [
            ((false, false, false), (false, false, false), (false, false)),
            ((false, false, false), (true, true, true), (true, true)),
            ((false, false, false), (true, false, false), (true, false)),
            ((true, true, true), (false, false, false), (true, true)),
            ((true, true, true), (true, true, true), (true, true)),
            ((true, true, true), (true, false, false), (true, false)),
            ((true, false, false), (true, true, true), (true, false)),
            ((true, false, true), (false, false, false), (true, false)),
            ((true, false, true), (true, true, true), (true, false)),
        ];
        for (leader, follower, after) in cases {
            // Resends are the machine's, said to every link alike, so the
            // follower's never add to the leader's.
            let ours = |(queued, in_vpn, vpn_queued): (bool, bool, bool)| oneway::Ours {
                queued,
                in_vpn,
                vpn_queued,
                resent: 3,
                ..oneway::Ours::default()
            };
            let mut seen = BTreeMap::from([
                ("eth1".to_owned(), ours(leader)),
                ("eth2".to_owned(), ours(follower)),
            ]);
            let follows = BTreeMap::from([("eth2".to_owned(), "eth1".to_owned())]);
            crossing(&mut seen, &follows);
            let read = (seen["eth1"].queued, seen["eth1"].in_vpn);
            assert_eq!(read, after, "leader {leader:?}, follower {follower:?}");
            assert_eq!(
                seen["eth1"].vpn_queued,
                leader.2 || follower.2,
                "a VPN's queue on either: leader {leader:?}, follower {follower:?}"
            );
            assert_eq!(seen["eth1"].resent, 3, "the follower's resends added");
        }
    }

    /// A wired link's most, from its speed: 50 Mb/s up and 2 Gb/s down.
    const MOST: Held = Held {
        up: 50_000,
        down: 2_000_000,
    };

    fn asked(up: Option<u64>, down: Option<u64>) -> replay::Rates {
        replay::Rates { up, down }
    }

    // On 16 November the eero's own shaper, past ours, was home's narrow
    // point. Its ceiling holds the link from the shaper's first step, before
    // the controller has asked anything, and on a link it doesn't hold, as
    // well as over what it asks.
    #[test]
    fn a_network_s_ceiling_holds_whatever_the_controller_asks() {
        let most = Some(9_500);
        let cases = [
            (asked(None, None), true, asked(most, None), "before a step"),
            (
                asked(Some(11_000), Some(50_000)),
                true,
                asked(most, Some(50_000)),
                "over the ceiling",
            ),
            (
                asked(Some(8_000), None),
                true,
                asked(Some(8_000), None),
                "under it",
            ),
            (
                asked(Some(8_000), None),
                false,
                asked(most, None),
                "a link not held",
            ),
        ];
        for (rates, held, wanted, case) in cases {
            assert_eq!(led(rates, held, most), wanted, "{case}");
        }
        assert_eq!(
            led(asked(Some(8_000), None), false, None),
            asked(None, None),
            "nor one with no ceiling"
        );
    }

    fn reason(up: u64) -> why::Reason {
        why::Reason {
            up: Some(up),
            said: "Upload queued 30 ms.".to_owned(),
        }
    }

    /// The controller's judgement holding the upload at `up`.
    fn holds(up: u64) -> why::Judged {
        why::Judged::Holds(reason(up))
    }

    const NOTHING: why::Judged = why::Judged::Nothing;
    const WAITING: why::Judged = why::Judged::Waiting;

    // The controller asked past the shaper's most, which holds the upload
    // instead: the line names the rate held, and not the controller's
    // reason, which speaks of the rate it asked.
    #[test]
    fn a_spell_held_under_what_was_asked_says_the_rate_held_without_its_reason() {
        let spell = spell(
            "eth0",
            (false, false),
            asked(Some(60_000), None),
            MOST,
            &holds(60_000),
        );
        assert_eq!(spell.held, MOST);
        assert_eq!(
            spell.said.as_deref(),
            Some("eth0: held to 50.0 Mb/s up."),
            "{spell:?}"
        );
        assert!(spell.told, "a clamped spell wasn't told");
    }

    #[test]
    fn a_spell_is_said_once_as_it_begins_and_once_as_it_ends() {
        let holding = holds(9_000);
        let limited = asked(Some(9_000), None);
        let begun = spell("eth0", (false, false), limited, MOST, &holding);
        assert_eq!(
            begun.said.as_deref(),
            Some("eth0: held to 9.0 Mb/s up. Upload queued 30 ms."),
            "{begun:?}"
        );
        assert!(begun.told, "a spell said as it began wasn't told");
        let going = spell("eth0", (true, false), limited, MOST, &holding);
        assert_eq!(going.said, None, "a spell under way was said again");
        assert!(going.told, "a spell under way wasn't kept told");
        let none = asked(None, None);
        let ended = spell("eth0", (true, false), none, MOST, &holding);
        assert_eq!(ended.held, MOST, "no limit holds the whole link");
        assert!(!ended.told, "an ended spell was still told");
        assert_eq!(
            ended.said.as_deref(),
            Some("eth0: neither direction is held now, so the link's whole rate is kept")
        );
        let resting = spell("eth0", (false, false), none, MOST, &holding);
        assert_eq!(resting.said, None, "a link at rest was said to end a spell");
    }

    // A spell the controller began, its recorder then silent: a trial's
    // ceiling holds what the controller's reason no longer speaks of, so
    // the journal says so, and says the controller's reason again once the
    // recorder is heard.
    #[test]
    fn a_spell_says_when_a_ceiling_holds_it_alone_and_when_it_no_longer_does() {
        let ceiling = asked(Some(8_500), None);
        let quiet = spell("eth0", (true, false), ceiling, MOST, &NOTHING);
        assert_eq!(quiet.said, Some(why::ceiling("eth0", 8_500)));
        assert!(quiet.told && quiet.ceiling, "{quiet:?}");
        let still = spell("eth0", (true, true), ceiling, MOST, &NOTHING);
        assert_eq!(still.said, None, "a ceiling still holding was said again");
        assert!(still.ceiling, "a ceiling still holding alone was let go");
        let limited = asked(Some(9_000), None);
        let heard = spell("eth0", (true, true), limited, MOST, &holds(9_000));
        assert_eq!(
            heard.said.as_deref(),
            Some("eth0: held to 9.0 Mb/s up. Upload queued 30 ms."),
            "{heard:?}"
        );
        assert!(heard.told && !heard.ceiling, "{heard:?}");
    }

    // The controller's rate from before a silence holds the link while its
    // reason waits for a second heard since: no trial need be on, so no
    // ceiling is blamed, and the spell is told once the reason comes.
    #[test]
    fn a_rate_waiting_for_its_reason_says_nothing_new() {
        let stale = asked(Some(11_500), None);
        for state in [(false, false), (true, false), (true, true)] {
            let waiting = spell("eth0", state, stale, MOST, &WAITING);
            assert_eq!(waiting.said, None, "from {state:?}: {waiting:?}");
            assert_eq!(waiting.state(), state, "from {state:?}: {waiting:?}");
            assert_eq!(waiting.held.up, 11_500, "the rate from before let go");
        }
        let heard = spell("eth0", (false, false), stale, MOST, &holds(11_500));
        assert_eq!(
            heard.said.as_deref(),
            Some("eth0: held to 11.5 Mb/s up. Upload queued 30 ms.")
        );
    }

    #[test]
    fn a_spell_begun_under_a_silent_recorder_says_the_ceiling_holds_it() {
        let quiet = spell(
            "eth0",
            (false, false),
            asked(Some(8_500), None),
            MOST,
            &NOTHING,
        );
        assert_eq!(quiet.said, Some(why::ceiling("eth0", 8_500)));
    }

    // A ceiling past what the link can carry asks for a rate the shaper's
    // own most holds instead, which the ceiling isn't credited with.
    #[test]
    fn a_ceiling_held_under_what_it_asked_isn_t_credited() {
        let quiet = spell(
            "eth0",
            (false, false),
            asked(Some(60_000), None),
            MOST,
            &NOTHING,
        );
        assert_eq!(quiet.said.as_deref(), Some("eth0: held to 50.0 Mb/s up."));
    }

    #[test]
    fn a_spell_ended_from_a_ceiling_alone_says_so() {
        let ended = spell("eth0", (true, true), asked(None, None), MOST, &NOTHING);
        assert_eq!(
            ended.said.as_deref(),
            Some("eth0: neither direction is held now, so the link's whole rate is kept")
        );
        assert_eq!(ended.state(), (false, false), "{ended:?}");
    }

    // The journal's lines over a run of seconds, each second's state
    // carried into the next as `hold` carries it, so a state dropped
    // between seconds says a line each second.
    #[test]
    fn a_spell_s_lines_over_its_seconds_are_said_once_each() {
        let holding = holds(9_000);
        let limited = asked(Some(9_000), None);
        let ceiling = asked(Some(8_500), None);
        let seconds = [
            (limited, &holding),
            (limited, &WAITING),
            (limited, &holding),
            (ceiling, &NOTHING),
            (ceiling, &NOTHING),
            (ceiling, &WAITING),
            (ceiling, &NOTHING),
            (limited, &holding),
            (limited, &holding),
            (asked(None, None), &holding),
            (asked(None, None), &holding),
        ];
        let mut state = (false, false);
        let said: Vec<Option<String>> = seconds
            .iter()
            .map(|&(rates, why)| {
                let spell = spell("eth0", state, rates, MOST, why);
                state = spell.state();
                spell.said
            })
            .collect();
        let held = Some("eth0: held to 9.0 Mb/s up. Upload queued 30 ms.".to_owned());
        let ended = "eth0: neither direction is held now, so the link's whole rate is kept";
        let ended = Some(ended.to_owned());
        assert_eq!(
            said,
            vec![
                held.clone(),
                None,
                None,
                Some(why::ceiling("eth0", 8_500)),
                None,
                None,
                None,
                held,
                None,
                ended,
                None,
            ]
        );
    }

    #[test]
    fn a_rate_asked_under_the_floor_is_held_at_it() {
        let floor = asked(Some(10), Some(10));
        let spell = spell("eth0", (true, false), floor, MOST, &holds(10));
        assert_eq!(spell.held, FLOOR);
    }

    // Only a device that isn't there reads as having no queues; a tc that
    // failed any other way, as one killed partway answers, reads as
    // nothing known, so its queues are left as they are.
    #[test]
    fn only_a_missing_device_reads_as_no_queues() {
        let json = r#"[{"kind":"cake","handle":"2:"}]"#;
        assert_eq!(
            seen_queues(Ok(json.to_string())).as_deref(),
            Some(json),
            "a read that went"
        );
        let missing = r#"tc -j qdisc show dev shaper4: Cannot find device "shaper4""#;
        assert_eq!(
            seen_queues(Err(missing.to_string())).as_deref(),
            Some(""),
            "a device that isn't there"
        );
        assert_eq!(
            seen_queues(Err("tc -j qdisc show dev wg0: ".to_string())),
            None,
            "a tc killed partway"
        );
        assert_eq!(
            seen_queues(Err("tc: Cannot allocate memory (os error 12)".to_string())),
            None,
            "a tc that couldn't start"
        );
    }

    // A read of the routes that failed says nothing of the links, where an
    // empty one says there are none.
    #[test]
    fn a_failed_read_of_the_routes_names_no_links() {
        let killed = || Err::<String, _>("ip -6 -j route show default: ".to_string());
        assert!(
            routes_seen(Ok("[]".into()), killed()).is_err(),
            "IPv6's failed"
        );
        assert!(
            routes_seen(killed(), Ok("[]".into())).is_err(),
            "IPv4's failed"
        );
        assert_eq!(
            routes_seen(Ok("[]".into()), Ok("[]".into())).map(|u| u.len()),
            Ok(0),
            "none routed"
        );
    }

    // The spacing between rounds crosses a restart in seconds since the
    // epoch, so an instant read back from its seconds is the same instant
    // to within the second the seconds drop (pin round 5's F5 and F13).
    #[test]
    fn a_rounds_time_crosses_a_restart_in_the_epochs_seconds() {
        let then = Instant::now()
            .checked_sub(Duration::from_secs(300))
            .expect("the clock began five minutes ago");
        let at = epoch_of(then).expect("an instant past has its seconds");
        let back = instant_of(at).expect("seconds past give an instant");
        let off = back.max(then).duration_since(back.min(then));
        assert!(
            off <= Duration::from_secs(2),
            "the round's time came back {off:?} off"
        );
        let to_come = Instant::now()
            .checked_add(Duration::from_secs(60))
            .expect("a minute on");
        assert_eq!(epoch_of(to_come), None, "a time to come was given seconds");
        // A restart after the clock stepped back reads seconds to come.
        assert_eq!(
            instant_of(at + 600),
            None,
            "seconds to come were given an instant"
        );
    }

    // What a restart needs is kept whole, and a write that fails takes away
    // what was kept before, rather than leave it to come back as a pin that
    // may have let go (pin round 6's F2 and F4).
    #[test]
    fn the_pin_is_kept_whole_and_a_failed_write_takes_the_stale_one_away() {
        let state = std::env::temp_dir().join(format!("shaper-pin-{}", std::process::id()));
        fs::create_dir_all(&state).expect("the stand-in state folder couldn't be made");
        let mut pinner = Pinner::new(&state);
        pinner.pinning.network("eth1", "aa:bb");
        pinner.keep();
        let kept = fs::read_to_string(state.join("pin")).expect("nothing was kept");
        let back = pin::restored(&kept, &pinner.boot).expect("what was kept didn't read back");
        assert_eq!(
            back.on,
            Some(("eth1".to_owned(), "aa:bb".to_owned())),
            "what was kept isn't the network the pin is on"
        );
        // A folder where the new file goes makes the write fail.
        fs::create_dir_all(state.join("pin.new")).expect("the folder in the way couldn't be made");
        pinner.pinning.network("eth1", "cc:dd");
        pinner.keep();
        let left = state.join("pin").exists();
        fs::remove_dir_all(&state).expect("the stand-in state folder couldn't be removed");
        assert!(!left, "a write that failed left what was kept before");
    }

    // A restart's sweep waits for the routes, every VPN there and every
    // link the routes name, since any not taken up yet would go with it.
    #[test]
    fn a_sweep_waits_for_every_link_the_routes_name() {
        let order = ["eth0".to_string(), "wlan0".to_string()];
        let shaped = |names: &[&str]| -> BTreeMap<String, u8> {
            names.iter().map(|n| ((*n).to_string(), 0)).collect()
        };
        let both = shaped(&["eth0", "wlan0"]);
        assert!(
            may_sweep(true, Some(&order[..]), &both),
            "every link taken up"
        );
        assert!(
            !may_sweep(true, Some(&order[..]), &shaped(&["eth0"])),
            "a link whose read failed"
        );
        assert!(
            !may_sweep(false, Some(&order[..]), &both),
            "a VPN not taken up"
        );
        assert!(!may_sweep(true, None, &both), "the routes unread");
        assert!(
            !may_sweep(true, Some(&[][..]), &shaped(&[])),
            "no link taken up"
        );
    }

    // The house's ceiling and the network's own both hold a link, the
    // lower of the two, and either alone holds it by itself.
    #[test]
    fn the_lower_of_two_ceilings_holds() {
        assert_eq!(lower(Some(9_500), Some(4_500)), Some(4_500));
        assert_eq!(lower(Some(4_000), Some(4_500)), Some(4_000));
        assert_eq!(lower(None, Some(4_500)), Some(4_500), "the house's alone");
        assert_eq!(lower(Some(9_500), None), Some(9_500), "the network's alone");
        assert_eq!(lower(None, None), None);
    }

    // The trace says what the house heard and held only on a link of the
    // network it holds.
    #[test]
    fn a_link_s_trace_says_its_own_network_s_house_alone() {
        let house = HouseHolds::from([("00:00:5e:00:53:01".to_string(), (2_500, 4_500))]);
        let mut line = trace::Line::default();
        house_traced(&mut line, Some("00:00:5e:00:53:01"), &house);
        assert_eq!(
            (line.house_sent, line.house_most, line.ceiling),
            (Some(2_500), Some(4_500), Some(4_500)),
            "its network's house, whose hold is the second's ceiling"
        );
        let mut line = trace::Line {
            ceiling: Some(5_250),
            ..trace::Line::default()
        };
        house_traced(&mut line, Some("00:00:5e:00:53:01"), &house);
        assert_eq!(
            line.ceiling,
            Some(4_500),
            "the second's hold, under the steady ceiling the controller had"
        );
        let mut line = trace::Line {
            ceiling: Some(4_000),
            ..trace::Line::default()
        };
        house_traced(&mut line, Some("00:00:5e:00:53:01"), &house);
        assert_eq!(line.ceiling, Some(4_000), "a trial's lower ceiling kept");
        let mut line = trace::Line::default();
        house_traced(&mut line, Some("00:00:5e:00:53:02"), &house);
        assert_eq!(
            (line.house_sent, line.house_most, line.ceiling),
            (None, None, None),
            "another network's"
        );
        house_traced(&mut line, None, &house);
        assert_eq!(line.house_sent, None, "a link not shaped");
    }

    // A network's house holds its links only while the other machine has
    // said lately what it sends, and only from the address named.
    #[test]
    fn a_house_holds_only_while_its_other_machine_says_what_it_sends() {
        let mac: std::net::IpAddr = "192.0.2.7".parse().expect("an address");
        let houses = networks::Houses::from([(
            "00:00:5e:00:53:01".to_string(),
            networks::House {
                most: 7_000,
                from: mac,
            },
        )]);
        let at = Instant::now();
        assert!(house_holds(&houses, None, at).is_empty(), "none heard");
        let ear = house::Ear::default();
        assert!(
            house_holds(&houses, Some(&ear), at).is_empty(),
            "nothing said"
        );
        ear.hear(mac, house::Report { bytes: 0, ms: 0 }, at);
        assert!(
            house_holds(&houses, Some(&ear), at).is_empty(),
            "one report says no rate"
        );
        ear.hear(
            mac,
            house::Report {
                bytes: 312_500,
                ms: 1_000,
            },
            at,
        );
        let held = house_holds(&houses, Some(&ear), at);
        assert_eq!(
            held.get("00:00:5e:00:53:01"),
            Some(&(2_500, 4_500)),
            "the Mac sending 2.5 Mb/s leaves 4.5 of 7"
        );
        let said = house_said(&held);
        assert!(said.contains("under 4500 kbit/s"), "{said}");
        let other: std::net::IpAddr = "192.0.2.8".parse().expect("an address");
        ear.hear(other, house::Report { bytes: 0, ms: 0 }, at);
        let report = house::Report {
            bytes: 1_000_000,
            ms: 1_000,
        };
        ear.hear(other, report, at);
        assert_eq!(
            house_holds(&houses, Some(&ear), at),
            held,
            "another address heard beside it holds nothing"
        );
        ear.hear(
            mac,
            house::Report {
                bytes: 1_312_500,
                ms: 2_000,
            },
            at,
        );
        assert_eq!(
            house_holds(&houses, Some(&ear), at).get("00:00:5e:00:53:01"),
            Some(&(8_000, 4_000)),
            "the Mac sending past the ceiling holds this one at the least, where a call fits"
        );
        let later = at + house::FRESH + Duration::from_secs(1);
        let quiet = house_holds(&houses, Some(&ear), later);
        assert!(quiet.is_empty(), "a report past its time holds nothing");
        assert!(
            house_said(&quiet).contains("holds nothing"),
            "{}",
            house_said(&quiet)
        );
    }
}
