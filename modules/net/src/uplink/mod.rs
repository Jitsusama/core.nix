//! Who fills this machine's uplink: what each TCP socket sent between two
//! samples, by purpose, by whose it is, by process and by destination,
//! against what the uplink's interface sent in all.
//!
//! A sample is the kernel's socket list (diag.rs) and each interface's
//! byte counter; a report is two samples' difference. What the sockets
//! don't account for, UDP such as QUIC and a VPN's own tunnel, headers and
//! the tunnel's wrapping, is said as unattributed rather than spread.

mod diag;
mod owners;
mod purpose;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub use diag::Socket;
pub use owners::{Owner, Owners};
pub use purpose::{Purpose, Side};

/// The schema every report is written in.
pub const SCHEMA: &str = "net.uplink/1";

/// The sockets and interface counters at one instant.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Seconds since the epoch, to the millisecond.
    pub at: f64,
    pub sockets: Vec<Socket>,
    /// Bytes each interface has sent, by name.
    pub sent: BTreeMap<String, u64>,
    /// The interface the default route leaves by, if there is one.
    pub uplink: Option<String>,
}

/// Takes a sample now.
pub fn sample() -> io::Result<Sample> {
    Ok(Sample {
        at: now(),
        sockets: diag::sockets()?,
        sent: interfaces_sent(),
        uplink: default_route(&fs::read_to_string("/proc/net/route").unwrap_or_default()),
    })
}

/// Who filled the uplink between two samples.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub from: f64,
    pub to: f64,
    pub uplink: Option<String>,
    /// All the uplink's interface sent, in kbit/s.
    pub uplink_kbit: f64,
    /// What TCP sockets to peers past this network sent, resends included.
    pub sockets_kbit: f64,
    /// The uplink's sending less the sockets': UDP, headers, a VPN's wrapping.
    pub unattributed_kbit: f64,
    /// Shares of the sockets' sending, each to the nearest 5%.
    pub joel_pct: u32,
    pub agents_pct: u32,
    pub purposes: Vec<Share<Purpose>>,
    pub sides: Vec<Share<Side>>,
    pub processes: Vec<Process>,
    pub destinations: Vec<Destination>,
    /// Every interface's sending, in kbit/s, tunnels included.
    pub interfaces: BTreeMap<String, f64>,
}

/// One group's sending and its share of the sockets'.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Share<T> {
    pub what: T,
    pub kbit: f64,
    pub pct: f64,
}

/// One process's sending.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Process {
    pub pid: Option<u32>,
    pub name: String,
    pub purpose: Purpose,
    pub side: Side,
    pub kbit: f64,
    /// What share of it was resent.
    pub resent_pct: f64,
    pub sockets: usize,
}

/// What went to one peer address.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Destination {
    pub peer: IpAddr,
    pub kbit: f64,
    pub resent_pct: f64,
}

/// What each socket sent between the samples: the counter's growth for a
/// socket in both, all of it for one opened between them. A socket closed
/// between them sent what nobody saw, which unattributed holds.
fn sent_between<'a>(earlier: &Sample, later: &'a Sample) -> Vec<(&'a Socket, u64, u64)> {
    let before: HashMap<u64, &Socket> = earlier
        .sockets
        .iter()
        .map(|socket| (socket.cookie, socket))
        .collect();
    later
        .sockets
        .iter()
        .filter_map(|socket| {
            let (sent, resent) = before.get(&socket.cookie).map_or(
                (socket.bytes_sent, socket.bytes_retrans),
                |was| {
                    (
                        socket.bytes_sent.saturating_sub(was.bytes_sent),
                        socket.bytes_retrans.saturating_sub(was.bytes_retrans),
                    )
                },
            );
            (sent > 0).then_some((socket, sent, resent))
        })
        .collect()
}

/// The report of who sent what between two samples, with each socket's
/// owner from `owners`.
pub fn report(earlier: &Sample, later: &Sample, owners: &mut Owners) -> Report {
    let moved = sent_between(earlier, later);
    let inodes: Vec<u32> = moved.iter().map(|(socket, ..)| socket.inode).collect();
    let known = owners.of(&inodes);
    report_with(earlier, later, &moved, known)
}

/// What one process sent in the window, while it's being added up.
#[derive(Default)]
struct Tally {
    purpose: Option<Purpose>,
    side: Option<Side>,
    sent: u64,
    resent: u64,
    sockets: usize,
}

/// Bytes as a float, for rates and shares.
#[expect(
    clippy::cast_precision_loss,
    reason = "a socket's lifetime bytes stay far below 2^52"
)]
fn float(bytes: u64) -> f64 {
    bytes as f64
}

