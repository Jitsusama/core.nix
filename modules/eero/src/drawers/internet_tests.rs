//! The internet's drawer the eero daemon serves, held to the figures the
//! shell's own drawer said of `home-network.json` (tests/home-network.qml
//! until the daemon served it), in the launcher's words, its times left
//! for the shell to word as it draws.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::super::node::tests::under;
use super::*;
use crate::model::{Outage, OutageSeenBy};
use crate::rows::tests::{model, words};

fn drawn(model: &HomeNetwork) -> Value {
    drawer(model, &words(), &Numbers::default())
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

#[test]
fn its_eyebrow_is_eeros_and_its_state_is_the_rows() {
    let drawer = drawn(&model());
    assert_eq!(
        [
            &drawer["eyebrow"],
            &drawer["eyebrowNames"],
            &drawer["title"]
        ],
        [
            &json!("via eero · internet"),
            &json!(true),
            &json!("Internet")
        ],
        "the eyebrow says it's eero's internet, who carries it a fact of its own"
    );
    assert_eq!(
        named_fact(&drawer, "State"),
        None,
        "its row's side says how it stands"
    );
}

#[test]
fn every_figure_the_contract_places_there_is_said() {
    let drawer = drawn(&model());
    for (name, wanted) in [
        ("Home Network", "connected"),
        ("Provider", "Example ISP \u{b7} AS64496"),
        ("Public Address", "203.0.113.1"),
        (
            "Lease",
            "DHCP \u{b7} 203.0.113.1/23 \u{b7} router 203.0.113.2",
        ),
        ("Gateway Node", "10.0.0.1"),
        ("Mode", "NAT \u{b7} addresses automatic"),
        (
            "DNS",
            "automatic \u{b7} 203.0.113.3 \u{b7} 203.0.113.4 \u{b7} caching off \u{b7} \
             IPv6 automatic",
        ),
        ("IPv6", "off"),
        (
            "Main Subnet",
            "10.0.0.0/22 \u{b7} DHCP 10.0.0.20 to 10.0.0.254",
        ),
        (
            "Guest Subnet",
            "off \u{b7} 10.0.0.0/21 \u{b7} DHCP 10.0.0.20 to 10.0.0.254",
        ),
        ("Port Forwards", "none"),
        ("IPv6 Pinholes", "none"),
        ("Down", "370 Mb/s"),
        ("Up", "11.9 Mb/s"),
        ("Conferencing and Gaming", "optimized"),
        ("UPnP", "on"),
        ("Port Randomization", "off"),
        ("Band Steering", "on"),
        ("WPA3", "off"),
        ("Wi-Fi Security", "WPA2"),
        ("Wi-Fi Mode", "default"),
        ("Wi-Fi 5 Compatibility", "off"),
        ("Name Clash", "none seen"),
        ("Multi-Link Operation", "off"),
        ("Thread", "off \u{b7} name 56 \u{b7} channel 25"),
        ("Guest Network", "off \u{b7} name 49"),
        ("Dynamic DNS", "off"),
        ("Backup Internet", "on"),
        ("Power Saving", "off"),
        ("Outages", "1 outage in 30 days"),
        ("To the Gateway Node", "2.00 ms"),
        ("To the Internet", "11.9 ms"),
        ("Name Lookup", "14.1 ms"),
        ("Nodes", "3 nodes online"),
        ("Firmware", "v7.17.4-5 \u{b7} up to date"),
        ("Traffic", "167 GB down \u{b7} 72.7 GB up"),
    ] {
        assert_eq!(
            said(&drawer, name).as_deref(),
            Some(wanted),
            "the internet's {name}"
        );
    }
}

#[test]
fn its_times_are_left_for_the_shell_to_word() {
    let drawer = drawn(&model());
    for (name, at, wanted) in [
        (
            "Speed Test",
            "2026-11-05T10:52:15Z",
            "370 Mb/s down \u{b7} 11.9 Mb/s up \u{b7} {time}",
        ),
        (
            "Kept",
            "2026-09-24T09:53:31Z",
            "15 speed tests since {time}",
        ),
        (
            "Updates",
            "2026-11-07T07:19:15Z",
            "at 02:00 \u{b7} last {time}",
        ),
        (
            "Last Read",
            "2026-11-07T22:54:43Z",
            "{time} \u{b7} answering",
        ),
        (
            "Last Look",
            "2026-11-07T22:54:43Z",
            "{time} \u{b7} from home",
        ),
    ] {
        let fact = named_fact(&drawer, name).expect("the fact is there");
        assert_eq!(
            [&fact["at"], &fact["as"], &fact["said"]],
            [&json!(ms(at)), &json!("moment"), &json!(wanted)],
            "the internet's {name}"
        );
    }
}

#[test]
fn it_leads_with_what_its_row_says_then_each_group() {
    let drawer = drawn(&model());
    let names: Vec<&Value> = facts(&drawer)
        .iter()
        .take(6)
        .map(|fact| &fact["name"])
        .collect();
    assert_eq!(
        names,
        [
            "Speed Test",
            "Nodes",
            "Outages",
            "Provider",
            "Home Network",
            "Firmware"
        ]
        .map(|name| json!(name))
        .iter()
        .collect::<Vec<_>>(),
        "what a narrow drawer has room for leads"
    );
    let headings: Vec<&Value> = facts(&drawer)
        .iter()
        .filter_map(|fact| fact.get("heading"))
        .collect();
    assert_eq!(
        headings,
        [
            "From This Machine",
            "Outages in the Last 30 Days",
            "Speed Tests",
            "Addressing",
            "Settings",
            "What Happened"
        ]
        .map(|heading| json!(heading))
        .iter()
        .collect::<Vec<_>>(),
        "the internet's groups"
    );
}

#[test]
fn no_device_is_named_under_the_internet() {
    let model = model();
    let drawer = drawn(&model);
    let says = facts(&drawer)
        .iter()
        .map(|fact| format!("{} {}", fact["name"], fact["said"]))
        .collect::<Vec<_>>()
        .join("\n")
        .replace(['\u{a0}', '\u{200b}'], " ")
        .to_lowercase();
    let named: Vec<&str> = model
        .devices
        .iter()
        .flat_map(|one| [one.name.as_deref().unwrap_or(&one.mac), &one.mac])
        .filter(|said| says.contains(&said.to_lowercase()))
        .collect();
    assert_eq!(
        named,
        Vec::<&str>::new(),
        "each device's drawer says its own"
    );
}

#[test]
fn each_target_is_sparked_over_the_hours_looks() {
    let model = model();
    let drawer = drawn(&model);
    let values = |name| {
        named_fact(&drawer, name).expect("the fact is there")["figure"]["values"]
            .as_array()
            .expect("a spark's places")
            .clone()
    };
    let gateway: Vec<f64> = values("To the Gateway Node")
        .iter()
        .map(|share| (share.as_f64().expect("a share") * 24.0).round() / 10.0)
        .collect();
    assert_eq!(
        gateway,
        [2.1, 1.8, 2.4, 1.9, 2.2, 2.0],
        "scaled to its most"
    );
    assert_eq!(values("Name Lookup").len(), 6, "each target sparked");

    let mut gap = model.clone();
    gap.internet.history.probes.internet_ms = vec![Some(13.1), None, Some(13.1)];
    let drawer = drawn(&gap);
    let fact = named_fact(&drawer, "To the Internet").expect("the fact is there");
    assert_eq!(
        fact["figure"]["values"],
        json!([1.0, null, 1.0]),
        "a look unanswered is a gap"
    );
}

#[test]
fn a_look_from_away_says_the_internet_was_not_reached() {
    let mut away = model();
    let probe = away.internet.probe.as_mut().expect("the fixture looked");
    probe.home = false;
    probe.gateway_ms = None;
    probe.internet_ms = None;
    probe.reachable = false;
    let drawer = drawn(&away);
    assert_eq!(
        said(&drawer, "Last Look").as_deref(),
        Some("{time} \u{b7} away from home \u{b7} internet not reached"),
        "where it looked from, and that it didn't reach the internet"
    );
    assert_eq!(
        [
            named_fact(&drawer, "To the Gateway Node"),
            named_fact(&drawer, "To the Internet")
        ],
        [None, None],
        "no time where none answered"
    );
}

#[test]
fn each_outage_says_when_how_long_and_who_saw_it() {
    let model = model();
    assert_eq!(
        under(&drawn(&model), "Outages in the Last 30 Days"),
        [json!({
            "name": {"at": ms("2026-11-03T03:12:40Z"), "as": "moment"},
            "said": "1 min \u{b7} seen by this machine",
        })],
        "an outage this machine saw"
    );
    let mut going = model.clone();
    going.internet.history.outages = vec![
        Outage {
            seen_by: OutageSeenBy::Probe,
            start: Some("2026-11-07T22:50:00Z".into()),
            end: None,
            eero: None,
        },
        Outage {
            seen_by: OutageSeenBy::Eero,
            start: None,
            end: None,
            eero: Some(json!({"kind": "lte"})),
        },
    ];
    assert_eq!(
        under(&drawn(&going), "Outages in the Last 30 Days"),
        [
            json!({
                "name": {"at": ms("2026-11-07T22:50:00Z"), "as": "moment"},
                "at": ms("2026-11-07T22:50:00Z"),
                "as": "lasted",
                "said": "{time} so far \u{b7} seen by this machine",
            }),
            json!({"name": "time not given", "said": "seen by eero"}),
        ],
        "an outage going on, and one of eero's"
    );
}

#[test]
fn an_ended_outage_says_its_largest_two_units() {
    let words = words();
    for (seconds, wanted) in [
        (30, "under a minute"),
        (90, "1 min"),
        (3 * 3600, "3 h"),
        (3 * 3600 + 120, "3 h 2 min"),
        (2 * 86400 + 3600 + 60, "2 d 1 h"),
        (2 * 86400 + 60, "2 d"),
    ] {
        assert_eq!(lasted(seconds * 1000, &words), wanted, "{seconds} s");
    }
}

#[test]
fn a_subnet_eero_has_not_named_is_named_as_eero_names_it() {
    let mut odd = model();
    let mut things = odd.internet.addresses.subnets[1].clone();
    things.kind = Some("iot".into());
    things.name = Some("things".into());
    things.mask = Some("255.0.255.0".into());
    things.on = Some(true);
    things.internet = Some(false);
    things.open = Some(true);
    odd.internet.addresses.subnets = vec![things];
    odd.internet.addresses.reservations = Vec::new();
    assert_eq!(
        said(&drawn(&odd), "things").as_deref(),
        Some(
            "no internet \u{b7} open \u{b7} 10.0.0.0 \u{b7} mask 255.0.255.0 \u{b7} DHCP 10.0.0.20 to 10.0.0.254"
        ),
        "by its name, its mask whole"
    );
}

#[test]
fn bands_that_differ_are_said_band_by_band_and_a_clash_is_said() {
    let mut odd = model();
    odd.internet.security = vec![
        crate::model::BandSecurity {
            band: "2.4 GHz".into(),
            security: "wpa2".into(),
        },
        crate::model::BandSecurity {
            band: "5 GHz".into(),
            security: "wpa3".into(),
        },
    ];
    odd.internet.name_clash = Some(true);
    let drawer = drawn(&odd);
    assert_eq!(
        [said(&drawer, "Wi-Fi Security"), said(&drawer, "Name Clash")],
        [
            Some("2.4 GHz WPA2 \u{b7} 5 GHz WPA3".to_owned()),
            Some("another network has its name".to_owned())
        ],
        "each band's security, and another network with its name"
    );
}

#[test]
fn the_speed_tests_and_the_days_traffic_are_sparked() {
    let drawer = drawn(&model());
    let places = |name| {
        named_fact(&drawer, name).expect("the fact is there")["figure"]["values"]
            .as_array()
            .expect("a spark's places")
            .len()
    };
    assert_eq!(
        [places("Down"), places("Up"), places("Traffic")],
        [15, 15, 20],
        "a spark of every test kept each way, and of the day by the hour"
    );
}

#[test]
fn what_happened_is_the_internets_newest_first() {
    let model = model();
    let drawer = drawn(&model);
    let timeline = under(&drawer, "What Happened");
    assert_eq!(
        timeline.len(),
        4,
        "every notice but a device's, and no device's event"
    );
    assert_eq!(
        timeline[..2],
        [
            json!({
                "name": {"at": ms("2026-11-07T07:24:13Z"), "as": "moment"},
                "said": "The nodes updated to v7.17.4-5. They were on v7.17.3-9.",
                "words": true,
            }),
            json!({
                "name": {"at": ms("2026-11-06T16:21:33Z"), "as": "moment"},
                "said": "A firmware update is waiting. \
                         eero updates the nodes to v7.17.4-5 at 02:00.",
                "words": true,
            }),
        ],
        "a notice by when it was told, then what it said"
    );
    assert_eq!(
        timeline[3]["said"],
        "The internet is out. Nothing past the home network answers this machine.",
        "the oldest last"
    );

    let mut quiet = model.clone();
    quiet.events = Vec::new();
    let mut back = model.told[3].clone();
    back.body = String::new();
    quiet.told = vec![back];
    assert_eq!(
        under(&drawn(&quiet), "What Happened")[0]["said"],
        "The internet is back",
        "a notice with nothing more to say is its summary"
    );
}
