//! The home network as one record, the shape `home-network.json` holds:
//! what the daemon keeps for the shell and what the CLI's views read.
//!
//! Every figure is one eero gives, in the unit its key names; one eero
//! leaves out is `None`, never a guess. Times are RFC 3339 in UTC, as
//! eero's own are once their offsets are read.
//!
//! What eero gives only as it stands now, the daemon keeps (history/),
//! so the model also carries each figure that moves over the last hour,
//! as a series: a sample a read, oldest first, each series's lists as long
//! as its `at`, so a figure is found by its sample's place in them. A key
//! for what the history holds defaults to empty, so a model written
//! before there was one still reads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeNetwork {
    /// When eero last answered.
    pub read: String,
    pub cloud: Cloud,
    pub name: String,
    pub internet: Internet,
    pub nodes: Vec<Node>,
    /// The devices on now, by node, the strongest signal first.
    pub devices: Vec<Device>,
    /// The devices eero knows that aren't on now, the last on first.
    #[serde(default)]
    pub devices_off: Vec<DeviceOff>,
    pub profiles: Vec<Profile>,
    /// The app's activity list, newest first.
    pub events: Vec<Event>,
    /// The devices that joined or left the home network in the last day,
    /// newest first, at the read that saw it, at most 50.
    #[serde(default)]
    pub joins_and_leaves: Vec<JoinOrLeave>,
    /// The changes the daemon told as notices in the last 30 days, newest
    /// first, at most 50, each in the words it said. An event from eero's
    /// activity list it told is left out, since `events` has it in eero's
    /// own.
    #[serde(default)]
    pub told: Vec<ToldChange>,
}

/// Whether eero's cloud is answering, and since when it's been so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cloud {
    pub state: CloudState,
    pub since: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloudState {
    Answering,
    Unreachable,
    SignedOut,
}

/// The home network's way out, and the settings that shape it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "the fields are the model's keys, and backup internet is eero's name for it"
)]
pub struct Internet {
    /// eero's word for it: connected, disconnected.
    pub state: Option<String>,
    pub isp_up: Option<bool>,
    /// The nodes' network as a whole, as eero sees it.
    pub mesh: Option<String>,
    pub isp: Option<String>,
    pub asn: Option<u32>,
    pub public_address: Option<String>,
    pub double_nat: Option<bool>,
    pub gateway_address: Option<String>,
    pub wan_type: Option<String>,
    pub lease: Lease,
    /// NAT or bridge.
    pub mode: Option<String>,
    pub dhcp: Option<String>,
    pub dns: Dns,
    pub ipv6: Option<bool>,
    pub upnp: Option<bool>,
    pub nat_port_randomization: Option<bool>,
    /// Optimize for Conferencing and Gaming.
    pub sqm: Option<bool>,
    pub band_steering: Option<bool>,
    pub wpa3: Option<bool>,
    pub wireless_mode: Option<String>,
    pub mlo_mode: Option<String>,
    pub speed_test: SpeedTest,
    /// Every speed test eero kept, and those the history kept from the
    /// last 30 days that eero has since dropped, newest first.
    pub speed_tests: Vec<SpeedTest>,
    /// Since when a speed test this daemon asked of eero has run without
    /// its figures (daemon.rs's testing); none while none runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub testing_since: Option<String>,
    pub firmware: Firmware,
    pub thread: Thread,
    pub guest: Guest,
    /// How the network hands out addresses, and what it keeps for whom.
    #[serde(default)]
    pub addresses: Addresses,
    /// Each band's Wi-Fi security, as eero names it: wpa2, wpa3.
    #[serde(default)]
    pub security: Vec<BandSecurity>,
    /// Whether the network keeps to what older Wi-Fi 5 (802.11ac) devices can join.
    #[serde(default)]
    pub ac_compatibility: Option<bool>,
    /// Whether eero's last scan saw another network with this one's name.
    #[serde(default)]
    pub name_clash: Option<bool>,
    pub ddns: Option<bool>,
    pub backup_internet: Option<bool>,
    pub power_saving: Option<bool>,
    /// What eero recorded, as it said it: no outage has been seen yet to
    /// give its shape a type.
    pub outages: Vec<Value>,
    pub nodes: NodeCounts,
    /// The devices eero counts.
    pub devices: Option<u32>,
    pub usage: Option<Usage>,
    /// This machine's own last look at the internet, which goes on while
    /// eero's cloud can't be reached.
    #[serde(default)]
    pub probe: Option<Probe>,
    /// What the history keeps of the internet.
    #[serde(default)]
    pub history: InternetHistory,
}

