//! The Home Network's own drawer, the one its row in Network shows, made
//! ready here as its rows are (rows.rs), so the shell only draws it: whose
//! it is, "via eero · network", a lead saying how the home network
//! stands, "Connected to the internet, last tested at 370 Mb/s down, with 3
//! nodes online and 15 devices on now.", then the figures it's made from. Every row has a drawer
//! (Joel, 2026-11-11 22:00), and this is the form VKCF3Z gave the Network
//! places' rows (2026-11-12): the way, then the kind, never the title
//! again.
//!
//! It's a record of its own, `drawer:eero:place`, served beside the rows
//! and made again with them, so a watch is told it only once it reads
//! differently. Its speed test's day travels as a time, as the internet
//! row's does, so it never changes as the clock does.

use ready_rows::numbers::Numbers;
use ready_rows::time::{self, Form};
use serde_json::{Value, json};

use crate::model::{CloudState, HomeNetwork};
use crate::rows::{self, Words, fill};
use crate::time::to_unix;

/// The kind the drawer's record is.
pub const KIND: &str = "drawer";

/// The key it goes by.
pub const KEY: &str = "drawer:eero:place";

/// The parts the drawer names, each by its thing in the vocabulary.
pub const NAMES: [&str; 7] = [
    "eero:internet:state",
    "eero:internet:isp",
    "eero:internet:down",
    "eero:internet:up",
    "eero:internet:speed-test",
    "eero:internet:outages",
    "eero:internet:nodes",
];

/// The lead's sentences, in the place's `tells`: one for each way the
/// internet can stand that eero says, and one for each reason it says
/// nothing.
pub const TELLS: [&str; 4] = ["connected", "disconnected", "signedOut", "unreachable"];

/// The drawer's record.
pub fn record(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Value {
    json!({
        "key": KEY,
        "kind": KIND,
        "drawer": {
            "eyebrow": words.say("eero", "eyebrow"),
            "title": words.place(),
            "body": lead(model, words, numbers),
            "facts": facts(model, words, numbers),
        },
    })
}

/// How the home network stands, in a sentence, its speed down where a
/// test said one; nothing for a way of standing eero has no word for here.
fn lead(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> String {
    let state = model.internet.state.as_deref().unwrap_or("").to_lowercase();
    let sentence = match model.cloud.state {
        CloudState::SignedOut => "signedOut",
        CloudState::Unreachable => "unreachable",
        CloudState::Answering if state == "connected" => "connected",
        CloudState::Answering if state == "disconnected" => "disconnected",
        CloudState::Answering => return String::new(),
    };
    let at = model
        .internet
        .speed_test
        .down_mbps
        .map(|down| {
            fill(
                words.say("eero", "leadAt"),
                &[("rate", &words.rate(down, numbers))],
            )
        })
        .unwrap_or_default();
    fill(
        words.tells(sentence),
        &[
            ("at", &at),
            ("nodes", &rows::nodes_said(model, words)),
            ("devices", &words.devices(model.devices.len())),
        ],
    )
}

/// The figures the lead is made from, each named as its part is: how it
/// stands, who carries it, the last test each way and its day, its
/// outages and its nodes. While eero isn't answering, how it stands
/// alone, as the rest are its last.
fn facts(model: &HomeNetwork, words: &Words, numbers: &Numbers) -> Vec<Value> {
    let fact = |id, said: String| json!({"name": words.name(id), "said": said});
    let mut facts = vec![fact("eero:internet:state", rows::state_said(model, words))];
    if model.cloud.state != CloudState::Answering {
        return facts;
    }
    let internet = &model.internet;
    let test = &internet.speed_test;
    if let Some(isp) = internet.isp.as_deref().filter(|isp| !isp.is_empty()) {
        let mut named = fact("eero:internet:isp", isp.to_owned());
        named["identifies"] = json!(true);
        facts.push(named);
    }
    for (id, mbps) in [
        ("eero:internet:down", test.down_mbps),
        ("eero:internet:up", test.up_mbps),
    ] {
        if let Some(mbps) = mbps {
            let mut rate = fact(id, words.rate(mbps, numbers));
            rate["figure"] = json!(true);
            facts.push(rate);
        }
    }
    if let Some(seconds) = test.at.as_deref().and_then(to_unix) {
        let mut day = time::at(seconds * 1000, Form::On);
        day["name"] = json!(words.name("eero:internet:speed-test"));
        facts.push(day);
    }
    facts.push(fact(
        "eero:internet:outages",
        rows::outages_said(model, words),
    ));
    let nodes = rows::nodes_said(model, words);
    if !nodes.is_empty() {
        facts.push(fact("eero:internet:nodes", nodes));
    }
    facts
}

#[cfg(test)]
#[path = "place_tests.rs"]
mod tests;
