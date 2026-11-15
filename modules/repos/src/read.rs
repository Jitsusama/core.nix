//! Reading the disk into the record. Under each root, up to four folders
//! deep and past no hidden folder, every folder holding `.git` is a way
//! into a checkout: a `.git` folder is the checkout's own, a `.git` file
//! a tree's, followed to the checkout it was cut from. Each checkout then
//! lists its own trees from its `worktrees/` folder, wherever their
//! folders are, so the harness's trees and a monorepo's are found from their
//! checkouts without walking the folders that hold them. Checkouts whose
//! remote is one repository are listed as one.
//!
//! Everything is read from git's own small files (HEAD, config, refs,
//! packed-refs or a reftable's ref blocks, each tree's `gitdir`); no git
//! runs, and no object is opened, so a reading of a thousand trees takes
//! milliseconds.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::keys::{self, Key};
use crate::model::{Checkout, Head, Record, Remote, Repository, SCHEMA, Source, Tree, TreeKind};
use crate::reftable::{self, Ref};

/// How deep a `.git` is looked for under a root: in
/// `src/github.com/someone/a/.git`, a checkout three folders deep.
const DEEPEST: usize = 4;

/// Folders never walked into: what a package manager or a build fills,
/// which holds no checkout of Joel's and can hold thousands of folders.
const NEVER: [&str; 2] = ["node_modules", "target"];

/// The files git writes whenever it changes a checkout's or a tree's
/// index, branches or what it fetched, whose newest says when it was
/// last worked in, as the programs daemon judges it.
const WRITTEN: [&str; 5] = ["HEAD", "index", "FETCH_HEAD", "ORIG_HEAD", "logs/HEAD"];

/// What a reading is told: where to look, from whose home, on which
/// machine, and when.
#[derive(Debug, Clone)]
pub struct Reading<'a> {
    pub roots: &'a [PathBuf],
    /// Home, which paths are said from as `~`; none says them whole.
    pub home: Option<&'a Path>,
    pub machine: &'a str,
    pub now: u64,
}

/// Every repository under the roots, each once.
pub fn read(reading: &Reading) -> Record {
    let mut commons = BTreeSet::new();
    for root in reading.roots {
        let root = fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        let mut found = Vec::new();
        walk(&root, 0, &mut found);
        commons.extend(found.iter().filter_map(|folder| common_dir(folder)));
    }
    let mut repositories: BTreeMap<String, (Key, Vec<Found>)> = BTreeMap::new();
    for common in &commons {
        let found = checkout(common, reading.home);
        repositories
            .entry(found.key.key.clone())
            .or_insert_with(|| (found.key.clone(), Vec::new()))
            .1
            .push(found);
    }
    let mut repositories: Vec<Repository> = repositories
        .into_values()
        .map(|(key, mut found)| {
            found.sort_by(|a, b| newest_first(a.checkout.written_at, b.checkout.written_at));
            let remotes = found.first().map(|f| f.remotes.clone()).unwrap_or_default();
            let trunk = found.iter().find_map(|f| f.trunk.clone());
            let checkouts: Vec<Checkout> = found.into_iter().map(|f| f.checkout).collect();
            let written_at = checkouts
                .iter()
                .flat_map(|c| {
                    std::iter::once(c.written_at).chain(c.trees.iter().map(|t| t.written_at))
                })
                .flatten()
                .max();
            Repository {
                key: key.key,
                name: key.name,
                owner: key.owner,
                remotes,
                trunk,
                written_at,
                checkouts,
            }
        })
        .collect();
    repositories
        .sort_by(|a, b| newest_first(a.written_at, b.written_at).then_with(|| a.name.cmp(&b.name)));
    Record {
        schema: SCHEMA.to_owned(),
        read_at: reading.now,
        machine: reading.machine.to_owned(),
        roots: reading
            .roots
            .iter()
            .map(|root| root.display().to_string())
            .collect(),
        sources: vec![Source {
            name: "git".to_owned(),
            what: "each checkout's and tree's own git files".to_owned(),
        }],
        repositories,
    }
}

/// The newer first, and a time not known after every known one.
fn newest_first(a: Option<u64>, b: Option<u64>) -> std::cmp::Ordering {
    b.cmp(&a)
}

/// Each folder under `dir` holding `.git`, `dir` included, stopping at
/// each one found, since a checkout's own folders hold its files.
fn walk(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if dir.join(".git").exists() {
        found.push(dir.to_path_buf());
        return;
    }
    if depth == DEEPEST {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // A symbolic link isn't followed, so a link back up can't loop.
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_dir && !name.starts_with('.') && !NEVER.contains(&name.as_ref()) {
            walk(&entry.path(), depth + 1, found);
        }
    }
}

