//! kitty's tabs as rows of Tabs (`tab_rows.rs`), beside Chrome's, made
//! from kitty's open list, a line a kitty, `<pid>`, a tab and its `ls` as
//! JSON, without the agents' panes (agents.rs), and the windows daemon's
//! `window` records. Each of a kitty's OS windows is niri's window of
//! that process and class, kitty's `wm_class` being niri's app id, paired
//! in the order both open them, so the sessions' kitty's hidden window,
//! of a class niri shows none of, has no tab here. An OS window niri
//! doesn't show isn't one Joel can go to, so its tabs aren't served.
//!
//! kitty keeps no time a tab was used that means anything as a time of
//! day (its `last_focused_at` is its own clock's), so a tab's is its OS
//! window's: when niri last focused it, moved from the monotonic clock
//! niri stamps it on to the wall's, Chrome's tabs' clock. The active tab
//! is used then, and the rest just before it in the order kitty used
//! them (Y2YBTS, 2026-11-12, 15e60549).

use std::collections::BTreeMap;

use chrome_session::one_line;
use serde_json::{Value, json};

use crate::tab_rows::Words;

/// The place these rows are of, Chrome's tabs' too.
const PLACE: &str = "tabs";

/// What kitty's tabs are made from.
pub struct Kitty<'a> {
    /// kitty's desktop entry id, which its rows are said `in`.
    pub app: &'a str,
    /// What its open list printed, a kitty a line.
    pub lines: Vec<&'a str>,
    /// What moves a time on the monotonic clock onto the wall's, in
    /// milliseconds.
    pub clock: i64,
    /// Joel's home, a folder in it said from `~`.
    pub home: &'a str,
}

/// Every tab of every kitty OS window niri shows, by the windows daemon's
/// records of its windows, each with when it was last used, in
/// milliseconds since 1970.
#[must_use]
pub fn rows(kitty: &Kitty, windows: &[&Value], words: &Words) -> Vec<(u64, Value)> {
    let mut rows = Vec::new();
    for line in &kitty.lines {
        let Some((pid, oses)) = heard(line) else {
            continue;
        };
        for (os, window) in on_screen(pid, &oses, windows) {
            let looked = wall(window["order"].as_u64().unwrap_or(0), kitty.clock);
            let tabs: Vec<&Value> = os["tabs"].as_array().into_iter().flatten().collect();
            for (before, tab) in (0..).zip(by_use(&tabs, &os["active_tab_history"])) {
                let row = row(kitty, pid, window, tab, words);
                rows.push((looked.saturating_sub(before), row));
            }
        }
    }
    rows
}

/// One kitty's process and its OS windows, from a line of its list; none
/// for a line that isn't whole, so one kitty answering badly hides only
/// itself.
fn heard(line: &str) -> Option<(u64, Vec<Value>)> {
    let (pid, list) = line.split_once('\t')?;
    let pid = pid.parse().ok().filter(|pid| *pid > 0)?;
    let Value::Array(oses) = serde_json::from_str(list).ok()? else {
        return None;
    };
    Some((pid, oses))
}

/// A time on the monotonic clock on the wall's.
fn wall(monotonic: u64, clock: i64) -> u64 {
    u64::try_from(
        i64::try_from(monotonic)
            .unwrap_or(i64::MAX)
            .saturating_add(clock),
    )
    .unwrap_or(0)
}

