//! Says how each connected Bluetooth link is doing, for the launcher: its
//! signal, how well it carries, how loud this end sends, and which of the
//! radio's 79 channels it hops over and which it skips as noisy, which is
//! Bluetooth's own view of how crowded the air is.
//!
//! `BlueZ` says none of it for a connected device, so every five seconds,
//! while some link is up, each is asked the controller's four commands for
//! them through the kernel's HCI socket (src/hci.rs), and what comes back
//! (src/reply.rs) is written as JSON:
//!
//! ```json
//! {"at": 1761350400, "links": [{"address": "00:1B:66:AA:BB:CC", "rssi": 0,
//!   "quality": 230, "power": 4, "channels": 61, "map": "1111...0111", "le": false}]}
//! ```
//!
//! with the map's 79 channels in full, channel 0 first.
//!
//! A command that fails leaves its field null. While nothing is connected
//! it writes `{"links": []}` once, and only looks at the list of
//! connections, which costs an ioctl. Nothing is spawned.
//!
//! Usage: bluetooth-links [FILE]   (/run/bluetooth-links/links.json unless
//! given)

mod hci;
mod reply;

use hci::Radio;
use reply::Connection;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TICK: Duration = Duration::from_secs(5);
const FILE: &str = "/run/bluetooth-links/links.json";

/// What one look at the links found.
#[derive(Debug, Serialize, PartialEq)]
struct Report {
    /// When, in seconds since 1970, left out when there's nothing to say.
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<u64>,
    links: Vec<Link>,
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
}

/// What the controller is asked, by `Watch`: a stand-in in the tests.
trait Controller {
    fn connections(&mut self) -> Vec<(u16, Connection)>;
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
    fn connections(&mut self) -> Vec<(u16, Connection)> {
        self.radio.connections().unwrap_or_else(|error| {
            self.complain(format!("Couldn't list the Bluetooth connections: {error}"));
            Vec::new()
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

/// Whether the empty list is already written, so it's written once.
#[derive(Default)]
struct Watch {
    quiet: bool,
}

impl Watch {
    /// One look: what to write, or nothing when it would say what the file
    /// already does.
    fn tick(&mut self, controller: &mut impl Controller, at: u64) -> Option<Report> {
        let links: Vec<Link> = controller
            .connections()
            .iter()
            .map(|(adapter, connection)| link(controller, *adapter, connection))
            .collect();
        if links.is_empty() {
            if self.quiet {
                return None;
            }
            self.quiet = true;
            return Some(Report { at: None, links });
        }
        self.quiet = false;
        Some(Report {
            at: Some(at),
            links,
        })
    }
}

/// Asks a link all it's asked. Link quality and the channel map are the
/// classic radio's, so a Low Energy link isn't asked them.
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
    loop {
        if let Some(report) = watch.tick(&mut asked, now())
            && let Err(error) = write(&file, &report)
        {
            asked.complain(format!("Couldn't write {}: {error}", file.display()));
            // Written again next time, though it says nothing new.
            watch.quiet = false;
        }
        thread::sleep(TICK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A controller that lists what it's given and answers what it has a
    /// reply for, by opcode and handle.
    #[derive(Default)]
    struct StandIn {
        listed: Vec<(u16, Connection)>,
        replies: HashMap<(u16, u16), Vec<u8>>,
        asked: Vec<u16>,
    }

    impl Controller for StandIn {
        fn connections(&mut self) -> Vec<(u16, Connection)> {
            self.listed.clone()
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
        }
    }

    fn mouse() -> Connection {
        Connection {
            handle: 0x0040,
            address: "C3:9C:E3:2A:F3:6E".into(),
            le: true,
        }
    }

    #[test]
    fn nothing_connected_is_written_once() {
        let mut watch = Watch::default();
        let mut controller = StandIn::default();
        let first = watch.tick(&mut controller, 10).unwrap();
        assert_eq!(serde_json::to_string(&first).unwrap(), r#"{"links":[]}"#);
        assert_eq!(watch.tick(&mut controller, 15), None);
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
        assert!(watch.tick(&mut controller, 10).is_some());
        assert!(
            watch.tick(&mut controller, 15).is_some(),
            "a link is written every tick"
        );
        controller.listed.clear();
        assert_eq!(watch.tick(&mut controller, 20).unwrap().links, vec![]);
        assert_eq!(watch.tick(&mut controller, 25), None);
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
            asked: vec![],
        };
        let report = Watch::default()
            .tick(&mut controller, 1_761_350_400)
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
    }

    #[test]
    fn a_command_that_fails_leaves_its_field_null() {
        let replies = HashMap::from([((reply::READ_RSSI, 0x0040), vec![0x00, 0x40, 0x00, 0xc6])]);
        let mut controller = StandIn {
            listed: vec![(0, mouse())],
            replies,
            asked: vec![],
        };
        let report = Watch::default().tick(&mut controller, 1).unwrap();
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            concat!(
                r#"{"at":1,"links":[{"address":"C3:9C:E3:2A:F3:6E","rssi":-58,"quality":null,"#,
                r#""power":null,"channels":null,"map":null,"le":true}]}"#
            )
        );
        assert_eq!(
            controller.asked,
            vec![reply::READ_RSSI, reply::READ_TRANSMIT_POWER],
            "a Low Energy link was asked the classic radio's commands"
        );
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
