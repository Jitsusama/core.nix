use std::collections::BTreeSet;

use super::*;
use crate::golden::MODEL;
use crate::model::{Outage, Probe};

/// The stand-in model, read at the contract's time.
fn model() -> HomeNetwork {
    serde_json::from_str(MODEL).unwrap()
}

/// When the stand-in was read, 2026-11-07T22:54:43Z.
const READ: i64 = 1_794_092_083;

#[test]
fn the_stand_in_is_read_when_the_tests_say() {
    assert_eq!(time::from_unix(READ), crate::golden::READ);
}

/// The model read `secs` after the stand-in.
fn later(secs: i64) -> HomeNetwork {
    let mut model = model();
    model.read = time::from_unix(READ + secs);
    if let Some(probe) = &mut model.internet.probe {
        probe.at = time::from_unix(READ + secs);
    }
    model
}

/// The model `secs` after the stand-in with eero unreachable and this
/// machine's look at home finding the internet `reachable` or not.
fn looked(secs: i64, reachable: bool) -> HomeNetwork {
    let mut model = model();
    model.cloud.state = CloudState::Unreachable;
    model.internet.probe = Some(Probe {
        at: time::from_unix(READ + secs),
        home: true,
        gateway_ms: Some(2.0),
        internet_ms: reachable.then_some(11.9),
        dns_ms: reachable.then_some(14.1),
        reachable,
    });
    model
}

/// A history stand-in: what was told, and an outage still told out.
#[derive(Default)]
struct Book {
    told: BTreeSet<Told>,
    out_since: Option<String>,
}

impl Book {
    fn with(told: &[Told]) -> Book {
        Book {
            told: told.iter().cloned().collect(),
            out_since: None,
        }
    }
}

impl Said for Book {
    fn said(&self, kind: Kind, subject: &str) -> bool {
        self.told.contains(&(kind, subject.to_owned()))
    }

    fn out_since(&self) -> Option<String> {
        self.out_since.clone()
    }
}

fn said(notices: &[Notice]) -> Vec<(&str, &str, Urgency)> {
    notices
        .iter()
        .map(|n| (n.summary.as_str(), n.body.as_str(), n.urgency))
        .collect()
}

#[test]
fn nothing_changing_says_nothing() {
    assert_eq!(between(&model(), &later(30), &Book::default()), vec![]);
}

#[test]
fn the_internet_out_from_home_is_told_as_this_machine_saw_it() {
    let out = between(&model(), &looked(30, false), &Book::default());

    assert_eq!(
        said(&out),
        [(
            "The internet is out",
            "Nothing past the home network answers this machine.",
            Urgency::Critical
        )]
    );
    assert_eq!(
        out[0].tells,
        [(Kind::InternetOut, time::from_unix(READ + 30))]
    );
    assert_eq!(out[0].about.as_deref(), Some("internet"));
}

#[test]
fn the_internet_out_is_told_as_eero_says_it() {
    let mut disconnected = later(30);
    disconnected.internet.state = Some("disconnected".into());
    let mut provider = later(30);
    provider.internet.isp_up = Some(false);

    let told = [disconnected, provider].map(|now| between(&model(), &now, &Book::default()));

    assert_eq!(told[0][0].body, "eero says the internet is disconnected.");
    assert_eq!(told[1][0].body, "eero says the internet provider is down.");
}

#[test]
fn the_internet_back_says_how_long_it_was_out() {
    let book = Book {
        out_since: Some(time::from_unix(READ + 30)),
        ..Book::default()
    };

    let back = between(&looked(240, false), &looked(300, true), &book);

    assert_eq!(
        said(&back),
        [(
            "The internet is back",
            "It was out for 4 min 30 s.",
            Urgency::Normal
        )]
    );
    assert_eq!(
        back[0].tells,
        [(Kind::InternetBack, time::from_unix(READ + 30))]
    );
}

#[test]
fn the_internet_back_is_timed_from_the_outage_this_machine_saw() {
    let mut now = looked(300, true);
    now.internet.history.outages.insert(
        0,
        Outage {
            seen_by: OutageSeenBy::Probe,
            start: Some(time::from_unix(READ + 100)),
            end: Some(time::from_unix(READ + 300)),
            eero: None,
        },
    );

    let back = between(&looked(270, false), &now, &Book::default());

    assert_eq!(back[0].body, "It was out for 3 min 20 s.");
}

