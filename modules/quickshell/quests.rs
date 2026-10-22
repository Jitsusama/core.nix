//! The launcher's quests (launcher/Quests.qml): every quest in pi's quest
//! store, read from the front matter and first heading of its README, as one
//! JSON array on standard output. Each says its id, title, kind, parent,
//! status, priority, rank, when it was last updated, and the folder pi
//! would be opened in for it, chosen as the harness chooses a spawned
//! session's: the newest of its trees still there, then the newest of its
//! sessions' folders still there, then the quest's own folder.
//!
//! The store is the folder given as the only argument. A README that can't
//! be read, or has no front matter, is left out. It reads only as far as
//! the heading, since the rest is the quest's story.

use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::ExitCode;

/// What the launcher needs of one quest.
#[derive(Debug, Default, PartialEq)]
struct Quest {
    id: String,
    title: String,
    kind: String,
    parent: String,
    status: String,
    priority: String,
    rank: String,
    updated: String,
    trees: Vec<String>,
    sessions: Vec<(String, String)>,
}

/// A value as YAML writes a plain scalar, its quotes taken off and a null
/// read as nothing.
fn scalar(value: &str) -> String {
    let value = value.trim();
    let unquoted = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(value);
    if unquoted == "null" || unquoted == "~" {
        String::new()
    } else {
        unquoted.to_string()
    }
}

/// A list item's field, at any indent, with or without the dash that opens
/// the item: "  - path: x" and "    cwd: x" alike.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let line = line.trim_start();
    let line = line.strip_prefix("- ").unwrap_or(line);
    line.strip_prefix(name)?.strip_prefix(':')
}

/// Reads a README's front matter and title, line by line.
fn parse(lines: impl Iterator<Item = String>) -> Option<Quest> {
    let mut quest = Quest::default();
    let mut fences = 0;
    let mut list = String::new();
    for line in lines {
        if line == "---" {
            fences += 1;
            continue;
        }
        match fences {
            0 => return None,
            1 => {
                if !line.starts_with(' ') && !line.starts_with('-') {
                    let Some((key, value)) = line.split_once(':') else {
                        continue;
                    };
                    list = key.to_string();
                    let value = scalar(value);
                    match key {
                        "id" => quest.id = value,
                        "kind" => quest.kind = value,
                        "parent" => quest.parent = value,
                        "status" => quest.status = value,
                        "priority" => quest.priority = value,
                        "rank" => quest.rank = value,
                        "updated" => quest.updated = value,
                        _ => {}
                    }
                } else if list == "trees" {
                    if let Some(path) = field(&line, "path") {
                        quest.trees.push(scalar(path));
                    }
                } else if list == "sessions" {
                    // An item's id opens it; its folder and start follow.
                    if line.trim_start().starts_with("- ") {
                        quest.sessions.push((String::new(), String::new()));
                    }
                    if let Some(session) = quest.sessions.last_mut() {
                        if let Some(cwd) = field(&line, "cwd") {
                            session.0 = scalar(cwd);
                        } else if let Some(started) = field(&line, "started") {
                            session.1 = scalar(started);
                        }
                    }
                }
            }
            _ => {
                if let Some(title) = line.strip_prefix("# ") {
                    quest.title = title.trim().to_string();
                    break;
                }
            }
        }
    }
    (fences >= 2 && !quest.id.is_empty()).then_some(quest)
}

/// The folder pi opens in for a quest, as the harness picks a spawned
/// session's: its newest tree still there, its newest session's folder
/// still there, then the quest's own folder.
fn folder(quest: &Quest, own: &Path, exists: impl Fn(&str) -> bool) -> String {
    if let Some(tree) = quest.trees.iter().rev().find(|tree| exists(tree)) {
        return tree.clone();
    }
    let mut sessions: Vec<&(String, String)> = quest.sessions.iter().collect();
    // ISO 8601 times sort as text.
    sessions.sort_by(|a, b| b.1.cmp(&a.1));
    if let Some((cwd, _)) = sessions
        .into_iter()
        .find(|(cwd, _)| !cwd.is_empty() && exists(cwd))
    {
        return cwd.clone();
    }
    own.to_string_lossy().into_owned()
}

