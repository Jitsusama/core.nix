//! The devices' cases, apart from devices.rs so it keeps to its length:
//! what each record says of its node, its card and the default metadata,
//! and what's let go.

use super::*;

fn node(id: u32, name: &str, class: &str) -> Change {
    Change::Node {
        id,
        properties: BTreeMap::from([
            ("node.name".to_owned(), name.to_owned()),
            ("node.description".to_owned(), format!("{name}, described")),
            ("media.class".to_owned(), class.to_owned()),
        ]),
    }
}

fn chose(key: &str, name: &str) -> Change {
    Change::Chose {
        key: Some(key.to_owned()),
        value: Some(format!(r#"{{"name":"{name}"}}"#)),
    }
}

fn keys(devices: &Devices) -> Vec<String> {
    devices
        .records()
        .iter()
        .map(|record| record["key"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn outputs_and_inputs_are_devices_and_streams_arent() {
    let mut devices = Devices::default();
    assert!(
        devices.hear(node(40, "speakers", "Audio/Sink")),
        "an output is a device"
    );
    assert!(
        devices.hear(node(41, "microphone", "Audio/Source")),
        "an input is a device"
    );
    assert!(
        devices.hear(node(42, "spotify", "Stream/Output/Audio")),
        "a program's stream is heard"
    );
    assert!(
        !devices.hear(node(43, "camera", "Video/Source")),
        "a camera is neither"
    );
    let heard = ["device:speakers", "device:microphone", "stream:42"];
    assert_eq!(keys(&devices), heard, "a program's stream isn't a device");
    let records = devices.records();
    assert_eq!(records[0]["sink"], json!(true));
    assert_eq!(records[1]["sink"], json!(false));
    assert_eq!(records[0]["description"], json!("speakers, described"));
}

#[test]
fn a_volume_is_said_as_the_shell_shows_it() {
    let mut devices = Devices::default();
    devices.hear(node(40, "speakers", "Audio/Sink"));
    let heard = devices.hear(Change::Levels {
        id: 40,
        volumes: Some(vec![0.125, 0.125]),
        muted: Some(true),
    });
    assert!(heard, "a device's levels change its record");
    let record = &devices.records()[0];
    assert_eq!(record["volume"], json!(0.5));
    assert_eq!(record["volumes"], json!([0.125, 0.125]));
    assert_eq!(record["muted"], json!(true));
    // A mute alone keeps the volumes said before it.
    devices.hear(Change::Levels {
        id: 40,
        volumes: None,
        muted: Some(false),
    });
    let record = &devices.records()[0];
    assert_eq!(
        (record["volume"].clone(), record["muted"].clone()),
        (json!(0.5), json!(false))
    );
    let heard = devices.hear(Change::Levels {
        id: 99,
        volumes: Some(vec![1.0]),
        muted: None,
    });
    assert!(!heard, "levels of a node not heard change nothing");
}

#[test]
fn a_device_says_which_channel_each_volume_is_of() {
    let mut devices = Devices::default();
    devices.hear(node(40, "speakers", "Audio/Sink"));
    let stereo = || Change::Channels {
        id: 40,
        channels: vec![3, 4],
    };
    assert!(
        devices.hear(stereo()),
        "a device's channels change its record"
    );
    assert_eq!(devices.records()[0]["channels"], json!([3, 4]));
    assert!(
        !devices.hear(stereo()),
        "the same channels again are no news"
    );
    let elsewhere = Change::Channels {
        id: 99,
        channels: vec![2],
    };
    assert!(!devices.hear(elsewhere), "a node not heard changes nothing");
}

// What a device is doing and the format the graph moves its audio in
// are its drawer's, said in PipeWire's words; each said again is no news.
#[test]
fn a_device_says_what_its_doing_and_in_what_format() {
    let mut devices = Devices::default();
    devices.hear(node(40, "speakers", "Audio/Sink"));
    assert_eq!(devices.records()[0]["state"], json!(""));
    let running = || Change::State {
        id: 40,
        state: "running",
    };
    assert!(devices.hear(running()), "a device's state moves its record");
    assert!(!devices.hear(running()), "said again, it moves nothing");
    let format = || Change::Format {
        id: 40,
        format: Some(Format {
            rate: 48_000,
            sample: "S24_32LE".into(),
        }),
    };
    assert!(devices.hear(format()), "a device's format moves its record");
    assert!(!devices.hear(format()), "said again, it moves nothing");
    let record = &devices.records()[0];
    assert_eq!(
        (record["state"].clone(), record["format"].clone()),
        (
            json!("running"),
            json!({"rate": 48_000, "sample": "S24_32LE"})
        )
    );
    assert_eq!(said_state(&NodeState::Suspended), "suspended");
    assert_eq!(said_state(&NodeState::Error("no card")), "error");
}

#[test]
fn a_streams_target_is_where_the_default_metadata_sends_it() {
    let mut devices = Devices::default();
    let sent = |stream, target: Option<&str>| Change::Sent {
        stream,
        target: target.map(str::to_owned),
    };
    let targets = |devices: &Devices| devices.targets()["targets"].clone();
    assert_eq!(devices.targets()["key"], "targets:default");
    assert_eq!(targets(&devices), json!({}));
    let to_headphones = || sent(Some(93), Some("headphones"));
    assert!(devices.hear(to_headphones()), "a send is news");
    assert!(!devices.hear(to_headphones()), "the same send isn't");
    devices.hear(sent(Some(94), Some("61")));
    assert_eq!(targets(&devices), json!({"93": "headphones", "94": "61"}));
    // Sent nowhere, or let go of, it follows the default again.
    devices.hear(sent(Some(94), Some("-1")));
    devices.hear(sent(Some(95), Some("speakers")));
    devices.hear(sent(Some(95), None));
    assert_eq!(targets(&devices), json!({"93": "headphones"}));
    let gone = devices.hear(sent(None, None));
    assert!(gone, "the metadata gone lets each go");
    assert_eq!(targets(&devices), json!({}));
    assert_eq!(devices.records(), Vec::<Value>::new());
}

#[test]
fn the_chosen_are_those_the_default_metadata_names() {
    let mut devices = Devices::default();
    devices.hear(node(40, "speakers", "Audio/Sink"));
    devices.hear(node(43, "headphones", "Audio/Sink"));
    devices.hear(node(41, "microphone", "Audio/Source"));
    devices.hear(chose("default.audio.sink", "headphones"));
    devices.hear(chose("default.audio.source", "microphone"));
    assert!(
        !devices.hear(chose("default.video.source", "camera")),
        "a video default chooses no sound device"
    );
    let chosen = |devices: &Devices| -> Vec<bool> {
        devices
            .records()
            .iter()
            .map(|record| record["chosen"].as_bool().unwrap())
            .collect()
    };
    // By id: the speakers 40, the microphone 41, the headphones 43.
    assert_eq!(keys(&devices)[2], "device:headphones");
    assert_eq!(chosen(&devices), [false, true, true]);
    devices.hear(Change::Chose {
        key: None,
        value: None,
    });
    assert_eq!(chosen(&devices), [false, false, false]);
}

#[test]
fn an_act_finds_the_route_a_card_plays_its_device_through() {
    let mut devices = Devices::default();
    let Change::Node { mut properties, .. } = node(40, "speakers", "Audio/Sink") else {
        unreachable!("node says a node");
    };
    properties.insert("device.id".into(), "7".into());
    properties.insert("card.profile.device".into(), "1".into());
    devices.hear(Change::Node { id: 40, properties });
    let routed = |devices: &Devices| devices.device("device:speakers").and_then(|d| d.route);
    assert_eq!(routed(&devices), None);
    let route = Change::Route {
        card: 7,
        index: 2,
        device: 1,
    };
    assert!(
        devices.hear(route.clone()),
        "a route moves its device's port"
    );
    assert!(!devices.hear(route), "said again, it moves nothing");
    assert_eq!(routed(&devices), Some((7, 2, 1)));
    devices.hear(Change::Gone { id: 7 });
    assert_eq!(routed(&devices), None);
    assert_eq!(devices.device("device:headphones"), None);
}

// What a card says it is, as BlueZ says a Bluetooth one is, reaches
// the records of the devices it plays, and only a change to it moves
// them.
#[test]
fn a_device_says_what_its_card_is() {
    let mut devices = Devices::default();
    let Change::Node { mut properties, .. } = node(40, "headphones", "Audio/Sink") else {
        unreachable!("node says a node");
    };
    properties.insert("device.id".into(), "7".into());
    devices.hear(Change::Node { id: 40, properties });
    let card = |form: &str| Change::Card {
        id: 7,
        properties: BTreeMap::from([
            ("device.form-factor".to_owned(), form.to_owned()),
            ("device.icon-name".to_owned(), "audio-headphones".to_owned()),
        ]),
    };
    assert!(
        devices.hear(card("headphone")),
        "a card saying what it is moves its device"
    );
    assert_eq!(devices.records()[0]["form"], "headphone audio-headphones");
    assert!(
        !devices.hear(card("headphone")),
        "said again, it moves nothing"
    );
    let other = Change::Card {
        id: 8,
        properties: BTreeMap::new(),
    };
    assert!(!devices.hear(other), "a card playing no device moves none");
    devices.hear(Change::Gone { id: 7 });
    assert_eq!(devices.records()[0]["form"], "");
}

// The laptop's card says its headphones' jack has nothing in it while
// none are plugged in, and can't tell of its speakers, which are
// always there.
#[test]
fn a_device_says_whether_anything_is_plugged_in_where_it_plays() {
    let mut devices = Devices::default();
    for (id, name, device) in [(40, "speakers", "0"), (41, "headphones", "1")] {
        let Change::Node { mut properties, .. } = node(id, name, "Audio/Sink") else {
            unreachable!("node says a node");
        };
        properties.insert("device.id".into(), "7".into());
        properties.insert("card.profile.device".into(), device.into());
        devices.hear(Change::Node { id, properties });
    }
    devices.hear(node(42, "usb-desk", "Audio/Sink"));
    let offer = |index: i32, unplugged: bool| Change::Offer {
        card: 7,
        port: Port {
            index,
            devices: vec![index],
            available: if unplugged { "no" } else { "unknown" },
            ..Port::default()
        },
    };
    let unplugged = |devices: &Devices| -> Vec<bool> {
        let records = devices.records();
        records
            .iter()
            .map(|record| record["unplugged"] == true)
            .collect()
    };
    assert_eq!(unplugged(&devices), [false, false, false]);
    assert!(
        devices.hear(offer(0, false)),
        "a route offered moves its device"
    );
    assert!(
        devices.hear(offer(1, true)),
        "an empty jack moves its device"
    );
    assert_eq!(unplugged(&devices), [false, true, false]);
    assert!(
        !devices.hear(offer(1, true)),
        "said again, it moves nothing"
    );
    devices.hear(offer(1, false));
    assert_eq!(unplugged(&devices), [false, false, false]);
    devices.hear(offer(1, true));
    devices.hear(Change::Gone { id: 7 });
    assert_eq!(unplugged(&devices), [false, false, false]);
}

// The laptop's analog output plays its speakers or its headphones,
// one port each way, and offers a microphone port too, which isn't
// the output's; a port of another profile isn't either.
#[test]
fn a_device_says_the_ports_its_card_offers_it() {
    let mut devices = Devices::default();
    let Change::Node { mut properties, .. } = node(40, "analog", "Audio/Sink") else {
        unreachable!("node says a node");
    };
    properties.insert("device.id".into(), "7".into());
    properties.insert("card.profile.device".into(), "0".into());
    devices.hear(Change::Node { id: 40, properties });
    let port = |index: i32, sink: bool, name: &str, available: &'static str| Change::Offer {
        card: 7,
        port: Port {
            index,
            sink: Some(sink),
            name: name.into(),
            devices: vec![0],
            profiles: vec![if index == 3 { 2 } else { 1 }],
            available,
        },
    };
    devices.hear(port(0, true, "Speakers", "unknown"));
    devices.hear(port(1, true, "Headphones", "no"));
    devices.hear(port(2, false, "Microphone", "unknown"));
    devices.hear(port(3, true, "Line Out", "yes"));
    devices.hear(Change::Profile { card: 7, index: 1 });
    assert!(
        devices.hear(Change::Route {
            card: 7,
            index: 0,
            device: 0
        }),
        "a port chosen moves its device"
    );
    assert_eq!(
        devices.records()[0]["ports"],
        json!([
            {"index": 0, "name": "Speakers", "available": "unknown", "chosen": true},
            {"index": 1, "name": "Headphones", "available": "no", "chosen": false},
        ])
    );
    let found = devices.device("device:analog").map(|device| device.ports);
    assert_eq!(found, Some(vec![0, 1]));
}

#[test]
fn a_node_gone_or_no_longer_a_device_is_let_go() {
    let mut devices = Devices::default();
    devices.hear(node(40, "speakers", "Audio/Sink"));
    devices.hear(node(41, "microphone", "Audio/Source"));
    assert!(
        devices.hear(Change::Gone { id: 40 }),
        "a device gone is let go"
    );
    assert!(
        !devices.hear(Change::Gone { id: 40 }),
        "a device gone twice changes nothing"
    );
    assert!(
        devices.hear(node(41, "microphone", "Video/Source")),
        "a device become a video node is let go"
    );
    assert_eq!(keys(&devices), Vec::<String>::new());
    devices.hear(node(44, "speakers", "Audio/Sink"));
    devices.forget();
    assert_eq!(keys(&devices), Vec::<String>::new());
}
