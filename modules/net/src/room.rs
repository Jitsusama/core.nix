//! Where the home network runs out of room, worked out from the figures
//! the sources gave: each device's path to the modem, the hop on it with
//! least room each way, and the concerns worth a look. Nothing here is
//! read from a device; it's arithmetic over the record, so it's the same
//! wherever the record is read.

use std::collections::{HashMap, VecDeque};

use crate::model::{
    Chokepoint, Concern, ConcernKind, Device, Internet, Link, Medium, Network, Path, Role,
};
use crate::sources::modem;

/// A radio busy this share of the time or more is a concern: past it,
/// everything the radio carries waits its turn often enough to be felt,
/// and it's the line the map draws in gold. At 30% every radio in the
/// home crossed it on a quiet night (33% to 46% on 15 Nov), so the gold
/// said nothing, and an engineer reading the map cold called those
/// figures normal; half the time is where waiting starts to show.
const BUSY_PCT: u32 = 50;

/// A Wi-Fi link heard this weakly or worse is a concern, since below it
/// the rate falls and retries climb.
const WEAK_DBM: i32 = -70;

/// A cable link slower than this is a concern: every port in the home
/// does a gigabit, so less is a bad cable or a port that fell back.
const GIGABIT: u32 = 1000;

/// A downstream channel that has lost at least this many pieces of data
/// it couldn't repair, and this many times more than any other channel,
/// is a concern. Losses on every channel alike are the line or the modem;
/// one channel far above the rest is noise on its frequency, which the
/// provider can find. Ten times keeps a little spread from counting, and
/// a hundred keeps a modem that has just started from flagging its first
/// few. On 15 Nov channel 16 had lost 878 and no other more than 18.
const LOST_MANY: u64 = 100;
const LOST_TIMES: u64 = 10;

/// A device using this share of the internet's speed one way, or more,
/// fills it: every other device shares what's left, so a call beside a
/// backup feels the backup. eero's own measure of the line is a sample
/// too, so a share just under the whole is the line full.
const FILLS_PCT: f64 = 80.0;

/// Fills in each device's path and the network's concerns, replacing any
/// worked out before, so the same record gives the same answer.
pub fn work_out(network: &mut Network) {
    let paths: Vec<Option<Path>> = {
        let toward = toward_the_modem(network);
        network
            .devices
            .iter()
            .map(|device| path(network, &toward, &device.id))
            .collect()
    };
    for (device, path) in network.devices.iter_mut().zip(paths) {
        device.path = path;
    }
    network.concerns = crate::model::worst_first(concerns(network));
}

/// For each device the links reach from the modem, the next device on its
/// way there: a breadth-first walk out from the modem, so each takes the
/// fewest hops, and links in the record's order break ties.
fn toward_the_modem(network: &Network) -> HashMap<&str, &str> {
    let mut toward = HashMap::new();
    let mut next = VecDeque::from([modem::ID]);
    while let Some(here) = next.pop_front() {
        for link in &network.links {
            let there = if link.a.device == here {
                link.b.device.as_str()
            } else if link.b.device == here {
                link.a.device.as_str()
            } else {
                continue;
            };
            if there != modem::ID && !toward.contains_key(there) {
                toward.insert(there, here);
                next.push_back(there);
            }
        }
    }
    toward
}

fn path(network: &Network, toward: &HashMap<&str, &str>, id: &str) -> Option<Path> {
    let mut through = Vec::new();
    let mut here = id;
    while let Some(&next) = toward.get(here) {
        through.push(next.to_owned());
        here = next;
    }
    if through.is_empty() {
        return None;
    }
    let mut hops: Vec<(String, Option<String>, Option<f64>)> = Vec::new();
    let mut from = id;
    for to in &through {
        let room = link_between(network, from, to).and_then(|link| room(network, link));
        hops.push((from.to_owned(), Some(to.clone()), room));
        from = to;
    }
    let internet = network.internet.as_ref();
    let down = narrowest(&hops, internet.and_then(|internet| internet.down_mbps));
    let up = narrowest(&hops, internet.and_then(|internet| internet.up_mbps));
    Some(Path { through, down, up })
}