fn report_with(
    earlier: &Sample,
    later: &Sample,
    moved: &[(&Socket, u64, u64)],
    known: &HashMap<u32, Owner>,
) -> Report {
    let seconds = (later.at - earlier.at).max(0.001);
    let kbit = |bytes: u64| float(bytes) * 8.0 / seconds / 1000.0;
    let mut purposes: BTreeMap<Purpose, u64> = BTreeMap::new();
    let mut sides: BTreeMap<Side, u64> = BTreeMap::new();
    let mut processes: BTreeMap<(Option<u32>, String), Tally> = BTreeMap::new();
    let mut destinations: BTreeMap<IpAddr, (u64, u64)> = BTreeMap::new();
    let mut total = 0;
    for (socket, sent, resent) in moved {
        let owner = known.get(&socket.inode);
        let Some(purpose) = purpose::purpose(socket.peer, owner) else {
            continue;
        };
        let side = purpose::side(owner);
        total += sent;
        *purposes.entry(purpose).or_default() += sent;
        *sides.entry(side).or_default() += sent;
        let name = owner.map_or_else(|| format!("uid {}", socket.uid), |held| held.name.clone());
        let row = processes
            .entry((owner.map(|held| held.pid), name))
            .or_default();
        row.purpose = Some(purpose);
        row.side = Some(side);
        row.sent += sent;
        row.resent += resent;
        row.sockets += 1;
        let peer = destinations.entry(socket.peer).or_default();
        peer.0 += sent;
        peer.1 += resent;
    }
    let share = |bytes: u64| {
        if total == 0 {
            0.0
        } else {
            float(bytes) * 100.0 / float(total)
        }
    };
    let interfaces: BTreeMap<String, f64> = later
        .sent
        .iter()
        .filter_map(|(name, bytes)| {
            let was = earlier.sent.get(name)?;
            Some((name.clone(), kbit(bytes.saturating_sub(*was))))
        })
        .collect();
    let uplink_kbit = later
        .uplink
        .as_ref()
        .and_then(|name| interfaces.get(name))
        .copied()
        .unwrap_or(0.0);
    let sockets_kbit = kbit(total);
    let mut purposes: Vec<Share<Purpose>> = purposes
        .into_iter()
        .map(|(what, bytes)| Share {
            what,
            kbit: kbit(bytes),
            pct: share(bytes),
        })
        .collect();
    purposes.sort_by(|a, b| b.kbit.total_cmp(&a.kbit));
    let side_bytes = |which: Side| sides.get(&which).copied().unwrap_or(0);
    let mut processes: Vec<Process> = processes
        .into_iter()
        .map(|((pid, name), tally)| Process {
            pid,
            name,
            purpose: tally.purpose.unwrap_or(Purpose::Unknown),
            side: tally.side.unwrap_or(Side::Unknown),
            kbit: kbit(tally.sent),
            resent_pct: resent(tally.sent, tally.resent),
            sockets: tally.sockets,
        })
        .collect();
    processes.sort_by(|a, b| b.kbit.total_cmp(&a.kbit));
    let mut destinations: Vec<Destination> = destinations
        .into_iter()
        .map(|(peer, (sent, again))| Destination {
            peer,
            kbit: kbit(sent),
            resent_pct: resent(sent, again),
        })
        .collect();
    destinations.sort_by(|a, b| b.kbit.total_cmp(&a.kbit));
    Report {
        schema: SCHEMA,
        from: earlier.at,
        to: later.at,
        uplink: later.uplink.clone(),
        uplink_kbit,
        sockets_kbit,
        unattributed_kbit: (uplink_kbit - sockets_kbit).max(0.0),
        joel_pct: nearest_five(share(side_bytes(Side::Joel))),
        agents_pct: nearest_five(share(side_bytes(Side::Agents))),
        sides: sides
            .iter()
            .map(|(what, bytes)| Share {
                what: *what,
                kbit: kbit(*bytes),
                pct: share(*bytes),
            })
            .collect(),
        purposes,
        processes,
        destinations,
        interfaces,
    }
}

/// What share of what was sent was sent again.
fn resent(sent: u64, again: u64) -> f64 {
    if sent == 0 {
        0.0
    } else {
        float(again) * 100.0 / float(sent)
    }
}

/// A share to the nearest 5%, as a glance reads it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a share is clamped to 0 to 100 first, so the cast keeps it whole"
)]
fn nearest_five(pct: f64) -> u32 {
    ((pct / 5.0).round() * 5.0).clamp(0.0, 100.0) as u32
}

/// Each interface's sent bytes, from sysfs; loopback carries nothing out.
fn interfaces_sent() -> BTreeMap<String, u64> {
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "lo" {
                return None;
            }
            let bytes = fs::read_to_string(entry.path().join("statistics/tx_bytes")).ok()?;
            Some((name, bytes.trim().parse().ok()?))
        })
        .collect()
}

