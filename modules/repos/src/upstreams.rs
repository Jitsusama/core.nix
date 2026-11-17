//! How far each head is from the branch it follows. The read gives each
//! head its upstream from git's small files (read.rs), and says at once
//! that a head at its upstream's own commit is even with it. Any other
//! pair needs git's commits counted, which for a tree far behind a very
//! large repository's trunk takes seconds, so a timer counts
//! (`repos count`) and keeps the counts as `repos.upstreams/1`
//! (`Counted`), which every reading merges in, as it does GitHub's part.
//!
//! A count is kept against its two commits, whose history never changes,
//! so each pair is counted once however often it's read; each counting
//! keeps only the pairs still in the record. Each side stops at
//! `COUNTED_TO`. repos never fetches, so a count is as fresh as the
//! checkout's last fetch.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::kept;
use crate::model::{COUNTED_TO, Count, Head, Record, Source};

/// The schema of what a counting kept, which a reading checks before
/// trusting its shape.
pub const SCHEMA: &str = "repos.upstreams/1";

/// What a counting kept: each pair's count, by `upstream...head`, the
/// way git writes a range between two commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counted {
    pub schema: String,
    pub counted_at: u64,
    pub counts: BTreeMap<String, Count>,
}

/// Two commits to count between, and the checkout whose history holds
/// both, since its trees share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub checkout: PathBuf,
    pub upstream: String,
    pub head: String,
}

impl Pair {
    fn key(&self) -> String {
        key(&self.upstream, &self.head)
    }
}

fn key(upstream: &str, head: &str) -> String {
    format!("{upstream}...{head}")
}

/// A head's upstream commit and its own, where both are known and differ:
/// the pair it waits on a count for.
fn apart(head: &Head) -> Option<(&str, &str)> {
    let commit = head.commit.as_deref()?;
    let upstream = head.upstream.as_ref()?.commit.as_deref()?;
    (upstream != commit).then_some((upstream, commit))
}

/// Every head in the record, its checkout's with it, as a mutable list.
fn heads(record: &mut Record) -> impl Iterator<Item = &mut Head> {
    record
        .repositories
        .iter_mut()
        .flat_map(|repository| &mut repository.checkouts)
        .flat_map(|checkout| {
            checkout
                .head
                .iter_mut()
                .chain(checkout.trees.iter_mut().map(|tree| &mut tree.head))
        })
}

/// Each pair the record's heads wait on, once however many heads share
/// it. A tree gone from the disk is left out, as its branch is git's to
/// prune.
pub fn pairs(record: &Record) -> Vec<Pair> {
    let mut pairs = BTreeMap::new();
    for checkout in record
        .repositories
        .iter()
        .flat_map(|repository| &repository.checkouts)
    {
        let present = checkout
            .trees
            .iter()
            .filter(|tree| tree.present)
            .map(|tree| &tree.head);
        for (upstream, head) in checkout.head.iter().chain(present).filter_map(apart) {
            pairs.entry(key(upstream, head)).or_insert_with(|| Pair {
                checkout: PathBuf::from(&checkout.path),
                upstream: upstream.to_owned(),
                head: head.to_owned(),
            });
        }
    }
    pairs.into_values().collect()
}

/// The counts of `pairs`: each kept from the last counting where it was
/// counted there, else counted now by `count`. One that can't be counted,
/// as when the checkout lacks a commit until it's fetched, is left out
/// and tried again next time.
pub fn count(
    pairs: &[Pair],
    kept: Option<&Counted>,
    now: u64,
    mut count: impl FnMut(&Pair) -> Option<Count>,
) -> Counted {
    let counts = pairs
        .iter()
        .filter_map(|pair| {
            let key = pair.key();
            kept.and_then(|kept| kept.counts.get(&key).copied())
                .or_else(|| count(pair))
                .map(|count| (key, count))
        })
        .collect();
    Counted {
        schema: SCHEMA.to_owned(),
        counted_at: now,
        counts,
    }
}

