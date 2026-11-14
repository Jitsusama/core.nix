//! The sweep after a release: each `.git` its command made in its tree,
//! or one it changed the settings or hooks of, is set aside, since it
//! wrote the tree with no fence's eye on what git would later read there
//! (ISXEG8, 14 November). What later git runs read of each `.git` is
//! marked just before the command starts and compared after it ends,
//! rather than change times weighed against when it started: a nested
//! repository's own `git status` moves its folder's change time, and a
//! folder made ready elsewhere and renamed in keeps its settings' old one.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::is_folder;

/// What a `.git` a release made is renamed to, beside where it was, with
/// when, so later git runs there don't take its settings or hooks.
const SET_ASIDE: &str = ".git.planted-";

/// How long a `.git` set aside is kept, so one set aside wrongly can be
/// put back, before a later release's sweep removes it.
const SET_ASIDE_FOR_SECS: u64 = 7 * 24 * 60 * 60;

/// The most entries of a tree looked through for a `.git`, so a release
/// in a tree as large as a home doesn't wait on it.
const MOST_LOOKED: usize = 200_000;

/// The names in a `.git` folder that say what later git runs there do,
/// beside the hooks in its `hooks`.
const GIT_SAYS: [&str; 4] = ["config", "config.worktree", "commondir", "hooks"];

/// What of each `.git` in a tree later git runs read, by where it is.
pub type Gits = HashMap<PathBuf, Vec<Option<Mark>>>;

/// Which file a path is and when it last changed: its device, inode and
/// change time, which a command can't set back as it can a modified
/// time.
type Mark = (u64, u64, i64, i64);

/// Each `.git` in a tree as it is now, and each one set aside.
type Found = (Vec<(PathBuf, std::fs::Metadata)>, Vec<PathBuf>);

/// Each `.git` in a tree and what of it later git runs read, before a
/// release's command runs; none where the tree is too large to look
/// through.
pub fn seen(tree: &Path) -> Option<Gits> {
    let (found, _) = gits(tree)?;
    Some(
        found
            .into_iter()
            .map(|(path, meta)| {
                let marks = marks(&path, &meta);
                (path, marks)
            })
            .collect(),
    )
}

/// Sets aside each `.git` in `tree` that isn't as `before` saw it, and
/// removes those set aside long ago, saying what to tell the agent the
/// ask `id` was for.
pub fn sweep(tree: &Path, id: &str, before: Option<Gits>) -> Vec<String> {
    let (Some(before), Some((found, aside))) = (before, gits(tree)) else {
        let why = format!("its tree holds over {MOST_LOOKED} entries");
        eprintln!("<4>passwords: didn't look for a .git {id} made: {why}");
        return vec![format!("Didn't look for a .git it made, since {why}.")];
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let mut set_aside = Vec::new();
    let mut kept = Vec::new();
    for (path, meta) in found {
        if before.get(&path) == Some(&marks(&path, &meta)) {
            continue;
        }
        let named = path
            .strip_prefix(tree)
            .unwrap_or(&path)
            .display()
            .to_string();
        let to = path.with_file_name(format!("{SET_ASIDE}{now}"));
        match std::fs::rename(&path, &to) {
            Ok(()) => {
                eprintln!("<4>passwords: set aside {} {id} made", path.display());
                set_aside.push(named);
            }
            Err(why) => {
                eprintln!("<3>passwords: can't set aside {}: {why}", path.display());
                kept.push(named);
            }
        }
    }
    for old in aside.iter().filter(|old| long_set_aside(old, now)) {
        let gone = if is_folder(old) {
            std::fs::remove_dir_all(old)
        } else {
            std::fs::remove_file(old)
        };
        if let Err(why) = gone {
            eprintln!("<4>passwords: can't remove {}: {why}", old.display());
        }
    }
    let mut said = Vec::new();
    if !set_aside.is_empty() {
        said.push(format!(
            "Set aside the .git it made at {}, renamed {SET_ASIDE}{now} and kept 7 days, \
             since later git runs there would take its settings and hooks.",
            set_aside.join(", ")
        ));
    }
    if !kept.is_empty() {
        said.push(format!(
            "Couldn't set aside the .git it made at {}, so later git runs there take its \
             settings and hooks.",
            kept.join(", ")
        ));
    }
    if !said.is_empty() {
        said.push(
            "A release can't make a repository: init a submodule or clone before asking, \
             outside a release."
                .to_owned(),
        );
    }
    said
}

/// Each `.git` in a tree, and each one set aside, by where they are, links
/// never followed and no `.git` looked into; none where the tree holds
/// more than `MOST_LOOKED` entries.
fn gits(tree: &Path) -> Option<Found> {
    let mut found = Vec::new();
    let mut aside = Vec::new();
    let mut folders = vec![tree.to_path_buf()];
    let mut looked = 0_usize;
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            looked += 1;
            if looked > MOST_LOOKED {
                return None;
            }
            let name = entry.file_name();
            let path = entry.path();
            if name == ".git" {
                if let Ok(meta) = std::fs::symlink_metadata(&path) {
                    found.push((path, meta));
                }
            } else if name.to_string_lossy().starts_with(SET_ASIDE) {
                aside.push(path);
            } else if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                folders.push(path);
            }
        }
    }
    Some((found, aside))
}

/// What of a `.git` later git runs read: the file or link itself, or a
/// folder's own identity, its settings and its hooks. A folder's own
/// change time is left out, since every git run in it changes it.
fn marks(path: &Path, meta: &std::fs::Metadata) -> Vec<Option<Mark>> {
    let mark = |meta: &std::fs::Metadata| (meta.dev(), meta.ino(), meta.ctime(), meta.ctime_nsec());
    if !meta.is_dir() {
        return vec![Some(mark(meta))];
    }
    let mut marks = vec![Some((meta.dev(), meta.ino(), 0, 0))];
    for said in GIT_SAYS {
        marks.push(
            std::fs::symlink_metadata(path.join(said))
                .ok()
                .as_ref()
                .map(mark),
        );
    }
    let mut hooks: Vec<Mark> = std::fs::read_dir(path.join("hooks"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|hook| std::fs::symlink_metadata(hook.path()).ok())
        .map(|hook| mark(&hook))
        .collect();
    hooks.sort_unstable();
    marks.extend(hooks.into_iter().map(Some));
    marks
}

/// Whether a `.git` set aside at `path` was set aside over a week before
/// `now`, as its name says.
fn long_set_aside(path: &Path, now: u64) -> bool {
    path.file_name()
        .and_then(|name| name.to_str()?.strip_prefix(SET_ASIDE)?.parse::<u64>().ok())
        .is_some_and(|when| now.saturating_sub(when) > SET_ASIDE_FOR_SECS)
}
