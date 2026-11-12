//! The Home Network's rows in the launcher, made ready here so the shell
//! only draws them: the internet first, then each node with the devices
//! on it under it, then the profiles. Each is worded from the launcher's
//! vocabulary (launcher/words.json) as the shell hands it over, so eero
//! says no word of its own, and each is the ready-row form every daemon
//! serves (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md), sent
//! whole only when something it draws changes (listeners.rs).
//!
//! A figure eero doesn't give is said by nothing here rather than guessed,
//! and the day of the last speed test travels as a time, so no row changes
//! as the clock does.

use std::collections::BTreeMap;

use ready_rows::numbers::Numbers;
use ready_rows::order::by_name;
use ready_rows::time::{self, Form};
use serde_json::{Value, json};

use crate::called;
use crate::drawers;
use crate::model::{BackhaulKind, CloudState, Device, HomeNetwork, Node, Profile};
use crate::place;
use crate::time::to_unix;

/// The launcher's place the Home Network's rows are in.
pub const PLACE: &str = "eero";

/// How the items a row's side or note gives way whole are joined, where
/// one item holds several.
const DOT: &str = " \u{b7} ";

/// The theme's mark for each kind of device eero names, by eero's word;
/// any other wears Wi-Fi's or Ethernet's, as how it's on the network is
/// all that's known of it.
const MARKS: &[(&str, &str)] = &[
    ("phone", "phone"),
    ("laptop_computer", "computer"),
    ("unknown_computer", "computer"),
    ("computer", "computer"),
    ("desktop_computer", "computer"),
    ("audio", "speaker"),
    ("speaker", "speaker"),
    ("game_console", "gamepad"),
];

/// A rate's units from Mb/s, as eero gives one, each a thousand of the
/// one before, by the word that says it.
const RATES: [(f64, &str); 4] = [(1e-6, "bps"), (1e-3, "kbps"), (1.0, "mbps"), (1e3, "gbps")];

/// The bands eero names a backhaul's radio by, each said as the node's
/// own radios name it: 5.8 GHz is the upper 5 GHz radio, 5.2 GHz the
/// lower. Any other is said by `ghz`.
const RADIOS: [&str; 5] = ["2.4GHz", "5.2GHz", "5.8GHz", "5GHz", "6GHz"];

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
        ],
    ),
    (
        "eero:node",
        &[
            "gateway", "ethernet", "wifi", "from", "over", "fromOver", "bars", "2.4GHz", "5.2GHz",
            "5.8GHz", "5GHz", "6GHz", "ghz",
        ],
    ),
    (
        "eero:device",
        &["thisMachine", "ethernet", "dbm", "channel", "idle", "made"],
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
];

/// The things whose glyph a row wears.
const GLYPHS: [&str; 4] = ["eero:internet", "eero:node", "eero:gateway", "eero:profile"];

