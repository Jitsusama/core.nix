//! A device's drawer, made ready here as its row is, every fact as the
//! step into it lists them (its drawer holds a glance of six, drawers.rs's
//! GLANCE): how well it hears and is heard, its band, each way's rate and
//! how the radio makes it, and its speed now, first; then what it is and which node it's on, what had to be sent
//! again, its network and addresses, when it was seen, its profile, where
//! it moved over the last day, and what eero said and the daemon told of
//! it. What moves sparks its last hour, as the history keeps it. Every
//! time travels as the shell words it (ready-rows' time.rs).

use ready_rows::numbers::Numbers;
use ready_rows::time::{self as said_at, Form};
use serde_json::{Value, json};

use super::node::{BREAK, KEPT, fact, heading, history_spark, stroke, timed, under};
use super::{DRAWER, joined, timeline};
use crate::called;
use crate::model::{Device, HomeNetwork, Rate};
use crate::rows::{Words, fill, node_name};
use crate::time;

/// The parts a device's drawer names, each by its thing in the vocabulary.
pub const NAMES: [&str; 31] = [
    "eero:device:signal",
    "eero:device:snr",
    "eero:device:score",
    "eero:device:band",
    "eero:device:port",
    "eero:device:rx",
    "eero:device:tx",
    "eero:device:speed",
    "eero:device:kind",
    "eero:device:node",
    "eero:device:bssid",
    "eero:device:retries",
    "eero:device:packets",
    "eero:device:resent",
    "eero:device:failed",
    "eero:device:dropped",
    "eero:device:where",
    "eero:device:network",
    "eero:device:address",
    "eero:device:reserved",
    "eero:device:addresses",
    "eero:device:mac",
    "eero:device:hostname",
    "eero:device:nickname",
    "eero:device:first-seen",
    "eero:device:on-since",
    "eero:device:last-active",
    "eero:device:profile",
    "eero:device:held",
    "eero:device:roams",
    "eero:device:events",
];

/// The faintest and the strongest a signal is drawn between, in dBm, as
/// every other signal the launcher draws (Interfaces.js's `level`).
const FAINTEST: f64 = -90.0;
const LOUDEST: f64 = -30.0;

/// All of eero's score.
const PERCENT: f64 = 100.0;

/// The units a figure in eero's own sentences runs into, "5.2GHz".
const UNITS: [&str; 3] = ["GHz", "MHz", "kHz"];

/// The key a device's drawer goes by.
pub fn key(mac: &str) -> String {
    format!("drawer:eero:device:{mac}")
}

/// Every device's drawer, as a record beside the rows.
pub fn records(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    model
        .devices
        .iter()
        .map(|device| {
            json!({
                "key": key(&device.mac),
                "kind": DRAWER,
                "drawer": drawer(model, device, words, numbers),
            })
        })
        .collect()
}

/// A device's drawer, titled as its row is called. Its eyebrow is the node
/// it's on and how, "on Den · Wi-Fi", a node's name kept as it's written.
fn drawer(model: &HomeNetwork, device: &Device, words: &Words, numbers: &Numbers) -> Value {
    let say = |word| words.say("eero:device", word);
    let title = called::device(device, say("made"));
    let on = device
        .node
        .as_deref()
        .and_then(|id| model.nodes.iter().find(|node| node.id == id));
    let how = say(if device.ethernet { "ethernet" } else { "wifi" });
    let at = on.map(|node| fill(say("onNode"), &[("name", node_name(node))]));
    let mut facts: Vec<Value> = link(device, words, numbers);
    facts.extend(what(model, device, words));
    facts.extend(resent(device, words, numbers));
    facts.extend(where_on(model, device, &title, words));
    facts.extend(seen_and_held(model, device, words));
    facts.extend(roams(model, device, words));
    facts.extend(events(model, device, words));
    json!({
        "eyebrow": joined(at.into_iter().chain([how.to_owned()])),
        "eyebrowNames": on.is_some(),
        "title": title,
        "facts": facts,
    })
}