/// The interface of the main table's IPv4 default route, from
/// /proc/net/route, the one with the lowest metric when there are several.
fn default_route(table: &str) -> Option<String> {
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (name, destination, mask, metric) = (
                fields.first()?,
                fields.get(1)?,
                fields.get(7)?,
                fields.get(6)?,
            );
            (*destination == "00000000" && *mask == "00000000").then(|| {
                (
                    metric.parse::<u64>().unwrap_or(u64::MAX),
                    (*name).to_owned(),
                )
            })
        })
        .min()
        .map(|(_, name)| name)
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket(cookie: u64, peer: &str, inode: u32, sent: u64, resent: u64) -> Socket {
        Socket {
            cookie,
            local: "192.168.4.41".parse().unwrap(),
            peer: peer.parse().unwrap(),
            peer_port: 443,
            uid: 1000,
            inode,
            bytes_sent: sent,
            bytes_retrans: resent,
        }
    }

    fn sampled(at: f64, sockets: Vec<Socket>, uplink_sent: u64) -> Sample {
        Sample {
            at,
            sockets,
            sent: BTreeMap::from([("wlan0".to_owned(), uplink_sent)]),
            uplink: Some("wlan0".into()),
        }
    }

    fn owner(pid: u32, name: &str, command: &str, cgroup: &str) -> Owner {
        Owner {
            pid,
            name: name.into(),
            command: command.into(),
            cgroup: cgroup.into(),
        }
    }

    #[test]
    fn a_window_counts_growth_new_sockets_whole_and_leaves_the_rest_unattributed() {
        let earlier = sampled(
            100.0,
            vec![
                socket(1, "203.0.113.7", 11, 1_000_000, 0),
                socket(2, "198.51.100.3", 12, 50_000, 0),
            ],
            10_000_000,
        );
        let later = sampled(
            110.0,
            vec![
                // A session's requests: 1 MB more, 100 kB of it resent.
                socket(1, "203.0.113.7", 11, 2_000_000, 100_000),
                // Joel's browser, idle.
                socket(2, "198.51.100.3", 12, 50_000, 0),
                // A push opened in the window: all it sent is new.
                socket(3, "198.51.100.9", 13, 250_000, 0),
                // The other machine on the desk is no part of the uplink.
                socket(4, "192.168.4.49", 14, 9_000_000, 0),
            ],
            // 1.5 MB went out: 1.25 MB of sockets and 250 kB unseen.
            11_500_000,
        );
        let known = HashMap::from([
            (11, owner(5, "pi", "pi", "/u/jobs.slice/a.scope")),
            (
                12,
                owner(6, "chrome", "chrome", "/u/app.slice/app-chrome.scope"),
            ),
            (
                13,
                owner(7, "podman", "podman push r/i:t", "/u/jobs.slice/b.scope"),
            ),
            (14, owner(8, "ssh", "ssh other", "/u/jobs.slice/c.scope")),
        ]);
        let moved = sent_between(&earlier, &later);
        let report = report_with(&earlier, &later, &moved, &known);

        assert_eq!(report.uplink_kbit, 1200.0);
        assert_eq!(report.sockets_kbit, 1000.0);
        assert_eq!(report.unattributed_kbit, 200.0);
        assert_eq!(report.purposes[0].what, Purpose::Sessions);
        assert_eq!(report.purposes[0].kbit, 800.0);
        assert_eq!(report.purposes[1].what, Purpose::Images);
        assert_eq!(report.purposes[1].pct, 20.0);
        assert_eq!(report.agents_pct, 100);
        assert_eq!(report.joel_pct, 0);
        assert_eq!(report.processes[0].resent_pct, 10.0);
        assert!(
            report
                .destinations
                .iter()
                .all(|to| !to.peer.to_string().starts_with("192.168"))
        );
    }

    #[test]
    fn shares_round_to_the_nearest_five() {
        assert_eq!(nearest_five(47.4), 45);
        assert_eq!(nearest_five(47.6), 50);
        assert_eq!(nearest_five(2.4), 0);
        assert_eq!(nearest_five(100.0), 100);
    }

    #[test]
    fn the_default_route_is_the_lowest_metric_default() {
        let table = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\n\
            wlan0\t00000000\t0104A8C0\t0003\t0\t0\t600\t00000000\n\
            eth0\t00000000\t0104A8C0\t0003\t0\t0\t100\t00000000\n\
            wlan0\t0004A8C0\t00000000\t0001\t0\t0\t600\t00FCFFFF\n";
        assert_eq!(default_route(table).as_deref(), Some("eth0"));
        assert_eq!(default_route("Iface\n"), None);
    }
}
