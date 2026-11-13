//! Agents' top level as ready rows (ready-rows.md), one row for each
//! orchestration and the sessions inside it a step in, so Joel talks to
//! six orchestrators rather than reading fifty sessions (VKCF3Z's
//! launcher-shape ruling, 2026-11-11: "talking to 6 instead of 53 is the
//! load he asked to lose"). An orchestration's row acts first with Talk
//! to It, since he steers a team through its orchestrator; a session
//! inside one acts first with a look at its drawer, and is gone to only
//! where it needs his own hands.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use crate::figures::Item;
use crate::row::{Row, Shown, Words};
use crate::said::Said;
use crate::talks::{self, Talk};

/// The launcher's place for the top level.
pub const PLACE: &str = "agents-top";

/// What the top level reads beyond the rows: what an orchestration and
/// those under it spent today in all, by its orchestrator's address,
/// none while the screen is shared; the closed orchestrators' records,
/// the one last live first (closed.rs); and Joel's talks with each
/// session, by its address in capitals, the newest first (talks.rs).
pub struct Known<'a> {
    pub spent: &'a dyn Fn(&str) -> Option<String>,
    pub closed: &'a [Value],
    pub talks: &'a HashMap<String, Vec<Talk>>,
}

/// Each session's drawer again under the top level's place, so a watch
/// of the place reads a row's drawer as it reads the row, without
/// watching Agents' own too (Y2YBTS, 2026-11-11).
pub fn drawers(drawers: &[Value]) -> Vec<Value> {
    drawers
        .iter()
        .filter_map(|drawer| {
            let address = drawer["key"].as_str()?.strip_prefix("drawer:agents:")?;
            let mut again = drawer.clone();
            again["key"] = json!(format!("drawer:{PLACE}:{address}"));
            Some(again)
        })
        .collect()
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
        "drawer": format!("drawer:{PLACE}:{address}"), "acts": acts,
        "held": {"address": address, "also": row.also, "state": row.state, "gone": row.gone},
    })
}

/// An orchestration's own acts as rows a step in, ahead of its sessions:
/// Talk to It first, then the rest as its row orders them. A row stepping
/// into rows lists what's inside as its verbs (ReadyRows.qml), so these
/// are how Ctrl+] and the command line reach its Pause, Stop or Close, as
/// a meeting's acts are rows inside it (Y2YBTS, 2026-11-12). Each is named
/// and marked as the launcher's words say, and an ending one is sure by
/// its words, as the row's own act is.
fn acts_inside(row: &Row, words: &Words) -> Vec<Value> {
    let talks = row.acts.contains(&"talk");
    let ids = talks
        .then_some("talk")
        .into_iter()
        .chain(row.acts.iter().copied().filter(|id| *id != "talk"));
    act_rows(row, ids, words, 1)
}

/// The acts `ids` of `row`'s session as rows inside it, `depth` steps in.
fn act_rows<'a>(
    row: &Row,
    ids: impl Iterator<Item = &'a str>,
    words: &Words,
    depth: usize,
) -> Vec<Value> {
    let address = address(row);
    let parent = key_of(address);
    ids.map(|id| {
        json!({
            "key": format!("{parent}:{id}"), "thing": format!("verb:agent:{id}"),
            "place": PLACE, "parent": parent, "kind": "act", "depth": depth,
            "name": words.acts.word(id), "glyph": words.act_glyphs.word(id),
            "hue": "action", "side": [], "acts": [act(id, address)],
        })
    })
    .collect()
}

/// The acts a session inside an orchestration offers: Talk and Go To only
/// where it needs Joel.
fn member_acts<'a>(row: &'a Row) -> impl Iterator<Item = &'a str> {
    let needs = row.state == "needs";
    row.acts
        .iter()
        .copied()
        .filter(move |id| needs || !matches!(*id, "switch" | "talk"))
}

