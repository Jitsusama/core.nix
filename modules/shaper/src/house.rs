//! What another machine on the same line sends up, as it reports it, so
//! the upload the two share is held under the house's ceiling. The other
//! machine's packets go out its own Wi-Fi, which this one never sees, and
//! the router's queue they fill is the one the VPN waits in: on 21
//! November the VPN ran late in 6 to 14% of seconds while the two sent
//! under 5 Mb/s between them, and in 87% at 11 to 12, whichever sent it.
//!
//! The other machine reports from the address of whichever interface it
//! sends by, so a house names one for each, and of the fresh reports the
//! one saying the most says what it sends (src/main.rs's `house_holds`),
//! so a report from an address it isn't sending by can't loosen the hold.
//!
//! The source address alone says a report is the other machine's, so any
//! device on the line can forge one by sending from that address: it
//! holds this machine's upload down to the least a trial may set, or lets
//! it go, for as long as it keeps sending. Unlike a trial it needs no
//! administrator. A private IPv4 or unique local IPv6 address is the one
//! to name; a link-local one is shared by every link the machine has, a
//! virtual machine's bridge among them, and the firewall's rule names no
//! interface.
//!
//! A report is one datagram a second to `PORT`: the bytes the machine's
//! uplink has sent since it started, and its own clock in milliseconds,
//! as text, `"<bytes> <ms>"`. Bytes over milliseconds, times eight, is
//! kbit/s, measured on the machine's clock rather than by when the
//! datagrams arrive.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// Where reports are heard, clear of the ports the probes use.
pub const PORT: u16 = 47_110;

/// How long a report's rate stands: past this the machine may have
/// stopped sending, or gone, and holding this one under it for nothing
/// would cost its agents' uploads.
pub const FRESH: Duration = Duration::from_secs(3);

/// The most two reports may lie apart on the reporter's clock and still
/// give a rate: past it, one was lost and the bytes between are spread
/// too thin to say a second's sending.
const APART_MS: u64 = 3_000;

/// How far ahead of the time since the last report a report may be on the
/// reporter's clock and still be taken, in ms. Reports come a second
/// apart, so two the line held back and let through together lie a
/// second apart on the clock and a moment apart here; a report lost
/// between widens the time since and the clock alike.
const AHEAD_MS: u64 = 1_500;

/// The most a rate is read as, in kbit/s: the most a house's ceiling may
/// name (nixos.nix). Reports are numbers off the wire, so one claiming
/// more is read as this, which holds this machine at its least all the
/// same, and the journal and the trace never say a count past it.
const SENT_MOST_KBIT: f64 = 10_000_000.0;

/// The most bytes two reports may lie apart by and be read as such: what
/// `SENT_MOST_KBIT` sends in `APART_MS`. The rate is bounded after all the
/// same; this keeps what `rate::float` is handed far below 2^53, as its
/// reason says every count is.
const BYTES_MOST: u64 = 3_750_000_000;

/// How long the controller is given the highest of the house's ceilings.
/// What the other machine sends moves each second with its own TCP's
/// sawtooth, and a ceiling that moved with it would clamp the controller
/// to each second's low and let it climb back only by its 5% a second, so
/// it would hold this machine under the lowest of the last several, not
/// what the house leaves. The link is held to each second's all the same.
pub const STEADY: Duration = Duration::from_secs(5);

/// The step the controller's ceiling moves by, in kbit/s, so it is tuned
/// again only when the house's moves by this much.
const STEP_KBIT: u64 = 250;

/// One report: the bytes sent, by the reporter's clock in ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    pub bytes: u64,
    pub ms: u64,
}

/// Reads a report's text, or nothing where it isn't one.
pub fn report(text: &str) -> Option<Report> {
    let mut words = text.split_whitespace();
    let bytes = words.next()?.parse().ok()?;
    let ms = words.next()?.parse().ok()?;
    words.next().is_none().then_some(Report { bytes, ms })
}

/// The rate between two reports in kbit/s, never past `SENT_MOST_KBIT`,
/// or nothing where they can't say one: the counter went back, as when
/// the machine restarts, the clock didn't move or went back, or they lie
/// too far apart.
pub fn rate(was: Report, now: Report) -> Option<f64> {
    let bytes = now.bytes.checked_sub(was.bytes)?.min(BYTES_MOST);
    let ms = now
        .ms
        .checked_sub(was.ms)
        .filter(|&ms| ms > 0 && ms <= APART_MS)?;
    Some((crate::rate::float(bytes) * 8.0 / crate::rate::float(ms)).min(SENT_MOST_KBIT))
}