/// niri's window of each of one kitty's OS windows that niri shows: of
/// each class, both sides in the order they were opened, paired where
/// they count alike, and none of a class they don't, which can't be
/// paired surely.
fn on_screen<'a, 'b>(
    pid: u64,
    oses: &'a [Value],
    windows: &[&'b Value],
) -> Vec<(&'a Value, &'b Value)> {
    let mut classes: BTreeMap<&str, (Vec<&Value>, Vec<&Value>)> = BTreeMap::new();
    for os in oses {
        let class = os["wm_class"].as_str().unwrap_or("");
        classes.entry(class).or_default().0.push(os);
    }
    for window in windows.iter().filter(|window| window["pid"] == pid) {
        let class = window["opened"].as_str().unwrap_or("");
        if let Some((_, theirs)) = classes.get_mut(class) {
            theirs.push(window);
        }
    }
    let id = |one: &&Value| one["id"].as_u64().unwrap_or(0);
    let mut placed = Vec::new();
    for (_, (mut own, mut theirs)) in classes {
        if own.len() != theirs.len() {
            continue;
        }
        own.sort_by_key(id);
        theirs.sort_by_key(id);
        placed.extend(own.into_iter().zip(theirs));
    }
    placed
}

/// `items` in the order they were used: the active one, then kitty's own
/// history of them, newest first, then the rest in the tab bar's order,
/// as Ctrl+Tab walks them.
fn by_use<'a>(items: &[&'a Value], history: &Value) -> Vec<&'a Value> {
    let active = items.iter().find(|item| item["is_active"] == true);
    let ids = active
        .map(|item| &item["id"])
        .into_iter()
        .chain(history.as_array().into_iter().flatten().rev())
        .chain(items.iter().map(|item| &item["id"]));
    let mut order: Vec<&Value> = Vec::new();
    for id in ids {
        if let Some(item) = items.iter().find(|item| &item["id"] == id)
            && !order.iter().any(|kept| std::ptr::eq(*kept, *item))
        {
            order.push(item);
        }
    }
    order
}

/// A tab's row: named by its title, said by its folder and found by both.
/// Return goes to it, its OS window through niri and then the tab inside
/// it through kitty; Close the Tab closes it; and Copy the Title copies
/// its title, as a tab has no link. Each of the first two is an ask of
/// this daemon (tabs.rs), Go with the token his press gave, so niri moves
/// focus only on it, and Close shutting the launcher as it's asked. Its
/// drawer says its folder.
fn row(kitty: &Kitty, pid: u64, window: &Value, tab: &Value, words: &Words) -> Value {
    let id = tab["id"].as_u64().unwrap_or(0);
    let title = one_line(tab["title"].as_str().unwrap_or(""));
    let folder = shown(&folder_of(tab), kitty.home);
    let name = [title.as_str(), folder.as_str(), kitty.app]
        .into_iter()
        .find(|name| !name.is_empty())
        .unwrap_or_default();
    let keywords: Vec<&str> = [title.as_str(), folder.as_str()]
        .into_iter()
        .filter(|word| !word.is_empty())
        .collect();
    let niri = window["id"].as_u64().unwrap_or(0);
    let app = kitty.app;
    let mut row = json!({
        "key": format!("row:{PLACE}:{app}:{pid}:{id}"),
        "thing": format!("tab:{app}:{pid}:{id}"),
        "place": PLACE,
        "kind": "tab",
        "in": app,
        "name": name,
        "glyph": "windows",
        "hue": "program",
        "keywords": keywords.join(" "),
        "held": {"pid": pid, "tab": id, "window": niri},
        "acts": [
            {"key": "verb:tabs:go", "token": true,
             "ask": {"ask": "tab:go", "pid": pid, "tab": id, "window": niri}},
            {"key": "verb:tabs:close", "closes": true,
             "ask": {"ask": "tab:close", "pid": pid, "tab": id}},
            {"key": "verb:tabs:title", "copy": name},
        ],
    });
    let mut facts = Vec::new();
    if !folder.is_empty() {
        row["side"] = json!([{"said": folder, "keep": 1}]);
        facts.push(json!({"label": words.folder, "value": folder}));
    }
    row["drawer"] = words.drawer(app, name, json!(facts), json!([]));
    row
}

