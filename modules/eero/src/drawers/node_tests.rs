//! The node drawer the eero daemon serves, held to the figures the shell's
//! own drawer said of `home-network.json` (tests/home-network.qml), in the
//! launcher's words, its times left for the shell to word as it draws.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::*;
use crate::model::{NodeDevices, NodeHistory, RadioHistory};
use crate::rows::tests::{model, words};

fn drawn(model: &HomeNetwork, id: &str) -> Value {
    let node = model
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("the fixture has the node");
    drawer(model, node, &words(), &Numbers::default())
}

fn facts(drawer: &Value) -> &Vec<Value> {
    drawer["facts"].as_array().expect("the facts are a list")
}

/// The fact named so, the first where several are.
fn named_fact<'a>(drawer: &'a Value, name: &str) -> Option<&'a Value> {
    facts(drawer).iter().find(|fact| fact["name"] == name)
}

fn said(drawer: &Value, name: &str) -> Option<String> {
    named_fact(drawer, name).map(|fact| fact["said"].as_str().unwrap_or("").to_owned())
}

/// The facts under a heading, up to the next.
fn under(drawer: &Value, heading: &str) -> Vec<Value> {
    facts(drawer)
        .iter()
        .skip_while(|fact| fact["heading"] != heading)
        .skip(1)
        .take_while(|fact| fact.get("heading").is_none())
        .cloned()
        .collect()
}

fn ms(at: &str) -> i64 {
    time::to_unix(at).expect("a time") * 1000
}

#[test]
fn a_node_leads_with_what_its_row_lacks_then_which_it_is() {
    let model = model();

    let hall = drawn(&model, "2003");

    assert_eq!(hall["eyebrow"], "on home \u{b7} gateway node");
    assert_eq!(hall["eyebrowNames"], true);
    assert_eq!(hall["title"], "Hall");
    assert_eq!(
        facts(&hall)[..5],
        [
            json!({
                "name": "Last Heard", "at": ms("2026-11-07T22:53:51Z"), "as": "ago",
                "said": "{time} ago",
            }),
            json!({"name": "Backhaul", "said": "Ethernet"}),
            json!({"name": "Devices", "said": "all on Wi-Fi"}),
            json!({"name": "Model", "said": "eero Pro 6 \u{b7} K010011"}),
            json!({"name": "Serial Number", "said": "SERIAL0003"}),
        ]
    );
    assert_eq!(drawn(&model, "2002")["eyebrow"], "on home \u{b7} node");
}

#[test]
fn a_node_says_its_state_only_where_it_isnt_online() {
    let mut model = model();
    assert!(named_fact(&drawn(&model, "2003"), "State").is_none());
    model.nodes[0].state = Some("OFFLINE".to_owned());

    let hall = drawn(&model, &model.nodes[0].id);

    assert_eq!(facts(&hall)[0], json!({"name": "State", "said": "offline"}));
}

#[test]
fn a_node_heard_late_says_so_and_one_never_heard_says_never() {
    let mut model = model();
    model.nodes[0].heartbeat_ok = Some(false);
    let id = model.nodes[0].id.clone();
    assert_eq!(
        named_fact(&drawn(&model, &id), "Last Heard").map(|fact| &fact["said"]),
        Some(&json!("{time} ago \u{b7} late"))
    );
    model.nodes[0].last_heartbeat = None;

    assert_eq!(
        said(&drawn(&model, &id), "Last Heard").as_deref(),
        Some("never")
    );
}

#[test]
fn a_backhaul_over_wifi_has_its_bars_sparked_over_the_hour() {
    let mut model = model();

    let study = drawn(&model, "2002");

    let backhaul = named_fact(&study, "Backhaul").expect("a backhaul");
    assert_eq!(
        backhaul["said"],
        "Wi-Fi from Hall over 5 GHz high \u{b7} 5 of 5 bars"
    );
    assert_eq!(
        backhaul["figure"],
        json!({"shape": "spark", "values": [1.0, 0.8, 1.0, 1.0, 1.0, 1.0], "warn": false})
    );
    let at = model
        .nodes
        .iter()
        .position(|node| node.id == "2002")
        .expect("Study");
    model.nodes[at].history = NodeHistory::default();
    assert_eq!(
        named_fact(&drawn(&model, "2002"), "Backhaul").map(|fact| &fact["figure"]),
        Some(&json!({"shape": "stroke", "values": [1.0], "warn": false}))
    );
}

