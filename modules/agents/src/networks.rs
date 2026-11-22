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
//!
//! Long because over half of it is its cases: each fold and each count a
//! heading says needs a mycelium of its own built beside it.

use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

use crate::row::Words;
use crate::top::PLACE;

/// Each kind of network shown, in the order its heading comes: the
/// kind its record says, the name its heading's word goes by, and the
/// mark its rows wear. An orchestration is the tree's own row already,
/// so none is here.
const HEADINGS: [(&str, &str, &str); 4] = [
    ("repository", "repositories", "repository"),
    // A quest's network wears the mark quests already wear (VKCF3Z, 692362f1).
    ("project", "quests", "agent"),
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

/// How a network's sessions stand, worst first and leaving out none, each
/// count in its words with how long it keeps its place, as its row and
/// its drawer both say it.
pub(crate) fn standings(network: &Value, said: &crate::said::Said) -> Vec<(String, u8)> {
    STANDINGS
        .iter()
        .filter_map(|(standing, words, keep)| {
            let many = count(network, standing);
            (many > 0).then(|| (counted(said, standing, words, many), *keep))
        })
        .collect()
}

/// Whether a network has a row of its own here: an orchestration is the
/// tree's.
pub(crate) fn shown(network: &Value) -> bool {
    HEADINGS
        .iter()
        .any(|(shape, _, _)| network["shape"] == *shape)
}

/// What a network's row and its drawer are called: its own name, as the
/// mesh names it, the machines' too, whose title is a sentence for its
/// drawer (VKCF3Z, 692362f1).
pub(crate) fn called(network: &Value) -> String {
    let name = network["name"].as_str().unwrap_or("");
    network["title"]
        .as_str()
        .filter(|_| network["shape"] != "machines")
        .or_else(|| network["short"].as_str())
        .unwrap_or(name)
        .to_owned()
}

/// What a network's row says at its right: how its sessions stand, worst
/// first and leaving out none, then its machines where there's more than
/// one.
fn side(network: &Value, words: &Words) -> Vec<Value> {
    let said = &words.agents;
    let machines = network["machines"].as_array().map_or(0, Vec::len);
    let standings = standings(network, said)
        .into_iter()
        .map(|(said, keep)| json!({"said": said, "keep": keep}));
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
pub(crate) fn sessions(top: &[Value]) -> HashMap<String, &Value> {
    let lead = key_of("");
    top.iter()
        .filter(|row| matches!(row["kind"].as_str(), Some("session" | "orchestration")))
        .filter_map(|row| {
            let address = row["key"].as_str()?.strip_prefix(&lead)?;
            (!address.contains(':')).then(|| (address.to_lowercase(), row))
        })
        .collect()
}

/// `network` as Joel's mark has it, each want counted once. The node
/// counts a cell by its own standing alone, any cell with a want as
/// needing Joel. So a cell whose want Joel put off (top.rs's `putOff`)
/// leaves needs-you for where it would stand wanting nothing, dormant
/// parked and quiet else, lighting nothing for its hour; and a lead
/// wearing his mark for a session under it (`anyWaitsOnJoel`) counts
/// once more, unless that session, or a marked lead under it, is a cell
/// here, counted already. A put-off lead still marked for one under it
/// still needs him, where the node counted it. A network so holding a
/// marked cell sorts first, says it needs him and never folds away, as
/// the cell does inside it. A cell the record names twice counts once. A
/// lead it moves leaves the count the node put it in, so the counts still
/// sum to the cells; the node's counts don't say which cell is where, so
/// it comes from the likeliest of a lead's standings first, moving.
pub(crate) fn as_marked<'a>(network: &'a Value, rows: &HashMap<String, &Value>) -> Cow<'a, Value> {
    if network["shape"] == "machines" {
        return Cow::Borrowed(network);
    }
    let mut named = HashSet::new();
    let cells: Vec<&Value> = network["cells"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_lowercase)
        .filter(|name| named.insert(name.clone()))
        .filter_map(|name| rows.get(&name).copied())
        .collect();
    let held = |row: &Value, field: &str| row["held"][field] == true;
    let mut unlit: BTreeMap<&str, u64> = BTreeMap::new();
    for row in cells
        .iter()
        .filter(|row| held(row, "putOff") && !held(row, "anyWaitsOnJoel"))
    {
        *unlit.entry(counted_as(row)).or_default() += 1;
    }
    let marked = |row: &&&Value| held(row, "anyWaitsOnJoel") && !held(row, "waitsOnJoel");
    let counted: HashSet<String> = cells
        .iter()
        .filter(|row| held(row, "waitsOnJoel") || marked(row))
        .flat_map(|row| leads_of(row, rows))
        .collect();
    let leads = cells
        .iter()
        .filter(marked)
        .filter(|row| !held(row, "putOff"))
        .filter(|row| !counted.contains(&address_of(row)))
        .count();
    let leads = u64::try_from(leads).unwrap_or(u64::MAX);
    if unlit.is_empty() && leads == 0 {
        return Cow::Borrowed(network);
    }
    let mut seen = network.clone();
    let mut counts: HashMap<&str, u64> = STANDINGS_HELD
        .into_iter()
        .map(|standing| (standing, count(network, standing)))
        .collect();
    for (into, many) in unlit {
        let moved = counts["needs-you"].min(many);
        *counts.entry("needs-you").or_default() -= moved;
        *counts.entry(into).or_default() += moved;
    }
    let mut moving = leads;
    for standing in MARKED_FROM {
        let taken = counts[standing].min(moving);
        *counts.entry(standing).or_default() -= taken;
        moving -= taken;
    }
    *counts.entry("needs-you").or_default() += leads;
    for (standing, many) in counts {
        if seen["counts"].get(standing).is_some() || many > 0 {
            seen["counts"][standing] = json!(many);
        }
    }
    Cow::Owned(seen)
}

