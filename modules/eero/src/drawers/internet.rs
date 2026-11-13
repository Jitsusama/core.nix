//! The internet's drawer, the one its row in the Home Network shows, made
//! ready here as every other row's is: first the last speed test, the
//! nodes and outages, who carries it, the firmware and the day's traffic,
//! so a narrow drawer shows those before its more; then this machine's
//! last looks at it, each outage, every speed test, how its addresses are
//! had, its switches and what happened to it, each under a heading. It
//! holds only the internet: each device, its address and what happened to
//! it are in its own drawer, and who came and went in its node's (Joel,
//! 2026-11-11: "home network internet seems to have devices connected to
//! the LAN thrown in"). How it stands is its row's side, so the drawer
//! doesn't say it again. Its eyebrow is "via eero · internet", as the
//! place's is "via eero · network", since who carries it is a fact of its
//! own (VKCF3Z, 2026-11-12).

use ready_rows::numbers::Numbers;
use ready_rows::time::{self as said_at, Form};
use serde_json::{Value, json};

use super::device::{name_of, sentence};
use super::node::{fact, heading, history_spark, timed, under};
use super::{DRAWER, joined, named, spark, timeline, traffic};
use crate::model::{CloudState, HomeNetwork, Internet, OutageSeenBy, SpeedTest, Subnet};
use crate::rows::{Words, fill, nodes_said, outages_said};
use crate::time;

/// The key the internet's drawer goes by.
pub const KEY: &str = "drawer:eero:internet";

/// The parts the drawer names, each by its thing in the vocabulary.
pub const NAMES: [&str; 47] = [
    "eero:internet:speed-test",
    "eero:internet:nodes",
    "eero:internet:outages",
    "eero:internet:isp",
    "eero:internet:mesh",
    "eero:internet:firmware",
    "eero:internet:updates",
    "eero:internet:traffic",
    "eero:internet:read",
    "eero:internet:probe",
    "eero:internet:looked",
    "eero:internet:to-gateway",
    "eero:internet:to-internet",
    "eero:internet:lookup",
    "eero:internet:outage-list",
    "eero:internet:speed-tests",
    "eero:internet:down",
    "eero:internet:up",
    "eero:internet:kept",
    "eero:internet:addressing",
    "eero:internet:public",
    "eero:internet:lease",
    "eero:internet:gateway",
    "eero:internet:mode",
    "eero:internet:dns",
    "eero:internet:ipv6",
    "eero:internet:main-subnet",
    "eero:internet:guest-subnet",
    "eero:internet:forwards",
    "eero:internet:pinholes",
    "eero:internet:settings",
    "eero:internet:sqm",
    "eero:internet:upnp",
    "eero:internet:randomization",
    "eero:internet:steering",
    "eero:internet:wpa3",
    "eero:internet:security",
    "eero:internet:wifi-mode",
    "eero:internet:ac-compatibility",
    "eero:internet:name-clash",
    "eero:internet:mlo",
    "eero:internet:thread",
    "eero:internet:guest",
    "eero:internet:ddns",
    "eero:internet:backup",
    "eero:internet:power-saving",
    "eero:internet:events",
];

