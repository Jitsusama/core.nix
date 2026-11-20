//! The home network as net keeps it: every device as one box with the
//! roles it plays, its ports and radios, and the links between them, each
//! drawn from a source, the vendor's way of saying it (docs/glossary.md's
//! Network words). The generic fields are what every device can say; a
//! role's own figures sit in its block, as a modem's channels do; and
//! everything a source gives about a device is kept as the source says it
//! under `detail`, so making a device fit the generic shape never loses a
//! figure. A figure a source doesn't give is `None`, never a guess, and a
//! figure keeps its unit in its name, as eero's record does.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The schema the record names, so a reader can tell it can read it.
pub const SCHEMA: &str = "net.network/1";

/// The whole home network, written to `$XDG_RUNTIME_DIR/net/network.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Network {
    pub schema: String,
    /// When the record was put together from its sources, in seconds since
    /// the epoch, as every time of net's own is, so a reading's age is one
    /// subtraction.
    pub collected_at: u64,
    pub sources: Vec<Source>,
    pub devices: Vec<Device>,
    pub links: Vec<Link>,
    /// The internet beyond the modem, as last measured. A record written
    /// before it had this reads as not said, as do the concerns below.
    #[serde(default)]
    pub internet: Option<Internet>,
    /// What's worth a look, worked out from the figures above
    /// (src/room.rs): a busy radio, a weak signal, a slow cable, or the
    /// internet being the narrowest hop.
    #[serde(default)]
    pub concerns: Vec<Concern>,
}

/// The internet the provider carries to the modem, as its source last
/// measured it each way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Internet {
    pub provider: Option<String>,
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
    pub measured_at: Option<u64>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried: Option<Carried>,
}

/// What the line carried over the last day, as its source counted it at
/// the gateway: the bytes each way, and each hour's average speed, oldest
/// first, so an hour reads against the line's own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Carried {
    pub down_bytes: u64,
    pub up_bytes: u64,
    pub hours: Vec<Hour>,
}

/// One hour of what the line carried, from `at`, as its average speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hour {
    pub at: u64,
    pub down_mbps: f64,
    pub up_mbps: f64,
}

/// A device's way to the internet: the devices its traffic crosses, in
/// order, up to the modem, and the hop with least room each way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub through: Vec<String>,
    pub down: Option<Chokepoint>,
    pub up: Option<Chokepoint>,
}

/// The hop on a path with least room left: a link between devices `a`, the
/// nearer the device, and `b`, the nearer the modem; or the internet beyond
/// `a`, the modem, when `b` is `None`.
/// A link's room is its speed less the share of time its radio is busy;
/// a link whose room isn't known is counted in `unmeasured`, since the
/// path's real chokepoint may be one of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chokepoint {
    pub a: String,
    pub b: Option<String>,
    pub room_mbps: f64,
    pub unmeasured: u32,
}

/// Something on the home network worth a look, where it is, and in words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Concern {
    pub kind: ConcernKind,
    pub device: String,
    /// The radio or port on the device it's about, when it's one of them,
    /// or the way, down or up, a device fills the internet or the internet
    /// is the narrowest point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    /// The device at a link's other end, when it's about a link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
    /// What it is, as one whole sentence, its stop included, since net's
    /// view, the network page and the launcher's Map row each say it
    /// whole; a reader joining it into a phrase trims the stop.
    pub says: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConcernKind {
    /// A radio busy half the time or more, slowing all it carries.
    Busy,
    /// A Wi-Fi link heard at -70 dBm or weaker.
    Weak,
    /// A cable link under a gigabit.
    Slow,
    /// The internet is the narrowest hop on some device's path.
    Ceiling,
    /// A device using most of the internet's speed one way, so every other
    /// device waits behind it.
    Fills,
    /// One of the modem's downstream channels lost far more than the
    /// others, which points at noise on its frequency.
    Lost,
}

impl ConcernKind {
    /// How far a concern of this kind reaches, the widest first: the line
    /// itself, and a device filling it, touch every device; a slow cable
    /// everything behind it; a busy radio all it carries; a weak signal
    /// one device. The internet's ceiling is the line's speed, not a fault.
    fn reach(self) -> u8 {
        match self {
            ConcernKind::Lost => 0,
            ConcernKind::Fills => 1,
            ConcernKind::Slow => 2,
            ConcernKind::Busy => 3,
            ConcernKind::Weak => 4,
            ConcernKind::Ceiling => 5,
        }
    }
}

/// The concerns worst first, by how far each kind reaches, keeping the
/// order they were found in within a kind. A reader meets the worst
/// first, and the launcher's Map row, which says only the first, says it.
pub(crate) fn worst_first(mut concerns: Vec<Concern>) -> Vec<Concern> {
    concerns.sort_by_key(|concern| concern.kind.reach());
    concerns
}

