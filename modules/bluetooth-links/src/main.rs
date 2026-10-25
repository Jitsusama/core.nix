//! Says how each connected Bluetooth link is doing, for the launcher: its
//! signal, how well it carries, how loud this end sends, which of the
//! radio's 79 channels it hops over and which it skips as noisy (Bluetooth's
//! own view of how crowded the air is), and how it's held: over which radio,
//! which end leads it and started it, and whether it's encrypted. Beside
//! them, each adapter: its address, the version of Bluetooth it follows and
//! who made it, and how many bytes a second it carries.
//!
//! `BlueZ` says none of it for a connected device, so every five seconds
//! the kernel's HCI socket (src/hci.rs) is asked what each adapter is and
//! what it's connected to, and each link is asked the controller's four
//! commands for it; what comes back (src/reply.rs) is written as JSON:
//!
//! ```json
//! {"at": 1761350400,
//!  "adapters": [{"name": "hci0", "address": "02:00:00:00:00:01", "up": true,
//!    "version": 14, "maker": 2, "received": 1200, "sent": 300,
//!    "traffic": [1500, 1450]}],
//!  "links": [{"address": "00:1B:66:AA:BB:CC", "rssi": 0, "quality": 230,
//!    "power": 4, "channels": 61, "map": "1111...0111", "le": false,
//!    "held": {"central": true, "outgoing": false, "security": "secure"},
//!    "history": {"rssi": [-2, 0], "quality": [225, 230]}}]}
//! ```
//!
//! with the map's 79 channels in full, channel 0 first. An adapter's
//! `received` and `sent` are bytes a second since the look before, and its
//! `traffic` and a link's `history` the last twenty looks, newest last, so
//! a graph drawn from the file is full the moment it's opened.
//!
//! A command that fails leaves its field null, and a read in a history.
//! While a link is up the file is written every look; while none is, only
//! when it would say something new, so once the adapters' traffic has
//! settled it's written no more, and the controller is asked nothing but
//! its version, once. Nothing is spawned.
//!
//! Usage: bluetooth-links [FILE]   (/run/bluetooth-links/links.json unless
//! given)

mod hci;
mod reply;

use hci::{Radio, Seen};
use reply::{Adapter, Connection, Version};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TICK: Duration = Duration::from_secs(5);
const FILE: &str = "/run/bluetooth-links/links.json";
/// How many looks a history keeps: as many as the launcher's graphs draw.
const KEPT: usize = 20;

/// What one look at the links found.
#[derive(Debug, Serialize, PartialEq)]
struct Report {
    /// When, in seconds since 1970, left out when there's nothing to say.
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<u64>,
    /// Left out on a machine with no Bluetooth.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    adapters: Vec<Told>,
    links: Vec<Link>,
}

/// One adapter: its name and address as the kernel says them, whether it's
/// up, the version of Bluetooth it follows and who made it (as
/// `reply::Version` numbers them, null until it's been up to ask), and the
/// bytes a second it received and sent since the look before, with their
/// sum's history.
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
struct Told {
    name: String,
    address: String,
    up: bool,
    version: Option<u8>,
    maker: Option<u16>,
    received: Option<u64>,
    sent: Option<u64>,
    traffic: Vec<u64>,
}

impl Told {
    /// It as the file's compared with it while nothing is connected: a
    /// history of nothing carried draws the same however long it is, so
    /// it's no reason to write the file again.
    fn settled(&self) -> Self {
        let mut settled = self.clone();
        if settled.traffic.iter().all(|&rate| rate == 0) {
            settled.traffic.clear();
        }
        settled
    }
}

/// One link. On a classic link `rssi` is how many dB the signal is above or
/// below the range its radio hears best, 0 inside it, as the specification
/// has a controller say; on a Low Energy link it's the signal in dBm.
#[derive(Debug, Serialize, PartialEq)]
struct Link {
    address: String,
    rssi: Option<i8>,
    quality: Option<u8>,
    power: Option<i8>,
    channels: Option<usize>,
    map: Option<String>,
    le: bool,
    held: Held,
    history: History,
}

/// How a link is held: whether this end leads it (`central`) and started
/// it (`outgoing`), and how well it's kept from others.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Held {
    central: bool,
    outgoing: bool,
    security: Security,
}

/// How well a link is kept from others, most first: encrypted with Secure
/// Connections' keys, encrypted with older ones, the other end only proved
/// who it is, or none of it.
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Security {
    Secure,
    Encrypted,
    Authenticated,
    None,
}

