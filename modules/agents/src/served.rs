//! What the daemon serves, record by record, in the launcher's one form
//! for every daemon's rows (Y2YBTS's ready-rows.md, as RTOIMQ's Google
//! rows first served it): one kind, `row`, each record keyed
//! `row:agents:<id>` with the row's fields under `row`, placed by a
//! spaced `order` the shell never sorts. A fold holds its members' keys,
//! each member saying the fold it's in, and how the sessions stand
//! together is a record of its own kind, `summary`, read by its key, as
//! each session's drawer is, `drawer`, watched by key while it's shown.
//! Each making is held against the last, so a change sends only the
//! records that read differently and the keys gone, and a session whose
//! step alone moved sends nothing.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::figures::Item;
use crate::row::{Row, Shown};

/// The launcher's place the rows are in.
pub const PLACE: &str = "agents";
/// How far apart each row's `order` is set, so one row moving between
/// two takes a number between theirs and the rest keep their own.
pub const SPACING: i64 = 1024;
/// The key of the record saying how the sessions stand together, its
/// kind's own, since it's no row and is read where Agents' row is, home
/// included.
pub const SUMMARY: &str = "summary:agents";

/// The records as last served, by key.
#[derive(Debug, Default)]
pub struct Served {
    records: BTreeMap<String, Value>,
}

/// What a making moved: the records that read differently, whole, and
/// the keys gone.
#[derive(Debug, Default, PartialEq)]
pub struct Moved {
    pub changed: Vec<Value>,
    pub gone: Vec<String>,
}

impl Moved {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.gone.is_empty()
    }
}

/// One making: every row's record in the order they're shown, not yet
/// given its `order`, and the summary's.
#[derive(Debug, Default)]
pub struct Made {
    rows: Vec<Value>,
    summary: Value,
    /// Each session's drawer, a whole record, sent only to a watch that
    /// names its key (drawer.rs).
    drawers: Vec<Value>,
    /// Each session there is to reopen, a whole record (reopen.rs).
    reopens: Vec<Value>,
    /// Each session Recent lists, a whole record (recent.rs).
    recent: Vec<Value>,
    /// Each landing Landed lists, and how many landed each day, whole
    /// records (landed.rs).
    landed: Vec<Value>,
    /// Each closed orchestrator, a whole record (closed.rs).
    closed: Vec<Value>,
    /// Which window each session here is in, a whole record (windows.rs).
    windows: Vec<Value>,
    /// What each session asks Joel, a whole record (wants.rs).
    wants: Vec<Value>,
}

impl Made {
    /// The same, with each session's drawer beside its rows.
    #[must_use]
    pub fn with_drawers(self, drawers: Vec<Value>) -> Self {
        Self { drawers, ..self }
    }

    /// The same, with every session there is to reopen beside its rows.
    #[must_use]
    pub fn with_reopens(self, reopens: Vec<Value>) -> Self {
        Self { reopens, ..self }
    }

    /// The same, with every session Recent lists beside its rows.
    #[must_use]
    pub fn with_recent(self, recent: Vec<Value>) -> Self {
        Self { recent, ..self }
    }

    /// The same, with every landing Landed lists beside its rows.
    #[must_use]
    pub fn with_landed(self, landed: Vec<Value>) -> Self {
        Self { landed, ..self }
    }

    /// The same, with every closed orchestrator beside its rows.
    #[must_use]
    pub fn with_closed(self, closed: Vec<Value>) -> Self {
        Self { closed, ..self }
    }

    /// The same, with which window each session here is in.
    #[must_use]
    pub fn with_windows(self, windows: Vec<Value>) -> Self {
        Self { windows, ..self }
    }

    /// The same, with what each session asks Joel.
    #[must_use]
    pub fn with_wants(self, wants: Vec<Value>) -> Self {
        Self { wants, ..self }
    }

    /// The same, with the top level's rows after Agents' own (top.rs),
    /// each given its `order` as theirs are.
    #[must_use]
    pub fn with_top(mut self, top: Vec<Value>) -> Self {
        self.rows.extend(top);
        self
    }
}

