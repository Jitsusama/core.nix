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
        };
    };
    Tree {
        uncommitted: uncommitted(git, path, &commit),
        commit: Some(commit),
    }
}
