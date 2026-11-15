//! What the line says of the tree an ask runs in, read by the daemon
//! itself rather than taken from the ask: its commit, and how many of its
//! files differ from it. A tree's own git settings are written by whatever
//! works in it, an agent in a fence included, and this runs outside any
//! fence, so nothing of the tree's that names a program may reach git.
//!
//! Turning settings off one by one can't do that, since a repository's own
//! config names programs under names it chooses: a clean filter that
//! `.git/info/attributes` gives every file runs as status reads a file
//! whose time moved. So the tree's settings are never read where a file
//! is. Its commit and where its index and objects lie are read first,
//! which starts nothing; then status runs over a git folder made for the
//! read, holding only that commit, with the tree's index and objects named
//! outright and no config, attributes or hooks of the tree's at all.
//!
//! Where its history is kept is read from files alone, git never run: its
//! own checkout, a worktree git vouches for, or a `.git` it wouldn't.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// The tree as git says it, or that it isn't one.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    /// Its commit, whole, or none where it isn't a git tree.
    pub commit: Option<String>,
    /// How many of its files differ from the commit, untracked ones
    /// included, or none where git couldn't say, which the line warns of
    /// as it would of changes rather than calling the tree clean.
    pub uncommitted: Option<u64>,
    /// Where its history is kept, which says how much of it a release
    /// may write.
    pub checkout: Checkout,
}

/// Where a tree's history is kept, as its `.git` says, read from files
/// alone so nothing of the tree's runs: the nearest `.git` at or above it.
#[derive(Clone, Debug, PartialEq)]
pub enum Checkout {
    /// In no repository.
    None,
    /// A repository's own checkout, its `.git` the repository's folder.
    Own { dot_git: PathBuf },
    /// A worktree git vouches for: its `.git` file names a worktree's own
    /// folder among `common`'s, and that folder names this `.git` back.
    Worktree {
        dot_git: PathBuf,
        common: PathBuf,
        own: PathBuf,
    },
    /// A `.git` git wouldn't vouch for: a file naming a worktree that
    /// doesn't name it back, or a link, so its history is never opened.
    Unvouched { dot_git: PathBuf },
}

/// Where the history of the tree at `path` is kept.
pub fn checkout(path: &Path) -> Checkout {
    let Some((dot_git, meta)) = path.ancestors().find_map(|folder| {
        let dot_git = folder.join(".git");
        std::fs::symlink_metadata(&dot_git)
            .ok()
            .map(|meta| (dot_git, meta))
    }) else {
        return Checkout::None;
    };
    // A link is never vouched for: one to another worktree's `.git` file
    // would resolve to where that worktree's folder names it back.
    if meta.is_dir() {
        return Checkout::Own { dot_git };
    }
    match meta.is_file().then(|| worktree(&dot_git)).flatten() {
        Some((common, own)) => Checkout::Worktree {
            dot_git,
            common,
            own,
        },
        None => Checkout::Unvouched { dot_git },
    }
}

/// The repository and the worktree's own git folder a worktree's `.git`
/// file names, where git's own record agrees: the folder is one of the
/// repository's worktrees and says it is this one, so a `.git` file a
/// command wrote can't open another's history.
fn worktree(dot_git: &Path) -> Option<(PathBuf, PathBuf)> {
    let read = |path: &Path| std::fs::read_to_string(path).ok();
    let resolved = |path: &Path| std::fs::canonicalize(path).ok();
    let text = read(dot_git)?;
    let named = text.strip_prefix("gitdir:")?.trim();
    let own = resolved(&dot_git.parent()?.join(named))?;
    let common = resolved(&own.join(read(&own.join("commondir"))?.trim()))?;
    let back = resolved(&own.join(read(&own.join("gitdir"))?.trim()))?;
    let folder = |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
    let belongs = own.parent() == Some(common.join("worktrees").as_path())
        && Some(back) == resolved(dot_git)
        && folder(&own)
        && folder(&common.join("objects"));
    belongs.then_some((common, own))
}

/// git's empty tree, given as where attributes come from so none apply.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// How many reads this daemon has made a git folder for, so no two share.
static READS: AtomicU64 = AtomicU64::new(0);

