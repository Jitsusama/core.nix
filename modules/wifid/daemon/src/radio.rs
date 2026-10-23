//! What an access point says of itself in the elements its beacons carry:
//! the Wi-Fi generation it speaks, how wide its channel is, how many
//! streams it hears on, how busy it is and how it's protected, and from
//! those, about how fast a card would get on with it.

use serde::{Deserialize, Serialize};
use zlink::introspect::CustomType;

/// An access point's radio, as its elements describe it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Radio {
    /// 4 to 7, for Wi-Fi 4 to Wi-Fi 7; none for anything older.
    pub generation: Option<u8>,
    /// How wide its channel is, in MHz.
    pub width: u16,
    /// How many streams it hears on.
    pub streams: u8,
    /// How many are on it and how busy its channel is, where it says.
    pub load: Option<Load>,
    /// How it's protected; none when it's open.
    pub protection: Option<Protection>,
}

/// How many are on an access point and how busy its channel is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Load {
    pub stations: u16,
    /// The share of the time the access point hears its channel in use,
    /// in percent.
    pub busy: u8,
}

/// How an access point is protected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Protection {
    /// In the Wi-Fi Alliance's words, as WPA2/WPA3.
    pub standard: String,
    /// The ways it can be joined, in wpa_supplicant's words, as sae.
    pub ways: Vec<String>,
    /// The ciphers a card's own traffic can be sent with.
    pub ciphers: Vec<String>,
    /// The cipher everyone's traffic to all of them is sent with.
    pub group: String,
    pub management: Management,
}

/// Whether an access point protects the frames that join and leave, which
/// stops anyone taking a card off it by pretending to be it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Management {
    Off,
    Optional,
    Required,
}

/// The band a channel is in.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, CustomType,
)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Band {
    Ghz2,
    Ghz5,
    Ghz6,
}

// The elements read, by their ids in 802.11; an extension's is 256 more
// than its own id, since they all sit under 255.
const BSS_LOAD: u16 = 11;
const HT_CAPABILITIES: u16 = 45;
const RSN: u16 = 48;
const HT_OPERATION: u16 = 61;
const VHT_CAPABILITIES: u16 = 191;
const VHT_OPERATION: u16 = 192;
const HE_CAPABILITIES: u16 = 256 + 35;
const HE_OPERATION: u16 = 256 + 36;
const EHT_OPERATION: u16 = 256 + 106;
const EHT_CAPABILITIES: u16 = 256 + 108;

