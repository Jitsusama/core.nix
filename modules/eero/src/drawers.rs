//! The drawers of the Home Network's rows, made ready here as the rows are
//! (rows.rs), so the shell only draws them: each a record of its own,
//! `drawer:eero:<kind>:<id>`, served beside the rows and named by its row's
//! `drawer`, so a watch is told one only once it reads differently, and its
//! row isn't made again for it. A profile's drawer comes first; the
//! internet's, a node's and a device's are still worked out in the shell
//! (HomeNetworkDrawers.js) until they're made here too.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::called;
use crate::model::{Filters, HomeNetwork, Profile, Usage};
use crate::place::KIND as DRAWER;
use crate::rows::{Words, fill, node_name};

/// The parts the drawers name, each by its thing in the vocabulary.
pub const NAMES: [&str; 8] = [
    "eero:profile:blocks",
    "eero:profile:enforces",
    "eero:profile:allows",
    "eero:profile:schedule",
    "eero:profile:default",
    "eero:profile:traffic",
    "eero:profile:devices",
    "eero:profile:listed",
];

/// A spark's places, the newest twenty, as every spark the drawers draw.
const SPARKED: usize = 20;

/// The bytes a unit holds of the one before it.
const KILO: f64 = 1000.0;

/// Each unit a count of bytes is said in, by its word in the vocabulary.
const BYTES: [&str; 5] = ["bytes", "kilobytes", "megabytes", "gigabytes", "terabytes"];

/// The key a profile's drawer goes by.
pub fn profile_key(id: &str) -> String {
    format!("drawer:eero:profile:{id}")
}

/// Every drawer the rows name, made as they are.
pub fn records(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    model
        .profiles
        .iter()
        .map(|profile| {
            json!({
                "key": profile_key(&profile.id),
                "kind": DRAWER,
                "drawer": profile_drawer(model, profile, words, numbers),
            })
        })
        .collect()
}

