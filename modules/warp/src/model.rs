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
    pub split_tunnel: Option<SplitTunnel>,
    pub set_by: BTreeMap<String, String>,
}

/// Which traffic the tunnel takes: `include`, only the hosts and networks
/// listed, or `exclude`, everything but them, and how many of each.
#[derive(Debug, PartialEq, Serialize)]
pub struct SplitTunnel {
    pub mode: Option<String>,
    pub hosts: usize,
    pub ips: usize,
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
