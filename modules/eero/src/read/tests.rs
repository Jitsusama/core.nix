use super::devices::{band, device, rate};
use super::profiles::filters;
use super::*;
use crate::golden::{MODEL, READ, answering, contract, contract_answers, differences, history};
use serde_json::json;

#[test]
fn the_contract_and_its_history_read_into_the_model() {
    let mut read =
        home_network(&contract_answers(), READ, answering()).expect("the network was read");
    // The model is the one the daemon keeps on the machine that's the
    // first laptop, so the views have this machine to put first.
    mark_this_machine(&mut read, &["02:00:00:00:00:0e".to_owned()], None);
    history()
        .draw(&mut read, time::to_unix(READ).unwrap())
        .unwrap();

    let out = differences(
        &serde_json::to_value(read).unwrap(),
        &serde_json::from_str(MODEL).unwrap(),
    );

    assert!(
        out.is_empty(),
        "differs from home-network.json:\n{}",
        out.join("\n")
    );
}

#[test]
fn this_machine_is_the_device_with_one_of_its_addresses() {
    let mut read =
        home_network(&contract_answers(), READ, answering()).expect("the network was read");

    mark_this_machine(
        &mut read,
        &[
            "02:00:00:00:00:99".to_owned(),
            "02:00:00:00:00:0E".to_owned(),
        ],
        None,
    );

    let here: Vec<&str> = read
        .devices
        .iter()
        .filter(|device| device.this_machine)
        .map(|device| device.mac.as_str())
        .collect();
    assert_eq!(here, ["02:00:00:00:00:0e"]);

    mark_this_machine(&mut read, &[], None);

    assert!(
        read.devices.iter().all(|device| !device.this_machine),
        "with no addresses of its own, no device is this machine"
    );
}

#[test]
fn this_machine_goes_by_its_own_hostname_where_eero_knows_none() {
    let mut read =
        home_network(&contract_answers(), READ, answering()).expect("the network was read");
    let own = ["02:00:00:00:00:0e".to_owned()];
    let here = |read: &HomeNetwork| {
        read.devices
            .iter()
            .find(|device| device.this_machine)
            .and_then(|device| device.hostname.clone())
    };

    mark_this_machine(&mut read, &own, Some("workstation"));
    assert_eq!(
        here(&read).as_deref(),
        Some("laptop computer 1"),
        "the hostname eero knows stands"
    );

    let machine = read.devices.iter_mut().find(|device| device.this_machine);
    machine.expect("this machine is listed").hostname = None;
    mark_this_machine(&mut read, &own, Some("workstation"));
    assert_eq!(here(&read).as_deref(), Some("workstation"));
    assert!(
        read.devices
            .iter()
            .filter(|device| !device.this_machine)
            .all(|device| device.hostname.as_deref() != Some("workstation")),
        "no other device is given this machine's hostname"
    );
}

#[test]
fn a_name_that_only_repeats_the_maker_is_no_name() {
    let named = |name: &str, maker: &str| {
        device(&json!({"mac": "02:00:00:00:00:99", "display_name": name, "manufacturer": maker}))
            .expect("the device is read")
            .name
    };

    assert_eq!(named("Intel Corporate", "Intel Corporate"), None);
    assert_eq!(named("intel corporate", "Intel Corporate"), None);
    assert_eq!(
        named("Den TV", "Sony Corporation").as_deref(),
        Some("Den TV")
    );
}

#[test]
fn nothing_is_drawn_before_the_network_is_read() {
    let answers = Answers {
        devices: contract("net.devices.json"),
        ..Answers::default()
    };

    assert_eq!(
        home_network(&answers, "2026-11-07T22:54:43Z", answering()),
        None
    );
}

#[test]
fn a_network_read_alone_leaves_the_slower_reads_unknown() {
    let answers = Answers {
        network: contract("network.json"),
        ..Answers::default()
    };

    let read = home_network(&answers, "2026-11-07T22:54:43Z", answering()).unwrap();

    assert_eq!(read.internet.usage, None);
    assert_eq!(
        read.internet.speed_tests,
        [] as [crate::model::SpeedTest; 0]
    );
    assert_eq!(read.internet.thread.name, None);
    assert_eq!(read.nodes.len(), 3);
    assert!(
        read.nodes.iter().all(|node| node.usage.is_none()),
        "no node has traffic while its usage answer is unread"
    );
    let hall = &read.nodes[0];
    assert_eq!(hall.ports[0].best_mbps, None);
    assert!(
        hall.ports[0].upstream,
        "the gateway's linked WAN port carries its backhaul"
    );
}