/// Where a tab's active pane is: the folder of what runs in front there,
/// its leader, the process whose group it is, else the pane's own.
fn folder_of(tab: &Value) -> String {
    let panes: Vec<&Value> = tab["windows"].as_array().into_iter().flatten().collect();
    let Some(pane) = by_use(&panes, &tab["active_window_history"])
        .first()
        .copied()
    else {
        return String::new();
    };
    let leader = pane["foreground_processes"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|one| one["pgrp"].is_u64() && one["pgrp"] == one["pid"]);
    leader
        .and_then(|one| one["cwd"].as_str())
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| pane["cwd"].as_str())
        .unwrap_or("")
        .to_owned()
}

/// A folder as it's said: from `~` within Joel's home.
fn shown(path: &str, home: &str) -> String {
    match path.strip_prefix(home) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => {
            format!("~{rest}")
        }
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: u64, active: bool, cwd: &str) -> Value {
        json!({"id": id, "is_active": active, "cwd": cwd, "foreground_processes": []})
    }

    fn tab(id: u64, active: bool, title: &str, panes: &[Value]) -> Value {
        json!({"id": id, "is_active": active, "title": title, "windows": panes})
    }

    fn os(id: u64, class: &str, tabs: &[Value], history: &[u64]) -> Value {
        json!({"id": id, "wm_class": class, "tabs": tabs, "active_tab_history": history})
    }

    fn window(id: u64, pid: u64, class: &str, order: u64) -> Value {
        json!({"key": format!("window:{id}"), "id": id, "pid": pid, "opened": class,
               "order": order})
    }

    fn line(pid: u64, oses: &[Value]) -> String {
        format!("{pid}\t{}", json!(oses))
    }

    /// kitty's list of these lines, and these windows.
    fn kitty<'a>(lines: &'a [String], windows: &'a [Value]) -> (Kitty<'a>, Vec<&'a Value>) {
        let kitty = Kitty {
            app: "kitty",
            lines: lines.iter().map(String::as_str).collect(),
            clock: 1_000_000,
            home: "/home/joel",
        };
        (kitty, windows.iter().collect())
    }

    fn rows((kitty, windows): &(Kitty, Vec<&Value>)) -> Vec<(u64, Value)> {
        super::rows(kitty, windows, &crate::tab_rows::tests::words())
    }

    fn keys(rows: &[(u64, Value)]) -> Vec<&str> {
        rows.iter()
            .map(|(_, row)| row["key"].as_str().unwrap())
            .collect()
    }

    #[test]
    fn each_tab_is_a_row_at_its_windows_focus_on_the_wall_clock_the_active_one_first() {
        let lines = [line(
            40,
            &[os(
                1,
                "kitty",
                &[
                    tab(1, false, "vim", &[pane(1, true, "/home/joel/src")]),
                    tab(2, true, "zsh", &[pane(2, true, "/tmp")]),
                    tab(3, false, "htop", &[pane(3, true, "/")]),
                ],
                &[1, 3],
            )],
        )];
        let windows = [window(7, 40, "kitty", 5_000)];
        let rows = rows(&kitty(&lines, &windows));
        assert_eq!(
            keys(&rows),
            [
                "row:tabs:kitty:40:2",
                "row:tabs:kitty:40:3",
                "row:tabs:kitty:40:1"
            ],
            "the tabs weren't the active one, then kitty's history newest first"
        );
        let times: Vec<u64> = rows.iter().map(|(at, _)| *at).collect();
        assert_eq!(
            times,
            [1_005_000, 1_004_999, 1_004_998],
            "the tabs weren't at their window's focus on the wall clock, the rest just before"
        );
    }

    #[test]
    fn a_tabs_row_is_said_in_kitty_by_its_folder_and_named_by_its_title() {
        let lines = [line(
            40,
            &[os(
                1,
                "kitty",
                &[tab(
                    5,
                    true,
                    "core.nix",
                    &[pane(9, true, "/home/joel/src/core.nix")],
                )],
                &[],
            )],
        )];
        let windows = [window(7, 40, "kitty", 1)];
        let rows = rows(&kitty(&lines, &windows));
        let row = &rows[0].1;
        assert_eq!(row["in"], "kitty", "a tab isn't said to be in kitty: {row}");
        assert_eq!(row["name"], "core.nix");
        assert_eq!(
            row["side"],
            json!([{"said": "~/src/core.nix", "keep": 1}]),
            "a tab isn't said by its folder from home: {row}"
        );
        assert_eq!(row["keywords"], "core.nix ~/src/core.nix");
        assert_eq!(row["thing"], "tab:kitty:40:5");
        assert_eq!(
            row["drawer"],
            json!({
                "eyebrow": [{"said": "in kitty", "keep": 2}, {"said": "tab", "keep": 1}],
                "title": "core.nix",
                "body": "",
                "facts": [{"label": "Folder", "value": "~/src/core.nix"}],
            }),
            "a tab's drawer isn't a tab in kitty, by its folder: {row}"
        );
    }

    #[test]
    fn go_to_the_tab_then_close_it_then_copy_its_title() {
        let lines = [line(
            40,
            &[os(
                1,
                "kitty",
                &[tab(5, true, "zsh", &[pane(9, true, "/")])],
                &[],
            )],
        )];
        let windows = [window(7, 40, "kitty", 1)];
        let rows = rows(&kitty(&lines, &windows));
        assert_eq!(
            rows[0].1["acts"],
            json!([
                {"key": "verb:tabs:go", "token": true,
                 "ask": {"ask": "tab:go", "pid": 40, "tab": 5, "window": 7}},
                {"key": "verb:tabs:close", "closes": true,
                 "ask": {"ask": "tab:close", "pid": 40, "tab": 5}},
                {"key": "verb:tabs:title", "copy": "zsh"},
            ]),
            "a tab's acts aren't Go, Close, then Copy the Title"
        );
    }

    #[test]
    fn os_windows_pair_with_niris_by_process_and_class_and_a_hidden_one_has_no_tabs() {
        let lines = [line(
            40,
            &[
                os(1, "pi-sessions-kept", &[tab(1, true, "kept", &[])], &[]),
                os(2, "kitty", &[tab(2, true, "a", &[])], &[]),
                os(3, "kitty", &[tab(3, true, "b", &[])], &[]),
            ],
        )];
        let windows = [
            window(20, 40, "kitty", 300),
            window(21, 40, "kitty", 100),
            window(22, 41, "kitty", 900),
        ];
        let rows = rows(&kitty(&lines, &windows));
        let placed: Vec<(&str, &Value)> = rows
            .iter()
            .map(|(_, row)| (row["name"].as_str().unwrap(), &row["held"]["window"]))
            .collect();
        assert_eq!(
            placed,
            [("a", &json!(20)), ("b", &json!(21))],
            "kitty's windows weren't paired with niri's of its process and class in order"
        );
    }

    #[test]
    fn a_line_that_isnt_whole_hides_only_its_kitty() {
        let lines = [
            "41\tnot json".to_owned(),
            line(40, &[os(1, "kitty", &[tab(1, true, "a", &[])], &[])]),
        ];
        let windows = [window(7, 40, "kitty", 1)];
        assert_eq!(
            keys(&rows(&kitty(&lines, &windows))),
            ["row:tabs:kitty:40:1"]
        );
    }

    #[test]
    fn a_tab_with_no_title_is_named_by_its_folder_and_a_leaders_folder_comes_first() {
        let leader = json!({"id": 1, "is_active": true, "cwd": "/home/joel",
        "foreground_processes": [
            {"pid": 8, "pgrp": 7, "cwd": "/elsewhere"},
            {"pid": 7, "pgrp": 7, "cwd": "/home/joel/notes"},
        ]});
        let lines = [line(
            40,
            &[os(1, "kitty", &[tab(1, true, " ", &[leader])], &[])],
        )];
        let windows = [window(7, 40, "kitty", 1)];
        let rows = rows(&kitty(&lines, &windows));
        assert_eq!(rows[0].1["name"], "~/notes");
    }
}
