//! Spend's rows ready for the launcher, in the one form every daemon
//! serves its rows in (the ready-rows crate, and Y2YBTS's ready-rows.md):
//! each line of the list the launcher shows, and of the one it shows
//! while the screen is shared, a record `row:<place>:<key>` whose `row`
//! holds what's drawn: its name, side and note, its glyph's and hue's
//! roles, how deep it sits and the row it's inside, its drawer and its
//! acts. The launcher places each by its `order` and draws it, working
//! nothing out (Joel, 2026-11-10: "the daemons need to be the data layer,
//! and they need to serve the launcher").
//!
//! What a row steps into comes after it, each a row of its own saying its
//! `parent`: for a finding Return acts on, copying it and marking it
//! expected, then the lines behind it. A row standing for a report says
//! which in `held`, and that report's rows stay its own record, told
//! while Spend is wanted. The summary's rows, which answer "spend", are
//! one record of their own kind, `summary:spend`, naming them in order.

use ready_rows::kept::Kept;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The launcher's place for the list it shows.
pub const PLACE: &str = "spend";
/// The launcher's place for the list it shows while the screen is
/// shared, which holds no amounts.
pub const QUIET: &str = "spend-quiet";
/// The key of the record naming the summary's rows.
pub const SUMMARY: &str = "summary:spend";

/// The launcher's words a ready row says beyond the collection's own,
/// spend's from its vocabulary (words.json): a name and a verb by id.
#[derive(Debug, Default)]
pub struct Words {
    words: BTreeMap<String, (String, String)>,
}

#[derive(Deserialize)]
struct Vocabulary {
    #[serde(default)]
    things: Vec<Thing>,
}

#[derive(Deserialize)]
struct Thing {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    verb: String,
}

impl Words {
    /// Spend's words from the vocabulary's text.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Words> {
        let vocabulary: Vocabulary = serde_json::from_str(text)?;
        let words = vocabulary
            .things
            .into_iter()
            .filter(|thing| thing.id.starts_with("spend:"))
            .map(|thing| (thing.id, (thing.name, thing.verb)))
            .collect();
        Ok(Words { words })
    }

    /// What the vocabulary names `id`, or the id itself where it names
    /// nothing, as the launcher says a word it doesn't hold.
    fn name(&self, id: &str) -> String {
        self.words
            .get(id)
            .map_or_else(|| id.to_owned(), |(name, _)| name.clone())
    }

    fn verb(&self, id: &str) -> String {
        self.words
            .get(id)
            .map(|(_, verb)| verb.clone())
            .unwrap_or_default()
    }
}

/// Spend's rows as last served, a place each, so each row keeps the order
/// it had where it can and one moving sends that one alone.
#[derive(Debug, Default)]
pub struct Served {
    places: BTreeMap<&'static str, Kept>,
}

impl Served {
    /// Takes a collection, as it's written, and answers every record its
    /// places serve now, by key.
    pub fn take(&mut self, whole: &Value, words: &Words) -> BTreeMap<String, Value> {
        let listed: BTreeMap<&str, &Value> = ["now", "findings", "more"]
            .iter()
            .flat_map(|list| lines(&whole[*list]))
            .map(|line| (text(line, "key"), line))
            .collect();
        let shown: Vec<&Value> = lines(&whole["shown"])
            .filter_map(|key| listed.get(key.as_str()?).copied())
            .collect();
        let summed: Vec<String> = lines(&whole["summaryRows"])
            .filter_map(Value::as_str)
            .map(|key| key_of(PLACE, key))
            .collect();
        let summary = json!({"key": SUMMARY, "kind": "summary", "rows": summed});
        let quiet: Vec<&Value> = lines(&whole["quiet"]).collect();
        let mut told = BTreeMap::new();
        for (place, of, beside) in [(PLACE, shown, vec![summary]), (QUIET, quiet, vec![])] {
            let kept = self.places.entry(place).or_default();
            kept.take(rows(place, &of, words), beside);
            told.extend(
                kept.all()
                    .map(|record| (text(record, "key").to_owned(), record.clone())),
            );
        }
        told
    }
}

/// The items of an array, or none of anything else.
fn lines(list: &Value) -> impl Iterator<Item = &Value> {
    list.as_array().into_iter().flatten()
}

