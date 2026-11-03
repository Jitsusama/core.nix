//! The shaper's next signal, recorded before anything acts on it: each
//! direction's own delay, from ICMP timestamps to a pool of reflectors,
//! the gateway's round trip, which crosses the air and not the modem, and
//! what the link carried each way. A week of it is what a new controller is
//! replayed over before it shapes anything, and `shaper record` writes it
//! while the shaper stays off.
//!
//! The pool is chosen on each new network from public resolvers that
//! answer timestamps, one per operator first, since one operator's outage
//! or a queue near its servers would otherwise speak for all of them. With
//! fewer than three operators answering, the samples still go down, marked
//! so, since a controller has nothing to vote with there.
//!
//! Probes go every 0.3 s while the link carries anything, and every 5 s
//! once it has carried under 1 Mbit/s for a minute, so an idle machine
//! wakes for it rarely while the baselines still follow the path.

use crate::icmp::{self, Socket};
use crate::stamp;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Public resolvers, by operator, that a timestamp may be asked of. On
/// 3 November four of eleven tried answered from a home line.
pub const CANDIDATES: &[(&str, [u8; 4])] = &[
    ("Quad9", [9, 9, 9, 9]),
    ("Quad9", [149, 112, 112, 112]),
    ("Quad9", [9, 9, 9, 11]),
    ("AdGuard", [94, 140, 14, 14]),
    ("AdGuard", [94, 140, 15, 15]),
    ("CleanBrowsing", [185, 228, 168, 9]),
    ("CleanBrowsing", [185, 228, 169, 9]),
    ("OpenDNS", [208, 67, 222, 222]),
    ("OpenDNS", [208, 67, 220, 220]),
    ("Level 3", [4, 2, 2, 1]),
    ("Level 3", [4, 2, 2, 2]),
    ("Control D", [76, 76, 2, 0]),
    ("Control D", [76, 76, 10, 0]),
    ("Verisign", [64, 6, 64, 6]),
    ("Verisign", [64, 6, 65, 6]),
    ("SafeDNS", [195, 46, 39, 39]),
    ("DNS.Watch", [84, 200, 69, 80]),
    ("Hurricane Electric", [74, 82, 42, 42]),
    ("Yandex", [77, 88, 8, 8]),
    ("UltraDNS", [156, 154, 70, 1]),
    ("Comodo", [8, 26, 56, 26]),
];

/// The reflectors to ask: `SHAPER_REFLECTORS`, comma-separated, when a
/// network of its own names them, a test's among them, and otherwise the
/// public resolvers.
fn candidates() -> Vec<(&'static str, Ipv4Addr)> {
    match std::env::var("SHAPER_REFLECTORS") {
        Ok(named) => named
            .split(',')
            .filter_map(|a| a.trim().parse().ok())
            .map(|a| ("named", a))
            .collect(),
        Err(_) => CANDIDATES
            .iter()
            .map(|&(operator, octets)| (operator, Ipv4Addr::from(octets)))
            .collect(),
    }
}

/// At most this many reflectors are probed, in turn.
const POOL: usize = 6;
/// Fewer operators than this answering leaves nothing to vote with.
pub const ENOUGH: usize = 3;
const BUSY_EVERY: Duration = Duration::from_millis(300);
const IDLE_EVERY: Duration = Duration::from_secs(5);
/// A link carrying under this many bits a second for a minute is idle.
const IDLE_BITS: u64 = 1_000_000;
const IDLE_FOR: Duration = Duration::from_secs(60);
/// A probe not answered within this is lost.
const LOST_AFTER: Duration = Duration::from_secs(1);
/// How many days' files are kept: a week to replay, and the day under way.
const KEEP_DAYS: usize = 8;
/// A pool member lost this many times running is replaced.
const MISSES: u32 = 3;
const REPOOL: Duration = Duration::from_secs(3_600);
const EMPTY_REPOOL: Duration = Duration::from_secs(300);

