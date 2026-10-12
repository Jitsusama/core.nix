//! When something was last built or written, in wall seconds.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// A path's own modification time, never through a symlink.
fn mtime(path: &Path) -> Option<f64> {
    let meta = fs::symlink_metadata(path).ok()?;
    Some(meta.mtime() as f64 + meta.mtime_nsec() as f64 / 1e9)
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let entries = entries.filter_map(Result::ok);
    entries
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect()
}

/// The HEAD reflog of the git checkout holding `dir`, which moves when
/// somebody commits, checks out, rebases or resets there. Not the index:
/// every `git status` rewrites that, and prompts run one on every command.
fn head_log(dir: &Path) -> Option<PathBuf> {
    for directory in dir.ancestors() {
        let dot_git = directory.join(".git");
        let Ok(meta) = fs::symlink_metadata(&dot_git) else {
            continue;
        };
        if meta.is_dir() {
            return Some(dot_git.join("logs/HEAD"));
        }
        // A linked worktree's .git is a file naming its git directory.
        let text = fs::read_to_string(&dot_git).ok()?;
        let gitdir = Path::new(text.trim().strip_prefix("gitdir:")?.trim());
        return Some(directory.join(gitdir).join("logs/HEAD"));
    }
    None
}

/// When anybody last built or worked in a target's tree. Only cargo writes
/// `deps` and `.fingerprint`, and the guard never changes either, so their
/// directories' times say when a build last produced something; the
/// checkout's HEAD reflog covers somebody committing without building. A
/// target with neither yet is dated by itself, so a new one isn't taken as
/// ancient.
pub fn last_build(target: &Path, checkout: Option<&Path>) -> f64 {
    let mut profiles = Vec::new();
    for child in subdirs(target) {
        profiles.extend(subdirs(&child));
        profiles.push(child);
    }
    let built = profiles
        .iter()
        .flat_map(|profile| [profile.join("deps"), profile.join(".fingerprint")])
        .filter_map(|dir| mtime(&dir))
        .reduce(f64::max)
        .or_else(|| mtime(target));
    let committed = checkout.and_then(head_log).and_then(|log| mtime(&log));
    built
        .into_iter()
        .chain(committed)
        .reduce(f64::max)
        .unwrap_or(0.0)
}

/// The newest modification time of `path` and everything within `depth`
/// levels of it. A tree with more than `budget` entries in reach counts as
/// written just now, so what can't be judged is kept.
pub fn last_write(path: &Path, depth: usize, budget: usize) -> f64 {
    let mut newest = mtime(path).unwrap_or(0.0);
    let is_dir = fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
    let mut level = if is_dir {
        vec![path.to_owned()]
    } else {
        Vec::new()
    };
    let mut seen = 0;
    for _ in 0..depth {
        let mut deeper = Vec::new();
        for dir in level {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                seen += 1;
                if seen > budget {
                    return f64::INFINITY;
                }
                let Ok(meta) = entry.metadata() else { continue };
                newest = newest.max(meta.mtime() as f64 + meta.mtime_nsec() as f64 / 1e9);
                if meta.is_dir() {
                    deeper.push(entry.path());
                }
            }
        }
        level = deeper;
    }
    newest
}
