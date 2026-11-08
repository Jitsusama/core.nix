//! The page `warp` prints for a person: how the link stands in its head,
//! then every figure the record holds, each named first and in its own
//! unit, the labels lined up so the figures start in one column. A figure
//! warp-cli didn't give is a dash, never a guess.

use std::fmt::Write as _;

use serde_json::Value;

use crate::model::{Dns, Settings, Status, Tunnel, Warp};

/// What a figure warp-cli didn't give reads as.
const UNKNOWN: &str = "\u{2013}";
/// What sets two parts of a line apart.
const AND: &str = " \u{b7} ";

pub fn page(warp: &Warp) -> String {
    let mut out = format!("WARP{AND}{}\n", head(&warp.status));
    match &warp.tunnel {
        Some(tunnel) => block(&mut out, None, &tunnel_lines(tunnel)),
        None => block(
            &mut out,
            None,
            &[("tunnel", unknown("warp-cli gave no stats for it"))],
        ),
    }
    if let Some(tls) = warp.tunnel.as_ref().and_then(|tunnel| tunnel.tls.as_ref()) {
        block(
            &mut out,
            Some("TLS"),
            &[
                ("version", maybe(tls.version.clone())),
                ("curve", maybe(tls.curve.clone())),
                ("cipher", maybe(tls.cipher.clone())),
                ("post-quantum", switch(tls.post_quantum)),
                ("server name", maybe(tls.sni.clone())),
            ],
        );
    }
    match &warp.settings {
        Some(settings) => block(&mut out, Some("settings"), &settings_lines(settings)),
        None => lone(&mut out, "settings", &unknown("warp-cli gave none")),
    }
    match &warp.dns {
        Some(dns) => block(&mut out, Some("DNS"), &dns_lines(dns)),
        None => lone(&mut out, "DNS", &unknown("warp-cli gave no counts")),
    }
    out
}

/// How the link stands, in words: its state, why, and any figures warp
/// gave with why.
fn head(status: &Status) -> String {
    let mut parts = vec![status.state.to_lowercase()];
    parts.extend(status.reason.as_deref().map(spoken));
    if let Some(figures) = &status.reason_figures {
        parts.extend(
            figures
                .iter()
                .filter_map(|(key, value)| measured(key, value)),
        );
    }
    parts.join(AND)
}

/// warp-cli's word as a person says it: `NetworkHealthy` is "network
/// healthy".
fn spoken(word: &str) -> String {
    let mut said = String::new();
    for (at, letter) in word.chars().enumerate() {
        if letter.is_uppercase() && at > 0 {
            said.push(' ');
        }
        said.extend(letter.to_lowercase());
    }
    said.replace('_', " ")
}

/// One of the figures a reason carries, by the unit its key ends in:
/// `loss_percent` is "5.37% loss", `rtt_ms` "170 ms round trip".
fn measured(key: &str, value: &Value) -> Option<String> {
    let number = value.as_f64()?;
    let (name, unit) = key.rsplit_once('_')?;
    let name = match name {
        "rtt" => "round trip".to_string(),
        other => spoken(other),
    };
    Some(match unit {
        "percent" => format!("{number:.2}% {name}"),
        "ms" => format!("{} ms {name}", trimmed(number)),
        other => format!("{} {other} {name}", trimmed(number)),
    })
}

fn tunnel_lines(tunnel: &Tunnel) -> Vec<(&'static str, String)> {
    let mut lines = vec![];
    if tunnel.on == Some(false) {
        lines.push(("tunnel", "off".to_string()));
    }
    lines.extend([
        (
            "latency",
            maybe(tunnel.latency_ms.map(|ms| format!("{ms} ms, estimated"))),
        ),
        (
            "loss",
            maybe(
                tunnel
                    .loss_fraction
                    .map(|loss| format!("{:.2}%, estimated", loss * 100.0)),
            ),
        ),
        ("protocol", maybe(tunnel.protocol.clone())),
        (
            "colo",
            maybe(tunnel.edge.as_ref().and_then(|edge| {
                let colo = edge.colo.clone()?;
                Some(match &edge.metal {
                    Some(metal) => format!("{colo}{AND}metal {metal}"),
                    None => colo,
                })
            })),
        ),
        (
            "endpoints",
            format!(
                "{}{AND}IPv6 {}",
                tunnel.v4_endpoint.as_deref().unwrap_or("IPv4 none"),
                tunnel.v6_endpoint.as_deref().unwrap_or("none")
            ),
        ),
        (
            "last handshake",
            maybe(tunnel.last_handshake_s.map(|s| format!("{} ago", ago(s)))),
        ),
        ("sent", maybe(tunnel.sent_bytes.map(bytes))),
        ("received", maybe(tunnel.received_bytes.map(bytes))),
    ]);
    lines
}

/// What each mode does, as warp-cli's own help says it.
fn mode(name: &str) -> Option<&'static str> {
    Some(match name {
        "warp" => "the tunnel, with DNS proxied over UDP",
        "doh" => "DNS over HTTPS alone, no tunnel",
        "warp+doh" => "the tunnel, with DNS over HTTPS",
        "dot" => "DNS over TLS alone, no tunnel",
        "warp+dot" => "the tunnel, with DNS over TLS",
        "proxy" => "the tunnel, as a SOCKS5 proxy",
        "tunnel_only" => "the tunnel, DNS left alone",
        _ => return None,
    })
}