/// A field's words, or "" where it says none.
fn text<'a>(of: &'a Value, field: &str) -> &'a str {
    of[field].as_str().unwrap_or_default()
}

fn key_of(place: &str, id: &str) -> String {
    format!("row:{place}:{id}")
}

/// The rows of one place, each line's after it, in the order shown.
pub fn rows(place: &str, lines: &[&Value], words: &Words) -> Vec<Value> {
    let mut laid = Laid {
        place,
        words,
        rows: Vec::new(),
        taken: BTreeSet::new(),
    };
    for line in lines {
        laid.lay(line, None, 0);
    }
    laid.rows
}

/// A place's rows as they're laid, and the ids taken in it.
struct Laid<'a> {
    place: &'a str,
    words: &'a Words,
    rows: Vec<Value>,
    taken: BTreeSet<String>,
}

impl Laid<'_> {
    /// The id a row of `of` takes: its own, or where a row behind another
    /// already took it, as one line can stand in two lists, the path to
    /// it.
    fn id(&mut self, of: &str, within: Option<&str>) -> String {
        let own = of.to_owned();
        let id = match within {
            Some(within) if self.taken.contains(&own) => format!("{within}/{of}"),
            _ => own,
        };
        self.taken.insert(id.clone());
        id
    }

    fn lay(&mut self, line: &Value, within: Option<&str>, depth: u64) {
        let id = self.id(text(line, "key"), within);
        let row = ready(self.place, &id, line, within, depth, self.words);
        self.rows.push(row);
        let inner = depth + 1;
        for doing in doings(line, self.words) {
            let own = format!("{}:{}", text(line, "key"), text(&doing, "kind"));
            let made = self.id(&own, Some(&id));
            self.rows.push(placed(doing, self.place, &made, &id, inner));
        }
        for behind in lines(&line["rows"]) {
            self.lay(behind, Some(&id), inner);
        }
    }
}

/// The theme's mark a row's glyph is, by its role: what AI use has cost
/// for how things stand, a trend for what was found, the warning or the
/// urgent mark for a finding that wants a look, all's fine for one marked
/// expected, the report's for a row standing for one, and signing in's.
fn glyph(role: &str) -> &'static str {
    match role {
        "alert" => "urgent",
        "warning" => "caution",
        "found" => "trend",
        "expected" => "fine",
        "report" => "report",
        "sign-in" => "signIn",
        _ => "cost",
    }
}

/// The theme's role a row's colour is, by its role; the warning is the
/// theme's caution, as every other warning wears.
fn hue(role: &str) -> &'static str {
    match role {
        "alert" => "alert",
        "warning" => "warning",
        "sign-in" => "action",
        "expected" => "muted",
        _ => "voice",
    }
}

/// The colour of a row whose role colours a mark of its own, a budget's,
/// a window's or a day's: muted at rest, the warning or the alert past
/// them (VKCF3Z, 2026-11-07).
fn marked_hue(role: &str) -> &'static str {
    match role {
        "alert" => "alert",
        "warning" => "warning",
        _ => "muted",
    }
}

/// What a line's own row is, ready.
fn ready(
    place: &str,
    id: &str,
    line: &Value,
    within: Option<&str>,
    depth: u64,
    words: &Words,
) -> Value {
    let of = text(line, "key");
    let role = text(line, "role");
    let mark = text(line, "mark");
    let marked = !mark.is_empty();
    let side = text(line, "side");
    let mut held = json!({});
    if marked {
        held["marked"] = json!(true);
    }
    let mut acts = Vec::new();
    if line["run"].is_array() {
        let verb = text(line, "verb");
        held["verb"] = json!(if verb.is_empty() { "run" } else { verb });
        acts.push(json!({"key": "verb:spend:run", "ask": asked("run", of)}));
    } else {
        acts.push(json!({"key": "verb:copy", "copy": text(line, "copy")}));
    }
    let report = text(line, "report");
    if !report.is_empty() {
        held["report"] = json!(report);
    }
    let mut row = json!({
        "key": key_of(place, id), "thing": format!("spend:{of}"), "place": place,
        "kind": of.split_once(':').map_or(of, |(kind, _)| kind),
        "name": text(line, "name"),
        "glyph": if marked { mark } else { glyph(role) },
        "hue": if marked { marked_hue(role) } else { hue(role) },
        "depth": depth,
        "side": if side.is_empty() { json!([]) } else { json!([{"said": side}]) },
        "acts": acts, "held": held,
    });
    if let Some(within) = within {
        row["parent"] = json!(key_of(place, within));
    }
    let note = text(line, "note");
    if !note.is_empty() {
        row["note"] = json!(note);
    }
    if line["drawn"].is_object() {
        row["drawer"] = drawer(&line["drawn"], text(line, "name"), words);
    }
    row
}

