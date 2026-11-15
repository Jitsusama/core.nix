//! A network's drawer, what Peek opens on a Networks row (VKCF3Z's ruling
//! of H2VYOB's note, 2026-11-14, [e1eb0ebe]). Its title is the row's name
//! and its shape the row's mark, so its glance holds six facts at most:
//! how its sessions stand, what they spent today, its busiest cells, its
//! machines, the network it grew from and when it last changed. Its short
//! name, which y copies, is folded under z a, and its name as the mesh
//! names it is its eyebrow, as a session's address is.
//!
//! Like a session's, it's a record of its own, watched by key only while
//! it's shown, and its time goes out as `{at, as}` for the shell to word.
//! Unlike a session's, it's served in ready-rows.md's own drawer shape,
//! facts as names and what they say, so the launcher draws it as any
//! ready-rows drawer, since a network has no session for its session
//! drawer to read.

use std::collections::{HashMap, HashSet};

use ready_rows::money::{dollars, maybe_about};
use ready_rows::time::{Form, at};
use serde_json::{Value, json};

use crate::networks;
use crate::row::Words;
use crate::top::PLACE;

const DOT: &str = " \u{b7} ";

/// The busiest cells its glance names, so the line stays one line.
const BUSIEST: usize = 3;

/// What a session spent today, by its address in lower case: its dollars
/// and whether any of them was estimated, as spend-watch serves them;
/// none while the screen is shared or where spend-watch says nothing.
pub type Spent<'a> = &'a dyn Fn(&str) -> Option<(f64, bool)>;

/// The key a network's drawer goes by.
pub fn key(name: &str) -> String {
    format!("drawer:{PLACE}:network:{name}")
}

/// The drawer of every network with a row in Networks, from the node's
/// `network` records, each cell's row taken from those the tree served
/// in `top`.
pub fn drawers(records: &[&Value], top: &[Value], words: &Words, spent: Spent) -> Vec<Value> {
    let rows = networks::sessions(top);
    let named: HashMap<&str, &Value> = records
        .iter()
        .filter_map(|record| Some((record["name"].as_str()?, *record)))
        .collect();
    records
        .iter()
        .filter(|record| networks::shown(record))
        .map(|record| drawer(record, &named, &rows, words, spent))
        .collect()
}

fn drawer(
    network: &Value,
    named: &HashMap<&str, &Value>,
    rows: &HashMap<String, &Value>,
    words: &Words,
    spent: Spent,
) -> Value {
    let name = network["name"].as_str().unwrap_or("");
    let fact = |id: &str, value: String| json!({"name": words.names.word(id), "said": value});
    let cells = cells(network);
    let standing: Vec<String> = networks::standings(network, &words.agents)
        .into_iter()
        .map(|(said, _)| said)
        .collect();
    let machines: Vec<&str> = network["machines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let branched = network["within"].as_str().map(|within| {
        named
            .get(within)
            .map_or_else(|| within.to_owned(), |grew| networks::called(grew))
    });
    let mut facts: Vec<Value> = [
        ("standing", standing.join(DOT)),
        ("spent", spent_by(&cells, spent)),
        ("busiest", busiest(&cells, rows)),
        ("machines", machines.join(", ")),
        ("branched", branched.unwrap_or_default()),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(id, value)| fact(id, value))
    .collect();
    if let Some(heard) = network["heard"].as_i64().filter(|heard| *heard > 0) {
        let mut changed = at(heard, Form::Moment);
        changed["name"] = json!(words.names.word("last-changed"));
        facts.push(changed);
    }
    let mut eyebrow = vec![name];
    // The machines' title is a sentence for its drawer, its row saying its
    // name (VKCF3Z, 692362f1).
    if network["shape"] == "machines"
        && let Some(title) = network["title"].as_str()
    {
        eyebrow.push(title);
    }
    // Its parts too, so the drawer gives way by whole parts, its name the
    // last to go; and set as written, since a network's name is the mesh's,
    // never re-cased.
    let items: Vec<Value> = eyebrow
        .iter()
        .enumerate()
        .map(|(at, said)| json!({"said": said, "keep": eyebrow.len() - at}))
        .collect();
    let mut drawn = json!({
        "key": key(name),
        "kind": "drawer",
        "drawer": {
            "eyebrow": eyebrow.join(DOT),
            "eyebrowItems": items,
            "eyebrowNames": true,
            "title": networks::called(network),
            "facts": facts,
            "body": ""
        }
    });
    if let Some(short) = network["short"].as_str().filter(|short| !short.is_empty()) {
        let label = words.names.word("short");
        drawn["drawer"]["nerd"] = json!([{"label": label, "value": short}]);
    }
    drawn
}

/// Its cells' addresses in lower case, each once, in its record's order.
fn cells(network: &Value) -> Vec<String> {
    let mut seen = HashSet::new();
    network["cells"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_lowercase)
        .filter(|cell| seen.insert(cell.clone()))
        .collect()
}

/// What its sessions spent today, each counted once, though a session may
/// sit in several networks, said as spend-watch says each of them; none
/// where none of them spent a cent or the screen is shared.
fn spent_by(cells: &[String], spent: Spent) -> String {
    let (total, estimated) = cells
        .iter()
        .filter_map(|cell| spent(cell))
        .fold((0.0, false), |(total, estimated), (dollars, its)| {
            (total + dollars, estimated || its)
        });
    if total < 0.005 {
        return String::new();
    }
    maybe_about(dollars(total), estimated)
}