impl Radio {
    /// Reads an access point's elements, as wpa_supplicant hands them over.
    /// Elements cut short are read as far as they go, since a beacon is
    /// anyone's to send.
    pub fn read(ies: &[u8], frequency: u32) -> Radio {
        let found: Vec<(u16, &[u8])> = elements(ies).collect();
        let get = |id: u16| {
            found
                .iter()
                .find(|(of, _)| *of == id)
                .map(|(_, body)| *body)
        };
        let has = |ids: &[u16]| ids.iter().any(|id| get(*id).is_some());
        let band = band(frequency);
        // VHT is a 5 GHz standard; some access points send it on 2.4 GHz
        // as well, for faster rates only cards of their own maker take.
        let vht = band != Band::Ghz2;
        let generation = if has(&[EHT_CAPABILITIES, EHT_OPERATION]) {
            Some(7)
        } else if has(&[HE_CAPABILITIES, HE_OPERATION]) {
            Some(6)
        } else if vht && has(&[VHT_CAPABILITIES, VHT_OPERATION]) {
            Some(5)
        } else if has(&[HT_CAPABILITIES, HT_OPERATION]) {
            Some(4)
        } else {
            None
        };
        let streams = get(HE_CAPABILITIES)
            .and_then(|he| he.get(17..19))
            .or_else(|| get(VHT_CAPABILITIES).and_then(|vht| vht.get(4..6)))
            .and_then(|map| streams_in(u16::from_le_bytes([map[0], map[1]])))
            .or_else(|| get(HT_CAPABILITIES).and_then(ht_streams))
            .unwrap_or(1);
        let mut width = 20;
        // A secondary channel above or below the first makes it 40 MHz.
        if get(HT_OPERATION)
            .and_then(|ht| ht.get(1))
            .is_some_and(|&ht| ht & 0x03 != 0)
        {
            width = 40;
        }
        if let Some(&[vht_width, first, second, ..]) = get(VHT_OPERATION).filter(|_| vht) {
            width = match (vht_width, first.abs_diff(second)) {
                (1, 8) if second > 0 => 160,
                (1, apart) if second > 0 && apart > 16 => 160,
                (1, _) => 80,
                (2 | 3, _) => 160,
                _ => width,
            };
        }
        if band == Band::Ghz6
            && let Some(control) = get(HE_OPERATION).and_then(six_ghz_control)
        {
            width = [20, 40, 80, 160][usize::from(control & 0x03)];
        }
        // Only EHT can say 320 MHz.
        if let Some(&[parameters, _, _, _, _, control, ..]) = get(EHT_OPERATION)
            && parameters & 0x01 != 0
        {
            let eht = [20, 40, 80, 160, 320].get(usize::from(control & 0x07));
            width = width.max(eht.copied().unwrap_or(width));
        }
        let load = get(BSS_LOAD).and_then(|load| match load {
            &[low, high, busy, ..] => Some(Load {
                stations: u16::from_le_bytes([low, high]),
                busy: u8::try_from((u32::from(busy) * 100 + 127) / 255).unwrap_or(100),
            }),
            _ => None,
        });
        Radio {
            generation,
            width,
            streams,
            load,
            protection: get(RSN).map(protection),
        }
    }
}

/// Each element's id and body, as far as they go.
fn elements(ies: &[u8]) -> impl Iterator<Item = (u16, &[u8])> {
    let mut rest = ies;
    std::iter::from_fn(move || {
        let [id, length, after @ ..] = rest else {
            return None;
        };
        let body = after.get(..usize::from(*length))?;
        rest = &after[body.len()..];
        if *id == 255 {
            let (extension, body) = body.split_first()?;
            return Some((256 + u16::from(*extension), body));
        }
        Some((u16::from(*id), body))
    })
}

/// The streams a map of the rates heard on each says: two bits a stream,
/// 3 for one not heard on.
fn streams_in(map: u16) -> Option<u8> {
    let streams = (0..8)
        .filter(|stream| (map >> (2 * stream)) & 0x03 != 0x03)
        .count();
    u8::try_from(streams).ok().filter(|streams| *streams > 0)
}

/// The streams HT's capabilities say, a byte of rates for each of the
/// first four.
fn ht_streams(ht: &[u8]) -> Option<u8> {
    let rates = ht.get(3..7)?;
    let streams = rates.iter().filter(|rates| **rates != 0).count();
    u8::try_from(streams).ok().filter(|streams| *streams > 0)
}

/// The control byte of HE operation's 6 GHz information, which comes after
/// whatever else its parameters say it carries.
fn six_ghz_control(he: &[u8]) -> Option<u8> {
    let &[low, middle, high, ..] = he else {
        return None;
    };
    let parameters = u32::from_le_bytes([low, middle, high, 0]);
    if parameters & (1 << 17) == 0 {
        return None;
    }
    let vht = if parameters & (1 << 14) != 0 { 3 } else { 0 };
    let cohosted = usize::from(parameters & (1 << 15) != 0);
    // Three bytes of parameters, the BSS colour and two of basic rates,
    // then the channel before the control byte.
    he.get(6 + vht + cohosted + 1).copied()
}

