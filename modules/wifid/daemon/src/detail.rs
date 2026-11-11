//! One network in the air, in detail: every access point of it heard,
//! how many in each band, what each card on it reads of its link, and the
//! access point wifid would pick, with why. The shell asks for this only
//! while it shows the network, so Describe stays small.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Value};
use zlink::introspect::{CustomType, Type};

use crate::air::Heard;
use crate::nl80211::{self, Rate, Traffic, Tuning};
use crate::radio::{self, Band, Load, Protection};
use crate::state;

/// A network in detail, as the shell is sent it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Detail {
    pub name: String,
    /// Every access point of it heard, the strongest first.
    pub points: Vec<Point>,
    /// How many are heard in each band, of those it's heard in.
    pub bands: Vec<Count>,
    /// Each card on it, as its driver last read the link.
    pub joined: Vec<Joined>,
    /// The access point wifid would pick, by how fast a card would get on
    /// with it.
    pub best: Option<Best>,
}

/// One access point of a network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Point {
    pub bssid: String,
    /// In dBm.
    pub signal: i16,
    /// The signal in bars, one to four, as a network's is shown.
    pub bars: u8,
    pub band: Band,
    pub channel: u16,
    /// In MHz.
    pub frequency: u32,
    /// In MHz.
    pub width: u16,
    /// 4 to 7, for Wi-Fi 4 to Wi-Fi 7; none for anything older.
    pub generation: Option<u8>,
    pub streams: u8,
    pub load: Option<Load>,
    /// None when it's open.
    pub protection: Option<Protection>,
    /// How many seconds ago it was last heard.
    pub age: u32,
    /// About how fast a card would get on with it, in Mb/s.
    pub estimate: u32,
    /// Whether a card is on it.
    pub joined: bool,
}

/// How many access points are heard in a band.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Count {
    pub band: Band,
    pub points: u32,
}

/// A card on the network, as its driver reads the link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Joined {
    pub card: String,
    /// The access point it's on.
    pub bssid: Option<String>,
    /// In dBm, now and on average.
    pub signal: Option<i32>,
    pub average: Option<i32>,
    /// In dBm, where the driver says.
    pub noise: Option<i32>,
    /// How far the signal stands above the noise, in dB.
    pub margin: Option<i32>,
    /// What it hears and what it sends, each way on its own.
    pub receive: Way,
    pub send: Way,
    /// What the driver expects the link to carry, after the air's own
    /// overhead, in Mb/s.
    pub expected: Option<u32>,
    /// How long since a frame last came or went, in milliseconds.
    pub idle: Option<u32>,
    /// As the driver says it, as 80 MHz.
    pub width: Option<String>,
    pub frequency: Option<u32>,
    /// What the kernel says of the link that wpa_supplicant doesn't read;
    /// none until it's read.
    pub station: Option<Station>,
}

/// What the kernel says of a card's link beyond wpa_supplicant's poll:
/// the antennas, the beacons, what went wrong, and how it's framed. Each
/// is none where the driver doesn't say.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Station {
    /// The signal now at each antenna, and of the access point's beacons on
    /// average, in dBm.
    pub chains: Vec<i32>,
    pub beacons: Option<i32>,
    /// What the card transmits at, in dBm.
    pub power: Option<f64>,
    /// Frames sent again and never sent, beacons missed and frames heard
    /// but dropped, since the card joined.
    pub retries: Option<u32>,
    pub failed: Option<u32>,
    pub missed: Option<u32>,
    pub dropped: Option<u64>,
    /// How long it's been joined, in seconds.
    pub connected: Option<u32>,
    /// Whether its management frames are protected, and it sorts traffic
    /// by kind (WMM).
    pub protected: Option<bool>,
    pub wmm: Option<bool>,
    /// How often the access point's beacons come, in its time units, and
    /// one in how many says what it holds for a card asleep (DTIM).
    pub interval: Option<u16>,
    pub dtim: Option<u8>,
    /// Whether frames open with the short preamble and wait the short slot.
    pub short_preamble: Option<bool>,
    pub short_slot: Option<bool>,
    /// The channel it's tuned to, and whether it's one older than Wi-Fi 4.
    pub channel: Option<u16>,
    pub legacy: bool,
}

