//! The device drawer the eero daemon serves, held to the figures the
//! shell's own drawer said of `home-network.json` (tests/home-network.qml),
//! in the launcher's words, its times left for the shell to word as it
//! draws.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::super::node::tests::under;
use super::*;
use crate::model::{DeviceHistory, Event, Reservation, Roam};
use crate::rows::tests::{model, words};

fn device(model: &HomeNetwork, name: &str) -> Device {
    model
        .devices
        .iter()
        .find(|device| device.name.as_deref() == Some(name))
        .expect("the fixture has the device")
        .clone()
}

fn drawn(model: &HomeNetwork, device: &Device) -> Value {
    drawer(model, device, &words(), &Numbers::default())
}

fn facts(drawer: &Value) -> &Vec<Value> {
    drawer["facts"].as_array().expect("the facts are a list")
}

fn named_fact<'a>(drawer: &'a Value, name: &str) -> Option<&'a Value> {
    facts(drawer).iter().find(|fact| fact["name"] == name)
}

fn said(drawer: &Value, name: &str) -> Option<String> {
    named_fact(drawer, name).map(|fact| fact["said"].as_str().unwrap_or("").to_owned())
}

fn ms(at: &str) -> i64 {
    time::to_unix(at).expect("a time") * 1000
}

/// A spark's places, each as `scale` makes it, rounded to a tenth.
fn places(fact: &Value, scale: impl Fn(f64) -> f64) -> Vec<Option<f64>> {
    fact["figure"]["values"]
        .as_array()
        .expect("a spark's places")
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|value| (scale(value) * 10.0).round() / 10.0)
        })
        .collect()
}

#[test]
fn a_device_leads_with_how_well_its_heard_then_what_it_is() {
    let model = model();

    let generic = drawn(&model, &device(&model, "generic 2"));

    assert_eq!(generic["eyebrow"], "on Den \u{b7} Wi-Fi");
    assert_eq!(generic["eyebrowNames"], true);
    assert_eq!(generic["title"], "generic 2");
    let first: Vec<&Value> = facts(&generic)[..7]
        .iter()
        .map(|fact| &fact["name"])
        .collect();
    assert_eq!(
        first,
        [
            "Signal",
            "Above the Noise",
            "Score",
            "Band",
            "Rate Down",
            "Rate Up",
            "Speed"
        ]
    );
    for (name, wanted) in [
        ("Kind", "generic"),
        ("Node", "Den \u{b7} eero Pro 6"),
        ("Signal", "\u{2212}37 dBm"),
        ("Above the Noise", "71 dB"),
        ("Score", "5 of 5 bars"),
        ("Band", "5 GHz \u{b7} channel 52 \u{b7} 5,260 MHz"),
        ("Radio's Hardware Address", "02:00:00:00:00:02"),
        (
            "Rate Down",
            "1.20 Gb/s \u{b7} MCS 11 \u{b7} 2 streams \u{b7} 80 MHz \u{b7} 800 ns guard \u{b7} Wi-Fi 6",
        ),
        (
            "Rate Up",
            "1.08 Gb/s \u{b7} MCS 10 \u{b7} 2 streams \u{b7} 80 MHz \u{b7} 800 ns guard \u{b7} Wi-Fi 6",
        ),
        ("Speed", "idle"),
        ("Retries", "0% \u{b7} quiet for 540 ms"),
        (
            "Packets",
            "52,846 down \u{b7} 94,913 up \u{b7} 147,759 in all",
        ),
        ("Sent Again", "735 \u{b7} 0 per million"),
        ("Never Sent", "5,494 per million"),
        ("Dropped", "0 \u{b7} 0 per million"),
        ("Network", "home \u{b7} WPA2 \u{b7} main"),
        ("Address", "10.0.0.45"),
        ("Other Addresses", "fe80::6"),
        ("Hardware Address", "02:00:00:00:00:0a \u{b7} private"),
    ] {
        assert_eq!(said(&generic, name).as_deref(), Some(wanted), "{name}");
    }
    assert_eq!(
        named_fact(&generic, "First Seen"),
        Some(&json!({
            "name": "First Seen", "at": ms("2026-10-22T16:54:41.749Z"), "as": "moment",
            "said": "{time}",
        }))
    );
    assert!(named_fact(&generic, "Hostname").is_none(), "its name again");
    assert!(named_fact(&generic, "Profile").is_none(), "in none");
    assert!(
        under(&generic, "Moved in the Last Day").is_empty(),
        "no moves, no heading"
    );
    assert!(
        under(&generic, "What Happened").is_empty(),
        "none of another's events"
    );
}