/// The most this machine's upload may be held to while the house's
/// ceiling is `most` and the other machine last sent `sent` kbit/s,
/// `age` ago: what the ceiling leaves, never under `floor`. A rate that
/// isn't fresh, or none, holds nothing.
pub fn held(most: u64, sent: Option<(f64, Duration)>, floor: u64) -> Option<u64> {
    let (kbit, _) = sent.filter(|&(_, age)| age <= FRESH)?;
    Some(crate::rate::whole(crate::rate::float(most) - kbit).max(floor))
}

/// The house's ceilings heard over the last `STEADY`, by network, so the
/// controller is given a ceiling that holds still. The controller may sit
/// above the link's hold for as long, by as much as the house swings. A
/// plateau it lowers is read from the most the link carried over a span
/// as long (`PROVEN_S` in src/oneway.rs), which while the link is busy is
/// near the high this ceiling holds, and never under a declared line's
/// hold, so a second the house held isn't learned as the line.
#[derive(Default)]
pub struct Steady {
    seen: BTreeMap<String, VecDeque<(Instant, u64)>>,
}

impl Steady {
    /// The ceiling the controller is given on each network at `now`, in
    /// kbit/s, from this moment's `held`: the highest of the last
    /// `STEADY`, rounded down to `STEP_KBIT`. `held` is never under the
    /// least a trial may set, a multiple of `STEP_KBIT` (a test holds it
    /// to that), so neither is this. A network whose house holds nothing now is forgotten at once,
    /// so a house that lets go lets the controller go too.
    pub fn of(&mut self, held: &BTreeMap<String, u64>, now: Instant) -> BTreeMap<String, u64> {
        self.seen.retain(|network, _| held.contains_key(network));
        held.iter()
            .map(|(network, &most)| {
                let seen = self.seen.entry(network.clone()).or_default();
                seen.push_back((now, most));
                while seen
                    .front()
                    .is_some_and(|&(at, _)| now.saturating_duration_since(at) > STEADY)
                {
                    seen.pop_front();
                }
                let high = seen.iter().map(|&(_, most)| most).max().unwrap_or(most);
                (network.clone(), high / STEP_KBIT * STEP_KBIT)
            })
            .collect()
    }
}

/// What each reporter last said, by its address: its report, and the
/// rate it gives with the one before and when that came.
#[derive(Clone, Default)]
pub struct Ear {
    heard: Arc<Mutex<BTreeMap<IpAddr, Heard>>>,
}

#[derive(Clone, Copy)]
struct Heard {
    report: Report,
    /// When `report` was heard.
    at: Instant,
    rate: Option<(f64, Instant)>,
}

impl Ear {
    /// Hears reports on `PORT` from `from` alone, on a thread of its own,
    /// which tries for the port each second until it has it.
    pub fn start(from: Vec<IpAddr>) -> Self {
        let ear = Ear::default();
        let heard = ear.clone();
        thread::spawn(move || {
            let socket = bound(&from);
            heard.listen(&socket, &from);
        });
        ear
    }

