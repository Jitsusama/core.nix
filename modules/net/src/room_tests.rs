//! The tests of room.rs, working the network out: each device's path to
//! the modem, its narrowest hop and the concerns net raises; and the
//! record and the helpers history's tests share.

use super::*;

use crate::history::{Moment, recall};

use crate::model::{ConcernKind, Device, End, Internet, Link, Medium};

const RECORD: &str = include_str!("../tests/network.json");

/// The golden record as the sources said it, before any working out,
/// with the internet as eero last measured it.
pub(crate) fn network() -> Network {
    let mut network: Network = serde_json::from_str(RECORD).expect("the record reads");
    for device in &mut network.devices {
        device.path = None;
    }
    network.concerns.clear();
    network.internet = Some(Internet {
        provider: Some("Example ISP".into()),
        down_mbps: Some(369.86),
        up_mbps: Some(11.93),
        measured_at: Some(1_793_875_935),
        source: "eero".into(),
        carried: None,
    });
    network
}

pub(crate) fn device<'a>(network: &'a Network, id: &str) -> &'a Device {
    network
        .devices
        .iter()
        .find(|device| device.id == id)
        .unwrap_or_else(|| panic!("the record has {id}"))
}

#[test]
fn a_client_goes_through_its_access_point_and_the_gateway_to_the_modem() {
    let mut network = network();
    work_out(&mut network);
    let path = device(&network, "mac:02:00:00:00:00:0e")
        .path
        .as_ref()
        .expect("a client on a satellite has a path");
    assert_eq!(path.through, ["eero:2002", "eero:2003", "modem"]);
}

#[test]
fn the_modem_and_a_device_no_link_reaches_have_no_path() {
    let mut network = network();
    work_out(&mut network);
    assert_eq!(device(&network, "modem").path, None);
    assert_eq!(device(&network, "mac:02:00:00:00:00:30").path, None);
}

#[test]
fn a_wifi_hop_has_its_speed_less_the_time_its_radio_is_busy() {
    // The tablet joins Den's 5 GHz low radio at 48 Mb/s, busy 5%,
    // so 45.6 Mb/s is left, less than the internet's 369.86 down.
    let mut network = network();
    work_out(&mut network);
    let path = device(&network, "mac:02:00:00:00:00:01")
        .path
        .clone()
        .expect("the tablet has a path");
    let down = path.down.expect("its way down has a chokepoint");
    assert_eq!(down.a, "mac:02:00:00:00:00:01");
    assert_eq!(down.b.as_deref(), Some("eero:2001"));
    assert!(
        (down.room_mbps - 45.6).abs() < 0.01,
        "room was {}",
        down.room_mbps
    );
    // Den's backhaul says no speed, so it's counted, not guessed.
    assert_eq!(down.unmeasured, 1);
}

#[test]
fn the_internet_is_the_chokepoint_when_it_has_least_room() {
    let mut network = network();
    work_out(&mut network);
    let path = device(&network, "mac:02:00:00:00:00:01")
        .path
        .clone()
        .expect("the tablet has a path");
    let up = path.up.expect("its way up has a chokepoint");
    assert_eq!((up.a.as_str(), up.b.as_deref()), ("modem", None));
    assert!(
        (up.room_mbps - 11.93).abs() < 0.01,
        "room was {}",
        up.room_mbps
    );
}

#[test]
fn without_the_internets_speed_a_path_still_finds_its_narrowest_link() {
    let mut network = network();
    network.internet = None;
    work_out(&mut network);
    let path = device(&network, "mac:02:00:00:00:00:0a")
        .path
        .clone()
        .expect("the device on Den's quiet radio has a path");
    let up = path.up.expect("its links say speeds");
    assert_eq!(up.a, "eero:2003");
    assert_eq!(up.b.as_deref(), Some("modem"));
    assert_eq!(
        up.room_mbps, 1000.0,
        "the cable is narrower than 1201 less 5%"
    );
    assert_eq!(up.unmeasured, 2, "the backhaul and the internet");
}

/// Sets what one of the modem's downstream channels has lost.
fn lost(network: &mut Network, channel: u32, count: u64) {
    let modem = network
        .devices
        .iter_mut()
        .find(|device| device.id == "modem")
        .and_then(|device| device.modem.as_mut())
        .expect("the record has the modem's channels");
    let channel = modem
        .downstream
        .iter_mut()
        .find(|each| each.channel == Some(channel))
        .expect("the modem has the channel");
    channel.uncorrectable = Some(count);
}

