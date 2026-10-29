//! What timesyncd says of how it keeps the clock, and what its last
//! exchange with its server says, worked out as `timedatectl
//! timesync-status` works it out: the clock's offset from the server and
//! the round trip's delay from the exchange's four times, the reference a
//! stratum-one server keeps by its name, and the leap second it announces
//! in words. Everything Time's Server holds is here (VKCF3Z's design,
//! 2026-10-29: Joel wants to dig into the NTP server's details).

use std::net::{Ipv4Addr, Ipv6Addr};

use serde::Serialize;

/// The version of what's written, which the shell reads only when it's
/// this one.
pub const VERSION: u32 = 2;

/// Microseconds in a second, as D-Bus gives each poll interval.
const MICROS: u64 = 1_000_000;

/// Microseconds in a millisecond, as the shell's clock counts.
const PER_MILLI: u64 = 1_000;

/// timesyncd's frequency is in 2^-16 parts per million, as the kernel's
/// adjtimex keeps it.
const FREQUENCY_SCALE: f64 = 65_536.0;

/// An address family as D-Bus says it, which Linux numbers.
const INET: i32 = 2;
const INET6: i32 = 10;

/// timesyncd's last exchange with its server, as its `NTPMessage`
/// property gives it, the times in microseconds of the realtime clock, the
/// root delay and dispersion in microseconds, and the precision a power of
/// two of a second.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub leap: u32,
    pub version: u32,
    pub stratum: u32,
    pub precision: i32,
    pub root_delay: u64,
    pub root_dispersion: u64,
    pub reference: Vec<u8>,
    pub origin: u64,
    pub receive: u64,
    pub transmit: u64,
    pub destination: u64,
    pub packets: u64,
    pub jitter: u64,
}

/// What's written for the launcher while timesyncd runs: whether the
/// clock counts as synchronised, how often the server is asked, the
/// clock's frequency, the servers it's given, and its last exchange, or
/// none before the first.
#[derive(Debug, PartialEq, Serialize)]
pub struct Reading {
    pub version: u32,
    pub synchronised: bool,
    pub poll: Poll,
    pub frequency: f64,
    pub servers: Servers,
    pub exchange: Option<Exchange>,
}

/// How often the server is asked, in seconds: now, and the least and most
/// timesyncd lets it be.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Poll {
    pub now: u64,
    pub least: u64,
    pub most: u64,
}

/// The servers timesyncd is given, by where each list comes from: its own
/// settings, the network's links, the runtime, and its fallbacks.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Servers {
    pub system: Vec<String>,
    pub link: Vec<String>,
    pub runtime: Vec<String>,
    pub fallback: Vec<String>,
}

/// The last exchange with the server: each time in microseconds, the
/// precision a power of two of a second, and when it was made in
/// milliseconds since the epoch, as the shell's clock counts.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exchange {
    pub server: String,
    pub address: String,
    pub stratum: u32,
    pub reference: String,
    pub offset: i64,
    pub delay: i64,
    pub jitter: u64,
    pub root_delay: u64,
    pub root_dispersion: u64,
    pub precision: i32,
    pub leap: &'static str,
    pub ntp_version: u32,
    pub last: u64,
    pub packets: u64,
}

/// What the rest of timesyncd says beside its last exchange, each poll
/// interval in microseconds, as D-Bus gives it.
#[derive(Default)]
pub struct Said {
    pub synchronised: bool,
    pub server: String,
    pub address: String,
    pub poll: u64,
    pub poll_least: u64,
    pub poll_most: u64,
    pub frequency: i64,
    pub servers: Servers,
}

impl Message {
    /// How far the clock is from the server's: half of how much later each
    /// end heard the other than it said.
    pub fn offset(&self) -> i64 {
        let out = signed(self.receive) - signed(self.origin);
        let back = signed(self.transmit) - signed(self.destination);
        i64::midpoint(out, back)
    }

    /// How long the exchange took on the network: the round trip, less the
    /// time the server held it.
    pub fn delay(&self) -> i64 {
        let round = signed(self.destination) - signed(self.origin);
        let held = signed(self.transmit) - signed(self.receive);
        round - held
    }

    /// What the server keeps its time by: a stratum-one server's is a name
    /// of up to four letters ("GNSS", "PPS"), another's the address of the
    /// server it asks in turn.
    pub fn reference(&self) -> String {
        if self.stratum <= 1 {
            return self
                .reference
                .iter()
                .take_while(|byte| **byte != 0)
                .filter(|byte| byte.is_ascii_graphic())
                .map(|byte| char::from(*byte))
                .collect();
        }
        <[u8; 4]>::try_from(self.reference.as_slice())
            .map(|octets| Ipv4Addr::from(octets).to_string())
            .unwrap_or_default()
    }

    /// The leap second the server announces, in words.
    pub fn leap(&self) -> &'static str {
        match self.leap {
            0 => "normal",
            1 => "second added",
            2 => "second removed",
            _ => "not synchronised",
        }
    }

    /// The exchange as it's written, with the server it was made with.
    pub fn exchange(&self, server: String, address: String) -> Exchange {
        Exchange {
            server,
            address,
            stratum: self.stratum,
            reference: self.reference(),
            offset: self.offset(),
            delay: self.delay(),
            jitter: self.jitter,
            root_delay: self.root_delay,
            root_dispersion: self.root_dispersion,
            precision: self.precision,
            leap: self.leap(),
            ntp_version: self.version,
            last: self.destination / PER_MILLI,
            packets: self.packets,
        }
    }
}

