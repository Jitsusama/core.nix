//! Quest workspaces: scratch kept beside a quest, spent once it's over.

use crate::clock::{Awake, local};
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use serde::Deserialize;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

/// A quest's workspace and what the quest's README says of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    pub path: PathBuf,
    pub status: String,
    /// The end of the day the quest was last updated, in wall seconds.
    pub finished: f64,
    /// When the workspace appeared.
    pub born: f64,
}

static QUEST_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^QEST-\d{8}-[0-9A-Z]{6}$").unwrap());
static FIELD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(status|updated):[ \t]*(\S+)[ \t]*$").unwrap());
static DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap());

/// The end of a `YYYY-MM-DD` day in local time. A README has no
/// conclusion date, so its last update stands in, and any later edit only
/// keeps the workspace longer.
fn end_of(day: &str) -> Option<f64> {
    let found = DATE.captures(day)?;
    let field = |n: usize| found[n].parse::<i32>().ok();
    local([field(1)?, field(2)?, field(3)? + 1, 0, 0, 0])
}

/// When a directory appeared: its birth time where the filesystem keeps
/// one, else its change time, which only makes it younger.
fn born(path: &Path) -> Option<f64> {
    let meta = fs::symlink_metadata(path).ok()?;
    let at = meta.created().or_else(|_| meta.modified()).ok()?;
    Some(at.duration_since(UNIX_EPOCH).ok()?.as_secs_f64())
}

/// Every workspace named for a quest whose README can be read and says
/// both its status and when it was updated. One with no readable quest is
/// left alone: the quests may be missing, not the quest.
pub fn find(root: &Path, quests: &Path) -> Vec<Workspace> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(id) = name.to_str().filter(|name| QUEST_ID.is_match(name)) else {
            continue;
        };
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(quests.join(id).join("README.md")) else {
            continue;
        };
        let Some(frontmatter) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split("\n---").next())
        else {
            continue;
        };
        let (mut status, mut updated) = (None, None);
        for field in FIELD.captures_iter(frontmatter) {
            let slot = if &field[1] == "status" {
                &mut status
            } else {
                &mut updated
            };
            *slot = Some(field[2].to_owned());
        }
        let (Some(status), Some(updated)) = (status, updated) else {
            continue;
        };
        let path = entry.path();
        let (Some(finished), Some(born)) = (end_of(&updated), born(&path)) else {
            continue;
        };
        found.push(Workspace {
            path,
            status,
            finished,
            born,
        });
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// The candidate a finished quest's workspace makes, idle for the awake
/// time since the last write in it, or since it appeared.
/// A quest retired before its workspace appeared has nothing to keep.
/// How deep a workspace is searched for linked worktrees, and how many
/// entries are read.
const LINK_DEPTH: usize = 8;
const LINK_BUDGET: usize = 50_000;