/// This machine's own look at the internet (probe.rs), taken with every
/// read of the network and every 10 seconds while eero's cloud can't be
/// reached: TCP connects timed, since a ping needs root, and a DNS query
/// of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Probe {
    pub at: String,
    /// Whether this machine was on the home network, so that what it saw
    /// was the home network's internet: the gateway node is timed only
    /// then, and an outage recorded.
    pub home: bool,
    /// A connect to the gateway node.
    pub gateway_ms: Option<f64>,
    /// The quicker connect of two public resolvers'.
    pub internet_ms: Option<f64>,
    /// The gateway node resolving a name.
    pub dns_ms: Option<f64>,
    /// Whether either public resolver took the connection.
    pub reachable: bool,
}

/// What the history keeps of the internet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InternetHistory {
    /// This machine's looks over the last hour, a sample a look, oldest
    /// first.
    pub probes: ProbeHistory,
    /// Every outage of the last 30 days, newest first.
    pub outages: Vec<Outage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProbeHistory {
    pub at: Vec<String>,
    pub gateway_ms: Vec<Option<f64>>,
    pub internet_ms: Vec<Option<f64>>,
    pub dns_ms: Vec<Option<f64>>,
    pub reachable: Vec<bool>,
}

/// The internet out, as eero recorded it or this machine's probe saw it
/// from the home network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outage {
    pub seen_by: OutageSeenBy,
    /// Seen by the probe, the first look that found the internet out.
    /// eero's record says when, in a shape no outage has shown yet, so
    /// one eero records gives `null`, and its record as eero wrote it.
    pub start: Option<String>,
    /// Seen by the probe, the first look that found it back, `null`
    /// while it lasts; when nothing looked for over 10 minutes before
    /// that, the last look that found it out, since when it came back
    /// isn't known.
    pub end: Option<String>,
    pub eero: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutageSeenBy {
    Eero,
    Probe,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub mode: Option<String>,
    pub address: Option<String>,
    pub mask: Option<String>,
    pub router: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dns {
    pub mode: Option<String>,
    pub servers: Vec<String>,
    pub caching: Option<bool>,
    pub ipv6_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeedTest {
    pub at: Option<String>,
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Firmware {
    pub running: Option<String>,
    pub next: Option<String>,
    pub has_update: Option<bool>,
    pub can_update_now: Option<bool>,
    /// The hour of the day eero updates at, in the network's time.
    pub update_hour: Option<u8>,
    pub last_started: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub on: Option<bool>,
    pub name: Option<String>,
    pub channel: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guest {
    pub on: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Addresses {
    /// The main network first, then the guest network, as eero lists them.
    pub subnets: Vec<Subnet>,
    /// The addresses kept for a device, in eero's order.
    pub reservations: Vec<Reservation>,
    /// What eero said of each port forward and IPv6 pinhole: none has been
    /// seen yet to give their shapes a type.
    pub forwards: Vec<Value>,
    pub pinholes: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subnet {
    /// eero's word for it: main, guest.
    pub kind: Option<String>,
    pub name: Option<String>,
    pub on: Option<bool>,
    pub address: Option<String>,
    pub mask: Option<String>,
    /// The first and last addresses DHCP hands out.
    pub first: Option<String>,
    pub last: Option<String>,
    /// Whether its devices reach the internet.
    pub internet: Option<bool>,
    /// Whether it asks no password.
    pub open: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    pub mac: String,
    /// What the reservation is called in the eero app.
    pub description: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandSecurity {
    /// The band as the views say it: 2.4 GHz, 5 GHz, 6 GHz.
    pub band: String,
    pub security: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeCounts {
    pub online: Option<u32>,
    pub offline: Option<u32>,
    pub pending: Option<u32>,
}

/// Bytes by hour, each way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub download: Series,
    pub upload: Series,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub total_bytes: u64,
    pub hours: Vec<Hour>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hour {
    pub at: String,
    pub bytes: u64,
}

/// One eero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub location: Option<String>,
    pub model: Option<String>,
    pub model_number: Option<String>,
    pub serial: Option<String>,
    /// The node on the internet.
    pub gateway: bool,
    pub state: Option<String>,
    pub heartbeat_ok: Option<bool>,
    pub last_heartbeat: Option<String>,
    pub backhaul: Backhaul,
    pub address: Option<String>,
    pub ipv6: Vec<String>,
    pub mac: Option<String>,
    pub firmware: Option<String>,
    pub update_available: Option<bool>,
    pub support_ends: Option<String>,
    pub joined: Option<String>,
    pub last_reboot: Option<String>,
    pub up_s: Option<u64>,
    /// How long it's been on eero's cloud since it last joined it.
    pub cloud_s: Option<u64>,
    pub devices: NodeDevices,
    pub radios: Vec<Radio>,
    pub ports: Vec<Port>,
    pub light: Option<Light>,
    pub power: Option<String>,
    pub usage: Option<Usage>,
    /// Its backhaul's bars over the last hour.
    #[serde(default)]
    pub history: NodeHistory,
}

/// A node's backhaul over the last hour, a sample a read, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeHistory {
    pub at: Vec<String>,
    pub bars: Vec<Option<u8>>,
}

/// How a node reaches the one before it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Backhaul {
    pub kind: BackhaulKind,
    /// Over Wi-Fi, the location of the node it hangs from: eero names it
    /// so, since its id is in another numbering than the nodes'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// Over Wi-Fi, the radio it uses, as eero says it: 5.8GHz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio: Option<String>,
    pub bars: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackhaulKind {
    Ethernet,
    WiFi,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDevices {
    pub all: Option<u32>,
    pub ethernet: Option<u32>,
    pub wifi: Option<u32>,
}

/// One of a node's radios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Radio {
    /// 2.4 GHz, 5 GHz low, 5 GHz high, 6 GHz.
    pub band: String,
    pub channel: Option<u16>,
    pub width_mhz: Option<u16>,
    pub power_dbm: Option<i16>,
    /// How much of the time the channel is busy.
    pub busy_pct: Option<u8>,
    pub devices: Option<u32>,
    pub bssid: Option<String>,
    /// Its channel, how busy and its power over the last hour.
    #[serde(default)]
    pub history: RadioHistory,
}

/// A radio over the last hour, a sample a read, oldest first: a channel
/// changing within one is the radio moving off a busy or radar-struck
/// one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioHistory {
    pub at: Vec<String>,
    pub channel: Vec<Option<u16>>,
    pub busy_pct: Vec<Option<u8>>,
    pub power_dbm: Vec<Option<i16>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub linked: bool,
    pub mbps: Option<u32>,
    /// The fastest it can link at.
    pub best_mbps: Option<u32>,
    pub wan: bool,
    /// Whether the node's backhaul runs through it.
    pub upstream: bool,
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub on: Option<bool>,
    pub brightness: Option<u8>,
}

/// A device on the home network now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub mac: String,
    /// Whether it's the machine the daemon runs on: one of that machine's
    /// own hardware addresses is this one (`read::mark_this_machine`).
    #[serde(default)]
    pub this_machine: bool,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub hostname: Option<String>,
    /// eero's word for what it is: `phone`, `laptop_computer`, `printer`.
    pub kind: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    /// Whether its hardware address is a private one it made up.
    pub private: Option<bool>,
    pub address: Option<String>,
    pub addresses: Vec<String>,
    pub first_seen: Option<String>,
    pub first_active: Option<String>,
    pub last_active: Option<String>,
    /// The id of the node it's on.
    pub node: Option<String>,
    pub ethernet: bool,
    pub ssid: Option<String>,
    pub auth: Option<String>,
    pub subnet: Option<String>,
    pub guest: Option<bool>,
    /// 2.4 GHz, 5 GHz or 6 GHz, from its frequency.
    pub band: Option<String>,
    pub channel: Option<u16>,
    pub frequency_mhz: Option<u32>,
    pub bssid: Option<String>,
    pub signal_dbm: Option<i16>,
    pub snr_db: Option<u16>,
    /// eero's own score of the link, 0 to 1, and as bars of 5.
    pub score: Option<f64>,
    pub bars: Option<u8>,
    /// What it receives at, and what it sends at.
    pub rx: Option<Rate>,
    pub tx: Option<Rate>,
    pub retry_pct: Option<f64>,
    pub idle_ms: Option<u64>,
    pub packets: Option<Packets>,
    /// Over Ethernet, the node's port and its speed.
    pub port: Option<DevicePort>,
    /// How fast it moves data now, each way.
    pub speed: Speed,
    /// The id of its profile.
    pub profile: Option<String>,
    pub paused: Option<bool>,
    pub blocked: Option<bool>,
    /// Its signal, rates and speed over the last hour.
    #[serde(default)]
    pub history: DeviceHistory,
    /// Each time it moved to another node or band in the last day, newest
    /// first, at most 20.
    #[serde(default)]
    pub roams: Vec<Roam>,
}

/// A device eero knows that isn't on now: what stays true of it while it's
/// off. eero keeps its last link's figures too, but they're of a link
/// that's gone, so they're left out rather than read as now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceOff {
    pub mac: String,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub hostname: Option<String>,
    pub kind: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    pub private: Option<bool>,
    pub first_seen: Option<String>,
    /// When it was last on.
    pub last_active: Option<String>,
    /// The id of the node it was last on, and how.
    pub node: Option<String>,
    pub ethernet: bool,
    pub band: Option<String>,
    /// The id of its profile.
    pub profile: Option<String>,
}

/// A device's link over the last hour, a sample a read, oldest first:
/// `null` where eero gave no figure at that read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceHistory {
    pub at: Vec<String>,
    pub signal_dbm: Vec<Option<i16>>,
    pub snr_db: Vec<Option<u16>>,
    /// The rates it receives and sends at.
    pub rx_mbps: Vec<Option<f64>>,
    pub tx_mbps: Vec<Option<f64>>,
    /// Its speed each way.
    pub down_mbps: Vec<Option<f64>>,
    pub up_mbps: Vec<Option<f64>>,
}

/// A device moving to another node or band, at the read that saw it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roam {
    pub at: String,
    /// The ids of the nodes it left and went to, either the same when
    /// it only changed band.
    pub from_node: Option<String>,
    pub node: Option<String>,
    pub from_band: Option<String>,
    pub band: Option<String>,
}

/// A device joining or leaving the home network, at the read that saw it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinOrLeave {
    pub at: String,
    pub kind: JoinOrLeaveKind,
    pub mac: String,
    /// Its name when last seen, since one that left is no longer among
    /// the devices.
    pub name: Option<String>,
    /// The node and band it joined on, or left from.
    pub node: Option<String>,
    pub band: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JoinOrLeaveKind {
    Joined,
    Left,
}

/// A link rate, and how the radio makes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    pub mbps: Option<f64>,
    /// How the radio makes it, where eero says: an older client reports
    /// its rate alone.
    pub mcs: Option<u8>,
    pub streams: Option<u8>,
    pub width_mhz: Option<u16>,
    pub guard_ns: Option<u16>,
    /// Wi-Fi 4, 5, 6 or 7.
    pub generation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "the fields are the model's keys, eero's own names for its counts"
)]
pub struct Packets {
    pub rx_drops: Option<u64>,
    pub tx_retries: Option<u64>,
    pub rx_packets: Option<u64>,
    pub tx_packets: Option<u64>,
    pub total_packets: Option<u64>,
    pub tx_retransmit_ppm: Option<u32>,
    pub tx_fail_ppm: Option<u32>,
    pub rx_drop_ppm: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevicePort {
    pub name: Option<String>,
    pub mbps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
}

/// A device a profile lists, as eero's profile names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub mac: String,
    pub name: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
    pub state: Option<String>,
    pub paused: Option<bool>,
    /// Its devices, by hardware address, on now or not.
    pub devices: Vec<String>,
    /// Its devices as the profile names them, in the same order, so one
    /// that isn't on now, which `devices` of the model doesn't hold, is
    /// still named.
    #[serde(default)]
    pub members: Vec<Member>,
    /// Whether it's eero's default profile.
    #[serde(default)]
    pub default: Option<bool>,
    /// How many schedules eero keeps for it. eero's list has held none
    /// yet, so a schedule's own shape has no type.
    #[serde(default)]
    pub schedules: Option<usize>,
    /// What it blocks, and lets through.
    #[serde(default)]
    pub filters: Filters,
    /// Its bytes by hour over the last day.
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// What a profile blocks, and lets through: each kind of content eero
/// filters, whether it's blocked, and the sites and apps named one by one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    /// Whether any content filter is set.
    pub set: Option<bool>,
    pub gaming: Option<bool>,
    pub illegal: Option<bool>,
    pub messaging: Option<bool>,
    pub pornographic: Option<bool>,
    pub social: Option<bool>,
    pub shopping: Option<bool>,
    pub streaming: Option<bool>,
    pub violent: Option<bool>,
    /// Whether search engines are held to their safe results.
    pub safe_search: Option<bool>,
    /// Whether `YouTube` is held to its restricted mode.
    pub youtube_restricted: Option<bool>,
    /// Whether it blocks ads, and whether the whole network does.
    pub ad_block: Option<bool>,
    pub ad_block_network: Option<bool>,
    /// Sites blocked, and let through, by name.
    pub blocked_sites: Vec<String>,
    pub allowed_sites: Vec<String>,
    /// Apps blocked, by eero's name for each.
    pub blocked_apps: Vec<String>,
}

/// A change the daemon told, and what its notice said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToldChange {
    /// When it was told.
    pub at: String,
    /// What it told of, by the history's name for it: `internet-out`,
    /// `internet-back`, `node-offline`, `node-online`, `joined`,
    /// `update-waiting` or `updated`.
    pub kind: String,
    pub summary: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub at: String,
    /// eero's category: Device, Network, eero.
    pub kind: String,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::{MODEL, differences};

    #[test]
    fn the_model_reads_and_writes_back_unchanged() {
        let model: HomeNetwork = serde_json::from_str(MODEL).expect("home-network.json reads");
        let again = serde_json::to_value(&model).unwrap();
        let read: Value = serde_json::from_str(MODEL).unwrap();

        let out = differences(&again, &read);

        assert!(
            out.is_empty(),
            "written back differently:\n{}",
            out.join("\n")
        );
    }
}