/// A string as JSON writes it.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json(quest: &Quest, folder: &str) -> String {
    let fields = [
        ("id", &quest.id),
        ("title", &quest.title),
        ("kind", &quest.kind),
        ("parent", &quest.parent),
        ("status", &quest.status),
        ("priority", &quest.priority),
        ("rank", &quest.rank),
        ("updated", &quest.updated),
    ];
    let mut out = String::from("{");
    for (name, value) in fields {
        out.push_str(&format!("{}:{},", quoted(name), quoted(value)));
    }
    out.push_str(&format!("{}:{}}}", quoted("folder"), quoted(folder)));
    out
}

fn main() -> ExitCode {
    let Some(store) = env::args().nth(1) else {
        eprintln!("usage: pi-quests <the quest store's folder>");
        return ExitCode::from(2);
    };
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(&store) {
        for entry in entries.flatten() {
            let own = entry.path();
            let Ok(file) = File::open(own.join("README.md")) else {
                continue;
            };
            let lines = BufReader::new(file).lines().map_while(Result::ok);
            if let Some(quest) = parse(lines) {
                let at = folder(&quest, &own, |path| Path::new(path).is_dir());
                found.push(json(&quest, &at));
            }
        }
    }
    let out = format!("[{}]\n", found.join(","));
    match io::stdout().lock().write_all(out.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> impl Iterator<Item = String> + '_ {
        text.lines().map(String::from)
    }

    const README: &str = "---
id: QEST-20261007-WCJQSG
kind: quest
parent: null
status: active
priority: driving
rank: 1
updated: 2026-10-22
aliases:
  - type: git-branch
    value: qest-20261007-wcjqsg
sessions:
  - id: one
    cwd: /old
    started: 2026-10-10T00:09:53.403Z
    status: detached
  - id: two
    cwd: \"/new\"
    started: 2026-10-12T18:32:23.032Z
    terminal:
      driverId: kitty
trees:
  - path: /first
    providerId: dev-tree
  - path: /second
---

# Bootstrap the laptop to replace the old one

## Summary
# Not the title
";

    #[test]
    fn a_readme_says_its_front_matter_and_title() {
        let quest = parse(lines(README)).unwrap();
        assert_eq!(quest.id, "QEST-20261007-WCJQSG");
        assert_eq!(quest.title, "Bootstrap the laptop to replace the old one");
        assert_eq!(quest.kind, "quest");
        assert_eq!(quest.parent, "");
        assert_eq!(quest.status, "active");
        assert_eq!(quest.priority, "driving");
        assert_eq!(quest.rank, "1");
        assert_eq!(quest.updated, "2026-10-22");
        assert_eq!(quest.trees, ["/first", "/second"]);
        assert_eq!(
            quest.sessions,
            [
                ("/old".to_string(), "2026-10-10T00:09:53.403Z".to_string()),
                ("/new".to_string(), "2026-10-12T18:32:23.032Z".to_string())
            ]
        );
    }

    #[test]
    fn a_file_without_front_matter_is_no_quest() {
        assert_eq!(parse(lines("# Just a heading\n")), None);
        assert_eq!(parse(lines("---\nkind: quest\n---\n# No id\n")), None);
    }

    #[test]
    fn the_newest_tree_still_there_comes_first() {
        let quest = parse(lines(README)).unwrap();
        let own = Path::new("/quests/QEST");
        assert_eq!(folder(&quest, own, |_| true), "/second");
        assert_eq!(folder(&quest, own, |path| path == "/first"), "/first");
    }

    #[test]
    fn then_the_newest_session_still_there_then_the_quest() {
        let mut quest = parse(lines(README)).unwrap();
        quest.trees.clear();
        let own = Path::new("/quests/QEST");
        assert_eq!(folder(&quest, own, |_| true), "/new");
        assert_eq!(folder(&quest, own, |path| path == "/old"), "/old");
        assert_eq!(folder(&quest, own, |_| false), "/quests/QEST");
    }

    #[test]
    fn its_words_are_written_as_json() {
        let quest = Quest {
            id: "Q".into(),
            title: "Say \"hi\"\\ now\n".into(),
            ..Quest::default()
        };
        assert_eq!(
            json(&quest, "/f"),
            "{\"id\":\"Q\",\"title\":\"Say \\\"hi\\\"\\\\ now\\n\",\"kind\":\"\",\"parent\":\"\",\
             \"status\":\"\",\"priority\":\"\",\"rank\":\"\",\"updated\":\"\",\"folder\":\"/f\"}"
        );
    }
}