/// Where a vendor's facts come from, and whether they're current.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    /// What it reads, in words.
    pub what: String,
    pub state: SourceState,
    /// When it last answered; `None` when it never has.
    pub read_at: Option<u64>,
    /// Since when it's been in this state, when that isn't answering.
    pub since: Option<u64>,
    /// Why it isn't answering.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceState {
    Answering,
    Unreachable,
    SignedOut,
}

/// One box on the home network, whatever it does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    /// Stable across reads: `modem`, `switch`, `eero:<id>` or `mac:<mac>`.
    pub id: String,
    pub name: String,
    pub roles: Vec<Role>,
    /// The source the device's generic fields come from.
    pub source: String,
    pub read_at: Option<u64>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub hardware: Option<String>,
    pub firmware: Option<String>,
    pub serial: Option<String>,
    pub macs: Vec<String>,
    pub addresses: Vec<String>,
    pub up_s: Option<u64>,
    pub ports: Vec<Port>,
    pub radios: Vec<Radio>,
    /// A modem's own figures, on a device with the modem role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modem: Option<Modem>,
    /// Its way to the internet, when the links reach the modem from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Path>,
    /// A receiver's own figures, on a device with the receiver role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver: Option<Receiver>,
    /// A printer's own figures, on a device with the printer role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printer: Option<Printer>,
    /// A television's own figures, on a device with the television role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub television: Option<Television>,
    /// How fast it carries data each way, on a device eero counts it for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<Speed>,
    /// Everything each source gives about the device, in its own words.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub detail: Map<String, Value>,
}

/// What a device does on the home network; one box may do several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Modem,
    Router,
    AccessPoint,
    Switch,
    /// A device that plays what's sent to it over the home network into
    /// speakers, as an AV receiver does; not `player`, which the glossary
    /// keeps for a program playing on the machine.
    Receiver,
    /// A device that prints what's sent to it over the home network.
    Printer,
    /// A device that shows what it's sent on a screen of its own, as a TV
    /// does; not `screen`, which the glossary keeps for the machine's
    /// monitor.
    Television,
    /// A thing that only joins the network, as a laptop or a phone does;
    /// not `host`, which the glossary keeps for pi's host.
    Client,
}

/// A socket a cable plugs into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub kind: PortKind,
    pub link: Option<LinkState>,
    pub speed_mbps: Option<u32>,
    pub duplex: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortKind {
    Ethernet,
    Coax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkState {
    Up,
    Down,
}

/// How fast a device carries data each way, in megabits a second, as eero
/// counts it: now, and at its busiest in the hour eero keeps, at five-minute
/// steps, with when. A way eero gave nothing for is null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
    pub busiest_down_mbps: Option<f64>,
    pub busiest_down_at: Option<u64>,
    pub busiest_up_mbps: Option<f64>,
    pub busiest_up_at: Option<u64>,
}

/// A Wi-Fi radio on one band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Radio {
    pub name: String,
    pub band: Option<String>,
    pub channel: Option<u32>,
    pub width_mhz: Option<u32>,
    pub busy_pct: Option<u32>,
    pub power_dbm: Option<i32>,
    /// Its busiest share over the last day, from net's own readings, and
    /// when; left out until net has read it. Like the two below, it's
    /// left out of the record while unknown, so a record without a day
    /// stays its own size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busiest_pct: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busiest_at: Option<u64>,
}

/// A cable or a radio link between two devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub a: End,
    pub b: End,
    pub medium: Medium,
    /// Whole megabits, as a port's speed is, so the two compare.
    pub speed_mbps: Option<u32>,
    pub channel: Option<u32>,
    /// How strongly a radio link's far end hears the near one, as its
    /// access point measures it. A record written before it had this
    /// reads as not said.
    #[serde(default)]
    pub signal_dbm: Option<i32>,
    /// The source that says the link is there.
    pub source: String,
}

/// One end of a link: a device, and the port or radio on it when known.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct End {
    pub device: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Medium {
    Ethernet,
    Wifi,
    Coax,
}

/// What a cable modem says of its line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modem {
    pub docsis: Option<String>,
    /// Whether the provider lets it on the network.
    pub network_access: Option<String>,
    pub config_file: Option<String>,
    /// The modem's own address on the provider's network.
    pub address: Option<String>,
    pub downstream: Vec<Channel>,
    pub upstream: Vec<Channel>,
    /// The modem's event log, newest first.
    pub log: Vec<LogEntry>,
    /// How far back each downstream channel's `lost_lately` counts, in
    /// seconds: net's earliest reading in the last day since the modem
    /// last started, and none until that reaches back an hour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lately_s: Option<u64>,
}

