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
//! keep the order the rows are served in, open programs first. Chrome's
//! open pages and its bookmarks are found the same way, by their name or
//! by their site and folder, so the shell ranks none of them either, and
//! Code's repositories by their name, owner, key and checkouts' paths. A
//! search `within` a row finds among the rows inside it instead, as a
//! checkout's files are found in Open a File by Name by their name or
//! folder, at most `most` of them, or `INSIDE` where the ask doesn't say,
//! since a large checkout holds thousands. What was taken is searched by
//! taken.rs, newest first as Capture's Recent lists it.

use std::collections::BTreeMap;

use programs::interface::Search;
use programs::{bookmark_rows, code_rows, tab_rows};
use ready_rows::fuzzy::{self, Item, Typed, runs};
use ready_rows::kept::Kept;
use serde_json::{Map, Value, json};

use super::{Daemon, Shared, taken};

/// The place of programs.
const PLACE: &str = "programs";

/// The most rows a search within a row answers with where the ask doesn't
/// say, more than a launcher shows at once.
const INSIDE: usize = 50;

/// What a search is answered with, at most `most` rows where the place
/// lists no more.
pub fn answer(shared: &Shared, daemon: &Daemon, id: &Value, asked: &Search) -> Value {
    let (place, words, most) = (asked.place.as_str(), asked.words.as_str(), asked.most);
    match place {
        PLACE => programs(daemon, id, words),
        tab_rows::PLACE => sites(&daemon.tabs, id, words),
        bookmark_rows::PLACE => sites(&daemon.bookmarks, id, words),
        code_rows::PLACE => match &asked.within {
            Some(within) => inside(&daemon.code, id, asked, within),
            None => sites(&daemon.code, id, words),
        },
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
        .also
        .iter()
        .map(|(id, said)| (id.as_str(), said.as_str()))
        .collect();
    let (keys, marks) = found(&daemon.rows, &typed, &Value::Null, |row| {
        let id = text(&row["id"]);
        called(text(&row["name"]), programs.get(id).copied(), also.get(id))
    });
    json!({"id": id, "words": words, "keys": keys, "marks": marks})
}

/// The open pages or bookmarks that what's typed finds, best first, each
/// by its name or by its keywords: its site, and a bookmark's folder.
fn sites(kept: &Kept, id: &Value, words: &str) -> Value {
    let typed = Typed::of(words);
    let (keys, marks) = found(kept, &typed, &Value::Null, named);
    json!({"id": id, "words": words, "keys": keys, "marks": marks})
}

/// The rows inside the row keyed `within` that what's typed finds, best
/// first, each by its name or its keywords, as a file is by its folder:
/// at most as many as the ask says, or `INSIDE`.
fn inside(kept: &Kept, id: &Value, asked: &Search, within: &str) -> Value {
    let typed = Typed::of(&asked.words);
    let (mut keys, mut marks) = found(kept, &typed, &json!(within), named);
    keys.truncate(asked.most.unwrap_or(INSIDE));
    marks.retain(|key, _| keys.contains(&key.as_str()));
    json!({"id": id, "words": asked.words, "keys": keys, "marks": marks})
}

/// What a row is found by where it says no more: its name and keywords.
fn named(row: &Value) -> String {
    format!("{} {}", text(&row["name"]), text(&row["keywords"]))
}

/// The keys of a place's rows that what's typed finds, best first, each
/// by its name or by all `called` says it's called, with the letters of
/// each name to bring forward. Those alike in score keep the order the
/// rows are served in. Only the rows whose parent is `parent` are looked
/// at: a place's own, for null, since what's typed at a place finds among
/// them and not those inside, as Code's files are inside Open a File by
/// Name; or those inside the row it keys.
fn found<'a>(
    kept: &'a Kept,
    typed: &Typed,
    parent: &Value,
    called: impl Fn(&Value) -> String,
) -> (Vec<&'a str>, Map<String, Value>) {
    let mut rows: Vec<&Value> = kept
        .all()
        .filter(|record| record["kind"] == "row" && record["row"]["parent"] == *parent)
        .collect();
    rows.sort_by_key(|record| record["row"]["order"].as_i64().unwrap_or(i64::MAX));
    let mut found: Vec<(i32, &str, Item)> = Vec::new();
    for record in rows {
        let row = &record["row"];
        let name = Item::of(text(&row["name"]));
        let also = Item::of(&called(row));
        if let Some(score) = name.score(typed).max(also.score(typed)) {
            found.push((score, text(&record["key"]), name));
        }
    }
    // A stable sort, so those alike keep the order they're served in.
    found.sort_by_key(|(score, _, _)| std::cmp::Reverse(*score));
    let mut marks = Map::new();
    for (_, key, name) in &found {
        if let Some((_, marked)) = name.marked(typed) {
            marks.insert((*key).to_owned(), json!(runs(&marked)));
        }
    }
    (found.into_iter().map(|(_, key, _)| key).collect(), marks)
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