/// The reflectors to probe from those that answered: one per operator in
/// the order they answered, then more of the same operators, up to the
/// pool's size.
pub fn pool(answered: &[(&'static str, Ipv4Addr)]) -> Vec<(&'static str, Ipv4Addr)> {
    let mut chosen: Vec<(&'static str, Ipv4Addr)> = Vec::new();
    let mut seen = BTreeSet::new();
    for &(operator, address) in answered {
        if chosen.len() < POOL && seen.insert(operator) {
            chosen.push((operator, address));
        }
    }
    for &(operator, address) in answered {
        if chosen.len() < POOL && !chosen.contains(&(operator, address)) {
            chosen.push((operator, address));
        }
    }
    chosen
}

/// How many operators a pool reaches.
pub fn operators(pool: &[(&'static str, Ipv4Addr)]) -> usize {
    pool.iter().map(|(o, _)| *o).collect::<BTreeSet<_>>().len()
}

/// How often to probe, given the bits the link carried over the last
/// `over`: often while it carries anything, rarely once it's been idle a
/// minute.
pub fn pace(bits: u64, over: Duration) -> Duration {
    if over >= IDLE_FOR && bits < IDLE_BITS * over.as_secs() {
        IDLE_EVERY
    } else {
        BUSY_EVERY
    }
}

/// The day's file for a moment, named by its UTC date.
pub fn day_file(folder: &Path, at: SystemTime) -> PathBuf {
    let days = at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() / 86_400;
    let (y, m, d) = civil(i64::try_from(days).unwrap_or(0));
    folder.join(format!("{y:04}-{m:02}-{d:02}.jsonl"))
}

/// The date a count of days since 1970 falls on (Howard Hinnant's
/// days-to-civil).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The day files past the newest `KEEP_DAYS`, which go.
pub fn expired(names: &[String]) -> Vec<String> {
    let mut days: Vec<&String> = names
        .iter()
        .filter(|n| {
            Path::new(n)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("jsonl"))
        })
        .collect();
    days.sort();
    let keep = days.len().saturating_sub(KEEP_DAYS);
    days[..keep].iter().map(|n| (*n).clone()).collect()
}

/// What the link carried in total, received and sent, in bytes.
fn carried(link: &str) -> (u64, u64) {
    let read = |which: &str| {
        fs::read_to_string(format!("/sys/class/net/{link}/statistics/{which}_bytes"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };
    (read("rx"), read("tx"))
}

fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(0)
}

fn ms(of: Duration) -> f64 {
    of.as_secs_f64() * 1_000.0
}

/// Where the samples go, one JSON line each, a file a day.
struct Journal {
    folder: PathBuf,
    file: Option<(PathBuf, fs::File)>,
}

impl Journal {
    fn write(&mut self, line: &Value) {
        let path = day_file(&self.folder, SystemTime::now());
        if self.file.as_ref().is_none_or(|(p, _)| *p != path) {
            self.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok()
                .map(|f| (path, f));
            self.tidy();
        }
        if let Some((_, file)) = self.file.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }

    fn tidy(&self) {
        let names: Vec<String> = fs::read_dir(&self.folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        for name in expired(&names) {
            let _ = fs::remove_file(self.folder.join(name));
        }
    }
}

/// A probe on its way, by number.
struct Sent {
    to: Option<(&'static str, Ipv4Addr)>,
    at: Instant,
}

/// The reflectors that answer timestamps from this link, two tries each.
fn answering(socket: &Socket, id: u16) -> Vec<(&'static str, Ipv4Addr)> {
    let mut asked = BTreeMap::new();
    let mut seq: u16 = 0;
    for _ in 0..2 {
        for &(operator, address) in &candidates() {
            seq = seq.wrapping_add(1);
            let request = stamp::request(id, seq, stamp::since_midnight(SystemTime::now()));
            if socket.send(address, &request).is_ok() {
                asked.insert(seq, (operator, address));
            }
        }
    }
    let mut heard = Vec::new();
    let mut buffer = [0_u8; 1_500];
    let until = Instant::now() + Duration::from_millis(1_500);
    while Instant::now() < until {
        let Ok(Some(length)) = socket.receive(&mut buffer) else {
            continue;
        };
        let packet = &buffer[..length];
        let Some(reply) = stamp::reply(packet, id) else {
            continue;
        };
        let finished = stamp::since_midnight(SystemTime::now());
        if let Some(&reflector) = asked.get(&reply.seq)
            && stamp::one_way(&reply, finished).is_some()
            && !heard.contains(&reflector)
        {
            heard.push(reflector);
        }
    }
    heard
}

/// The pool's line in the journal.
fn pool_line(reflectors: &[(&'static str, Ipv4Addr)]) -> Value {
    json!({
        "at": now_ms(),
        "pool": reflectors.iter().map(|(o, a)| json!([o, a.to_string()])).collect::<Vec<_>>(),
        "operators": operators(reflectors),
        "enough": operators(reflectors) >= ENOUGH,
    })
}

/// One link's recording under way.
struct Recorder<'a> {
    link: &'a str,
    gateway: Option<Ipv4Addr>,
    id: u16,
    socket: Socket,
    journal: Journal,
    reflectors: Vec<(&'static str, Ipv4Addr)>,
    pooled: Instant,
    misses: BTreeMap<Ipv4Addr, u32>,
    outstanding: BTreeMap<u16, Sent>,
    seq: u16,
    turn: usize,
    next: Instant,
    window: ((u64, u64), Instant),
}

impl Recorder<'_> {
    /// Send the next probes when they're due: a timestamp to the next
    /// reflector in turn, and an echo to the gateway.
    fn probe(&mut self, now: Instant) {
        if now < self.next {
            return;
        }
        let (counted, since) = self.window;
        let total = carried(self.link);
        let bits = (total.0 + total.1).saturating_sub(counted.0 + counted.1) * 8;
        let over = now.duration_since(since);
        self.next = now + pace(bits, over);
        if over >= IDLE_FOR {
            self.window = (total, now);
        }
        if let Some(&reflector) = self
            .reflectors
            .get(self.turn % self.reflectors.len().max(1))
        {
            self.seq = self.seq.wrapping_add(1);
            let request =
                stamp::request(self.id, self.seq, stamp::since_midnight(SystemTime::now()));
            if self.socket.send(reflector.1, &request).is_ok() {
                let sent = Sent {
                    to: Some(reflector),
                    at: now,
                };
                self.outstanding.insert(self.seq, sent);
            }
            self.turn += 1;
        }
        if let Some(gateway) = self.gateway {
            self.seq = self.seq.wrapping_add(1);
            if self
                .socket
                .send(gateway, &icmp::echo_request(self.id, self.seq))
                .is_ok()
            {
                self.outstanding
                    .insert(self.seq, Sent { to: None, at: now });
            }
        }
    }

    /// Hear one packet, if one comes within the socket's wait, and write
    /// down what it answers.
    fn hear(&mut self, buffer: &mut [u8]) {
        let Ok(Some(length)) = self.socket.receive(buffer) else {
            return;
        };
        let packet = &buffer[..length];
        let arrived = Instant::now();
        let (rx, tx) = carried(self.link);
        if let Some(reply) = stamp::reply(packet, self.id)
            && let Some(sent) = self.outstanding.remove(&reply.seq)
            && let Some((operator, address)) = sent.to
        {
            self.misses.remove(&address);
            let finished = stamp::since_midnight(SystemTime::now());
            let rtt = ms(arrived.duration_since(sent.at));
            let mut line = json!({
                "at": now_ms(), "reflector": address.to_string(), "operator": operator,
                "rtt": rtt, "rx": rx, "tx": tx,
            });
            match stamp::one_way(&reply, finished) {
                Some(delays) => {
                    line["out"] = json!(delays.out_ms);
                    line["back"] = json!(delays.back_ms);
                }
                None => line["clock"] = json!("not standard"),
            }
            self.journal.write(&line);
        } else if let Some(echoed) = icmp::echo_reply(packet, self.id)
            && icmp::source(packet) == self.gateway
            && let Some(sent) = self.outstanding.remove(&echoed)
        {
            self.journal.write(&json!({
                "at": now_ms(), "gateway": ms(arrived.duration_since(sent.at)),
                "rx": rx, "tx": tx,
            }));
        }
    }

    /// Write down each probe that hasn't come back in time, as lost, and
    /// count it against its reflector.
    fn expire(&mut self) {
        let late: Vec<u16> = self
            .outstanding
            .iter()
            .filter(|(_, s)| s.at.elapsed() > LOST_AFTER)
            .map(|(n, _)| *n)
            .collect();
        for number in late {
            let Some(sent) = self.outstanding.remove(&number) else {
                continue;
            };
            let line = match sent.to {
                Some((operator, address)) => {
                    *self.misses.entry(address).or_default() += 1;
                    json!({"at": now_ms(), "reflector": address.to_string(),
                           "operator": operator, "lost": true})
                }
                None => json!({"at": now_ms(), "gateway": Value::Null, "lost": true}),
            };
            self.journal.write(&line);
        }
    }

    /// Choose the pool again when one of it keeps missing or an hour has
    /// passed; an empty pool asks again only every five minutes, so a
    /// network that answers no timestamps isn't flooded with them.
    fn repool(&mut self) {
        let failing = self.misses.values().any(|m| *m >= MISSES);
        let due = if self.reflectors.is_empty() {
            EMPTY_REPOOL
        } else {
            REPOOL
        };
        if failing || self.pooled.elapsed() > due {
            self.reflectors = pool(&answering(&self.socket, self.id));
            self.pooled = Instant::now();
            self.misses.clear();
            self.journal.write(&pool_line(&self.reflectors));
        }
    }
}

/// Record a link's signal while `still` says it's the uplink it was, then
/// return so the caller finds the next, with a pool of its own.
pub fn record(
    link: &str,
    network: &str,
    gateway: Option<Ipv4Addr>,
    folder: &Path,
    still: &dyn Fn() -> bool,
) {
    let id = u16::try_from(std::process::id() & 0xffff).unwrap_or(1);
    let socket = match Socket::bound(link, Duration::from_millis(50)) {
        Ok(socket) => socket,
        Err(error) => {
            eprintln!("couldn't open a raw socket on {link}: {error}");
            return;
        }
    };
    let _ = fs::create_dir_all(folder);
    let reflectors = pool(&answering(&socket, id));
    eprintln!(
        "recording {link} on {network}: {} reflectors from {} operators",
        reflectors.len(),
        operators(&reflectors)
    );
    let mut journal = Journal {
        folder: folder.to_path_buf(),
        file: None,
    };
    let mut first = pool_line(&reflectors);
    first["link"] = json!(link);
    first["network"] = json!(network);
    journal.write(&first);
    let mut recorder = Recorder {
        link,
        gateway,
        id,
        socket,
        journal,
        reflectors,
        pooled: Instant::now(),
        misses: BTreeMap::new(),
        outstanding: BTreeMap::new(),
        seq: 0,
        turn: 0,
        next: Instant::now(),
        window: (carried(link), Instant::now()),
    };
    let mut buffer = [0_u8; 1_500];
    let mut checked = Instant::now();
    loop {
        let now = Instant::now();
        if now.duration_since(checked) > Duration::from_secs(10) {
            if !still() {
                return;
            }
            checked = now;
        }
        recorder.probe(now);
        recorder.hear(&mut buffer);
        recorder.expire();
        recorder.repool();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(n: u8) -> Ipv4Addr {
        Ipv4Addr::new(10, 0, 0, n)
    }

    #[test]
    fn a_pool_takes_one_of_each_operator_before_seconds() {
        let answered = [
            ("Quad9", at(1)),
            ("Quad9", at(2)),
            ("AdGuard", at(3)),
            ("CleanBrowsing", at(4)),
        ];
        let chosen = pool(&answered);
        assert_eq!(
            chosen,
            vec![
                ("Quad9", at(1)),
                ("AdGuard", at(3)),
                ("CleanBrowsing", at(4)),
                ("Quad9", at(2))
            ]
        );
        assert_eq!(operators(&chosen), 3);
    }

    #[test]
    fn a_pool_stops_at_six() {
        let answered: Vec<(&'static str, Ipv4Addr)> = (1..=9).map(|n| ("Quad9", at(n))).collect();
        assert_eq!(pool(&answered).len(), 6);
        assert_eq!(
            operators(&pool(&answered)),
            1,
            "one operator is still one vote"
        );
    }

    #[test]
    fn a_link_idle_a_minute_is_probed_rarely() {
        let minute = Duration::from_secs(60);
        assert_eq!(
            pace(59_000_000, minute),
            Duration::from_secs(5),
            "under 1 Mbit/s"
        );
        assert_eq!(
            pace(61_000_000, minute),
            Duration::from_millis(300),
            "a flow"
        );
        assert_eq!(
            pace(0, Duration::from_secs(20)),
            Duration::from_millis(300),
            "not yet a minute"
        );
    }

    #[test]
    fn each_day_has_its_own_file() {
        // 16:00 UTC on 3 November 2026, and midnight on 29 February 2000.
        let at = UNIX_EPOCH + Duration::from_hours(498_257);
        assert_eq!(
            day_file(Path::new("/r"), at),
            PathBuf::from("/r/2026-11-03.jsonl")
        );
        let leap = UNIX_EPOCH + Duration::from_hours(264_384);
        assert_eq!(
            day_file(Path::new("/r"), leap),
            PathBuf::from("/r/2000-02-29.jsonl")
        );
    }

    #[test]
    fn only_the_last_eight_days_are_kept() {
        let names: Vec<String> = (1..=10)
            .map(|d| format!("2026-11-{d:02}.jsonl"))
            .chain(["notes.txt".to_string()])
            .collect();
        assert_eq!(
            expired(&names),
            vec!["2026-11-01.jsonl", "2026-11-02.jsonl"]
        );
    }
}