/// One way of a card's link, as its driver last read it: the rate, how it
/// was coded, and how much has gone that way since the card joined. Each
/// is none where the driver doesn't say, as wpa_supplicant leaves out a
/// reading of nothing, MCS 0 among them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Way {
    /// In Mb/s.
    pub mbps: Option<u32>,
    /// In MHz, as the kernel says it.
    pub width: Option<u16>,
    /// 4 to 7, as the coding says it: EHT is Wi-Fi 7, which only the
    /// kernel reads, HE 6, VHT 5 and HT 4.
    pub generation: Option<u8>,
    /// The modulation and coding, and on how many streams.
    pub mcs: Option<u32>,
    pub streams: Option<u32>,
    /// The pause between symbols, in microseconds.
    pub guard: Option<f64>,
    /// Whether Wi-Fi 6's dual carrier is on: each symbol sent twice, on
    /// subcarriers apart, at half the rate, to ride out a weak signal.
    pub dcm: Option<bool>,
    pub bytes: Option<u64>,
    pub packets: Option<u32>,
}

/// The access point wifid would pick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Best {
    pub bssid: String,
    /// About how fast a card would get on with it, in Mb/s.
    pub estimate: u32,
    /// Why, a few words each, the most telling first, against the one a
    /// card is on; none when it's the only one heard.
    pub reasons: Vec<String>,
}

/// What a card's driver last read of its link, as wpa_supplicant's
/// `SignalPoll` says it, with the access point it's on by its path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polled {
    pub point: String,
    /// What the kernel read of the station and what the card's tuned to,
    /// when it answered.
    pub station: Option<nl80211::Station>,
    pub tuning: Option<Tuning>,
    pub signal: Option<i32>,
    pub average: Option<i32>,
    pub noise: Option<i32>,
    pub receive: Way,
    pub send: Way,
    pub expected: Option<u32>,
    pub idle: Option<u32>,
    pub width: Option<String>,
    pub frequency: Option<u32>,
}

impl Polled {
    pub fn read(point: String, said: &HashMap<String, OwnedValue>) -> Polled {
        let value = |key: &str| said.get(key).map(|value| &**value);
        // wpa_supplicant says 9999 for a reading the driver doesn't give.
        let signed = |key: &str| match value(key) {
            Some(Value::I32(reading)) if reading.abs() != INVALID => Some(*reading),
            _ => None,
        };
        let unsigned = |key: &str| match value(key) {
            Some(Value::U32(reading)) => Some(*reading),
            _ => None,
        };
        let width = match value("width") {
            Some(Value::Str(width)) => Some(width.to_string()),
            _ => None,
        };
        let speed = unsigned("linkspeed");
        let way = |rate: &str, from: &str| {
            let key = |figure: &str| format!("{from}-{figure}");
            let (generation, mcs, streams) = coding(|figure| unsigned(&key(figure)));
            Way {
                mbps: unsigned(rate).map(mbps).or(speed),
                width: None,
                generation,
                mcs,
                streams,
                guard: match value(&key("guard-interval")) {
                    Some(Value::F64(microseconds)) => Some(*microseconds),
                    _ => None,
                },
                dcm: match value(&key("dcm")) {
                    Some(Value::Bool(on)) => Some(*on),
                    _ => None,
                },
                bytes: match value(&key("bytes")) {
                    Some(Value::U64(bytes)) => Some(*bytes),
                    _ => None,
                },
                packets: unsigned(&key("packets")),
            }
        };
        Polled {
            point,
            signal: signed("rssi"),
            average: signed("avg-rssi"),
            noise: signed("noise"),
            receive: way("linkrxspeed", "rx"),
            send: way("linktxspeed", "tx"),
            expected: unsigned("expected-throughput").map(mbps),
            idle: unsigned("inactive-time"),
            width,
            frequency: unsigned("frequency"),
            station: None,
            tuning: None,
        }
    }
}

