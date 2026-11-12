//! Agents' top level as ready rows (ready-rows.md), one row for each
//! orchestration and the sessions inside it a step in, so Joel talks to
//! six orchestrators rather than reading fifty sessions (VKCF3Z's
//! launcher-shape ruling, 2026-11-11: "talking to 6 instead of 53 is the
//! load he asked to lose"). An orchestration's row acts first with Talk
//! to It, since he steers a team through its orchestrator; a session
//! inside one acts first with a look at its drawer, and is gone to only
//! where it needs his own hands.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::figures::Item;
use crate::row::{Row, Shown};
use crate::said::Said;

/// The launcher's place for the top level.
pub const PLACE: &str = "agents-top";

/// What the top level reads beyond the rows: what an orchestration and
/// those under it spent today in all, by its orchestrator's address,
/// none while the screen is shared.
pub struct Known<'a> {
    pub spent: &'a dyn Fn(&str) -> Option<String>,
}

fn key_of(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

fn address(row: &Row) -> &str {
    row.key.trim_start_matches("agent:")
}

fn items(items: &[Item]) -> Value {
    json!(
        items
            .iter()
            .map(|item| json!({"said": item.said, "keep": item.keep}))
            .collect::<Vec<_>>()
    )
}

/// Every session's row `shown` holds, a fold's members too.
fn gathered<'a>(shown: &'a [Shown], into: &mut Vec<&'a Row>) {
    for one in shown {
        match one {
            Shown::Session(row) => into.push(row),
            Shown::Fold(fold) => gathered(&fold.held, into),
        }
    }
}

/// Each orchestration's entries: an orchestrator's row at the top of the
/// tree and every entry laid under it, in the order laid.
fn orchestrations(shown: &[Shown]) -> Vec<&[Shown]> {
    let starts: Vec<usize> = shown
        .iter()
        .enumerate()
        .filter(|(_, one)| matches!(one, Shown::Session(row) if row.depth == 0))
        .map(|(at, _)| at)
        .collect();
    starts
        .iter()
        .zip(starts.iter().skip(1).copied().chain([shown.len()]))
        .map(|(from, upto)| &shown[*from..upto])
        .collect()
}

/// What an orchestration's row says at its right: how many of its
/// sessions need Joel, how many of all of them work, and what it spent
/// in all, "1 needs you · 3 of 5 working · about $604 in all". Each part
/// is left out where it would say none.
fn side(all: &[&Row], spent: Option<&str>, said: &Said) -> Vec<Item> {
    let count = |state: &str| all.iter().filter(|row| row.state == state).count();
    let needing = count("needs");
    let working = count("working");
    let needs = match needing {
        0 => String::new(),
        1 => said.fill("needsCount", &[("count", "1")]),
        _ => said.fill("needCount", &[("count", &needing.to_string())]),
    };
    let works = if working > 0 {
        said.fill(
            "workingOf",
            &[
                ("count", &working.to_string()),
                ("all", &all.len().to_string()),
            ],
        )
    } else {
        String::new()
    };
    let spent = spent.map_or_else(String::new, |spent| {
        said.fill("aboutAllOf", &[("spent", spent)])
    });
    [(needs, 5), (works, 4), (spent, 3)]
        .into_iter()
        .filter(|(said, _)| !said.is_empty())
        .map(|(said, keep)| Item { said, keep })
        .collect()
}

/// The ask that does a session's act, as Agents' rows send it.
fn act(id: &str, address: &str) -> Value {
    json!({"key": format!("verb:agent:{id}"), "ask": {"ask": id, "address": address}})
}

fn peek() -> Value {
    json!({"key": "verb:details", "step": "drawer"})
}

fn more() -> Value {
    json!({"key": "verb:more", "step": "rows"})
}

