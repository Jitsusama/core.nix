//! Agents' Networks row, after the tree: every network of the mycelium
//! that isn't an orchestration, a step in, under a heading for each kind
//! (VKCF3Z's ruling of H2VYOB's note, 2026-11-14, [e1eb0ebe]). Each is
//! made from the node's `network` record, the very model `mycelium
//! networks` and the map draw, so the launcher holds no second one, and
//! each network steps into its cells, each drawn as the tree draws it.
//!
//! At thousands of cells a heading folds worst first: the networks
//! needing Joel and the stuck, then the busiest few, then one row
//! holding the rest; a network's cells fold the same way.

use std::cmp::Reverse;
use std::collections::HashMap;

use serde_json::{Value, json};

use crate::row::Words;
use crate::top::PLACE;

/// Each kind of network shown, in the order its heading comes: the
/// kind its record says, the name its heading's word goes by, and the
/// mark its rows wear. An orchestration is the tree's own row already,
/// so none is here.
const HEADINGS: [(&str, &str, &str); 4] = [
    ("repository", "repositories", "repository"),
    ("project", "quests", "quest"),
    ("group", "groups", "group"),
    ("machines", "machines", "computer"),
];

/// How a network's sessions stand, worst first: the word its record
/// counts each by, the words a count of it is said in, and how long it
/// keeps its place on a row short of room.
const STANDINGS: [(&str, &str, u8); 5] = [
    ("needs-you", "needCount", 5),
    ("stuck", "stuckCount", 4),
    ("moving", "movingCount", 3),
    ("quiet", "quietCount", 2),
    ("dormant", "dormantCount", 1),
];

/// A heading holding more networks than this folds the rest.
const FOLDS_PAST: usize = 12;

/// The busiest networks a folding heading shows beyond those needing
/// Joel or stuck.
const BUSIEST: usize = 5;

/// A network holding more cells than this folds the rest.
const CELLS_FOLD_PAST: usize = 60;

/// The cells a folding network shows before its fold.
const CELLS_SHOWN: usize = 50;