    fn listen(&self, socket: &UdpSocket, from: &[IpAddr]) {
        let mut buf = [0_u8; 64];
        let mut failed = false;
        let mut first = BTreeSet::new();
        loop {
            let (n, peer) = match socket.recv_from(&mut buf) {
                Ok(heard) => heard,
                Err(e) => {
                    if !failed {
                        eprintln!("{e}: the house's reports go unheard until a read works");
                    }
                    failed = true;
                    // A read that keeps failing would otherwise spin a
                    // core; a report a second needs no quicker retry.
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            if failed {
                eprintln!("the house's reports are read again");
            }
            failed = false;
            let heard = self.take(from, peer.ip(), &buf[..n], Instant::now());
            if let Some(address) = heard.filter(|&address| first.insert(address)) {
                eprintln!("the house's first report from {address} was heard");
            }
        }
    }

    /// Takes a datagram from `peer`, heard `at`, when it comes from an
    /// address in `from` and reads as a report, giving that address.
    fn take(&self, from: &[IpAddr], peer: IpAddr, datagram: &[u8], at: Instant) -> Option<IpAddr> {
        let address = canonical(peer);
        if !from.contains(&address) {
            return None;
        }
        let report = std::str::from_utf8(datagram).ok().and_then(report)?;
        self.hear(address, report, at);
        Some(address)
    }

    /// Takes a report from `address`, heard `at`. Within `FRESH` of the
    /// last one taken, a report is left out when it's no later on the
    /// reporter's clock, as a copy or one come out of order is, or further
    /// ahead than the time since plus `AHEAD_MS`, as a clock that jumped
    /// is: either would become what the next is read against, and the
    /// second would leave out every true one for `FRESH`. A forged report
    /// in step is still taken, and is what the next true one is read
    /// against, so it sets that second's rate: one with a low count puts
    /// this machine at its floor for a second, and one within `AHEAD_MS`
    /// ahead leaves out the true ones for as long. A forger sending its own
    /// reports could do either anyway.
    /// Past `FRESH` any report is read against from then, as one is after
    /// the reporter restarts. One that can't give a rate with the last,
    /// after a report was lost, is read against from then too, and the
    /// last rate stands until it is past `FRESH`, as it would through
    /// silence.
    pub fn hear(&self, address: IpAddr, report: Report, at: Instant) {
        let mut heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let was = heard.get(&address).copied();
        let since = |w: &Heard| at.saturating_duration_since(w.at);
        let in_step = |w: &Heard| {
            let since_ms = u64::try_from(since(w).as_millis()).unwrap_or(u64::MAX);
            report.ms > w.report.ms && report.ms - w.report.ms <= since_ms.saturating_add(AHEAD_MS)
        };
        if was.is_some_and(|w| since(&w) <= FRESH && !in_step(&w)) {
            return;
        }
        let rate = was
            .and_then(|w| rate(w.report, report))
            .map(|kbit| (kbit, at))
            .or(was.and_then(|w| w.rate));
        heard.insert(address, Heard { report, at, rate });
    }

    /// What `address` last sent in kbit/s, and how long ago it said so.
    pub fn sent(&self, address: IpAddr, now: Instant) -> Option<(f64, Duration)> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let (kbit, at) = heard.get(&address)?.rate?;
        Some((kbit, now.saturating_duration_since(at)))
    }
}

/// The addresses to listen on for houses at `from`, in the order tried:
/// IPv4's alone where every one is IPv4, so a machine with IPv6 turned off
/// hears them, else IPv6's, which hears both, then IPv4's, so IPv4 houses
/// are still heard where IPv6's port can't be had.
fn wildcards(from: &[IpAddr]) -> Vec<IpAddr> {
    let v4 = Ipv4Addr::UNSPECIFIED.into();
    if from.iter().all(IpAddr::is_ipv4) {
        vec![v4]
    } else {
        vec![Ipv6Addr::UNSPECIFIED.into(), v4]
    }
}

/// A socket on `PORT` for houses at `from`, tried for each second until
/// one is had, saying once that it isn't, and saying where reports are
/// heard once it is, so a house configured but never heard shows why.
fn bound(from: &[IpAddr]) -> UdpSocket {
    let named: Vec<String> = from.iter().map(ToString::to_string).collect();
    let mut said = false;
    loop {
        let mut refused = Vec::new();
        for any in wildcards(from) {
            match UdpSocket::bind((any, PORT)) {
                Ok(socket) => {
                    // Only IPv6's can be refused before another is had.
                    for e in &refused {
                        eprintln!("{e}: the house's reports over IPv6 go unheard");
                    }
                    eprintln!(
                        "the house's reports are heard on {any} port {PORT}, from {}",
                        named.join(", ")
                    );
                    return socket;
                }
                Err(e) => refused.push(e),
            }
        }
        if let Some(e) = refused.last().filter(|_| !said) {
            eprintln!("{e}: the house's reports on port {PORT} go unheard until it can be had");
            said = true;
        }
        // Another program may hold the port a while, as across a switch;
        // a report a second needs no quicker try.
        thread::sleep(Duration::from_secs(1));
    }
}

/// An IPv4 address heard on the IPv6 socket, as IPv4, so it matches the
/// address named.
fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_its_bytes_and_its_clock() {
        assert_eq!(
            report("1250000 9000\n"),
            Some(Report {
                bytes: 1_250_000,
                ms: 9_000
            })
        );
        for wrong in ["", "12", "12 x", "-1 5", "1 2 3"] {
            assert_eq!(report(wrong), None, "{wrong:?}");
        }
    }

    #[test]
    fn two_reports_give_the_rate_between_them_by_the_reporter_s_clock() {
        let was = Report {
            bytes: 0,
            ms: 1_000,
        };
        let now = Report {
            bytes: 500_000,
            ms: 2_000,
        };
        let kbit = rate(was, now).expect("a rate");
        assert!(
            (kbit - 4_000.0).abs() < 1e-9,
            "500 kB in a second is 4 Mb/s: {kbit}"
        );
    }

