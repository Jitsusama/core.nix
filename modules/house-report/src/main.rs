//! Tells the shaped machine on this line, once a second, what this machine's
//! uplink has sent, so the shaper there holds its own upload under what the
//! two may send between them (modules/shaper/src/house.rs hears it). This
//! machine's packets go out its own link, which the shaped one never sees,
//! and fill the router's queue its VPN waits in.
//!
//! Usage: house-report [--key FILE] ADDRESS:PORT [INTERFACE]. Each second
//! it sends one UDP datagram to ADDRESS:PORT, the text `"<bytes> <ms>"`:
//! the bytes its uplink has sent since the reporter started, and its own
//! clock in milliseconds since then. Given the house's key in FILE, it
//! signs each (src/signed.rs), reading FILE each second, so a key put
//! there or swapped while it runs is taken without a restart, where FILE
//! is the key itself or a copy kept fresh, as a Mac's is (darwin.nix),
//! rather than a copy made at start, as systemd's credentials are
//! (nixos.nix restarts it for those). A copy made at start can't
//! change while it runs, so one that can't be used at start, empty or
//! short, as a key made first and filled after can be copied, ends it
//! failed, for its service to start it again on a fresh copy, as a
//! missing one does (house signing round 3's F9). The uplink
//! is INTERFACE, or, without one, the interface this machine's routes
//! send to ADDRESS by that second, so a
//! Mac given a cable beside its Wi-Fi is counted by the one it sends by.
//! A report goes from that interface's address, so the shaped machine's
//! house names each interface's reserved address, and the log says which
//! interface and address reports go by, and again each time they move.
//! It reads a counter any account may read, 64 bits wide on Linux and on a
//! Mac, so a count that went back was made again, never wrapped.
//!
//! A second with no route, whose counter can't be read, or whose key
//! can't be read when one is named, sends nothing,
//! since a count that went back reads as a restart there; a report that
//! can't be sent, as while the link changes networks, is dropped. Each
//! trouble is said once in the log as it starts, and again once it has
//! cleared.
//!
//! It takes its help, exits and failures from the terminal kit
//! (docs/commands.md); it answers nothing, since what it says goes to the
//! other machine.

mod route;
mod sent;
mod signed;

use std::env;
use std::ffi::OsStr;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use terminal_kit::{Exit, Failure, Help};

/// How often a report goes: the shaper takes a rate from two a second
/// apart, and lets the house go when none came for three.
const EVERY: Duration = Duration::from_secs(1);

/// What the uplink has sent since the reporter started, summed from its
/// counter's readings, so a counter that starts over, as an interface made
/// again does, never makes the report go back.
#[derive(Debug, Default)]
struct Total {
    last: Option<(String, u64)>,
    bytes: u64,
}

impl Total {
    /// Takes a reading of `interface`'s counter. A reading of another
    /// interface than the last, the uplink having moved, counts nothing: its
    /// counter counted all it ever sent, and from the next it counts on.
    fn take(&mut self, interface: &str, reading: u64) {
        if let Some((was, last)) = &self.last
            && was == interface
        {
            self.bytes = self.bytes.saturating_add(grew(*last, reading));
        }
        self.last = Some((interface.to_owned(), reading));
    }
}