#[test]
fn a_nodes_devices_say_how_they_are_on() {
    let mut model = model();
    let at = model
        .nodes
        .iter()
        .position(|node| node.id == "2002")
        .expect("Study");
    let given = |all, wifi, ethernet| NodeDevices {
        all: Some(all),
        wifi: Some(wifi),
        ethernet: Some(ethernet),
    };
    model.nodes[at].devices = given(9, 6, 3);
    assert_eq!(
        said(&drawn(&model, "2002"), "Devices").as_deref(),
        Some("6 on Wi-Fi \u{b7} 3 on Ethernet")
    );
    model.nodes[at].devices = given(2, 0, 2);
    assert_eq!(
        said(&drawn(&model, "2002"), "Devices").as_deref(),
        Some("all on Ethernet")
    );
    model.nodes[at].devices = given(0, 0, 0);

    assert!(named_fact(&drawn(&model, "2002"), "Devices").is_none());
}

#[test]
fn each_radio_by_band_how_busy_and_how_strongly_it_sends() {
    let model = model();

    let hall = drawn(&model, "2003");

    let pairs = |heading| -> Vec<(String, String)> {
        under(&hall, heading)
            .iter()
            .map(|fact| {
                (
                    fact["name"].as_str().unwrap_or("").to_owned(),
                    fact["said"].as_str().unwrap_or("").to_owned(),
                )
            })
            .collect()
    };
    let owned = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(name, said)| ((*name).to_owned(), (*said).to_owned()))
            .collect()
    };
    assert_eq!(
        pairs("Radios"),
        owned(&[
            ("2.4 GHz", "channel 6 \u{b7} 20 MHz \u{b7} 1 device"),
            ("5 GHz high", "channel 149 \u{b7} 80 MHz \u{b7} 1 device"),
            ("5 GHz low", "channel 52 \u{b7} 80 MHz \u{b7} 0 devices"),
        ])
    );
    assert_eq!(
        pairs("How Busy"),
        owned(&[
            ("2.4 GHz", "8%"),
            ("5 GHz high", "41%"),
            ("5 GHz low", "5%")
        ])
    );
    assert_eq!(
        under(&hall, "How Busy")[1]["figure"],
        json!({"shape": "spark", "values": [0.45, 0.38, 0.5, 0.43, 0.39, 0.41], "warn": false})
    );
    assert_eq!(
        pairs("Transmit Power"),
        owned(&[
            ("2.4 GHz", "30 dBm"),
            ("5 GHz high", "28 dBm"),
            ("5 GHz low", "22 dBm"),
        ])
    );
    assert_eq!(
        under(&hall, "Transmit Power")[0]["figure"]["values"],
        json!([1.0, 1.0, 1.0, 1.0, 1.0, 1.0])
    );
    assert_eq!(
        pairs("Radios' Hardware Addresses")[0],
        ("2.4 GHz".to_owned(), "02:00:00:00:00:0c".to_owned())
    );
}

#[test]
fn a_radio_with_no_history_is_busy_as_a_stroke_and_sparks_no_power() {
    let mut model = model();
    for radio in &mut model.nodes[0].radios {
        radio.history = RadioHistory::default();
    }
    let id = model.nodes[0].id.clone();

    let hall = drawn(&model, &id);

    assert_eq!(
        under(&hall, "How Busy")[1]["figure"],
        json!({"shape": "stroke", "values": [0.41], "warn": false})
    );
    assert!(under(&hall, "Transmit Power")[1].get("figure").is_none());
}

#[test]
fn each_port_as_linked_and_what_went_through_it() {
    let model = model();

    let ports = |id| under(&drawn(&model, id), "Ports");

    assert_eq!(
        ports("2003"),
        [
            json!({
                "name": "Port 1",
                "said": "1.00 Gb/s of 1.00 Gb/s \u{b7} internet \u{b7} backhaul \u{b7} 0 B in \
                    \u{b7} 153 MB out",
            }),
            json!({"name": "Port 2", "said": "1.00 Gb/s of 1.00 Gb/s \u{b7} 0 B in \u{b7} 53.7 kB out"}),
        ]
    );
    assert_eq!(
        ports("2002")[0],
        json!({"name": "Port 1", "said": "not linked \u{b7} up to 1.00 Gb/s"})
    );
}