/// The repositories outside a workspace that have worktrees linked into
/// it: each `.git` file names `<repository>/worktrees/<name>`. Their
/// records outlive the workspace, keeping its branches from being checked
/// out elsewhere, until pruned. Repositories inside go with it.
pub fn linked(workspace: &Path) -> Vec<PathBuf> {
    let (Ok(top), Ok(inside)) = (fs::symlink_metadata(workspace), workspace.canonicalize()) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = Vec::new();
    let mut budget = LINK_BUDGET;
    let mut pending = vec![(workspace.to_path_buf(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(listing) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in listing.filter_map(Result::ok) {
            if budget == 0 {
                return found;
            }
            budget -= 1;
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            let named_git = entry.file_name() == ".git";
            if meta.dev() != top.dev() {
                continue;
            }
            if meta.is_dir() && !named_git && depth + 1 < LINK_DEPTH {
                pending.push((path, depth + 1));
            } else if meta.is_file()
                && named_git
                && let Some(common) = linked_from(&path)
                && !common.starts_with(&inside)
                && !found.contains(&common)
            {
                found.push(common);
            }
        }
    }
    found.sort();
    found
}

/// The repository a worktree's `.git` file links it from.
fn linked_from(dot_git: &Path) -> Option<PathBuf> {
    let text = fs::read(dot_git).ok().filter(|text| text.len() <= 4096)?;
    let line = std::str::from_utf8(&text).ok()?.lines().next()?;
    let gitdir = Path::new(line.strip_prefix("gitdir:")?.trim());
    // Relative paths, as worktree.useRelativePaths writes, are from the
    // worktree.
    let gitdir = dot_git.parent()?.join(gitdir);
    let worktrees = gitdir.parent()?;
    if worktrees.file_name()? != "worktrees" {
        return None;
    }
    worktrees.parent()?.canonicalize().ok()
}

/// Tells a repository to forget worktrees that no longer exist.
pub fn prune(common: &Path) -> std::io::Result<()> {
    let job = crate::commands::Job {
        argv: vec![
            "git".into(),
            format!("--git-dir={}", common.display()).into(),
            "worktree".into(),
            "prune".into(),
        ],
        cwd: None,
        env: Vec::new(),
        timeout: std::time::Duration::from_secs(60),
        limit: 64 << 10,
        idle: false,
    };
    match crate::commands::run(&job)? {
        crate::commands::Ran::Finished(output) if output.status == Some(0) => Ok(()),
        crate::commands::Ran::Finished(output) => Err(std::io::Error::other(format!(
            "git worktree prune failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
        crate::commands::Ran::TimedOut => {
            Err(std::io::Error::other("git worktree prune timed out"))
        }
        crate::commands::Ran::Stopped => Err(std::io::Error::other(
            "git worktree prune was stopped with the guard",
        )),
    }
}

/// The trees every concluded or retired quest lists in its README's
/// frontmatter, as `trees:` entries of `- path: ...`: their builds are
/// spent with the quest. A README that can't be read lists nothing.
pub fn finished_trees(quests: &Path) -> Vec<PathBuf> {
    finished(quests)
        .iter()
        .flat_map(|(_, frontmatter)| listed(frontmatter))
        .collect()
}

/// The trees a README's frontmatter lists.
fn listed(frontmatter: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut listing = false;
    for line in frontmatter.lines() {
        if !line.starts_with(' ') {
            listing = line.trim_end() == "trees:";
        } else if listing && let Some(path) = line.trim_start().strip_prefix("- path:") {
            found.push(PathBuf::from(path.trim()));
        }
    }
    found
}

/// A process a tree broker's record names as holding its tree, and when it
/// started, in wall milliseconds.
#[derive(Deserialize)]
struct Owner {
    pid: u32,
    #[serde(rename = "startedAt")]
    started_at: f64,
}

/// A tree broker's record of one tree it cut: where, and who holds it.
#[derive(Deserialize)]
struct Record {
    path: PathBuf,
    #[serde(default)]
    owners: Vec<Owner>,
}

/// The trees a tree broker's records in `records` name that nobody holds:
/// every process a record names has ended, and no quest still going lists
/// the tree as its own. Their builds were their holders', and go with them.
/// `running` says whether a pid still runs as the process that started at
/// a wall second, so a pid the kernel has given another since holds
/// nothing. A record that can't be read names no tree, so keeps its own.
pub fn unheld_trees(
    records: &Path,
    quests: &Path,
    running: impl Fn(u32, f64) -> bool,
) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(records) else {
        return Vec::new();
    };
    let going: Vec<PathBuf> = quest_frontmatters(quests)
        .iter()
        .filter(|(_, _, over)| !over)
        .flat_map(|(_, frontmatter, _)| listed(frontmatter))
        .collect();
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let Ok(text) = fs::read(entry.path()) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<Record>(&text) else {
            continue;
        };
        let held = record
            .owners
            .iter()
            .any(|owner| running(owner.pid, owner.started_at / 1000.0));
        if !held && !going.contains(&record.path) {
            found.push(record.path);
        }
    }
    found
}

/// Each concluded or retired quest, by its id, with its README's
/// frontmatter. A README that can't be read is no quest's.
pub fn finished(quests: &Path) -> Vec<(String, String)> {
    quest_frontmatters(quests)
        .into_iter()
        .filter(|(_, _, over)| *over)
        .map(|(id, frontmatter, _)| (id, frontmatter))
        .collect()
}

/// Each quest, by its id, with its README's frontmatter and whether it's
/// over: concluded or retired.
fn quest_frontmatters(quests: &Path) -> Vec<(String, String, bool)> {
    let Ok(entries) = fs::read_dir(quests) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(id) = name.to_str().filter(|name| QUEST_ID.is_match(name)) else {
            continue;
        };
        let Ok(text) = fs::read_to_string(entry.path().join("README.md")) else {
            continue;
        };
        let Some(frontmatter) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split("\n---").next())
        else {
            continue;
        };
        let over = FIELD
            .captures_iter(frontmatter)
            .any(|field| &field[1] == "status" && matches!(&field[2], "concluded" | "retired"));
        found.push((id.to_owned(), frontmatter.to_owned(), over));
    }
    found
}

/// Whether the workspace's quest is over, which a workspace has to be for
/// the guard to take it. Knowing this from the README alone spares sizing
/// the workspaces of quests still going, which hold most of the files.
pub fn over(workspace: &Workspace) -> bool {
    matches!(workspace.status.as_str(), "concluded" | "retired")
}

pub fn candidate(
    workspace: &Workspace,
    written: f64,
    size: u64,
    awake: &Awake,
    now: f64,
) -> Option<Candidate> {
    let idle = match workspace.status.as_str() {
        "retired" if workspace.born <= workspace.finished => f64::INFINITY,
        // Idle from the last write, or from when it appeared: a README
        // dates only the day it was updated, which kept a workspace until
        // that day's end, and a quest that's over owns nothing after it.
        "concluded" | "retired" => {
            let kept = workspace.born.max(written).min(now);
            awake.since(kept, now).max(0.0)
        }
        _ => return None,
    };
    Some(Candidate {
        path: workspace.path.clone(),
        class: Class::Workspace,
        idle,
        size,
        occupied: false,
        spent: false,
        checkout: None,
    })
}
