//! The words the Home Network's rows and drawers are said in, read from
//! the launcher's vocabulary (launcher/words.json) as the shell hands it
//! over, so eero says no word of its own: every part's name, each thing's
//! said words and glyph, and the rates and counts worded from them. A
//! vocabulary lacking one is refused, so a word is said rather than drawn
//! as a blank.

use std::collections::BTreeMap;

use ready_rows::numbers::Numbers;
use serde_json::Value;

use crate::drawers;
use crate::place;
use crate::rows::{self, DOT};

/// A rate's units from Mb/s, as eero gives one, each a thousand of the
/// one before, by the word that says it.
const RATES: [(f64, &str); 4] = [(1e-6, "bps"), (1e-3, "kbps"), (1.0, "mbps"), (1e3, "gbps")];

/// The words each thing's `said` gives the rows, by the thing's id; a
/// vocabulary lacking one is refused, so it's said rather than drawn as
/// a blank.
const SAID: &[(&str, &[&str])] = &[
    (
        "eero",
        &[
            "bps",
            "kbps",
            "mbps",
            "gbps",
            "down",
            "up",
            "device",
            "devices",
            "eyebrow",
            "leadAt",
            "bytes",
            "kilobytes",
            "megabytes",
            "gigabytes",
            "terabytes",
        ],
    ),
    (
        "eero:internet",
        &[
            "mesh",
            "providerDown",
            "sqmOn",
            "sqmOff",
            "noOutages",
            "outage",
            "outages",
            "inDays",
            "node",
            "nodes",
            "offline",
            "pending",
            "onNow",
            "signedOut",
            "unreachable",
            "tested",
            "eyebrow",
            "asn",
            "ispDown",
            "updateReady",
            "anUpdate",
            "upToDate",
            "updateAt",
            "lastStarted",
            "answering",
            "fromHome",
            "awayFromHome",
            "notReached",
            "ms",
            "byProbe",
            "byEero",
            "timeNotGiven",
            "soFar",
            "days",
            "hours",
            "minutes",
            "underMinute",
            "speedTest",
            "speedTests",
            "since",
            "behindNat",
            "router",
            "addresses",
            "caching",
            "ipv6",
            "mask",
            "dhcp",
            "off",
            "on",
            "noInternet",
            "open",
            "subnet",
            "none",
            "optimized",
            "notOptimized",
            "clash",
            "noClash",
            "channel",
        ],
    ),
    (
        "eero:node",
        &[
            "gateway",
            "ethernet",
            "wifi",
            "from",
            "over",
            "fromOver",
            "bars",
            "2.4GHz",
            "5.2GHz",
            "5.8GHz",
            "5GHz",
            "6GHz",
            "ghz",
            "onNetwork",
            "node",
            "allWifi",
            "allEthernet",
            "onWifi",
            "onEthernet",
            "mhz",
            "updateAvailable",
            "upToDate",
            "on",
            "off",
            "percent",
            "internet",
            "backhaul",
            "in",
            "out",
            "notLinked",
            "upTo",
            "of",
            "late",
            "never",
            "ago",
            "joined",
            "left",
        ],
    ),
    (
        "eero:device",
        &[
            "thisMachine",
            "ethernet",
            "dbm",
            "channel",
            "idle",
            "made",
            "onNode",
            "wifi",
            "db",
            "bars",
            "percent",
            "mhz",
            "mcs",
            "stream",
            "streams",
            "guard",
            "quiet",
            "packetsDown",
            "packetsUp",
            "inAll",
            "perMillion",
            "guest",
            "reserved",
            "private",
            "paused",
            "blocked",
            "to",
        ],
    ),
    (
        "eero:profile",
        &[
            "paused",
            "onNow",
            "noneOnNow",
            "eyebrow",
            "gaming",
            "illegal",
            "messaging",
            "pornographic",
            "social",
            "shopping",
            "streaming",
            "violent",
            "ads",
            "adsNetwork",
            "blocksNothing",
            "safeSearch",
            "youtubeRestricted",
            "noSchedule",
            "schedule",
            "schedules",
            "yes",
            "no",
            "listed",
            "onNode",
            "notOnNow",
        ],
    ),
    ("verb:eero:speedtest", &["note"]),
    ("eero:sqm", &["note"]),
];

/// The things whose glyph a row wears.
const GLYPHS: [&str; 6] = [
    "eero:internet",
    "eero:node",
    "eero:gateway",
    "eero:profile",
    rows::SPEED_TEST,
    rows::SQM,
];

