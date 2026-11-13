//! A node's drawer, made ready here as its row is, every fact as the step
//! into it lists them (its drawer holds a glance of six, drawers.rs's
//! GLANCE): when it was last heard from, how it reaches the one before it
//! and how its devices are on, first; then
//! which node it is, each radio and port, its firmware, how long it's been
//! up, its addresses, its light and the day's traffic through it; then the
//! devices on it, those that were and aren't now, and who joined or left
//! it over the last day. Its state is said only where it isn't online,
//! which its row says too. Every time travels as the shell words it, so the
//! drawer reads the same from one minute to the next (ready-rows' time.rs).

use ready_rows::numbers::Numbers;
use ready_rows::time::{self as said_at, Form};
use serde_json::{Value, json};

use super::{DRAWER, bytes, joined, named, spark_of, traffic};
use crate::called;
use crate::model::{BackhaulKind, HomeNetwork, JoinOrLeaveKind, Node, Port, Radio};
use crate::rows::{Words, backhaul_said, fill, node_name};
use crate::time;

/// The parts a node's drawer names, each by its thing in the vocabulary.
pub const NAMES: [&str; 27] = [
    "eero:node:state",
    "eero:node:heard",
    "eero:node:backhaul",
    "eero:node:devices",
    "eero:node:model",
    "eero:node:serial",
    "eero:node:radios",
    "eero:node:bssids",
    "eero:node:busy",
    "eero:node:radio-power",
    "eero:node:ports",
    "eero:node:port",
    "eero:node:about",
    "eero:node:firmware",
    "eero:node:support",
    "eero:node:up",
    "eero:node:cloud",
    "eero:node:added",
    "eero:node:address",
    "eero:node:ipv6",
    "eero:node:mac",
    "eero:node:light",
    "eero:node:power",
    "eero:node:traffic",
    "eero:node:on-it",
    "eero:node:devices-off",
    "eero:node:joins-and-leaves",
];

/// A history's places: each sample its own while twenty hold them all,
/// else gathered into twenty runs, as every spark the place draws.
const PLACES: usize = 20;

/// The bars a backhaul has at most.
const BARS: f64 = 5.0;

/// All of a radio's time.
const PERCENT: f64 = 100.0;

/// A name's words kept on one line.
pub(super) const KEPT: &str = "\u{a0}";

/// Where a line may break after a kept name's last number.
pub(super) const BREAK: &str = "\u{200b}";

/// The key a node's drawer goes by.
pub fn key(id: &str) -> String {
    format!("drawer:eero:node:{id}")
}

/// Every node's drawer, as a record beside the rows.
pub fn records(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    model
        .nodes
        .iter()
        .map(|node| {
            json!({
                "key": key(&node.id),
                "kind": DRAWER,
                "drawer": drawer(model, node, words, numbers),
            })
        })
        .collect()
}