/// How an RSN element says the access point is protected. Anything it
/// leaves off is as 802.11 says it is when left off.
fn protection(rsn: &[u8]) -> Protection {
    let mut rest = rsn.get(2..).unwrap_or_default();
    let mut take = |length: usize| {
        let taken = rest.get(..length)?;
        rest = &rest[length..];
        Some(taken)
    };
    let suite = |suite: &[u8]| match suite {
        &[0x00, 0x0f, 0xac, kind] => Some(kind),
        _ => None,
    };
    let group = take(4).map_or(Some(4), suite);
    let mut suites = |name: fn(u8) -> Option<&'static str>, otherwise: u8| {
        let Some(&[low, high]) = take(2) else {
            return vec![name(otherwise).unwrap_or_default().to_string()];
        };
        let count = usize::from(u16::from_le_bytes([low, high]));
        let suites = (0..count).filter_map(|_| take(4));
        let named = suites.filter_map(|taken| suite(taken).and_then(name));
        named.map(str::to_string).collect()
    };
    let ciphers = suites(cipher, 4);
    let ways = suites(way, 1);
    let capabilities = take(2).map_or(0, |taken| u16::from_le_bytes([taken[0], taken[1]]));
    let management = if capabilities & (1 << 6) != 0 {
        Management::Required
    } else if capabilities & (1 << 7) != 0 {
        Management::Optional
    } else {
        Management::Off
    };
    Protection {
        standard: standard(&ways, management).into(),
        ways,
        ciphers,
        group: group.and_then(cipher).unwrap_or_default().into(),
        management,
    }
}

/// What wpa_supplicant calls each cipher of 802.11's.
fn cipher(kind: u8) -> Option<&'static str> {
    Some(match kind {
        1 => "wep40",
        2 => "tkip",
        4 => "ccmp",
        5 => "wep104",
        8 => "gcmp",
        9 => "gcmp-256",
        10 => "ccmp-256",
        _ => return None,
    })
}

/// What wpa_supplicant calls each way of joining of 802.11's.
fn way(kind: u8) -> Option<&'static str> {
    Some(match kind {
        1 => "wpa-eap",
        2 => "wpa-psk",
        3 => "wpa-ft-eap",
        4 => "wpa-ft-psk",
        5 => "wpa-eap-sha256",
        6 => "wpa-psk-sha256",
        8 => "sae",
        9 => "ft-sae",
        11 => "wpa-eap-suite-b",
        12 => "wpa-eap-suite-b-192",
        18 => "owe",
        24 => "sae-ext-key",
        25 => "ft-sae-ext-key",
        _ => return None,
    })
}

/// The Wi-Fi Alliance's name for the ways an access point takes. WPA3 asks
/// for protected management; an access point taking WPA2 as well makes it
/// optional, for the cards that can't.
fn standard(ways: &[String], management: Management) -> &'static str {
    let any = |known: &[&str]| ways.iter().any(|way| known.contains(&way.as_str()));
    let wpa3 = any(&["sae", "sae-ext-key", "ft-sae", "ft-sae-ext-key"]);
    let wpa2 = any(&["wpa-psk", "wpa-ft-psk", "wpa-psk-sha256"]);
    let account = any(&["wpa-eap", "wpa-ft-eap"]);
    let account3 = any(&["wpa-eap-sha256"]);
    match (wpa3, wpa2) {
        (true, true) => return "WPA2/WPA3",
        (true, false) => return "WPA3",
        (false, true) => return "WPA2",
        (false, false) => {}
    }
    if any(&["wpa-eap-suite-b-192"]) {
        "WPA3 Enterprise 192-bit"
    } else if account3 && management == Management::Required {
        "WPA3 Enterprise"
    } else if account3 && management == Management::Optional {
        "WPA2/WPA3 Enterprise"
    } else if account || account3 || any(&["wpa-eap-suite-b"]) {
        "WPA2 Enterprise"
    } else if any(&["owe"]) {
        "OWE"
    } else {
        "RSN"
    }
}

/// The band a frequency in MHz is in.
pub fn band(frequency: u32) -> Band {
    match frequency {
        ..2500 => Band::Ghz2,
        2500..5935 => Band::Ghz5,
        _ => Band::Ghz6,
    }
}

