//! WARP as one record: how its link stands, the tunnel's figures, the
//! settings that shape it and its DNS proxy's counts, each figure in the
//! unit its key names. A figure warp-cli didn't give is `None`, written
//! `null`, never a guess; a part it couldn't give at all, as the tunnel's
//! while it's off, is `None` whole. `warp.json` beside this crate is the
//! record drawn from `contract/`, which the tests hold `read` to and the
//! shell's own cases read.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, PartialEq, Serialize)]
pub struct Warp {
    pub status: Status,
    pub tunnel: Option<Tunnel>,
    pub settings: Option<Settings>,
    pub dns: Option<Dns>,
    /// An administrator's override, which unlocks a held switch for a
    /// while, as `warp-cli override show` gives it.
    pub r#override: Option<Override>,
}

/// Whether an override is in force, and the seconds it has left.
#[derive(Debug, PartialEq, Serialize)]
pub struct Override {
    pub set: Option<bool>,
    pub ends_in_secs: Option<u64>,
}

/// How the link stands, in warp-cli's own words: its state
/// (`Connected`) and why (`NetworkHealthy`). A reason warp-cli gives
/// with figures of its own, as `NetworkDegraded` gives the loss and round
/// trip it measured, keeps them under warp-cli's own keys, which carry
/// their units (`loss_percent`, `rtt_ms`).
#[derive(Debug, PartialEq, Serialize)]
pub struct Status {
    pub state: String,
    pub reason: Option<String>,
    pub reason_figures: Option<Map<String, Value>>,
}

/// The tunnel's figures, as `warp-cli tunnel stats` gives them. An
/// endpoint warp-cli gives as the unspecified address (`::`), as it does
/// for IPv6 on a network without it, is none.
#[derive(Debug, PartialEq, Serialize)]
pub struct Tunnel {
    pub on: Option<bool>,
    pub protocol: Option<String>,
    pub v4_endpoint: Option<String>,
    pub v6_endpoint: Option<String>,
    pub last_handshake_s: Option<u64>,
    pub sent_bytes: Option<u64>,
    pub received_bytes: Option<u64>,
    pub latency_ms: Option<u64>,
    /// WARP's estimate as it gives it, a fraction: 0.0124 is 1.24%.
    pub loss_fraction: Option<f64>,
    pub tls: Option<Tls>,
    pub edge: Option<Edge>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Tls {
    pub version: Option<String>,
    pub curve: Option<String>,
    pub cipher: Option<String>,
    pub post_quantum: Option<bool>,
    pub sni: Option<String>,
}

/// The data centre the tunnel ends in, by its airport code, and
/// the server there.
#[derive(Debug, PartialEq, Serialize)]
pub struct Edge {
    pub colo: Option<String>,
    pub metal: Option<String>,
}

/// The settings that shape the link, each with who set it, in warp-cli's
/// words (`default`, `local_policy`, `network_policy`, `override`), by
/// the setting's key here.
#[derive(Debug, PartialEq, Serialize)]
pub struct Settings {
    pub mode: Option<String>,
    pub always_on: Option<bool>,
    pub switch_locked: Option<bool>,
    pub auto_connect_after_min: Option<u64>,
    pub organisation: Option<String>,
    pub tunnel_protocol: Option<String>,
    pub post_quantum: Option<String>,
    /// The server names are looked up at over `DoH`, and the addresses
    /// it's found at.
    pub doh_hostname: Option<String>,
    pub doh_resolver_ips: Vec<String>,
    /// The domains looked up on the network's own servers instead.
    pub fallback_domains: Vec<FallbackDomain>,
    pub captive_portal: Option<CaptivePortal>,
    /// Whether it finds the largest packet the way carries.
    pub enable_pmtud: Option<bool>,
    pub allow_mode_switch: Option<bool>,
    /// Whether this machine may leave the organisation.
    pub allowed_to_leave: Option<bool>,
    /// Where the organisation says to ask for help, none when it names none.
    pub support_url: Option<String>,
    /// The prefix of the largest local network an override lets it reach.
    pub lan_subnet_size: Option<u64>,
    /// Whether the organisation has disconnected every device at once.
    pub emergency_disconnect: Option<bool>,
    pub split_tunnel: Option<SplitTunnel>,
    pub set_by: BTreeMap<String, String>,
}

/// A domain looked up on the network's own servers, which ones, and what
/// it's for.
#[derive(Debug, PartialEq, Serialize)]
pub struct FallbackDomain {
    pub domain: String,
    pub description: Option<String>,
    pub dns_servers: Vec<String>,
}

/// How WARP meets a sign-in page: whether it looks for one, and how long
/// it opens its firewall for one to be signed in to.
#[derive(Debug, PartialEq, Serialize)]
pub struct CaptivePortal {
    pub detection: Option<bool>,
    pub firewall_off_s: Option<u64>,
}

/// Which traffic the tunnel takes: `include`, only the hosts and networks
/// listed, or `exclude`, everything but them, and how many of each.
#[derive(Debug, PartialEq, Serialize)]
pub struct SplitTunnel {
    pub mode: Option<String>,
    pub hosts: usize,
    pub ips: usize,
    /// Each domain and network, as warp-cli lists them, with what it's for.
    pub host_list: Vec<Listed>,
    pub ip_list: Vec<Listed>,
}

/// A domain or network listed, and what it's for, none where it says nothing.
#[derive(Debug, PartialEq, Serialize)]
pub struct Listed {
    pub value: String,
    pub description: Option<String>,
}

/// What WARP's DNS proxy has answered on this connection.
#[derive(Debug, PartialEq, Serialize)]
pub struct Dns {
    pub proxy_enabled: Option<bool>,
    pub average_ms: Option<f64>,
    pub total: Option<u64>,
    pub success: Option<u64>,
    pub no_records_found: Option<u64>,
    pub timed_out: Option<u64>,
    pub other_error: Option<u64>,
}
