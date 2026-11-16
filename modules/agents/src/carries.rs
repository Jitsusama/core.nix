//! Agents' Carried row, after the networks: each session an orchestrator
//! carried to another machine in the last day, newest first, so Joel sees
//! every carry made in his name's stead and how it stands (ISXEG8's fourth
//! condition, 26dbc61f). Each is made from the node's `carry` record,
//! which judges its state from what the node sees itself, never from what
//! a session or its orchestrator says (mycelium's PROTOCOL.md).
//!
//! A carry's row is its session's title, how it stands at its right, and
//! where it went and who carried it, and why, as its note. The node keeps
//! the day; the row holds no clock of its own.

use std::cmp::Reverse;
use std::collections::HashMap;

use serde_json::{Value, json};

use crate::networks;
use crate::row::Words;
use crate::top::PLACE;

/// The states a carry's record says, in the order its row's group counts
/// them: the word its record says, the words its row says it in, the words
/// a count of them is said in, how long it keeps its place on a row short
/// of room, and its hue.
const STATES: [(&str, &str, &str, u8, &str); 5] = [
    ("refused", "refused", "refusedCount", 5, "alert"),
    ("ordered", "ordered", "underWayCount", 4, ""),
    ("carrying", "carrying", "underWayCount", 4, ""),
    ("arrived", "arrived", "arrivedCount", 3, ""),
    ("back", "back", "backCount", 2, ""),
];

fn key_of(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

fn text<'a>(record: &'a Value, field: &str) -> &'a str {
    record[field].as_str().unwrap_or("")
}

/// Where a carry's state stands among `STATES`, or none for a word the
/// launcher doesn't know yet.
fn state_of(
    record: &Value,
) -> Option<&'static (&'static str, &'static str, &'static str, u8, &'static str)> {
    STATES.iter().find(|(word, ..)| record["state"] == *word)
}

/// A side item saying `said`, kept `keep`, in `hue` where it has one.
fn item(said: &str, keep: u8, hue: &str) -> Value {
    if hue.is_empty() {
        json!({"said": said, "keep": keep})
    } else {
        json!({"said": said, "keep": keep, "hue": hue})
    }
}

/// What the group's row says at its right: how many stand each way, the
/// refused first, those ordered and on their way as one.
fn group_side(carries: &[&Value], words: &Words) -> Vec<Value> {
    let said = &words.carried;
    let mut counted: Vec<(&str, u8, &str, usize)> = Vec::new();
    for one in carries {
        let Some((_, _, count, keep, hue)) = state_of(one) else {
            continue;
        };
        match counted.iter_mut().find(|(each, ..)| each == count) {
            Some(found) => found.3 += 1,
            None => counted.push((count, *keep, hue, 1)),
        }
    }
    counted.sort_by_key(|(_, keep, ..)| Reverse(*keep));
    counted
        .iter()
        .map(|(count, keep, hue, many)| {
            let words = if *many == 1 {
                format!("{count}One")
            } else {
                (*count).to_owned()
            };
            item(
                &said.fill(&words, &[("count", &many.to_string())]),
                *keep,
                hue,
            )
        })
        .collect()
}

/// The name a session's row goes by in `rows`, by its address.
fn named(rows: &HashMap<String, &Value>, address: &str) -> Option<String> {
    let row = rows.get(&address.to_lowercase())?;
    row["name"].as_str().map(str::to_owned)
}

/// A carry's row under `parent`: its session's title, how it stands, and
/// where it went, by whom and why, or why it was refused.
fn carry(record: &Value, parent: &str, rows: &HashMap<String, &Value>, words: &Words) -> Value {
    let said = &words.carried;
    let id = text(record, "key").trim_start_matches("carry:");
    let name =
        named(rows, text(record, "address")).unwrap_or_else(|| said.word("ended").to_owned());
    let by = named(rows, text(record, "by")).unwrap_or_else(|| said.word("someone").to_owned());
    let to = text(record, "to");
    let side = state_of(record)
        .map(|(_, word, _, keep, hue)| vec![item(said.word(word), *keep, hue)])
        .unwrap_or_default();
    let (lead, why) = if record["state"] == "refused" {
        (
            said.fill("refusedTo", &[("machine", to)]),
            text(record, "refused"),
        )
    } else {
        (
            said.fill("to", &[("machine", to), ("by", &by)]),
            text(record, "why"),
        )
    };
    let note = if why.is_empty() {
        lead
    } else {
        said.fill("note", &[("lead", &lead), ("why", why)])
    };
    json!({
        "key": key_of(&format!("carried:{id}")), "thing": format!("carry:{id}"),
        "place": PLACE, "parent": parent, "kind": "carry", "name": name, "glyph": "agent",
        "hue": "program", "depth": 1, "side": side, "note": note, "acts": back(record),
        "held": {"address": text(record, "address"), "state": text(record, "state"),
            "from": text(record, "from"), "to": to},
    })
}