/// Counts a pair with the `git` at `git`: the commits only the upstream
/// has, then those only the head has, each stopping at `COUNTED_TO`. Only
/// a commit's name in hex reaches git's arguments, and git reads no
/// repository but the checkout's, whatever the caller's environment says.
pub fn by_git(git: &OsStr) -> impl Fn(&Pair) -> Option<Count> + use<> {
    let git = git.to_owned();
    move |pair| {
        if !(commit_name(&pair.upstream) && commit_name(&pair.head)) {
            return None;
        }
        let only = |from: &str, to: &str| -> Option<u32> {
            let answer = Command::new(&git)
                .arg("-C")
                .arg(&pair.checkout)
                .args([
                    "rev-list",
                    "--count",
                    &format!("--max-count={COUNTED_TO}"),
                    &format!("{from}..{to}"),
                    "--",
                ])
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_COMMON_DIR")
                .env_remove("GIT_INDEX_FILE")
                .env_remove("GIT_OBJECT_DIRECTORY")
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()?;
            if !answer.status.success() {
                return None;
            }
            String::from_utf8(answer.stdout).ok()?.trim().parse().ok()
        };
        Some(Count {
            behind: only(&pair.head, &pair.upstream)?,
            ahead: only(&pair.upstream, &pair.head)?,
        })
    }
}

/// Whether `name` is a whole commit's name, as a ref holds it: 40 hex
/// digits, or 64 in a SHA-256 repository.
fn commit_name(name: &str) -> bool {
    matches!(name.len(), 40 | 64) && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Where a counting keeps what it counted:
/// `$XDG_STATE_HOME/repos/upstreams.json`, else under `home`'s
/// ~/.local/state; none when neither is set.
pub fn counted_file(home: Option<&Path>) -> Option<PathBuf> {
    kept::file(home, "upstreams.json")
}

/// The keeping of what a counting counted at `file`, whole or not at all,
/// and readable by its owner alone, since it names private branches'
/// commits.
pub fn keep(file: &Path, counted: &Counted) -> io::Result<()> {
    kept::write(
        file,
        &serde_json::to_vec(counted).map_err(io::Error::other)?,
    )
}

/// What the last counting kept at `file`, where one has and it reads as
/// one.
pub fn read(file: &Path) -> Option<Counted> {
    let text = std::fs::read_to_string(file).ok()?;
    serde_json::from_str::<Counted>(&text)
        .ok()
        .filter(|counted| counted.schema == SCHEMA)
}

/// `counted` merged into the record: each head waiting on a pair it holds
/// takes its count.
pub fn merge(record: &mut Record, counted: &Counted) {
    for head in heads(record) {
        let Some(key) = apart(head).map(|(upstream, commit)| key(upstream, commit)) else {
            continue;
        };
        if let (Some(count), Some(upstream)) = (counted.counts.get(&key), head.upstream.as_mut()) {
            upstream.count = Some(*count);
        }
    }
    record.sources.push(Source {
        name: "upstreams".to_owned(),
        what: "how far each head is from the branch it follows".to_owned(),
        read_at: Some(counted.counted_at),
        unread: Vec::new(),
    });
}

/// The record with what a counting kept at `file` merged in, as every
/// reader of the record takes it. No file means no counting has run here,
/// and only heads even with their upstreams say how far they are; a file
/// no counting wrote is said unread.
pub fn merge_kept(record: &mut Record, file: &Path) {
    if !file.exists() {
        return;
    }
    match read(file) {
        Some(counted) => merge(record, &counted),
        None => record.sources.push(Source {
            name: "upstreams".to_owned(),
            what: "how far each head is from the branch it follows".to_owned(),
            read_at: None,
            unread: vec![format!("{} isn't a counting's record", file.display())],
        }),
    }
}

// Its git helper builds the weighing's repository too (weight/tests.rs).
#[cfg(test)]
pub(crate) mod tests;
