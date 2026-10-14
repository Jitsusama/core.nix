//! One network in the air, in detail: every access point of it heard,
//! how many in each band, what each card on it reads of its link, and the
//! access point wifid would pick, with why. The shell asks for this only
//! while it shows the network, so Describe stays small.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Value};
use zlink::introspect::{CustomType, Type};

use crate::air::Heard;
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
    /// The rates it last heard and sent at, in Mb/s.
    pub receive: Option<u32>,
    pub send: Option<u32>,
    /// As the driver says it, as 80 MHz.
    pub width: Option<String>,
    pub frequency: Option<u32>,
    /// The modulation and coding it last heard at, and on how many streams.
    pub mcs: Option<u32>,
    pub streams: Option<u32>,
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
/// SignalPoll says it, with the access point it's on by its path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polled {
    pub point: String,
    pub signal: Option<i32>,
    pub average: Option<i32>,
    pub noise: Option<i32>,
    pub receive: Option<u32>,
    pub send: Option<u32>,
    pub width: Option<String>,
    pub frequency: Option<u32>,
    pub mcs: Option<u32>,
    pub streams: Option<u32>,
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
        let speed = unsigned("linkspeed");
        let rate = |key: &str| unsigned(key).map(|kbps| kbps / 1000).or(speed);
        let width = match value("width") {
            Some(Value::Str(width)) => Some(width.to_string()),
            _ => None,
        };
        Polled {
            point,
            signal: signed("rssi"),
            average: signed("avg-rssi"),
            noise: signed("noise"),
            receive: rate("linkrxspeed"),
            send: rate("linktxspeed"),
            width,
            frequency: unsigned("frequency"),
            mcs: unsigned("rx-he-mcs")
                .or_else(|| unsigned("rx-vht-mcs"))
                .or_else(|| unsigned("rx-mcs")),
            streams: unsigned("rx-he-nss").or_else(|| unsigned("rx-vht-nss")),
        }
    }
}

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
            receive: polled.receive,
            send: polled.send,
            width: polled.width.clone(),
            frequency: polled.frequency,
            mcs: polled.mcs,
            streams: polled.streams,
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

    fn owned(value: Value<'_>) -> OwnedValue {
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
            receive: Some(458),
            send: Some(458),
            width: Some("40 MHz".into()),
            frequency: Some(5200),
            mcs: Some(9),
            streams: Some(2),
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
        assert_eq!(
            (joined.signal, joined.receive, joined.mcs),
            (Some(-59), Some(458), Some(9))
        );
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
        // As an Intel card said it of a hotel's 5 GHz access point.
        let said = HashMap::from([
            ("rssi".into(), owned(Value::from(-59i32))),
            ("avg-rssi".into(), owned(Value::from(-58i32))),
            ("noise".into(), owned(Value::from(9999i32))),
            ("linkspeed".into(), owned(Value::from(458u32))),
            ("linkrxspeed".into(), owned(Value::from(458_800u32))),
            ("linktxspeed".into(), owned(Value::from(286_700u32))),
            ("frequency".into(), owned(Value::from(5200u32))),
            ("width".into(), owned(Value::from("40 MHz"))),
            ("rx-he-mcs".into(), owned(Value::from(9u32))),
            ("rx-he-nss".into(), owned(Value::from(2u32))),
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
                receive: Some(458),
                send: Some(286),
                width: Some("40 MHz".into()),
                frequency: Some(5200),
                mcs: Some(9),
                streams: Some(2),
            }
        );
    }

    #[test]
    fn signal_poll_without_link_rates_takes_the_one_it_has() {
        let said = HashMap::from([
            ("linkspeed".into(), owned(Value::from(54u32))),
            ("rx-mcs".into(), owned(Value::from(7u32))),
        ]);

        let polled = Polled::read(String::new(), &said);

        assert_eq!(
            (polled.receive, polled.send, polled.mcs),
            (Some(54), Some(54), Some(7))
        );
    }
}