/// Sets a node's 5 GHz high radio busy this share of the time.
pub(crate) fn busy(network: &mut Network, node: &str, pct: u32) {
    let radio = network
        .devices
        .iter_mut()
        .find(|device| device.id == node)
        .and_then(|device| {
            device
                .radios
                .iter_mut()
                .find(|radio| radio.name == "5 GHz high")
        })
        .expect("the node has a 5 GHz high radio");
    radio.busy_pct = Some(pct);
}

/// Cables Hall's port 2 to Closet at 100 Mb/s.
fn slow_cable(network: &mut Network) {
    network.links.push(Link {
        a: End {
            device: "eero:2003".into(),
            port: Some("2".into()),
            radio: None,
        },
        b: End {
            device: "mac:02:00:00:00:00:30".into(),
            port: Some("8".into()),
            radio: None,
        },
        medium: Medium::Ethernet,
        speed_mbps: Some(100),
        channel: None,
        signal_dbm: None,
        source: "eero".into(),
    });
}

#[test]
fn a_device_using_most_of_the_internet_one_way_fills_it() {
    // The record's computer moves 9.86 of the line's 11.93 up; down,
    // 290 of 369.86 is 78%, under the line.
    let mut network = network();
    let computer = network
        .devices
        .iter_mut()
        .find(|device| device.id == "mac:02:00:00:00:00:08")
        .and_then(|device| device.speed.as_mut())
        .expect("the record has the computer's speed");
    computer.down_mbps = Some(290.0);
    work_out(&mut network);
    let fills: Vec<(Option<&str>, &str)> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Fills)
        .map(|concern| (concern.on.as_deref(), concern.says.as_str()))
        .collect();
    assert_eq!(
        fills,
        [(
            Some("up"),
            "unknown computer 1 is using 83% of the internet's speed up: 9.86 Mb/s of its 11.9 \
             Mb/s."
        )],
        "78% of the way down is under the line, and 83% up is over it"
    );
}

#[test]
fn each_radio_busy_half_the_time_or_more_is_a_concern() {
    let mut network = network();
    busy(&mut network, "eero:2003", 64);
    busy(&mut network, "eero:2002", 49);
    busy(&mut network, "eero:2001", 50);
    work_out(&mut network);
    let busy: Vec<(&str, Option<&str>)> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Busy)
        .map(|concern| (concern.device.as_str(), concern.on.as_deref()))
        .collect();
    assert_eq!(
        busy,
        [
            ("eero:2003", Some("5 GHz high")),
            ("eero:2001", Some("5 GHz high")),
        ]
    );
    let hall = network
        .concerns
        .iter()
        .find(|concern| concern.kind == ConcernKind::Busy)
        .expect("Hall's radio is busy");
    assert_eq!(
        hall.says,
        "Hall's 5 GHz high radio, on channel 149, is busy 64% of the time."
    );
}

#[test]
fn a_radio_busy_under_half_the_time_is_no_concern() {
    let mut network = network();
    work_out(&mut network);
    let busy: Vec<_> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Busy)
        .collect();
    assert!(busy.is_empty(), "radios 37% to 41% busy: {busy:?}");
}

#[test]
fn a_channel_losing_far_more_than_the_rest_is_a_concern() {
    // The recorded modem's channel 16 lost 878 it couldn't repair; no
    // other channel lost more than 18.
    let mut network = network();
    work_out(&mut network);
    let lost: Vec<_> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Lost)
        .collect();
    assert_eq!(lost.len(), 1, "one channel stands out: {lost:?}");
    assert_eq!(lost[0].device, "modem");
    assert_eq!(lost[0].on.as_deref(), Some("downstream channel 16"));
    assert_eq!(
        lost[0].says,
        "The modem's downstream channel 16 has lost 878 pieces of data it couldn't repair \
         since the modem started, against at most 18 on any other; if that keeps climbing, \
         Example ISP can check the line."
    );
}

