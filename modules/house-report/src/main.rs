//! Tells the shaped machine on this line, once a second, what this machine's
//! uplink has sent, so the shaper there holds its own upload under what the
//! two may send between them (modules/shaper/src/house.rs hears it). This
//! machine's packets go out its own link, which the shaped one never sees,
//! and fill the router's queue its VPN waits in.
//!
//! Usage: house-report ADDRESS:PORT INTERFACE. Each second it sends one UDP
//! datagram to ADDRESS:PORT, the text `"<bytes> <ms>"`: the bytes INTERFACE
//! has sent since the reporter started, and its own clock in milliseconds
//! since then. The shaped machine hears it only from the reserved address
//! its house names, so this machine sends from that address by its route,
//! and needs no administrator: it reads a counter any account may read.
//!
//! A second whose counter can't be read sends nothing, since a count that
//! went back reads as a restart there; a report that can't be sent, as
//! while the link changes networks, is dropped. Each is said once in the
//! log as it starts, and again once it has cleared and comes back.
//!
//! It takes its help, exits and failures from the terminal kit
//! (docs/commands.md); it answers nothing, since what it says goes to the
//! other machine.

mod sent;

use std::net::{SocketAddr, UdpSocket};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use terminal_kit::{Exit, Failure, Help};

/// How often a report goes: the shaper takes a rate from two a second
/// apart, and lets the house go when none came for three.
const EVERY: Duration = Duration::from_secs(1);

/// The most an uplink is taken to send in a second, in bytes, 10 Gbit/s: a
/// counter narrower than 64 bits that reads less than it did wrapped, unless
/// that would mean sending more than this since the last reading, when the
/// interface was made again and counts from nothing.
const MOST_PER_SECOND: u64 = 1_250_000_000;

/// What the uplink has sent since the reporter started, summed from its
/// counter's readings, so a counter that wraps or starts over never makes
/// the report go back.
#[derive(Debug, Default)]
struct Total {
    last: Option<u64>,
    bytes: u64,
}

impl Total {
    /// Takes a reading of a counter `bits` wide, `elapsed` after the last.
    fn take(&mut self, reading: u64, bits: u32, elapsed: Duration) {
        if let Some(last) = self.last {
            self.bytes = self
                .bytes
                .saturating_add(grew(last, reading, bits, elapsed));
        }
        self.last = Some(reading);
    }
}

/// What a counter `bits` wide sent between a reading of `last` and one of
/// `now`, `elapsed` apart: the difference, or past a wrap, what it counted to
/// its top and from nothing again, or, for a counter made again, `now`.
fn grew(last: u64, now: u64, bits: u32, elapsed: Duration) -> u64 {
    if let Some(sent) = now.checked_sub(last) {
        return sent;
    }
    let seconds = elapsed.as_secs().saturating_add(1);
    let wrapped = 1u64
        .checked_shl(bits)
        .filter(|_| bits < 64)
        .map(|top| top - last + now);
    wrapped
        .filter(|&sent| sent <= MOST_PER_SECOND.saturating_mul(seconds))
        .unwrap_or(now)
}

/// A report's text, as house.rs reads it.
fn report(bytes: u64, ms: u128) -> String {
    format!("{bytes} {ms}")
}

/// Says a trouble once as it starts, and once it has cleared, so a link
/// that stays down fills no log.
#[derive(Debug, Default)]
struct Once {
    said: bool,
}

impl Once {
    /// Takes whether the trouble stands now, and gives what to say, if
    /// anything.
    fn now(&mut self, trouble: Option<String>, cleared: &str) -> Option<String> {
        match (trouble, self.said) {
            (Some(trouble), false) => {
                self.said = true;
                Some(trouble)
            }
            (None, true) => {
                self.said = false;
                Some(cleared.to_owned())
            }
            _ => None,
        }
    }
}

fn help() -> Help {
    Help::new(
        "house-report",
        "tells the shaped machine on this line what this one sends up",
    )
    .command(
        "ADDRESS:PORT INTERFACE",
        "reports INTERFACE's sent bytes there each second",
    )
    .example(
        "house-report 192.0.2.3:47110 en0",
        "what the agent runs on a machine beside a shaped one",
    )
    .exit(Exit::Usage, "the address or the interface is missing")
    .exit(Exit::Failed, "no socket could be opened to send from")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [to, interface] = context.args.as_slice() else {
            return Err(
                Failure::new(Exit::Usage, "it takes ADDRESS:PORT, then INTERFACE")
                    .try_this("house-report --help"),
            );
        };
        let to: SocketAddr = to.parse().map_err(|_| {
            Failure::new(Exit::Usage, format!("{to} isn't an address and a port"))
                .try_this("house-report 192.0.2.3:47110 en0")
        })?;
        reports(to, interface)
    })
}