/// The words the Home Network's rows are said in, from the launcher's
/// vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// The place's name, and the internet row's.
    place: String,
    internet: String,
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
        for id in place::NAMES.into_iter().chain(drawers::NAMES) {
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

    fn glyph(&self, id: &'static str) -> &str {
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
    fn each_way(&self, down: Option<f64>, up: Option<f64>, numbers: &Numbers) -> String {
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

/// Items as a row's side or note serves them, those saying nothing left
/// out.
fn items(said: impl IntoIterator<Item = Value>) -> Vec<Value> {
    said.into_iter()
        .filter(|item| item.get("said").is_none_or(|said| said != ""))
        .collect()
}

fn item(said: impl Into<String>) -> Value {
    json!({"said": said.into()})
}

pub fn key(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

/// Every row, in the order the place lists them, each without its
/// `order`, which ready-rows' `Kept` gives it.
pub fn rows(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let mut rows = vec![internet_row(model, words, numbers)];
    for (node, devices) in grouped(model) {
        if let Some(node) = node {
            rows.push(node_row(node, words));
        }
        for device in devices {
            rows.push(device_row(device, &model.profiles, words, numbers));
        }
    }
    for profile in &model.profiles {
        rows.push(profile_row(model, profile, words));
    }
    rows
}

/// What a node is called: where it stands, as eero names it.
pub(crate) fn node_name(node: &Node) -> &str {
    node.location
        .as_deref()
        .or(node.model.as_deref())
        .unwrap_or(&node.id)
}

/// The nodes as the place lists them: the gateway node first, then each
/// node after the one it hangs from, as the backhaul runs, siblings by
/// location, and any whose upstream eero didn't name last.
fn nodes_in_order(nodes: &[Node]) -> Vec<&Node> {
    fn under<'a>(node: &'a Node, nodes: &'a [Node], listed: &mut Vec<&'a Node>) {
        listed.push(node);
        let mut below: Vec<&Node> = nodes
            .iter()
            .filter(|other| {
                !listed.iter().any(|one| one.id == other.id)
                    && other.backhaul.upstream.is_some()
                    && other.backhaul.upstream == node.location
            })
            .collect();
        below.sort_by(|one, other| by_name(node_name(one), node_name(other)));
        for other in below {
            if !listed.iter().any(|one| one.id == other.id) {
                under(other, nodes, listed);
            }
        }
    }
    let mut sorted: Vec<&Node> = nodes.iter().collect();
    sorted.sort_by(|one, other| by_name(node_name(one), node_name(other)));
    let mut listed = Vec::new();
    for node in sorted.iter().filter(|node| node.gateway) {
        if !listed.iter().any(|one: &&Node| one.id == node.id) {
            under(node, nodes, &mut listed);
        }
    }
    for node in &sorted {
        if !listed.iter().any(|one| one.id == node.id) {
            under(node, nodes, &mut listed);
        }
    }
    listed
}

/// Each node in that order with its devices, as eero orders them (by
/// node, the strongest signal first) but for this machine, first under
/// its node, and the devices on no node eero listed, as one group with no
/// node, last.
fn grouped(model: &HomeNetwork) -> Vec<(Option<&Node>, Vec<&Device>)> {
    let nodes = nodes_in_order(&model.nodes);
    let devices: Vec<&Device> = model
        .devices
        .iter()
        .filter(|device| device.this_machine)
        .chain(model.devices.iter().filter(|device| !device.this_machine))
        .collect();
    let on = |node: &Node| {
        devices
            .iter()
            .copied()
            .filter(|device| device.node.as_deref() == Some(node.id.as_str()))
            .collect()
    };
    let mut groups: Vec<(Option<&Node>, Vec<&Device>)> =
        nodes.iter().map(|node| (Some(*node), on(node))).collect();
    let astray: Vec<&Device> = devices
        .iter()
        .copied()
        .filter(|device| {
            !nodes
                .iter()
                .any(|node| device.node.as_deref() == Some(node.id.as_str()))
        })
        .collect();
    if !astray.is_empty() {
        groups.push((None, astray));
    }
    groups
}

/// The internet's row, the place's head: how it stands then the last
/// speed test's down at its side, and after its name the test's up and
/// its day, how it's optimized, its outages, the nodes by state and how
/// many devices are on now. While eero isn't answering, its side says that
/// alone, as the figures are its last.
fn internet_row(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Value {
    let internet = &model.internet;
    let test = &internet.speed_test;
    let answering = model.cloud.state == CloudState::Answering;
    let state = state_said(model, words);
    let down = words.each_way(test.down_mbps, None, numbers);
    // The side's figure is the last speed test's, not the speed now, so
    // it says so (VKCF3Z, 2026-11-12).
    let tested = if down.is_empty() {
        String::new()
    } else {
        fill(words.say("eero:internet", "tested"), &[("speed", &down)])
    };
    let up = words.each_way(None, test.up_mbps, numbers);
    let day = test
        .at
        .as_deref()
        .and_then(to_unix)
        .filter(|_| !(up.is_empty() && down.is_empty()))
        .map(|seconds| time::at(seconds * 1000, Form::On));
    let mesh = internet.mesh.as_deref().unwrap_or("");
    let mesh = if mesh.is_empty() || mesh == "connected" {
        String::new()
    } else {
        fill(words.say("eero:internet", "mesh"), &[("state", mesh)])
    };
    let provider = if internet.isp_up == Some(false) {
        words.say("eero:internet", "providerDown")
    } else {
        ""
    };
    let sqm = match internet.sqm {
        Some(true) => words.say("eero:internet", "sqmOn"),
        Some(false) => words.say("eero:internet", "sqmOff"),
        None => "",
    };
    let outages = outages_said(model, words);
    let on_now = match model.devices.len() {
        0 => String::new(),
        count => fill(
            words.say("eero:internet", "onNow"),
            &[("devices", &words.devices(count))],
        ),
    };
    let mut note = vec![item(mesh), item(provider), item(up)];
    note.extend(day);
    note.extend([
        item(sqm),
        item(outages),
        item(nodes_said(model, words)),
        item(on_now),
    ]);
    json!({
        "key": key("internet"), "thing": "eero:internet", "place": PLACE, "kind": "internet",
        "name": words.internet, "glyph": words.glyph("eero:internet"), "hue": "machine",
        "depth": 0,
        "side": items([item(state), item(if answering { tested } else { String::new() })]),
        "note": items(note),
    })
}

/// How the internet stands, in eero's word lowered, or why nothing's
/// known while eero isn't answering.
pub(crate) fn state_said(model: &HomeNetwork, words: &Words) -> String {
    match model.cloud.state {
        CloudState::SignedOut => words.say("eero:internet", "signedOut").to_owned(),
        CloudState::Unreachable => words.say("eero:internet", "unreachable").to_owned(),
        CloudState::Answering => model.internet.state.as_deref().unwrap_or("").to_lowercase(),
    }
}

/// The outages the last 30 days held: "1 outage in 30 days".
pub(crate) fn outages_said(model: &HomeNetwork, words: &Words) -> String {
    let outages = match model.internet.history.outages.len() {
        0 => words.say("eero:internet", "noOutages").to_owned(),
        count => words.counted(count, "eero:internet", "outage", "outages"),
    };
    fill(
        words.say("eero:internet", "inDays"),
        &[("outages", &outages)],
    )
}

/// The nodes by state, as one item: "3 nodes online", with any offline or
/// pending after.
pub(crate) fn nodes_said(model: &HomeNetwork, words: &Words) -> String {
    let counts = &model.internet.nodes;
    let Some(online) = counts.online else {
        return String::new();
    };
    let each = |count: Option<u32>, word| {
        count.filter(|count| *count > 0).map(|count| {
            fill(
                words.say("eero:internet", word),
                &[("count", &count.to_string())],
            )
        })
    };
    let online = usize::try_from(online).unwrap_or(usize::MAX);
    std::iter::once(words.counted(online, "eero:internet", "node", "nodes"))
        .chain(each(counts.offline, "offline"))
        .chain(each(counts.pending, "pending"))
        .collect::<Vec<_>>()
        .join(DOT)
}

/// How a node reaches the one before it, with its bars over Wi-Fi:
/// "Ethernet", or "Wi-Fi from Hall over 5 GHz high · 5 of 5 bars".
fn backhaul_said(node: &Node, words: &Words) -> String {
    let way = &node.backhaul;
    if way.kind == BackhaulKind::Ethernet {
        return words.say("eero:node", "ethernet").to_owned();
    }
    let radio = way.radio.as_deref().map(|radio| {
        RADIOS.iter().find(|named| **named == radio).map_or_else(
            || {
                let ghz = radio.trim_end_matches("GHz").trim_end_matches("ghz").trim();
                fill(words.say("eero:node", "ghz"), &[("n", ghz)])
            },
            |named| words.say("eero:node", named).to_owned(),
        )
    });
    let upstream = way.upstream.as_deref();
    let said = match (upstream, &radio) {
        (Some(from), Some(radio)) => fill(
            words.say("eero:node", "fromOver"),
            &[("node", from), ("radio", radio)],
        ),
        (Some(from), None) => fill(words.say("eero:node", "from"), &[("node", from)]),
        (None, Some(radio)) => fill(words.say("eero:node", "over"), &[("radio", radio)]),
        (None, None) => words.say("eero:node", "wifi").to_owned(),
    };
    match way.bars {
        Some(bars) => format!(
            "{said}{DOT}{}",
            fill(words.say("eero:node", "bars"), &[("n", &bars.to_string())])
        ),
        None => said,
    }
}

/// A node's row: how many devices are on it at its side, and after its
/// name how it's doing where that isn't online, whether it's the gateway
/// node, its backhaul, its model and its firmware, the firmware giving way
/// first as the row narrows.
fn node_row(node: &Node, words: &Words) -> Value {
    let state = node.state.as_deref().unwrap_or("").to_lowercase();
    let devices = node
        .devices
        .all
        .map(|all| words.devices(usize::try_from(all).unwrap_or(usize::MAX)));
    let glyph = if node.gateway {
        words.glyph("eero:gateway")
    } else {
        words.glyph("eero:node")
    };
    let keywords: Vec<&str> = [&node.model, &node.serial]
        .into_iter()
        .filter_map(|one| one.as_deref())
        .collect();
    json!({
        "key": key(&format!("node:{}", node.id)), "thing": format!("eero:node:{}", node.id),
        "place": PLACE, "kind": "node", "name": node_name(node), "glyph": glyph,
        "hue": "machine", "depth": 0,
        "side": items(devices.map(item)),
        "note": items([
            item(if state == "online" { String::new() } else { state }),
            item(if node.gateway { words.say("eero:node", "gateway") } else { "" }),
            item(backhaul_said(node, words)),
            item(node.model.clone().unwrap_or_default()),
            item(node.firmware.clone().unwrap_or_default()),
        ]),
        "keywords": keywords,
        "held": {"id": node.id, "gateway": node.gateway},
    })
}

/// A device's row: this machine and its signal at its side, and after its
/// name its band and channel, its rate each way, its speed now, its
/// address and the profile it's in.
fn device_row(device: &Device, profiles: &[Profile], words: &Words, numbers: &Numbers) -> Value {
    let signal = if device.ethernet {
        words.say("eero:device", "ethernet").to_owned()
    } else {
        device.signal_dbm.map_or_else(String::new, |dbm| {
            fill(
                words.say("eero:device", "dbm"),
                &[("n", &numbers.whole(f64::from(dbm)))],
            )
        })
    };
    let band = if device.ethernet {
        words.say("eero:device", "ethernet").to_owned()
    } else {
        let channel = device.channel.map(|channel| {
            fill(
                words.say("eero:device", "channel"),
                &[("n", &channel.to_string())],
            )
        });
        device
            .band
            .clone()
            .into_iter()
            .chain(channel)
            .collect::<Vec<_>>()
            .join(DOT)
    };
    let rate = |one: &Option<crate::model::Rate>| one.as_ref().and_then(|rate| rate.mbps);
    let rates = words.each_way(rate(&device.rx), rate(&device.tx), numbers);
    let speed = &device.speed;
    let speed = if speed.down_mbps == Some(0.0) && speed.up_mbps == Some(0.0) {
        words.say("eero:device", "idle").to_owned()
    } else {
        words.each_way(speed.down_mbps, speed.up_mbps, numbers)
    };
    let profile = device
        .profile
        .as_deref()
        .and_then(|id| profiles.iter().find(|profile| profile.id == id))
        .and_then(|profile| profile.name.clone())
        .unwrap_or_default();
    let kind = device.kind.as_deref().unwrap_or("");
    let glyph = MARKS.iter().find(|(named, _)| *named == kind).map_or(
        if device.ethernet { "wired" } else { "wireless" },
        |(_, mark)| mark,
    );
    let keywords: Vec<String> = [
        Some(device.mac.clone()),
        device.hostname.clone(),
        device.address.clone(),
        Some(kind.replace('_', " ").trim().to_owned()),
    ]
    .into_iter()
    .flatten()
    .filter(|word| !word.is_empty())
    .collect();
    json!({
        "key": key(&format!("device:{}", device.mac)),
        "thing": format!("eero:device:{}", device.mac), "place": PLACE, "kind": "device",
        "name": called::device(device, words.say("eero:device", "made")), "glyph": glyph, "hue": "machine", "depth": 1,
        "side": items([
            item(if device.this_machine { words.say("eero:device", "thisMachine") } else { "" }),
            item(signal),
        ]),
        "note": items([
            item(band),
            item(rates),
            item(speed),
            item(device.address.clone().unwrap_or_default()),
            item(profile),
        ]),
        "keywords": keywords,
        "held": {"mac": device.mac, "node": device.node},
    })
}

/// A profile's row: how it stands at its side, and how many devices it
/// holds and how many of them are on now.
fn profile_row(model: &HomeNetwork, profile: &Profile, words: &Words) -> Value {
    let state = if profile.paused == Some(true) {
        words.say("eero:profile", "paused").to_owned()
    } else {
        profile.state.as_deref().unwrap_or("").to_lowercase()
    };
    let on_now = profile
        .devices
        .iter()
        .filter(|mac| model.devices.iter().any(|device| &device.mac == *mac))
        .count();
    let on_now = if on_now == 0 {
        words.say("eero:profile", "noneOnNow").to_owned()
    } else {
        fill(
            words.say("eero:profile", "onNow"),
            &[("count", &on_now.to_string())],
        )
    };
    json!({
        "key": key(&format!("profile:{}", profile.id)),
        "thing": format!("eero:profile:{}", profile.id), "place": PLACE, "kind": "profile",
        "name": profile.name.as_deref().unwrap_or(&profile.id),
        "glyph": words.glyph("eero:profile"), "hue": "machine", "depth": 0,
        "side": items([item(state)]),
        "note": items([item(words.devices(profile.devices.len())), item(on_now)]),
        "drawer": drawers::profile_key(&profile.id),
        "held": {"id": profile.id},
    })
}

#[cfg(test)]
#[path = "rows_tests.rs"]
pub(crate) mod tests;
