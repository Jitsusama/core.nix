//! The harness's quests, read only: each README's front matter says what
//! the quest is (its kind, parent, status and dates), what it points at
//! (aliases, its pull requests among them) and which sessions served it.
//! Only those fields are read, with a reader for the plain YAML the
//! harness writes rather than all of YAML.

use rusqlite::params;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub parent: Option<String>,
    pub status: String,
    pub started: Option<String>,
    pub updated: Option<String>,
    /// How many pull requests it points at, from any host.
    pub prs: u32,
    pub sessions: Vec<String>,
}

/// Whether an alias type names a pull request or merge request, on any
/// host: github-pr, gitlab-mr and the like.
fn pull_request(kind: &str) -> bool {
    kind.ends_with("-pr") || kind.ends_with("-mr")
}

pub fn parse(text: &str) -> Option<Quest> {
    let body = text.strip_prefix("---\n")?;
    let (front, rest) = body.split_once("\n---")?;
    let mut quest = Quest::default();
    let mut list: Option<&str> = None;
    for line in front.lines() {
        if !line.starts_with(' ') && !line.starts_with('-') {
            list = None;
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = unquote(value.trim());
            let set =
                |value: &str| (value != "null" && !value.is_empty()).then(|| value.to_owned());
            match key {
                "id" => value.clone_into(&mut quest.id),
                "kind" => value.clone_into(&mut quest.kind),
                "parent" => quest.parent = set(value),
                "status" => value.clone_into(&mut quest.status),
                "started" => quest.started = set(value),
                "updated" => quest.updated = set(value),
                "aliases" | "sessions" if value.is_empty() => list = Some(key),
                _ => {}
            }
            continue;
        }
        let item = line.trim_start();
        match list {
            Some("aliases") => {
                if let Some(kind) = item.strip_prefix("- type:")
                    && pull_request(unquote(kind.trim()))
                {
                    quest.prs += 1;
                }
            }
            Some("sessions") => {
                // A session's own id, not its process's or terminal's.
                if let Some(id) = item.strip_prefix("- id:") {
                    quest.sessions.push(unquote(id.trim()).to_owned());
                }
            }
            _ => {}
        }
    }
    rest.lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or(&quest.id)
        .trim()
        .clone_into(&mut quest.title);
    (!quest.id.is_empty()).then_some(quest)
}

fn unquote(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

/// Every quest in the store: none where there's no store, and an error
/// where there is one that can't be read, so what's kept isn't wiped.
pub fn read(dir: &Path) -> Result<Vec<Quest>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries =
        std::fs::read_dir(dir).map_err(|error| format!("reading {}: {error}", dir.display()))?;
    let mut quests: Vec<Quest> = entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("README.md")).ok())
        .filter_map(|text| parse(&text))
        .collect();
    quests.sort_by(|one, other| one.id.cmp(&other.id));
    Ok(quests)
}

pub fn store(history: &crate::history::History, quests: &[Quest]) -> Result<(), String> {
    let db = &history.db;
    db.execute("DELETE FROM quests", [])
        .map_err(|error| error.to_string())?;
    let mut insert = db
        .prepare("INSERT OR REPLACE INTO quests VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)")
        .map_err(|error| error.to_string())?;
    for quest in quests {
        insert
            .execute(params![
                quest.id,
                quest.title,
                quest.kind,
                quest.parent,
                quest.status,
                quest.started,
                quest.updated,
                quest.prs,
                serde_json::to_string(&quest.sessions).unwrap_or_else(|_| "[]".into()),
            ])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// The quest at the top of one's family: its parent's parent, and so on.
pub fn root<'a>(quests: &'a [Quest], id: &'a str) -> &'a str {
    let mut at = id;
    for _ in 0..16 {
        match quests
            .iter()
            .find(|quest| quest.id == at)
            .and_then(|quest| quest.parent.as_deref())
        {
            Some(parent) => at = parent,
            None => break,
        }
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    const README: &str = "---
id: QEST-20261013-33NLRW
kind: subquest
parent: QEST-20261001-AAAAAA
status: active
priority: active
started: 2026-10-13
updated: 2026-10-14
aliases:
  - type: github-pr
    value: someone/thing#12
  - type: slack-message
    value: x
  - type: meteorite-pr
    value: \"12\"
sessions:
  - id: 01a1388b
    cwd: /home/someone
    process:
      hostId: here
      pid: 905923
  - id: 01b2
    cwd: /home/someone
primaryPlanId: PLAN-1
---
# AI Spend on the Desktop

Text.
";

    #[test]
    fn the_front_matter_reads() {
        let quest = parse(README).unwrap();
        assert_eq!(quest.id, "QEST-20261013-33NLRW");
        assert_eq!(quest.kind, "subquest");
        assert_eq!(quest.parent.as_deref(), Some("QEST-20261001-AAAAAA"));
        assert_eq!(quest.status, "active");
        assert_eq!(quest.updated.as_deref(), Some("2026-10-14"));
        assert_eq!(quest.prs, 2);
        assert_eq!(quest.sessions, vec!["01a1388b", "01b2"]);
        assert_eq!(quest.title, "AI Spend on the Desktop");
    }

    #[test]
    fn a_null_parent_is_none() {
        let quest = parse(&README.replace("parent: QEST-20261001-AAAAAA", "parent: null")).unwrap();
        assert_eq!(quest.parent, None);
    }

    #[test]
    fn a_family_has_one_root() {
        let quests = vec![
            Quest {
                id: "a".into(),
                ..Quest::default()
            },
            Quest {
                id: "b".into(),
                parent: Some("a".into()),
                ..Quest::default()
            },
            Quest {
                id: "c".into(),
                parent: Some("b".into()),
                ..Quest::default()
            },
        ];
        assert_eq!(root(&quests, "c"), "a");
        assert_eq!(root(&quests, "a"), "a");
    }
}
