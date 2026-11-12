//! What the line says of the tree an ask runs in, read by the daemon
//! itself rather than taken from the ask: its commit, and how many of its
//! files differ from it. A tree's own git settings are written by whatever
//! works in it, an agent in a fence included, and this runs outside any
//! fence, so git is asked with nothing of the tree's that runs a program:
//! no hooks, no file monitor, no attributes (whose filters run programs as
//! a file is read), no replacement objects, no submodules entered, and no
//! settings but the tree's own and these.

use std::path::Path;
use std::process::{Command, Stdio};

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

/// The tree at `path`, asked of the git at `git`.
pub fn read(git: &Path, path: &Path) -> Tree {
    let asked = |args: &[&str]| {
        let ran = Command::new(git)
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_ATTR_SOURCE", EMPTY_TREE)
            .env("GIT_TERMINAL_PROMPT", "0")
            // Status writes the tree's index only to save work, so it's
            // asked not to: this reads the tree and changes nothing in it.
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("HOME", "/var/empty")
            .arg("-C")
            .arg(path)
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
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        ran.status
            .success()
            .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
    };
    let Some(commit) = asked(&["rev-parse", "--verify", "HEAD"]) else {
        return Tree {
            commit: None,
            uncommitted: None,
        };
    };
    let status = asked(&[
        "status",
        "--porcelain=v1",
        "--untracked-files=normal",
        "--ignore-submodules=all",
    ]);
    Tree {
        commit: Some(commit.trim().to_owned()),
        uncommitted: status.map(|said| said.lines().count() as u64),
    }
}