#[test]
fn an_outage_already_told_is_not_told_again() {
    let book = Book {
        out_since: Some(time::from_unix(READ + 30)),
        ..Book::default()
    };
    let mut away = looked(60, false);
    away.internet.probe.as_mut().unwrap().home = false;

    let still = between(&looked(30, false), &looked(60, false), &book);
    let again = between(&away, &looked(90, false), &book);

    assert_eq!(still, vec![]);
    assert_eq!(again, vec![], "seen out again after nothing could tell");
}

#[test]
fn away_from_home_with_eero_unreachable_says_nothing() {
    let mut away = looked(30, false);
    away.internet.probe.as_mut().unwrap().home = false;

    assert_eq!(between(&model(), &away, &Book::default()), vec![]);
}

fn den(model: &mut HomeNetwork) -> &mut Node {
    model
        .nodes
        .iter_mut()
        .find(|node| node.id == "2001")
        .unwrap()
}

#[test]
fn a_node_offline_is_told_by_its_location_with_what_it_held() {
    let mut now = later(30);
    den(&mut now).state = Some("OFFLINE".into());

    let out = between(&model(), &now, &Book::default());

    assert_eq!(
        said(&out),
        [(
            "Den is offline",
            "4 devices were on it. Its backhaul was Wi-Fi from Hall.",
            Urgency::Critical
        )]
    );
    assert_eq!(
        out[0].tells,
        [(Kind::NodeOffline, "2001 2026-11-07T22:54:00.584Z".into())]
    );
}

#[test]
fn the_gateway_node_offline_says_the_internet_comes_through_it() {
    let mut now = later(30);
    now.nodes[0].state = Some("OFFLINE".into());

    let out = between(&model(), &now, &Book::default());

    assert_eq!(out[0].summary, "Hall is offline");
    assert_eq!(
        out[0].body,
        "2 devices were on it. The internet comes in through it."
    );
}

#[test]
fn a_node_back_online_says_how_long_it_was_offline() {
    let mut was = later(300);
    den(&mut was).state = Some("OFFLINE".into());

    let back = between(&was, &later(330), &Book::default());

    assert_eq!(
        said(&back),
        [(
            "Den is back online",
            "It was offline for 6 min 13 s. 4 devices are on it.",
            Urgency::Normal
        )]
    );
    assert_eq!(back[0].about.as_deref(), Some("node 2001"));
}

#[test]
fn a_change_told_before_is_not_told_again() {
    let mut now = later(30);
    den(&mut now).state = Some("OFFLINE".into());
    let book = Book::with(&[(Kind::NodeOffline, "2001 2026-11-07T22:54:00.584Z".into())]);

    assert_eq!(between(&model(), &now, &book), vec![]);
}

fn stranger(first_seen: i64) -> Device {
    let mut device = model().devices[11].clone();
    device.mac = "02:00:00:00:00:20".into();
    device.name = Some("phone 3".into());
    device.first_seen = Some(time::from_unix(first_seen));
    device
}

#[test]
fn a_device_never_seen_before_is_told_with_the_node_it_joined() {
    let mut now = later(30);
    now.devices.push(stranger(READ + 12));

    let joined = between(&model(), &now, &Book::default());

    assert_eq!(
        said(&joined),
        [(
            "phone 3 joined the home network",
            "Never seen before, on Study over Wi-Fi at 5 GHz.",
            Urgency::Normal
        )]
    );
    assert_eq!(
        joined[0].tells,
        [(
            Kind::Joined,
            format!("02:00:00:00:00:20 {}", time::from_unix(READ + 12))
        )]
    );
}

#[test]
fn a_device_over_ethernet_says_so() {
    let mut device = stranger(READ + 12);
    device.ethernet = true;
    let mut now = later(30);
    now.devices.push(device);

    let joined = between(&model(), &now, &Book::default());

    assert_eq!(joined[0].body, "Never seen before, on Study over Ethernet.");
}