/// An orchestration's row: Talk to It first, a look at its drawer, a
/// step into its sessions where it has any, then its orchestrator's own
/// acts.
fn orchestration(row: &Row, members: &[&Row], known: &Known, said: &Said) -> Value {
    let address = address(row);
    let mut all = vec![row];
    all.extend(members.iter().copied());
    let side = if members.is_empty() {
        row.side_items.clone()
    } else {
        self::side(&all, (known.spent)(address).as_deref(), said)
    };
    let mut acts = vec![act("talk", address), peek()];
    if !members.is_empty() {
        acts.push(more());
    }
    acts.extend(
        row.acts
            .iter()
            .filter(|id| **id != "talk")
            .map(|id| act(id, address)),
    );
    json!({
        "key": key_of(address), "thing": row.key, "place": PLACE,
        "kind": if members.is_empty() { "session" } else { "orchestration" },
        "name": row.name, "glyph": row.glyph, "hue": row.hue, "depth": 0,
        "side": items(&side), "note": row.note,
        "drawer": format!("drawer:agents:{address}"), "acts": acts,
        "held": {"address": address, "also": row.also, "state": row.state, "gone": row.gone},
    })
}

/// A session inside an orchestration: a look at its drawer first, gone to
/// or talked to only where it needs Joel (VKCF3Z: he acts at a leaf only
/// where it needs his own hands there), then what pauses, wakes or ends
/// it.
fn member(row: &Row, parent: &str, depth: usize) -> Value {
    let address = address(row);
    let needs = row.state == "needs";
    let mut acts = vec![peek()];
    acts.extend(
        row.acts
            .iter()
            .filter(|id| needs || !matches!(**id, "switch" | "talk"))
            .map(|id| act(id, address)),
    );
    json!({
        "key": key_of(address), "thing": row.key, "place": PLACE, "parent": parent,
        "kind": "session", "name": row.name, "glyph": row.glyph, "hue": row.hue,
        "depth": depth, "side": items(&row.side_items), "note": row.note,
        "drawer": format!("drawer:agents:{address}"), "acts": acts,
        "held": {"address": address, "also": row.also, "state": row.state, "gone": row.gone},
    })
}

/// The entries laid under an orchestrator, each under the nearest laid
/// above it less deep, a fold's members under the fold, at one step in
/// for each row above. Those `above` are never left, so a fold's members,
/// laid from depth 0 again (tree.rs), stay under the fold.
fn members(shown: &[Shown], above: &[(usize, String)], rows: &mut Vec<Value>) {
    let mut stack = above.to_vec();
    for one in shown {
        let (depth, key) = match one {
            Shown::Session(row) => (row.depth, key_of(address(row))),
            Shown::Fold(fold) => (fold.depth, key_of(fold.key.trim_start_matches("agents:"))),
        };
        while stack.len() > above.len() && stack.last().is_some_and(|(at, _)| *at >= depth) {
            stack.pop();
        }
        let parent = stack
            .last()
            .map_or_else(String::new, |(_, key)| key.clone());
        let steps = stack.len();
        match one {
            Shown::Session(row) => rows.push(member(row, &parent, steps)),
            Shown::Fold(fold) => {
                rows.push(json!({
                    "key": key, "place": PLACE, "parent": parent, "kind": "fold",
                    "name": fold.name, "glyph": fold.glyph, "hue": fold.hue,
                    "depth": steps, "side": [], "acts": [more()],
                }));
                let mut within = stack.clone();
                within.push((depth, key.clone()));
                members(&fold.held, &within, rows);
            }
        }
        stack.push((depth, key));
    }
}