/// A profile's drawer: what it blocks and holds search to, its schedules
/// and its traffic over the last day, sparked by the hour, then how many
/// devices it lists, and each of them, saying which node it's on now or
/// that it isn't on, one not on named as the profile names it. How it
/// stands is its row's side, so the drawer doesn't say it again.
fn profile_drawer(
    model: &HomeNetwork,
    profile: &Profile,
    words: &Words,
    numbers: &Numbers,
) -> Value {
    let say = |word| words.say("eero:profile", word);
    let fact = |id, said: String| (!said.is_empty()).then(|| named(words, id, &said));
    let filters = &profile.filters;
    let on_now = profile
        .devices
        .iter()
        .filter(|mac| model.devices.iter().any(|device| &device.mac == *mac))
        .count();
    let schedules = profile.schedules.map(|count| match count {
        0 => say("noSchedule").to_owned(),
        count => words.counted(count, "eero:profile", "schedule", "schedules"),
    });
    let default = profile
        .default
        .map(|default| say(if default { "yes" } else { "no" }).to_owned());
    let listed = fill(
        say("listed"),
        &[("count", &profile.devices.len().to_string())],
    );
    let on = (on_now > 0).then(|| fill(say("onNow"), &[("count", &on_now.to_string())]));
    let mut facts: Vec<Value> = [
        fact("eero:profile:blocks", blocks(filters, words)),
        fact(
            "eero:profile:enforces",
            joined(
                [
                    filters.safe_search == Some(true),
                    filters.youtube_restricted == Some(true),
                ]
                .into_iter()
                .zip(["safeSearch", "youtubeRestricted"])
                .filter(|(on, _)| *on)
                .map(|(_, word)| say(word).to_owned()),
            ),
        ),
        fact(
            "eero:profile:allows",
            joined(filters.allowed_sites.iter().cloned()),
        ),
        schedules.and_then(|said| fact("eero:profile:schedule", said)),
        default.and_then(|said| fact("eero:profile:default", said)),
        profile
            .usage
            .as_ref()
            .map(|usage| traffic(usage, words, numbers)),
        fact(
            "eero:profile:devices",
            joined([Some(listed), on].into_iter().flatten()),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !profile.devices.is_empty() {
        facts.push(json!({"heading": words.name("eero:profile:listed")}));
    }
    facts.extend(
        profile
            .devices
            .iter()
            .map(|mac| member(model, profile, mac, words)),
    );
    json!({
        "eyebrow": say("eyebrow"),
        "eyebrowNames": true,
        "title": profile.name.as_deref().unwrap_or(&profile.id),
        "facts": facts,
    })
}

/// A part's fact, named as the vocabulary names it.
fn named(words: &Words, id: &'static str, said: &str) -> Value {
    json!({"name": words.name(id), "said": said})
}

/// One device a profile lists: what it's called and the node it's on, or
/// that it isn't on, then by the name the profile gives it, else its
/// hardware address.
fn member(model: &HomeNetwork, profile: &Profile, mac: &str, words: &Words) -> Value {
    let say = |word| words.say("eero:profile", word);
    let on = model.devices.iter().find(|device| device.mac == mac);
    let name = match on {
        Some(device) => called::device(device, words.say("eero:device", "made")).into_owned(),
        None => profile
            .members
            .iter()
            .find(|member| member.mac == mac)
            .and_then(|member| member.name.clone())
            .unwrap_or_else(|| mac.to_owned()),
    };
    let node = on.and_then(|device| {
        model
            .nodes
            .iter()
            .find(|node| device.node.as_deref() == Some(node.id.as_str()))
    });
    let said = node.map_or_else(
        || say("notOnNow").to_owned(),
        |node| fill(say("onNode"), &[("node", node_name(node))]),
    );
    json!({"name": name, "said": said})
}

/// What a profile blocks, as one fact: each kind of content, ads, and each
/// site and app it names, a filter that's off saying nothing; "nothing"
/// where none is on, and no fact where eero said none of its filters.
fn blocks(filters: &Filters, words: &Words) -> String {
    let kinds = [
        ("gaming", filters.gaming),
        ("illegal", filters.illegal),
        ("messaging", filters.messaging),
        ("pornographic", filters.pornographic),
        ("social", filters.social),
        ("shopping", filters.shopping),
        ("streaming", filters.streaming),
        ("violent", filters.violent),
    ];
    if kinds.iter().all(|(_, on)| on.is_none()) {
        return String::new();
    }
    let say = |word| words.say("eero:profile", word).to_owned();
    let ads = if filters.ad_block == Some(true) {
        Some(say("ads"))
    } else if filters.ad_block_network == Some(true) {
        Some(say("adsNetwork"))
    } else {
        None
    };
    let blocked = joined(
        kinds
            .into_iter()
            .filter(|(_, on)| *on == Some(true))
            .map(|(kind, _)| say(kind))
            .chain(ads)
            .chain(filters.blocked_sites.iter().cloned())
            .chain(filters.blocked_apps.iter().cloned()),
    );
    if blocked.is_empty() {
        say("blocksNothing")
    } else {
        blocked
    }
}

/// The day's traffic, "150 MB down · 3.10 MB up", sparked by the hour.
fn traffic(usage: &Usage, words: &Words, numbers: &Numbers) -> Value {
    let way = |bytes, word| {
        fill(
            words.say("eero", word),
            &[("rate", &self::bytes(bytes, words, numbers))],
        )
    };
    let said = joined([
        way(usage.download.total_bytes, "down"),
        way(usage.upload.total_bytes, "up"),
    ]);
    let mut fact = named(words, "eero:profile:traffic", &said);
    let hours: Vec<f64> = usage
        .download
        .hours
        .iter()
        .enumerate()
        .map(|(at, hour)| {
            let up = usage.upload.hours.get(at).map_or(0, |hour| hour.bytes);
            to_f64(hour.bytes + up)
        })
        .collect();
    if let Some(spark) = spark(&hours) {
        fact["figure"] = spark;
    }
    fact
}

/// Readings as Graph's spark: the newest twenty, scaled to their most;
/// none for fewer than two.
fn spark(values: &[f64]) -> Option<Value> {
    let recent = &values[values.len().saturating_sub(SPARKED)..];
    if recent.len() < 2 {
        return None;
    }
    let most = recent.iter().copied().fold(0.0, f64::max);
    let drawn: Vec<f64> = recent
        .iter()
        .map(|value| if most > 0.0 { value / most } else { 0.0 })
        .collect();
    Some(json!({"shape": "spark", "values": drawn, "warn": false}))
}

/// A count of bytes in whichever unit says it in three digits: "0 B",
/// "53.7 kB", "2.40 MB".
fn bytes(count: u64, words: &Words, numbers: &Numbers) -> String {
    let mut scaled: f64 = format!("{:.2e}", to_f64(count)).parse().unwrap_or(0.0);
    let mut unit = 0;
    while scaled >= KILO && unit < BYTES.len() - 1 {
        scaled /= KILO;
        unit += 1;
    }
    let figure = if unit == 0 {
        numbers.number(scaled, 0)
    } else {
        numbers.significant(scaled)
    };
    fill(words.say("eero", BYTES[unit]), &[("n", &figure)])
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a day's bytes are far short of 2^52"
)]
fn to_f64(count: u64) -> f64 {
    count as f64
}

/// Parts said together, a dot between each, those saying nothing left out.
fn joined(parts: impl IntoIterator<Item = String>) -> String {
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

#[cfg(test)]
#[path = "drawers_tests.rs"]
mod tests;