#[test]
fn a_device_named_only_by_its_maker_is_titled_as_its_row_is() {
    let model = model();
    let mut unnamed = device(&model, "generic 2");
    unnamed.name = None;
    unnamed.hostname = None;
    unnamed.make = None;
    unnamed.manufacturer = Some("Intel".to_owned());

    assert_eq!(drawn(&model, &unnamed)["title"], "Intel device");
}

#[test]
fn what_moves_sparks_the_devices_hour() {
    let model = model();

    let generic = drawn(&model, &device(&model, "generic 2"));

    let dbm = |level: f64| level * 60.0 - 90.0;
    assert_eq!(
        places(named_fact(&generic, "Signal").unwrap(), dbm),
        [-40.0, -38.0, -41.0, -39.0, -36.0, -37.0].map(Some)
    );
    assert_eq!(
        places(named_fact(&generic, "Above the Noise").unwrap(), |share| {
            share
        }),
        [Some(1.0); 6]
    );
    assert_eq!(
        places(named_fact(&generic, "Rate Down").unwrap(), |share| share
            * 1200.9),
        [1200.9, 1080.8, 900.7, 1200.9, 1200.9, 1200.9].map(Some)
    );
    assert_eq!(
        places(named_fact(&generic, "Speed").unwrap(), |share| share),
        [Some(0.0); 6],
        "an idle hour sparks as nothing moving"
    );
    let busy = drawn(&model, &device(&model, "unknown computer 1"));
    let both = places(named_fact(&busy, "Speed").unwrap(), |share| share);
    assert_eq!(both.iter().position(|place| *place == Some(1.0)), Some(1));
    let tablet = drawn(&model, &device(&model, "tablet 1"));
    assert!(
        named_fact(&tablet, "Above the Noise").is_none(),
        "no SNR eero never gave"
    );
    assert!(
        named_fact(&tablet, "Rate Up").is_none(),
        "no rate up eero never gave"
    );
}

#[test]
fn a_read_eero_gave_nothing_at_is_a_gap_and_no_history_a_stroke() {
    let model = model();
    let mut gapped = device(&model, "generic 2");
    gapped.history = DeviceHistory {
        signal_dbm: vec![Some(-60), None, Some(-60)],
        ..DeviceHistory::default()
    };

    let drawer = drawn(&model, &gapped);

    let dbm = |level: f64| level * 60.0 - 90.0;
    assert_eq!(
        places(named_fact(&drawer, "Signal").unwrap(), dbm),
        [Some(-60.0), None, Some(-60.0)]
    );
    gapped.history = DeviceHistory::default();
    let unkept = drawn(&model, &gapped);
    assert_eq!(
        named_fact(&unkept, "Signal").unwrap()["figure"]["shape"],
        "stroke"
    );
    assert!(
        named_fact(&unkept, "Rate Down")
            .unwrap()
            .get("figure")
            .is_none(),
        "no spark of a rate with no history"
    );
}

#[test]
fn where_a_device_moved_over_the_day_by_when() {
    let model = model();
    let mut tablet = device(&model, "tablet 1");

    assert_eq!(
        under(&drawn(&model, &tablet), "Moved in the Last Day"),
        [json!({
            "name": {"at": ms("2026-11-07T22:44:43Z"), "as": "moment"},
            "said": "Study to Den \u{b7} 2.4 GHz to 5 GHz",
        })]
    );
    tablet.roams = vec![Roam {
        at: "2026-11-07T21:10:00Z".to_owned(),
        from_node: Some("2001".to_owned()),
        node: Some("2001".to_owned()),
        from_band: Some("2.4 GHz".to_owned()),
        band: Some("5 GHz".to_owned()),
    }];
    assert_eq!(
        under(&drawn(&model, &tablet), "Moved in the Last Day")[0]["said"],
        "Den \u{b7} 2.4 GHz to 5 GHz"
    );
}