/// Every row of the top level in the order it's shown: the orchestrations
/// needing Joel first, then the rest as Agents lists them, the one used
/// last first (session.rs's `in_order`), each followed by its sessions.
/// A row stepped into from another names it as `parent`; one with rows
/// under it steps into them.
pub fn rows(shown: &[Shown], known: &Known, said: &Said) -> Vec<Value> {
    let mut each: Vec<(bool, Vec<Value>)> = orchestrations(shown)
        .into_iter()
        .map(|entries| {
            let Some(Shown::Session(head)) = entries.first() else {
                return (false, Vec::new());
            };
            let mut all = Vec::new();
            gathered(&entries[1..], &mut all);
            let needs = head.state == "needs" || all.iter().any(|row| row.state == "needs");
            let mut rows = vec![orchestration(head, &all, known, said)];
            members(&entries[1..], &[(0, key_of(address(head)))], &mut rows);
            (needs, rows)
        })
        .collect();
    each.sort_by_key(|(needs, _)| !needs);
    let mut rows: Vec<Value> = each.into_iter().flat_map(|(_, rows)| rows).collect();
    let parents: HashSet<String> = rows
        .iter()
        .filter_map(|row| row["parent"].as_str().map(str::to_owned))
        .collect();
    for row in &mut rows {
        let steps_in = row["acts"]
            .as_array()
            .is_some_and(|acts| acts.iter().any(|act| act["key"] == "verb:more"));
        if !steps_in
            && row["kind"] == "session"
            && parents.contains(row["key"].as_str().unwrap_or(""))
            && let Some(acts) = row["acts"].as_array_mut()
        {
            acts.insert(1, more());
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::{Fold, Ranked};

    fn said() -> Said {
        Said::of_thing(crate::said::LAUNCHERS, "agents").expect("the launcher's words read")
    }

    /// A session's row as row.rs makes it: `address`, how it `state`s,
    /// how deep it's laid and its acts.
    fn row(address: &str, state: &'static str, depth: usize, acts: &[&'static str]) -> Shown {
        Shown::Session(Box::new(Row {
            key: format!("agent:{address}"),
            name: format!("{address}'s quest"),
            also: address.to_owned(),
            glyph: "agent",
            hue: "program",
            side: state.to_owned(),
            side_items: vec![Item {
                said: state.to_owned(),
                keep: 3,
            }],
            note: String::new(),
            depth,
            orchestrates: false,
            gone: false,
            state,
            returns: "switch",
            acts: acts.to_vec(),
            ranked: Ranked {
                note: String::new(),
                side: String::new(),
                side_items: Vec::new(),
                belongs: String::new(),
            },
        }))
    }

    const LIVE: &[&str] = &["switch", "talk", "pause", "close"];

    fn top(shown: &[Shown]) -> Vec<Value> {
        let spent = |address: &str| (address == "O").then(|| "$604".to_owned());
        rows(shown, &Known { spent: &spent }, &said())
    }

    fn found<'a>(rows: &'a [Value], key: &str) -> &'a Value {
        rows.iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no {key} among {rows:?}"))
    }

    fn sides(row: &Value) -> Vec<&str> {
        row["side"]
            .as_array()
            .map(|side| {
                side.iter()
                    .filter_map(|item| item["said"].as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn acts(row: &Value) -> Vec<&str> {
        row["acts"]
            .as_array()
            .map(|acts| acts.iter().filter_map(|act| act["key"].as_str()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn an_orchestrations_row_says_how_all_its_sessions_stand_and_talks_to_it_first() {
        let rows = top(&[
            row("O", "working", 0, LIVE),
            row("A", "needs", 1, LIVE),
            row("B", "working", 1, LIVE),
            row("C", "listening", 1, LIVE),
            row("D", "working", 1, LIVE),
        ]);
        let head = found(&rows, "row:agents-top:O");
        assert_eq!(
            sides(head),
            ["1 needs you", "3 of 5 working", "about $604 in all"],
            "the side counts the orchestrator and every session under it"
        );
        assert_eq!(head["kind"], "orchestration", "it's an orchestration's row");
        assert_eq!(head["depth"], 0, "it's at the top: {head}");
        assert!(head.get("parent").is_none(), "nothing holds it: {head}");
        assert_eq!(
            acts(head),
            [
                "verb:agent:talk",
                "verb:details",
                "verb:more",
                "verb:agent:switch",
                "verb:agent:pause",
                "verb:agent:close"
            ],
            "Talk to It leads, then a look, a step in, and its own acts"
        );
        assert_eq!(
            head["drawer"], "drawer:agents:O",
            "its drawer is the session's own, as Agents serves it"
        );
    }

    #[test]
    fn a_session_inside_is_gone_to_only_where_it_needs_joel() {
        let rows = top(&[
            row("O", "working", 0, LIVE),
            row("A", "needs", 1, LIVE),
            row("B", "working", 1, LIVE),
        ]);
        for (key, wanted) in [
            (
                "row:agents-top:A",
                vec![
                    "verb:details",
                    "verb:agent:switch",
                    "verb:agent:talk",
                    "verb:agent:pause",
                    "verb:agent:close",
                ],
            ),
            (
                "row:agents-top:B",
                vec!["verb:details", "verb:agent:pause", "verb:agent:close"],
            ),
        ] {
            let member = found(&rows, key);
            assert_eq!(acts(member), wanted, "{key}'s acts: a look first");
            assert_eq!(member["parent"], "row:agents-top:O", "{key} is inside O");
            assert_eq!(member["depth"], 1, "{key} is a step in");
        }
        assert_eq!(
            found(&rows, "row:agents-top:B")["side"][0]["said"],
            "working",
            "a session inside says what its own row says"
        );
    }

    #[test]
    fn orchestrations_needing_joel_come_first_then_as_agents_lists_them() {
        let rows = top(&[
            row("P", "working", 0, LIVE),
            row("Q", "listening", 0, LIVE),
            row("O", "working", 0, LIVE),
            row("A", "needs", 1, LIVE),
        ]);
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:agents-top:O",
                "row:agents-top:A",
                "row:agents-top:P",
                "row:agents-top:Q"
            ],
            "the one with a session needing Joel leads, its sessions after it"
        );
    }

    #[test]
    fn a_lone_session_stands_as_it_is_with_nothing_to_step_into() {
        let rows = top(&[row("P", "listening", 0, LIVE)]);
        let lone = found(&rows, "row:agents-top:P");
        assert_eq!(
            lone["kind"], "session",
            "a session alone is no orchestration"
        );
        assert_eq!(sides(lone), ["listening"], "it says what its own row says");
        assert!(
            !acts(lone).contains(&"verb:more"),
            "there's nothing to step into: {lone}"
        );
    }

    #[test]
    fn a_folds_members_are_inside_the_fold_and_a_session_with_some_steps_into_them() {
        let rows = top(&[
            row("O", "working", 0, LIVE),
            row("S", "working", 1, LIVE),
            row("T", "working", 2, LIVE),
            Shown::Fold(Fold {
                key: "agents:fold:O".into(),
                name: "2 listening".into(),
                glyph: "agent",
                hue: "program",
                depth: 1,
                held: vec![
                    row("L", "listening", 0, LIVE),
                    row("M", "listening", 0, LIVE),
                ],
            }),
        ]);
        let fold = found(&rows, "row:agents-top:fold:O");
        assert_eq!(fold["parent"], "row:agents-top:O", "the fold is inside O");
        assert_eq!(acts(fold), ["verb:more"], "the fold steps into its members");
        for key in ["row:agents-top:L", "row:agents-top:M"] {
            let member = found(&rows, key);
            assert_eq!(
                member["parent"], "row:agents-top:fold:O",
                "{key} is in the fold"
            );
            assert_eq!(member["depth"], 2, "{key} is two steps in");
        }
        assert_eq!(
            found(&rows, "row:agents-top:T")["parent"],
            "row:agents-top:S",
            "a session under one inside is inside that one"
        );
        assert_eq!(
            acts(found(&rows, "row:agents-top:S"))[..2],
            ["verb:details", "verb:more"],
            "one with sessions under it steps into them after a look"
        );
        assert_eq!(
            sides(found(&rows, "row:agents-top:O")),
            ["3 of 5 working", "about $604 in all"],
            "a fold's members are counted with the rest"
        );
    }
}