/// The channel a frequency in MHz is, by its number.
pub fn channel(frequency: u32) -> u16 {
    let channel = match (band(frequency), frequency) {
        (Band::Ghz2, 2484) => 14,
        (Band::Ghz2, _) => frequency.saturating_sub(2407) / 5,
        (Band::Ghz5, _) => frequency.saturating_sub(5000) / 5,
        (Band::Ghz6, 5935) => 2,
        (Band::Ghz6, _) => frequency.saturating_sub(5950) / 5,
    };
    u16::try_from(channel).unwrap_or_default()
}

/// The noise wpa_supplicant takes a card to hear in each band where the
/// card doesn't say, in dBm.
const NOISE_2_GHZ: i16 = -89;
const NOISE_ABOVE: i16 = -92;

/// The least signal above the noise, in dB, each rate of one stream at
/// 20 MHz takes, in kb/s, as wpa_supplicant estimates them. HT's last is
/// VHT's alone, and HE's last two EHT's.
const HT_RATES: &[(i16, u32)] = &[
    (2, 6_500),
    (5, 13_000),
    (9, 19_500),
    (11, 26_000),
    (15, 39_000),
    (18, 52_000),
    (20, 58_500),
    (25, 65_000),
    (29, 78_000),
];
const HE_RATES: &[(i16, u32)] = &[
    (2, 8_600),
    (5, 17_200),
    (9, 25_800),
    (11, 34_400),
    (15, 51_600),
    (18, 68_800),
    (20, 77_400),
    (25, 86_000),
    (29, 103_200),
    (31, 114_700),
    (34, 129_000),
    (36, 143_400),
    (39, 154_900),
    (42, 172_100),
];
/// Before HT, a/g's rates, as wpa_supplicant limits them by signal.
const OLDER_RATES: &[(i16, u32)] = &[
    (2, 6_000),
    (5, 9_000),
    (6, 12_000),
    (9, 18_000),
    (10, 24_000),
    (14, 36_000),
    (18, 48_000),
    (20, 54_000),
];

/// How many more carriers each doubling of the channel carries data on
/// than 20 MHz, as its rates scale.
const HT_CARRIERS: [u32; 4] = [52, 108, 234, 468];
const HE_CARRIERS: [u32; 5] = [234, 468, 980, 1960, 3920];

/// A laptop's card hears on two streams at most, so an access point's
/// third and fourth go unused.
const CARD_STREAMS: u8 = 2;

/// About how fast a card would get on with an access point heard at this
/// signal, in Mb/s: what wpa_supplicant estimates one stream reaches, at
/// the access point's generation and width, for each stream the card can
/// share with it, less the time its channel is busy.
pub fn estimate(radio: &Radio, frequency: u32, signal: i16) -> u32 {
    let noise = if band(frequency) == Band::Ghz2 {
        NOISE_2_GHZ
    } else {
        NOISE_ABOVE
    };
    let (rates, carriers): (&[(i16, u32)], &[u32]) = match radio.generation {
        None => (OLDER_RATES, &HE_CARRIERS[..1]),
        Some(4) => (&HT_RATES[..8], &HT_CARRIERS[..2]),
        Some(5) => (HT_RATES, &HT_CARRIERS),
        Some(6) => (&HE_RATES[..12], &HE_CARRIERS[..4]),
        Some(_) => (HE_RATES, &HE_CARRIERS),
    };
    // Each doubling of the width spreads the signal over twice as much,
    // which takes 3 dB more for the same rate.
    let doublings = (radio.width.max(20) / 20).ilog2() as usize;
    let doublings = doublings.min(carriers.len() - 1);
    let above = signal.saturating_sub(noise) - 3 * i16::try_from(doublings).unwrap_or(0);
    let one = rate(rates, above) * carriers[doublings] / carriers[0] / 1000;
    let streams = u32::from(radio.streams.clamp(1, CARD_STREAMS));
    let free = 100 - u32::from(radio.load.map_or(0, |load| load.busy.min(100)));
    one * streams * free / 100
}