    #[test]
    fn reports_that_can_t_say_a_rate_give_none() {
        let was = Report {
            bytes: 900,
            ms: 5_000,
        };
        for (now, case) in [
            (
                Report {
                    bytes: 100,
                    ms: 6_000,
                },
                "the counter went back",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 5_000,
                },
                "the clock didn't move",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 4_000,
                },
                "the clock went back",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 8_001,
                },
                "too far apart",
            ),
        ] {
            assert_eq!(rate(was, now), None, "{case}");
        }
        let apart = Report {
            bytes: 999,
            ms: 5_000 + APART_MS,
        };
        assert!(rate(was, apart).is_some(), "reports just near enough");
    }

    #[test]
    fn a_rate_is_never_read_past_the_most_a_house_may_name() {
        let was = Report { bytes: 0, ms: 0 };
        let now = Report {
            bytes: u64::MAX,
            ms: 1,
        };
        assert_eq!(rate(was, now), Some(SENT_MOST_KBIT), "a count off the wire");
    }

    // The next report reads 4 Mb/s against the last in order and 3 Mb/s
    // against the late one, so a late one taken as its baseline shows.
    #[test]
    fn a_copy_or_a_report_out_of_order_leaves_the_rate_as_it_was() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let ms = |ms: u64| at + Duration::from_millis(ms);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 0), at);
        ear.hear(mac, report(250_000, 1_000), ms(1_000));
        let half = Some((2_000.0, Duration::from_millis(500)));
        ear.hear(mac, report(250_000, 1_000), ms(1_500));
        assert_eq!(
            ear.sent(mac, ms(1_500)),
            half,
            "a copy, which renews nothing"
        );
        ear.hear(mac, report(0, 0), ms(1_500));
        assert_eq!(ear.sent(mac, ms(1_500)), half, "one come late");
        ear.hear(mac, report(750_000, 2_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((4_000.0, Duration::ZERO)),
            "the next read against the last in order"
        );
    }

    // A report far ahead on the reporter's clock, forged or from a clock
    // that jumped, would otherwise be what the next is read against, and
    // leave out every true one for `FRESH`.
    #[test]
    fn a_report_far_ahead_of_its_clock_is_left_out() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let ms = |ms: u64| at + Duration::from_millis(ms);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 0), at);
        ear.hear(mac, report(250_000, 1_000), ms(1_000));
        ear.hear(mac, report(0, u64::MAX), ms(1_500));
        ear.hear(mac, report(0, 1_000 + 500 + AHEAD_MS + 1), ms(1_500));
        ear.hear(mac, report(750_000, 2_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((4_000.0, Duration::ZERO)),
            "the true report after them heard at once"
        );
        ear.hear(mac, report(875_000, 3_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((1_000.0, Duration::ZERO)),
            "one a second ahead, as two the line held back come, taken"
        );
        ear.hear(mac, report(0, 3_000 + 500 + AHEAD_MS), ms(2_500));
        ear.hear(mac, report(1_000_000, 4_000), ms(3_000));
        ear.hear(mac, report(1_125_000, 5_000), ms(4_000));
        assert_eq!(
            ear.sent(mac, ms(4_000)),
            Some((1_000.0, Duration::from_secs(2))),
            "a forged one within the slack leaves out the true ones until they pass it"
        );
        ear.hear(mac, report(1_250_000, 6_000), ms(5_000));
        assert_eq!(
            ear.sent(mac, ms(5_000)),
            Some((10_000.0, Duration::ZERO)),
            "and the first past it is read against it, as against a forger's own reports"
        );
    }

    // The controller's ceiling holds the highest of the last few seconds,
    // stepped, while the other machine's sending swings each second.
    #[test]
    fn the_controller_s_ceiling_holds_still_while_the_house_s_swings() {
        let mut steady = Steady::default();
        let at = Instant::now();
        let home = |most| BTreeMap::from([("home".to_string(), most)]);
        let at_s = |s: u64| at + Duration::from_secs(s);
        let mut said = Vec::new();
        for (s, most) in (0..).zip([5_400, 4_600, 5_300, 4_500, 5_200, 4_700]) {
            said.push(steady.of(&home(most), at_s(s))["home"]);
        }
        assert_eq!(said, [5_250; 6], "the highest, stepped down");
        assert_eq!(
            steady.of(&home(4_600), at_s(7))["home"],
            5_250,
            "one 5 s old kept"
        );
        assert_eq!(
            steady.of(&home(4_600), at_s(8))["home"],
            5_000,
            "one 6 s old gone, the next highest stepped down"
        );
        assert_eq!(
            steady.of(&home(4_600), at_s(14))["home"],
            4_500,
            "a lower that lasts"
        );
        assert!(
            steady.of(&BTreeMap::new(), at_s(14)).is_empty(),
            "a house let go"
        );
        assert_eq!(
            steady.of(&home(4_100), at_s(14))["home"],
            4_000,
            "from nothing after it"
        );
        assert_eq!(
            crate::rate::whole(crate::oneway::MOST_KBIT_LEAST) % STEP_KBIT,
            0,
            "the least a trial may set is a step, so rounding never takes a ceiling under it"
        );
    }

    #[test]
    fn ipv4_houses_alone_listen_on_ipv4_and_any_ipv6_one_tries_ipv6_first() {
        let v4: IpAddr = "192.0.2.7".parse().expect("an address");
        let v6: IpAddr = "2001:db8::7".parse().expect("an address");
        let any4: IpAddr = Ipv4Addr::UNSPECIFIED.into();
        let any6: IpAddr = Ipv6Addr::UNSPECIFIED.into();
        assert_eq!(wildcards(&[v4]), [any4], "IPv4 alone");
        assert_eq!(
            wildcards(&[v4, v6]),
            [any6, any4],
            "IPv6's, then IPv4's where it can't be had"
        );
    }

    #[test]
    fn a_restarted_reporter_is_read_again_once_its_last_rate_is_past() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let second = |s: u64| at + Duration::from_secs(s);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 100_000), at);
        ear.hear(mac, report(250_000, 101_000), second(1));
        ear.hear(mac, report(0, 0), second(2));
        assert_eq!(
            ear.sent(mac, second(2)),
            Some((2_000.0, Duration::from_secs(1))),
            "its last rate stands through the restart"
        );
        ear.hear(mac, report(0, 0), second(5));
        ear.hear(mac, report(125_000, 1_000), second(6));
        assert_eq!(
            ear.sent(mac, second(6)),
            Some((1_000.0, Duration::ZERO)),
            "read from its restart on"
        );
    }

    #[test]
    fn only_a_report_from_an_address_named_is_taken() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().expect("an address");
        let other: IpAddr = "192.0.2.8".parse().expect("an address");
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        ear.take(&[mac], other, b"0 0", at);
        ear.take(&[mac], other, b"250000 1000", later);
        assert_eq!(ear.sent(other, later), None, "an address not named");
        ear.take(&[mac], mapped, b"0 0", at);
        ear.take(&[mac], mapped, b"not a report", later);
        assert_eq!(ear.sent(mac, later), None, "no report in it");
        ear.take(&[mac], mapped, b"250000 1000", later);
        assert_eq!(
            ear.sent(mac, later),
            Some((2_000.0, Duration::ZERO)),
            "the named address, heard on the IPv6 socket"
        );
    }

    #[test]
    fn the_house_s_ceiling_holds_what_the_other_machine_leaves() {
        let now = Duration::from_millis(500);
        assert_eq!(held(7_000, Some((2_500.0, now)), 1_000), Some(4_500));
        assert_eq!(
            held(7_000, Some((6_800.0, now)), 1_000),
            Some(1_000),
            "never under the floor"
        );
        assert_eq!(
            held(7_000, Some((0.0, now)), 1_000),
            Some(7_000),
            "a quiet machine leaves the whole ceiling"
        );
    }

    #[test]
    fn a_stale_report_or_none_holds_nothing() {
        assert_eq!(held(7_000, None, 1_000), None, "no report");
        assert_eq!(
            held(
                7_000,
                Some((2_500.0, FRESH + Duration::from_millis(1))),
                1_000
            ),
            None,
            "a report past its time"
        );
        assert_eq!(
            held(7_000, Some((2_500.0, FRESH)), 1_000),
            Some(4_500),
            "one just in time"
        );
    }

    #[test]
    fn the_ear_gives_the_rate_from_a_reporter_s_last_two_reports() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        ear.hear(mac, Report { bytes: 0, ms: 0 }, at);
        assert_eq!(ear.sent(mac, at), None, "one report gives no rate");
        ear.hear(
            mac,
            Report {
                bytes: 250_000,
                ms: 1_000,
            },
            at,
        );
        let later = at + Duration::from_secs(2);
        assert_eq!(
            ear.sent(mac, later),
            Some((2_000.0, Duration::from_secs(2)))
        );
        let other: IpAddr = "192.0.2.8".parse().expect("an address");
        assert_eq!(ear.sent(other, later), None, "another address said nothing");
    }

    #[test]
    fn an_ipv4_address_heard_on_the_ipv6_socket_reads_as_ipv4() {
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().expect("an address");
        assert_eq!(
            canonical(mapped),
            "192.0.2.7".parse::<IpAddr>().expect("an address")
        );
        let v6: IpAddr = "2001:db8::7".parse().expect("an address");
        assert_eq!(canonical(v6), v6);
    }
}