/// An act asked of spend-watch's socket for the line `of` keys.
fn asked(act: &str, of: &str) -> Value {
    json!({"ask": "act", "act": act, "line": of})
}

/// What a finding Return acts on offers before the lines behind it, each
/// a row of its own not yet placed: copying what it found, and marking it
/// expected where it can be.
fn doings(line: &Value, words: &Words) -> Vec<Value> {
    let of = text(line, "key");
    let role = text(line, "role");
    let mut made = Vec::new();
    if line["run"].is_array() && of.starts_with("finding:") {
        made.push(json!({
            "kind": "copy", "name": words.name("spend:copy"), "glyph": glyph(role),
            "hue": hue(role), "side": [],
            "acts": [{"key": "verb:copy", "copy": text(line, "copy")}],
            "held": {"figure": false},
        }));
    }
    if line["expect"].is_array() {
        made.push(json!({
            "kind": "expect", "name": words.name("spend:expect"), "glyph": "fine",
            "hue": "action", "side": [],
            "acts": [{"key": "verb:spend:expect", "ask": asked("expect", of)}],
            "held": {"verb": words.verb("spend:expect")},
        }));
    }
    made
}

/// A row made of no line of its own, placed behind `within`.
fn placed(mut row: Value, place: &str, id: &str, within: &str, depth: u64) -> Value {
    row["key"] = json!(key_of(place, id));
    row["thing"] = json!(format!("spend:{id}"));
    row["place"] = json!(place);
    row["parent"] = json!(key_of(place, within));
    row["depth"] = json!(depth);
    row
}

