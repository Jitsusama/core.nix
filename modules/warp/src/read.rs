//! WARP drawn from what warp-cli says: its status, the tunnel's stats, the
//! settings, the DNS proxy's counts and any override, asked at once, made into the
//! record (model.rs). The status is what says whether WARP is there at
//! all, so a status warp-cli can't give fails the read and says why; any
//! other part it can't give is left out of the record, unknown, as the
//! tunnel's stats are while there's no tunnel.

use std::collections::BTreeMap;
use std::thread;

use serde::Deserialize;
use serde_json::Value;

use crate::ask::{Ask, Said, Unasked};
use crate::model::{
    CaptivePortal, Dns, Edge, FallbackDomain, Listed, Override, Settings, SplitTunnel, Status, Tls,
    Tunnel, Warp,
};

/// Why WARP couldn't be read, each its own exit code (main.rs).
#[derive(Debug, PartialEq)]
pub enum Failure {
    /// There's no warp-cli, so no WARP on this machine to read.
    NotInstalled(String),
    /// WARP's service didn't answer warp-cli, in warp-cli's words.
    NotAnswering(String),
    /// warp-cli said something this can't read.
    Unreadable(String),
}

/// warp-cli's word for a service it couldn't reach.
const NOT_ANSWERING: &str = "FailedToConnectToDaemon";

/// The questions, asked at once: each is a round trip to WARP's service,
/// about 20 ms apiece on a laptop, so asked in turn they'd take five times
/// as long for nothing.
const STATUS: &[&str] = &["status"];
const TUNNEL: &[&str] = &["tunnel", "stats"];
const SETTINGS: &[&str] = &["settings"];
const DNS: &[&str] = &["dns", "stats"];
const OVERRIDE: &[&str] = &["override", "show"];

pub fn read(cli: &impl Ask) -> Result<Warp, Failure> {
    let [status, tunnel, settings, dns, held_open] = thread::scope(|scope| {
        [STATUS, TUNNEL, SETTINGS, DNS, OVERRIDE]
            .map(|question| scope.spawn(move || cli.ask(question)))
            .map(|asked| asked.join().expect("asking warp-cli doesn't panic"))
    });
    let status = status_of(&answer(STATUS, status)?)?;
    Ok(Warp {
        status,
        tunnel: part(TUNNEL, tunnel).map(tunnel_of),
        settings: part(SETTINGS, settings).and_then(settings_of),
        dns: part(DNS, dns).map(dns_of),
        r#override: part(OVERRIDE, held_open).map(|said: SaidOverride| Override {
            set: said.set,
            ends_in_secs: said.ends_in_secs,
        }),
    })
}

#[derive(Deserialize)]
struct SaidOverride {
    set: Option<bool>,
    ends_in_secs: Option<u64>,
}

/// What warp-cli answered a question as JSON, or why there's no answer.
fn answer(question: &[&str], said: Result<Said, Unasked>) -> Result<Value, Failure> {
    let asked = question.join(" ");
    let said = said.map_err(|unasked| match unasked {
        Unasked::Missing(at) => Failure::NotInstalled(at),
        Unasked::Stuck(why) => Failure::NotAnswering(why),
    })?;
    let value: Value = serde_json::from_str(&said.text).map_err(|_| {
        Failure::Unreadable(format!(
            "warp-cli {asked} said what isn't JSON: {}",
            said.text.trim()
        ))
    })?;
    // warp-cli says an error as { code, error }, and exits with a failure.
    let code = value["code"].as_str();
    if !said.ok || code.is_some() {
        let error = value["error"].as_str().unwrap_or_default().to_string();
        return Err(match code {
            Some(NOT_ANSWERING) => Failure::NotAnswering(error),
            Some(code) => Failure::Unreadable(format!("warp-cli {asked} said {code}: {error}")),
            None => Failure::Unreadable(format!("warp-cli {asked} failed: {}", said.text.trim())),
        });
    }
    Ok(value)
}

