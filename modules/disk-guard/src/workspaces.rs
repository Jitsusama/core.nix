//! Quest workspaces: scratch kept beside a quest, spent once it's over.

use crate::clock::{Awake, local};
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::fs;
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