/// One way of a card's link, as the kernel read it where it did, and as
/// wpa_supplicant polled it where it didn't.
fn way(polled: &Way, read: Option<(&Rate, &Traffic)>) -> Way {
    let Some((rate, traffic)) = read else {
        return polled.clone();
    };
    Way {
        mbps: rate.kbps.map(mbps).or(polled.mbps),
        width: rate.width.or(polled.width),
        generation: rate.generation.or(polled.generation),
        mcs: rate.mcs.or(polled.mcs),
        streams: rate.streams.or(polled.streams),
        guard: rate.guard.or(polled.guard),
        dcm: rate.dcm.or(polled.dcm),
        bytes: traffic.bytes.or(polled.bytes),
        packets: traffic.packets.or(polled.packets),
    }
}

/// What the kernel says beyond wpa_supplicant's poll, as the shell is
/// sent it.
fn beyond(station: &nl80211::Station, tuning: Option<&Tuning>) -> Station {
    Station {
        chains: station.chains.clone(),
        beacons: station.beacons,
        power: tuning.and_then(|tuning| tuning.power),
        retries: station.retries,
        failed: station.failed,
        missed: station.missed,
        dropped: station.dropped,
        connected: station.connected,
        protected: station.protected,
        wmm: station.wmm,
        interval: station.interval,
        dtim: station.dtim,
        short_preamble: station.short_preamble,
        short_slot: station.short_slot,
        channel: tuning
            .and_then(|tuning| tuning.frequency)
            .map(radio::channel),
        legacy: tuning.is_some_and(|tuning| tuning.legacy),
    }
}

/// A rate in kb/s, as nl80211 says it, to the nearest Mb/s.
fn mbps(kbps: u32) -> u32 {
    kbps.saturating_add(500) / 1000
}

/// One way's generation, MCS and streams, from its figures by name (`he-mcs`,
/// `vht-nss`), the newest coding said: HE's and VHT's say their streams,
/// and HT's index does, eight to a stream.
fn coding(figure: impl Fn(&str) -> Option<u32>) -> (Option<u8>, Option<u32>, Option<u32>) {
    if let Some(mcs) = figure("he-mcs") {
        return (Some(6), Some(mcs), figure("he-nss"));
    }
    if let Some(mcs) = figure("vht-mcs") {
        return (Some(5), Some(mcs), figure("vht-nss"));
    }
    match figure("mcs") {
        Some(index) => (
            Some(4),
            Some(index % HT_PER_STREAM),
            Some(index / HT_PER_STREAM + 1),
        ),
        None => (None, None, None),
    }
}

/// How many of Wi-Fi 4's MCS indexes each stream count takes.
const HT_PER_STREAM: u32 = 8;

/// What wpa_supplicant calls a reading the driver didn't give.
const INVALID: i32 = 9999;