/// A node's drawer. Its eyebrow is the network it's on, then its kind,
/// "on home · node", a name from the world kept as it's written.
fn drawer(model: &HomeNetwork, node: &Node, words: &Words, numbers: &Numbers) -> Value {
    let say = |word| words.say("eero:node", word);
    let kind = if node.gateway {
        say("gateway")
    } else {
        say("node")
    };
    let on = (!model.name.is_empty()).then(|| fill(say("onNetwork"), &[("name", &model.name)]));
    let read_ms = time::to_unix(&model.read).map(|seconds| seconds * 1000);
    let mut facts: Vec<Value> = [
        state(node, words),
        heard(node, words),
        backhaul(node, words),
        fact(words, "eero:node:devices", &devices(node, words)),
        fact(
            words,
            "eero:node:model",
            &joined(
                [&node.model, &node.model_number]
                    .into_iter()
                    .flatten()
                    .cloned(),
            ),
        ),
        fact(
            words,
            "eero:node:serial",
            node.serial.as_deref().unwrap_or(""),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    facts.extend(radios(node, words, numbers));
    facts.extend(ports(&node.ports, words, numbers));
    facts.push(heading(words, "eero:node:about"));
    facts.extend(
        [
            fact(words, "eero:node:firmware", &firmware(node, words)),
            node.support_ends
                .as_deref()
                .and_then(|at| timed(words, "eero:node:support", at, Form::Day, "{time}")),
            read_ms
                .zip(node.up_s)
                .map(|(read, up)| lasted(words, "eero:node:up", read - seconds_ms(up))),
            read_ms
                .zip(node.cloud_s)
                .map(|(read, cloud)| lasted(words, "eero:node:cloud", read - seconds_ms(cloud))),
            node.joined
                .as_deref()
                .and_then(|at| timed(words, "eero:node:added", at, Form::Day, "{time}")),
            fact(
                words,
                "eero:node:address",
                node.address.as_deref().unwrap_or(""),
            ),
            fact(words, "eero:node:ipv6", &joined(node.ipv6.iter().cloned())),
            fact(words, "eero:node:mac", node.mac.as_deref().unwrap_or("")),
            node.light.as_ref().and_then(|light| {
                let on = light
                    .on
                    .map(|on| say(if on { "on" } else { "off" }).to_owned());
                let bright = light
                    .brightness
                    .map(|n| fill(say("percent"), &[("n", &n.to_string())]));
                fact(
                    words,
                    "eero:node:light",
                    &joined([on, bright].into_iter().flatten()),
                )
            }),
            fact(
                words,
                "eero:node:power",
                node.power.as_deref().unwrap_or(""),
            ),
            node.usage
                .as_ref()
                .map(|usage| traffic(usage, "eero:node:traffic", words, numbers)),
        ]
        .into_iter()
        .flatten(),
    );
    facts.extend(on_it(model, node, words, numbers));
    facts.extend(devices_off(model, node, words));
    facts.extend(joins_and_leaves(model, node, words));
    json!({
        "eyebrow": joined(on.into_iter().chain([kind.to_owned()])),
        "eyebrowNames": true,
        "title": node_name(node),
        "facts": facts,
    })
}

/// A fact, or none where it says nothing.
pub(super) fn fact(words: &Words, id: &'static str, said: &str) -> Option<Value> {
    (!said.is_empty()).then(|| named(words, id, said))
}

pub(super) fn heading(words: &Words, id: &'static str) -> Value {
    json!({"heading": words.name(id)})
}

/// A time eero gives, as a fact the shell words in `form`, put in `said`'s
/// `{time}`; none where it doesn't read as a time.
pub(super) fn timed(
    words: &Words,
    id: &'static str,
    at: &str,
    form: Form,
    said: &str,
) -> Option<Value> {
    let ms = time::to_unix(at)? * 1000;
    let mut fact = said_at::said(ms, form, said);
    fact["name"] = Value::from(words.name(id));
    Some(fact)
}

/// How long since `since_ms`, as the shell says it at the moment it draws.
fn lasted(words: &Words, id: &'static str, since_ms: i64) -> Value {
    let mut fact = said_at::at(since_ms, Form::Lasted);
    fact["name"] = Value::from(words.name(id));
    fact
}

fn seconds_ms(seconds: u64) -> i64 {
    i64::try_from(seconds).unwrap_or(i64::MAX / 1000) * 1000
}

/// Its state, only where it isn't online, which its row says.
fn state(node: &Node, words: &Words) -> Option<Value> {
    let state = node.state.as_deref().unwrap_or("").to_lowercase();
    if state == "online" {
        return None;
    }
    fact(words, "eero:node:state", &state)
}

/// When it was last heard from, "2 h 15 min ago", and late where eero says
/// so; never, where eero heard none and says it's late.
fn heard(node: &Node, words: &Words) -> Option<Value> {
    let say = |word| words.say("eero:node", word);
    let late = node.heartbeat_ok == Some(false);
    let Some(at) = node.last_heartbeat.as_deref() else {
        return late.then(|| named(words, "eero:node:heard", say("never")));
    };
    let ago = say("ago");
    let said = if late {
        joined([ago.to_owned(), say("late").to_owned()])
    } else {
        ago.to_owned()
    };
    let ms = time::to_unix(at)? * 1000;
    let mut fact = said_at::said(ms, Form::Ago, &said);
    fact["name"] = Value::from(words.name("eero:node:heard"));
    Some(fact)
}

/// How it reaches the one before it, with its bars over Wi-Fi sparked over
/// the last hour, or a stroke of now where none is kept; Ethernet has none.
fn backhaul(node: &Node, words: &Words) -> Option<Value> {
    let mut fact = fact(words, "eero:node:backhaul", &backhaul_said(node, words))?;
    let level = (node.backhaul.kind != BackhaulKind::Ethernet)
        .then_some(node.backhaul.bars)
        .flatten()
        .map(|bars| f64::from(bars) / BARS);
    if let Some(level) = level {
        let history: Vec<Option<f64>> = node
            .history
            .bars
            .iter()
            .map(|bars| bars.map(|bars| f64::from(bars) / BARS))
            .collect();
        fact["figure"] = history_spark(&history, Some(1.0)).unwrap_or_else(|| stroke(level));
    }
    Some(fact)
}

/// How its devices are on, a part that's none giving way: "all on Wi-Fi",
/// or "6 on Wi-Fi · 3 on Ethernet". How many is its row's side.
fn devices(node: &Node, words: &Words) -> String {
    let say = |word| words.say("eero:node", word);
    let counts = &node.devices;
    let Some(all) = counts.all.filter(|all| *all > 0) else {
        return String::new();
    };
    let wifi = counts.wifi.unwrap_or(0);
    let ethernet = counts.ethernet.unwrap_or(0);
    if wifi == all {
        return say("allWifi").to_owned();
    }
    if ethernet == all {
        return say("allEthernet").to_owned();
    }
    let each =
        |count: u32, word| (count > 0).then(|| fill(say(word), &[("count", &count.to_string())]));
    joined(
        [each(wifi, "onWifi"), each(ethernet, "onEthernet")]
            .into_iter()
            .flatten(),
    )
}

/// Its firmware, and whether it has an update.
fn firmware(node: &Node, words: &Words) -> String {
    let update = node.update_available.map(|available| {
        words
            .say(
                "eero:node",
                if available {
                    "updateAvailable"
                } else {
                    "upToDate"
                },
            )
            .to_owned()
    });
    joined(node.firmware.clone().into_iter().chain(update))
}

/// Each radio by band: its channel, width and devices; then how busy each
/// is and how strongly it sends, each beside its last hour; then each
/// radio's hardware address.
fn radios(node: &Node, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:node", word);
    let mut list: Vec<&Radio> = node.radios.iter().collect();
    list.sort_by_key(|radio| radio.band.to_lowercase());
    if list.is_empty() {
        return Vec::new();
    }
    let by_band = |radio: &Radio, said: String| json!({"name": radio.band, "said": said});
    let mut facts = vec![heading(words, "eero:node:radios")];
    facts.extend(list.iter().map(|radio| {
        by_band(
            radio,
            joined(
                [
                    radio.channel.map(|n| {
                        fill(
                            words.say("eero:device", "channel"),
                            &[("n", &n.to_string())],
                        )
                    }),
                    radio
                        .width_mhz
                        .map(|n| fill(say("mhz"), &[("n", &n.to_string())])),
                    radio
                        .devices
                        .map(|n| words.devices(usize::try_from(n).unwrap_or(usize::MAX))),
                ]
                .into_iter()
                .flatten(),
            ),
        )
    }));
    let busy: Vec<Value> = list
        .iter()
        .filter_map(|radio| {
            let pct = radio.busy_pct?;
            let mut fact = by_band(radio, fill(say("percent"), &[("n", &pct.to_string())]));
            let history: Vec<Option<f64>> = radio
                .history
                .busy_pct
                .iter()
                .map(|pct| pct.map(|pct| f64::from(pct) / PERCENT))
                .collect();
            fact["figure"] = history_spark(&history, Some(1.0))
                .unwrap_or_else(|| stroke(f64::from(pct) / PERCENT));
            Some(fact)
        })
        .collect();
    let power: Vec<Value> = list
        .iter()
        .filter_map(|radio| {
            let dbm = radio.power_dbm?;
            let mut fact = by_band(
                radio,
                fill(
                    words.say("eero:device", "dbm"),
                    &[("n", &numbers.whole(f64::from(dbm)))],
                ),
            );
            let history: Vec<Option<f64>> = radio
                .history
                .power_dbm
                .iter()
                .map(|dbm| dbm.map(f64::from))
                .collect();
            if let Some(spark) = history_spark(&history, None) {
                fact["figure"] = spark;
            }
            Some(fact)
        })
        .collect();
    let addresses: Vec<Value> = list
        .iter()
        .filter_map(|radio| Some(by_band(radio, radio.bssid.clone()?)))
        .collect();
    for (id, part) in [
        ("eero:node:busy", busy),
        ("eero:node:radio-power", power),
        ("eero:node:bssids", addresses),
    ] {
        if !part.is_empty() {
            facts.push(heading(words, id));
            facts.extend(part);
        }
    }
    facts
}

/// Each port: linked at what speed of its best, whether it's the
/// internet's or the backhaul's, and what's gone through it each way.
fn ports(ports: &[Port], words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:node", word);
    if ports.is_empty() {
        return Vec::new();
    }
    let mut facts = vec![heading(words, "eero:node:ports")];
    facts.extend(ports.iter().map(|port| {
        let through = |count: Option<u64>, word| {
            count.map(|count| fill(say(word), &[("bytes", &bytes(count, words, numbers))]))
        };
        let said = joined(
            [
                Some(link(port, words, numbers)),
                port.wan.then(|| say("internet").to_owned()),
                port.upstream.then(|| say("backhaul").to_owned()),
                through(port.rx_bytes, "in"),
                through(port.tx_bytes, "out"),
            ]
            .into_iter()
            .flatten(),
        );
        json!({
            "name": fill(words.name("eero:node:port"), &[("n", &port.name)]),
            "said": said,
        })
    }));
    facts
}

/// How a port is linked: "1.00 Gb/s of 1.00 Gb/s", or "not linked · up to
/// 1.00 Gb/s".
fn link(port: &Port, words: &Words, numbers: &Numbers) -> String {
    let say = |word| words.say("eero:node", word);
    let speed = |mbps: Option<u32>| mbps.map(|mbps| words.rate(f64::from(mbps), numbers));
    let best = speed(port.best_mbps);
    if !port.linked {
        return joined(
            [
                Some(say("notLinked").to_owned()),
                best.map(|best| fill(say("upTo"), &[("rate", &best)])),
            ]
            .into_iter()
            .flatten(),
        );
    }
    let now = speed(port.mbps).unwrap_or_default();
    match best {
        Some(best) if !now.is_empty() => fill(say("of"), &[("rate", &now), ("best", &best)]),
        _ => now,
    }
}

/// The devices on it, as eero lists them, strongest first: each by name,
/// with its signal, band and channel, or its port over Ethernet.
fn on_it(model: &HomeNetwork, node: &Node, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let listed: Vec<Value> = model
        .devices
        .iter()
        .filter(|device| device.node.as_deref() == Some(node.id.as_str()))
        .map(|device| {
            let said = if device.ethernet {
                let port = device.port.as_ref();
                joined(
                    [
                        Some(words.say("eero:node", "ethernet").to_owned()),
                        port.and_then(|port| port.name.clone()),
                        port.and_then(|port| port.mbps)
                            .map(|mbps| words.rate(f64::from(mbps), numbers)),
                    ]
                    .into_iter()
                    .flatten(),
                )
            } else {
                joined(
                    [
                        device.signal_dbm.map(|dbm| {
                            fill(
                                words.say("eero:device", "dbm"),
                                &[("n", &numbers.whole(f64::from(dbm)))],
                            )
                        }),
                        device.band.clone(),
                        device.channel.map(|n| {
                            fill(
                                words.say("eero:device", "channel"),
                                &[("n", &n.to_string())],
                            )
                        }),
                    ]
                    .into_iter()
                    .flatten(),
                )
            };
            let name = called::device(device, words.say("eero:device", "made"));
            json!({"name": name, "said": said})
        })
        .collect();
    under(words, "eero:node:on-it", listed)
}

/// The devices eero knows that aren't on now and were last on it, the
/// last on first, each by name, with the band it was last on and when:
/// "5 GHz · 21:38".
fn devices_off(model: &HomeNetwork, node: &Node, words: &Words) -> Vec<Value> {
    let listed: Vec<Value> = model
        .devices_off
        .iter()
        .filter(|device| device.node.as_deref() == Some(node.id.as_str()))
        .map(|device| {
            let how = if device.ethernet {
                Some(words.say("eero:node", "ethernet").to_owned())
            } else {
                device.band.clone()
            };
            let name = called::off(device, words.say("eero:device", "made"));
            let at = device.last_active.as_deref().and_then(time::to_unix);
            match at {
                Some(seconds) => {
                    let said = joined(how.into_iter().chain(["{time}".to_owned()]));
                    let mut fact = said_at::said(seconds * 1000, Form::Moment, &said);
                    fact["name"] = Value::from(name.into_owned());
                    fact
                }
                None => json!({"name": name, "said": how.unwrap_or_default()}),
            }
        })
        .collect();
    under(words, "eero:node:devices-off", listed)
}

/// Who joined or left it over the last day, newest first, each by when,
/// "phone 1 left · 5 GHz", the name kept on one line.
fn joins_and_leaves(model: &HomeNetwork, node: &Node, words: &Words) -> Vec<Value> {
    let mut list: Vec<_> = model
        .joins_and_leaves
        .iter()
        .filter(|one| one.node.as_deref() == Some(node.id.as_str()))
        .filter_map(|one| Some((time::to_unix(&one.at)?, one)))
        .collect();
    list.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    let listed: Vec<Value> = list
        .into_iter()
        .map(|(at, one)| {
            let did = match one.kind {
                JoinOrLeaveKind::Joined => "joined",
                JoinOrLeaveKind::Left => "left",
            };
            let name = one.name.as_deref().unwrap_or(&one.mac);
            let kept = format!("{}{BREAK}", name.replace(' ', KEPT));
            let said = joined(
                [
                    Some(fill(words.say("eero:node", did), &[("name", &kept)])),
                    one.band.clone(),
                ]
                .into_iter()
                .flatten(),
            );
            json!({"name": said_at::at(at * 1000, Form::Moment), "said": said})
        })
        .collect();
    under(words, "eero:node:joins-and-leaves", listed)
}

/// A list under its heading, or nothing where it holds none.
pub(super) fn under(words: &Words, id: &'static str, listed: Vec<Value>) -> Vec<Value> {
    if listed.is_empty() {
        return listed;
    }
    std::iter::once(heading(words, id)).chain(listed).collect()
}

/// A share as Meter's stroke.
pub(super) fn stroke(share: f64) -> Value {
    json!({"shape": "stroke", "values": [share.clamp(0.0, 1.0)], "warn": false})
}

/// A history as Graph's spark: a gap where eero gave nothing, each place a
/// share of `most`, or of the history's own most where none is given;
/// none until two places hold a figure. Past twenty samples, each place
/// is the mean of a run of them, so a spark spans the whole hour kept.
pub(super) fn history_spark(values: &[Option<f64>], most: Option<f64>) -> Option<Value> {
    let places = fitted(values);
    let read: Vec<f64> = places.iter().flatten().copied().collect();
    if read.len() < 2 {
        return None;
    }
    let top = most.unwrap_or_else(|| read.iter().copied().fold(f64::MIN, f64::max));
    let drawn: Vec<Value> = places
        .iter()
        .map(|place| match place {
            Some(value) => Value::from(if top > 0.0 { value / top } else { 0.0 }),
            None => Value::Null,
        })
        .collect();
    Some(spark_of(&drawn))
}

fn fitted(values: &[Option<f64>]) -> Vec<Option<f64>> {
    if values.len() <= PLACES {
        return values.to_vec();
    }
    (0..PLACES)
        .map(|at| {
            let run: Vec<f64> = values
                [at * values.len() / PLACES..(at + 1) * values.len() / PLACES]
                .iter()
                .flatten()
                .copied()
                .collect();
            (!run.is_empty()).then(|| run.iter().sum::<f64>() / count_f64(run.len()))
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a run holds far fewer than 2^52 samples"
)]
fn count_f64(count: usize) -> f64 {
    count as f64
}

#[cfg(test)]
#[path = "node_tests.rs"]
pub(super) mod tests;