impl Security {
    const fn of(connection: &Connection) -> Self {
        if connection.encrypted() {
            if connection.secure() {
                Self::Secure
            } else {
                Self::Encrypted
            }
        } else if connection.authenticated() {
            Self::Authenticated
        } else {
            Self::None
        }
    }
}

/// A link's last reads of its signal and quality, newest last.
#[derive(Debug, Serialize, PartialEq, Eq, Default, Clone)]
struct History {
    rssi: Vec<Option<i8>>,
    quality: Vec<Option<u8>>,
}

/// Adds a read to a history, keeping the last few.
fn kept<T>(history: &mut Vec<T>, read: T) {
    history.push(read);
    if history.len() > KEPT {
        history.drain(..history.len() - KEPT);
    }
}

/// What the controller is asked, by `Watch`: a stand-in in the tests.
trait Controller {
    fn look(&mut self) -> Seen;
    fn ask(&mut self, adapter: u16, opcode: u16, handle: u16) -> Option<Vec<u8>>;
}

/// The radio, saying each kind of trouble once rather than every tick.
struct Asked {
    radio: Radio,
    said: HashSet<String>,
}

impl Asked {
    fn complain(&mut self, what: String) {
        if !self.said.contains(&what) {
            eprintln!("{what}");
            self.said.insert(what);
        }
    }
}

impl Controller for Asked {
    fn look(&mut self) -> Seen {
        self.radio.look().unwrap_or_else(|error| {
            self.complain(format!("Couldn't list the Bluetooth adapters: {error}"));
            Seen::default()
        })
    }

    fn ask(&mut self, adapter: u16, opcode: u16, handle: u16) -> Option<Vec<u8>> {
        self.radio
            .ask(adapter, opcode, handle)
            .map_err(|error| {
                self.complain(format!("Couldn't ask hci{adapter} {opcode:#06x}: {error}"));
            })
            .ok()
    }
}

/// An adapter's counters at the look before, and the history of its
/// traffic.
struct Counted {
    at: Duration,
    received: u32,
    sent: u32,
    traffic: Vec<u64>,
}

/// What's kept between looks: the adapters as last written with nothing
/// connected, so they're written once, each link's history, each adapter's
/// counters, and each adapter's version, which never changes.
#[derive(Default)]
struct Watch {
    quiet: Option<Vec<Told>>,
    histories: HashMap<String, History>,
    counted: HashMap<u16, Counted>,
    versions: HashMap<u16, Version>,
}

/// Bytes a second between two reads of a counter. A counter that went
/// backwards started again, the adapter brought up afresh, so what it says
/// is all it's carried since.
fn per_second(now: u32, before: u32, over: Duration) -> u64 {
    let carried = u64::from(now.checked_sub(before).unwrap_or(now));
    let millis = u64::try_from(over.as_millis()).unwrap_or(u64::MAX).max(1);
    carried.saturating_mul(1000) / millis
}

impl Watch {
    /// One look, at `at` seconds since 1970 and `clock` on a clock that
    /// only goes forward: what to write, or nothing when it would say what
    /// the file already does.
    fn tick(
        &mut self,
        controller: &mut impl Controller,
        at: u64,
        clock: Duration,
    ) -> Option<Report> {
        let seen = controller.look();
        let adapters: Vec<Told> = seen
            .adapters
            .iter()
            .map(|adapter| self.adapter(controller, adapter, clock))
            .collect();
        let listed: Vec<u16> = seen.adapters.iter().map(|adapter| adapter.id).collect();
        self.counted.retain(|id, _| listed.contains(id));
        let links: Vec<Link> = seen
            .connections
            .iter()
            .map(|(adapter, connection)| link(controller, *adapter, connection))
            .collect();
        let linked: Vec<&str> = links.iter().map(|link| link.address.as_str()).collect();
        self.histories
            .retain(|address, _| linked.contains(&address.as_str()));
        let links = links
            .into_iter()
            .map(|mut link| {
                let history = self.histories.entry(link.address.clone()).or_default();
                kept(&mut history.rssi, link.rssi);
                kept(&mut history.quality, link.quality);
                link.history = history.clone();
                link
            })
            .collect::<Vec<_>>();
        if links.is_empty() {
            let settled: Vec<Told> = adapters.iter().map(Told::settled).collect();
            if self.quiet.as_ref() == Some(&settled) {
                return None;
            }
            self.quiet = Some(settled);
            return Some(Report {
                at: None,
                adapters,
                links,
            });
        }
        self.quiet = None;
        Some(Report {
            at: Some(at),
            adapters,
            links,
        })
    }