/// A network in detail, from the access points heard, by their paths in
/// wpa_supplicant, the paths of those the cards on it are joined to, and
/// what each of those cards last polled, by its name.
pub fn detail<'a>(
    name: &str,
    heard: impl IntoIterator<Item = (&'a str, &'a Heard)>,
    on: impl IntoIterator<Item = &'a str>,
    polled: impl IntoIterator<Item = (&'a str, &'a Polled)>,
) -> Detail {
    let on: Vec<&str> = on.into_iter().collect();
    let polled: Vec<(&str, &Polled)> = polled.into_iter().collect();
    let mut of: Vec<(&str, &Heard)> = heard
        .into_iter()
        .filter(|(_, heard)| heard.name == name)
        .collect();
    of.sort_by(|(_, one), (_, other)| {
        let stronger = other.signal.cmp(&one.signal);
        stronger.then_with(|| one.bssid.cmp(&other.bssid))
    });
    let mut points: Vec<Point> = Vec::new();
    for (path, heard) in &of {
        let joined = on.contains(path) || polled.iter().any(|(_, polled)| polled.point == *path);
        // Two cards hearing one access point hear it as two; the stronger
        // comes first, so it's the one kept.
        match points.iter_mut().find(|point| point.bssid == heard.bssid) {
            Some(kept) => kept.joined |= joined,
            None => points.push(point(heard, joined)),
        }
    }
    let bands = [Band::Ghz2, Band::Ghz5, Band::Ghz6]
        .into_iter()
        .filter_map(|band| {
            let points = points.iter().filter(|point| point.band == band).count();
            let points = u32::try_from(points).ok().filter(|points| *points > 0)?;
            Some(Count { band, points })
        });
    let joined = polled.iter().map(|(card, polled)| {
        let on = of.iter().find(|(path, _)| *path == polled.point);
        let noise = polled.noise;
        let station = polled.station.as_ref();
        Joined {
            card: (*card).into(),
            bssid: on.map(|(_, heard)| heard.bssid.clone()),
            signal: polled.signal,
            average: polled.average,
            noise,
            margin: polled
                .signal
                .zip(noise)
                .map(|(signal, noise)| signal - noise),
            receive: way(
                &polled.receive,
                station.map(|station| (&station.receive, &station.received)),
            ),
            send: way(
                &polled.send,
                station.map(|station| (&station.send, &station.sent)),
            ),
            expected: polled
                .expected
                .or(station.and_then(|station| station.expected.map(mbps))),
            idle: polled.idle.or(station.and_then(|station| station.idle)),
            width: polled.width.clone(),
            frequency: polled
                .frequency
                .or(polled.tuning.as_ref().and_then(|tuning| tuning.frequency)),
            station: station.map(|station| beyond(station, polled.tuning.as_ref())),
        }
    });
    let joined = joined.collect();
    let best = fastest(points.iter()).map(|best| Best {
        bssid: best.bssid.clone(),
        estimate: best.estimate,
        reasons: reasons(best, &points),
    });
    Detail {
        name: name.into(),
        bands: bands.collect(),
        joined,
        best,
        points,
    }
}

fn point(heard: &Heard, joined: bool) -> Point {
    let at = heard.frequency;
    Point {
        bssid: heard.bssid.clone(),
        signal: heard.signal,
        bars: state::bars(heard.signal),
        band: radio::band(at),
        channel: radio::channel(at),
        frequency: at,
        width: heard.radio.width,
        generation: heard.radio.generation,
        streams: heard.radio.streams,
        load: heard.radio.load,
        protection: heard.radio.protection.clone(),
        age: heard.age,
        estimate: radio::estimate(&heard.radio, at, heard.signal),
        joined,
    }
}

/// The fastest, the stronger of two as fast, the first of two as strong.
fn fastest<'a>(points: impl Iterator<Item = &'a Point>) -> Option<&'a Point> {
    points.reduce(|best, point| {
        let faster = (point.estimate, point.signal) > (best.estimate, best.signal);
        if faster { point } else { best }
    })
}

/// Why the best is best, a few words each, the most telling first: against
/// the one a card is on, or the next fastest when none is.
fn reasons(best: &Point, points: &[Point]) -> Vec<String> {
    if points.len() == 1 {
        return Vec::new();
    }
    if best.joined {
        return vec![format!("the fastest of {}", points.len())];
    }
    let others = points.iter().filter(|point| point.bssid != best.bssid);
    let Some(other) = points
        .iter()
        .find(|point| point.joined)
        .or_else(|| fastest(others))
    else {
        return Vec::new();
    };
    let mut why = Vec::new();
    if best.band != other.band {
        why.push(format!(
            "on {} GHz, not {}",
            ghz(best.band),
            ghz(other.band)
        ));
    }
    if best.width > other.width {
        why.push(format!("{} MHz wide, not {}", best.width, other.width));
    }
    if best.generation > other.generation {
        let was = other
            .generation
            .map_or("older".into(), |was| was.to_string());
        let generation = best.generation.unwrap_or_default();
        why.push(format!("Wi-Fi {generation}, not {was}"));
    }
    let streams = |point: &Point| point.streams.min(2);
    if streams(best) > streams(other) {
        why.push(format!("{} streams, not {}", streams(best), streams(other)));
    }
    // Signal wanders a few dB from one look to the next.
    let stronger = best.signal - other.signal;
    if stronger >= 3 {
        why.push(format!("{stronger} dB stronger"));
    }
    if let (Some(best), Some(other)) = (best.load, other.load)
        && other.busy >= best.busy + 10
    {
        why.push(format!("{}% busy, not {}%", best.busy, other.busy));
    }
    if why.is_empty() {
        why.push("a little faster".into());
    }
    why
}