/// The checkout's own git folder that a folder holding `.git` leads to:
/// its `.git` folder, or, for a tree, the folder its `.git` file names,
/// then that one's `commondir`.
fn common_dir(folder: &Path) -> Option<PathBuf> {
    let dot_git = folder.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let said = fs::read_to_string(&dot_git).ok()?;
        let named = said.trim().strip_prefix("gitdir:")?.trim();
        folder.join(named)
    };
    let common = match fs::read_to_string(git_dir.join("commondir")) {
        Ok(said) => git_dir.join(said.trim()),
        Err(_) => git_dir,
    };
    fs::canonicalize(&common).ok()
}

/// One checkout as it was read, with what its repository takes from it.
struct Found {
    checkout: Checkout,
    remotes: Vec<Remote>,
    trunk: Option<String>,
    key: Key,
}

/// One checkout, read: itself, its remotes, its trunk and its key.
fn checkout(common: &Path, home: Option<&Path>) -> Found {
    let bare = common.file_name().is_none_or(|name| name != ".git");
    let folder = if bare {
        common.to_path_buf()
    } else {
        common.parent().unwrap_or(common).to_path_buf()
    };
    let refs = Refs::of(common);
    let remotes = remotes(&fs::read_to_string(common.join("config")).unwrap_or_default());
    let mut remotes = remotes;
    // The remote the key comes from goes first: origin, else the first.
    if let Some(at) = remotes.iter().position(|remote| remote.name == "origin") {
        let origin = remotes.remove(at);
        remotes.insert(0, origin);
    }
    let key = remotes
        .first()
        .and_then(|remote| keys::of_remote(&remote.url));
    let key = key.unwrap_or_else(|| {
        let name = folder_name(&folder, bare);
        keys::of_folder(&shown(&folder, home), &name)
    });
    let trunk = trunk(&refs, remotes.first().map(|remote| remote.name.as_str()));
    let mut trees = trees(common, &refs);
    trees.sort_by(|a, b| newest_first(a.written_at, b.written_at).then(a.path.cmp(&b.path)));
    let checkout = Checkout {
        path: folder.display().to_string(),
        bare,
        head: (!bare).then(|| head(common, &refs)).flatten(),
        written_at: written(common),
        trees,
    };
    Found {
        checkout,
        remotes,
        trunk,
        key,
    }
}

/// A checkout's name when it has no remote: its folder's, or for a bare
/// one called `git`, as a monorepo's often is, the folder holding it.
fn folder_name(folder: &Path, bare: bool) -> String {
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let own = name(folder);
    let own = own.strip_suffix(".git").map_or(own.clone(), str::to_owned);
    if bare && own == "git" {
        folder.parent().map_or(own, name)
    } else {
        own
    }
}

/// A path said from home as `~`, as a person reads it.
pub fn shown(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Every tree cut from a checkout, from its `worktrees/` folder: each
/// entry's `gitdir` names the tree's `.git` file, in the folder that is
/// the tree.
fn trees(common: &Path, refs: &Refs) -> Vec<Tree> {
    let Ok(entries) = fs::read_dir(common.join("worktrees")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let own = entry.path();
            let said = fs::read_to_string(own.join("gitdir")).ok()?;
            let dot_git = PathBuf::from(said.trim());
            let dot_git = if dot_git.is_absolute() {
                dot_git
            } else {
                own.join(dot_git)
            };
            let folder = dot_git.parent()?.to_path_buf();
            let head = head(&own, refs)?;
            Some(Tree {
                path: folder.display().to_string(),
                kind: if head.branch.is_some() {
                    TreeKind::Worktree
                } else {
                    TreeKind::Snapshot
                },
                present: dot_git.exists(),
                head,
                written_at: written(&own),
            })
        })
        .collect()
}

/// What a git folder's HEAD is at: a branch, and its commit where the
/// ref can be read, or a commit alone.
fn head(git_dir: &Path, refs: &Refs) -> Option<Head> {
    let said = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let mut at = loose_ref(said.trim());
    // A reftable checkout's HEAD file names a branch that can't be, and
    // its real HEAD is in the git folder's own table.
    if at == Ref::Symbolic(UNREADABLE.to_owned()) {
        at = if git_dir == refs.common {
            refs.get("HEAD")?
        } else {
            reftable::stack(&git_dir.join("reftable"))?.remove("HEAD")?
        };
    }
    Some(match at {
        Ref::Symbolic(name) => Head {
            branch: Some(name.strip_prefix("refs/heads/").unwrap_or(&name).to_owned()),
            commit: refs.commit(&name),
        },
        Ref::Commit(commit) => Head {
            branch: None,
            commit: Some(commit),
        },
    })
}

