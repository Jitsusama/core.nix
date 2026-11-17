//! What each network's line is known to carry, as nixos.nix writes
//! `jitsusama.shaper.networks` for `shaper run --networks`: the one-way
//! controller guards the upload near it (src/oneway.rs), and under the
//! network's ceiling where one is set.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Each network's declared upload rate, its anchor, in kbit/s, by its
/// router's hardware address, as `jitsusama.shaper.networks` sets them.
pub type Anchors = BTreeMap<String, f64>;

/// Each network's ceiling, the most its upload is held to, in kbit/s, by
/// its router's hardware address, where `jitsusama.shaper.networks` sets
/// one: under a second shaper past ours, so the queue stands in ours.
pub type Ceilings = BTreeMap<String, f64>;

#[derive(Deserialize)]
struct Network {
    up: f64,
    #[serde(default)]
    most: Option<f64>,
}

/// The networks written at `path`, by router in lower case, or none where
/// there is no such file or it can't be read as one.
fn networks(path: &Path) -> BTreeMap<String, Network> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<BTreeMap<String, Network>>(&text).ok())
        .map(|networks| {
            networks
                .into_iter()
                .map(|(router, network)| (router.to_lowercase(), network))
                .collect()
        })
        .unwrap_or_default()
}

/// The anchors written at `path`, or none where there is no such file or it
/// can't be read as one, which leaves every network unguarded.
pub fn anchors(path: &Path) -> Anchors {
    networks(path)
        .into_iter()
        .map(|(router, network)| (router, network.up))
        .collect()
}

/// The ceilings written at `path`, of the networks that set one.
pub fn ceilings(path: &Path) -> Ceilings {
    networks(path)
        .into_iter()
        .filter_map(|(router, network)| Some((router, network.most?)))
        .collect()
}

/// What a link joining `network` starts its upload at, in kbit/s, where
/// the network has an anchor: the anchor's hold, or its ceiling where that
/// is lower, as the one-way controller starts its stay there.
pub fn hold(anchors: &Anchors, ceilings: &Ceilings, network: &str) -> Option<u64> {
    let most = ceilings.get(network).copied().unwrap_or(f64::INFINITY);
    anchors
        .get(network)
        .map(|&up| crate::rate::whole((up * crate::oneway::ANCHOR_HOLD).min(most)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_network_with_an_anchor_starts_at_its_hold() {
        let anchors = Anchors::from([("00:00:5e:00:53:01".to_string(), 12_000.0)]);
        let none = Ceilings::new();
        assert_eq!(hold(&anchors, &none, "00:00:5e:00:53:01"), Some(10_800));
        assert_eq!(
            hold(&anchors, &none, "link wlan0"),
            None,
            "a network with none"
        );
        let under = Ceilings::from([("00:00:5e:00:53:01".to_string(), 9_500.0)]);
        assert_eq!(
            hold(&anchors, &under, "00:00:5e:00:53:01"),
            Some(9_500),
            "a ceiling under the hold starts it there"
        );
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
        assert!(ceilings(&file).is_empty(), "a network with no ceiling");
        fs::write(
            &file,
            r#"{"00:00:5E:00:53:01": {"up": 12000, "most": 9500}, "00:00:5e:00:53:02": {"up": 9000, "most": null}}"#,
        )
        .expect("the networks");
        assert_eq!(
            ceilings(&file),
            Ceilings::from([("00:00:5e:00:53:01".to_string(), 9_500.0)]),
            "as nixos.nix writes them, null where none is set"
        );
        assert_eq!(
            anchors(&file).len(),
            2,
            "a ceiling leaves the anchors alone"
        );
        fs::write(&file, "not json").expect("a broken file");
        assert!(anchors(&file).is_empty(), "a broken file anchors nothing");
        assert!(ceilings(&file).is_empty(), "nor sets a ceiling");
        let _ = fs::remove_dir_all(&folder);
    }
}