#[test]
fn an_ofdm_channel_losing_most_is_named_by_its_frequency() {
    // Neither OFDM channel has a number, so only the frequency says which
    // one the provider should look at.
    let mut network = network();
    lost(&mut network, 16, 0);
    let modem = network
        .devices
        .iter_mut()
        .find(|device| device.id == "modem")
        .and_then(|device| device.modem.as_mut())
        .expect("the record has the modem's channels");
    modem
        .downstream
        .iter_mut()
        .find(|each| each.frequency_hz == Some(635_600_000))
        .expect("the modem has the 635.6 MHz channel")
        .uncorrectable = Some(900);
    work_out(&mut network);
    let lost = network
        .concerns
        .iter()
        .find(|concern| concern.kind == ConcernKind::Lost)
        .unwrap_or_else(|| panic!("a Lost concern: {:?}", network.concerns));
    assert_eq!(
        lost.on.as_deref(),
        Some("downstream ofdm channel at 635.6 MHz"),
        "the OFDM channel wasn't named by its frequency"
    );
}

#[test]
fn channels_losing_alike_are_no_concern() {
    // 878 against 100 is under ten times, so it's the line, not one
    // frequency; and a channel that's lost few is never a concern.
    let mut alike = network();
    lost(&mut alike, 3, 100);
    work_out(&mut alike);
    assert!(
        !alike
            .concerns
            .iter()
            .any(|concern| concern.kind == ConcernKind::Lost),
        "878 against 100 is no lone channel: {:?}",
        alike.concerns
    );
    let mut few = network();
    lost(&mut few, 16, 90);
    work_out(&mut few);
    assert!(
        !few.concerns
            .iter()
            .any(|concern| concern.kind == ConcernKind::Lost),
        "90 lost is few: {:?}",
        few.concerns
    );
}

#[test]
fn a_name_ending_in_s_takes_the_apostrophe_alone() {
    let mut network = network();
    network.devices[2].name = "Upstairs".into();
    busy(&mut network, "eero:2003", 64);
    work_out(&mut network);
    let busy = network
        .concerns
        .iter()
        .find(|concern| concern.device == "eero:2003")
        .expect("the renamed node's radio is busy");
    assert_eq!(
        busy.says,
        "Upstairs' 5 GHz high radio, on channel 149, is busy 64% of the time."
    );
}

#[test]
fn a_wifi_link_heard_at_minus_70_or_weaker_is_a_concern() {
    let mut network = network();
    work_out(&mut network);
    let weak: Vec<_> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Weak)
        .collect();
    assert_eq!(weak.len(), 1, "only the -75 dBm laptop: {weak:?}");
    assert_eq!(weak[0].device, "mac:02:00:00:00:00:0e");
    assert_eq!(weak[0].other.as_deref(), Some("eero:2002"));
    assert_eq!(
        weak[0].says,
        "laptop computer 1 has a weak signal to Study: \u{2212}75 dBm, and from \u{2212}70 \
         down its speed falls."
    );
}

#[test]
fn a_cable_under_a_gigabit_is_a_concern() {
    let mut network = network();
    slow_cable(&mut network);
    work_out(&mut network);
    let slow: Vec<_> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Slow)
        .collect();
    assert_eq!(slow.len(), 1, "one slow cable: {slow:?}");
    assert_eq!(slow[0].on.as_deref(), Some("2"));
    assert_eq!(slow[0].says, "Hall's cable to Closet runs at 100 Mb/s.");
}

#[test]
fn the_internet_as_the_chokepoint_is_one_concern_each_way() {
    let mut network = network();
    work_out(&mut network);
    let ceilings: Vec<&str> = network
        .concerns
        .iter()
        .filter(|concern| concern.kind == ConcernKind::Ceiling)
        .map(|concern| concern.says.as_str())
        .collect();
    assert_eq!(
        ceilings,
        [
            "The internet is the narrowest point down for 8 of the 19 devices on the eeros: \
             370 Mb/s, as eero last measured it.",
            "The internet is the narrowest point up for every device on the eeros: 11.9 \
             Mb/s, as eero last measured it.",
        ]
    );
}