#[test]
fn a_device_coming_back_says_nothing() {
    let mut was = model();
    was.devices
        .retain(|device| device.mac != "02:00:00:00:00:0e");
    let mut before = later(30);
    before.devices.push(stranger(READ - 3_600));

    assert_eq!(between(&was, &later(30), &Book::default()), vec![]);
    assert_eq!(between(&model(), &before, &Book::default()), vec![]);
}

#[test]
fn a_firmware_update_waiting_says_what_and_when() {
    let mut now = later(30);
    now.internet.firmware.has_update = Some(true);
    now.internet.firmware.next = Some("v7.18.0-1".into());

    let waiting = between(&model(), &now, &Book::default());

    assert_eq!(
        said(&waiting),
        [(
            "A firmware update is waiting",
            "eero updates the nodes to v7.18.0-1 at 02:00.",
            Urgency::Normal
        )]
    );
    assert_eq!(
        waiting[0].tells,
        [(Kind::UpdateWaiting, "v7.18.0-1".into())]
    );
}

#[test]
fn nodes_updating_together_are_told_once() {
    let mut all = later(30);
    for node in &mut all.nodes {
        node.firmware = Some("v7.18.0-1".into());
    }
    let mut one = later(30);
    den(&mut one).firmware = Some("v7.18.0-1".into());

    let told = [all, one].map(|now| between(&model(), &now, &Book::default()));

    assert_eq!(
        said(&told[0]),
        [(
            "The nodes updated to v7.18.0-1",
            "They were on v7.17.4-5.",
            Urgency::Normal
        )]
    );
    assert_eq!(told[0][0].tells.len(), 3);
    assert_eq!(told[1][0].summary, "Den updated to v7.18.0-1");
    assert_eq!(told[1][0].body, "It was on v7.17.4-5.");
}

#[test]
fn a_node_already_told_updated_is_left_out() {
    let mut now = later(30);
    for node in &mut now.nodes {
        node.firmware = Some("v7.18.0-1".into());
    }
    let book = Book::with(&[(Kind::Updated, "2001 v7.18.0-1".into())]);

    let told = between(&model(), &now, &book);

    assert_eq!(told[0].summary, "Hall and Study updated to v7.18.0-1");
}

fn event(secs: i64, kind: &str, text: &str) -> Event {
    Event {
        at: time::from_unix(READ + secs),
        kind: kind.into(),
        text: text.into(),
    }
}

#[test]
fn an_event_is_told_only_where_it_says_something_new() {
    let mut now = later(900);
    let mut events = vec![
        event(630, "Network", "Guest network turned on."),
        event(620, "Network", "Your internet connection was restored."),
        event(610, "eero", "Den eero went offline."),
        event(600, "Device", "phone 1 connected to Hall eero Pro 6."),
    ];
    events.append(&mut now.events);
    now.events = events;

    let told = between(&model(), &now, &Book::default());

    assert_eq!(
        said(&told),
        [(
            "Guest network turned on.",
            "eero's activity list, under Network.",
            Urgency::Normal
        )]
    );
}

#[test]
fn an_event_already_in_the_list_or_in_the_first_list_says_nothing() {
    let mut was = model();
    was.events
        .insert(0, event(600, "Network", "Guest network turned on."));
    let mut none = model();
    none.events.clear();

    assert_eq!(between(&was, &was, &Book::default()), vec![]);
    assert_eq!(between(&none, &was, &Book::default()), vec![]);
}

#[test]
fn nothing_is_told_until_eero_answers_nor_on_its_first_answer() {
    let mut watch = Watch::default();
    let mut offline = later(60);
    den(&mut offline).state = Some("OFFLINE".into());
    let book = Book::default();

    let stale = watch.next(&model(), false, &book);
    let first = watch.next(&later(30), true, &book);
    let then = watch.next(&offline, true, &book);

    assert_eq!(stale, Next::default());
    assert_eq!(first, Next::default());
    assert_eq!(said(&then.notices)[0].0, "Den is offline");
}

#[test]
fn an_outage_told_before_a_start_that_ended_unwatched_is_settled() {
    let mut watch = Watch::default();
    let book = Book {
        out_since: Some(time::from_unix(READ - 600)),
        ..Book::default()
    };

    let first = watch.next(&model(), true, &book);

    assert_eq!(first.notices, vec![]);
    assert_eq!(
        first.settled,
        [(Kind::InternetBack, time::from_unix(READ - 600))]
    );
}