/// A part besides the status, which is simply unknown when warp-cli can't
/// give it.
fn part<T: for<'de> Deserialize<'de>>(question: &[&str], said: Result<Said, Unasked>) -> Option<T> {
    serde_json::from_value(answer(question, said).ok()?).ok()
}

/// The reason warp-cli gives, a word (`NetworkHealthy`) or a word holding
/// its figures (`{ "NetworkDegraded": { "loss_percent": 5.37 } }`).
fn status_of(said: &Value) -> Result<Status, Failure> {
    let state = said["status"]
        .as_str()
        .ok_or_else(|| Failure::Unreadable(format!("warp-cli status gave no status: {said}")))?;
    let (reason, reason_figures) = match &said["reason"] {
        Value::String(word) => (Some(word.clone()), None),
        Value::Object(held) if held.len() == 1 => {
            let (word, figures) = held.iter().next().expect("it holds one");
            (Some(word.clone()), figures.as_object().cloned())
        }
        _ => (None, None),
    };
    Ok(Status {
        state: state.to_string(),
        reason,
        reason_figures,
    })
}

#[derive(Deserialize)]
struct SaidTunnel {
    warp_is_on: Option<bool>,
    protocol: Option<String>,
    v4_endpoint: Option<String>,
    v6_endpoint: Option<String>,
    secs_since_last_handshake: Option<u64>,
    bytes_sent: Option<u64>,
    bytes_received: Option<u64>,
    estimated_latency_ms: Option<u64>,
    estimated_loss: Option<f64>,
    tls: Option<SaidTls>,
    edge: Option<SaidEdge>,
}

#[derive(Deserialize)]
struct SaidTls {
    version: Option<String>,
    curve: Option<String>,
    cipher: Option<String>,
    post_quantum_enabled: Option<bool>,
    sni: Option<String>,
}

#[derive(Deserialize)]
struct SaidEdge {
    colo: Option<String>,
    metal: Option<String>,
}

fn tunnel_of(said: SaidTunnel) -> Tunnel {
    Tunnel {
        on: said.warp_is_on,
        protocol: said.protocol,
        v4_endpoint: said.v4_endpoint.and_then(endpoint),
        v6_endpoint: said.v6_endpoint.and_then(endpoint),
        last_handshake_s: said.secs_since_last_handshake,
        sent_bytes: said.bytes_sent,
        received_bytes: said.bytes_received,
        latency_ms: said.estimated_latency_ms,
        loss_fraction: said.estimated_loss,
        tls: said.tls.map(|tls| Tls {
            version: tls.version,
            curve: tls.curve,
            cipher: tls.cipher,
            post_quantum: tls.post_quantum_enabled,
            sni: tls.sni,
        }),
        edge: said.edge.map(|edge| Edge {
            colo: edge.colo,
            metal: edge.metal,
        }),
    }
}

/// An endpoint, unless it's the unspecified address, which is none.
fn endpoint(said: String) -> Option<String> {
    match said.parse::<std::net::IpAddr>() {
        Ok(address) if address.is_unspecified() => None,
        _ if said.is_empty() => None,
        _ => Some(said),
    }
}

#[derive(Deserialize)]
struct SaidSettings {
    #[serde(default)]
    settings: SaidSetting,
    #[serde(default)]
    sources: BTreeMap<String, String>,
}