/// What a carry's Return does: one that has arrived is carried back to
/// where it came from, asked of the node itself in its own shape, since
/// only the person's own login may carry a quest and the node tells that
/// by who asks (mycelium's PROTOCOL.md). The shell asks it, never this
/// daemon, which would ask in its own name. One still on its way, refused
/// or back offers nothing.
fn back(record: &Value) -> Value {
    let _ = record;
    json!([])
}

/// Every row of the Carried row, it first, from the node's `carry`
/// records, newest first, each session named as the tree served it in
/// `top`; none while the node keeps no carry.
pub fn rows(records: &[&Value], top: &[Value], words: &Words) -> Vec<Value> {
    if records.is_empty() {
        return Vec::new();
    }
    let mut newest: Vec<&Value> = records.to_vec();
    newest.sort_by_key(|record| Reverse(text(record, "at")));
    let parent = key_of("carried");
    let sessions = networks::sessions(top);
    let mut all = vec![json!({
        "key": parent, "place": PLACE, "kind": "carried",
        "name": words.names.word("carried"), "glyph": "elsewhere", "hue": "program",
        "depth": 0, "side": group_side(&newest, words), "note": "",
        "acts": [{"key": "verb:more", "step": "rows"}],
    })];
    all.extend(
        newest
            .iter()
            .map(|record| carry(record, &parent, &sessions, words)),
    );
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    /// A carry record as the node serves it: `address`'s session, carried
    /// from skiff to tern by KELP01 at `at`, standing `state`.
    fn record(address: &str, at: &str, state: &str) -> Value {
        let session = format!("s-{}", address.to_lowercase());
        json!({
            "key": format!("carry:{session}:{at}"), "kind": "carry", "at": at,
            "by": "KELP01", "session": session, "address": address,
            "quest": "QEST-20261115-HARBOR", "from": "skiff", "to": "tern",
            "why": "it needs the camera", "state": state,
        })
    }

    /// A session's row as the tree serves it at the top level.
    fn session(address: &str, name: &str, kind: &str) -> Value {
        json!({
            "key": key_of(address), "thing": format!("agent:{address}"), "place": PLACE,
            "kind": kind, "name": name, "glyph": "agent", "hue": "program", "depth": 0,
            "side": [], "note": "", "acts": [], "held": {"address": address},
        })
    }

    fn top() -> Vec<Value> {
        vec![
            session("KELP01", "Kelp Beds", "orchestration"),
            session("HARBOR", "Chart the Harbour", "session"),
        ]
    }

    fn said(row: &Value) -> Vec<&str> {
        row["side"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|one| one["said"].as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn no_carry_makes_no_row() {
        assert_eq!(rows(&[], &top(), &words()), Vec::<Value>::new());
    }

    #[test]
    fn a_carry_is_its_sessions_title_where_it_went_by_whom_and_why() {
        let arrived = record("HARBOR", "2026-11-16T01:00:00Z", "arrived");
        let made = rows(&[&arrived], &top(), &words());
        let keys: Vec<&str> = made.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:agents-top:carried",
                "row:agents-top:carried:s-harbor:2026-11-16T01:00:00Z"
            ],
            "the group, then the carry under it"
        );
        assert_eq!(made[0]["name"], "Carried");
        assert_eq!(made[0]["depth"], 0);
        assert_eq!(said(&made[0]), ["1 arrived"]);
        let carry = &made[1];
        assert_eq!(carry["parent"], "row:agents-top:carried");
        assert_eq!(
            carry["name"], "Chart the Harbour",
            "its session's title, never its id"
        );
        assert_eq!(said(carry), ["arrived"]);
        assert_eq!(
            carry["note"], "carried to tern by Kelp Beds \u{b7} it needs the camera",
            "where it went, by its orchestrator's name, and why"
        );
    }

    #[test]
    fn carries_come_newest_first_and_the_group_counts_each_way_refused_first() {
        let records = [
            record("HARBOR", "2026-11-16T01:00:00Z", "arrived"),
            record("TIDES1", "2026-11-16T03:00:00Z", "carrying"),
            record("REEFS1", "2026-11-16T02:00:00Z", "ordered"),
            record("SHOAL1", "2026-11-16T00:30:00Z", "arrived"),
            {
                let mut refused = record("KELPIE", "2026-11-16T00:10:00Z", "refused");
                refused["refused"] = json!("it holds a lease");
                refused
            },
        ];
        let made = rows(&records.iter().collect::<Vec<_>>(), &top(), &words());
        let addresses: Vec<&str> = made[1..]
            .iter()
            .filter_map(|row| row["held"]["address"].as_str())
            .collect();
        assert_eq!(
            addresses,
            ["TIDES1", "REEFS1", "HARBOR", "SHOAL1", "KELPIE"],
            "newest first"
        );
        assert_eq!(
            said(&made[0]),
            ["1 refused", "2 on their way", "2 arrived"],
            "the refused first, ordered and carrying as one"
        );
        assert_eq!(
            made[0]["side"][0]["hue"], "alert",
            "a refusal is said in alert"
        );
    }

    #[test]
    fn a_refusal_says_why_and_a_session_gone_from_the_tree_is_named_in_words() {
        let mut refused = record("GONE01", "2026-11-16T01:00:00Z", "refused");
        refused["refused"] = json!("it holds a lease");
        refused["by"] = json!("NOBODY");
        let made = rows(&[&refused], &top(), &words());
        let carry = &made[1];
        assert_eq!(
            carry["name"], "A Session That Ended",
            "no id where its row is gone"
        );
        assert_eq!(said(carry), ["refused"]);
        assert_eq!(carry["side"][0]["hue"], "alert");
        assert_eq!(
            carry["note"],
            "refused to go to tern \u{b7} it holds a lease"
        );
        let mut ordered = record("GONE01", "2026-11-16T01:00:00Z", "ordered");
        ordered["by"] = json!("NOBODY");
        ordered["why"] = json!("");
        let made = rows(&[&ordered], &top(), &words());
        assert_eq!(
            made[1]["note"], "carried to tern by an orchestrator",
            "and no why, none said"
        );
        assert_eq!(said(&made[1]), ["about to go"]);
    }

    #[test]
    fn a_state_the_launcher_does_not_know_yet_still_shows_the_carry() {
        let later = record("HARBOR", "2026-11-16T01:00:00Z", "paused");
        let made = rows(&[&later], &top(), &words());
        assert_eq!(made.len(), 2, "the carry is listed");
        assert_eq!(
            said(&made[1]),
            Vec::<&str>::new(),
            "saying no state it can't word"
        );
    }

    #[test]
    fn an_arrived_carry_is_carried_back_in_one_ask_of_the_node_and_no_other_is() {
        let arrived = record("HARBOR", "2026-11-16T01:00:00Z", "arrived");
        let made = rows(&[&arrived], &top(), &words());
        assert_eq!(
            made[1]["acts"],
            json!([{"key": "verb:carry:back",
                "ask": {"ask": "carry", "quest": "QEST-20261115-HARBOR", "to": "skiff"}}]),
            "back to where it came from, asked of the node as it stands"
        );
        assert_eq!(made[1]["held"]["quest"], "QEST-20261115-HARBOR");
        for state in ["ordered", "carrying", "refused", "back", "paused"] {
            let other = record("HARBOR", "2026-11-16T01:00:00Z", state);
            let made = rows(&[&other], &top(), &words());
            assert_eq!(made[1]["acts"], json!([]), "{state} offers nothing");
        }
        let mut questless = arrived.clone();
        questless["quest"] = json!("");
        let made = rows(&[&questless], &top(), &words());
        assert_eq!(made[1]["acts"], json!([]), "nor one naming no quest");
    }

    #[test]
    fn a_carried_session_is_named_as_its_network_cell_is() {
        // The same address map the networks' cells are named from.
        let tree = top();
        let rows = networks::sessions(&tree);
        assert_eq!(named(&rows, "harbor").as_deref(), Some("Chart the Harbour"));
    }
}
