//! Where Start New Work's words can be said (the launcher's Agents.qml):
//! the repositories the open quests work in, each once, in Quests' order,
//! named as a shell names them under home, served as one record beside
//! the rows so the launcher works none of it out (Y2YBTS on #5274). Home
//! itself is never one, since a session begun there has none of a
//! repository's skills, nor is the root or a folder that isn't a whole
//! path; and with home unknown none is, since home couldn't be told from
//! them.

use std::collections::BTreeSet;

use serde_json::{Value, json};

/// The record's key.
pub const KEY: &str = "folders:quests";

/// The record of the folders `rows` work in, each the folder its `held`
/// says, as `{ folder, name }`, for `home`.
pub fn record(rows: &[Value], home: &str) -> Value {
    let folders = of(
        rows.iter().filter_map(|row| row["held"]["folder"].as_str()),
        home,
    );
    json!({"key": KEY, "kind": "folders", "folders": folders})
}

/// Each of `folders` once, in their order, as `{ folder, name }`.
fn of<'a>(folders: impl Iterator<Item = &'a str>, home: &str) -> Vec<Value> {
    if !home.starts_with('/') {
        return Vec::new();
    }
    let home = plain(home);
    let mut seen = BTreeSet::new();
    folders
        .filter(|given| given.starts_with('/'))
        .map(plain)
        .filter(|folder| folder != &home && folder != "/")
        .filter(|folder| seen.insert(folder.clone()))
        .map(|folder| {
            let name = folder
                .strip_prefix(&home)
                .filter(|rest| rest.starts_with('/'))
                .map_or_else(|| folder.clone(), |rest| format!("~{rest}"));
            json!({"folder": folder, "name": name})
        })
        .collect()
}

/// `path` as plain steps: no empty step, no `.`, and `..` taking back the
/// step before, so two spellings of one folder read the same.
fn plain(path: &str) -> String {
    let mut steps = Vec::new();
    for step in path.split('/') {
        match step {
            ".." => {
                steps.pop();
            }
            "" | "." => {}
            step => steps.push(step),
        }
    }
    format!("/{}", steps.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(folders: &[&str], home: &str) -> Vec<Value> {
        of(folders.iter().copied(), home)
    }

    #[test]
    fn each_repository_once_in_the_quests_order_home_never() {
        assert_eq!(
            named(
                &[
                    "/home/j/src/core",
                    "/home/j",
                    "/home/j/src/harness",
                    "/home/j/src/core",
                    "",
                    "src/relative",
                ],
                "/home/j"
            ),
            [
                json!({"folder": "/home/j/src/core", "name": "~/src/core"}),
                json!({"folder": "/home/j/src/harness", "name": "~/src/harness"}),
            ]
        );
    }

    #[test]
    fn home_spelled_any_way_is_still_home() {
        assert_eq!(
            named(
                &["/home/j/", "/home//j", "/home/j/.", "/home/j/src/.."],
                "/home/j"
            ),
            Vec::<Value>::new()
        );
        assert_eq!(
            named(&["/home/j", "/home/j/src/core/"], "/home/j/"),
            [json!({"folder": "/home/j/src/core", "name": "~/src/core"})],
            "home said with a slash after it"
        );
    }

    #[test]
    fn a_folder_outside_home_reads_as_its_whole_path() {
        assert_eq!(
            named(&["/home/jo/src", "/srv/work"], "/home/j"),
            [
                json!({"folder": "/home/jo/src", "name": "/home/jo/src"}),
                json!({"folder": "/srv/work", "name": "/srv/work"}),
            ],
            "one only beginning as home does isn't under it"
        );
    }

    #[test]
    fn the_root_or_an_unknown_home_offers_nothing() {
        assert_eq!(named(&["/"], "/home/j"), Vec::<Value>::new());
        assert_eq!(named(&["/home/j/src/core"], ""), Vec::<Value>::new());
        assert_eq!(named(&["/home/j/src/core"], "home"), Vec::<Value>::new());
    }

    #[test]
    fn the_record_reads_each_rows_held_folder() {
        let rows = [
            json!({"key": "row:quests:A", "held": {"folder": "/home/j/src/core"}}),
            json!({"key": "row:quests:B", "held": {"folder": "/home/j"}}),
            json!({"key": "row:quests:C", "held": {}}),
        ];
        assert_eq!(
            record(&rows, "/home/j"),
            json!({"key": "folders:quests", "kind": "folders",
                "folders": [{"folder": "/home/j/src/core", "name": "~/src/core"}]})
        );
    }
}