#[derive(Default, Deserialize)]
struct SaidSetting {
    operation_mode: Option<String>,
    always_on: Option<bool>,
    switch_locked: Option<bool>,
    auto_connect_after_min: Option<u64>,
    organization: Option<String>,
    warp_tunnel_protocol: Option<String>,
    post_quantum_config: Option<String>,
    split_tunnel_mode: Option<String>,
    split_tunnel_hosts: Option<Vec<Value>>,
    split_tunnel_ips: Option<Vec<Value>>,
    doh_hostname: Option<String>,
    doh_resolver_ips: Option<Vec<String>>,
    // Lists and parts read loosely, as values, so one entry warp-cli
    // shapes otherwise loses only itself rather than every setting.
    fallback_domains: Option<Vec<Value>>,
    captive_portal_detection_enabled: Option<bool>,
    captive_portal_firewall_off_secs: Option<u64>,
    enable_pmtud: Option<bool>,
    allow_mode_switch: Option<bool>,
    allowed_to_leave: Option<bool>,
    support_url: Option<String>,
    lan_subnet_size: Option<u64>,
    emergency_disconnect: Option<Value>,
}

/// Text warp-cli gives, none where it gives none or an empty string.
fn given(text: Option<&str>) -> Option<String> {
    text.filter(|text| !text.is_empty()).map(str::to_string)
}

/// Each domain or network a split lists, as `{ value, description }`.
fn listed(entries: Option<&[Value]>) -> Vec<Listed> {
    entries
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            Some(Listed {
                value: entry["value"].as_str()?.to_string(),
                description: given(entry["description"].as_str()),
            })
        })
        .collect()
}

