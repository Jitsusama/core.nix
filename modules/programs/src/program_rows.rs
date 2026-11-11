//! The Programs collection's rows, made ready for the shell (ready-rows.md,
//! the ready-row form): every program Quickshell lists, the open ones first
//! and then the rest, each A to Z as Order.js reads names, so a program
//! moves only as it opens or closes (collection-order.md). Each says what
//! it is and how many windows it has open, what Return does, and the
//! window Return goes to, worked out here from the windows daemon's
//! `window` records rather than by the shell at each step into Programs.
//! What the shell attaches to a row, its acts and its drawer, it makes
//! by the row's `id` (launcher/Applications.qml).

use std::collections::BTreeMap;

use ready_rows::order::by_name;
use serde_json::{Value, json};

/// What one program has open: how many windows, and the niri id of the
/// one looked at last, with when that was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Opened {
    pub windows: usize,
    pub latest: Option<u64>,
    looked: u64,
}

/// What each program has open, by its entry's id, from the windows
/// daemon's `window` records: each counts for its `owner`, and the one
/// with the largest `order` is the one looked at last.
#[must_use]
pub fn opened<'a>(windows: impl IntoIterator<Item = &'a Value>) -> BTreeMap<String, Opened> {
    let mut by: BTreeMap<String, Opened> = BTreeMap::new();
    for window in windows {
        let Some(owner) = window["owner"].as_str() else {
            continue;
        };
        let id = window["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("window:"))
            .and_then(|id| id.parse::<u64>().ok());
        let looked = window["order"].as_u64().unwrap_or(0);
        let one = by.entry(owner.to_owned()).or_default();
        one.windows += 1;
        if one.latest.is_none() || looked > one.looked {
            one.latest = id;
            one.looked = looked;
        }
    }
    by
}

/// The rows in the order shown, each without its `order`, which Kept
/// spaces: of every `program` record that's an application with a name,
/// leaving out those saying `NoDisplay` unless `shown` names them, as
/// another provider giving one acts does.
#[must_use]
pub fn rows(programs: &[Value], opened: &BTreeMap<String, Opened>, shown: &[String]) -> Vec<Value> {
    let mut listed: Vec<(&Value, Option<&Opened>)> = programs
        .iter()
        .filter(|program| listed(program, shown))
        .map(|program| (program, opened.get(text(&program["id"]))))
        .collect();
    listed.sort_by(|(one, one_open), (other, other_open)| {
        other_open
            .is_some()
            .cmp(&one_open.is_some())
            .then_with(|| by_name(text(&one["name"]), text(&other["name"])))
            .then_with(|| text(&one["id"]).cmp(text(&other["id"])))
    });
    listed
        .into_iter()
        .map(|(program, open)| row(program, open))
        .collect()
}

/// Whether Quickshell lists this program among its applications, or the
/// shell asked for it all the same.
fn listed(program: &Value, shown: &[String]) -> bool {
    let id = text(&program["id"]);
    program["application"] == true
        && !text(&program["name"]).is_empty()
        && (program["no_display"] != true || shown.iter().any(|one| one == id))
}

/// One program's row: what it is, lowered, follows its name on every
/// row, and its windows follow that while it has any.
fn row(program: &Value, open: Option<&Opened>) -> Value {
    let id = text(&program["id"]);
    let what = text(&program["generic_name"]).to_lowercase();
    let windows = match open.map_or(0, |open| open.windows) {
        0 => String::new(),
        1 => "1 window".to_owned(),
        many => format!("{many} windows"),
    };
    let count = [what.as_str(), windows.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    json!({
        "key": format!("row:programs:{id}"),
        "thing": format!("program:{id}"),
        "id": id,
        "name": text(&program["name"]),
        "what": what,
        "count": count,
        "verb": if open.is_some() { "go to" } else { "start" },
        "window": open.and_then(|open| open.latest),
    })
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(id: &str, name: &str, what: &str) -> Value {
        json!({
            "id": id, "name": name, "generic_name": what,
            "application": true, "no_display": false,
        })
    }

    fn window(id: u64, owner: &str, order: u64) -> Value {
        json!({ "key": format!("window:{id}"), "owner": owner, "order": order })
    }

    #[test]
    fn each_program_counts_its_windows_and_knows_the_one_looked_at_last() {
        let windows = [
            window(3, "kitty", 300),
            window(7, "kitty", 700),
            window(5, "kitty", 500),
            window(9, "zathura", 0),
            json!({ "key": "window:11", "order": 900 }),
        ];
        let by = opened(&windows);
        assert_eq!(
            by.len(),
            2,
            "a window with no program counts for none: {by:?}"
        );
        assert_eq!(by["kitty"].windows, 3);
        assert_eq!(by["kitty"].latest, Some(7), "the one looked at last");
        assert_eq!(
            by["zathura"].latest,
            Some(9),
            "one never looked at is still its latest"
        );
    }

    #[test]
    fn open_programs_come_first_then_the_rest_each_a_to_z() {
        let programs = [
            program("zathura", "Zathura", "Document Viewer"),
            program("kitty", "kitty", "Terminal emulator"),
            program("chrome", "Google Chrome", "Web Browser"),
            program("inkscape", "Inkscape", "Vector Graphics Editor"),
        ];
        let by = opened(&[window(1, "zathura", 1), window(2, "chrome", 2)]);
        let made = rows(&programs, &by, &[]);
        let ids: Vec<&str> = made.iter().map(|row| row["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["chrome", "zathura", "inkscape", "kitty"]);
    }

    #[test]
    fn a_row_says_what_it_is_its_windows_and_what_return_does() {
        let programs = [
            program("kitty", "kitty", "Terminal emulator"),
            program("zathura", "Zathura", "Document Viewer"),
            program("chrome", "Google Chrome", "Web Browser"),
        ];
        let by = opened(&[
            window(4, "kitty", 4),
            window(6, "kitty", 6),
            window(8, "chrome", 8),
        ]);
        let made = rows(&programs, &by, &[]);
        let of = |id: &str| made.iter().find(|row| row["id"] == id).unwrap();
        assert_eq!(of("kitty")["key"], "row:programs:kitty");
        assert_eq!(of("kitty")["thing"], "program:kitty");
        assert_eq!(of("kitty")["what"], "terminal emulator");
        assert_eq!(of("kitty")["count"], "terminal emulator \u{b7} 2 windows");
        assert_eq!(of("kitty")["verb"], "go to");
        assert_eq!(of("kitty")["window"], 6);
        assert_eq!(of("chrome")["count"], "web browser \u{b7} 1 window");
        assert_eq!(of("zathura")["count"], "document viewer");
        assert_eq!(of("zathura")["verb"], "start");
        assert_eq!(of("zathura")["window"], Value::Null);
    }

    #[test]
    fn a_hidden_program_is_listed_only_where_the_shell_asks_for_it() {
        let mut tuple = program("tuple", "Tuple", "");
        tuple["no_display"] = json!(true);
        let mut link = program("open-link", "Open Link", "");
        link["application"] = json!(false);
        let programs = [
            tuple,
            link,
            program("", "", ""),
            program("kitty", "kitty", ""),
        ];
        let ids = |shown: &[String]| -> Vec<String> {
            rows(&programs, &BTreeMap::new(), shown)
                .iter()
                .map(|row| row["id"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(
            ids(&[]),
            ["kitty"],
            "no link, no nameless entry, no hidden one"
        );
        assert_eq!(ids(&["tuple".to_owned()]), ["kitty", "tuple"]);
    }
}
