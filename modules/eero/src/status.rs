//! What the status says: a line a network for a person, and a file for the
//! shell, the way the shaper and the disk guard keep theirs.

use serde_json::{Value, json};

use crate::api::Network;

/// A network's line for a person.
pub fn line(network: &Network, devices: Option<usize>) -> String {
    let mut parts = vec![network.name.clone()];
    parts.push(
        match network.online {
            Some(true) => "online",
            Some(false) => "offline",
            None => "online unknown",
        }
        .into(),
    );
    if let (Some(down), Some(up)) = (network.down_mbps, network.up_mbps) {
        parts.push(match &network.tested {
            Some(tested) => format!("down {down:.1} up {up:.1} Mbit/s (tested {})", when(tested)),
            None => format!("down {down:.1} up {up:.1} Mbit/s"),
        });
    }
    if let Some(on) = network.sqm {
        parts.push(format!("conferencing {}", if on { "on" } else { "off" }));
    }
    if let Some(count) = devices {
        parts.push(format!(
            "{count} device{}",
            if count == 1 { "" } else { "s" }
        ));
    }
    parts.join("  ")
}

/// A timestamp eero gives, cut to the minute for reading.
fn when(stamp: &str) -> String {
    stamp
        .get(..16)
        .map_or_else(|| stamp.to_owned(), |minute| minute.replace('T', " "))
}

/// The status file's contents for networks read at a moment.
pub fn file(at: u64, networks: &[(Network, Option<usize>)]) -> Value {
    let networks: Vec<Value> = networks
        .iter()
        .map(|(network, devices)| {
            json!({
                "name": network.name,
                "online": network.online,
                "down_mbps": network.down_mbps,
                "up_mbps": network.up_mbps,
                "tested": network.tested,
                "sqm": network.sqm,
                "devices": devices,
            })
        })
        .collect();
    json!({"version": 1, "at": at, "networks": networks})
}

/// The status file's contents when eero couldn't be read.
pub fn failed(at: u64, reason: &str) -> Value {
    json!({"version": 1, "at": at, "networks": [], "error": reason})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> Network {
        Network {
            name: "home".into(),
            url: "/2.2/networks/1".into(),
            online: Some(true),
            down_mbps: Some(76.8),
            up_mbps: Some(12.4),
            tested: Some("2026-10-25T18:00:12.000Z".into()),
            sqm: Some(false),
            devices_url: None,
            settings_url: None,
        }
    }

    #[test]
    fn a_line_says_what_a_person_wants_first() {
        assert_eq!(
            line(&home(), Some(14)),
            "home  online  down 76.8 up 12.4 Mbit/s (tested 2026-10-25 18:00)  \
             conferencing off  14 devices"
        );
    }

    #[test]
    fn a_line_leaves_out_what_eero_did_not_say() {
        let network = Network {
            online: None,
            down_mbps: None,
            sqm: None,
            ..home()
        };
        assert_eq!(line(&network, Some(1)), "home  online unknown  1 device");
    }

    #[test]
    fn the_file_holds_each_network_and_when_it_was_read() {
        let file = file(1_792_970_000, &[(home(), Some(14))]);
        assert_eq!(file["version"], 1);
        assert_eq!(file["at"], 1_792_970_000);
        assert_eq!(file["networks"][0]["name"], "home");
        assert_eq!(file["networks"][0]["online"], true);
        assert_eq!(file["networks"][0]["sqm"], false);
        assert_eq!(file["networks"][0]["devices"], 14);
        assert!(file.get("error").is_none());
    }

    #[test]
    fn a_failed_read_says_why_and_lists_nothing() {
        let file = failed(5, "not signed in");
        assert_eq!(file["error"], "not signed in");
        assert_eq!(file["networks"], json!([]));
    }
}