    /// What's said of an adapter, asking its version the first time it's
    /// up, and working out its traffic since the look before.
    fn adapter(
        &mut self,
        controller: &mut impl Controller,
        adapter: &Adapter,
        clock: Duration,
    ) -> Told {
        if adapter.up && !self.versions.contains_key(&adapter.id) {
            let said = controller.ask(adapter.id, reply::READ_LOCAL_VERSION, 0);
            if let Some(version) = said.as_deref().and_then(reply::version) {
                self.versions.insert(adapter.id, version);
            }
        }
        let version = self.versions.get(&adapter.id);
        let before = self.counted.remove(&adapter.id);
        let mut traffic = before
            .as_ref()
            .map(|before| before.traffic.clone())
            .unwrap_or_default();
        let rates = before.as_ref().and_then(|before| {
            let over = clock
                .checked_sub(before.at)
                .filter(|over| !over.is_zero())?;
            Some((
                per_second(adapter.received, before.received, over),
                per_second(adapter.sent, before.sent, over),
            ))
        });
        if let Some((received, sent)) = rates {
            kept(&mut traffic, received + sent);
        }
        self.counted.insert(
            adapter.id,
            Counted {
                at: clock,
                received: adapter.received,
                sent: adapter.sent,
                traffic: traffic.clone(),
            },
        );
        Told {
            name: adapter.name.clone(),
            address: adapter.address.clone(),
            up: adapter.up,
            version: version.map(|said| said.version),
            maker: version.map(|said| said.maker),
            received: rates.map(|(received, _)| received),
            sent: rates.map(|(_, sent)| sent),
            traffic,
        }
    }
}

/// Asks a link all it's asked. Link quality and the channel map are the
/// classic radio's, so a Low Energy link isn't asked them. Its history is
/// filled in by `Watch`.
fn link(controller: &mut impl Controller, adapter: u16, connection: &Connection) -> Link {
    let handle = connection.handle;
    let mut asked = |opcode| controller.ask(adapter, opcode, handle);
    let rssi = asked(reply::READ_RSSI).and_then(|said| reply::rssi(&said, handle));
    let power = asked(reply::READ_TRANSMIT_POWER).and_then(|said| reply::power(&said, handle));
    let (quality, map) = if connection.le {
        (None, None)
    } else {
        (
            asked(reply::READ_LINK_QUALITY).and_then(|said| reply::quality(&said, handle)),
            asked(reply::READ_AFH_MAP).and_then(|said| reply::channel_map(&said, handle)),
        )
    };
    Link {
        address: connection.address.clone(),
        rssi,
        quality,
        power,
        channels: map.as_deref().map(reply::in_use),
        map,
        le: connection.le,
        held: Held {
            central: connection.central(),
            outgoing: connection.outgoing,
            security: Security::of(connection),
        },
        history: History::default(),
    }
}