fn settings_lines(settings: &Settings) -> Vec<(&'static str, String)> {
    let by = |key: &str, said: String| match settings.set_by.get(key) {
        Some(source) if said != UNKNOWN => format!("{said}{AND}by {}", spoken(source)),
        _ => said,
    };
    vec![
        (
            "mode",
            by(
                "mode",
                maybe(settings.mode.as_deref().map(|name| match mode(name) {
                    Some(does) => format!("{name}{AND}{does}"),
                    None => name.to_string(),
                })),
            ),
        ),
        ("always on", by("always_on", switch(settings.always_on))),
        (
            "switch locked",
            by("switch_locked", switch(settings.switch_locked)),
        ),
        (
            "auto-connect after",
            by(
                "auto_connect_after_min",
                maybe(
                    settings
                        .auto_connect_after_min
                        .map(|min| format!("{min} min")),
                ),
            ),
        ),
        (
            "organisation",
            by("organisation", maybe(settings.organisation.clone())),
        ),
        (
            "tunnel protocol",
            by("tunnel_protocol", maybe(settings.tunnel_protocol.clone())),
        ),
        (
            "post-quantum",
            by(
                "post_quantum",
                maybe(settings.post_quantum.as_deref().map(spoken)),
            ),
        ),
        (
            "split tunnel",
            by(
                "split_tunnel",
                maybe(settings.split_tunnel.as_ref().map(|split| {
                    format!(
                        "{}{AND}{}{AND}{}",
                        split.mode.as_deref().unwrap_or(UNKNOWN),
                        counted(split.hosts, "host", "hosts"),
                        counted(split.ips, "network", "networks")
                    )
                })),
            ),
        ),
    ]
}

fn dns_lines(dns: &Dns) -> Vec<(&'static str, String)> {
    let count = |value: Option<u64>| maybe(value.map(|value| value.to_string()));
    vec![
        ("proxy", switch(dns.proxy_enabled)),
        ("asked", count(dns.total)),
        ("answered", count(dns.success)),
        ("no record", count(dns.no_records_found)),
        ("timed out", count(dns.timed_out)),
        ("failed", count(dns.other_error)),
        (
            "average",
            maybe(dns.average_ms.map(|ms| format!("{ms:.1} ms"))),
        ),
    ]
}

/// Lines of a part, under its name when it has one and indented then,
/// each label padded so the figures start in one column; a blank line
/// before it sets it apart from the one above.
fn block(out: &mut String, title: Option<&str>, lines: &[(&str, String)]) {
    let indent = if title.is_some() { "  " } else { "" };
    let width = lines
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);
    out.push('\n');
    if let Some(title) = title {
        let _ = writeln!(out, "{title}");
    }
    for (label, said) in lines {
        let _ = writeln!(out, "{indent}{label:width$}  {said}");
    }
}

/// A part warp-cli gave nothing of, said under its name.
fn lone(out: &mut String, title: &str, said: &str) {
    let _ = write!(out, "\n{title}\n  {said}\n");
}

fn unknown(why: &str) -> String {
    format!("{UNKNOWN}, {why}")
}

fn maybe(value: Option<String>) -> String {
    value.unwrap_or_else(|| UNKNOWN.to_string())
}

fn switch(value: Option<bool>) -> String {
    maybe(value.map(|on| if on { "on" } else { "off" }.to_string()))
}

fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// A figure without the fraction it doesn't have: 170, but 170.5.
fn trimmed(number: f64) -> String {
    let said = format!("{number:.2}");
    said.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Bytes in the powers of ten networks count in, to a tenth: 9.3 GB.
fn bytes(count: u64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e12, "TB"), (1e9, "GB"), (1e6, "MB"), (1e3, "kB")];
    #[expect(clippy::cast_precision_loss, reason = "a size shown to a tenth")]
    let size = count as f64;
    UNITS.iter().find(|(scale, _)| size >= *scale).map_or_else(
        || format!("{count} B"),
        |(scale, unit)| format!("{:.1} {unit}", size / scale),
    )
}

/// How long, to the two units that say it: 45 s, 3 min 5 s, 2 h 25 min.
fn ago(seconds: u64) -> String {
    let (hours, minutes) = (seconds / 3600, seconds % 3600 / 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds} s"),
        (0, _) => format!("{minutes} min {} s", seconds % 60),
        _ => format!("{hours} h {minutes} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ask::Said;
    use crate::contract::{Contract, said};
    use crate::read::read;

    /// The page held to its golden one in golden/, or written there when
    /// `WARP_GOLDEN=write`.
    fn golden(name: &str, warp: &Warp) {
        let path = format!("{}/src/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let drawn = page(warp);
        if std::env::var("WARP_GOLDEN").as_deref() == Ok("write") {
            std::fs::write(&path, &drawn).expect("the golden page is written");
            return;
        }
        let kept = std::fs::read_to_string(&path).expect("the golden page is there");
        for (at, (drew, held)) in drawn.lines().zip(kept.lines()).enumerate() {
            assert_eq!(drew, held, "line {} of {name}.txt differs", at + 1);
        }
        assert_eq!(
            drawn.lines().count(),
            kept.lines().count(),
            "{name}.txt's length differs"
        );
        assert_eq!(drawn, kept);
    }

    #[test]
    fn the_whole_record_reads_as_its_golden_page() {
        golden("connected", &read(&Contract::whole()).expect("it reads"));
    }

    #[test]
    fn a_degraded_network_is_said_in_the_head_with_what_warp_measured() {
        let cli = Contract::whole().with("status", said(true, "status.degraded.json"));
        golden("degraded", &read(&cli).expect("it reads"));
    }

    #[test]
    fn a_link_down_with_nothing_else_given_says_each_part_is_unknown() {
        let cli = Contract::none().with(
            "status",
            Ok(Said {
                ok: true,
                text: r#"{"status": "Disconnected", "reason": "Manual"}"#.into(),
            }),
        );
        golden("disconnected", &read(&cli).expect("it reads"));
    }
}