/// Joel's talks with each session of an orchestration that has any, as
/// rows inside it, after its acts: the orchestrator's own acts are rows
/// already where it has sessions, and any other's are made rows here, so
/// stepping in to read its talks leaves its acts in reach.
fn talks_inside(head: &Row, members: &[&Row], known: &Known, words: &Words, rows: &mut Vec<Value>) {
    for row in std::iter::once(head).chain(members.iter().copied()) {
        let Some(talks) = known.talks.get(&address(row).to_uppercase()) else {
            continue;
        };
        let key = key_of(address(row));
        let depth = rows
            .iter()
            .find(|one| one["key"] == key.as_str())
            .and_then(|one| one["depth"].as_u64())
            .and_then(|depth| usize::try_from(depth).ok())
            .unwrap_or(0)
            + 1;
        if std::ptr::eq(row, head) {
            if members.is_empty() {
                rows.extend(acts_inside(row, words));
            }
        } else {
            rows.extend(act_rows(row, member_acts(row), words, depth));
        }
        rows.extend(talks::rows(talks, &key, PLACE, depth, words));
    }
}

/// A session inside an orchestration: a look at its drawer first, gone to
/// or talked to only where it needs Joel (VKCF3Z: he acts at a leaf only
/// where it needs his own hands there), then what pauses, wakes or ends
/// it.
fn member(row: &Row, parent: &str, depth: usize) -> Value {
    let address = address(row);
    let mut acts = vec![peek()];
    acts.extend(member_acts(row).map(|id| act(id, address)));
    json!({
        "key": key_of(address), "thing": row.key, "place": PLACE, "parent": parent,
        "kind": "session", "name": row.name, "glyph": row.glyph, "hue": row.hue,
        "depth": depth, "side": items(&row.side_items), "note": row.note,
        "drawer": format!("drawer:{PLACE}:{address}"), "acts": acts,
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
/// under it steps into them. The closed orchestrators are one row at the
/// end (VKCF3Z, 2026-11-11), stepped into as the rest are.
pub fn rows(shown: &[Shown], known: &Known, words: &Words) -> Vec<Value> {
    let said = &words.agents;
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
            if !all.is_empty() {
                rows.extend(acts_inside(head, words));
            }
            members(&entries[1..], &[(0, key_of(address(head)))], &mut rows);
            talks_inside(head, &all, known, words, &mut rows);
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
            // After the look at its drawer, as an orchestration's is.
            let at = acts
                .iter()
                .position(|act| act["key"] == "verb:details")
                .map_or(1, |peek| peek + 1);
            acts.insert(at.min(acts.len()), more());
        }
    }
    rows.extend(closed(known.closed, words));
    rows
}

/// The closed orchestrators' row and each of them inside it, muted, as a
/// closed session's glyph is. Reopening one stays the shell's, which
/// opens its window, so each carries its record as `held.closed`, and
/// lists no act of the daemon's until the daemon reopens through the
/// node.
fn closed(records: &[Value], words: &Words) -> Vec<Value> {
    if records.is_empty() {
        return Vec::new();
    }
    let parent = key_of("closed");
    let count = records.len().to_string();
    let mut rows = vec![json!({
        "key": parent, "place": PLACE, "kind": "closed",
        "name": words.names.word("closed"), "glyph": "agent", "hue": "muted", "depth": 0,
        "side": [{"said": words.agents.fill("closedCount", &[("count", &count)]), "keep": 3}],
        "acts": [more()],
    })];
    for record in records {
        let one = &record["closed"];
        let address = one["address"].as_str().unwrap_or("");
        let spent = one["spent"].as_str().filter(|spent| !spent.is_empty());
        let side: Vec<Item> = [
            Some((words.agents.word("closed").to_owned(), 4)),
            spent.map(|spent| (words.agents.fill("allOf", &[("spent", spent)]), 3)),
        ]
        .into_iter()
        .flatten()
        .map(|(said, keep)| Item { said, keep })
        .collect();
        rows.push(json!({
            "key": key_of(&format!("closed:{address}")), "thing": format!("orchestrator:{address}"),
            "place": PLACE, "parent": parent, "kind": "closed-orchestrator",
            "name": one["name"], "glyph": "agent", "hue": "muted", "depth": 1,
            "side": items(&side), "note": one["title"], "acts": [],
            "held": {"address": address, "closed": one},
        }));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::{Fold, Ranked};

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
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
        closing(shown, &[])
    }

    fn closing(shown: &[Shown], closed: &[Value]) -> Vec<Value> {
        talking(shown, closed, &HashMap::new())
    }

    fn talking(
        shown: &[Shown],
        closed: &[Value],
        talks: &HashMap<String, Vec<Talk>>,
    ) -> Vec<Value> {
        let spent = |address: &str| (address == "O").then(|| "$604".to_owned());
        let known = Known {
            spent: &spent,
            closed,
            talks,
        };
        rows(shown, &known, &words())
    }

    /// One talk to each of `addresses`, Joel's words naming it, replied.
    fn talked(addresses: &[&str]) -> HashMap<String, Vec<Talk>> {
        addresses
            .iter()
            .map(|address| {
                let talk = Talk {
                    id: format!("t-{address}"),
                    words: format!("to {address}"),
                    stage: "replied".into(),
                    reply: "done".into(),
                    ..Talk::default()
                };
                ((*address).to_owned(), vec![talk])
            })
            .collect()
    }

    /// The names of the rows inside `parent`, in their order.
    fn inside<'a>(rows: &'a [Value], parent: &str) -> Vec<&'a str> {
        rows.iter()
            .filter(|row| row["parent"] == parent)
            .filter_map(|row| row["name"].as_str())
            .collect()
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
    fn an_orchestrators_talks_are_inside_it_after_its_acts_and_sessions() {
        let shown = [row("O", "working", 0, LIVE), row("A", "working", 1, LIVE)];
        let rows = talking(&shown, &[], &talked(&["O"]));
        assert_eq!(
            inside(&rows, "row:agents-top:O"),
            ["Talk", "Go To", "Pause", "Close", "A's quest", "to O"],
            "its acts, its session, then what Joel said to it"
        );
        let talk = found(&rows, "row:agents-top:O:talk:t-O");
        assert_eq!(
            (&talk["depth"], sides(talk)),
            (&json!(1), vec!["replied"]),
            "a step in, saying how it went: {talk}"
        );
        assert_eq!(
            inside(&rows, "row:agents-top:O:talk:t-O"),
            ["You Said", "How It Went"],
            "it steps into its parts"
        );
    }

    #[test]
    fn a_session_talked_to_steps_into_its_acts_then_its_talks() {
        let lone = [row("L", "working", 0, LIVE)];
        let rows = talking(&lone, &[], &talked(&["L"]));
        let head = found(&rows, "row:agents-top:L");
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
            "a lone orchestrator steps in after its drawer, as one with sessions does"
        );
        assert_eq!(
            inside(&rows, "row:agents-top:L"),
            ["Talk", "Go To", "Pause", "Close", "to L"],
            "its acts, still in reach, then its talk"
        );
        let shown = [row("O", "working", 0, LIVE), row("A", "working", 1, LIVE)];
        let rows = talking(&shown, &[], &talked(&["A"]));
        let member = found(&rows, "row:agents-top:A");
        assert_eq!(
            acts(member),
            [
                "verb:details",
                "verb:more",
                "verb:agent:pause",
                "verb:agent:close"
            ],
            "a session inside steps in after its drawer"
        );
        assert_eq!(
            inside(&rows, "row:agents-top:A"),
            ["Pause", "Close", "to A"],
            "the acts it offers inside, then its talk"
        );
        assert_eq!(
            found(&rows, "row:agents-top:A:talk:t-A")["depth"],
            json!(2),
            "a step inside its session"
        );
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
            head["drawer"], "drawer:agents-top:O",
            "its drawer is the session's own, served under the top level"
        );
    }

    #[test]
    fn an_orchestrations_own_acts_are_rows_inside_it_ahead_of_its_sessions() {
        let rows = top(&[
            row(
                "O",
                "working",
                0,
                &["switch", "talk", "pause", "stop", "close"],
            ),
            row("A", "working", 1, LIVE),
        ]);
        let inside: Vec<&Value> = rows
            .iter()
            .filter(|row| row["parent"] == "row:agents-top:O")
            .collect();
        let said: Vec<(&str, &str, &str)> = inside
            .iter()
            .map(|row| {
                (
                    row["kind"].as_str().unwrap_or(""),
                    row["name"].as_str().unwrap_or(""),
                    row["glyph"].as_str().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                ("act", "Talk", "reply"),
                ("act", "Go To", "windows"),
                ("act", "Pause", "pause"),
                ("act", "Stop", "stopped"),
                ("act", "Close", "close"),
                ("session", "A's quest", "agent"),
            ],
            "Talk first, then its own acts as its row orders them, each named as the \
             launcher names it, then its sessions"
        );
        let stop = found(&rows, "row:agents-top:O:stop");
        assert_eq!(
            stop["acts"],
            json!([{"key": "verb:agent:stop", "ask": {"ask": "stop", "address": "O"}}]),
            "each does its act on the orchestrator, by the act's own key: {stop}"
        );
        assert_eq!(stop["depth"], 1, "each is a step in: {stop}");
        assert_eq!(stop["thing"], "verb:agent:stop", "each is its act: {stop}");
        let alone = top(&[row("L", "listening", 0, LIVE)]);
        assert_eq!(
            alone.len(),
            1,
            "one with no sessions under it has no step, so its acts stay its row's: {alone:?}"
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
        // Its own acts inside it are another case's.
        let keys: Vec<&str> = rows
            .iter()
            .filter(|row| row["kind"] != "act")
            .filter_map(|row| row["key"].as_str())
            .collect();
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

    #[test]
    fn the_closed_orchestrators_are_one_row_at_the_end_each_inside_it() {
        let closed = [
            json!({"key": "closed:agents:BEACON", "closed": {"address": "BEACON",
                "name": "Beacon", "title": "Light the beacon", "spent": "$12"}}),
            json!({"key": "closed:agents:LAMPLT", "closed": {"address": "LAMPLT",
                "name": "Lamplight", "title": "Keep the lamp lit", "spent": ""}}),
        ];
        let rows = closing(&[row("P", "working", 0, LIVE)], &closed);
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:agents-top:P",
                "row:agents-top:closed",
                "row:agents-top:closed:BEACON",
                "row:agents-top:closed:LAMPLT"
            ],
            "Closed comes last, its orchestrators in the order served"
        );
        let all = found(&rows, "row:agents-top:closed");
        assert_eq!(
            all["name"], "Closed",
            "it's named from the launcher's words"
        );
        assert_eq!(sides(all), ["2 closed"], "it says how many it holds");
        assert_eq!(acts(all), ["verb:more"], "it steps into them");
        let beacon = found(&rows, "row:agents-top:closed:BEACON");
        assert_eq!(
            beacon["parent"], "row:agents-top:closed",
            "Beacon is inside it"
        );
        assert_eq!(
            sides(beacon),
            ["closed", "$12 in all"],
            "with what it spent"
        );
        assert_eq!(
            sides(found(&rows, "row:agents-top:closed:LAMPLT")),
            ["closed"],
            "no amount where none was said"
        );
        assert_eq!(
            beacon["held"]["closed"]["title"], "Light the beacon",
            "its record goes with it, for the shell to reopen it by"
        );
        assert!(
            !closing(&[], &[]).iter().any(|row| row["kind"] == "closed"),
            "with none closed there's no Closed row"
        );
    }

    #[test]
    fn each_drawer_is_served_again_under_the_top_level() {
        let again = drawers(&[
            json!({"key": "drawer:agents:O", "kind": "drawer", "drawer": {"title": "O"}}),
            json!({"key": "landed:agents:x"}),
        ]);
        assert_eq!(
            again,
            [json!({"key": "drawer:agents-top:O", "kind": "drawer", "drawer": {"title": "O"}})],
            "a session's drawer, keyed under the top level, and nothing else"
        );
    }
}