fn key_of(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

fn more() -> Value {
    json!({"key": "verb:more", "step": "rows"})
}

fn count(network: &Value, standing: &str) -> u64 {
    network["counts"][standing].as_u64().unwrap_or(0)
}

/// Where a network's worst standing puts it: needing Joel, then stuck,
/// then the rest alike.
fn worst(network: &Value) -> usize {
    STANDINGS[..2]
        .iter()
        .position(|(standing, _, _)| count(network, standing) > 0)
        .unwrap_or(2)
}

/// The words a count of `standing` is said in, `1 needs you` apart from
/// the rest.
fn counted(said: &crate::said::Said, standing: &str, words: &str, many: u64) -> String {
    if standing == "needs-you" && many == 1 {
        said.word("needsCount").to_owned()
    } else {
        said.fill(words, &[("count", &many.to_string())])
    }
}

/// What a network's row says at its right: how its sessions stand, worst
/// first and leaving out none, then its machines where there's more than
/// one.
fn side(network: &Value, words: &Words) -> Vec<Value> {
    let said = &words.agents;
    let machines = network["machines"].as_array().map_or(0, Vec::len);
    let standings = STANDINGS.iter().filter_map(|(standing, words, keep)| {
        let many = count(network, standing);
        (many > 0).then(|| json!({"said": counted(said, standing, words, many), "keep": keep}))
    });
    let spread = (machines > 1 && network["shape"] != "machines").then(|| {
        json!({"said": said.fill("machinesCount", &[("count", &machines.to_string())]), "keep": 1})
    });
    standings.chain(spread).collect()
}

/// `networks` in the one order: needing Joel first, then stuck, then the
/// most moving, then the one heard from last.
fn ordered(mut networks: Vec<&Value>) -> Vec<&Value> {
    networks.sort_by_key(|network| {
        (
            worst(network),
            Reverse(count(network, "moving")),
            Reverse(network["heard"].as_i64().unwrap_or(0)),
            network["name"].as_str().unwrap_or(""),
        )
    });
    networks
}

/// The rows of every session the tree served, by its address in lower
/// case, which each network's cells are named by.
fn sessions(top: &[Value]) -> HashMap<String, &Value> {
    let lead = key_of("");
    top.iter()
        .filter(|row| matches!(row["kind"].as_str(), Some("session" | "orchestration")))
        .filter_map(|row| {
            let address = row["key"].as_str()?.strip_prefix(&lead)?;
            (!address.contains(':')).then(|| (address.to_lowercase(), row))
        })
        .collect()
}

/// A cell of a network keyed `parent`: its session's row as the tree
/// serves it, under the network, stepping into nothing more; none where
/// the tree served none.
fn cell(name: &str, parent: &str, depth: usize, rows: &HashMap<String, &Value>) -> Option<Value> {
    let mut cell = (*rows.get(&name.to_lowercase())?).clone();
    cell["key"] = json!(format!("{parent}:{name}"));
    cell["parent"] = json!(parent);
    cell["depth"] = json!(depth);
    cell["kind"] = json!("session");
    if let Some(acts) = cell["acts"].as_array_mut() {
        acts.retain(|act| act["key"] != "verb:more");
    }
    Some(cell)
}

/// The cells of `network`, keyed under `parent`, those needing Joel
/// first, then as its record lists them, folded past sixty; the machines'
/// network holds its machines.
fn cells(
    network: &Value,
    parent: &str,
    depth: usize,
    rows: &HashMap<String, &Value>,
    words: &Words,
) -> Vec<Value> {
    let names = network["cells"].as_array().map_or(&[][..], Vec::as_slice);
    let mut made: Vec<Value> = if network["shape"] == "machines" {
        names
            .iter()
            .filter_map(Value::as_str)
            .map(|name| {
                json!({
                    "key": format!("{parent}:{name}"), "place": PLACE, "parent": parent,
                    "kind": "machine", "name": name, "glyph": "computer", "hue": "program",
                    "depth": depth, "side": [], "acts": [],
                })
            })
            .collect()
    } else {
        names
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|name| cell(name, parent, depth, rows))
            .collect()
    };
    made.sort_by_key(|cell| cell["held"]["state"] != "needs");
    if made.len() <= CELLS_FOLD_PAST {
        return made;
    }
    let rest = made.split_off(CELLS_SHOWN);
    let fold = format!("{parent}:more");
    let count = rest.len().to_string();
    made.push(json!({
        "key": fold, "place": PLACE, "parent": parent, "kind": "fold",
        "name": words.agents.fill("moreCount", &[("count", &count)]),
        "glyph": "agent", "hue": "program", "depth": depth, "side": [], "acts": [more()],
    }));
    made.extend(rest.into_iter().map(|mut cell| {
        cell["parent"] = json!(fold);
        cell["depth"] = json!(depth + 1);
        cell
    }));
    made
}

/// A network's row under `parent`, `depth` steps in, then its cells.
fn network(
    network: &Value,
    parent: &str,
    depth: usize,
    glyph: &str,
    (rows, words): (&HashMap<String, &Value>, &Words),
) -> Vec<Value> {
    let name = network["name"].as_str().unwrap_or("");
    let key = key_of(&format!("network:{name}"));
    let called = if network["shape"] == "machines" {
        words.names.word("machines").to_owned()
    } else {
        network["title"]
            .as_str()
            .or_else(|| network["short"].as_str())
            .unwrap_or(name)
            .to_owned()
    };
    let inside = cells(network, &key, depth + 1, rows, words);
    let acts = if inside.is_empty() {
        json!([])
    } else {
        json!([more()])
    };
    let mut made = vec![json!({
        "key": key, "thing": format!("network:{name}"), "place": PLACE, "parent": parent,
        "kind": "network", "name": called, "glyph": glyph, "hue": "program", "depth": depth,
        "side": side(network, words), "acts": acts,
        "held": {"name": name, "short": network["short"]},
    })];
    made.extend(inside);
    made
}

/// What a heading's fold says at its right: how the networks it holds
/// stand, each by its worst, worst first, or that all stand alike.
fn fold_side(held: &[&Value], words: &Words) -> Vec<Value> {
    let said = &words.agents;
    let by: Vec<(&str, &str, u8, usize)> = STANDINGS[2..]
        .iter()
        .map(|(standing, words, keep)| {
            let many = held
                .iter()
                .filter(|network| {
                    STANDINGS[2..]
                        .iter()
                        .find(|(each, _, _)| count(network, each) > 0)
                        .is_some_and(|(each, _, _)| each == standing)
                })
                .count();
            (*standing, *words, *keep, many)
        })
        .filter(|(_, _, _, many)| *many > 0)
        .collect();
    if let [(standing, _, keep, many)] = by.as_slice()
        && *many == held.len()
    {
        let all = format!("all{}", capitalised(standing));
        return vec![json!({"said": said.word(&all), "keep": keep})];
    }
    by.iter()
        .map(|(_, words, keep, many)| {
            json!({"said": said.fill(words, &[("count", &many.to_string())]), "keep": keep})
        })
        .collect()
}

fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// Every row of the Networks row, it first, from the node's `network`
/// records, each cell's row taken from those the tree served in `top`;
/// none where the node serves no network but orchestrations.
pub fn rows(records: &[&Value], top: &[Value], words: &Words) -> Vec<Value> {
    let sessions = sessions(top);
    let parent = key_of("networks");
    let mut rows = Vec::new();
    for (shape, heading, glyph) in HEADINGS {
        let each = ordered(
            records
                .iter()
                .copied()
                .filter(|record| record["shape"] == shape)
                .collect(),
        );
        if each.is_empty() {
            continue;
        }
        let head = key_of(&format!("networks:{heading}"));
        rows.push(json!({
            "key": head, "place": PLACE, "parent": parent, "kind": "heading",
            "name": words.names.word(heading), "depth": 1, "side": [], "acts": [],
        }));
        let shown = if each.len() > FOLDS_PAST {
            let pressing = each.iter().take_while(|network| worst(network) < 2).count();
            pressing + BUSIEST
        } else {
            each.len()
        };
        let (first, rest) = each.split_at(shown.min(each.len()));
        for one in first {
            rows.extend(network(one, &parent, 1, glyph, (&sessions, words)));
        }
        if rest.is_empty() {
            continue;
        }
        let fold = format!("{head}:more");
        let count = rest.len().to_string();
        rows.push(json!({
            "key": fold, "place": PLACE, "parent": parent, "kind": "fold",
            "name": words.agents.fill("moreCount", &[("count", &count)]), "glyph": glyph,
            "hue": "program", "depth": 1, "side": fold_side(rest, words), "acts": [more()],
        }));
        for one in rest {
            rows.extend(network(one, &fold, 2, glyph, (&sessions, words)));
        }
    }
    if rows.is_empty() {
        return rows;
    }
    let mut all = vec![json!({
        "key": parent, "place": PLACE, "kind": "networks",
        "name": words.names.word("networks"), "glyph": "networks", "hue": "program",
        "depth": 0, "side": [], "acts": [more()],
    })];
    all.extend(rows);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    /// A network record as the node serves it, named `name`, of `shape`,
    /// holding `cells`, standing as `counts` says, heard at `heard`.
    fn record(name: &str, shape: &str, cells: &[&str], counts: &Value, heard: i64) -> Value {
        json!({
            "key": format!("network:{name}"), "kind": "network", "heard": heard,
            "name": name, "shape": shape, "short": name.rsplit('.').next(),
            "title": name.rsplit('.').next(), "glyph": null, "within": null,
            "cells": cells, "machines": ["optimus"], "counts": counts,
        })
    }

    /// A session's row as the tree serves it at the top level.
    fn session(address: &str, state: &str, kind: &str) -> Value {
        json!({
            "key": key_of(address), "thing": format!("agent:{address}"), "place": PLACE,
            "kind": kind, "name": format!("{address}'s quest"), "glyph": "agent",
            "hue": "program", "depth": 0, "side": [], "note": "",
            "drawer": format!("drawer:{PLACE}:{address}"),
            "acts": [{"key": "verb:talk"}, {"key": "verb:details", "step": "drawer"}, more()],
            "held": {"address": address, "state": state},
        })
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter().filter_map(|row| row["key"].as_str()).collect()
    }

    fn said(row: &Value) -> Vec<&str> {
        row["side"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["said"].as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Counts of sessions needing Joel, stuck and moving.
    fn counts(needs: u64, stuck: u64, moving: u64) -> Value {
        json!({"needs-you": needs, "stuck": stuck, "moving": moving})
    }

    fn quiet() -> Value {
        json!({"needs-you": 0, "stuck": 0, "moving": 0, "quiet": 1, "dormant": 0})
    }

    #[test]
    fn no_networks_but_orchestrations_makes_no_row() {
        let teams = record(
            "orchestration.kelp01",
            "orchestration",
            &["KELP01"],
            &quiet(),
            1,
        );
        assert!(
            rows(&[&teams], &[], &words()).is_empty(),
            "an orchestration is the tree's own row, so alone it makes no Networks row"
        );
    }

    #[test]
    fn the_networks_row_steps_into_a_heading_for_each_kind_it_holds() {
        let repo = record("repo.world", "repository", &[], &quiet(), 1);
        let machines = record("machines", "machines", &["optimus"], &quiet(), 1);
        let group = record("hub.tide", "group", &[], &quiet(), 1);
        let made = rows(&[&machines, &repo, &group], &[], &words());
        assert_eq!(
            keys(&made),
            [
                "row:agents-top:networks",
                "row:agents-top:networks:repositories",
                "row:agents-top:network:repo.world",
                "row:agents-top:networks:groups",
                "row:agents-top:network:hub.tide",
                "row:agents-top:networks:machines",
                "row:agents-top:network:machines",
                "row:agents-top:network:machines:optimus",
            ],
            "the headings come in their one order, each only where it holds a network"
        );
        assert_eq!(
            made[0]["name"], "Networks",
            "the row is named as the launcher says"
        );
        assert_eq!(
            made[0]["acts"][0]["key"], "verb:more",
            "the row steps into its networks"
        );
        let heading = &made[1];
        assert_eq!(
            (&heading["kind"], &heading["name"], &heading["acts"]),
            (&json!("heading"), &json!("Repositories"), &json!([])),
            "a heading is named, and carries no act"
        );
        assert!(heading.get("glyph").is_none(), "a heading carries no mark");
        assert_eq!(
            made[2]["parent"], "row:agents-top:networks",
            "a network sits under the row"
        );
        assert_eq!(
            made[2]["glyph"], "repository",
            "a repository's network wears its mark"
        );
        assert_eq!(
            made[6]["name"], "Machines",
            "the machines' network is named as they are"
        );
    }

    #[test]
    fn a_network_says_how_its_sessions_stand_worst_first_leaving_out_none() {
        let counts = json!({"needs-you": 1, "stuck": 0, "moving": 3, "quiet": 2, "dormant": 0});
        let mut repo = record("repo.world", "repository", &[], &counts, 1);
        repo["machines"] = json!(["optimus", "penelope"]);
        let made = rows(&[&repo], &[], &words());
        assert_eq!(
            said(&made[2]),
            ["1 needs you", "3 moving", "2 quiet", "2 machines"],
            "the counts come worst first, zeroes left out, then the machines past one"
        );
    }

    #[test]
    fn networks_sit_needing_joel_first_then_stuck_then_the_most_moving() {
        let standing = counts;
        let calm = record("repo.calm", "repository", &[], &standing(0, 0, 0), 9);
        let late = record("repo.late", "repository", &[], &standing(0, 0, 0), 1);
        let busy = record("repo.busy", "repository", &[], &standing(0, 0, 4), 1);
        let moving = record("repo.moving", "repository", &[], &standing(0, 0, 1), 1);
        let stuck = record("repo.stuck", "repository", &[], &standing(0, 1, 0), 1);
        let needs = record("repo.needs", "repository", &[], &standing(1, 0, 0), 1);
        let made = rows(
            &[&calm, &late, &busy, &moving, &stuck, &needs],
            &[],
            &words(),
        );
        let names: Vec<&str> = made[2..]
            .iter()
            .filter_map(|row| row["name"].as_str())
            .collect();
        assert_eq!(
            names,
            ["needs", "stuck", "busy", "moving", "calm", "late"],
            "needing Joel, stuck, the most moving, then the one heard from last"
        );
    }

    #[test]
    fn a_network_steps_into_its_cells_each_as_the_tree_draws_it() {
        let top = [
            session("KELP01", "working", "orchestration"),
            session("REEF02", "needs", "session"),
        ];
        let repo = record(
            "repo.world",
            "repository",
            &["kelp01", "reef02", "gone03"],
            &quiet(),
            1,
        );
        let made = rows(&[&repo], &top, &words());
        let network = &made[2];
        assert_eq!(
            network["acts"][0]["key"], "verb:more",
            "a network steps into its cells"
        );
        assert_eq!(
            network["held"]["short"], "world",
            "its short name is held, for y"
        );
        assert_eq!(
            keys(&made[3..]),
            [
                "row:agents-top:network:repo.world:reef02",
                "row:agents-top:network:repo.world:kelp01"
            ],
            "its cells come needing Joel first, and a cell the tree never served is left out"
        );
        let cell = &made[4];
        assert_eq!(
            cell["parent"], "row:agents-top:network:repo.world",
            "a cell sits in it"
        );
        assert_eq!(cell["depth"], 2, "a cell sits a step further in");
        assert_eq!(
            cell["drawer"], "drawer:agents-top:KELP01",
            "a cell opens its session's drawer"
        );
        assert_eq!(
            cell["kind"], "session",
            "an orchestrator's cell is its session"
        );
        let acts: Vec<&Value> = cell["acts"].as_array().expect("acts").iter().collect();
        assert_eq!(
            acts,
            [
                &json!({"key": "verb:talk"}),
                &json!({"key": "verb:details", "step": "drawer"})
            ],
            "a cell keeps its session's acts, but steps into nothing further"
        );
    }

    #[test]
    fn a_network_with_no_cells_served_steps_into_nothing() {
        let repo = record("repo.world", "repository", &["gone03"], &quiet(), 1);
        let made = rows(&[&repo], &[], &words());
        assert_eq!(
            made[2]["acts"],
            json!([]),
            "Return on it steps into nothing empty"
        );
    }

    #[test]
    fn a_crowded_heading_folds_all_but_the_pressing_and_the_busiest() {
        let standing = |needs: u64, moving: u64| {
            json!({"needs-you": needs, "stuck": 0, "moving": moving, "quiet": 1 - moving.min(1),
                   "dormant": 0})
        };
        let mut records: Vec<Value> = (0..20)
            .map(|at| {
                record(
                    &format!("repo.r{at:02}"),
                    "repository",
                    &[],
                    &standing(0, 0),
                    at,
                )
            })
            .collect();
        records.push(record("repo.needs", "repository", &[], &standing(1, 0), 0));
        for at in 0..6 {
            records.push(record(
                &format!("repo.m{at}"),
                "repository",
                &[],
                &standing(0, 6 - at),
                0,
            ));
        }
        let all: Vec<&Value> = records.iter().collect();
        let made = rows(&all, &[], &words());
        let under: Vec<&Value> = made
            .iter()
            .filter(|row| row["parent"] == "row:agents-top:networks")
            .collect();
        let names: Vec<&str> = under
            .iter()
            .filter_map(|row| row["name"].as_str())
            .collect();
        assert_eq!(
            names,
            ["needs", "m0", "m1", "m2", "m3", "m4", "21 More"],
            "the pressing and the five busiest show, and one row holds the rest, last"
        );
        let fold = under.last().expect("a fold");
        assert_eq!(
            fold["glyph"], "repository",
            "the fold wears its heading's kind"
        );
        assert_eq!(
            said(fold),
            ["1 moving", "20 quiet"],
            "it says how those it holds stand"
        );
        let held = made
            .iter()
            .filter(|row| row["parent"] == "row:agents-top:networks:repositories:more")
            .count();
        assert_eq!(held, 21, "every network it holds is a step into it");
    }

    #[test]
    fn a_fold_of_networks_all_standing_alike_says_so() {
        let records: Vec<Value> = (0..14)
            .map(|at| record(&format!("repo.r{at:02}"), "repository", &[], &quiet(), at))
            .collect();
        let all: Vec<&Value> = records.iter().collect();
        let made = rows(&all, &[], &words());
        let fold = made
            .iter()
            .find(|row| row["kind"] == "fold")
            .expect("a fold");
        assert_eq!(
            said(fold),
            ["all quiet"],
            "a fold whose networks all stand alike says all"
        );
    }

    #[test]
    fn a_crowded_network_folds_its_cells_past_sixty() {
        let top: Vec<Value> = (0..70)
            .map(|at| session(&format!("C{at:04}"), "working", "session"))
            .collect();
        let names: Vec<String> = (0..70).map(|at| format!("c{at:04}")).collect();
        let cells: Vec<&str> = names.iter().map(String::as_str).collect();
        let repo = record("repo.world", "repository", &cells, &quiet(), 1);
        let made = rows(&[&repo], &top, &words());
        let inside = made
            .iter()
            .filter(|row| row["parent"] == "row:agents-top:network:repo.world")
            .count();
        assert_eq!(
            inside, 51,
            "fifty cells show, then one row holding the rest"
        );
        let fold = made
            .iter()
            .find(|row| row["kind"] == "fold")
            .expect("a fold");
        assert_eq!(fold["name"], "20 More", "the fold says how many it holds");
        let held = made
            .iter()
            .filter(|row| Some(&row["parent"]) == fold.get("key"))
            .count();
        assert_eq!(held, 20, "every cell it holds is a step into it");
    }
}