#[test]
fn an_older_devices_rate_its_score_as_a_share_and_its_profile() {
    let mut model = model();
    let printer = drawn(&model, &device(&model, "printer 1"));
    assert_eq!(
        said(&printer, "Rate Down").as_deref(),
        Some(
            "72.2 Mb/s \u{b7} MCS 7 \u{b7} 1 stream \u{b7} 20 MHz \u{b7} 400 ns guard \u{b7} Wi-Fi 4"
        )
    );
    assert!(
        named_fact(&printer, "Rate Up").is_none(),
        "no rate up it never gave"
    );
    let mut scored = model
        .devices
        .iter()
        .find(|one| one.mac == "02:00:00:00:00:05")
        .expect("the fixture has it")
        .clone();
    scored.bars = None;
    assert_eq!(
        said(&drawn(&model, &scored), "Score").as_deref(),
        Some("78%")
    );
    model.profiles[0].name = Some("profile 1".to_owned());
    model.profiles[0].paused = Some(true);
    let mut phone = device(&model, "phone 1");
    phone.profile = Some(model.profiles[0].id.clone());

    assert_eq!(
        said(&drawn(&model, &phone), "Profile").as_deref(),
        Some("profile 1 \u{b7} paused")
    );
}

#[test]
fn the_address_kept_for_a_device_is_said_once() {
    let mut model = model();
    let printer = device(&model, "printer 1");
    assert!(
        named_fact(&drawn(&model, &printer), "Reserved Address").is_none(),
        "none for a device with none kept"
    );
    let kept = model.internet.addresses.reservations.clone();
    model.internet.addresses.reservations.push(Reservation {
        mac: printer.mac.to_uppercase(),
        description: Some("office printer".to_owned()),
        address: Some("10.0.0.30".to_owned()),
    });

    let elsewhere = drawn(&model, &printer);

    assert_eq!(said(&elsewhere, "Address").as_deref(), Some("10.0.0.31"));
    assert_eq!(
        said(&elsewhere, "Reserved Address").as_deref(),
        Some("10.0.0.30")
    );
    model.internet.addresses.reservations = kept;
    model.internet.addresses.reservations.push(Reservation {
        mac: printer.mac.clone(),
        description: None,
        address: printer.address.clone(),
    });
    let same = drawn(&model, &printer);
    assert_eq!(
        said(&same, "Address").as_deref(),
        Some("10.0.0.31 \u{b7} reserved")
    );
    assert!(
        named_fact(&same, "Reserved Address").is_none(),
        "the address it has, said once"
    );
}

#[test]
fn what_happened_to_a_device_its_names_kept_whole() {
    let mut model = model();

    let tablet = drawn(&model, &device(&model, "tablet 1"));
    let laptop = drawn(&model, &device(&model, "laptop computer 1"));

    let told: Vec<String> = under(&tablet, "What Happened")
        .iter()
        .map(|fact| fact["said"].as_str().unwrap_or("").to_owned())
        .collect();
    assert_eq!(
        told,
        [
            "tablet\u{a0}1\u{200b} connected to Den eero Pro 6 on 5.2\u{a0}GHz channel 52.",
            "tablet\u{a0}1\u{200b} disconnected from Study eero Pro 6.",
        ]
    );
    assert!(
        under(&laptop, "What Happened")
            .iter()
            .any(|fact| fact["said"]
                .as_str()
                .unwrap_or("")
                .contains("joined the home network")),
        "a notice the daemon told of it"
    );
    model.told.clear();
    model.events = vec![Event {
        at: "2026-11-07T22:50:00Z".to_owned(),
        kind: "Device".to_owned(),
        text: "laptop computer 1 joined, then phone 1.".to_owned(),
    }];
    assert_eq!(
        under(
            &drawn(&model, &device(&model, "laptop computer 1")),
            "What Happened"
        ),
        [json!({
            "name": {"at": ms("2026-11-07T22:50:00Z"), "as": "moment"},
            "said": "laptop\u{a0}computer\u{a0}1\u{200b} joined, then phone\u{a0}1.",
            "words": true,
        })]
    );
}