#[test]
fn about_a_node_its_firmware_times_addresses_light_and_traffic() {
    let model = model();
    let read = ms(&model.read);

    let hall = drawn(&model, "2003");

    let about = under(&hall, "This Node");
    let by = |name: &str| {
        about
            .iter()
            .find(|fact| fact["name"] == name)
            .cloned()
            .unwrap_or(Value::Null)
    };
    assert_eq!(by("Firmware")["said"], "v7.17.4-5 \u{b7} up to date");
    assert_eq!(by("Support Ends"), Value::Null);
    assert_eq!(
        by("Up For"),
        json!({"name": "Up For", "at": read - 55_878_000, "as": "lasted"})
    );
    assert_eq!(
        by("On the Cloud For"),
        json!({"name": "On the Cloud For", "at": read - 54_413_000, "as": "lasted"})
    );
    assert_eq!(
        by("Added"),
        json!({
            "name": "Added", "at": ms("2020-11-13T21:47:15Z"), "as": "day", "said": "{time}",
        })
    );
    for (name, wanted) in [
        ("Address", "10.0.0.1"),
        ("IPv6", "fe80::7/64 \u{b7} fe80::8/64"),
        ("Hardware Address", "02:00:00:00:00:21"),
        ("Light", "on \u{b7} 100%"),
        ("Power", "USB"),
        ("Traffic", "150 MB down \u{b7} 10.1 MB up"),
    ] {
        assert_eq!(by(name)["said"], wanted, "{name}");
    }
    assert_eq!(
        by("Traffic")["figure"]["values"].as_array().map(Vec::len),
        Some(20)
    );
}

#[test]
fn the_devices_on_a_node_strongest_first() {
    let model = model();

    let on = under(&drawn(&model, "2003"), "Devices on It");

    assert_eq!(
        on,
        [
            json!({"name": "generic 3", "said": "\u{2212}48 dBm \u{b7} 2.4 GHz \u{b7} channel 6"}),
            json!({"name": "phone 1", "said": "\u{2212}52 dBm \u{b7} 5 GHz \u{b7} channel 149"}),
        ]
    );
}

#[test]
fn each_device_not_on_now_under_the_node_it_was_last_on() {
    let mut model = model();

    let off = |model: &HomeNetwork, id| under(&drawn(model, id), "Not on Now");

    assert_eq!(
        ["2001", "2002", "2003"].map(|id| off(&model, id).len()),
        [5, 3, 1]
    );
    assert_eq!(
        off(&model, "2002")[0],
        json!({
            "name": "game console 2", "at": ms("2026-11-07T21:38:27Z"), "as": "moment",
            "said": "5 GHz \u{b7} {time}",
        })
    );
    assert_eq!(off(&model, "2003")[0]["said"], "Ethernet \u{b7} {time}");
    assert!(
        off(&model, "2001")
            .iter()
            .any(|fact| fact["name"] == "02:00:00:00:00:1b")
    );
    model.devices_off.clear();
    assert!(
        facts(&drawn(&model, "2001"))
            .iter()
            .all(|fact| fact["heading"] != "Not on Now")
    );
}

#[test]
fn who_joined_or_left_a_node_named_by_when() {
    let model = model();

    let moved = |id| under(&drawn(&model, id), "Joined and Left in the Last Day");

    assert_eq!(
        moved("2001"),
        [json!({
            "name": {"at": ms("2026-11-07T22:49:43Z"), "as": "moment"},
            "said": "phone\u{a0}1\u{200b} left \u{b7} 5 GHz",
        })]
    );
    assert_eq!(
        moved("2002")[0]["said"],
        "laptop\u{a0}computer\u{a0}1\u{200b} joined \u{b7} 5 GHz"
    );
    assert_eq!(moved("2003"), Vec::<Value>::new());
}

#[test]
fn a_history_past_twenty_places_is_gathered_into_twenty_means() {
    let values: Vec<Option<f64>> = (0..40).map(|at| Some(f64::from(at % 2))).collect();

    let spark = history_spark(&values, Some(1.0)).expect("a spark");

    assert_eq!(spark["values"], json!(vec![0.5; 20]));
    assert_eq!(history_spark(&[Some(1.0), None], Some(1.0)), None);
    assert_eq!(
        history_spark(&[Some(1.0), None, Some(0.5)], Some(1.0))
            .map(|spark| spark["values"].clone()),
        Some(json!([1.0, null, 0.5]))
    );
}

#[test]
fn every_node_has_its_drawer_served_and_its_row_names_it() {
    let model = model();

    let served = records(&model, &words(), &Numbers::default());

    assert_eq!(
        served
            .iter()
            .map(|record| record["key"].clone())
            .collect::<Vec<_>>(),
        model
            .nodes
            .iter()
            .map(|node| json!(key(&node.id)))
            .collect::<Vec<_>>()
    );
    assert!(served.iter().all(|record| record["kind"] == DRAWER));
}