/// What a 64-bit counter sent between a reading of `last` and one of
/// `now`: the difference, or, for a counter that went back, which a 64-bit
/// one does only when it was made again, all it counted since, `now`.
fn grew(last: u64, now: u64) -> u64 {
    now.checked_sub(last).unwrap_or(now)
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
        "[--key FILE] ADDRESS:PORT [INTERFACE]",
        "reports the sent bytes of INTERFACE, or of the one sent there by, each second, \
         signed with the key in FILE",
    )
    .example(
        "house-report --key /etc/house.key 192.0.2.3:47110",
        "what a machine beside a shaped one runs",
    )
    .exit(Exit::Usage, "the address is missing or isn't one")
    .exit(
        Exit::Failed,
        "no socket could be opened to send from, or a key copied at start can't be used",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (key, rest) = match context.args.as_slice() {
            [flag, file, rest @ ..] if flag == "--key" => (Some(Path::new(file.as_str())), rest),
            rest => (None, rest),
        };
        let (to, interface) = match rest {
            [to] => (to, None),
            [to, interface] => (to, Some(interface.as_str())),
            _ => {
                return Err(Failure::new(
                    Exit::Usage,
                    "it takes --key FILE if any, ADDRESS:PORT, then INTERFACE if any",
                )
                .try_this("house-report --help"));
            }
        };
        let to: SocketAddr = to.parse().map_err(|_| {
            Failure::new(Exit::Usage, format!("{to} isn't an address and a port"))
                .try_this("house-report 192.0.2.3:47110")
        })?;
        reports(to, interface, key)
    })
}

/// What the log says of the way reports go, when it's another than
/// `last`'s: the interface the routes send to `to` by, and the address
/// they go from, which the shaped machine's house must name.
fn moved(
    last: Option<&(String, IpAddr)>,
    now: &(String, IpAddr),
    to: SocketAddr,
) -> Option<String> {
    (last != Some(now)).then(|| {
        let (interface, from) = now;
        format!("reports go to {to} by {interface}, from {from}")
    })
}

/// The troubles a second can meet, each said once as it starts and once as
/// it clears.
#[derive(Debug, Default)]
struct Troubles {
    unrouted: Once,
    unread: Once,
    unkeyed: Once,
    unsent: Once,
}

/// Says `said` in the log, if anything.
fn say(said: Option<String>) {
    if let Some(said) = said {
        eprintln!("house-report: {said}");
    }
}

/// The uplink to count this second, `interface` or the one the routes
/// send to `to` by, saying in the log when the way reports go moves or its
/// route can't be found.
fn uplink(
    to: SocketAddr,
    interface: Option<&str>,
    way: &mut Option<(String, IpAddr)>,
    troubles: &mut Troubles,
) -> Option<String> {
    if let Some(interface) = interface {
        return Some(interface.to_owned());
    }
    let found = route::toward(to);
    let trouble = found
        .as_ref()
        .err()
        .map(|error| format!("no interface sends to {to}, so nothing is reported: {error}"));
    say(troubles
        .unrouted
        .now(trouble, &format!("an interface sends to {to} again")));
    let now = found.ok()?;
    say(moved(way.as_ref(), &now, to));
    let uplink = now.0.clone();
    *way = Some(now);
    Some(uplink)
}

/// The report's text, signed when a key is named: nothing where the key
/// or the clock can't be read, said in the log once as it starts and
/// once as it clears, since an unsigned report would only be refused.
fn text(bytes: u64, ms: u128, key: Option<&Path>, troubles: &mut Troubles) -> Option<String> {
    let text = report(bytes, ms);
    let Some(path) = key else {
        return Some(text);
    };
    let read = signed::key(path).and_then(|key| {
        let unix_ms = signed::unix_ms().ok_or_else(|| "the clock is set before 1970".to_owned())?;
        Ok(signed::signed(&text, &key, unix_ms))
    });
    let trouble = read
        .as_ref()
        .err()
        .map(|error| format!("no report can be signed, so nothing is reported: {error}"));
    say(troubles.unkeyed.now(
        trouble,
        &format!("the key in {} is read, so reports go again", path.display()),
    ));
    read.ok()
}

/// Whether the key at `path` is a copy its service made as it started,
/// in the folder `credentials` names, which nothing changes while it runs.
fn copied_at_start(path: &Path, credentials: Option<&OsStr>) -> bool {
    credentials.is_some_and(|folder| !folder.is_empty() && path.starts_with(folder))
}

