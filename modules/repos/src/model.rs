//! The record every view of repos reads, `repos.repositories/1`: each
//! repository once, by its key, with every checkout of it on the machine
//! and every tree cut from each (docs/glossary.md's Code words). Times are
//! seconds since the epoch here, said in RFC 3339 in an answer.

use serde::{Deserialize, Serialize};

/// The record's schema, which a reader checks before trusting its shape.
pub const SCHEMA: &str = "repos.repositories/1";

/// Every repository on the machine, as one reading of the disk found them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    /// When the disk was read.
    pub read_at: u64,
    /// The machine the checkouts are on, as it calls itself.
    pub machine: String,
    /// The folders looked under for checkouts.
    pub roots: Vec<String>,
    /// What read the record: git's own files, for now the only source.
    pub sources: Vec<Source>,
    /// Each repository, the one written to last first.
    pub repositories: Vec<Repository>,
}

/// What a record's facts were read from, as net's sources are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    /// What it reads, for a person.
    pub what: String,
}

/// One history, listed once however many checkouts of it there are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    /// Its identity: where its remote keeps it (`github:Jitsusama/core.nix`),
    /// or `local:` and its checkout's folder when it has no remote.
    pub key: String,
    /// Its own name, the key's last part: `core.nix`.
    pub name: String,
    /// Whose it is on its remote, `Jitsusama`, said only to tell two of one
    /// name apart; none for a repository with no remote.
    pub owner: Option<String>,
    /// Its remotes, as the checkout written to last names them, the one its
    /// key comes from first.
    pub remotes: Vec<Remote>,
    /// The branch its changes merge into, as its remote says.
    pub trunk: Option<String>,
    /// When git last wrote in any of its checkouts or trees.
    pub written_at: Option<u64>,
    /// Each clone of it on the machine, the one written to last first.
    pub checkouts: Vec<Checkout>,
}

/// A place a repository is kept away from the machine, as git names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

/// A repository's clone, which trees are cut from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkout {
    /// Its folder: the one holding `.git`, or a bare one's own.
    pub path: String,
    /// Whether it has no files of its own, only trees, as a very large repository's often has.
    pub bare: bool,
    /// What its own files are at; none for a bare one.
    pub head: Option<Head>,
    /// When git last wrote in it, apart from its trees.
    pub written_at: Option<u64>,
    /// Every tree cut from it, wherever its folder is, the one written to
    /// last first.
    pub trees: Vec<Tree>,
}

/// A folder holding a repository's files to work in, cut from a checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tree {
    pub path: String,
    /// A worktree is at a branch, a snapshot pinned at a commit.
    pub kind: TreeKind,
    pub head: Head,
    /// Whether its folder is still there; one that isn't is git's to prune.
    pub present: bool,
    pub written_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeKind {
    Worktree,
    Snapshot,
}

/// Where a checkout's or a tree's files are: a branch, and the commit
/// it's at where its ref can be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    pub branch: Option<String>,
    pub commit: Option<String>,
}

impl Head {
    /// The head as a person reads it: its branch, or its commit's first
    /// seven characters.
    pub fn said(&self) -> String {
        match (&self.branch, &self.commit) {
            (Some(branch), _) => branch.clone(),
            (None, Some(commit)) => commit.chars().take(7).collect(),
            (None, None) => String::new(),
        }
    }
}
