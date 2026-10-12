//! Quest workspaces: scratch kept beside a quest, spent once it's over.

use crate::clock::{Awake, local};
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
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
/// time since its keeping began: when the quest finished, or when the
/// workspace appeared if later, and never before the last write in it.
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
    }
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
        "concluded" | "retired" => {
            let kept = workspace.finished.max(workspace.born).max(written).min(now);
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