fn link_between<'a>(network: &'a Network, one: &str, other: &str) -> Option<&'a Link> {
    network.links.iter().find(|link| {
        (link.a.device == one && link.b.device == other)
            || (link.a.device == other && link.b.device == one)
    })
}

/// A link's room: a cable's speed, or a radio link's speed less the share
/// of time the busier of its radios is busy. A radio link whose radios say
/// nothing of how busy they are has no room worked out, as one with no
/// speed hasn't.
fn room(network: &Network, link: &Link) -> Option<f64> {
    let speed = f64::from(link.speed_mbps?);
    match link.medium {
        Medium::Ethernet | Medium::Coax => Some(speed),
        Medium::Wifi => {
            let busy = [&link.a, &link.b]
                .into_iter()
                .filter_map(|end| {
                    let radio = end.radio.as_deref()?;
                    device(network, &end.device)?
                        .radios
                        .iter()
                        .find(|each| each.name == radio)?
                        .busy_pct
                })
                .max()?;
            Some(tenths(speed * f64::from(100 - busy.min(100)) / 100.0))
        }
    }
}

/// The hop with least room, the nearest the device winning a tie, and the
/// internet beyond the modem last; the hops with no room worked out are
/// counted, since the real chokepoint may be one of them.
fn narrowest(
    hops: &[(String, Option<String>, Option<f64>)],
    internet: Option<f64>,
) -> Option<Chokepoint> {
    let modem_end = hops.last().and_then(|(_, b, _)| b.clone())?;
    let internet_hop = (modem_end, None, internet);
    let all = hops.iter().chain([&internet_hop]);
    let unmeasured = all.clone().filter(|(_, _, room)| room.is_none()).count();
    let (a, b, room) = all
        .filter_map(|(a, b, room)| room.map(|room| (a, b, room)))
        .fold(
            None,
            |least: Option<(&String, &Option<String>, f64)>, hop| match least {
                Some(least) if least.2 <= hop.2 => Some(least),
                _ => Some(hop),
            },
        )?;
    Some(Chokepoint {
        a: a.clone(),
        b: b.clone(),
        room_mbps: room,
        unmeasured: u32::try_from(unmeasured).unwrap_or(u32::MAX),
    })
}

fn concerns(network: &Network) -> Vec<Concern> {
    let mut concerns = Vec::new();
    for device in &network.devices {
        for radio in &device.radios {
            if let Some(busy) = radio.busy_pct.filter(|busy| *busy >= BUSY_PCT) {
                // The channel tells two radios called "5 GHz high" apart
                // for a reader who knows only what their eero app shows.
                let channel = radio
                    .channel
                    .map(|channel| format!(", on channel {channel},"))
                    .unwrap_or_default();
                let busiest = match (radio.busiest_pct, radio.busiest_at) {
                    (Some(most), Some(at)) if most > busy => format!(
                        "; {most}% at its busiest, {} ago",
                        span(network.collected_at.saturating_sub(at))
                    ),
                    _ => String::new(),
                };
                concerns.push(Concern {
                    kind: ConcernKind::Busy,
                    device: device.id.clone(),
                    on: Some(radio.name.clone()),
                    other: None,
                    says: format!(
                        "{} {} radio{channel} is busy {busy}% of the time{busiest}.",
                        possessive(&device.name),
                        radio.name
                    ),
                });
            }
        }
    }
    for link in &network.links {
        let near = name(network, &link.a.device);
        let far = name(network, &link.b.device);
        let concern = match (link.medium, link.signal_dbm, link.speed_mbps) {
            (Medium::Wifi, Some(signal), _) if signal <= WEAK_DBM => Some((
                ConcernKind::Weak,
                link.a.radio.clone(),
                format!(
                    "{near} has a weak signal to {far}: {} dBm, and from {} down its speed falls.",
                    dbm(signal),
                    dbm(WEAK_DBM)
                ),
            )),
            (Medium::Ethernet, _, Some(speed)) if speed < GIGABIT => Some((
                ConcernKind::Slow,
                link.a.port.clone(),
                format!(
                    "{} cable to {far} runs at {speed} Mbit/s.",
                    possessive(near)
                ),
            )),
            _ => None,
        };
        if let Some((kind, on, says)) = concern {
            concerns.push(Concern {
                kind,
                device: link.a.device.clone(),
                on,
                other: Some(link.b.device.clone()),
                says,
            });
        }
    }
    concerns.extend(lost(network));
    if let Some(internet) = &network.internet {
        concerns.extend(fills(network, internet));
        concerns.extend(ceiling(network, internet, "down", |path| {
            path.down.as_ref()
        }));
        concerns.extend(ceiling(network, internet, "up", |path| path.up.as_ref()));
    }
    concerns
}