/// A drawer as the launcher draws it: its eyebrow, whether that names
/// something from the world, its title, its row's name unless it says
/// one, and each fact that says something, named by its label where it
/// carries one from the world, a window's, else by the vocabulary's name
/// for its id, with its meter and whether it says words.
fn drawer(drawn: &Value, name: &str, words: &Words) -> Value {
    let facts: Vec<Value> = lines(&drawn["facts"])
        .filter(|fact| !text(fact, "value").is_empty())
        .map(|fact| {
            let label = text(fact, "label");
            let named = if label.is_empty() {
                words.name(&format!("spend:{}", text(fact, "id")))
            } else {
                label.to_owned()
            };
            let mut said = json!({"name": named, "said": text(fact, "value")});
            if fact["figure"].is_object() {
                said["figure"] = fact["figure"].clone();
            }
            if fact["words"] == true {
                said["words"] = json!(true);
            }
            said
        })
        .collect();
    let title = text(drawn, "title");
    let mut made = json!({
        "eyebrow": text(drawn, "eyebrow"),
        "title": if title.is_empty() { name } else { title },
        "facts": facts, "body": "",
    });
    if drawn["names"] == true {
        made["names"] = json!(true);
    }
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(
            r#"{"things": [
                {"id": "spend:copy", "name": "Copy What It Found"},
                {"id": "spend:expect", "name": "It's Expected", "verb": "expect"},
                {"id": "spend:resets", "name": "Resets"},
                {"id": "agents", "name": "Agents"}
            ]}"#,
        )
        .unwrap()
    }

    fn finding() -> Value {
        json!({
            "key": "finding:burn", "name": "Burning Fast", "side": "3× usual",
            "note": "since 14:00", "copy": "Burning fast", "role": "warning",
            "run": ["spend-watch", "go", "s-1"], "verb": "go",
            "expect": ["spend-watch", "findings", "expect", "burn"],
            "rows": [{"key": "finding:burn/why", "name": "Why", "side": "",
                "copy": "why", "role": ""}],
        })
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter().map(|row| text(row, "key")).collect()
    }

    // A finding Return acts on is served ready: its glyph's and hue's
    // roles, its act asked of the socket, and behind it copying it and
    // marking it expected, then its own lines, each saying where it is.
    #[test]
    fn a_finding_is_served_ready_with_what_it_offers_behind_it() {
        let made = rows(PLACE, &[&finding()], &words());
        assert_eq!(
            keys(&made),
            [
                "row:spend:finding:burn",
                "row:spend:finding:burn:copy",
                "row:spend:finding:burn:expect",
                "row:spend:finding:burn/why",
            ]
        );
        let row = &made[0];
        assert_eq!(row["thing"], "spend:finding:burn");
        assert_eq!(row["kind"], "finding");
        assert_eq!(row["glyph"], "caution");
        assert_eq!(row["hue"], "warning");
        assert_eq!(row["side"], json!([{"said": "3× usual"}]));
        assert_eq!(row["note"], "since 14:00");
        assert_eq!(
            row["acts"],
            json!([{"key": "verb:spend:run",
                "ask": {"ask": "act", "act": "run", "line": "finding:burn"}}])
        );
        assert_eq!(row["held"], json!({"verb": "go"}));
        assert_eq!(row.get("parent"), None);
        let copy = &made[1];
        assert_eq!(copy["name"], "Copy What It Found");
        assert_eq!(
            copy["acts"],
            json!([{"key": "verb:copy", "copy": "Burning fast"}])
        );
        assert_eq!(copy["parent"], "row:spend:finding:burn");
        assert_eq!(copy["depth"], 1);
        let expect = &made[2];
        assert_eq!(expect["name"], "It's Expected");
        assert_eq!(expect["held"], json!({"verb": "expect"}));
        assert_eq!(expect["acts"][0]["ask"]["act"], "expect");
        let why = &made[3];
        assert_eq!(why["acts"], json!([{"key": "verb:copy", "copy": "why"}]));
        assert_eq!(why["side"], json!([]));
        assert_eq!(why["glyph"], "cost");
    }

    // A row whose role colours a mark of its own wears that mark, muted at
    // rest and the warning past it; a report's row says which report.
    #[test]
    fn a_marked_row_wears_its_mark_and_a_report_s_says_which() {
        let day = json!({"key": "today", "name": "Today", "side": "$4", "copy": "",
            "role": "", "mark": "agent"});
        let near = json!({"key": "cap:weekly", "name": "Weekly", "side": "90%", "copy": "",
            "role": "warning", "mark": "clock"});
        let report = json!({"key": "report:week/model", "name": "By Model", "side": "",
            "copy": "", "role": "report", "report": "week/model"});
        let made = rows(PLACE, &[&day, &near, &report], &words());
        let worn: Vec<(&str, &str)> = made
            .iter()
            .map(|row| (text(row, "glyph"), text(row, "hue")))
            .collect();
        assert_eq!(
            worn,
            [
                ("agent", "muted"),
                ("clock", "warning"),
                ("report", "voice")
            ]
        );
        assert_eq!(made[0]["held"], json!({"marked": true}));
        assert_eq!(made[2]["held"], json!({"report": "week/model"}));
    }

    // A drawer comes worded: each fact named by its label, else by the
    // vocabulary's name for its id, one that says nothing left out, and
    // its title its row's name unless it says one.
    #[test]
    fn a_drawer_comes_worded() {
        let window = json!({"key": "budget:claude:five-hours", "name": "Five Hours",
        "side": "", "copy": "", "role": "", "drawn": {
            "eyebrow": "in Claude Pro · window", "names": true, "facts": [
                {"id": "resets", "value": "16:20"},
                {"id": "warns", "value": ""},
                {"id": "window", "label": "Week", "value": "40%",
                    "figure": {"shape": "stroke", "values": [0.4], "warn": false}},
                {"id": "unnamed", "value": "said", "words": true},
            ]}});
        let made = rows(PLACE, &[&window], &words());
        assert_eq!(
            made[0]["drawer"],
            json!({"eyebrow": "in Claude Pro · window", "names": true, "title": "Five Hours",
            "body": "", "facts": [
                {"name": "Resets", "said": "16:20"},
                {"name": "Week", "said": "40%",
                    "figure": {"shape": "stroke", "values": [0.4], "warn": false}},
                {"name": "spend:unnamed", "said": "said", "words": true},
            ]})
        );
    }

    // A line listed in two places of one list keeps both rows, the one
    // behind another keyed by its path.
    #[test]
    fn a_line_standing_twice_keeps_both_rows() {
        let cap = json!({"key": "cap:day", "name": "Day", "side": "", "copy": "", "role": ""});
        let today = json!({"key": "today", "name": "Today", "side": "", "copy": "",
            "role": "", "rows": [cap.clone()]});
        let made = rows(PLACE, &[&cap, &today], &words());
        assert_eq!(
            keys(&made),
            [
                "row:spend:cap:day",
                "row:spend:today",
                "row:spend:today/cap:day"
            ]
        );
        assert_eq!(made[2]["thing"], "spend:cap:day");
    }

    // The collection is served as each place's rows, wrapped as every
    // daemon's are and spaced in the order shown, with the summary's rows
    // named in their own record; served again with one line moved, every
    // other row keeps its order.
    #[test]
    fn the_collection_is_served_a_place_each() {
        fn line(key: &str, side: &str) -> Value {
            json!({"key": key, "name": key, "side": side, "copy": "", "role": ""})
        }
        let mut whole = json!({
            "now": [line("today", "$4"), line("week", "$30")],
            "findings": [line("finding:burn", "")], "more": [],
            "quiet": [line("today", "")],
            "shown": ["finding:burn", "today", "week"], "summaryRows": ["today"],
        });
        let mut served = Served::default();
        let told = served.take(&whole, &words());
        assert_eq!(
            told.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "row:spend-quiet:today",
                "row:spend:finding:burn",
                "row:spend:today",
                "row:spend:week",
                "summary:spend",
            ]
        );
        let record = &told["row:spend:today"];
        assert_eq!(record["kind"], "row");
        assert_eq!(record["row"]["place"], "spend");
        let orders = |told: &BTreeMap<String, Value>| -> Vec<i64> {
            [
                "row:spend:finding:burn",
                "row:spend:today",
                "row:spend:week",
            ]
            .iter()
            .map(|key| told[*key]["row"]["order"].as_i64().unwrap_or_default())
            .collect()
        };
        let first = orders(&told);
        assert!(first.windows(2).all(|two| two[0] < two[1]), "{first:?}");
        assert_eq!(told[SUMMARY]["rows"], json!(["row:spend:today"]));
        whole["now"][1]["side"] = json!("$31");
        let again = served.take(&whole, &words());
        assert_eq!(orders(&again), first);
        assert_eq!(
            again["row:spend:week"]["row"]["side"],
            json!([{"said": "$31"}])
        );
    }

    // The daemon's socket serves the ready rows beside the rest of a pass's
    // records, to an ask for rows and the summary.
    #[test]
    fn the_socket_serves_the_ready_rows() {
        use std::io::{BufRead, BufReader};
        let hub = crate::interface::Hub::new(0).worded(words());
        hub.publish(&crate::collection::held(1), None);
        let (ours, theirs) = std::os::unix::net::UnixStream::pair().expect("a pair of sockets");
        let reading = BufReader::new(theirs.try_clone().expect("a reading half"));
        let asked = r#"{"id":"rows","ask":"list","kind":["row","summary"]}"#;
        std::thread::spawn(move || hub.serve(asked, reading, theirs));
        let mut line = String::new();
        BufReader::new(ours)
            .read_line(&mut line)
            .expect("an answer");
        let answer: Value = serde_json::from_str(&line).expect("a line of JSON");
        let told: BTreeMap<&str, &Value> = lines(&answer["records"])
            .map(|record| (text(record, "key"), record))
            .collect();
        assert_eq!(told["row:spend:held"]["row"]["name"], "Watch on hold");
        assert_eq!(told["row:spend-quiet:held"]["row"]["hue"], "warning");
        assert_eq!(told[SUMMARY]["rows"], json!(["row:spend:held"]));
    }
}