/// Each domain looked up on the network's own servers, with them.
fn fallbacks(entries: Option<&[Value]>) -> Vec<FallbackDomain> {
    entries
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            Some(FallbackDomain {
                domain: entry["domain"].as_str()?.to_string(),
                description: given(entry["description"].as_str()),
                dns_servers: entry["dns_servers"]
                    .as_array()
                    .map(|servers| {
                        servers
                            .iter()
                            .filter_map(|server| server.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Each setting's key here, and warp-cli's for it, whose source says who
/// set it.
const SOURCES: [(&str, &str); 8] = [
    ("mode", "operation_mode"),
    ("always_on", "always_on"),
    ("switch_locked", "switch_locked"),
    ("auto_connect_after_min", "auto_connect_after_min"),
    ("organisation", "organization"),
    ("tunnel_protocol", "warp_tunnel_protocol"),
    ("post_quantum", "post_quantum_config"),
    ("split_tunnel", "split_tunnel_mode"),
];

fn settings_of(said: SaidSettings) -> Option<Settings> {
    let setting = said.settings;
    let split_tunnel = setting.split_tunnel_mode.is_some()
        || setting.split_tunnel_hosts.is_some()
        || setting.split_tunnel_ips.is_some();
    let portal = setting.captive_portal_detection_enabled.is_some()
        || setting.captive_portal_firewall_off_secs.is_some();
    let settings = Settings {
        mode: setting.operation_mode,
        always_on: setting.always_on,
        switch_locked: setting.switch_locked,
        auto_connect_after_min: setting.auto_connect_after_min,
        organisation: setting.organization,
        tunnel_protocol: setting.warp_tunnel_protocol,
        post_quantum: setting.post_quantum_config,
        doh_hostname: given(setting.doh_hostname.as_deref()),
        doh_resolver_ips: setting.doh_resolver_ips.unwrap_or_default(),
        fallback_domains: fallbacks(setting.fallback_domains.as_deref()),
        captive_portal: portal.then_some(CaptivePortal {
            detection: setting.captive_portal_detection_enabled,
            firewall_off_s: setting.captive_portal_firewall_off_secs,
        }),
        enable_pmtud: setting.enable_pmtud,
        allow_mode_switch: setting.allow_mode_switch,
        allowed_to_leave: setting.allowed_to_leave,
        support_url: given(setting.support_url.as_deref()),
        lan_subnet_size: setting.lan_subnet_size,
        emergency_disconnect: setting
            .emergency_disconnect
            .as_ref()
            .and_then(|emergency| emergency["disconnect"].as_bool()),
        split_tunnel: split_tunnel.then(|| SplitTunnel {
            mode: setting.split_tunnel_mode,
            hosts: setting.split_tunnel_hosts.as_ref().map_or(0, Vec::len),
            ips: setting.split_tunnel_ips.as_ref().map_or(0, Vec::len),
            host_list: listed(setting.split_tunnel_hosts.as_deref()),
            ip_list: listed(setting.split_tunnel_ips.as_deref()),
        }),
        set_by: SOURCES
            .iter()
            .filter_map(|(ours, theirs)| {
                Some(((*ours).to_string(), said.sources.get(*theirs)?.clone()))
            })
            .collect(),
    };
    let empty = Settings {
        mode: None,
        always_on: None,
        switch_locked: None,
        auto_connect_after_min: None,
        organisation: None,
        tunnel_protocol: None,
        post_quantum: None,
        doh_hostname: None,
        doh_resolver_ips: Vec::new(),
        fallback_domains: Vec::new(),
        captive_portal: None,
        enable_pmtud: None,
        allow_mode_switch: None,
        allowed_to_leave: None,
        support_url: None,
        lan_subnet_size: None,
        emergency_disconnect: None,
        split_tunnel: None,
        set_by: BTreeMap::new(),
    };
    (settings != empty).then_some(settings)
}

#[derive(Clone, Copy, Deserialize)]
struct SaidDns {
    dns_proxy_enabled: Option<bool>,
    avg_duration_millis: Option<f64>,
    total: Option<u64>,
    success: Option<u64>,
    no_records_found: Option<u64>,
    timed_out: Option<u64>,
    other_error: Option<u64>,
}

fn dns_of(said: SaidDns) -> Dns {
    Dns {
        proxy_enabled: said.dns_proxy_enabled,
        average_ms: said.avg_duration_millis,
        total: said.total,
        success: said.success,
        no_records_found: said.no_records_found,
        timed_out: said.timed_out,
        other_error: said.other_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contract::{Contract, said};

    fn as_json(warp: &Warp) -> Value {
        serde_json::to_value(warp).expect("the record is JSON")
    }

    fn golden() -> Value {
        serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/warp.json"))
                .expect("warp.json is there"),
        )
        .expect("warp.json is JSON")
    }

    #[test]
    fn every_answer_in_the_contract_reads_as_warp_json_holds_it() {
        let warp = read(&Contract::whole()).expect("the contract reads");
        assert_eq!(as_json(&warp), golden());
    }

    #[test]
    fn a_degraded_network_keeps_the_figures_warp_measured_it_by() {
        let cli = Contract::whole().with("status", said(true, "status.degraded.json"));
        let status = read(&cli).expect("it reads").status;
        assert_eq!(status.state, "Connected");
        assert_eq!(status.reason.as_deref(), Some("NetworkDegraded"));
        let figures = status.reason_figures.expect("it measured the network");
        assert_eq!(figures["loss_percent"], 5.370_822_4);
        assert_eq!(figures["rtt_ms"], 170);
    }

    #[test]
    fn a_service_that_isnt_answering_is_said_in_warp_clis_words() {
        let down = || said(false, "daemon-down.json");
        let cli = Contract::none()
            .with("status", down())
            .with("tunnel stats", down())
            .with("settings", down())
            .with("dns stats", down());
        assert_eq!(
            read(&cli),
            Err(Failure::NotAnswering(
                "No such file or directory (os error 2)".into()
            ))
        );
    }

    #[test]
    fn no_warp_cli_is_no_warp_installed() {
        let missing = || Err(Unasked::Missing("warp-cli".into()));
        let cli = Contract::none()
            .with("status", missing())
            .with("tunnel stats", missing())
            .with("settings", missing())
            .with("dns stats", missing());
        assert_eq!(read(&cli), Err(Failure::NotInstalled("warp-cli".into())));
    }

    #[test]
    fn a_warp_cli_that_hangs_is_a_service_not_answering() {
        let cli = Contract::whole().with(
            "status",
            Err(Unasked::Stuck(
                "warp-cli status didn't answer within 5 s".into(),
            )),
        );
        assert_eq!(
            read(&cli),
            Err(Failure::NotAnswering(
                "warp-cli status didn't answer within 5 s".into()
            ))
        );
    }

    #[test]
    fn a_status_refused_or_unreadable_says_what_warp_cli_said() {
        let refused = Contract::whole().with("status", said(false, "refused.json"));
        assert_eq!(
            read(&refused),
            Err(Failure::Unreadable(
                "warp-cli status said InvalidAccountType: Invalid account type.".into()
            ))
        );
        let garbled = Contract::whole().with(
            "status",
            Ok(Said {
                ok: true,
                text: "Status update: Connected".into(),
            }),
        );
        assert!(
            matches!(read(&garbled), Err(Failure::Unreadable(why)) if why.starts_with("warp-cli status said what isn't JSON")),
        );
    }

    #[test]
    fn a_part_warp_cli_wont_give_is_unknown_and_the_rest_still_reads() {
        let cli = Contract::whole()
            .with("tunnel stats", said(false, "refused.json"))
            .with(
                "settings",
                Ok(Said {
                    ok: true,
                    text: "{}".into(),
                }),
            )
            .with("dns stats", Err(Unasked::Stuck("too slow".into())));
        let warp = read(&cli).expect("the status reads");
        assert_eq!(warp.tunnel, None);
        assert_eq!(warp.dns, None);
        // Settings with nothing in them are settings warp-cli didn't give.
        assert_eq!(warp.settings, None);
        assert_eq!(warp.status.reason.as_deref(), Some("NetworkHealthy"));
    }

    #[test]
    fn an_endpoint_given_as_no_address_is_none() {
        let tunnel = read(&Contract::whole())
            .expect("it reads")
            .tunnel
            .expect("there's a tunnel");
        assert_eq!(tunnel.v6_endpoint, None);
        assert_eq!(tunnel.v4_endpoint.as_deref(), Some("198.51.100.2"));
    }

    #[test]
    fn a_figure_warp_cli_leaves_out_is_unknown_not_zero() {
        let cli = Contract::whole().with(
            "tunnel stats",
            Ok(Said {
                ok: true,
                text: r#"{"warp_is_on": false, "protocol": "WireGuard"}"#.into(),
            }),
        );
        let tunnel = read(&cli)
            .expect("it reads")
            .tunnel
            .expect("it said something");
        assert_eq!(tunnel.on, Some(false));
        assert_eq!(tunnel.protocol.as_deref(), Some("WireGuard"));
        assert_eq!(tunnel.latency_ms, None);
        assert_eq!(tunnel.sent_bytes, None);
        assert_eq!(tunnel.tls, None);
    }

    #[test]
    fn an_override_in_force_says_how_long_it_has_left() {
        let cli = Contract::whole().with(
            "override show",
            Ok(Said {
                ok: true,
                text: r#"{"set": true, "ends_in_secs": 1500}"#.into(),
            }),
        );
        let held = read(&cli)
            .expect("it reads")
            .r#override
            .expect("it said one");
        assert_eq!((held.set, held.ends_in_secs), (Some(true), Some(1500)));
    }

    #[test]
    fn an_override_warp_cli_wont_say_is_unknown() {
        let cli = Contract::whole().with("override show", said(false, "refused.json"));
        assert_eq!(read(&cli).expect("it reads").r#override, None);
    }

    #[test]
    fn an_emergency_disconnect_is_read_from_its_own_switch() {
        let mut said: Value =
            serde_json::from_str(&crate::contract::file("settings.json")).expect("it's JSON");
        said["settings"]["emergency_disconnect"]["disconnect"] = Value::Bool(true);
        let cli = Contract::whole().with(
            "settings",
            Ok(Said {
                ok: true,
                text: said.to_string(),
            }),
        );
        let settings = read(&cli)
            .expect("it reads")
            .settings
            .expect("it has settings");
        assert_eq!(settings.emergency_disconnect, Some(true));
    }
}
