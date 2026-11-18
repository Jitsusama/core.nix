//! The shaper's next signal, recorded before anything acts on it: each
//! direction's own delay, from ICMP timestamps to a pool of reflectors,
//! the gateway's round trip, which crosses the air and not the modem, and
//! what the link carried each way. A week of it is what a new controller is
//! replayed over before it shapes anything (src/replay.rs), and `shaper
//! record` writes it while the shaper stays off.
//!
//! The pool is chosen on each new network, and again when it's due or
//! failing, as src/reflectors.rs says.
//!
//! Probes go every 0.3 s while the link carries anything, and every 5 s
//! once it has carried under 1 Mbit/s for a minute, so an idle machine
//! wakes for it rarely while the baselines still follow the path.

use crate::icmp::{self, Socket};
use crate::reflectors::{Asking, ENOUGH, Pool, candidates, operators};
use crate::stamp;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BUSY_EVERY: Duration = Duration::from_millis(300);
const IDLE_EVERY: Duration = Duration::from_secs(5);
/// A link carrying under this many bits a second for a minute is idle.
const IDLE_BITS: u64 = 1_000_000;
const IDLE_FOR: Duration = Duration::from_secs(60);
/// A probe not answered within this is lost.
const LOST_AFTER: Duration = Duration::from_secs(1);
/// How many days' files are kept: a week to replay, and the day under way.
const KEEP_DAYS: usize = 8;
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
pub fn civil(days: i64) -> (i64, u32, u32) {
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

/// The pool's line in the journal, which says when a choice `kept` it.
fn pool_line(reflectors: &[(&'static str, Ipv4Addr)], kept: bool) -> Value {
    let mut line = json!({
        "at": now_ms(),
        "pool": reflectors.iter().map(|(o, a)| json!([o, a.to_string()])).collect::<Vec<_>>(),
        "operators": operators(reflectors),
        "enough": operators(reflectors) >= ENOUGH,
    });
    if kept {
        line["kept"] = json!(true);
    }
    line
}

/// One link's recording under way.
struct Recorder<'a> {
    link: &'a str,
    gateway: Option<Ipv4Addr>,
    id: u16,
    socket: Socket,
    journal: Journal,
    pool: Pool,
    outstanding: BTreeMap<u16, Sent>,
    seq: u16,
    turn: usize,
    next: Instant,
    window: ((u64, u64), Instant),
    asking: Option<Asking>,
}

impl Recorder<'_> {
    /// Ask every candidate for a timestamp, twice, to choose the next pool
    /// from those that answer.
    fn ask(&mut self, now: Instant) {
        let mut asked = BTreeMap::new();
        for _ in 0..2 {
            for &(operator, address) in &candidates() {
                self.seq = self.seq.wrapping_add(1);
                let request =
                    stamp::request(self.id, self.seq, stamp::since_midnight(SystemTime::now()));
                if self.socket.send(address, &request).is_ok() {
                    asked.insert(self.seq, (operator, address));
                }
            }
        }
        self.asking = Some(Asking::new(asked, now));
    }

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
        let reflectors = &self.pool.reflectors;
        if let Some(&reflector) = reflectors.get(self.turn % reflectors.len().max(1)) {
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
        let stamped = stamp::reply(packet, self.id);
        if let Some(reply) = &stamped
            && let Some(asking) = self.asking.as_mut()
        {
            let finished = stamp::since_midnight(SystemTime::now());
            let standard = stamp::one_way(reply, finished).is_some();
            if asking.answered(reply.seq, icmp::source(packet), standard) {
                return;
            }
        }
        let (rx, tx) = carried(self.link);
        if let Some(reply) = stamped
            && let Some(sent) = self.outstanding.remove(&reply.seq)
            && let Some((operator, address)) = sent.to
        {
            self.pool.heard(address);
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
    /// tell the pool of it.
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
                    self.pool.missed(address, sent.at);
                    json!({"at": now_ms(), "reflector": address.to_string(),
                           "operator": operator, "lost": true})
                }
                None => json!({"at": now_ms(), "gateway": Value::Null, "lost": true}),
            };
            self.journal.write(&line);
        }
    }

    /// Writes the pool in use, naming its link, so a shaper started
    /// mid-stay (src/live.rs) learns within the hour which link it holds,
    /// a kept pool's choice included, which says so.
    fn write_pool(&mut self) {
        let mut line = pool_line(&self.pool.reflectors, self.pool.kept);
        line["link"] = json!(self.link);
        self.journal.write(&line);
    }

    /// Choose the pool again when it's due (src/reflectors.rs says when),
    /// probing the old until the new is chosen.
    fn repool(&mut self, now: Instant) {
        if let Some(asking) = self.asking.take_if(|asking| asking.done(now)) {
            if let Some(said) = self.pool.settle(self.link, &asking, now) {
                eprintln!("{said}");
            }
            self.write_pool();
            return;
        }
        if self.asking.is_none() && self.pool.asks_again(now) {
            self.ask(now);
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
    eprintln!("recording {link} on {network}, choosing reflectors");
    let mut journal = Journal {
        folder: folder.to_path_buf(),
        file: None,
    };
    // The recording opens with its network, which begins a stay
    // (src/live.rs), before any sample; its pool follows once chosen, so
    // this one says it's choosing rather than that nothing answered.
    let mut first = pool_line(&[], false);
    first["choosing"] = json!(true);
    first["link"] = json!(link);
    first["network"] = json!(network);
    journal.write(&first);
    let mut recorder = Recorder {
        link,
        gateway,
        id,
        socket,
        journal,
        pool: Pool::new(Instant::now(), operators(&candidates())),
        outstanding: BTreeMap::new(),
        seq: 0,
        turn: 0,
        next: Instant::now(),
        window: (carried(link), Instant::now()),
        asking: None,
    };
    recorder.ask(Instant::now());
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
        recorder.repool(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A kept pool's line is the pool that was, not a choice, and the
    // record says so.
    #[test]
    fn a_kept_pool_s_line_says_it_was_kept() {
        let pool = [("Quad9", Ipv4Addr::new(9, 9, 9, 9))];
        assert_eq!(pool_line(&pool, true)["kept"], json!(true));
        assert_eq!(pool_line(&pool, false).get("kept"), None);
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