/// The words the Home Network's rows are said in, from the launcher's
/// vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// The place's name, and the internet row's.
    place: String,
    pub(crate) internet: String,
    /// The names of the parts the place's drawer says (place.rs's NAMES),
    /// and its lead's sentences, the place's `tells` (place.rs's TELLS).
    names: BTreeMap<&'static str, String>,
    tells: BTreeMap<&'static str, String>,
    glyphs: BTreeMap<&'static str, String>,
    said: BTreeMap<(&'static str, &'static str), String>,
}

impl Words {
    /// The Home Network's words from the whole vocabulary, or the first it
    /// lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let thing = |id: &str| {
            vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let word = |id: &str, at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(thing(id)?, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let mut words = Words {
            place: word("eero", &["name"])?,
            internet: word("eero:internet", &["name"])?,
            ..Words::default()
        };
        for id in place::NAMES
            .into_iter()
            .chain(drawers::NAMES)
            .chain(drawers::NODE_NAMES)
            .chain(drawers::DEVICE_NAMES)
            .chain(drawers::INTERNET_NAMES)
            .chain([rows::SPEED_TEST, rows::SQM])
        {
            words.names.insert(id, word(id, &["name"])?);
        }
        for one in place::TELLS {
            words.tells.insert(one, word("eero", &["tells", one])?);
        }
        for id in GLYPHS {
            words.glyphs.insert(id, word(id, &["glyph"])?);
        }
        for (id, said) in SAID {
            for one in *said {
                words.said.insert((id, one), word(id, &["said", one])?);
            }
        }
        Ok(words)
    }

    /// A word a thing says, which `read` made sure of.
    pub(crate) fn say(&self, id: &'static str, word: &'static str) -> &str {
        self.said.get(&(id, word)).map_or("", String::as_str)
    }

    pub(crate) fn place(&self) -> &str {
        &self.place
    }

    /// A part's name, of those `read` made sure of.
    pub(crate) fn name(&self, id: &'static str) -> &str {
        self.names.get(id).map_or("", String::as_str)
    }

    /// One of the place's sentences, which `read` made sure of.
    pub(crate) fn tells(&self, one: &'static str) -> &str {
        self.tells.get(one).map_or("", String::as_str)
    }

    pub(crate) fn glyph(&self, id: &'static str) -> &str {
        self.glyphs.get(id).map_or("", String::as_str)
    }

    /// A count with its noun, the thing's `one` word for one and its
    /// `many` with `{count}` for any other: "1 device", "9 devices".
    pub(crate) fn counted(
        &self,
        count: usize,
        id: &'static str,
        one: &'static str,
        many: &'static str,
    ) -> String {
        if count == 1 {
            self.say(id, one).to_owned()
        } else {
            fill(self.say(id, many), &[("count", &count.to_string())])
        }
    }

    pub(crate) fn devices(&self, count: usize) -> String {
        self.counted(count, "eero", "device", "devices")
    }

    /// A rate given in Mb/s, in whichever unit says it in three digits:
    /// "370 Mb/s", "1.20 Gb/s", "98.2 b/s".
    pub(crate) fn rate(&self, mbps: f64, numbers: &Numbers) -> String {
        let size: f64 = format!("{:.2e}", mbps.abs()).parse().unwrap_or(0.0);
        let (scale, unit) = RATES
            .iter()
            .rev()
            .find(|(scale, _)| size >= *scale)
            .unwrap_or(&RATES[0]);
        fill(
            self.say("eero", unit),
            &[("n", &numbers.significant(mbps / scale))],
        )
    }

    /// Rates each way, as one item: "1.20 Gb/s down · 1.08 Gb/s up", or
    /// the one way eero gives.
    pub(crate) fn each_way(&self, down: Option<f64>, up: Option<f64>, numbers: &Numbers) -> String {
        let way = |mbps: Option<f64>, word| {
            mbps.map(|mbps| {
                fill(
                    self.say("eero", word),
                    &[("rate", &self.rate(mbps, numbers))],
                )
            })
        };
        [way(down, "down"), way(up, "up")]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(DOT)
    }
}

/// Words with each `{hole}` filled.
pub(crate) fn fill(words: &str, holes: &[(&str, &str)]) -> String {
    holes.iter().fold(words.to_owned(), |said, (hole, with)| {
        said.replace(&format!("{{{hole}}}"), with)
    })
}