fn key_of(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

fn items(items: &[Item]) -> Value {
    json!(
        items
            .iter()
            .map(|item| json!({"said": item.said, "keep": item.keep}))
            .collect::<Vec<_>>()
    )
}

/// A session's row in the launcher's form: its acts by their verbs' keys,
/// each with the ask that does it, the line the shell sends back (with
/// its own id) as it is; and what
/// only Agents reads, how it's found, its note, how it stands, what
/// Return does and itself ranked flat, held.
fn session(row: &Row, fold: Option<&str>) -> Value {
    let address = row.key.trim_start_matches("agent:");
    let acts: Vec<Value> = row.acts.iter().map(|id| row.act(id)).collect();
    let mut held = json!({
        "also": row.also, "note": row.note, "state": row.state, "gone": row.gone,
        "orchestrates": row.orchestrates,
        "returns": format!("verb:agent:{}", row.returns),
        "ranked": {"note": row.ranked.note, "side": items(&row.ranked.side_items),
            "belongs": row.ranked.belongs},
    });
    if let Some(fold) = fold {
        held["fold"] = json!(fold);
    }
    json!({
        "key": key_of(address), "row": row.key, "place": PLACE, "kind": "session",
        "name": row.name, "glyph": row.glyph, "hue": row.hue, "depth": row.depth,
        "side": items(&row.side_items), "acts": acts, "held": held,
    })
}

/// Every row of `shown`, a fold's members after it, in the order shown.
fn laid(shown: &[Shown], fold: Option<&str>, rows: &mut Vec<Value>) {
    for one in shown {
        match one {
            Shown::Session(row) => rows.push(session(row, fold)),
            Shown::Fold(folded) => {
                let key = key_of(folded.key.trim_start_matches("agents:"));
                let members: Vec<String> = folded
                    .held
                    .iter()
                    .map(|member| key_of(member.key().trim_start_matches("agent:")))
                    .collect();
                rows.push(json!({
                    "key": key, "row": folded.key, "place": PLACE, "kind": "fold",
                    "name": folded.name, "glyph": folded.glyph, "hue": folded.hue,
                    "depth": folded.depth, "side": [], "acts": [],
                    "held": {"members": members},
                }));
                laid(&folded.held, Some(&key), rows);
            }
        }
    }
}

/// A making of every row shown, and how the sessions stand together.
pub fn made(shown: &[Shown], summary: &str) -> Made {
    let mut rows = Vec::new();
    laid(shown, None, &mut rows);
    let summary = json!({"place": PLACE, "said": summary});
    Made {
        rows,
        summary,
        drawers: Vec::new(),
        reopens: Vec::new(),
        recent: Vec::new(),
        landed: Vec::new(),
        closed: Vec::new(),
        windows: Vec::new(),
        wants: Vec::new(),
    }
}

/// The `order` each row takes, in the order shown, from the order each
/// had before (`was`): the most rows that can keep theirs do (the longest
/// run still rising), and the rest take numbers spaced between their
/// neighbours, so one row moving changes one row. Where no room is left
/// between two, every row is spaced again.
pub fn spaced(was: &[Option<i64>]) -> Vec<i64> {
    let kept = rising(was);
    let mut order = vec![0; was.len()];
    let mut at = 0;
    let mut below: Option<i64> = None;
    for upto in kept.iter().copied().chain(std::iter::once(was.len())) {
        let above = was.get(upto).copied().flatten();
        let between = upto - at;
        let count = i64::try_from(between).unwrap_or(i64::MAX);
        let room = match (below, above) {
            (Some(low), Some(high)) => {
                (high - low > count).then(|| (low, (high - low) / (count + 1)))
            }
            (None, Some(high)) => Some((high - SPACING * (count + 1), SPACING)),
            (Some(low), None) => Some((low, SPACING)),
            (None, None) => Some((0, SPACING)),
        };
        let Some((from, step)) = room else {
            return (1..=i64::try_from(was.len()).unwrap_or(i64::MAX))
                .map(|n| n * SPACING)
                .collect();
        };
        for (step_at, one) in (1..).zip(at..upto) {
            order[one] = from + step * step_at;
        }
        if let Some(high) = above {
            order[upto] = high;
            below = Some(high);
        }
        at = upto + 1;
    }
    order
}

/// The places of the longest run of orders still rising, among those
/// that had one.
fn rising(was: &[Option<i64>]) -> Vec<usize> {
    // Patience: `tails[n]` is the place ending the best run of n + 1.
    let mut tails: Vec<usize> = Vec::new();
    let mut before = vec![None; was.len()];
    for (place, order) in was.iter().enumerate() {
        let Some(order) = order else { continue };
        let at = tails.partition_point(|tail| was[*tail].is_some_and(|tail| tail < *order));
        before[place] = at.checked_sub(1).map(|one| tails[one]);
        if at == tails.len() {
            tails.push(place);
        } else {
            tails[at] = place;
        }
    }
    let mut run = Vec::new();
    let mut place = tails.last().copied();
    while let Some(one) = place {
        run.push(one);
        place = before[one];
    }
    run.reverse();
    run
}

impl Served {
    /// A record as last served.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.records.get(key)
    }

    /// Every record as last served, in the order of their keys.
    pub fn all(&self) -> impl Iterator<Item = &Value> {
        self.records.values()
    }

    /// Takes a making in place of the last, each row given its `order`
    /// against what it had before, answering what moved.
    pub fn take(&mut self, made: Made) -> Moved {
        let was: Vec<Option<i64>> = made
            .rows
            .iter()
            .map(|row| {
                let key = row["key"].as_str().unwrap_or("");
                self.records
                    .get(key)
                    .and_then(|was| was["row"]["order"].as_i64())
            })
            .collect();
        let mut records = BTreeMap::new();
        for (mut row, order) in made.rows.into_iter().zip(spaced(&was)) {
            row["order"] = json!(order);
            let key = row["key"].as_str().unwrap_or("").to_owned();
            records.insert(key.clone(), json!({"key": key, "kind": "row", "row": row}));
        }
        records.insert(
            SUMMARY.to_owned(),
            json!({"key": SUMMARY, "kind": "summary", "summary": made.summary}),
        );
        let wholes = made
            .drawers
            .into_iter()
            .chain(made.reopens)
            .chain(made.recent)
            .chain(made.landed)
            .chain(made.closed)
            .chain(made.windows)
            .chain(made.wants);
        for whole in wholes {
            if let Some(key) = whole["key"].as_str() {
                records.insert(key.to_owned(), whole);
            }
        }
        let gone = self
            .records
            .keys()
            .filter(|key| !records.contains_key(*key))
            .cloned()
            .collect();
        let changed = records
            .iter()
            .filter(|(key, record)| self.records.get(*key) != Some(record))
            .map(|(_, record)| record.clone())
            .collect();
        self.records = records;
        Moved { changed, gone }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::{Fold, Ranked};

    fn row(address: &str, side: &str) -> Shown {
        Shown::Session(Box::new(Row {
            key: format!("agent:{address}"),
            name: address.to_owned(),
            also: String::new(),
            glyph: "agent",
            hue: "program",
            side: side.to_owned(),
            side_items: vec![Item {
                said: side.to_owned(),
                keep: 3,
            }],
            note: String::new(),
            depth: 0,
            orchestrates: false,
            gone: false,
            state: "working",
            returns: "switch",
            acts: vec!["switch", "talk"],
            leaves: String::new(),
            ranked: Ranked {
                note: String::new(),
                side: side.to_owned(),
                side_items: Vec::new(),
                belongs: String::new(),
            },
        }))
    }

    fn fold(held: Vec<Shown>) -> Shown {
        Shown::Fold(Fold {
            key: "agents:fold:O".into(),
            name: "2 listening".into(),
            glyph: "agent",
            hue: "program",
            depth: 1,
            held,
        })
    }

    fn orders(served: &Served, keys: &[&str]) -> Vec<i64> {
        keys.iter()
            .map(|key| {
                served
                    .get(&key_of(key))
                    .and_then(|one| one["row"]["order"].as_i64())
                    .unwrap_or(-1)
            })
            .collect()
    }

    #[test]
    fn a_row_is_served_in_the_launchers_form() {
        let mut served = Served::default();
        served.take(made(&[row("P", "working")], "1 working"));
        let record = served.get("row:agents:P").expect("its row");
        assert_eq!(record["kind"], "row");
        let row = &record["row"];
        assert_eq!(
            (row["row"].as_str(), row["place"].as_str()),
            (Some("agent:P"), Some("agents"))
        );
        assert_eq!(row["side"], json!([{"said": "working", "keep": 3}]));
        assert_eq!(
            row["acts"][1],
            json!({"key": "verb:agent:talk", "ask": {"ask": "talk", "address": "P"}})
        );
        assert_eq!(row["held"]["returns"], "verb:agent:switch");
        assert_eq!(row["order"], SPACING);
        assert_eq!(
            served
                .get(SUMMARY)
                .map(|one| one["summary"]["said"].clone()),
            Some(json!("1 working"))
        );
    }

    #[test]
    fn a_fold_holds_its_members_keys_and_each_says_its_fold() {
        let mut served = Served::default();
        served.take(made(
            &[row("O", "working"), fold(vec![row("A", ""), row("B", "")])],
            "",
        ));
        let folded = &served.get("row:agents:fold:O").expect("the fold")["row"];
        assert_eq!(
            folded["held"]["members"],
            json!(["row:agents:A", "row:agents:B"])
        );
        assert_eq!(
            served
                .get("row:agents:A")
                .map(|a| a["row"]["held"]["fold"].clone()),
            Some(json!("row:agents:fold:O"))
        );
        assert_eq!(
            orders(&served, &["O", "fold:O", "A", "B"]),
            [1024, 2048, 3072, 4096]
        );
    }

    #[test]
    fn only_what_reads_differently_is_sent_again() {
        let mut served = Served::default();
        let first = served.take(made(&[row("O", "working"), row("P", "")], ""));
        assert_eq!(
            first.changed.len(),
            3,
            "both rows and the summary, the first time"
        );
        assert!(
            served
                .take(made(&[row("O", "working"), row("P", "")], ""))
                .is_empty(),
            "a making that reads the same sends nothing"
        );
        let moved = served.take(made(&[row("O", "listening")], ""));
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(keys, [json!("row:agents:O")]);
        assert_eq!(moved.gone, ["row:agents:P"]);
    }

    #[test]
    fn one_row_moving_sends_that_row_alone() {
        let mut served = Served::default();
        served.take(made(&[row("A", ""), row("B", ""), row("C", "")], ""));
        let moved = served.take(made(&[row("C", ""), row("A", ""), row("B", "")], ""));
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(
            keys,
            [json!("row:agents:C")],
            "C moved to the front, A and B kept theirs"
        );
        assert_eq!(orders(&served, &["C", "A", "B"]), [0, 1024, 2048]);
        served.take(made(
            &[row("C", ""), row("D", ""), row("A", ""), row("B", "")],
            "",
        ));
        assert_eq!(
            orders(&served, &["C", "D", "A", "B"]),
            [0, 512, 1024, 2048],
            "D between"
        );
    }

    #[test]
    fn spacing_keeps_the_longest_rising_run_and_spaces_again_when_full() {
        assert_eq!(spaced(&[None, None]), [1024, 2048]);
        assert_eq!(
            spaced(&[Some(5), Some(3), Some(4), None]),
            [-1021, 3, 4, 1028]
        );
        assert_eq!(
            spaced(&[Some(1), None, Some(2)]),
            [1024, 2048, 3072],
            "no room between"
        );
    }
}