#[test]
fn a_profile_says_its_schedules_what_it_blocks_and_its_traffic() {
    let first_usage = ("3001".to_owned(), contract("usage.profile.3001.json"));
    let answers = Answers {
        network: contract("network.json"),
        profiles: contract("net.profiles.json"),
        profile_usage: BTreeMap::from([first_usage]),
        ..Answers::default()
    };

    let read = home_network(&answers, READ, answering()).unwrap();

    let first = &read.profiles[0];
    assert_eq!((first.default, first.schedules), (Some(false), Some(0)));
    let filters = &first.filters;
    assert_eq!(filters.set, Some(false));
    assert_eq!(
        (filters.gaming, filters.violent),
        (Some(false), Some(false))
    );
    assert_eq!(filters.youtube_restricted, Some(false));
    assert_eq!(
        (filters.ad_block, filters.ad_block_network),
        (Some(false), Some(false))
    );
    assert!(
        filters.blocked_sites.is_empty() && filters.blocked_apps.is_empty(),
        "profile 1 blocks no sites and no apps"
    );
    let usage = first.usage.as_ref().expect("profile 1's traffic was read");
    assert_eq!(usage.download.hours.len(), 25);
    assert_eq!(usage.upload.total_bytes, 0);
    assert_eq!(
        read.profiles[1].usage, None,
        "profile 2's traffic wasn't read"
    );
}

#[test]
fn each_profile_is_named_by_its_id_to_ask_its_traffic() {
    assert_eq!(
        profile_ids(&contract("net.profiles.json")),
        ["3001", "3002", "3003"]
    );
}

#[test]
fn a_kind_the_unified_filters_leave_out_is_the_older_filters_word() {
    let profile = json!({
        "premium_dns": {
            "dns_policies": {
                "block_pornographic_content": true,
                "block_violent_content": true
            },
            "ad_block_settings": {"enabled": true, "enabled_for_network": false},
            "advanced_content_filters": {"blocked_list": ["example.com"], "allowed_list": []},
            "blocked_applications": ["tiktok"]
        },
        "unified_content_filters": {
            "is_content_filters_set": true,
            "dns_policies": {"block_gaming_content": true, "block_violent_content": false}
        }
    });

    let filters = filters(&profile);

    assert_eq!(filters.set, Some(true));
    assert_eq!(filters.gaming, Some(true));
    assert_eq!(
        filters.pornographic,
        Some(true),
        "only the older filters say it"
    );
    assert_eq!(
        filters.violent,
        Some(false),
        "the unified filters say it first"
    );
    assert_eq!(filters.shopping, None);
    assert_eq!(filters.ad_block, Some(true));
    assert_eq!(filters.blocked_sites, ["example.com"]);
    assert_eq!(filters.blocked_apps, ["tiktok"]);
}

#[test]
fn a_band_is_drawn_from_the_frequency() {
    assert_eq!(band(2437), "2.4 GHz");
    assert_eq!(band(5180), "5 GHz");
    assert_eq!(band(5925), "6 GHz");
    assert_eq!(band(6115), "6 GHz");
}

#[test]
fn a_rate_eero_explains_says_how_the_radio_makes_it() {
    let info = json!({
        "rate_bps": 2_882_400_000_u64, "mcs": 13, "nss": 2,
        "guard_interval": "GI_800NS", "channel_width": "WIDTH_160MHz", "phy_type": "EHT"
    });

    let rate = rate(Some(&info), Some(48.0)).unwrap();

    assert_eq!(rate.mbps, Some(2882.4));
    assert_eq!((rate.mcs, rate.streams), (Some(13), Some(2)));
    assert_eq!((rate.width_mhz, rate.guard_ns), (Some(160), Some(800)));
    assert_eq!(rate.generation.as_deref(), Some("Wi-Fi 7"));
}

#[test]
fn a_rate_eero_only_names_is_the_bare_bitrate() {
    let empty = json!({"rate_bps": null, "mcs": null, "nss": null,
        "guard_interval": null, "channel_width": null, "phy_type": null});

    let rate = rate(Some(&empty), Some(48.0)).unwrap();

    assert_eq!(rate.mbps, Some(48.0));
    assert_eq!(rate.mcs, None);
    assert_eq!(super::devices::rate(Some(&empty), None), None);
}

#[test]
fn an_ethernet_device_names_its_port_and_speed() {
    let record = json!({
        "mac": "02:00:00:00:00:40", "connected": true, "wireless": false,
        "connectivity": {"ethernet_status": {"port_name": "2", "speed": "P1000"}}
    });

    let device = device(&record).unwrap();

    assert!(device.ethernet, "a wired device reads as on ethernet");
    let port = device.port.unwrap();
    assert_eq!((port.name.as_deref(), port.mbps), (Some("2"), Some(1000)));
    assert_eq!(device.band, None);
}

#[test]
fn only_devices_on_now_are_listed_by_node_then_signal() {
    let answer = json!([
        {"mac": "a", "connected": true, "source": {"url": "/2.2/eeros/2002"},
         "connectivity": {"signal": "-70 dBm"}},
        {"mac": "b", "connected": true, "source": {"url": "/2.2/eeros/2001"},
         "connectivity": {"signal": "-60 dBm"}},
        {"mac": "c", "connected": true, "source": {"url": "/2.2/eeros/2002"},
         "connectivity": {"signal": "-40 dBm"}},
        {"mac": "d", "connected": false, "source": {"url": "/2.2/eeros/2001"}}
    ]);

    let macs: Vec<String> = devices(&answer).into_iter().map(|d| d.mac).collect();

    assert_eq!(macs, ["b", "c", "a"]);
}