/// git at `git`, with nothing of the machine's or Joel's own settings and
/// none of the tree's that runs a program on the command line's terms.
fn git_command(git: &Path) -> Command {
    let mut command = Command::new(git);
    command
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_ATTR_SOURCE", EMPTY_TREE)
        .env("GIT_TERMINAL_PROMPT", "0")
        // Status writes the tree's index only to save work, so it's asked
        // not to: this reads the tree and changes nothing in it.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("HOME", "/var/empty")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "diff.ignoreSubmodules=all",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// What `command` printed, when it succeeded.
fn said(command: &mut Command) -> Option<String> {
    let ran = command.output().ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// Where the tree at `path` keeps what status reads, asked of git with
/// the tree's own settings, which rev-parse runs no program for.
struct Places {
    top: PathBuf,
    index: PathBuf,
    objects: PathBuf,
}

fn places(git: &Path, path: &Path) -> Option<Places> {
    let told = said(git_command(git).arg("-C").arg(path).args([
        "rev-parse",
        "--path-format=absolute",
        "--show-toplevel",
        "--git-path",
        "index",
        "--git-common-dir",
    ]))?;
    let mut lines = told.lines().map(PathBuf::from);
    let (top, index, common) = (lines.next()?, lines.next()?, lines.next()?);
    Some(Places {
        top,
        index,
        objects: common.join("objects"),
    })
}

/// A git folder of the daemon's own, holding `commit` and nothing else,
/// removed when it goes.
struct Made(PathBuf);

impl Made {
    fn new(commit: &str) -> Option<Made> {
        let read = READS.fetch_add(1, Ordering::Relaxed);
        let folder =
            std::env::temp_dir().join(format!("secret-asks-read-{}-{read}", std::process::id()));
        // create_dir refuses a folder already there, so nothing planted at
        // this name beforehand is used.
        std::fs::create_dir(&folder).ok()?;
        let made = Made(folder);
        std::fs::create_dir(made.0.join("refs")).ok()?;
        std::fs::write(made.0.join("HEAD"), format!("{commit}\n")).ok()?;
        Some(made)
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        // A folder left in the temporary folder holds no secret, only a
        // commit's name, so a failure to remove it isn't worth stopping for.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// How many files of the tree differ from `commit`, read over a git folder
/// made for the read, so none of the tree's settings is read.
fn uncommitted(git: &Path, path: &Path, commit: &str) -> Option<u64> {
    let places = places(git, path)?;
    let made = Made::new(commit)?;
    let status = said(
        git_command(git)
            .env("GIT_DIR", &made.0)
            .env("GIT_WORK_TREE", &places.top)
            .env("GIT_INDEX_FILE", &places.index)
            .env("GIT_OBJECT_DIRECTORY", &places.objects)
            .arg("-C")
            .arg(&places.top)
            .args([
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=all",
            ]),
    )?;
    Some(status.lines().count() as u64)
}

/// The tree at `path`, asked of the git at `git`.
pub fn read(git: &Path, path: &Path) -> Tree {
    let commit = said(
        git_command(git)
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--verify", "HEAD"]),
    )
    .map(|said| said.trim().to_owned());
    let Some(commit) = commit else {
        return Tree {
            commit: None,
            uncommitted: None,
            // A `.git` with no commit yet still says what a release may
            // write of it.
            checkout: checkout(path),
        };
    };
    Tree {
        uncommitted: uncommitted(git, path, &commit),
        commit: Some(commit),
        checkout: checkout(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own, emptied first.
    fn folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("secret-asks-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::canonicalize(folder).unwrap()
    }

    /// A repository at `top/repo` with a worktree named `name`, whose own
    /// folder names `back` as its tree's `.git`.
    fn repository(top: &Path, name: &str, back: &Path) -> (PathBuf, PathBuf) {
        let common = top.join("repo/.git");
        let own = common.join("worktrees").join(name);
        std::fs::create_dir_all(common.join("objects")).unwrap();
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(own.join("commondir"), "../..\n").unwrap();
        std::fs::write(own.join("gitdir"), format!("{}\n", back.display())).unwrap();
        (common, own)
    }

    #[test]
    fn a_repositorys_own_checkout_is_found_from_a_folder_within_it() {
        let top = folder("own");
        std::fs::create_dir_all(top.join(".git")).unwrap();
        std::fs::create_dir_all(top.join("deep/within")).unwrap();
        let own = Checkout::Own {
            dot_git: top.join(".git"),
        };
        assert_eq!(checkout(&top), own, "the checkout wasn't found");
        assert_eq!(
            checkout(&top.join("deep/within")),
            own,
            "a folder within it wasn't"
        );
    }

    #[test]
    fn a_worktree_is_vouched_for_only_where_its_folder_names_it_back() {
        let top = folder("worktree");
        let tree = top.join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        let dot_git = tree.join(".git");
        let (common, own) = repository(&top, "tree", &dot_git);
        std::fs::write(&dot_git, format!("gitdir: {}\n", own.display())).unwrap();
        assert_eq!(
            checkout(&tree),
            Checkout::Worktree {
                dot_git: dot_git.clone(),
                common,
                own: own.clone()
            },
            "the worktree wasn't vouched for"
        );
        std::fs::write(own.join("gitdir"), "/elsewhere/.git\n").unwrap();
        assert_eq!(
            checkout(&tree),
            Checkout::Unvouched { dot_git },
            "a worktree naming another tree was vouched for"
        );
    }

    #[test]
    fn a_linked_git_folder_is_never_vouched_for() {
        let top = folder("linked");
        std::fs::create_dir_all(top.join("repo/.git")).unwrap();
        std::fs::create_dir_all(top.join("tree")).unwrap();
        let dot_git = top.join("tree/.git");
        std::os::unix::fs::symlink(top.join("repo/.git"), &dot_git).unwrap();
        assert_eq!(
            checkout(&top.join("tree")),
            Checkout::Unvouched { dot_git },
            "a link to another's history was taken as the tree's own"
        );
    }

    #[test]
    fn a_folder_in_no_repository_is_none() {
        let top = folder("none");
        assert_eq!(
            checkout(&top),
            Checkout::None,
            "a .git was found where none is"
        );
    }
}
