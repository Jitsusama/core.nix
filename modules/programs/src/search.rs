//! Programs' search, ranked here rather than by the shell (Joel,
//! 2026-11-10: "the daemons need to be the data layer, and they need to
//! serve the launcher"): the `search` ask names a place and what's typed,
//! and is answered with the keys of the rows it finds, best first, and
//! the letters of each name to bring forward, as ready-rows.md says.
//! Each row is found by its name or by what else it's called, as the
//! shell's Provider.thing makes a row ready to match: its name, any word
//! of it written with a hyphen closed up, what it is, its comment, its
//! entry's keywords and what its key binds do, which the shell's watch
//! says (`also`), since niri's binds are the shell's. Those alike in score
//! keep the order the rows are served in, open programs first. What was
//! taken is searched by taken.rs, newest first as Capture's Recent lists
//! it.

use std::collections::BTreeMap;

use ready_rows::fuzzy::{self, Item, Typed, runs};
use serde_json::{Map, Value, json};

use super::{Daemon, Shared, taken};

/// The place of programs.
const PLACE: &str = "programs";

/// What a search of `place` for `words` is answered with, at most `most`
/// rows where the place lists no more.
pub fn answer(
    shared: &Shared,
    daemon: &Daemon,
    id: &Value,
    place: &str,
    words: &str,
    most: Option<usize>,
) -> Value {
    match place {
        PLACE => programs(daemon, id, words),
        taken::PLACE => taken::search(shared, daemon, id, words, most.unwrap_or(taken::FOUND)),
        _ => json!({
            "id": id, "words": words,
            "error": {"code": "unknown-place", "message": format!("no place {place:?} here")},
        }),
    }
}

/// Programs' rows that what's typed finds, best first.
fn programs(daemon: &Daemon, id: &Value, words: &str) -> Value {
    let typed = Typed::of(words);
    let programs: BTreeMap<&str, &Value> = daemon
        .catalogue
        .iter()
        .flatten()
        .map(|program| (text(&program["id"]), program))
        .collect();
    let also: BTreeMap<&str, &str> = daemon
        .watches
        .iter()
        .flat_map(|watch| watch.also.iter())
        .map(|(id, said)| (id.as_str(), said.as_str()))
        .collect();
    let mut rows: Vec<&Value> = daemon
        .rows
        .all()
        .filter(|record| record["kind"] == "row")
        .collect();
    rows.sort_by_key(|record| record["row"]["order"].as_i64().unwrap_or(i64::MAX));
    let mut found: Vec<(i32, &str, Item)> = Vec::new();
    for record in rows {
        let row = &record["row"];
        let id = text(&row["id"]);
        let name = Item::of(text(&row["name"]));
        let called = Item::of(&called(
            text(&row["name"]),
            programs.get(id).copied(),
            also.get(id),
        ));
        if let Some(score) = name.score(&typed).max(called.score(&typed)) {
            found.push((score, text(&record["key"]), name));
        }
    }
    // A stable sort, so those alike keep the order they're served in.
    found.sort_by_key(|(score, _, _)| std::cmp::Reverse(*score));
    let mut marks = Map::new();
    for (_, key, name) in &found {
        if let Some((_, marked)) = name.marked(&typed) {
            marks.insert((*key).to_owned(), json!(runs(&marked)));
        }
    }
    let keys: Vec<&str> = found.iter().map(|(_, key, _)| *key).collect();
    json!({"id": id, "words": words, "keys": keys, "marks": marks})
}

/// Everything a program is called, as the shell's Provider.thing joins
/// it: its name, each word of it written with a hyphen closed up, then
/// what it is, its comment, its keywords and what its binds do.
fn called(name: &str, program: Option<&Value>, bound: Option<&&str>) -> String {
    let field = |field: &str| program.map_or("", |program| text(&program[field]));
    let keywords = program
        .and_then(|program| program["keywords"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    let said: Vec<&str> = [field("generic_name"), field("comment")]
        .into_iter()
        .chain(keywords)
        .chain(bound.copied())
        .collect();
    let closed = fuzzy::words(name)
        .into_iter()
        .filter(|word| word.contains('-'))
        .map(|word| word.replace('-', ""));
    std::iter::once(name.to_owned())
        .chain(closed)
        .chain(std::iter::once(said.join(" ")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_called_by_all_the_shell_finds_it_by() {
        let program = json!({
            "generic_name": "Terminal emulator", "comment": "Fast",
            "keywords": ["shell", "tty"],
        });
        assert_eq!(
            called("kitty", Some(&program), Some(&"Open a terminal below")),
            "kitty Terminal emulator Fast shell tty Open a terminal below"
        );
        assert_eq!(
            called("Wi-Fi Radio", None, None),
            "Wi-Fi Radio wifi  ",
            "a hyphened word closed up, and the shell's join of two empty fields"
        );
    }
}
