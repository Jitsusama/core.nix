//! What another machine on the same line sends up, as it reports it, so
//! the upload the two share is held under the house's ceiling. The other
//! machine's packets go out its own Wi-Fi, which this one never sees, and
//! the router's queue they fill is the one the VPN waits in: on 21
//! November the VPN ran late in 6 to 11% of seconds while the two sent
//! under 5 Mb/s between them, and in 87% at 11 to 12, whichever sent it.
//!
//! A report is one datagram a second to `PORT`: the bytes the machine's
//! uplink has sent since it started, and its own clock in milliseconds,
//! as text, `"<bytes> <ms>"`. Bytes over milliseconds, times eight, is
//! kbit/s, measured on the machine's clock rather than by when the
//! datagrams arrive.

use std::collections::BTreeMap;
use std::net::{IpAddr, UdpSocket};
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

/// The rate between two reports in kbit/s, or nothing where they can't
/// say one: the counter went back, as when the machine restarts, the
/// clock didn't move or went back, or they lie too far apart.
pub fn rate(was: Report, now: Report) -> Option<f64> {
    let bytes = now.bytes.checked_sub(was.bytes)?;
    let ms = now
        .ms
        .checked_sub(was.ms)
        .filter(|&ms| ms > 0 && ms <= APART_MS)?;
    Some(crate::rate::float(bytes) * 8.0 / crate::rate::float(ms))
}

/// The most this machine's upload may be held to while the house's
/// ceiling is `most` and the other machine last sent `sent` kbit/s,
/// `age` ago: what the ceiling leaves, never under `floor`. A rate that
/// isn't fresh, or none, holds nothing.
pub fn held(most: u64, sent: Option<(f64, Duration)>, floor: u64) -> Option<u64> {
    let (kbit, _) = sent.filter(|&(_, age)| age <= FRESH)?;
    Some(crate::rate::whole(crate::rate::float(most) - kbit).max(floor))
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
    rate: Option<(f64, Instant)>,
}

impl Ear {
    /// Hears reports on `PORT` from `from` alone, on a thread of its own;
    /// one the port can't be had for says so and hears nothing.
    pub fn start(from: Vec<IpAddr>) -> Self {
        let ear = Ear::default();
        match UdpSocket::bind(("::", PORT)) {
            Ok(socket) => {
                let heard = ear.clone();
                thread::spawn(move || heard.listen(&socket, &from));
            }
            Err(e) => eprintln!("{e}: the house's reports on port {PORT} go unheard"),
        }
        ear
    }

    fn listen(&self, socket: &UdpSocket, from: &[IpAddr]) {
        let mut buf = [0_u8; 64];
        loop {
            let Ok((n, peer)) = socket.recv_from(&mut buf) else {
                continue;
            };
            let address = canonical(peer.ip());
            if !from.contains(&address) {
                continue;
            }
            let Some(report) = std::str::from_utf8(&buf[..n]).ok().and_then(report) else {
                continue;
            };
            self.hear(address, report, Instant::now());
        }
    }

    /// Takes a report from `address`, heard `at`.
    pub fn hear(&self, address: IpAddr, report: Report, at: Instant) {
        let mut heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let was = heard.get(&address).copied();
        let rate = was
            .and_then(|w| rate(w.report, report))
            .map(|kbit| (kbit, at));
        heard.insert(address, Heard { report, rate });
    }

    /// What `address` last sent in kbit/s, and how long ago it said so.
    pub fn sent(&self, address: IpAddr, now: Instant) -> Option<(f64, Duration)> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let (kbit, at) = heard.get(&address)?.rate?;
        Some((kbit, now.saturating_duration_since(at)))
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