/// Its busiest cells by what they're doing: those working, by the names
/// their rows go by, in its record's order.
fn busiest(cells: &[String], rows: &HashMap<String, &Value>) -> String {
    let working: Vec<&str> = cells
        .iter()
        .filter_map(|cell| rows.get(cell))
        .filter(|row| row["held"]["state"] == "working")
        .filter_map(|row| row["name"].as_str())
        .take(BUSIEST)
        .collect();
    working.join(DOT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    fn record(name: &str, shape: &str, cells: &[&str]) -> Value {
        json!({
            "key": format!("network:{name}"), "kind": "network", "heard": 1_762_790_400_000_i64,
            "name": name, "shape": shape, "short": name.rsplit('.').next(),
            "title": name.rsplit('.').next(), "glyph": null, "within": null,
            "cells": cells, "machines": ["optimus", "skiff"],
            "counts": {"needs-you": 1, "stuck": 0, "moving": 2, "quiet": 0, "dormant": 0},
        })
    }

    fn session(address: &str, state: &str) -> Value {
        json!({
            "key": format!("row:{PLACE}:{address}"), "kind": "session",
            "name": format!("{address}'s Quest"), "held": {"address": address, "state": state},
        })
    }

    fn facts(drawn: &Value) -> Vec<(String, Value)> {
        drawn["drawer"]["facts"]
            .as_array()
            .expect("facts")
            .iter()
            .map(|fact| {
                let name = fact["name"].as_str().unwrap_or("").to_owned();
                let said = fact.get("at").map_or_else(
                    || fact["said"].clone(),
                    |at| json!({"at": at, "as": fact["as"]}),
                );
                (name, said)
            })
            .collect()
    }

    fn nothing_spent(_: &str) -> Option<(f64, bool)> {
        None
    }

    #[test]
    fn a_networks_drawer_glances_at_six_facts_in_their_order() {
        let mut repo = record("repo.world", "repository", &["KELP01", "REEF02", "TIDE03"]);
        repo["within"] = json!("hub.tide");
        let hub = record("hub.tide", "group", &[]);
        let top = [
            session("KELP01", "working"),
            session("REEF02", "needs"),
            session("TIDE03", "working"),
        ];
        let spent = |address: &str| match address {
            "kelp01" => Some((12.5, false)),
            "reef02" => Some((3.25, true)),
            _ => None,
        };
        let drawn = drawers(&[&repo, &hub], &top, &words(), &spent);
        assert_eq!(
            drawn[0]["key"], "drawer:agents-top:network:repo.world",
            "keyed as its row says"
        );
        assert_eq!(
            drawn[0]["drawer"]["title"], "world",
            "titled as its row is named"
        );
        assert_eq!(
            facts(&drawn[0]),
            [
                ("Standing".to_owned(), json!("1 needs you \u{b7} 2 moving")),
                (
                    "Spent Today".to_owned(),
                    json!(maybe_about(dollars(15.75), true))
                ),
                (
                    "Busiest".to_owned(),
                    json!("KELP01's Quest \u{b7} TIDE03's Quest")
                ),
                ("Machines".to_owned(), json!("optimus, skiff")),
                ("Branched From".to_owned(), json!("tide")),
                (
                    "Last Changed".to_owned(),
                    json!({"at": 1_762_790_400_000_i64, "as": "moment"})
                ),
            ],
            "how it stands, what it spent, its busiest, its machines, where it grew from, when"
        );
        assert_eq!(
            drawn[0]["drawer"]["nerd"],
            json!([{"label": "Short Name", "value": "world"}]),
            "its short name is folded"
        );
        assert_eq!(
            [
                &drawn[0]["drawer"]["eyebrow"],
                &drawn[0]["drawer"]["eyebrowItems"],
                &drawn[0]["drawer"]["eyebrowNames"],
            ],
            [
                &json!("repo.world"),
                &json!([{"said": "repo.world", "keep": 1}]),
                &json!(true),
            ],
            "its name as the mesh names it is its eyebrow, set as written"
        );
    }

    #[test]
    fn a_session_in_a_network_twice_is_spent_once() {
        let repo = record("repo.world", "repository", &["KELP01", "kelp01"]);
        let spent = |_: &str| Some((2.0, false));
        let drawn = drawers(&[&repo], &[], &words(), &spent);
        assert_eq!(
            facts(&drawn[0])[1],
            ("Spent Today".to_owned(), json!(dollars(2.0))),
            "each session counted once"
        );
    }

    #[test]
    fn what_none_says_is_left_out() {
        let mut quiet = record("hub.tide", "group", &["KELP01"]);
        quiet["counts"] = json!({});
        quiet["machines"] = json!([]);
        quiet["heard"] = json!(null);
        let drawn = drawers(
            &[&quiet],
            &[session("KELP01", "waiting")],
            &words(),
            &nothing_spent,
        );
        assert_eq!(
            drawn[0]["drawer"]["facts"],
            json!([]),
            "no standing, spend, busy cell, machine, parent or time is guessed"
        );
    }

    #[test]
    fn the_machines_drawer_says_its_title_and_an_orchestration_has_none() {
        let mut machines = record("machines", "machines", &["optimus"]);
        machines["title"] = json!("the realm's machines");
        let teams = record("orchestration.kelp01", "orchestration", &["KELP01"]);
        let drawn = drawers(&[&machines, &teams], &[], &words(), &nothing_spent);
        assert_eq!(
            drawn.len(),
            1,
            "an orchestration's drawer is its orchestrator's"
        );
        assert_eq!(
            drawn[0]["drawer"]["title"], "machines",
            "titled by its name"
        );
        assert_eq!(
            drawn[0]["drawer"]["eyebrow"], "machines \u{b7} the realm's machines",
            "its title a sentence above its facts"
        );
        assert_eq!(
            drawn[0]["drawer"]["eyebrowItems"][0],
            json!({"said": "machines", "keep": 2}),
            "its name the last part to give way"
        );
    }
}