/// A span of the day in words: minutes under two hours, hours after.
fn span(seconds: u64) -> String {
    let minutes = seconds / 60;
    match minutes {
        1 => "1 minute".to_owned(),
        0..120 => format!("{minutes} minutes"),
        _ => format!("{} hours", minutes / 60),
    }
}

/// A signal in dBm as it's written, with a true minus.
fn dbm(signal: i32) -> String {
    if signal < 0 {
        format!("\u{2212}{}", signal.unsigned_abs())
    } else {
        signal.to_string()
    }
}

/// The modem's downstream channel that lost far more than every other, if
/// one did.
fn lost(network: &Network) -> Option<Concern> {
    let device = network
        .devices
        .iter()
        .find(|device| device.id == modem::ID)?;
    let channels = &device.modem.as_ref()?.downstream;
    let (worst, most) = channels
        .iter()
        .enumerate()
        .filter_map(|(at, channel)| channel.uncorrectable.map(|count| (at, count)))
        .max_by_key(|(_, count)| *count)?;
    let others = channels
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != worst)
        .filter_map(|(_, channel)| channel.uncorrectable)
        .max()
        .unwrap_or(0);
    if most < LOST_MANY || most < others.saturating_mul(LOST_TIMES) {
        return None;
    }
    let channel = &channels[worst];
    let which = channel.channel.map_or_else(
        || format!("downstream {} channel", channel.kind),
        |number| format!("downstream channel {number}"),
    );
    let provider = network
        .internet
        .as_ref()
        .and_then(|internet| internet.provider.as_deref())
        .unwrap_or("the provider");
    // With the day's readings it says how much the count climbed, which is
    // what the provider would ask first. A channel that lost none lately
    // is calm, so it's no concern, its counts left to the channel's figures.
    let lately = device
        .modem
        .as_ref()
        .and_then(|modem| modem.lately_s)
        .zip(channel.lost_lately);
    let after = match lately {
        Some((_, 0)) => return None,
        Some((over, recent)) => format!(
            ", {recent} of them in the last {}; {provider} can check the line",
            span(over)
        ),
        None => format!("; if that keeps climbing, {provider} can check the line"),
    };
    Some(Concern {
        kind: ConcernKind::Lost,
        device: device.id.clone(),
        on: Some(which.clone()),
        other: None,
        says: format!(
            "The modem's {which} has lost {most} pieces of data it couldn't repair since the \
             modem started, against at most {others} on any other{after}."
        ),
    })
}

