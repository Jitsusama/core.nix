//! The collection and the budgets cut into records, as the daemon's socket
//! tells them (`interface.rs`), each kind watched only while it's drawn:
//!
//! - `session:<id>`, what one pi session spent today, how its day went
//!   and what its helpers cost, and `quest:<id>`, a quest's family, for
//!   Agents;
//! - `day:all` and `day:agents`, how today stands, for the home row and
//!   Agents;
//! - `row:spend:<id>` and `row:spend-quiet:<id>`, the rows of the
//!   collection's lists (`now`, `findings`, `more` and `quiet`) ready to
//!   draw in the form every daemon serves its rows in, and
//!   `summary:spend` and `summary:spend-quiet`, how Spend stands as each
//!   says it, the first naming the summary's rows, made by `ready.rs`
//!   as the daemon serves them, for the launcher once it reads them;
//! - `report:<id>`, one report, and `words:periods`, `words:cuts` and
//!   `words:examples`, for Spend's answers while it's wanted;
//! - `budget:<name>`, how one budget stands, and `drew:<id>`, the budget a
//!   session drew on last, from what `budgets --json` says, for the
//!   launcher's drawers while it's open.
//!
//! The shell cuts collection.json and budgets.json into the same records
//! while the socket isn't there (modules/quickshell/SpendWatch.js), so
//! the two cuts change together.

use crate::collection::Collection;
use crate::standing::Standing;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The kinds of record the collection and the budgets are told as.
pub const KINDS: [&str; 9] = [
    "session", "quest", "day", "row", "report", "words", "budget", "drew", "summary",
];

/// The kinds cut from budgets.json rather than the collection.
pub const BUDGETS: [&str; 2] = ["budget", "drew"];

/// The kind a key names: `session` for `session:01a1...`.
pub fn kind_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    (KINDS.contains(&kind) && !rest.is_empty()).then_some(kind)
}

/// The kinds an ask names, as one or a list, or every kind when it names
/// none; or the first that isn't one.
pub fn kinds(said: &Value) -> Result<Vec<String>, String> {
    let named: Vec<String> = match said.get("kind") {
        None | Some(Value::Null) => return Ok(KINDS.iter().map(|k| (*k).to_owned()).collect()),
        Some(Value::String(kind)) => vec![kind.clone()],
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(other) => return Err(other.to_string()),
    };
    match named.iter().find(|kind| !KINDS.contains(&kind.as_str())) {
        Some(unknown) => Err(unknown.clone()),
        None => Ok(named),
    }
}

/// The collection as records, by key.
pub fn records(collection: &Collection) -> BTreeMap<String, Value> {
    serde_json::to_value(collection).map_or_else(|_| BTreeMap::new(), cut)
}

/// How the budgets stand as records, by key.
pub fn budget_records(standing: &Standing) -> BTreeMap<String, Value> {
    serde_json::to_value(standing).map_or_else(|_| BTreeMap::new(), |whole| cut_budgets(&whole))
}

/// budgets.json, as it's written, cut into records by key: each budget by
/// its name, and each session's last budget by its id. Its time and total
/// are left out, since nothing draws them and the time moves each pass.
pub fn cut_budgets(whole: &Value) -> BTreeMap<String, Value> {
    let mut made = BTreeMap::new();
    if let Some(budgets) = whole["budgets"].as_array() {
        for budget in budgets {
            let Some(name) = budget["name"].as_str() else {
                continue;
            };
            let key = format!("budget:{name}");
            made.insert(
                key.clone(),
                json!({"key": key, "kind": "budget", "budget": budget}),
            );
        }
    }
    if let Some(sessions) = whole["sessions"].as_object() {
        for (id, budget) in sessions {
            let key = format!("drew:{id}");
            made.insert(
                key.clone(),
                json!({"key": key, "kind": "drew", "budget": budget}),
            );
        }
    }
    made
}

/// The collection's lines by key, from its lists, so an act finds what
/// the line it names offers: the rows are told only as `ready.rs` serves
/// them, which say no command. A key in two lists is the first's, in the
/// order the lists are named here.
pub fn lines(whole: &Value) -> BTreeMap<String, Value> {
    let mut lines = BTreeMap::new();
    for list in ["findings", "more", "now", "quiet"] {
        for line in whole[list].as_array().into_iter().flatten() {
            if let Some(key) = line["key"].as_str() {
                lines.entry(key.to_owned()).or_insert_with(|| line.clone());
            }
        }
    }
    lines
}

/// A collection, as it's written, cut into records by key.
pub fn cut(whole: Value) -> BTreeMap<String, Value> {
    let Value::Object(mut whole) = whole else {
        return BTreeMap::new();
    };
    let mut made = BTreeMap::new();
    let mut take = |key: &str| match whole.remove(key) {
        Some(Value::Object(entries)) => entries,
        _ => Map::new(),
    };
    // A session's three figures are one record, so a change to any of them
    // is told as one.
    let fields = [
        ("sessions", "spent"),
        ("sessionsToday", "today"),
        ("helpers", "helpers"),
    ];
    for (from, field) in fields {
        for (id, value) in take(from) {
            let key = format!("session:{id}");
            let record = made
                .entry(key.clone())
                .or_insert_with(|| json!({"key": key, "kind": "session"}));
            record[field] = value;
        }
    }
    for (id, family) in take("families") {
        let mut record = json!({"key": format!("quest:{id}"), "kind": "quest"});
        if let Value::Object(fields) = family {
            for (field, value) in fields {
                record[field] = value;
            }
        }
        made.insert(format!("quest:{id}"), record);
    }
    for (id, report) in take("reports") {
        let key = format!("report:{id}");
        made.insert(
            key.clone(),
            json!({"key": key, "kind": "report", "report": report}),
        );
    }
    for (id, from) in [("all", "today"), ("agents", "agentsToday")] {
        let key = format!("day:{id}");
        let spent = whole.remove(from).unwrap_or(Value::Null);
        made.insert(
            key.clone(),
            json!({"key": key, "kind": "day", "spent": spent}),
        );
    }
    for id in ["periods", "cuts", "examples"] {
        let key = format!("words:{id}");
        let words = whole.remove(id).unwrap_or(Value::Array(Vec::new()));
        made.insert(
            key.clone(),
            json!({"key": key, "kind": "words", "words": words}),
        );
    }
    made
}