/// Sends a report each second, for as long as it runs.
fn reports(to: SocketAddr, interface: &str) -> Result<(), Failure> {
    let any: SocketAddr = if to.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let socket = UdpSocket::bind(any)
        .map_err(|error| Failure::new(Exit::Failed, format!("no socket to send from: {error}")))?;
    let started = Instant::now();
    let mut total = Total::default();
    let mut read_at = started;
    let (mut unread, mut unsent) = (Once::default(), Once::default());
    let mut next = started;
    loop {
        let now = Instant::now();
        let reading = sent::by(interface);
        let trouble = reading.as_ref().err().map(|error| {
            format!("{interface}'s sent bytes can't be read, so nothing is reported: {error}")
        });
        if let Some(said) = unread.now(trouble, &format!("{interface}'s sent bytes read again")) {
            eprintln!("house-report: {said}");
        }
        if let Ok(reading) = reading {
            total.take(reading, sent::BITS, now.duration_since(read_at));
            read_at = now;
            let text = report(total.bytes, now.duration_since(started).as_millis());
            let trouble = socket
                .send_to(text.as_bytes(), to)
                .err()
                .map(|error| format!("reports can't reach {to}: {error}"));
            if let Some(said) = unsent.now(trouble, &format!("reports reach {to} again")) {
                eprintln!("house-report: {said}");
            }
        }
        // A machine that slept wakes far past its next tick, and starts the
        // count of ticks again rather than sending the ones it missed at once.
        next += EVERY;
        let now = Instant::now();
        if next <= now {
            next = now + EVERY;
        }
        thread::sleep(next - now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_the_bytes_then_the_ms() {
        assert_eq!(
            report(1_500, 2_000),
            "1500 2000",
            "a report reads as house.rs parses it"
        );
    }

    #[test]
    fn the_total_starts_at_nothing_and_grows_by_what_the_counter_counts() {
        let mut total = Total::default();
        total.take(9_000, 64, EVERY);
        assert_eq!(
            total.bytes, 0,
            "what the counter held before the reporter started counted"
        );
        total.take(10_500, 64, EVERY);
        total.take(12_000, 64, EVERY);
        assert_eq!(
            total.bytes, 3_000,
            "the total isn't what the counter counted since"
        );
    }

    #[test]
    fn a_narrow_counter_that_wraps_counts_on() {
        let top = 1u64 << 32;
        assert_eq!(
            grew(top - 1_000, 500, 32, EVERY),
            1_500,
            "a 32-bit counter's wrap didn't count what it sent past its top"
        );
    }

    #[test]
    fn a_counter_made_again_counts_from_nothing() {
        assert_eq!(
            grew(3_000_000_000, 700, 64, EVERY),
            700,
            "a 64-bit counter that went back wasn't taken as made again"
        );
        assert_eq!(
            grew(1_000, 700, 32, EVERY),
            700,
            "a 32-bit counter that went back more than a second could send was taken as wrapped"
        );
    }

    #[test]
    fn a_wrap_after_a_long_gap_is_still_a_wrap() {
        let top = 1u64 << 32;
        let (last, now) = (top - 1_000, 4_000_000_000);
        assert_eq!(
            grew(last, now, 32, Duration::from_secs(10)),
            now + 1_000,
            "a wrap ten seconds on, within what ten seconds could send, was taken as made again"
        );
        assert_eq!(
            grew(last, now, 32, EVERY),
            now,
            "four gigabytes a second on was taken as a wrap rather than a counter made again"
        );
    }

    #[test]
    fn a_trouble_is_said_once_and_its_end_once() {
        let mut once = Once::default();
        let down = || Some("down".to_owned());
        assert_eq!(
            once.now(down(), "up"),
            Some("down".to_owned()),
            "a trouble went unsaid"
        );
        assert_eq!(
            once.now(down(), "up"),
            None,
            "a trouble that stood was said again"
        );
        assert_eq!(
            once.now(None, "up"),
            Some("up".to_owned()),
            "a trouble's end went unsaid"
        );
        assert_eq!(once.now(None, "up"), None, "an end was said again");
    }
}