/// Writes the report beside the file and renames it into place, so a
/// reader never sees it half written, readable by everyone.
fn write(file: &Path, report: &Report) -> io::Result<()> {
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("links.json");
    let next = file.with_file_name(format!(".{name}.next"));
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o644)
        .open(&next)?;
    // The mode given is cut by the umask; the file is for every account.
    out.set_permissions(fs::Permissions::from_mode(0o644))?;
    serde_json::to_writer(&mut out, report)?;
    out.write_all(b"\n")?;
    fs::rename(&next, file)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn main() {
    let file = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from(FILE), PathBuf::from);
    let mut asked = Asked {
        radio: Radio::default(),
        said: HashSet::new(),
    };
    let mut watch = Watch::default();
    let started = Instant::now();
    loop {
        if let Some(report) = watch.tick(&mut asked, now(), started.elapsed())
            && let Err(error) = write(&file, &report)
        {
            asked.complain(format!("Couldn't write {}: {error}", file.display()));
            // Written again next time, though it says nothing new.
            watch.quiet = None;
        }
        thread::sleep(TICK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A controller that lists what it's given and answers what it has a
    /// reply for, by opcode and handle.
    #[derive(Default)]
    struct StandIn {
        adapters: Vec<Adapter>,
        listed: Vec<(u16, Connection)>,
        replies: HashMap<(u16, u16), Vec<u8>>,
        asked: Vec<u16>,
    }

    impl Controller for StandIn {
        fn look(&mut self) -> Seen {
            Seen {
                adapters: self.adapters.clone(),
                connections: self.listed.clone(),
            }
        }

        fn ask(&mut self, _: u16, opcode: u16, handle: u16) -> Option<Vec<u8>> {
            self.asked.push(opcode);
            self.replies.get(&(opcode, handle)).cloned()
        }
    }

    fn headphones() -> Connection {
        Connection {
            handle: 0x000b,
            address: "00:1B:66:AA:BB:CC".into(),
            le: false,
            outgoing: true,
            mode: 0x0007,
        }
    }

    fn mouse() -> Connection {
        Connection {
            handle: 0x0040,
            address: "C3:9C:E3:2A:F3:6E".into(),
            le: true,
            outgoing: false,
            mode: 0x0000,
        }
    }

    fn radio(received: u32, sent: u32) -> Adapter {
        Adapter {
            id: 0,
            name: "hci0".into(),
            address: "02:00:00:00:00:01".into(),
            up: true,
            received,
            sent,
        }
    }

    fn seconds(count: u64) -> Duration {
        Duration::from_secs(count)
    }

    #[test]
    fn nothing_connected_is_written_once() {
        let mut watch = Watch::default();
        let mut controller = StandIn::default();
        let first = watch.tick(&mut controller, 10, seconds(0)).unwrap();
        assert_eq!(serde_json::to_string(&first).unwrap(), r#"{"links":[]}"#);
        assert_eq!(watch.tick(&mut controller, 15, seconds(5)), None);
        assert!(
            controller.asked.is_empty(),
            "it asked a command with nothing connected"
        );
    }

    #[test]
    fn a_link_coming_and_going_is_written_each_time() {
        let mut watch = Watch::default();
        let mut controller = StandIn {
            listed: vec![(0, headphones())],
            ..StandIn::default()
        };
        assert!(watch.tick(&mut controller, 10, seconds(0)).is_some());
        assert!(
            watch.tick(&mut controller, 15, seconds(5)).is_some(),
            "a link is written every tick"
        );
        controller.listed.clear();
        let gone = watch.tick(&mut controller, 20, seconds(10)).unwrap();
        assert_eq!(gone.links, vec![]);
        assert_eq!(watch.tick(&mut controller, 25, seconds(15)), None);
    }

    #[test]
    fn a_classic_link_is_asked_all_four() {
        let mut map = vec![0x00, 0x0b, 0x00, 0x01];
        map.extend_from_slice(&[0xff, 0xff, 0x3f, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x7f]);
        let replies = HashMap::from([
            ((reply::READ_RSSI, 0x000b), vec![0x00, 0x0b, 0x00, 0xfa]),
            (
                (reply::READ_LINK_QUALITY, 0x000b),
                vec![0x00, 0x0b, 0x00, 0xe6],
            ),
            (
                (reply::READ_TRANSMIT_POWER, 0x000b),
                vec![0x00, 0x0b, 0x00, 0x04],
            ),
            ((reply::READ_AFH_MAP, 0x000b), map),
        ]);
        let mut controller = StandIn {
            listed: vec![(0, headphones())],
            replies,
            ..StandIn::default()
        };
        let report = Watch::default()
            .tick(&mut controller, 1_761_350_400, seconds(0))
            .unwrap();
        let link = &report.links[0];
        assert_eq!(report.at, Some(1_761_350_400));
        assert_eq!(
            (link.rssi, link.quality, link.power),
            (Some(-6), Some(230), Some(4))
        );
        assert_eq!(link.channels, Some(61));
        // Channels 22 to 39 skipped, where a Wi-Fi network on its channel 6
        // sits.
        let map = link.map.as_deref().unwrap();
        assert_eq!((&map[20..24], &map[38..42]), ("1100", "0011"));
        assert_eq!(
            link.held,
            Held {
                central: true,
                outgoing: true,
                security: Security::Encrypted
            }
        );
    }

    #[test]
    fn a_command_that_fails_leaves_its_field_null() {
        let replies = HashMap::from([((reply::READ_RSSI, 0x0040), vec![0x00, 0x40, 0x00, 0xc6])]);
        let mut controller = StandIn {
            listed: vec![(0, mouse())],
            replies,
            ..StandIn::default()
        };
        let report = Watch::default()
            .tick(&mut controller, 1, seconds(0))
            .unwrap();
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            concat!(
                r#"{"at":1,"links":[{"address":"C3:9C:E3:2A:F3:6E","rssi":-58,"quality":null,"#,
                r#""power":null,"channels":null,"map":null,"le":true,"#,
                r#""held":{"central":false,"outgoing":false,"security":"none"},"#,
                r#""history":{"rssi":[-58],"quality":[null]}}]}"#
            )
        );
        assert_eq!(
            controller.asked,
            vec![reply::READ_RSSI, reply::READ_TRANSMIT_POWER],
            "a Low Energy link was asked the classic radio's commands"
        );
    }

    #[test]
    fn a_link_keeps_its_last_twenty_reads() {
        let mut watch = Watch::default();
        let mut controller = StandIn {
            listed: vec![(0, mouse())],
            ..StandIn::default()
        };
        for step in 0..25u8 {
            let said = vec![0x00, 0x40, 0x00, 0xc0 + step];
            controller.replies.insert((reply::READ_RSSI, 0x0040), said);
            watch.tick(&mut controller, 1, seconds(u64::from(step) * 5));
        }
        let report = watch.tick(&mut controller, 1, seconds(125)).unwrap();
        let history = &report.links[0].history.rssi;
        assert_eq!(history.len(), KEPT);
        // Twenty-six reads, the newest last: the seventh, 0xc6, is -58, and
        // the last, 0xd8 read twice, is -40.
        assert_eq!((history[0], history[19]), (Some(-58), Some(-40)));
        controller.listed.clear();
        watch.tick(&mut controller, 1, seconds(130));
        controller.listed = vec![(0, mouse())];
        let back = watch.tick(&mut controller, 1, seconds(135)).unwrap();
        assert_eq!(
            back.links[0].history.rssi.len(),
            1,
            "a link that came back started its history afresh"
        );
    }

    #[test]
    fn an_adapter_says_its_traffic_and_version() {
        let mut watch = Watch::default();
        let version = vec![0x00, 0x0e, 0x00, 0x01, 0x0e, 0x02, 0x00, 0x00, 0x01];
        let mut controller = StandIn {
            adapters: vec![radio(1_000, 500)],
            replies: HashMap::from([((reply::READ_LOCAL_VERSION, 0), version)]),
            ..StandIn::default()
        };
        let first = watch.tick(&mut controller, 1, seconds(0)).unwrap();
        let told = &first.adapters[0];
        assert_eq!((told.version, told.maker), (Some(14), Some(2)));
        assert_eq!(
            (told.received, told.traffic.len()),
            (None, 0),
            "a first look has no rate"
        );
        controller.adapters = vec![radio(6_000, 1_500)];
        let second = watch.tick(&mut controller, 6, seconds(5)).unwrap();
        let told = &second.adapters[0];
        assert_eq!((told.received, told.sent), (Some(1_000), Some(200)));
        assert_eq!(told.traffic, vec![1_200]);
        assert_eq!(
            controller.asked,
            vec![reply::READ_LOCAL_VERSION],
            "its version was asked more than once"
        );
    }

    #[test]
    fn an_idle_adapter_stops_being_written() {
        let mut watch = Watch::default();
        let mut controller = StandIn {
            adapters: vec![radio(1_000, 500)],
            ..StandIn::default()
        };
        let written = (0..30u64)
            .filter(|step| watch.tick(&mut controller, 1, seconds(step * 5)).is_some())
            .count();
        // The first look, and the one with its first rate, of nothing.
        assert_eq!(written, 2, "an idle adapter was written {written} times");
        // Traffic rolls the history on until what it carried has gone.
        controller.adapters = vec![radio(6_000, 500)];
        let written = (30..60u64)
            .filter(|step| watch.tick(&mut controller, 1, seconds(step * 5)).is_some())
            .count();
        assert_eq!(written, KEPT + 1, "traffic was written {written} times");
    }

    #[test]
    fn a_links_security_is_the_most_it_has() {
        let with = |mode| Security::of(&Connection { mode, ..mouse() });
        assert_eq!(with(0x0027), Security::Secure);
        assert_eq!(with(0x0006), Security::Encrypted);
        assert_eq!(with(0x0004), Security::Encrypted, "encrypted unproved");
        assert_eq!(with(0x0002), Security::Authenticated);
        assert_eq!(with(0x0001), Security::None, "only leading it");
    }

    #[test]
    fn a_counter_that_starts_again_counts_from_nothing() {
        assert_eq!(per_second(10_000, 5_000, seconds(5)), 1_000);
        assert_eq!(per_second(2_000, 9_000, seconds(2)), 1_000);
        assert_eq!(per_second(5, 0, Duration::ZERO), 5_000);
    }

    #[test]
    fn the_file_is_replaced_whole_and_readable_by_all() {
        let folder = std::env::temp_dir().join(format!("bluetooth-links-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let file = folder.join("links.json");
        fs::write(&file, "old").unwrap();
        write(
            &file,
            &Report {
                at: None,
                adapters: vec![],
                links: vec![],
            },
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\"links\":[]}\n");
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(
            fs::read_dir(&folder).unwrap().count(),
            1,
            "the next file was left behind"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}