/// Each device using most of the internet's speed one way, as eero counts
/// it now.
fn fills(network: &Network, internet: &Internet) -> Vec<Concern> {
    let mut concerns = Vec::new();
    for device in &network.devices {
        let Some(speed) = &device.speed else {
            continue;
        };
        let ways = [
            ("down", speed.down_mbps, internet.down_mbps),
            ("up", speed.up_mbps, internet.up_mbps),
        ];
        for (way, using, line) in ways {
            let (Some(using), Some(line)) = (using, line) else {
                continue;
            };
            let share = using / line * 100.0;
            if line > 0.0 && share >= FILLS_PCT {
                concerns.push(Concern {
                    kind: ConcernKind::Fills,
                    device: device.id.clone(),
                    on: Some(way.into()),
                    other: None,
                    says: format!(
                        "{} is using {share:.0}% of the internet's speed {way}: {:.1} of its \
                         {line:.1} Mbit/s.",
                        device.name,
                        tenths(using)
                    ),
                });
            }
        }
    }
    concerns
}

/// The internet as the narrowest hop one way, said once for every device
/// it's narrowest for, rather than once a device.
fn ceiling(
    network: &Network,
    internet: &Internet,
    way: &str,
    chokepoint: impl Fn(&Path) -> Option<&Chokepoint>,
) -> Option<Concern> {
    let mbps = if way == "down" {
        internet.down_mbps
    } else {
        internet.up_mbps
    }?;
    // The devices on the eeros, counted as the map counts them: the eeros
    // themselves carry the others' traffic, so they aren't among them.
    let paths: Vec<&Path> = network
        .devices
        .iter()
        .filter(|device| {
            !device
                .roles
                .iter()
                .any(|role| matches!(role, Role::Router | Role::AccessPoint))
        })
        .filter_map(|device| device.path.as_ref())
        .collect();
    let devices = paths
        .iter()
        .filter_map(|path| chokepoint(path))
        .filter(|hop| hop.b.is_none())
        .count();
    // Every device or so many of them all, since a bare count beside the
    // map's own count of devices on the eeros reads as a mistake.
    let whom = if devices == paths.len() {
        "every device on the eeros".to_owned()
    } else {
        format!("{devices} of the {} devices on the eeros", paths.len())
    };
    (devices > 0).then(|| Concern {
        kind: ConcernKind::Ceiling,
        device: modem::ID.into(),
        on: Some(way.into()),
        other: None,
        says: format!(
            "The internet is the narrowest point {way} for {whom}: {mbps:.1} Mbit/s, as eero \
             last measured it."
        ),
    })
}

/// A name as an owner, with the apostrophe alone after a final s, as in
/// "Upstairs' radio".
fn possessive(name: &str) -> String {
    if name.ends_with('s') {
        format!("{name}'")
    } else {
        format!("{name}'s")
    }
}

fn device<'a>(network: &'a Network, id: &str) -> Option<&'a Device> {
    network.devices.iter().find(|device| device.id == id)
}

fn name<'a>(network: &'a Network, id: &'a str) -> &'a str {
    device(network, id).map_or(id, |device| device.name.as_str())
}

/// Rounds to a tenth of a megabit, so a record says no more than the
/// figures it came from can carry.
fn tenths(mbps: f64) -> f64 {
    (mbps * 10.0).round() / 10.0
}

/// The golden record and ways to set its figures, shared with the day's
/// tests in src/history.rs, which work concerns out over it too.
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

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
        // The tablet joins Den's 5 GHz low radio at 48 Mbit/s, busy 5%,
        // so 45.6 Mbit/s is left, less than the internet's 369.86 down.
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
                "unknown computer 1 is using 83% of the internet's speed up: 9.9 of its 11.9 Mbit/s."
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
        work_out(&mut network);
        let slow: Vec<_> = network
            .concerns
            .iter()
            .filter(|concern| concern.kind == ConcernKind::Slow)
            .collect();
        assert_eq!(slow.len(), 1, "one slow cable: {slow:?}");
        assert_eq!(slow[0].on.as_deref(), Some("2"));
        assert_eq!(slow[0].says, "Hall's cable to Closet runs at 100 Mbit/s.");
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
                "The internet is the narrowest point down for 7 of the 18 devices on the eeros: \
                 369.9 Mbit/s, as eero last measured it.",
                "The internet is the narrowest point up for every device on the eeros: 11.9 \
                 Mbit/s, as eero last measured it.",
            ]
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
}
