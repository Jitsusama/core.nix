//! Files in the worktree that git neither tracks nor ignores, found by
//! walking it under each folder's ignore file. While a file system monitor
//! names nothing in a folder, what it held stands without walking it.

use super::{Cache, Counts, Limits, Listing, Paths, Patterns, full, os};
use crate::index::Index;
use crate::pattern::Pattern;
use std::collections::HashMap;
use std::sync::Arc;

/// Files in the worktree that git neither tracks nor ignores. A folder of
/// them counts once, as `git status` lists it.
pub fn untracked(
    index: &Arc<Index>,
    paths: &Paths,
    cache: &mut Cache,
    limits: Limits,
    counts: &mut Counts,
) {
    // What a folder held untracked stands only against the same index.
    if !paths.monitored {
        if !cache.walked.is_empty() {
            cache.walked = HashMap::new();
        }
        cache.walked_index = None;
    } else if !cache
        .walked_index
        .as_ref()
        .is_some_and(|seen| Arc::ptr_eq(seen, index))
    {
        cache.walked.clear();
        cache.walked_index = Some(index.clone());
    }
    let mut global: Vec<Arc<Vec<Pattern>>> = Vec::new();
    // Checked first to last: info/exclude, then core.excludesFile.
    global.push(cache.ignore_file(&paths.common.join("info/exclude")));
    if let Some(file) = &paths.excludes_file {
        global.push(cache.ignore_file(file));
    }
    let mut walker = Untracked {
        index,
        paths,
        cache,
        global,
        stack: Vec::new(),
        limits,
    };
    walker.walk(b"", counts);
}

struct Untracked<'a> {
    index: &'a Index,
    paths: &'a Paths<'a>,
    cache: &'a mut Cache,
    global: Vec<Arc<Vec<Pattern>>>,
    /// Each folder's own ignore file, outermost first, with the folder.
    stack: Vec<(Vec<u8>, Arc<Vec<Pattern>>)>,
    limits: Limits,
}

impl Untracked<'_> {
    fn ignored(&self, path: &[u8], is_dir: bool) -> bool {
        for (base, patterns) in self.stack.iter().rev() {
            for pattern in patterns.iter().rev() {
                if pattern.matches(path, is_dir, base) {
                    return !pattern.negated;
                }
            }
        }
        for patterns in &self.global {
            for pattern in patterns.iter().rev() {
                if pattern.matches(path, is_dir, b"") {
                    return !pattern.negated;
                }
            }
        }
        false
    }

    /// Lists a folder and takes in its ignore file, if it has one.
    fn enter(&mut self, dir: &[u8]) -> Arc<Listing> {
        let listing = self
            .cache
            .listing(self.paths.workdir, dir, self.paths.changed);
        let patterns = if listing.ignore_file {
            let file = self.paths.workdir.join(os(dir)).join(".gitignore");
            let mut name = dir.to_vec();
            name.extend_from_slice(b".gitignore");
            let unchanged = self.paths.changed.is_some_and(|c| !c.touches(&name))
                && self.cache.checked.contains(&name);
            match self.cache.ignores.get(&file) {
                Some((_, patterns)) if unchanged => patterns.clone(),
                _ => {
                    self.cache.checked.insert(name);
                    self.cache.ignore_file(&file)
                }
            }
        } else {
            Arc::default()
        };
        let base = dir.strip_suffix(b"/").unwrap_or(dir).to_vec();
        self.stack.push((base, patterns));
        listing
    }

    /// The ignore patterns that apply above a folder: its parents' files
    /// and the global ones.
    fn above(&self) -> Vec<Patterns> {
        self.global
            .iter()
            .chain(self.stack.iter().map(|(_, patterns)| patterns))
            .cloned()
            .collect()
    }

    fn same_above(&self, seen: &[Patterns]) -> bool {
        seen.len() == self.global.len() + self.stack.len()
            && self
                .global
                .iter()
                .chain(self.stack.iter().map(|(_, patterns)| patterns))
                .zip(seen)
                .all(|(now, then)| Arc::ptr_eq(now, then) || (now.is_empty() && then.is_empty()))
    }

    fn walk(&mut self, dir: &[u8], counts: &mut Counts) {
        // A folder the monitor names nothing in holds what it held, while
        // the patterns above it are the ones it was counted under.
        if self.paths.changed.is_some_and(|c| !c.touches_under(dir)) {
            let remembered = self
                .cache
                .walked
                .get(dir)
                .filter(|(above, _)| self.same_above(above))
                .map(|(_, found)| *found);
            if let Some(found) = remembered {
                counts.untracked += found;
                return;
            }
        }
        let before = counts.untracked;
        let above = self.paths.monitored.then(|| self.above());
        let listing = self.enter(dir);
        for (path, is_dir) in &listing.entries {
            if full(counts.untracked, self.limits.untracked) {
                break;
            }
            if !is_dir {
                if self.index.find(path).is_none() && !self.ignored(path, false) {
                    counts.untracked += 1;
                }
                continue;
            }
            let mut sub = path.clone();
            sub.push(b'/');
            if let Some(entry) = self.index.find(&sub)
                && entry.is_sparse_dir()
            {
                continue;
            }
            // A submodule, or a tracked file that's become a folder: the
            // change is the entry's, and nothing in it is untracked.
            if self.index.find(path).is_some() {
                continue;
            }
            if !self.index.under(&sub).is_empty() {
                // What git doesn't track inside an ignored folder is
                // ignored, even where it tracks something.
                if !self.ignored(path, true) {
                    self.walk(&sub, counts);
                }
                continue;
            }
            if !self.ignored(path, true) && self.holds_untracked(&sub) {
                counts.untracked += 1;
            }
        }
        self.stack.pop();
        // Only a count that ran to the end is the folder's whole.
        if let Some(above) = above
            && !full(counts.untracked, self.limits.untracked)
        {
            self.cache
                .walked
                .insert(dir.to_vec(), (above, counts.untracked - before));
        }
    }

    /// Whether a folder git tracks nothing in holds a file it doesn't ignore.
    /// A repository of its own counts as one such file.
    fn holds_untracked(&mut self, dir: &[u8]) -> bool {
        let listing = self.enter(dir);
        let mut found = listing.repository;
        for (path, is_dir) in &listing.entries {
            if found {
                break;
            }
            if *is_dir {
                let mut sub = path.clone();
                sub.push(b'/');
                found = !self.ignored(path, true) && self.holds_untracked(&sub);
            } else {
                found = !self.ignored(path, false);
            }
        }
        self.stack.pop();
        found
    }
}