/// How well it hears and is heard, its band or port, each way's rate and
/// its speed now, each sparked over the hour where the history keeps it.
fn link(device: &Device, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:device", word);
    let history = &device.history;
    let wide = |values: &[Option<u16>]| -> Vec<Option<f64>> {
        values.iter().map(|value| value.map(f64::from)).collect()
    };
    let signal = device.signal_dbm.and_then(|dbm| {
        let said = fill(say("dbm"), &[("n", &numbers.whole(f64::from(dbm)))]);
        let levels: Vec<Option<f64>> = history
            .signal_dbm
            .iter()
            .map(|dbm| dbm.map(|dbm| level(f64::from(dbm))))
            .collect();
        let figure =
            history_spark(&levels, Some(1.0)).unwrap_or_else(|| stroke(level(f64::from(dbm))));
        drawn(fact(words, "eero:device:signal", &said), Some(figure))
    });
    let snr = device.snr_db.and_then(|db| {
        let said = fill(say("db"), &[("n", &db.to_string())]);
        drawn(
            fact(words, "eero:device:snr", &said),
            history_spark(&wide(&history.snr_db), None),
        )
    });
    // eero's score says one thing: its bars, or where it gives none, its
    // share as a percentage, as every other share here is (VKCF3Z,
    // 2026-11-08).
    let score = match (device.bars, device.score) {
        (Some(bars), _) => fill(say("bars"), &[("n", &bars.to_string())]),
        (None, Some(score)) => fill(say("percent"), &[("n", &numbers.whole(score * PERCENT))]),
        (None, None) => String::new(),
    };
    let band = if device.ethernet {
        String::new()
    } else {
        joined(
            [
                device.band.clone(),
                device
                    .channel
                    .map(|n| fill(say("channel"), &[("n", &n.to_string())])),
                device
                    .frequency_mhz
                    .map(|mhz| fill(say("mhz"), &[("n", &numbers.number(f64::from(mhz), 0))])),
            ]
            .into_iter()
            .flatten(),
        )
    };
    let port = device.port.as_ref().map_or_else(String::new, |port| {
        joined(
            [
                port.name.clone(),
                port.mbps.map(|mbps| words.rate(f64::from(mbps), numbers)),
            ]
            .into_iter()
            .flatten(),
        )
    });
    let speed = &device.speed;
    let speed_said = if speed.down_mbps == Some(0.0) && speed.up_mbps == Some(0.0) {
        say("idle").to_owned()
    } else {
        words.each_way(speed.down_mbps, speed.up_mbps, numbers)
    };
    [
        signal,
        snr,
        fact(words, "eero:device:score", &score),
        fact(words, "eero:device:band", &band),
        fact(words, "eero:device:port", &port),
        drawn(
            fact(
                words,
                "eero:device:rx",
                &rate_of(device.rx.as_ref(), words, numbers),
            ),
            history_spark(&history.rx_mbps, None),
        ),
        drawn(
            fact(
                words,
                "eero:device:tx",
                &rate_of(device.tx.as_ref(), words, numbers),
            ),
            history_spark(&history.tx_mbps, None),
        ),
        drawn(
            fact(words, "eero:device:speed", &speed_said),
            history_spark(&together(&history.down_mbps, &history.up_mbps), None),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// What it is, which node it's on and which of its radios it hears.
fn what(model: &HomeNetwork, device: &Device, words: &Words) -> Vec<Value> {
    let on = device
        .node
        .as_deref()
        .and_then(|id| model.nodes.iter().find(|node| node.id == id));
    let kind = device
        .kind
        .as_deref()
        .map(|kind| kind.replace('_', " ").trim().to_owned());
    [
        fact(
            words,
            "eero:device:kind",
            &joined(
                [
                    kind,
                    device.make.clone(),
                    device.model.clone(),
                    device.manufacturer.clone(),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:node",
            &on.map_or_else(String::new, |node| {
                joined(
                    [Some(node_name(node).to_string()), node.model.clone()]
                        .into_iter()
                        .flatten(),
                )
            }),
        ),
        fact(
            words,
            "eero:device:bssid",
            device.bssid.as_deref().unwrap_or(""),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// What had to be sent again: how often it retries and how long it's been
/// quiet, its packets each way, and those sent again, never sent and
/// dropped.
fn resent(device: &Device, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:device", word);
    let packets = device.packets.as_ref();
    let counted = |count: Option<u64>, word: Option<&'static str>| {
        count.map(|count| {
            let figure = numbers.number(count_f64(count), 0);
            word.map_or_else(|| figure.clone(), |word| fill(say(word), &[("n", &figure)]))
        })
    };
    let ppm = |count: Option<u32>| counted(count.map(u64::from), Some("perMillion"));
    let facts: Vec<Option<Value>> = vec![
        fact(
            words,
            "eero:device:retries",
            &joined(
                [
                    device
                        .retry_pct
                        .map(|pct| fill(say("percent"), &[("n", &numbers.number(pct, 1))])),
                    device
                        .idle_ms
                        .map(|ms| fill(say("quiet"), &[("n", &numbers.number(count_f64(ms), 0))])),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:packets",
            &joined(
                [
                    counted(packets.and_then(|one| one.rx_packets), Some("packetsDown")),
                    counted(packets.and_then(|one| one.tx_packets), Some("packetsUp")),
                    counted(packets.and_then(|one| one.total_packets), Some("inAll")),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:resent",
            &joined(
                [
                    counted(packets.and_then(|one| one.tx_retries), None),
                    ppm(packets.and_then(|one| one.tx_retransmit_ppm)),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:failed",
            &ppm(packets.and_then(|one| one.tx_fail_ppm)).unwrap_or_default(),
        ),
        fact(
            words,
            "eero:device:dropped",
            &joined(
                [
                    counted(packets.and_then(|one| one.rx_drops), None),
                    ppm(packets.and_then(|one| one.rx_drop_ppm)),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
    ];
    facts.into_iter().flatten().collect()
}

/// Under its own heading, its network, its addresses and its names.
fn where_on(model: &HomeNetwork, device: &Device, title: &str, words: &Words) -> Vec<Value> {
    let say = |word| words.say("eero:device", word);
    let mut facts: Vec<Option<Value>> = vec![
        Some(heading(words, "eero:device:where")),
        fact(
            words,
            "eero:device:network",
            &joined(
                [
                    device.ssid.clone(),
                    device.auth.as_deref().map(str::to_uppercase),
                    device.subnet.clone(),
                    (device.guest == Some(true)).then(|| say("guest").to_owned()),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
    ];
    facts.extend(address(model, device, words));
    let address = device.address.as_deref();
    facts.extend([
        fact(
            words,
            "eero:device:addresses",
            &joined(
                device
                    .addresses
                    .iter()
                    .filter(|each| Some(each.as_str()) != address)
                    .cloned(),
            ),
        ),
        fact(
            words,
            "eero:device:mac",
            &joined(
                [
                    Some(device.mac.clone()),
                    (device.private == Some(true)).then(|| say("private").to_owned()),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:hostname",
            device
                .hostname
                .as_deref()
                .filter(|hostname| *hostname != title)
                .unwrap_or(""),
        ),
        fact(
            words,
            "eero:device:nickname",
            device.nickname.as_deref().unwrap_or(""),
        ),
    ]);
    facts.into_iter().flatten().collect()
}

/// When it was seen, its profile and whether it's held off.
fn seen_and_held(model: &HomeNetwork, device: &Device, words: &Words) -> Vec<Value> {
    let say = |word| words.say("eero:device", word);
    [
        seen(
            words,
            "eero:device:first-seen",
            device.first_seen.as_deref(),
        ),
        seen(
            words,
            "eero:device:on-since",
            device.first_active.as_deref(),
        ),
        seen(
            words,
            "eero:device:last-active",
            device.last_active.as_deref(),
        ),
        fact(
            words,
            "eero:device:profile",
            &device
                .profile
                .as_deref()
                .and_then(|id| model.profiles.iter().find(|profile| profile.id == id))
                .map_or_else(String::new, |profile| {
                    joined(
                        [
                            profile.name.clone(),
                            (profile.paused == Some(true)).then(|| say("paused").to_owned()),
                        ]
                        .into_iter()
                        .flatten(),
                    )
                }),
        ),
        fact(
            words,
            "eero:device:held",
            &joined(
                [
                    (device.paused == Some(true)).then(|| say("paused").to_owned()),
                    (device.blocked == Some(true)).then(|| say("blocked").to_owned()),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Its address, said reserved where it's the one kept for it, and the kept
/// one as a fact of its own only where they differ, so no figure is said
/// twice (VKCF3Z, 2026-11-08).
fn address(model: &HomeNetwork, device: &Device, words: &Words) -> [Option<Value>; 2] {
    let kept = model
        .internet
        .addresses
        .reservations
        .iter()
        .find(|one| one.mac.eq_ignore_ascii_case(&device.mac))
        .and_then(|one| one.address.as_deref())
        .unwrap_or("");
    let has = device.address.as_deref().unwrap_or("");
    let same = !kept.is_empty() && kept == has;
    [
        fact(
            words,
            "eero:device:address",
            &joined(
                [
                    (!has.is_empty()).then(|| has.to_owned()),
                    same.then(|| words.say("eero:device", "reserved").to_owned()),
                ]
                .into_iter()
                .flatten(),
            ),
        ),
        fact(
            words,
            "eero:device:reserved",
            if kept == has { "" } else { kept },
        ),
    ]
}

/// When it was seen, as the shell words a moment.
fn seen(words: &Words, id: &'static str, at: Option<&str>) -> Option<Value> {
    timed(words, id, at?, Form::Moment, "{time}")
}

/// Where it moved over the last day, newest first, each by when: "Study to
/// Den · 2.4 GHz to 5 GHz", or "Den · 2.4 GHz to 5 GHz" on one node.
fn roams(model: &HomeNetwork, device: &Device, words: &Words) -> Vec<Value> {
    let named = |id: Option<&str>| {
        id.map(|id| {
            model
                .nodes
                .iter()
                .find(|node| node.id == id)
                .map_or_else(|| id.to_owned(), |node| node_name(node).to_string())
        })
    };
    let listed: Vec<Value> = device
        .roams
        .iter()
        .filter_map(|roam| {
            let at = time::to_unix(&roam.at)?;
            let said = joined([
                from_to(
                    named(roam.from_node.as_deref()),
                    named(roam.node.as_deref()),
                    words,
                ),
                from_to(roam.from_band.clone(), roam.band.clone(), words),
            ]);
            Some(json!({"name": said_at::at(at * 1000, Form::Moment), "said": said}))
        })
        .collect();
    under(words, "eero:device:roams", listed)
}

/// Where it came from and went, the one place alone where they're the same.
fn from_to(from: Option<String>, to: Option<String>, words: &Words) -> String {
    match (from.filter(|from| !from.is_empty()), to) {
        (Some(from), Some(to)) if from != to => fill(
            words.say("eero:device", "to"),
            &[("from", &from), ("to", &to)],
        ),
        (Some(from), None) => from,
        (_, to) => to.unwrap_or_default(),
    }
}

/// What eero said and the daemon told of it, newest first, each by when,
/// as prose, the names in it kept whole.
fn events(model: &HomeNetwork, device: &Device, words: &Words) -> Vec<Value> {
    let its = format!(
        "{} ",
        name_of(
            &[&device.name, &device.nickname, &device.hostname],
            &device.mac
        )
    );
    let listed: Vec<Value> = timeline(model)
        .into_iter()
        .filter(|(_, text)| text.starts_with(&its))
        .map(|(at, text)| {
            json!({
                "name": said_at::at(at * 1000, Form::Moment),
                "said": sentence(model, &text),
                "words": true,
            })
        })
        .collect();
    under(words, "eero:device:events", listed)
}

/// The name eero's sentences call a device by: its name, nickname or
/// hostname, else its hardware address.
pub(super) fn name_of<'a>(names: &[&'a Option<String>; 3], mac: &'a str) -> &'a str {
    names
        .iter()
        .filter_map(|name| name.as_deref())
        .find(|name| !name.is_empty())
        .unwrap_or(mac)
}

/// eero's sentence set as the launcher's figures are: a figure run into its
/// unit kept to it, "5.2 GHz", and each device's name of several words
/// kept on one line, the longest first so a name inside another stays part
/// of it, a line free to break after a kept name's last number.
pub(super) fn sentence(model: &HomeNetwork, text: &str) -> String {
    let mut names: Vec<&str> = model
        .devices
        .iter()
        .map(|one| name_of(&[&one.name, &one.nickname, &one.hostname], &one.mac))
        .chain(
            model
                .devices_off
                .iter()
                .map(|one| name_of(&[&one.name, &one.nickname, &one.hostname], &one.mac)),
        )
        .filter(|name| name.contains(' '))
        .collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let said = names.into_iter().fold(run_in(text), |said, name| {
        said.replace(name, &name.replace(' ', KEPT))
    });
    breakable(&said)
}

/// A figure kept to the unit eero ran it into.
fn run_in(text: &str) -> String {
    let mut said = String::with_capacity(text.len());
    for (at, one) in text.char_indices() {
        said.push(one);
        let rest = &text[at + one.len_utf8()..];
        let unit = UNITS.iter().find(|unit| {
            rest.starts_with(*unit)
                && rest[unit.len()..]
                    .chars()
                    .next()
                    .is_none_or(|next| !next.is_alphanumeric() && next != '_')
        });
        if one.is_ascii_digit() && unit.is_some() {
            said.push_str(KEPT);
        }
    }
    said
}

/// A line free to break after a kept name's last number, where a space
/// follows it.
fn breakable(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut said = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        said.push(chars[at]);
        if chars[at] == '\u{a0}' {
            let end = chars[at + 1..]
                .iter()
                .position(|one| !one.is_ascii_digit())
                .map_or(chars.len(), |past| at + 1 + past);
            if end > at + 1 && chars.get(end) == Some(&' ') {
                said.extend(&chars[at + 1..end]);
                said.push_str(BREAK);
                at = end;
                continue;
            }
        }
        at += 1;
    }
    said
}

/// A rate and how the radio makes it: "1.20 Gb/s · MCS 11 · 2 streams ·
/// 80 MHz · 800 ns guard · Wi-Fi 6".
fn rate_of(rate: Option<&Rate>, words: &Words, numbers: &Numbers) -> String {
    let say = |word| words.say("eero:device", word);
    let Some(rate) = rate else {
        return String::new();
    };
    joined(
        [
            rate.mbps.map(|mbps| words.rate(mbps, numbers)),
            rate.mcs.map(|n| fill(say("mcs"), &[("n", &n.to_string())])),
            rate.streams.map(|n| {
                if n == 1 {
                    say("stream").to_owned()
                } else {
                    fill(say("streams"), &[("count", &n.to_string())])
                }
            }),
            rate.width_mhz
                .map(|n| fill(say("mhz"), &[("n", &n.to_string())])),
            rate.guard_ns
                .map(|n| fill(say("guard"), &[("n", &n.to_string())])),
            rate.generation.clone(),
        ]
        .into_iter()
        .flatten(),
    )
}

/// A fact with its figure, where it has one.
fn drawn(fact: Option<Value>, figure: Option<Value>) -> Option<Value> {
    let mut fact = fact?;
    if let Some(figure) = figure {
        fact["figure"] = figure;
    }
    Some(fact)
}

/// A signal as a share of the range the launcher draws.
fn level(dbm: f64) -> f64 {
    ((dbm - FAINTEST) / (LOUDEST - FAINTEST)).clamp(0.0, 1.0)
}

/// Its speed both ways together, a gap only where neither was read.
fn together(down: &[Option<f64>], up: &[Option<f64>]) -> Vec<Option<f64>> {
    down.iter()
        .enumerate()
        .map(|(at, down)| {
            let up = up.get(at).copied().flatten();
            (down.is_some() || up.is_some()).then(|| down.unwrap_or(0.0) + up.unwrap_or(0.0))
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a device's packet counts are far short of 2^52"
)]
fn count_f64(count: u64) -> f64 {
    count as f64
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