fn ghz(band: Band) -> &'static str {
    match band {
        Band::Ghz2 => "2.4",
        Band::Ghz5 => "5",
        Band::Ghz6 => "6",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radio::Radio;
    use crate::state::Security;

    fn heard(name: &str, bssid: &str, frequency: u32, signal: i16, radio: Radio) -> Heard {
        Heard {
            name: name.into(),
            security: Security::Personal,
            signal,
            bssid: bssid.into(),
            frequency,
            age: 3,
            radio,
        }
    }

    fn radio(generation: u8, width: u16, busy: Option<u8>) -> Radio {
        Radio {
            generation: Some(generation),
            width,
            streams: 2,
            load: busy.map(|busy| Load { stations: 4, busy }),
            protection: None,
        }
    }

    fn owned(value: &Value<'_>) -> OwnedValue {
        value.try_to_owned().unwrap()
    }

    /// The hotel's access points: two in the hall on 2.4 GHz, one on
    /// 5 GHz, and a neighbour's network.
    fn hotel() -> Vec<(String, Heard)> {
        let at = |n: u32| format!("/fi/w1/wpa_supplicant1/Interfaces/0/BSSs/{n}");
        vec![
            (
                at(0),
                heard(
                    "Hilton",
                    "e4:55:a8:57:49:e0",
                    2412,
                    -59,
                    radio(6, 20, Some(29)),
                ),
            ),
            (
                at(1),
                heard(
                    "Hilton",
                    "e6:55:b8:57:49:e0",
                    5200,
                    -56,
                    radio(6, 40, Some(13)),
                ),
            ),
            (
                at(2),
                heard("Hilton", "e4:55:a8:2f:63:0e", 2437, -65, radio(6, 20, None)),
            ),
            (
                at(3),
                heard(
                    "Next Door",
                    "ac:84:c9:f6:14:36",
                    2437,
                    -87,
                    radio(4, 20, None),
                ),
            ),
        ]
    }

    fn of(heard: &[(String, Heard)]) -> impl Iterator<Item = (&str, &Heard)> {
        heard.iter().map(|(path, heard)| (path.as_str(), heard))
    }

    fn polled(point: u32) -> Polled {
        Polled {
            point: format!("/fi/w1/wpa_supplicant1/Interfaces/0/BSSs/{point}"),
            signal: Some(-59),
            average: Some(-58),
            noise: None,
            receive: way(458, 9),
            send: way(458, 8),
            expected: Some(371),
            idle: Some(12),
            width: Some("40 MHz".into()),
            frequency: Some(5200),
            station: None,
            tuning: None,
        }
    }

    fn way(mbps: u32, mcs: u32) -> Way {
        Way {
            mbps: Some(mbps),
            width: None,
            generation: Some(6),
            mcs: Some(mcs),
            streams: Some(2),
            guard: Some(0.8),
            dcm: Some(false),
            bytes: None,
            packets: None,
        }
    }

    #[test]
    fn each_access_point_has_bars_for_its_signal_as_a_network_does() {
        let hotel = hotel();

        let detail = detail("Hilton", of(&hotel), [], []);

        let bars: Vec<_> = detail.points.iter().map(|point| point.bars).collect();
        assert_eq!(bars, [4, 4, 3]);
    }

    #[test]
    fn a_network_is_detailed_by_its_own_access_points_strongest_first() {
        let hotel = hotel();

        let detail = detail("Hilton", of(&hotel), [], []);

        let points: Vec<_> = detail
            .points
            .iter()
            .map(|point| point.bssid.as_str())
            .collect();
        assert_eq!(
            points,
            [
                "e6:55:b8:57:49:e0",
                "e4:55:a8:57:49:e0",
                "e4:55:a8:2f:63:0e"
            ]
        );
    }

    #[test]
    fn each_access_point_says_where_it_is_and_how_fast_it_would_be() {
        let hotel = hotel();

        let detail = detail("Hilton", of(&hotel), [], []);

        let five = &detail.points[0];
        assert_eq!((five.band, five.channel, five.width), (Band::Ghz5, 40, 40));
        assert_eq!(
            (five.generation, five.age, five.joined),
            (Some(6), 3, false)
        );
        assert_eq!(
            five.estimate,
            radio::estimate(&radio(6, 40, Some(13)), 5200, -56)
        );
    }

    #[test]
    fn the_access_points_are_counted_in_each_band_heard() {
        let hotel = hotel();

        let detail = detail("Hilton", of(&hotel), [], []);

        let counted = [(Band::Ghz2, 2), (Band::Ghz5, 1)];
        let counted = counted.map(|(band, points)| Count { band, points });
        assert_eq!(detail.bands, counted);
    }

    #[test]
    fn one_access_point_heard_by_two_cards_is_one_access_point() {
        let mut hotel = hotel();
        let mut again = hotel[1].1.clone();
        again.signal = -70;
        hotel.push(("/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/0".into(), again));

        let detail = detail("Hilton", of(&hotel), [], []);

        assert_eq!(detail.points.len(), 3);
        assert_eq!(detail.points[0].signal, -56);
    }

    #[test]
    fn the_fastest_is_the_best_guess_with_why_against_the_next() {
        let hotel = hotel();

        let detail = detail("Hilton", of(&hotel), [], []);

        let best = detail.best.unwrap();
        assert_eq!(best.bssid, "e6:55:b8:57:49:e0");
        assert_eq!(best.estimate, detail.points[0].estimate);
        // The next is the hall's on channel 6, the other's channel too busy.
        assert_eq!(
            best.reasons,
            ["on 5 GHz, not 2.4", "40 MHz wide, not 20", "9 dB stronger"]
        );
    }

    #[test]
    fn a_card_on_the_best_already_is_told_so() {
        let hotel = hotel();
        let on = polled(1);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        assert!(detail.points[0].joined);
        assert_eq!(detail.best.unwrap().reasons, ["the fastest of 3"]);
    }

    #[test]
    fn an_access_point_a_card_is_on_is_marked_joined_before_its_link_is_read() {
        let hotel = hotel();
        let on = &hotel[1].0;

        let detail = detail("Hilton", of(&hotel), [on.as_str()], []);

        assert!(detail.points[0].joined);
        assert_eq!(detail.joined, []);
        assert_eq!(detail.best.unwrap().reasons, ["the fastest of 3"]);
    }

    #[test]
    fn a_card_on_another_is_told_why_the_best_is_better_than_its_own() {
        let hotel = hotel();
        let on = polled(0);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        assert_eq!(
            detail.best.unwrap().reasons,
            [
                "on 5 GHz, not 2.4",
                "40 MHz wide, not 20",
                "3 dB stronger",
                "13% busy, not 29%"
            ]
        );
    }

    #[test]
    fn a_network_heard_from_one_access_point_has_only_that_one() {
        let hotel = hotel();

        let detail = detail("Next Door", of(&hotel), [], []);

        assert_eq!(detail.best.unwrap().reasons, Vec::<String>::new());
    }

    #[test]
    fn a_network_not_heard_has_nothing_in_detail() {
        let hotel = hotel();

        let detail = detail("Elsewhere", of(&hotel), [], []);

        assert_eq!((detail.points.len(), detail.best), (0, None));
    }

    #[test]
    fn a_card_on_it_says_its_link_and_which_access_point() {
        let hotel = hotel();
        let on = polled(1);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        let joined = &detail.joined[0];
        assert_eq!(joined.card, "wlan0");
        assert_eq!(joined.bssid.as_deref(), Some("e6:55:b8:57:49:e0"));
        assert_eq!(joined.signal, Some(-59));
        assert_eq!(
            (joined.receive.mbps, joined.receive.mcs),
            (Some(458), Some(9))
        );
    }

    #[test]
    fn a_card_on_it_says_its_send_side_what_it_can_carry_and_how_long_it_has_been_idle() {
        let hotel = hotel();
        let on = polled(1);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        let joined = &detail.joined[0];
        assert_eq!(joined.send, way(458, 8));
        assert_eq!((joined.expected, joined.idle), (Some(371), Some(12)));
    }

    #[test]
    fn what_the_kernel_read_wins_and_the_poll_fills_what_it_did_not() {
        let hotel = hotel();
        let mut on = polled(1);
        on.station = Some(nl80211::Station {
            chains: vec![-60, -63],
            send: Rate {
                kbps: Some(2_882_000),
                generation: Some(7),
                mcs: Some(13),
                width: Some(320),
                ..Rate::default()
            },
            retries: Some(17),
            dtim: Some(3),
            ..nl80211::Station::default()
        });
        on.tuning = Some(Tuning {
            frequency: Some(6_115),
            width: Some(320),
            legacy: false,
            power: Some(22.0),
        });

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        let joined = &detail.joined[0];
        let send = &joined.send;
        assert_eq!(
            (
                send.mbps,
                send.generation,
                send.mcs,
                send.width,
                send.streams
            ),
            (Some(2_882), Some(7), Some(13), Some(320), Some(2))
        );
        assert_eq!(joined.receive, way(458, 9));
        let station = joined.station.as_ref().expect("the kernel's reading");
        assert_eq!(
            (
                &station.chains,
                station.power,
                station.retries,
                station.dtim,
                station.channel
            ),
            (&vec![-60, -63], Some(22.0), Some(17), Some(3), Some(33))
        );
    }

    #[test]
    fn a_card_the_kernel_has_not_answered_for_has_no_station() {
        let hotel = hotel();
        let on = polled(1);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        assert_eq!(detail.joined[0].station, None);
    }

    #[test]
    fn a_card_hearing_the_noise_says_how_far_its_signal_stands_above_it() {
        let hotel = hotel();
        let mut on = polled(1);
        on.noise = Some(-95);

        let detail = detail("Hilton", of(&hotel), [], [("wlan0", &on)]);

        assert_eq!(detail.joined[0].margin, Some(36));
    }

    #[test]
    fn signal_poll_is_read_as_the_driver_says_it() {
        // As an Intel card said it of a hotel's 5 GHz access point, the
        // figures each way as iw read them of a home one.
        let said = HashMap::from([
            ("rssi".into(), owned(&Value::from(-59i32))),
            ("avg-rssi".into(), owned(&Value::from(-58i32))),
            ("noise".into(), owned(&Value::from(9999i32))),
            ("linkspeed".into(), owned(&Value::from(458u32))),
            ("linkrxspeed".into(), owned(&Value::from(458_800u32))),
            ("linktxspeed".into(), owned(&Value::from(286_700u32))),
            ("frequency".into(), owned(&Value::from(5200u32))),
            ("width".into(), owned(&Value::from("40 MHz"))),
            ("rx-he-mcs".into(), owned(&Value::from(9u32))),
            ("rx-he-nss".into(), owned(&Value::from(2u32))),
            ("rx-guard-interval".into(), owned(&Value::from(0.8f64))),
            ("tx-he-mcs".into(), owned(&Value::from(7u32))),
            ("tx-he-nss".into(), owned(&Value::from(1u32))),
            ("tx-guard-interval".into(), owned(&Value::from(1.6f64))),
            ("rx-dcm".into(), owned(&Value::from(false))),
            ("tx-dcm".into(), owned(&Value::from(true))),
            (
                "expected-throughput".into(),
                owned(&Value::from(887_703u32)),
            ),
            ("rx-bytes".into(), owned(&Value::from(23_661_070_224u64))),
            ("tx-bytes".into(), owned(&Value::from(10_734_060_673u64))),
            ("rx-packets".into(), owned(&Value::from(27_210_952u32))),
            ("tx-packets".into(), owned(&Value::from(12_881_183u32))),
            ("inactive-time".into(), owned(&Value::from(1u32))),
        ]);

        let polled = Polled::read("/fi/w1/wpa_supplicant1/Interfaces/0/BSSs/1".into(), &said);

        assert_eq!(
            polled,
            Polled {
                point: "/fi/w1/wpa_supplicant1/Interfaces/0/BSSs/1".into(),
                signal: Some(-59),
                average: Some(-58),
                // The driver's way of saying it doesn't know.
                noise: None,
                // Each rate to the nearest Mb/s, as iw's are shown.
                receive: Way {
                    mbps: Some(459),
                    width: None,
                    generation: Some(6),
                    mcs: Some(9),
                    streams: Some(2),
                    guard: Some(0.8),
                    dcm: Some(false),
                    bytes: Some(23_661_070_224),
                    packets: Some(27_210_952),
                },
                send: Way {
                    mbps: Some(287),
                    width: None,
                    generation: Some(6),
                    mcs: Some(7),
                    streams: Some(1),
                    guard: Some(1.6),
                    dcm: Some(true),
                    bytes: Some(10_734_060_673),
                    packets: Some(12_881_183),
                },
                expected: Some(888),
                idle: Some(1),
                width: Some("40 MHz".into()),
                frequency: Some(5200),
                station: None,
                tuning: None,
            }
        );
    }

    #[test]
    fn signal_poll_of_a_wifi_5_link_says_its_coding_as_vht() {
        let said = HashMap::from([
            ("rx-vht-mcs".into(), owned(&Value::from(8u32))),
            ("rx-vht-nss".into(), owned(&Value::from(2u32))),
            ("tx-vht-mcs".into(), owned(&Value::from(9u32))),
            ("tx-vht-nss".into(), owned(&Value::from(1u32))),
        ]);

        let polled = Polled::read(String::new(), &said);

        let coded = |way: &Way| (way.generation, way.mcs, way.streams);
        assert_eq!(coded(&polled.receive), (Some(5), Some(8), Some(2)));
        assert_eq!(coded(&polled.send), (Some(5), Some(9), Some(1)));
    }

    #[test]
    fn signal_poll_of_a_wifi_4_link_takes_its_streams_from_the_index() {
        // HT numbers MCS 0 to 31, eight to a stream.
        let said = HashMap::from([
            ("rx-mcs".into(), owned(&Value::from(15u32))),
            ("tx-mcs".into(), owned(&Value::from(4u32))),
        ]);

        let polled = Polled::read(String::new(), &said);

        let coded = |way: &Way| (way.generation, way.mcs, way.streams);
        assert_eq!(coded(&polled.receive), (Some(4), Some(7), Some(2)));
        assert_eq!(coded(&polled.send), (Some(4), Some(4), Some(1)));
    }

    #[test]
    fn signal_poll_without_link_rates_takes_the_one_it_has() {
        let said = HashMap::from([
            ("linkspeed".into(), owned(&Value::from(54u32))),
            ("rx-mcs".into(), owned(&Value::from(7u32))),
        ]);

        let polled = Polled::read(String::new(), &said);

        assert_eq!(
            (polled.receive.mbps, polled.send.mbps, polled.receive.mcs),
            (Some(54), Some(54), Some(7))
        );
    }
}