impl Said {
    /// The whole reading, with the last exchange when there's been one.
    pub fn reading(self, message: Option<&Message>) -> Reading {
        #[expect(clippy::cast_precision_loss, reason = "a frequency is never near 2^52")]
        let frequency = self.frequency as f64 / FREQUENCY_SCALE;
        Reading {
            version: VERSION,
            synchronised: self.synchronised,
            poll: Poll {
                now: self.poll / MICROS,
                least: self.poll_least / MICROS,
                most: self.poll_most / MICROS,
            },
            frequency: (frequency * 1000.0).round() / 1000.0,
            servers: self.servers,
            exchange: message.map(|message| message.exchange(self.server, self.address)),
        }
    }
}

/// A time as a signed count, which every realtime microsecond count fits.
fn signed(time: u64) -> i64 {
    i64::try_from(time).unwrap_or(i64::MAX)
}

/// An address as D-Bus gives it, a family and its bytes, written as people
/// read it, or "" for none.
pub fn address(family: i32, bytes: &[u8]) -> String {
    match family {
        INET => <[u8; 4]>::try_from(bytes).map(|octets| Ipv4Addr::from(octets).to_string()),
        INET6 => <[u8; 16]>::try_from(bytes).map(|octets| Ipv6Addr::from(octets).to_string()),
        _ => return String::new(),
    }
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An exchange as a laptop's timesyncd reported one, kept to a GNSS clock.
    fn heard() -> Message {
        Message {
            leap: 0,
            version: 4,
            stratum: 1,
            precision: -25,
            root_delay: 15,
            root_dispersion: 457,
            reference: b"GNSS".to_vec(),
            origin: 1_793_248_675_277_792,
            receive: 1_793_248_675_296_236,
            transmit: 1_793_248_675_296_297,
            destination: 1_793_248_675_313_965,
            packets: 13,
            jitter: 11_888,
        }
    }

    #[test]
    fn the_offset_and_delay_are_worked_out_as_timedatectl_does() {
        assert_eq!(heard().offset(), 388, "half of 18,444 less 17,668");
        assert_eq!(
            heard().delay(),
            36_112,
            "a round trip of 36,173 less 61 held"
        );
    }

    #[test]
    fn a_stratum_one_server_keeps_its_time_by_a_name() {
        assert_eq!(heard().reference(), "GNSS");
        let padded = Message {
            reference: b"PPS\0".to_vec(),
            ..heard()
        };
        assert_eq!(
            padded.reference(),
            "PPS",
            "the name ends where its padding starts"
        );
    }

    #[test]
    fn a_lower_server_keeps_its_time_by_the_server_it_asks() {
        let lower = Message {
            stratum: 2,
            reference: vec![192, 0, 2, 7],
            ..heard()
        };
        assert_eq!(lower.reference(), "192.0.2.7");
    }

    #[test]
    fn an_address_is_written_as_people_read_it() {
        assert_eq!(address(INET, &[209, 227, 142, 155]), "209.227.142.155");
        let mut six = [0_u8; 16];
        six[0] = 0x20;
        six[1] = 0x01;
        six[2] = 0x0d;
        six[3] = 0xb8;
        six[15] = 1;
        assert_eq!(address(INET6, &six), "2001:db8::1");
        assert_eq!(address(0, &[]), "", "no address yet says none");
    }

    fn said() -> Said {
        Said {
            synchronised: true,
            server: "2.nixos.pool.ntp.org".to_owned(),
            address: "209.227.142.155".to_owned(),
            poll: 2_048_000_000,
            poll_least: 32_000_000,
            poll_most: 2_048_000_000,
            frequency: 410_704,
            servers: Servers {
                fallback: vec!["0.nixos.pool.ntp.org".to_owned()],
                ..Servers::default()
            },
        }
    }

    #[test]
    fn the_reading_says_the_polls_in_seconds_and_the_frequency_in_ppm() {
        let reading = said().reading(Some(&heard()));
        assert_eq!(
            reading.poll,
            Poll {
                now: 2048,
                least: 32,
                most: 2048
            }
        );
        assert!(
            (reading.frequency - 6.267).abs() < 1e-9,
            "{}",
            reading.frequency
        );
        assert_eq!(reading.servers.fallback, ["0.nixos.pool.ntp.org"]);
        assert_eq!(reading.version, VERSION);
    }

    #[test]
    fn the_exchange_holds_all_the_server_says() {
        let exchange = said()
            .reading(Some(&heard()))
            .exchange
            .expect("an exchange was made");
        assert_eq!(exchange.server, "2.nixos.pool.ntp.org");
        assert_eq!(exchange.leap, "normal");
        assert_eq!((exchange.root_delay, exchange.root_dispersion), (15, 457));
        assert_eq!((exchange.precision, exchange.ntp_version), (-25, 4));
        assert_eq!(
            exchange.last, 1_793_248_675_313,
            "when it was heard back, in milliseconds"
        );
    }

    #[test]
    fn before_the_first_exchange_it_says_how_it_keeps_the_clock_alone() {
        let reading = said().reading(None);
        assert_eq!(reading.exchange, None);
        assert_eq!(reading.poll.least, 32);
    }

    #[test]
    fn it_is_written_as_the_shell_reads_it() {
        let text = serde_json::to_string(&said().reading(Some(&heard()))).expect("it writes");
        for key in [
            "\"rootDelay\":15",
            "\"ntpVersion\":4",
            "\"poll\":{\"now\":2048",
        ] {
            assert!(text.contains(key), "{key} in {text}");
        }
    }
}
