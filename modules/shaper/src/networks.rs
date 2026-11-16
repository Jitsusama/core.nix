//! What each network's line is known to carry, as nixos.nix writes
//! `jitsusama.shaper.networks` for `shaper run --networks`: the one-way
//! controller guards the upload near it (src/oneway.rs).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Each network's declared upload rate, its anchor, in kbit/s, by its
/// router's hardware address, as `jitsusama.shaper.networks` sets them.
pub type Anchors = BTreeMap<String, f64>;

/// The anchors written at `path`, or none where there is no such file or it
/// can't be read as one, which leaves every network unguarded.
pub fn anchors(path: &Path) -> Anchors {
    #[derive(Deserialize)]
    struct Network {
        up: f64,
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<BTreeMap<String, Network>>(&text).ok())
        .map(|networks| {
            networks
                .into_iter()
                .map(|(router, network)| (router.to_lowercase(), network.up))
                .collect()
        })
        .unwrap_or_default()
}

/// What a link joining `network` starts its upload at, in kbit/s, where
/// the network has an anchor: the anchor's hold, as the one-way controller
/// starts its stay there.
pub fn hold(anchors: &Anchors, network: &str) -> Option<u64> {
    anchors
        .get(network)
        .map(|&up| crate::rate::whole(up * crate::oneway::ANCHOR_HOLD))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_network_with_an_anchor_starts_at_its_hold() {
        let anchors = Anchors::from([("00:00:5e:00:53:01".to_string(), 12_000.0)]);
        assert_eq!(hold(&anchors, "00:00:5e:00:53:01"), Some(10_800));
        assert_eq!(hold(&anchors, "link wlan0"), None, "a network with none");
    }

    #[test]
    fn each_network_s_anchor_is_read_by_its_router_in_lower_case() {
        let folder = std::env::temp_dir().join(format!("shaper-anchors-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        assert!(anchors(&file).is_empty(), "with no file");
        fs::write(&file, r#"{"00:00:5E:00:53:01": {"up": 12000}}"#).expect("the networks");
        assert_eq!(
            anchors(&file).get("00:00:5e:00:53:01"),
            Some(&12_000.0),
            "as nixos.nix writes it"
        );
        fs::write(&file, "not json").expect("a broken file");
        assert!(anchors(&file).is_empty(), "a broken file anchors nothing");
        let _ = fs::remove_dir_all(&folder);
    }
}