/// Sends a report each second, for as long as it runs.
fn reports(to: SocketAddr, interface: Option<&str>, key: Option<&Path>) -> Result<(), Failure> {
    if let Some(path) = key
        && copied_at_start(path, env::var_os("CREDENTIALS_DIRECTORY").as_deref())
        && let Err(error) = signed::key(path)
    {
        return Err(Failure::new(
            Exit::Failed,
            format!("the key copied at start can't be used, so it ends for a fresh copy: {error}"),
        ));
    }
    let any: SocketAddr = if to.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let socket = UdpSocket::bind(any)
        .map_err(|error| Failure::new(Exit::Failed, format!("no socket to send from: {error}")))?;
    if let Some(interface) = interface {
        eprintln!("house-report: reports go to {to}, counting {interface}'s sent bytes");
    }
    match key {
        Some(path) => eprintln!(
            "house-report: reports are signed with the key in {}",
            path.display()
        ),
        None => {
            eprintln!(
                "house-report: reports go unsigned, so the house takes them by address alone"
            );
        }
    }
    let started = Instant::now();
    let mut total = Total::default();
    let mut troubles = Troubles::default();
    let mut way = None;
    let mut next = started;
    loop {
        let now = Instant::now();
        if let Some(uplink) = uplink(to, interface, &mut way, &mut troubles) {
            let count = sent::by(&uplink);
            let trouble = count.as_ref().err().map(|error| {
                format!("{uplink}'s sent bytes can't be read, so nothing is reported: {error}")
            });
            // One trouble stands for every uplink, so a move from one that
            // can't be read to one that can clears it, and what's said
            // then names the uplink read now, not the one that failed.
            say(troubles.unread.now(
                trouble,
                &format!("{uplink}'s sent bytes can be read, so reports go again"),
            ));
            if let Ok(count) = count {
                total.take(&uplink, count);
                let ms = now.duration_since(started).as_millis();
                if let Some(text) = text(total.bytes, ms, key, &mut troubles) {
                    let trouble = socket
                        .send_to(text.as_bytes(), to)
                        .err()
                        .map(|error| format!("reports can't reach {to}: {error}"));
                    say(troubles
                        .unsent
                        .now(trouble, &format!("reports reach {to} again")));
                }
            }
        }
        // A step that ran late, its process starved, starts the count of
        // ticks again rather than sending the ones it missed at once. A
        // machine's sleep isn't one: Instant stops while it sleeps.
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
    fn only_a_key_in_the_credentials_folder_is_a_copy_made_at_start() {
        let folder = OsStr::new("/run/credentials/house-report.service");
        assert!(
            copied_at_start(
                Path::new("/run/credentials/house-report.service/house.key"),
                Some(folder)
            ),
            "a credential wasn't a copy"
        );
        assert!(
            !copied_at_start(Path::new("/etc/house.key"), Some(folder)),
            "a key read in place was a copy"
        );
        assert!(
            !copied_at_start(Path::new("/etc/house.key"), None),
            "a key with no credentials was a copy"
        );
        assert!(
            !copied_at_start(Path::new("/etc/house.key"), Some(OsStr::new(""))),
            "an empty credentials folder named every key a copy"
        );
    }

    #[test]
    fn a_report_is_the_bytes_then_the_ms() {
        assert_eq!(
            report(1_500, 2_000),
            "1500 2000",
            "a report's text isn't the bytes, a space, then the ms, as house.rs reads it"
        );
    }

    #[test]
    fn a_report_is_signed_only_when_a_key_is_named_and_none_goes_while_it_cant_be_read() {
        let mut troubles = Troubles::default();
        assert_eq!(
            text(1_500, 2_000, None, &mut troubles).as_deref(),
            Some("1500 2000"),
            "a report with no key named wasn't the plain text"
        );
        let folder = std::env::temp_dir().join(format!("house-report-text-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        let path = folder.join("house.key");
        let unread = text(1_500, 2_000, Some(&path), &mut troubles);
        assert!(
            troubles.unkeyed.said,
            "a key that can't be read went unsaid"
        );
        std::fs::write(&path, [9_u8; 32]).expect("a key");
        let signed = text(1_500, 2_000, Some(&path), &mut troubles);
        std::fs::remove_dir_all(&folder).expect("the folder removed");
        assert_eq!(
            unread, None,
            "a report went unsigned while its key couldn't be read"
        );
        assert!(!troubles.unkeyed.said, "a key read again wasn't said to be");
        let now = signed::unix_ms().expect("a clock past 1970");
        let words: Vec<&str> = signed.as_deref().unwrap_or_default().split(' ').collect();
        let ["1500", "2000", clock, _] = words.as_slice() else {
            panic!("a report under a key wasn't the text, a clock and a mac: {signed:?}");
        };
        let clock: u128 = clock.parse().expect("a clock in ms");
        // Stamped by the wall clock, not the reporter's own, and signed
        // with the key's bytes, not its path (house signing round 1's F6).
        assert!(
            now.abs_diff(clock) <= 5_000,
            "a report stamped {clock}, {now} by the wall clock"
        );
        assert_eq!(
            signed.as_deref(),
            Some(signed::signed("1500 2000", &[9_u8; 32], clock).as_str()),
            "a report under a key wasn't signed with its bytes"
        );
    }

    #[test]
    fn the_total_starts_at_nothing_and_grows_by_what_the_counter_counts() {
        let mut total = Total::default();
        total.take("en0", 9_000);
        assert_eq!(
            total.bytes, 0,
            "what the counter held before the reporter started counted"
        );
        total.take("en0", 10_500);
        total.take("en0", 12_000);
        assert_eq!(
            total.bytes, 3_000,
            "the total isn't what the counter counted since"
        );
    }

    #[test]
    fn a_counter_made_again_counts_from_nothing() {
        for last in [1_000, 3_000_000_000, 5_000_000_000] {
            assert_eq!(
                grew(last, 700),
                700,
                "a counter that went back from {last} wasn't taken as made again"
            );
        }
        let mut total = Total::default();
        for reading in [5_000_000_000, 5_000_001_000, 700, 1_700] {
            total.take("en0", reading);
        }
        assert_eq!(
            total.bytes, 2_700,
            "a counter made again made the total go back or jump"
        );
    }

    #[test]
    fn an_uplink_that_moves_counts_on_from_the_new_ones_count() {
        let mut total = Total::default();
        for (uplink, reading) in [
            ("en0", 1_000),
            ("en0", 2_000),
            ("en5", 90_000),
            ("en5", 90_500),
        ] {
            total.take(uplink, reading);
        }
        assert_eq!(
            total.bytes, 1_500,
            "the move counted the new uplink's whole count, or what it sent after was lost"
        );
    }

    #[test]
    fn the_way_reports_go_is_said_first_and_each_time_it_moves() {
        let to: SocketAddr = "192.0.2.3:47110".parse().expect("an address");
        let wifi = ("en0".to_owned(), "192.0.2.7".parse().expect("an address"));
        let cable = ("en5".to_owned(), "192.0.2.8".parse().expect("an address"));
        assert_eq!(
            moved(None, &wifi, to).as_deref(),
            Some("reports go to 192.0.2.3:47110 by en0, from 192.0.2.7"),
            "the first way reports go wasn't said, or not with its interface and address"
        );
        assert_eq!(
            moved(Some(&wifi), &wifi, to),
            None,
            "a way that stood was said again"
        );
        assert_eq!(
            moved(Some(&wifi), &cable, to).as_deref(),
            Some("reports go to 192.0.2.3:47110 by en5, from 192.0.2.8"),
            "a move to the cable went unsaid"
        );
        let renumbered = ("en0".to_owned(), "192.0.2.9".parse().expect("an address"));
        assert!(
            moved(Some(&wifi), &renumbered, to).is_some(),
            "a new address on the same interface went unsaid"
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
