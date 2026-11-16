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
    /// What read the record: git's own files, and GitHub where a refresh
    /// has asked it.
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
    /// When it was last read, where that isn't the record's own reading,
    /// as GitHub's is a refresh's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<u64>,
    /// Why it couldn't be read, or couldn't see a repository, for a person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unread: Vec<String>,
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
    /// Its changes still open on its remote, the one updated last first;
    /// none where no source has read them.
    #[serde(default)]
    pub changes: Vec<Change>,
    /// Which way its changes were read, where any source read them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_through: Option<Through>,
}

/// Work put up for review on a repository's remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub title: String,
    /// Who put it up, by their name on the remote.
    pub author: Option<String>,
    /// Whether it's put up only to be seen, not yet to be reviewed.
    pub draft: bool,
    /// The branch it holds, which a tree may be checked out at.
    pub branch: String,
    /// The branch it would merge into.
    pub base: String,
    /// The commit its branch is at.
    pub commit: Option<String>,
    pub updated_at: u64,
    pub checks: Checks,
    pub review: Review,
    /// What GitHub calls it, in its own words, where GitHub holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<GitHubChange>,
    /// The people whose review GitHub says it asks, by login: held for the
    /// GitHub receptor's counts as a refresh reads them, and never kept or
    /// shown, so no file or answer holds a login it didn't before.
    #[serde(skip)]
    pub review_asked_of: Vec<String>,
}

/// Which way a repository's changes were read from GitHub: through the
/// machine's GitHub App, which most are, or through gh's sign-in, for the
/// few a machine lists that the App can't see (github.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Through {
    App,
    Gh,
}

/// A change as GitHub names it: its pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubChange {
    pub number: u64,
    pub url: String,
}

/// What a change's checks say, as one: failing if any failed, else
/// pending if any still runs, else passing if any ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checks {
    pub state: ChecksState,
    pub passed: usize,
    /// The names of those that failed, as their source names them.
    pub failed: Vec<String>,
    /// The names of those still to finish.
    pub pending: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecksState {
    Passing,
    Failing,
    Pending,
    /// No check runs where its source holds it, as for a repository
    /// proved on the machine rather than by its remote.
    None,
}

/// Where a change's review stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    Approved,
    ChangesAsked,
    /// A review is asked for and none has come.
    Waiting,
    /// Its remote asks for none.
    None,
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
    /// The branch it follows, as its checkout's config names it; none for
    /// a commit alone or a branch that follows nothing.
    #[serde(default)]
    pub upstream: Option<Upstream>,
}

/// The branch a head follows and the commit it's at here, as last
/// fetched: a head is out of date when the two commits differ. repos
/// never fetches, so this is as fresh as the checkout's last fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upstream {
    /// As git says it: `origin/main`, or `main` for a branch of its own.
    pub branch: String,
    /// None when the branch isn't there, as when the remote deleted it.
    pub commit: Option<String>,
    /// How far the head is from it, where that's known: at once where
    /// they're at one commit, else once the timer has counted the two
    /// (upstreams.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<Count>,
}

/// How many commits a branch's upstream has that the head lacks (behind),
/// and the head has that its upstream lacks (ahead). Each side stops at
/// `COUNTED_TO`, which says that many or more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Count {
    pub behind: u32,
    pub ahead: u32,
}

/// Where a count stops: past it, how far a head has fallen behind tells
/// nobody more, and counting a tree thousands of commits behind a very
/// large repository's trunk takes seconds where this takes one.
pub const COUNTED_TO: u32 = 100;

impl Count {
    /// Neither behind nor ahead.
    pub const EVEN: Self = Self {
        behind: 0,
        ahead: 0,
    };
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