/// The rate a signal this far above the noise reaches, between the rates
/// it falls between.
fn rate(rates: &[(i16, u32)], above: i16) -> u32 {
    let Some(reached) = rates.iter().rposition(|(least, _)| above >= *least) else {
        return 0;
    };
    let (least, rate) = rates[reached];
    let Some(&(next, faster)) = rates.get(reached + 1) else {
        return rate;
    };
    let between = u32::try_from(above - least).unwrap_or(0);
    let apart = u32::try_from(next - least).unwrap_or(1);
    rate + between * (faster - rate) / apart
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Elements as an access point sends them, from their ids and bodies in
    /// hex; an id past 255 is an extension's, after 255.
    fn elements(elements: &[(u16, &str)]) -> Vec<u8> {
        let mut ies = Vec::new();
        for (id, body) in elements {
            let mut body = hex(body);
            let id = match u8::try_from(*id) {
                Ok(id) if id < 255 => id,
                _ => {
                    body.insert(0, u8::try_from(id - 256).unwrap());
                    255
                }
            };
            ies.push(id);
            ies.push(u8::try_from(body.len()).unwrap());
            ies.extend(body);
        }
        ies
    }

    fn hex(text: &str) -> Vec<u8> {
        let digits = text.as_bytes().chunks(2);
        let byte = |pair: &[u8]| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16);
        digits.map(|pair| byte(pair).unwrap()).collect()
    }

    // The bodies below were heard from access points near a hotel room.
    const HT_TWO_STREAMS: &str = "ef0903ffff000000000000000000000100000000000000000000";
    const VHT_TWO_STREAMS: &str = "b2e98933faff0000faff0000";
    const HE_TWO_STREAMS: &str = "0d01081a4010047008881f418104110800fafffaff391cc7711c07";
    const HE_OPERATING: &str = "f43f0035fcff";
    const WPA2_AND_WPA3: &str = "0100000fac040100000fac040200000fac02000fac088c00";
    const WPA2: &str = "0100000fac040100000fac040100000fac020c00";

    /// An access point on 5 GHz channel 40, 40 MHz wide.
    fn hotel() -> Vec<u8> {
        elements(&[
            (BSS_LOAD, "020020c06a"),
            (HT_CAPABILITIES, HT_TWO_STREAMS),
            (HT_OPERATION, "28070400000000000000000000000000000000000000"),
            (VHT_CAPABILITIES, VHT_TWO_STREAMS),
            (VHT_OPERATION, "002600fcff"),
            (HE_CAPABILITIES, HE_TWO_STREAMS),
            (HE_OPERATION, HE_OPERATING),
        ])
    }

    #[test]
    fn an_access_point_says_its_generation_width_streams_and_load() {
        let radio = Radio::read(&hotel(), 5200);

        assert_eq!(
            radio,
            Radio {
                generation: Some(6),
                width: 40,
                streams: 2,
                load: Some(Load {
                    stations: 2,
                    busy: 13
                }),
                protection: None,
            }
        );
    }

    #[test]
    fn a_channel_80_mhz_wide_is_said_in_its_vht_operation() {
        let phone = elements(&[
            (
                HT_CAPABILITIES,
                "6f001bffff000000000000000000000000000000000000000000",
            ),
            (HT_OPERATION, "95050400000000000000000000000000000000000000"),
            (VHT_CAPABILITIES, "3270810ffaff0000faff0000"),
            (VHT_OPERATION, "019b000000"),
        ]);

        let radio = Radio::read(&phone, 5745);

        assert_eq!((radio.generation, radio.width), (Some(5), 80));
    }

    #[test]
    fn a_channel_160_mhz_wide_is_one_whose_second_segment_sits_8_from_its_first() {
        let wide = elements(&[
            (HT_OPERATION, "24050400000000000000000000000000000000000000"),
            (VHT_OPERATION, "012a3200"),
        ]);

        let radio = Radio::read(&wide, 5180);

        assert_eq!(radio.width, 160);
    }

    #[test]
    fn a_channel_on_6_ghz_is_as_wide_as_its_he_operation_says() {
        // Its 6 GHz information: channel 37, 160 MHz wide, centred on 47.
        let six = elements(&[(HE_OPERATION, "04040200aabb25032f2f01")]);

        let radio = Radio::read(&six, 6135);

        assert_eq!((radio.generation, radio.width), (Some(6), 160));
    }

    #[test]
    fn a_channel_320_mhz_wide_is_said_only_in_its_eht_operation() {
        let widest = elements(&[
            (HE_OPERATION, "04040200aabb25032f2f01"),
            (EHT_OPERATION, "01110000000420"),
        ]);

        let radio = Radio::read(&widest, 6135);

        assert_eq!((radio.generation, radio.width), (Some(7), 320));
    }

    #[test]
    fn wi_fi_7_is_said_by_its_eht_capabilities() {
        let seven = elements(&[
            (HT_OPERATION, "06000400000000000000000000000000000000000000"),
            (
                HE_CAPABILITIES,
                "0d01181a4010006040880f41811c110800fafffaff191cc771",
            ),
            (EHT_CAPABILITIES, "9700e00101001876001200222220"),
            (EHT_OPERATION, "0411000000"),
        ]);

        let radio = Radio::read(&seven, 2437);

        assert_eq!(
            (radio.generation, radio.width, radio.streams),
            (Some(7), 20, 2)
        );
    }

    #[test]
    fn vht_on_2_4_ghz_is_still_wi_fi_4() {
        let stretched = elements(&[
            (HT_CAPABILITIES, HT_TWO_STREAMS),
            (HT_OPERATION, "06080000000000000000000000000000000000000000"),
            (VHT_CAPABILITIES, VHT_TWO_STREAMS),
            (VHT_OPERATION, "000000"),
        ]);

        let radio = Radio::read(&stretched, 2437);

        assert_eq!((radio.generation, radio.width), (Some(4), 20));
    }

    #[test]
    fn wi_fi_4_counts_its_streams_in_its_ht_capabilities() {
        let four = elements(&[(
            HT_CAPABILITIES,
            "ef1117ffffffff00000000000000000000000000000000000000",
        )]);

        let radio = Radio::read(&four, 2437);

        assert_eq!((radio.generation, radio.streams), (Some(4), 4));
    }

    #[test]
    fn an_access_point_saying_nothing_of_itself_is_older_one_stream_at_20_mhz() {
        let radio = Radio::read(&[], 2412);

        assert_eq!(
            (radio.generation, radio.width, radio.streams),
            (None, 20, 1)
        );
    }

    #[test]
    fn wpa2_and_wpa3_together_take_a_password_either_way() {
        let both = elements(&[(RSN, WPA2_AND_WPA3)]);

        let protection = Radio::read(&both, 5745).protection.unwrap();

        assert_eq!(
            protection,
            Protection {
                standard: "WPA2/WPA3".into(),
                ways: vec!["wpa-psk".into(), "sae".into()],
                ciphers: vec!["ccmp".into()],
                group: "ccmp".into(),
                management: Management::Optional,
            }
        );
    }

    #[test]
    fn wpa2_alone_leaves_its_management_unprotected() {
        let older = elements(&[(RSN, WPA2)]);

        let protection = Radio::read(&older, 2437).protection.unwrap();

        assert_eq!(protection.standard, "WPA2");
        assert_eq!(protection.management, Management::Off);
    }

    #[test]
    fn wpa3_alone_requires_its_management_protected() {
        let newer = elements(&[(RSN, "0100000fac040100000fac040100000fac08c000")]);

        let protection = Radio::read(&newer, 5180).protection.unwrap();

        assert_eq!(protection.standard, "WPA3");
        assert_eq!(protection.ways, ["sae"]);
        assert_eq!(protection.management, Management::Required);
    }

    #[test]
    fn an_account_protected_as_wpa3_asks_its_management_protected_too() {
        let office = elements(&[(RSN, "0100000fac040100000fac040100000fac05c000")]);
        let older = elements(&[(RSN, "0100000fac040100000fac040100000fac010000")]);

        let office = Radio::read(&office, 5180).protection.unwrap();
        let older = Radio::read(&older, 5180).protection.unwrap();

        assert_eq!(office.standard, "WPA3 Enterprise");
        assert_eq!(older.standard, "WPA2 Enterprise");
    }

    #[test]
    fn elements_cut_short_are_read_as_far_as_they_go() {
        let mut cut = hotel();
        cut.truncate(cut.len() - 3);
        cut.extend([u8::try_from(BSS_LOAD).unwrap(), 200]);

        let radio = Radio::read(&cut, 5200);

        assert_eq!(radio.load.unwrap().stations, 2);
        assert_eq!(radio.width, 40);
    }

    #[test]
    fn a_frequency_is_a_band_and_a_channel() {
        let heard = [2412, 2484, 5200, 5825, 5955, 6135, 7115];

        let said: Vec<_> = heard.map(|at| (band(at), channel(at))).into();

        assert_eq!(
            said,
            [
                (Band::Ghz2, 1),
                (Band::Ghz2, 14),
                (Band::Ghz5, 40),
                (Band::Ghz5, 165),
                (Band::Ghz6, 1),
                (Band::Ghz6, 37),
                (Band::Ghz6, 233),
            ]
        );
    }

    fn radio(generation: Option<u8>, width: u16, streams: u8, busy: Option<u8>) -> Radio {
        Radio {
            generation,
            width,
            streams,
            load: busy.map(|busy| Load { stations: 1, busy }),
            protection: None,
        }
    }

    #[test]
    fn the_estimate_is_what_one_stream_reaches_at_that_signal() {
        // 36 dB above 5 GHz's noise is HE's fastest at 20 MHz.
        let one = radio(Some(6), 20, 1, None);

        assert_eq!(estimate(&one, 5200, -56), 143);
    }

    #[test]
    fn a_stronger_signal_is_faster() {
        let one = radio(Some(6), 80, 2, None);

        assert!(estimate(&one, 5200, -50) > estimate(&one, 5200, -70));
    }

    #[test]
    fn a_wider_channel_is_faster_where_the_signal_carries_it() {
        let narrow = radio(Some(6), 20, 2, None);
        let wide = radio(Some(6), 80, 2, None);

        assert!(estimate(&wide, 5200, -55) > 3 * estimate(&narrow, 5200, -55));
    }

    #[test]
    fn a_newer_generation_is_faster_at_the_same_signal() {
        let four = radio(Some(4), 40, 2, None);
        let six = radio(Some(6), 40, 2, None);

        assert!(estimate(&six, 5200, -55) > estimate(&four, 5200, -55));
    }

    #[test]
    fn a_second_stream_doubles_it_and_a_third_goes_unused() {
        let one = radio(Some(6), 80, 1, None);
        let two = radio(Some(6), 80, 2, None);
        let four = radio(Some(6), 80, 4, None);

        let one = estimate(&one, 5200, -55);

        assert_eq!(estimate(&two, 5200, -55), 2 * one);
        assert_eq!(estimate(&four, 5200, -55), 2 * one);
    }

    #[test]
    fn a_busy_channel_leaves_less_of_it() {
        let free = radio(Some(6), 80, 2, Some(0));
        let busy = radio(Some(6), 80, 2, Some(50));

        assert_eq!(estimate(&busy, 5200, -55), estimate(&free, 5200, -55) / 2);
    }

    #[test]
    fn a_signal_lost_in_the_noise_reaches_nothing() {
        let one = radio(Some(6), 80, 2, None);

        assert_eq!(estimate(&one, 5200, -95), 0);
    }

    #[test]
    fn an_older_access_point_reaches_54_mb_s_at_most() {
        let old = radio(None, 20, 1, None);

        assert_eq!(estimate(&old, 2412, -40), 54);
    }
}