/// What a television says of itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Television {
    /// Whether it's on or in standby, in its own word; a TV switched off
    /// at the wall says nothing, so it has no figures at all.
    pub power: Option<String>,
}

/// What a receiver says it's doing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receiver {
    /// Its volume, on its own scale.
    pub volume: Option<u32>,
    /// The top of its volume's scale, as its rendering service describes
    /// it.
    pub loudest: Option<u32>,
    pub muted: Option<bool>,
    /// What its transport is doing, in plain words: playing, paused,
    /// stopped or nothing to play. `UPnP`'s own word stays in its detail.
    pub state: Option<String>,
    /// What it plays from, as it names it, such as network; none while it
    /// plays from nothing.
    pub medium: Option<String>,
    /// Where what it plays comes from, while it plays.
    pub track: Option<String>,
}

/// What a printer says it's doing, its ink and what it has printed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Printer {
    /// What it's doing, in plain words, each thing it says at once: ready,
    /// printing, out of paper, saving power.
    pub state: Vec<String>,
    /// Each ink cartridge, in the order it sits in the printer.
    pub ink: Vec<Ink>,
    /// How many pages it has printed in its life.
    pub pages: Option<u64>,
}

/// One ink cartridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ink {
    /// Its colour: black, colour (cyan, magenta and yellow in one), or
    /// one of those three alone.
    pub colour: String,
    /// How much is left, as a percent of a full one.
    pub left_percent: Option<u32>,
    /// How the printer judges what's left, in plain words: ok, low.
    pub state: Option<String>,
}

/// One channel of a cable line, down or up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub channel: Option<u32>,
    /// sc-qam, ofdm, atdma or ofdma.
    pub kind: String,
    pub frequency_hz: Option<u64>,
    pub width_hz: Option<u64>,
    pub modulation: Option<String>,
    pub power_dbmv: Option<f64>,
    pub snr_db: Option<f64>,
    pub corrected: Option<u64>,
    pub uncorrectable: Option<u64>,
    /// Down: whether it's locked. Up: whether it's in use.
    pub active: bool,
    /// How many of `uncorrectable` it lost over the modem's `lately_s`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost_lately: Option<u64>,
}

/// One line of a device's event log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// When, in seconds since the epoch, read from the device's clock;
    /// `None` when that clock's zone isn't known.
    pub at: Option<u64>,
    /// When, as the device's clock wrote it, so a clock that's wrong can
    /// be seen.
    pub clock: String,
    pub priority: String,
    pub code: Option<String>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The record every view is drawn from in its tests, as the daemon
    /// writes it.
    const RECORD: &str = include_str!("../tests/network.json");

    #[test]
    fn the_record_reads_into_the_model_and_back_unchanged() {
        let network: Network = serde_json::from_str(RECORD).expect("the record reads");
        let written = serde_json::to_value(&network).expect("the model writes");
        let record: Value = serde_json::from_str(RECORD).expect("the record is JSON");
        assert_eq!(written, record, "the model lost or added a field");
    }

    #[test]
    fn concerns_go_worst_first_keeping_their_order_within_a_kind() {
        use ConcernKind::{Busy, Ceiling, Fills, Lost, Slow, Weak};
        let concern = |kind, device: &str| Concern {
            kind,
            device: device.to_owned(),
            on: None,
            other: None,
            says: String::new(),
        };
        let found = vec![
            concern(Busy, "hall"),
            concern(Weak, "tablet"),
            concern(Ceiling, "modem"),
            concern(Busy, "den"),
            concern(Lost, "modem"),
            concern(Slow, "desk"),
            concern(Fills, "laptop"),
        ];
        let order: Vec<(ConcernKind, String)> = worst_first(found)
            .into_iter()
            .map(|concern| (concern.kind, concern.device))
            .collect();
        let want: Vec<(ConcernKind, String)> = [
            (Lost, "modem"),
            (Fills, "laptop"),
            (Slow, "desk"),
            (Busy, "hall"),
            (Busy, "den"),
            (Weak, "tablet"),
            (Ceiling, "modem"),
        ]
        .into_iter()
        .map(|(kind, device)| (kind, device.to_owned()))
        .collect();
        assert_eq!(order, want, "the widest reach comes first");
    }

    #[test]
    fn the_record_names_its_schema() {
        let network: Network = serde_json::from_str(RECORD).expect("the record reads");
        assert_eq!(network.schema, SCHEMA, "the record names another schema");
    }
}
