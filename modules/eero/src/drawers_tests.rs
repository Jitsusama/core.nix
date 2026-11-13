//! The profile drawer the eero daemon serves, held to the figures the
//! shell's own drawer said of `home-network.json` (tests/home-network.qml),
//! in the launcher's words.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::*;
use crate::model::{Hour, Series};
use crate::rows::tests::{model, words};

fn drawer(model: &HomeNetwork, profile: &Profile) -> Value {
    profile_drawer(model, profile, &words(), &Numbers::default())
}

/// Each fact's value by its name.
fn said(drawer: &Value, name: &str) -> Option<String> {
    drawer["facts"]
        .as_array()
        .expect("the facts are a list")
        .iter()
        .find(|fact| fact["name"] == name)
        .map(|fact| fact["said"].as_str().unwrap_or("").to_owned())
}

fn held() -> (HomeNetwork, Profile) {
    let model = model();
    let mut held = model.profiles[1].clone();
    held.paused = Some(true);
    held.devices = vec![
        "02:00:00:00:00:0a".to_owned(),
        "02:00:00:00:00:35".to_owned(),
    ];
    (model, held)
}

#[test]
fn a_profile_says_what_it_blocks_its_traffic_and_each_device_in_it() {
    let (model, held) = held();

    let drawer = drawer(&model, &held);

    assert_eq!(drawer["eyebrow"], "profile");
    assert_eq!(drawer["title"], "profile 2");
    let flat = json!({"shape": "spark", "values": vec![0.0; 20], "warn": false});
    assert_eq!(
        drawer["facts"],
        json!([
            {"name": "Blocks", "said": "nothing"},
            {"name": "Schedule", "said": "none"},
            {"name": "Default", "said": "no"},
            {"name": "Traffic", "said": "0 B down \u{b7} 0 B up", "figure": flat},
            {"name": "Devices", "said": "2 listed \u{b7} 1 on now"},
            {"heading": "Devices in It"},
            {"name": "generic 2", "said": "on Den"},
            {"name": "tablet 3", "said": "not on now"},
        ])
    );
}

#[test]
fn a_device_the_profile_doesnt_name_goes_by_its_hardware_address() {
    let (model, mut held) = held();
    held.members.clear();

    let drawer = drawer(&model, &held);

    let last = drawer["facts"]
        .as_array()
        .and_then(|facts| facts.last())
        .cloned();
    assert_eq!(
        last,
        Some(json!({"name": "02:00:00:00:00:35", "said": "not on now"}))
    );
}

#[test]
fn each_filter_on_is_said_and_each_off_says_nothing() {
    let model = model();
    let mut profile = model.profiles[0].clone();
    profile.default = Some(true);
    profile.schedules = Some(2);
    profile.filters.gaming = Some(true);
    profile.filters.social = Some(true);
    profile.filters.ad_block_network = Some(true);
    profile.filters.blocked_sites = vec!["example.com".to_owned()];
    profile.filters.blocked_apps = vec!["tiktok".to_owned()];
    profile.filters.safe_search = Some(true);
    profile.filters.youtube_restricted = Some(true);
    profile.filters.allowed_sites = vec!["example.org".to_owned(), "example.net".to_owned()];
    let hours = |bytes: [u64; 2]| {
        bytes
            .into_iter()
            .map(|bytes| Hour {
                at: String::new(),
                bytes,
            })
            .collect()
    };
    profile.usage = Some(Usage {
        download: Series {
            total_bytes: 2_400_000,
            hours: hours([400_000, 2_000_000]),
        },
        upload: Series {
            total_bytes: 53_741,
            hours: hours([53_741, 0]),
        },
    });

    let drawer = drawer(&model, &profile);

    assert_eq!(
        said(&drawer, "Blocks").as_deref(),
        Some("gaming \u{b7} social \u{b7} ads network-wide \u{b7} example.com \u{b7} tiktok")
    );
    assert_eq!(
        said(&drawer, "Enforces").as_deref(),
        Some("safe search \u{b7} YouTube restricted mode")
    );
    assert_eq!(
        said(&drawer, "Allows").as_deref(),
        Some("example.org \u{b7} example.net")
    );
    assert_eq!(said(&drawer, "Schedule").as_deref(), Some("2 schedules"));
    assert_eq!(said(&drawer, "Default").as_deref(), Some("yes"));
    assert_eq!(
        said(&drawer, "Traffic").as_deref(),
        Some("2.40 MB down \u{b7} 53.7 kB up")
    );
    let traffic = drawer["facts"]
        .as_array()
        .and_then(|facts| facts.iter().find(|fact| fact["name"] == "Traffic"))
        .expect("its traffic is said");
    assert_eq!(
        traffic["figure"]["values"],
        json!([(400_000.0 + 53_741.0) / 2_000_000.0, 1.0])
    );
}

#[test]
fn its_own_ad_blocking_is_said_plainly() {
    let model = model();
    let mut profile = model.profiles[0].clone();
    profile.filters = Filters {
        gaming: Some(false),
        ad_block: Some(true),
        ad_block_network: Some(true),
        ..Filters::default()
    };

    assert_eq!(
        said(&drawer(&model, &profile), "Blocks").as_deref(),
        Some("ads")
    );
}

#[test]
fn a_profile_eero_said_none_of_its_figures_of_says_only_its_devices() {
    let model = model();
    let profile = Profile {
        id: "3009".to_owned(),
        name: Some("profile 9".to_owned()),
        devices: Vec::new(),
        members: Vec::new(),
        default: None,
        schedules: None,
        filters: Filters::default(),
        usage: None,
        ..model.profiles[0].clone()
    };

    let drawer = drawer(&model, &profile);

    assert_eq!(
        drawer["facts"],
        json!([{"name": "Devices", "said": "0 listed"}])
    );
}

#[test]
fn every_row_names_the_drawer_served_beside_it() {
    let model = model();
    let words = words();
    let numbers = Numbers::default();

    let rows = crate::rows::rows(&model, &words, &numbers);
    let drawers = records(&model, &words, &numbers);

    // A row inside another is something to do to it, as the internet's
    // speed test is, said in that row's drawer rather than one of its own.
    let mut named: Vec<&Value> = rows
        .iter()
        .filter(|row| row.get("parent").is_none())
        .map(|row| &row["drawer"])
        .collect();
    let mut served: Vec<&Value> = drawers.iter().map(|record| &record["key"]).collect();
    named.sort_by_key(|key| key.as_str());
    served.sort_by_key(|key| key.as_str());
    assert_eq!(named, served, "the internet, each node, device and profile");
    assert!(
        drawers.iter().all(|record| record["kind"] == "drawer"),
        "each is a drawer's record"
    );
}