/// The addresses of every lead above `row` in the tree, in lower case. A
/// fold never holds a session marked for Joel nor one above it (tree.rs
/// folds only what has nothing more urgent under it), so the walk ends at
/// one.
fn leads_of(row: &Value, rows: &HashMap<String, &Value>) -> Vec<String> {
    let lead = key_of("");
    let mut above = Vec::new();
    let mut at = row;
    while let Some(parent) = at["parent"].as_str() {
        let Some(parent) = parent.strip_prefix(&lead) else {
            break;
        };
        if parent.contains(':') {
            break;
        }
        let address = parent.to_lowercase();
        if above.contains(&address) {
            break;
        }
        above.push(address.clone());
        match rows.get(&address) {
            Some(next) => at = next,
            None => break,
        }
    }
    above
}

/// The standing the node would count a put-off cell under were it to want
/// nothing, from how its row says it would stand (`putOffAs`), in the
/// node's order: paused is stuck, parked dormant, working moving, and the
/// rest quiet (mycelium's site, view.js).
fn counted_as(row: &Value) -> &'static str {
    match row["held"]["putOffAs"].as_str() {
        Some("paused") => "stuck",
        Some("parked") => "dormant",
        Some("working") => "moving",
        _ => "quiet",
    }
}

/// A served row's address, in lower case, as `sessions` keys it.
fn address_of(row: &Value) -> String {
    row["held"]["address"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase()
}

/// The counts a network's record holds, by the node's standings.
const STANDINGS_HELD: [&str; 5] = ["needs-you", "stuck", "moving", "quiet", "dormant"];

/// The counts a lead wearing Joel's mark leaves, likeliest first: a lead
/// waiting on a session under it is most often moving. If none is left
/// (the node a beat behind), the lead counts anyway, so its want shows.
const MARKED_FROM: [&str; 4] = ["moving", "quiet", "dormant", "stuck"];

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
    // Waiting on Joel, as the tree has it and its mark says: it or a session
    // under it, parked with his want open too.
    made.sort_by_key(|cell| cell["held"]["anyWaitsOnJoel"] != true);
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
    let inside = cells(network, &key, depth + 1, rows, words);
    // Return steps in, so the step comes first; Peek opens its drawer.
    let peek = json!({"key": "verb:details", "step": "drawer"});
    let acts = if inside.is_empty() {
        json!([peek])
    } else {
        json!([more(), peek])
    };
    let mut made = vec![json!({
        "key": key, "thing": format!("network:{name}"), "place": PLACE, "parent": parent,
        "kind": "network", "name": called(network), "glyph": glyph, "hue": "program",
        "depth": depth, "side": side(network, words), "acts": acts,
        "drawer": crate::network_drawer::key(name),
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
    let marked: Vec<Cow<'_, Value>> = records
        .iter()
        .map(|record| as_marked(record, &sessions))
        .collect();
    let parent = key_of("networks");
    let mut rows = Vec::new();
    for (shape, heading, glyph) in HEADINGS {
        let each = ordered(
            marked
                .iter()
                .map(AsRef::as_ref)
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
            "cells": cells, "machines": ["kestrel"], "counts": counts,
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
            "held": {"address": address, "state": state, "waitsOnJoel": state == "needs",
                "anyWaitsOnJoel": state == "needs"},
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
        let mut machines = record("machines", "machines", &["kestrel"], &quiet(), 1);
        machines["title"] = json!("the realm's machines");
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
                "row:agents-top:network:machines:kestrel",
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
            made[6]["name"], "machines",
            "the machines' network is named as the mesh names it"
        );
    }

    #[test]
    fn a_network_says_how_its_sessions_stand_worst_first_leaving_out_none() {
        let counts = json!({"needs-you": 1, "stuck": 0, "moving": 3, "quiet": 2, "dormant": 0});
        let mut repo = record("repo.world", "repository", &[], &counts, 1);
        repo["machines"] = json!(["kestrel", "skiff"]);
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
    fn a_cell_parked_with_joels_want_open_comes_first_as_one_needing_him() {
        let mut asking = session("REEF02", "parked", "session");
        asking["held"]["waitsOnJoel"] = json!(true);
        asking["held"]["anyWaitsOnJoel"] = json!(true);
        let top = [
            session("KELP01", "working", "orchestration"),
            session("TERN03", "parked", "session"),
            asking,
        ];
        let repo = record(
            "repo.world",
            "repository",
            &["kelp01", "tern03", "reef02"],
            &quiet(),
            1,
        );
        let made = rows(&[&repo], &top, &words());
        assert_eq!(
            keys(&made[3..]),
            [
                "row:agents-top:network:repo.world:reef02",
                "row:agents-top:network:repo.world:kelp01",
                "row:agents-top:network:repo.world:tern03",
            ],
            "its state stays parked, yet waiting on Joel it comes first, as the tree has it"
        );
    }

    #[test]
    fn a_cell_wearing_joels_mark_for_one_under_it_comes_first() {
        // KELP01 works, but a session in its team waits on Joel, so its row
        // wears his mark (top.rs's worn) and sorts with those needing him.
        let mut team = session("KELP01", "working", "orchestration");
        team["held"]["anyWaitsOnJoel"] = json!(true);
        let top = [session("TERN03", "working", "session"), team];
        let repo = record(
            "repo.world",
            "repository",
            &["tern03", "kelp01"],
            &quiet(),
            1,
        );
        let made = rows(&[&repo], &top, &words());
        assert_eq!(
            keys(&made[3..]),
            [
                "row:agents-top:network:repo.world:kelp01",
                "row:agents-top:network:repo.world:tern03",
            ],
            "the order and the mark agree"
        );
    }

    #[test]
    fn a_network_holding_a_marked_cell_needs_joel_however_the_node_counts_it() {
        // KELP01's network counts it moving, the node judging it by its own
        // standing, yet its row wears Joel's mark for a session in its team.
        let mut team = session("KELP01", "working", "orchestration");
        team["held"]["anyWaitsOnJoel"] = json!(true);
        let top = [team];
        let mut records: Vec<Value> = (0..20)
            .map(|at| {
                let moving = json!({"needs-you": 0, "stuck": 0, "moving": 3, "quiet": 0});
                record(&format!("repo.r{at:02}"), "repository", &[], &moving, at)
            })
            .collect();
        let moving = json!({"needs-you": 0, "stuck": 0, "moving": 1, "quiet": 0});
        records.push(record("repo.world", "repository", &["kelp01"], &moving, 0));
        let all: Vec<&Value> = records.iter().collect();
        let made = rows(&all, &top, &words());
        let world = made
            .iter()
            .find(|row| row["key"] == "row:agents-top:network:repo.world")
            .expect("its row");
        assert_eq!(
            [&world["parent"], &made[2]["key"]],
            [
                "row:agents-top:networks",
                "row:agents-top:network:repo.world"
            ],
            "it sorts first and never folds away, past twelve networks"
        );
        assert_eq!(
            said(world),
            ["1 needs you"],
            "and says so, its one cell counted once"
        );
    }

    #[test]
    fn a_marked_cell_leaves_the_count_the_node_put_it_in() {
        let mut lead = session("KELP01", "working", "orchestration");
        lead["held"]["anyWaitsOnJoel"] = json!(true);
        let mut quiet_lead = session("KELP02", "working", "orchestration");
        quiet_lead["held"]["anyWaitsOnJoel"] = json!(true);
        let top = [lead, quiet_lead];
        let counts = json!({"needs-you": 0, "stuck": 1, "moving": 1, "quiet": 1, "dormant": 0});
        let team = record(
            "repo.world",
            "repository",
            &["kelp01", "kelp02", "tern03"],
            &counts,
            0,
        );
        let seen = as_marked(&team, &sessions(&top));
        assert_eq!(
            seen["counts"],
            json!({"needs-you": 2, "stuck": 1, "moving": 0, "quiet": 0, "dormant": 0}),
            "two marked cells leave moving, then quiet, and the counts still sum to three cells"
        );
    }

    #[test]
    fn marked_leads_leave_what_is_left_and_count_even_when_nothing_is() {
        let lead = |address: &str| {
            let mut lead = session(address, "working", "orchestration");
            lead["held"]["anyWaitsOnJoel"] = json!(true);
            lead
        };
        let top = [lead("KELP01"), lead("KELP02")];
        let seen = |counts: Value| {
            let team = record(
                "repo.world",
                "repository",
                &["kelp01", "kelp02"],
                &counts,
                0,
            );
            as_marked(&team, &sessions(&top))["counts"].clone()
        };
        assert_eq!(
            [
                seen(json!({"needs-you": 0, "stuck": 0, "moving": 1, "quiet": 0, "dormant": 1})),
                seen(json!({"needs-you": 0, "stuck": 0, "moving": 0, "quiet": 0, "dormant": 0})),
                seen(json!({"needs-you": 0, "stuck": 0, "moving": 1})),
            ],
            [
                json!({"needs-you": 2, "stuck": 0, "moving": 0, "quiet": 0, "dormant": 0}),
                json!({"needs-you": 2, "stuck": 0, "moving": 0, "quiet": 0, "dormant": 0}),
                json!({"needs-you": 2, "stuck": 0, "moving": 0}),
            ],
            "leads leave moving, then dormant past an empty quiet; with nothing left, the node a \
             beat behind, they count anyway, so their wants show"
        );
    }

    #[test]
    fn a_lead_and_its_waiting_member_both_cells_need_joel_once() {
        let mut lead = session("KELP01", "working", "orchestration");
        lead["held"]["anyWaitsOnJoel"] = json!(true);
        let member = |parent: &str| {
            let mut member = session("REEF02", "needs", "session");
            member["parent"] = json!(key_of(parent));
            member
        };
        // As the node counts them: REEF02 needs Joel, KELP01 moves.
        let counts = json!({"needs-you": 1, "stuck": 0, "moving": 1, "quiet": 0, "dormant": 0});
        let repo = record(
            "repo.world",
            "repository",
            &["kelp01", "reef02"],
            &counts,
            0,
        );
        let top = [lead, member("KELP01")];
        let made = rows(&[&repo], &top, &words());
        assert_eq!(
            said(&made[2]).join(", "),
            "1 needs you, 1 moving",
            "the lead's mark is its member's want, counted once already"
        );
    }

    /// A lead marked for a session under it, as top.rs serves one, under
    /// `parent` where it has one.
    fn marked_lead(address: &str, parent: Option<&str>) -> Value {
        let mut lead = session(address, "working", "orchestration");
        lead["held"]["anyWaitsOnJoel"] = json!(true);
        if let Some(parent) = parent {
            lead["parent"] = json!(key_of(parent));
        }
        lead
    }

    #[test]
    fn a_lead_and_a_marked_lead_under_it_count_one_want_beyond_them_once() {
        let top = [
            marked_lead("KELP01", None),
            marked_lead("REEF02", Some("KELP01")),
        ];
        // The waiting session works in another repository, no cell here.
        let counts = json!({"needs-you": 0, "stuck": 0, "moving": 2, "quiet": 0, "dormant": 0});
        let repo = record(
            "repo.world",
            "repository",
            &["kelp01", "reef02"],
            &counts,
            0,
        );
        assert_eq!(
            as_marked(&repo, &sessions(&top))["counts"],
            json!({"needs-you": 1, "stuck": 0, "moving": 1, "quiet": 0, "dormant": 0}),
            "REEF02 counts the want under it, and KELP01 above it counts nothing more"
        );
    }

    #[test]
    fn a_cell_the_record_names_twice_counts_once() {
        let top = [marked_lead("KELP01", None)];
        let counts = json!({"needs-you": 0, "stuck": 0, "moving": 1, "quiet": 0, "dormant": 0});
        let repo = record(
            "repo.world",
            "repository",
            &["KELP01", "kelp01"],
            &counts,
            0,
        );
        assert_eq!(
            as_marked(&repo, &sessions(&top))["counts"],
            json!({"needs-you": 1, "stuck": 0, "moving": 0, "quiet": 0, "dormant": 0}),
            "one lead, named in two casings, needs Joel once"
        );
    }

    #[test]
    fn a_put_off_lead_still_marked_for_one_under_it_stays_where_the_node_put_it() {
        let mut lead = marked_lead("KELP01", None);
        lead["held"]["putOff"] = json!(true);
        let top = [lead, session("TERN03", "working", "session")];
        // The node counted KELP01 needing Joel for its own want, TERN03 moving.
        let counts = json!({"needs-you": 1, "stuck": 0, "moving": 1, "quiet": 0, "dormant": 0});
        let repo = record(
            "repo.world",
            "repository",
            &["kelp01", "tern03"],
            &counts,
            0,
        );
        assert_eq!(
            as_marked(&repo, &sessions(&top))["counts"],
            counts,
            "its own want put off, it still needs Joel for the one under it, and TERN03 still \
             moves"
        );
    }

    #[test]
    fn a_network_whose_only_want_joel_put_off_needs_him_for_nothing() {
        let mut put_off = session("TERN03", "needs", "session");
        put_off["held"] = json!({"address": "TERN03", "state": "needs",
            "waitsOnJoel": false, "anyWaitsOnJoel": false, "putOff": true});
        let counts = json!({"needs-you": 1, "stuck": 0, "moving": 0, "quiet": 0, "dormant": 0});
        let repo = record("repo.world", "repository", &["tern03"], &counts, 0);
        let calm = record("repo.calm", "repository", &[], &quiet(), 9);
        let made = rows(&[&repo, &calm], &[put_off.clone()], &words());
        let world = made
            .iter()
            .find(|row| row["key"] == "row:agents-top:network:repo.world")
            .expect("its row");
        assert_eq!(
            (said(world), &made[2]["key"]),
            (vec!["1 quiet"], &json!("row:agents-top:network:repo.calm")),
            "put off, its cell is quiet for the hour, so the network neither says it needs Joel \
             nor sorts first"
        );
        let counted = |wanting_nothing: &str| {
            let mut one = put_off.clone();
            one["held"]["putOffAs"] = json!(wanting_nothing);
            let marked = as_marked(&repo, &sessions(&[one]))["counts"].clone();
            STANDINGS_HELD
                .into_iter()
                .find(|standing| marked[*standing] == 1)
                .unwrap_or_default()
        };
        assert_eq!(
            ["parked", "paused", "working", "listening"].map(counted),
            ["dormant", "stuck", "moving", "quiet"],
            "each counts as the node counts a cell standing so wanting nothing, as its row and \
             Home have it"
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
            (&network["acts"][1], &network["drawer"]),
            (
                &json!({"key": "verb:details", "step": "drawer"}),
                &json!("drawer:agents-top:network:repo.world")
            ),
            "Peek opens its drawer"
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
            json!([{"key": "verb:details", "step": "drawer"}]),
            "Return on it steps into nothing empty, though Peek still opens its drawer"
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
            [
                "Repositories",
                "needs",
                "m0",
                "m1",
                "m2",
                "m3",
                "m4",
                "21 More"
            ],
            "under its heading, the pressing and the five busiest, then one row for the rest"
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
