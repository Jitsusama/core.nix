//! What `eero sqm` says once eero reads the setting back: a line a network,
//! straight from eero's answer, since the record may not hold it yet.

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
}
