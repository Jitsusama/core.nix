//! Quests worked on with no live plan. Every session keeps its quest's one
//! plan current, ticking items as they land, so a quest with a session at
//! work and no plan in think, draft or build is work nobody can follow or
//! steer. A quest nobody works on needs none, so only those with a live
//! session count: one whose pi still runs on this machine.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Bar, Reading};

/// A quest as the store keeps it: its README and each of its plans.
#[derive(Debug, Default)]
pub struct Quest {
    pub readme: String,
    pub plans: Vec<String>,
}

/// Everything this reading is read from.
#[derive(Debug, Default)]
pub struct Sources {
    /// The active quests, each with its plans.
    pub quests: Vec<Quest>,
    /// This machine's name, as a session's `hostId` gives it.
    pub host: String,
    /// The processes running now.
    pub running: BTreeSet<u32>,
}

/// How many quests worked on now have no live plan, of how many are.
#[must_use]
pub fn reading(s: &Sources) -> Reading {
    let worked: Vec<&Quest> = s.quests.iter().filter(|q| worked_on(q, s)).collect();
    let unplanned = worked
        .iter()
        .filter(|q| !q.plans.iter().any(|p| live(p)))
        .count();
    Reading::new(
        "{} quests worked on with no live plan",
        (!s.quests.is_empty()).then_some(count(unplanned)),
        "",
        "WCJQSG",
    )
    .bar(Bar::AtMost(0.0))
    .detail(Some(format!("of {} with a session at work", worked.len())))
    .from("the quest store")
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Whether one of the quest's sessions still runs here.
fn worked_on(quest: &Quest, s: &Sources) -> bool {
    sessions(front(&quest.readme))
        .into_iter()
        .any(|(host, pid)| host == s.host && s.running.contains(&pid))
}

/// Whether a plan is being worked to: thought about, drafted or built.
fn live(plan: &str) -> bool {
    field(front(plan), "stage").is_some_and(|stage| ["think", "draft", "build"].contains(&stage))
}

fn active(readme: &str) -> bool {
    field(front(readme), "status") == Some("active")
}

/// A document's frontmatter, between its opening `---` and the next.
fn front(text: &str) -> &str {
    text.strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---").map(|(front, _)| front))
        .unwrap_or_default()
}

/// A field at the frontmatter's top level, not one nested in a list.
fn field<'a>(front: &'a str, key: &str) -> Option<&'a str> {
    front
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
        .map(str::trim)
}

/// Each session's machine and process, from the README's `sessions` list,
/// where every item opens with `- id:` and holds a `hostId` and a `pid`.
fn sessions(front: &str) -> Vec<(&str, u32)> {
    let mut found = Vec::new();
    let mut host = None;
    let mut pid = None;
    let mut inside = false;
    for line in front.lines() {
        let trimmed = line.trim_start();
        if !line.starts_with(' ') {
            found.extend(host.take().zip(pid.take()));
            inside = line.starts_with("sessions:");
            continue;
        }
        if !inside {
            continue;
        }
        if trimmed.starts_with("- ") {
            found.extend(host.take().zip(pid.take()));
        }
        if let Some(value) = trimmed.strip_prefix("hostId: ") {
            host = Some(value.trim());
        } else if let Some(value) = trimmed.strip_prefix("pid: ") {
            pid = value.trim().parse().ok();
        }
    }
    found.extend(host.zip(pid));
    found
}

/// Reads the store's active quests and their plans, this machine's name
/// and its running processes: files only, with no command run.
#[must_use]
pub fn gather() -> Sources {
    let data = std::env::var_os("XDG_DATA_HOME").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"),
        PathBuf::from,
    );
    let store = data.join("pi/agentic-harness.pi/quest-workflow/quests");
    let quests = entries(&store)
        .filter_map(|quest| {
            let readme = fs::read_to_string(quest.join("README.md")).ok()?;
            active(&readme).then(|| Quest {
                plans: entries(&quest.join("plans"))
                    .filter_map(|plan| fs::read_to_string(plan).ok())
                    .collect(),
                readme,
            })
        })
        .collect();
    Sources {
        quests,
        host: fs::read_to_string("/proc/sys/kernel/hostname")
            .unwrap_or_default()
            .trim()
            .to_owned(),
        running: entries(Path::new("/proc"))
            .filter_map(|p| p.file_name()?.to_str()?.parse().ok())
            .collect(),
    }
}

fn entries(folder: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    fn readme(status: &str, sessions: &[(&str, u32)]) -> String {
        let mut text = format!("---\nid: QEST-1\nstatus: {status}\npriority: active\nsessions:\n");
        for (host, pid) in sessions {
            let _ = write!(
                text,
                "  - id: s\n    status: active\n    process:\n      hostId: {host}\n      pid: {pid}\n"
            );
        }
        text + "updated: 2026-11-12\n---\n# A Quest\n"
    }

    fn plan(stage: &str) -> String {
        format!("---\nid: PLAN-1\nkind: plan\nstage: {stage}\n---\n# A Plan\n")
    }

    fn sources(quests: Vec<Quest>) -> Sources {
        Sources {
            quests,
            host: "optimus".into(),
            running: [10, 20].into(),
        }
    }

    #[test]
    fn a_quest_at_work_with_no_live_plan_is_counted() {
        let s = sources(vec![
            Quest {
                readme: readme("active", &[("optimus", 10)]),
                plans: vec![plan("concluded")],
            },
            Quest {
                readme: readme("active", &[("optimus", 20)]),
                plans: vec![plan("build")],
            },
        ]);
        let reading = reading(&s);
        assert_eq!(reading.figure(), Some(1.0));
        assert!(reading.out(), "an unplanned quest at work was in spec");
    }

    #[test]
    fn a_quest_nobody_works_on_here_needs_no_plan() {
        let s = sources(vec![
            Quest {
                readme: readme("active", &[("optimus", 99)]),
                plans: vec![],
            },
            Quest {
                readme: readme("active", &[("penelope", 10)]),
                plans: vec![],
            },
        ]);
        assert_eq!(reading(&s).figure(), Some(0.0));
    }

    #[test]
    fn every_session_of_a_quest_is_read() {
        let text = readme("active", &[("penelope", 5), ("optimus", 20)]);
        assert_eq!(
            sessions(front(&text)),
            vec![("penelope", 5), ("optimus", 20)]
        );
    }

    #[test]
    fn a_plan_is_live_while_thought_about_drafted_or_built() {
        for stage in ["think", "draft", "build"] {
            assert!(live(&plan(stage)), "a plan in {stage} was not live");
        }
        assert!(!live(&plan("concluded")), "a concluded plan was live");
        assert!(
            !live("no frontmatter"),
            "a plan with no frontmatter was live"
        );
    }

    #[test]
    fn only_a_top_level_status_says_a_quest_is_active() {
        assert!(active(&readme("active", &[])), "an active quest was not");
        assert!(
            !active(&readme("concluded", &[("optimus", 10)])),
            "a session's status made a concluded quest active"
        );
    }

    #[test]
    fn with_no_store_the_reading_is_unread() {
        assert_eq!(reading(&Sources::default()).figure(), None);
    }
}
