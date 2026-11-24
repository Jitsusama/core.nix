//! Which of the tree's tracked names the near side may read: a name
//! within the tree, holding nothing a list can't carry, reached through
//! the tree's own folders and never through a link that could lead out
//! of it.

use super::travels;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Whether `path` names somewhere within a tree: a relative path of plain
/// names, never from the root, through `..` or `.`. Git writes no other
/// into its index, but an index written by hand could, and the near side
/// would then read from outside the tree; one from the root names `""`.
pub(super) fn within_tree(path: &str) -> bool {
    !path
        .split('/')
        .any(|name| name.is_empty() || name == "." || name == "..")
}

/// Whether the tracked `path` travels, or why the job can't: one outside
/// the tree is refused first, as `..` would read as hidden and pass unsaid,
/// and one that travels holds no tab or newline, which no list carries.
pub(super) fn tracked(path: &str, excludes: &[String]) -> Result<bool, String> {
    if !within_tree(path) {
        return Err(format!(
            "the tracked name {path:?} isn't a path within the tree"
        ));
    }
    if !travels(path, excludes) {
        return Ok(false);
    }
    if path.contains(['\t', '\n']) {
        return Err(format!(
            "the tracked name {path:?} holds a tab or a newline"
        ));
    }
    Ok(true)
}

/// The folders of a tree found to be its own, each looked at once however
/// many files lie in it.
#[derive(Default)]
pub(super) struct Folders(HashSet<PathBuf>);

impl Folders {
    /// Whether each folder on the way to the tracked `path` within `root`
    /// is a folder of the tree, not a link: a folder git tracked a file in
    /// may since have been swapped for a link, which could lead anywhere,
    /// and the name's own text can't say so.
    pub(super) fn own(&mut self, root: &Path, path: &str) -> bool {
        let Some((folders, _)) = path.rsplit_once('/') else {
            return true;
        };
        let mut at = root.to_path_buf();
        for name in folders.split('/') {
            at.push(name);
            if self.0.contains(&at) {
                continue;
            }
            if !fs::symlink_metadata(&at).is_ok_and(|meta| meta.is_dir()) {
                return false;
            }
            self.0.insert(at.clone());
        }
        true
    }
}