/// The branch a reftable checkout's HEAD files name, so that a git too old
/// to read reftables refuses it rather than misreading it.
const UNREADABLE: &str = "refs/heads/.invalid";

/// A ref as a loose file says it: `ref: ` and the ref it names, or a
/// commit.
fn loose_ref(said: &str) -> Ref {
    match said.strip_prefix("ref:") {
        Some(name) => Ref::Symbolic(name.trim().to_owned()),
        None => Ref::Commit(said.to_owned()),
    }
}

/// The newest time git wrote one of a git folder's own files.
fn written(git_dir: &Path) -> Option<u64> {
    WRITTEN
        .iter()
        .filter_map(|file| fs::metadata(git_dir.join(file)).ok()?.modified().ok())
        .filter_map(|at| at.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_secs())
        .max()
}

/// The branch a repository's changes merge into: what its remote's HEAD
/// names, else main or master as the remote has them; with no remote,
/// main or master among its own branches.
fn trunk(refs: &Refs, remote: Option<&str>) -> Option<String> {
    let (prefix, named) = match remote {
        Some(remote) => {
            let prefix = format!("refs/remotes/{remote}/");
            let named = refs.symbolic(&format!("{prefix}HEAD"));
            (prefix, named)
        }
        None => ("refs/heads/".to_owned(), None),
    };
    if let Some(named) = named.and_then(|named| named.strip_prefix(&prefix).map(str::to_owned)) {
        return Some(named);
    }
    ["main", "master"]
        .into_iter()
        .find(|branch| refs.has(&format!("{prefix}{branch}")))
        .map(str::to_owned)
}

/// A checkout's refs: its loose files and its packed-refs, or the stack
/// of tables a reftable checkout keeps them in (reftable.rs).
struct Refs {
    common: PathBuf,
    packed: HashMap<String, String>,
    table: Option<HashMap<String, Ref>>,
}

impl Refs {
    fn of(common: &Path) -> Self {
        let tables = common.join("reftable");
        let table = tables
            .is_dir()
            .then(|| reftable::stack(&tables).unwrap_or_default());
        let packed = fs::read_to_string(common.join("packed-refs"))
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.starts_with(['#', '^']))
            .filter_map(|line| line.split_once(' '))
            .map(|(commit, name)| (name.trim().to_owned(), commit.to_owned()))
            .collect();
        Self {
            common: common.to_path_buf(),
            packed,
            table,
        }
    }

    /// A ref as the checkout keeps it.
    fn get(&self, name: &str) -> Option<Ref> {
        if let Some(table) = &self.table {
            return table.get(name).cloned();
        }
        if name.split('/').any(|part| part == "..") {
            return None;
        }
        match fs::read_to_string(self.common.join(name)) {
            Ok(said) => Some(loose_ref(said.trim())),
            Err(_) => self.packed.get(name).cloned().map(Ref::Commit),
        }
    }

    /// The commit a ref is at, following a symbolic ref as git does, a
    /// few steps at most, so a ref naming itself can't loop.
    fn commit(&self, name: &str) -> Option<String> {
        let mut name = name.to_owned();
        for _ in 0..5 {
            match self.get(&name)? {
                Ref::Symbolic(next) => name = next,
                Ref::Commit(commit) => return Some(commit),
            }
        }
        None
    }

    /// The ref a symbolic ref names.
    fn symbolic(&self, name: &str) -> Option<String> {
        match self.get(name)? {
            Ref::Symbolic(named) => Some(named),
            Ref::Commit(_) => None,
        }
    }

    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

/// The remotes a checkout's config names, in its order, each with the
/// first address it gives. Only what's needed is read: sections, and the
/// url key within a remote's.
fn remotes(config: &str) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    let mut current: Option<String> = None;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with(['#', ';']) || line.is_empty() {
            continue;
        }
        if let Some(section) = line.strip_prefix('[') {
            let section = section.split(']').next().unwrap_or_default().trim();
            current = section
                .split_once(char::is_whitespace)
                .filter(|(kind, _)| kind.eq_ignore_ascii_case("remote"))
                .map(|(_, name)| name.trim().trim_matches('"').to_owned());
            continue;
        }
        let Some(name) = &current else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("url") || remotes.iter().any(|r| &r.name == name) {
            continue;
        }
        remotes.push(Remote {
            name: name.clone(),
            url: value.trim().trim_matches('"').to_owned(),
        });
    }
    remotes
}

#[cfg(test)]
mod tests;