/// The golden record's figures as they were `ago` seconds before it,
/// with channel 16's losses and Hall's busy 5 GHz high radio as given.
pub(crate) fn earlier(network: &Network, ago: u64, sixteen: u64, hall: u32) -> Moment {
    let mut moment = Moment::of(network);
    moment.at = network.collected_at - ago;
    for lost in &mut moment.lost {
        if lost.channel == Some(16) {
            lost.uncorrectable = sixteen;
        }
    }
    for busy in &mut moment.busy {
        if busy.device == "eero:2003" && busy.radio == "5 GHz high" {
            busy.pct = hall;
        }
    }
    moment
}

/// Whether words are one sentence ending in a single stop: no other stop,
/// question mark, exclamation mark or ellipsis, but for a decimal point
/// between digits.
fn is_one_sentence(says: &str) -> bool {
    let Some(body) = says.strip_suffix('.').filter(|body| !body.is_empty()) else {
        return false;
    };
    let padded: Vec<char> = std::iter::once(' ')
        .chain(body.chars())
        .chain(std::iter::once(' '))
        .collect();
    padded.windows(3).all(|around| match around {
        [before, '.', after] => before.is_ascii_digit() && after.is_ascii_digit(),
        [_, '?' | '!' | '\u{2026}', _] => false,
        _ => true,
    })
}

#[test]
fn one_sentence_ends_once_and_holds_only_decimal_points() {
    let told: Vec<(&str, bool)> = [
        "Up 11.9 Mb/s.",
        "No stop",
        ".",
        "Ends twice..",
        "One. Two.",
        "One.Two.",
        "Asks? No.",
        "Trails\u{2026} off.",
    ]
    .into_iter()
    .map(|says| (says, is_one_sentence(says)))
    .collect();
    assert_eq!(
        told,
        [
            ("Up 11.9 Mb/s.", true),
            ("No stop", false),
            (".", false),
            ("Ends twice..", false),
            ("One. Two.", false),
            ("One.Two.", false),
            ("Asks? No.", false),
            ("Trails\u{2026} off.", false),
        ],
        "only a decimal point may stand inside, and one stop at the end"
    );
}

#[test]
fn every_concern_is_one_whole_sentence_ending_in_a_stop() {
    // The system command says the worst concern among its own sentences,
    // adding a stop only to words without one, and the network page and
    // the Map row show each whole, so each must be one sentence ending
    // once. This holds net's own wording: a device's name is the
    // network's, shown as it was named.
    let mut bare = network();
    busy(&mut bare, "eero:2003", 64);
    slow_cable(&mut bare);
    // A day recalled as the daemon does each minute, so the busiest share
    // is said in minutes and the recent loss in hours.
    let mut recalled = bare.clone();
    let day = [
        earlier(&bare, 5 * 3600, 800, 41),
        earlier(&bare, 30 * 60, 850, 90),
    ];
    recall(&mut recalled, &day);
    // Naming every kind in a match makes a new kind fail to build here,
    // to be added to every as well.
    let every = [
        ConcernKind::Busy,
        ConcernKind::Weak,
        ConcernKind::Slow,
        ConcernKind::Ceiling,
        ConcernKind::Fills,
        ConcernKind::Lost,
    ];
    for kind in every {
        match kind {
            ConcernKind::Busy
            | ConcernKind::Weak
            | ConcernKind::Slow
            | ConcernKind::Ceiling
            | ConcernKind::Fills
            | ConcernKind::Lost => {}
        }
    }
    let mut spliced = Vec::new();
    for mut network in [bare, recalled] {
        work_out(&mut network);
        let missing: Vec<ConcernKind> = every
            .into_iter()
            .filter(|kind| !network.concerns.iter().any(|concern| concern.kind == *kind))
            .collect();
        assert!(
            missing.is_empty(),
            "the record raises no {missing:?}: {:?}",
            network.concerns
        );
        for concern in &network.concerns {
            assert!(
                is_one_sentence(&concern.says),
                "one sentence ending in one stop: {}",
                concern.says
            );
        }
        spliced.push(
            network
                .concerns
                .iter()
                .filter(|concern| {
                    concern.says.contains("at its busiest") || concern.says.contains("in the last")
                })
                .count(),
        );
    }
    assert_eq!(
        spliced,
        [0, 2],
        "a recalled day splices in the busiest share and the recent loss"
    );
}

#[test]
fn working_it_out_twice_gives_the_same_record() {
    let mut once = network();
    work_out(&mut once);
    let mut twice = once.clone();
    work_out(&mut twice);
    assert_eq!(once, twice);
}