const SECOND: i64 = 1000;
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// The internet's drawer, as a record beside the rows.
pub fn record(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Value {
    json!({"key": KEY, "kind": DRAWER, "drawer": drawer(model, words, numbers)})
}

fn drawer(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Value {
    let mut facts = head(model, words, numbers);
    facts.extend(probe(&model.internet, words, numbers));
    facts.extend(outages(&model.internet, words));
    facts.extend(speed_tests(&model.internet, words, numbers));
    facts.extend(addressing(&model.internet, words));
    facts.extend(switches(&model.internet, words));
    facts.extend(events(model, words));
    json!({
        "eyebrow": words.say("eero:internet", "eyebrow"),
        "eyebrowNames": true,
        "title": words.internet,
        "facts": facts,
    })
}

/// What its row says, and what a narrow drawer has room for before its
/// more: the last speed test and its day, the nodes and outages, who
/// carries it, the nodes' network, the firmware and its updates, the day's
/// traffic and when eero was last read.
fn head(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let net = &model.internet;
    let test = &net.speed_test;
    let tested = words.each_way(test.down_mbps, test.up_mbps, numbers);
    let asn = net
        .asn
        .map(|asn| fill(say("asn"), &[("asn", &asn.to_string())]));
    let down = (net.isp_up == Some(false)).then(|| say("ispDown").to_owned());
    let cloud = match model.cloud.state {
        CloudState::Answering => say("answering"),
        CloudState::Unreachable => say("unreachable"),
        CloudState::SignedOut => say("signedOut"),
    };
    let read = joined(["{time}".to_owned(), cloud.to_owned()]);
    let traffic = net
        .usage
        .as_ref()
        .map(|usage| traffic(usage, "eero:internet:traffic", words, numbers));
    [
        test.at
            .as_deref()
            .and_then(|at| {
                timed(
                    words,
                    "eero:internet:speed-test",
                    at,
                    Form::Moment,
                    &joined([tested.clone(), "{time}".to_owned()]),
                )
            })
            .or_else(|| fact(words, "eero:internet:speed-test", &tested)),
        fact(words, "eero:internet:nodes", &nodes_said(model, words)),
        fact(words, "eero:internet:outages", &outages_said(model, words)),
        fact(
            words,
            "eero:internet:isp",
            &joined(
                [net.isp.clone().unwrap_or_default()]
                    .into_iter()
                    .chain(asn)
                    .chain(down),
            ),
        ),
        fact(
            words,
            "eero:internet:mesh",
            net.mesh.as_deref().unwrap_or(""),
        ),
    ]
    .into_iter()
    .chain(firmware(net, words))
    .chain([
        traffic,
        timed(
            words,
            "eero:internet:read",
            &model.read,
            Form::Moment,
            &read,
        ),
    ])
    .flatten()
    .collect()
}

/// The firmware the nodes run and whether an update waits, then the hour
/// eero updates them at and when it last began to.
fn firmware(net: &Internet, words: &Words) -> [Option<Value>; 2] {
    let say = |word| words.say("eero:internet", word);
    let firmware = &net.firmware;
    let update = match (firmware.has_update, firmware.next.as_deref()) {
        (Some(true), Some(version)) => fill(say("updateReady"), &[("version", version)]),
        (Some(true), None) => say("anUpdate").to_owned(),
        _ => say("upToDate").to_owned(),
    };
    let hour = firmware
        .update_hour
        .map(|hour| fill(say("updateAt"), &[("hour", &format!("{hour:02}"))]))
        .unwrap_or_default();
    let updates = firmware
        .last_started
        .as_deref()
        .and_then(|at| {
            timed(
                words,
                "eero:internet:updates",
                at,
                Form::Moment,
                &joined([hour.clone(), say("lastStarted").to_owned()]),
            )
        })
        .or_else(|| fact(words, "eero:internet:updates", &hour));
    [
        fact(
            words,
            "eero:internet:firmware",
            &joined([firmware.running.clone().unwrap_or_default(), update]),
        ),
        updates,
    ]
}

/// This machine's last look at the internet, each target's time beside a
/// spark of the last hour's looks, a gap where one went unanswered: the
/// gateway node, timed only from home, the quicker of two public
/// resolvers, and the gateway node looking a name up.
fn probe(net: &Internet, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let Some(last) = net.probe.as_ref() else {
        return Vec::new();
    };
    let looks = &net.history.probes;
    let looked = joined([
        "{time}".to_owned(),
        say(if last.home {
            "fromHome"
        } else {
            "awayFromHome"
        })
        .to_owned(),
        if last.reachable {
            String::new()
        } else {
            say("notReached").to_owned()
        },
    ]);
    let timed_to = |id, ms: Option<f64>, history: &[Option<f64>]| {
        let said = ms.map(|ms| fill(say("ms"), &[("n", &numbers.significant(ms))]))?;
        let mut drawn = named(words, id, &said);
        if let Some(spark) = history_spark(history, None) {
            drawn["figure"] = spark;
        }
        Some(drawn)
    };
    let mut facts = vec![heading(words, "eero:internet:probe")];
    facts.extend(
        [
            timed(
                words,
                "eero:internet:looked",
                &last.at,
                Form::Moment,
                &looked,
            ),
            timed_to(
                "eero:internet:to-gateway",
                last.gateway_ms,
                &looks.gateway_ms,
            ),
            timed_to(
                "eero:internet:to-internet",
                last.internet_ms,
                &looks.internet_ms,
            ),
            timed_to("eero:internet:lookup", last.dns_ms, &looks.dns_ms),
        ]
        .into_iter()
        .flatten(),
    );
    facts
}

/// Each outage of the last 30 days, newest first as the history keeps
/// them, by when it began, how long it lasted or has so far, and who saw
/// it: eero, or this machine's own looks. eero's record gives no start in
/// a shape the record reads yet, so one of eero's says its time isn't
/// given.
fn outages(net: &Internet, words: &Words) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let listed = net
        .history
        .outages
        .iter()
        .map(|outage| {
            let seen = say(match outage.seen_by {
                OutageSeenBy::Probe => "byProbe",
                OutageSeenBy::Eero => "byEero",
            })
            .to_owned();
            let start = outage.start.as_deref().and_then(time::to_unix);
            let end = outage.end.as_deref().and_then(time::to_unix);
            let Some(start) = start else {
                return json!({"name": say("timeNotGiven"), "said": seen});
            };
            let mut drawn = match end {
                Some(end) => json!({"said": joined([lasted((end - start) * SECOND, words), seen])}),
                None => said_at::said(
                    start * SECOND,
                    Form::Lasted,
                    &joined([say("soFar").to_owned(), seen]),
                ),
            };
            drawn["name"] = said_at::at(start * SECOND, Form::Moment);
            drawn
        })
        .collect();
    under(words, "eero:internet:outage-list", listed)
}

/// How long an outage that ended lasted, the largest two of days, hours
/// and minutes, the smaller left out where it's none, as the shell says
/// how long something has lasted so far.
fn lasted(ms: i64, words: &Words) -> String {
    let say = |word| words.say("eero:internet", word);
    let seconds = ms.max(0) / SECOND;
    let (days, hours, minutes) = (seconds / DAY, seconds % DAY / HOUR, seconds % HOUR / MINUTE);
    let each = |word, n: i64| fill(say(word), &[("n", &n.to_string())]);
    let (larger, smaller) = match (days, hours, minutes) {
        (0, 0, 0) => return say("underMinute").to_owned(),
        (0, 0, _) => (each("minutes", minutes), String::new()),
        (0, _, 0) => (each("hours", hours), String::new()),
        (0, _, _) => (each("hours", hours), each("minutes", minutes)),
        (_, 0, _) => (each("days", days), String::new()),
        _ => (each("days", days), each("hours", hours)),
    };
    [larger, smaller]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every speed test kept, a spark of each way with the last's figures
/// beside, and how many are kept since when; nothing where one alone is,
/// since the head says it.
fn speed_tests(net: &Internet, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let tests: Vec<_> = net.speed_tests.iter().rev().collect();
    if tests.len() < 2 {
        return Vec::new();
    }
    let sparked = |id, mbps: Option<f64>, each: fn(&SpeedTest) -> Option<f64>| {
        let mut drawn = fact(
            words,
            id,
            &mbps
                .map(|mbps| words.rate(mbps, numbers))
                .unwrap_or_default(),
        )?;
        let values: Vec<f64> = tests.iter().filter_map(|test| each(test)).collect();
        if let Some(spark) = spark(&values) {
            drawn["figure"] = spark;
        }
        Some(drawn)
    };
    let counted = words.counted(tests.len(), "eero:internet", "speedTest", "speedTests");
    let since = fill(say("since"), &[("tests", &counted)]);
    let kept = tests
        .first()
        .and_then(|oldest| oldest.at.as_deref())
        .and_then(|at| timed(words, "eero:internet:kept", at, Form::Moment, &since))
        .or_else(|| fact(words, "eero:internet:kept", &counted));
    let test = &net.speed_test;
    let mut facts = vec![heading(words, "eero:internet:speed-tests")];
    facts.extend(
        [
            sparked("eero:internet:down", test.down_mbps, |test| test.down_mbps),
            sparked("eero:internet:up", test.up_mbps, |test| test.up_mbps),
            kept,
        ]
        .into_iter()
        .flatten(),
    );
    facts
}

/// How the internet's address is had and how the home network hands its
/// own out: the public address, the lease, the gateway node, the mode and
/// the resolvers, then each subnet and the range DHCP hands out, and
/// what's let in from outside.
fn addressing(net: &Internet, words: &Words) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let owned = |value: &Option<String>| value.clone().unwrap_or_default();
    let lease = &net.lease;
    let dns = &net.dns;
    let public = joined([
        owned(&net.public_address),
        if net.double_nat == Some(true) {
            say("behindNat").to_owned()
        } else {
            String::new()
        },
    ]);
    let router = lease
        .router
        .as_deref()
        .map(|router| fill(say("router"), &[("address", router)]));
    let mode = joined([
        owned(&net.mode).to_uppercase(),
        net.dhcp
            .as_deref()
            .map(|dhcp| fill(say("addresses"), &[("dhcp", dhcp)]))
            .unwrap_or_default(),
    ]);
    let servers = dns.servers.join(" \u{b7} ");
    let caching = dns
        .caching
        .map(|on| fill(say("caching"), &[("state", on_off(Some(on), words))]));
    let ipv6 = dns
        .ipv6_mode
        .as_deref()
        .map(|mode| fill(say("ipv6"), &[("mode", mode)]));
    let mut facts = vec![heading(words, "eero:internet:addressing")];
    facts.extend(
        [
            fact(words, "eero:internet:public", &public),
            fact(
                words,
                "eero:internet:lease",
                &joined(
                    [
                        owned(&net.wan_type),
                        with_prefix(lease.address.as_deref(), lease.mask.as_deref(), words),
                    ]
                    .into_iter()
                    .chain(router),
                ),
            ),
            fact(words, "eero:internet:gateway", &owned(&net.gateway_address)),
            fact(words, "eero:internet:mode", &mode),
            fact(
                words,
                "eero:internet:dns",
                &joined(
                    [owned(&dns.mode), servers]
                        .into_iter()
                        .chain(caching)
                        .chain(ipv6),
                ),
            ),
            fact(words, "eero:internet:ipv6", on_off(net.ipv6, words)),
        ]
        .into_iter()
        .chain(net.addresses.subnets.iter().map(|one| subnet(one, words)))
        .chain([
            fact(
                words,
                "eero:internet:forwards",
                &eeros_words(&net.addresses.forwards, words),
            ),
            fact(
                words,
                "eero:internet:pinholes",
                &eeros_words(&net.addresses.pinholes, words),
            ),
        ])
        .flatten(),
    );
    facts
}

/// A subnet, by its kind: how it stands first, a part that isn't so giving
/// way, then its address and prefix and the range DHCP hands out, "off ·
/// 10.0.0.0/21 · DHCP 10.0.0.20 to 10.0.0.254" (VKCF3Z, 2026-11-08). One
/// of a kind the vocabulary doesn't name is named as eero names it.
fn subnet(one: &Subnet, words: &Words) -> Option<Value> {
    let say = |word| words.say("eero:internet", word);
    let when = |is: bool, word| {
        if is {
            say(word).to_owned()
        } else {
            String::new()
        }
    };
    let range = match (one.first.as_deref(), one.last.as_deref()) {
        (Some(first), Some(last)) => fill(say("dhcp"), &[("first", first), ("last", last)]),
        _ => String::new(),
    };
    let said = joined([
        when(one.on == Some(false), "off"),
        when(one.internet == Some(false), "noInternet"),
        when(one.open == Some(true), "open"),
        with_prefix(one.address.as_deref(), one.mask.as_deref(), words),
        range,
    ]);
    match one.kind.as_deref() {
        Some("main") => fact(words, "eero:internet:main-subnet", &said),
        Some("guest") => fact(words, "eero:internet:guest-subnet", &said),
        _ => (!said.is_empty())
            .then(|| json!({"name": one.name.as_deref().unwrap_or(say("subnet")), "said": said})),
    }
}

/// An address with its mask as a prefix, "10.0.0.0/22", the mask whole
/// only where it's no prefix.
fn with_prefix(address: Option<&str>, mask: Option<&str>, words: &Words) -> String {
    let bits = mask.and_then(prefix);
    match (address, bits) {
        (Some(address), Some(bits)) if !address.is_empty() => format!("{address}/{bits}"),
        _ => joined([
            address.unwrap_or("").to_owned(),
            mask.filter(|mask| !mask.is_empty())
                .map(|mask| fill(words.say("eero:internet", "mask"), &[("mask", mask)]))
                .unwrap_or_default(),
        ]),
    }
}

/// The prefix length a mask such as 255.255.252.0 is, none for one whose
/// ones don't run unbroken from the left.
fn prefix(mask: &str) -> Option<u32> {
    let octets: Vec<u8> = mask
        .split('.')
        .map(|octet| octet.parse().ok())
        .collect::<Option<_>>()?;
    let [a, b, c, d] = octets[..] else {
        return None;
    };
    let bits = u32::from_be_bytes([a, b, c, d]);
    let ones = bits.leading_ones();
    (bits.checked_shl(ones).unwrap_or(0) == 0).then_some(ones)
}

/// What's let in from outside, in eero's own words, since none has given
/// its shape yet; none where eero lists nothing.
fn eeros_words(list: &[Value], words: &Words) -> String {
    if list.is_empty() {
        return words.say("eero:internet", "none").to_owned();
    }
    list.iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// A switch as words: on or off.
fn on_off(value: Option<bool>, words: &Words) -> &str {
    match value {
        Some(true) => words.say("eero:internet", "on"),
        Some(false) => words.say("eero:internet", "off"),
        None => "",
    }
}

/// The home network's settings, each a switch said on or off, or as eero
/// words it.
fn switches(net: &Internet, words: &Words) -> Vec<Value> {
    let say = |word| words.say("eero:internet", word);
    let switch = |id, value| fact(words, id, on_off(value, words));
    let optimized = match net.sqm {
        Some(true) => say("optimized"),
        Some(false) => say("notOptimized"),
        None => "",
    };
    let security = security(net);
    let clash = match net.name_clash {
        Some(true) => say("clash"),
        Some(false) => say("noClash"),
        None => "",
    };
    let mlo = match net.mlo_mode.as_deref() {
        Some("disabled") => say("off"),
        Some("enabled") => say("on"),
        other => other.unwrap_or(""),
    };
    let thread = &net.thread;
    let channel = thread
        .channel
        .map(|n| fill(say("channel"), &[("n", &n.to_string())]));
    let thread = joined(
        [
            on_off(thread.on, words).to_owned(),
            thread.name.clone().unwrap_or_default(),
        ]
        .into_iter()
        .chain(channel),
    );
    let guest = joined([
        on_off(net.guest.on, words).to_owned(),
        net.guest.name.clone().unwrap_or_default(),
    ]);
    let mut facts = vec![heading(words, "eero:internet:settings")];
    facts.extend(
        [
            fact(words, "eero:internet:sqm", optimized),
            switch("eero:internet:upnp", net.upnp),
            switch("eero:internet:randomization", net.nat_port_randomization),
            switch("eero:internet:steering", net.band_steering),
            switch("eero:internet:wpa3", net.wpa3),
            fact(words, "eero:internet:security", &security),
            fact(
                words,
                "eero:internet:wifi-mode",
                net.wireless_mode.as_deref().unwrap_or(""),
            ),
            switch("eero:internet:ac-compatibility", net.ac_compatibility),
            fact(words, "eero:internet:name-clash", clash),
            fact(words, "eero:internet:mlo", mlo),
            fact(words, "eero:internet:thread", &thread),
            fact(words, "eero:internet:guest", &guest),
            switch("eero:internet:ddns", net.ddns),
            switch("eero:internet:backup", net.backup_internet),
            switch("eero:internet:power-saving", net.power_saving),
        ]
        .into_iter()
        .flatten(),
    );
    facts
}

/// The bands' security as its initialism: one where every band agrees,
/// "WPA2", else band by band, "2.4 GHz WPA2 · 5 GHz WPA3".
fn security(net: &Internet) -> String {
    let said: Vec<(&str, String)> = net
        .security
        .iter()
        .filter(|one| !one.security.is_empty())
        .map(|one| (one.band.as_str(), one.security.to_uppercase()))
        .collect();
    match said.first() {
        Some((_, first)) if said.iter().all(|(_, each)| each == first) => first.clone(),
        _ => joined(
            said.into_iter()
                .map(|(band, each)| format!("{band} {each}")),
        ),
    }
}

/// What happened to the internet over the last 30 days, newest first:
/// eero's activity in its own words, and each change the daemon told as a
/// notice, its summary then what it said, each by when, as prose. What
/// happened to a device, its words starting with its name, is in the
/// device's own drawer.
fn events(model: &HomeNetwork, words: &Words) -> Vec<Value> {
    let devices: Vec<String> = model
        .devices
        .iter()
        .map(|one| name_of(&[&one.name, &one.nickname, &one.hostname], &one.mac))
        .chain(
            model
                .devices_off
                .iter()
                .map(|one| name_of(&[&one.name, &one.nickname, &one.hostname], &one.mac)),
        )
        .map(|name| format!("{name} "))
        .collect();
    let listed = timeline(model)
        .into_iter()
        .filter(|(_, text)| !devices.iter().any(|name| text.starts_with(name)))
        .map(|(at, text)| {
            json!({
                "name": said_at::at(at * SECOND, Form::Moment),
                "said": sentence(model, &text),
                "words": true,
            })
        })
        .collect();
    under(words, "eero:internet:events", listed)
}

#[cfg(test)]
#[path = "internet_tests.rs"]
mod tests;
