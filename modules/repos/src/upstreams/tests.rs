//! The counting: which pairs it counts, what it keeps, and how a reading
//! takes it in, over a stand-in counter; and git's own counting, over a
//! small repository built for the test.

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use super::*;
use crate::model::{Checkout, Repository, Tree, TreeKind, Upstream};

const TRUNK: &str = "1111111111111111111111111111111111111111";
const BEHIND: &str = "2222222222222222222222222222222222222222";
const AHEAD: &str = "3333333333333333333333333333333333333333";

fn head(commit: &str, upstream: Option<&str>) -> Head {
    Head {
        branch: Some("work".to_owned()),
        commit: Some(commit.to_owned()),
        upstream: Some(Upstream {
            branch: "origin/main".to_owned(),
            commit: upstream.map(str::to_owned),
            count: None,
        }),
    }
}

fn tree(path: &str, head: Head, present: bool) -> Tree {
    Tree {
        path: path.to_owned(),
        kind: TreeKind::Worktree,
        head,
        present,
        written_at: None,
    }
}

/// A record of one repository whose checkout is at `own` and has `trees`.
fn record(own: Head, trees: Vec<Tree>) -> Record {
    Record {
        schema: crate::model::SCHEMA.to_owned(),
        read_at: 0,
        machine: "here".to_owned(),
        roots: Vec::new(),
        sources: Vec::new(),
        repositories: vec![Repository {
            key: "github:o/r".to_owned(),
            name: "r".to_owned(),
            owner: Some("o".to_owned()),
            remotes: Vec::new(),
            trunk: None,
            written_at: None,
            checkouts: vec![Checkout {
                path: "/src/r".to_owned(),
                bare: false,
                head: Some(own),
                written_at: None,
                trees,
            }],
            changes: Vec::new(),
            read_through: None,
        }],
    }
}

fn kept_with(pairs: &[(&str, &str, Count)]) -> Counted {
    Counted {
        schema: SCHEMA.to_owned(),
        counted_at: 1,
        counts: pairs
            .iter()
            .map(|(upstream, head, count)| (key(upstream, head), *count))
            .collect(),
    }
}

#[test]
fn only_heads_apart_from_their_upstreams_are_counted_each_pair_once() {
    let record = record(
        head(BEHIND, Some(TRUNK)),
        vec![
            tree("/w/same", head(BEHIND, Some(TRUNK)), true),
            tree("/w/even", head(TRUNK, Some(TRUNK)), true),
            tree("/w/gone-upstream", head(AHEAD, None), true),
            tree("/w/gone", head(AHEAD, Some(TRUNK)), false),
        ],
    );
    let pairs = pairs(&record);
    assert_eq!(
        pairs,
        [Pair {
            checkout: PathBuf::from("/src/r"),
            upstream: TRUNK.to_owned(),
            head: BEHIND.to_owned(),
        }],
        "an even head, a gone upstream or a gone tree was counted, or a pair twice"
    );
}

#[test]
fn a_counting_reuses_what_it_kept_and_drops_what_no_head_waits_on() {
    let pairs = pairs(&record(
        head(BEHIND, Some(TRUNK)),
        vec![tree("/w/ahead", head(AHEAD, Some(TRUNK)), true)],
    ));
    let kept = kept_with(&[
        (
            TRUNK,
            BEHIND,
            Count {
                behind: 4,
                ahead: 0,
            },
        ),
        (
            AHEAD,
            BEHIND,
            Count {
                behind: 9,
                ahead: 9,
            },
        ),
    ]);
    let asked = RefCell::new(Vec::new());
    let counted = count(&pairs, Some(&kept), 7, |pair| {
        asked.borrow_mut().push(pair.head.clone());
        Some(Count {
            behind: 0,
            ahead: 2,
        })
    });
    assert_eq!(asked.into_inner(), [AHEAD], "a kept pair was counted again");
    assert_eq!(
        counted,
        kept_with(&[
            (
                TRUNK,
                BEHIND,
                Count {
                    behind: 4,
                    ahead: 0
                }
            ),
            (
                TRUNK,
                AHEAD,
                Count {
                    behind: 0,
                    ahead: 2
                }
            ),
        ])
        .with_time(7),
        "a stale pair was kept, or a counted one lost"
    );
}

#[test]
fn a_pair_git_cant_count_is_left_to_the_next_counting() {
    let pairs = pairs(&record(head(BEHIND, Some(TRUNK)), Vec::new()));
    let counted = count(&pairs, None, 7, |_| None);
    assert!(counted.counts.is_empty(), "{counted:?}");
}

impl Counted {
    fn with_time(mut self, counted_at: u64) -> Self {
        self.counted_at = counted_at;
        self
    }
}

#[test]
fn a_reading_takes_each_heads_count_from_its_pair() {
    let mut record = record(
        head(BEHIND, Some(TRUNK)),
        vec![tree("/w/other", head(AHEAD, Some(TRUNK)), true)],
    );
    let behind = Count {
        behind: 4,
        ahead: 1,
    };
    merge(&mut record, &kept_with(&[(TRUNK, BEHIND, behind)]));
    let checkout = &record.repositories[0].checkouts[0];
    let count_of = |head: &Head| head.upstream.as_ref().and_then(|upstream| upstream.count);
    assert_eq!(
        count_of(checkout.head.as_ref().expect("its head")),
        Some(behind)
    );
    assert_eq!(
        count_of(&checkout.trees[0].head),
        None,
        "an uncounted pair took a count"
    );
    let source = record
        .sources
        .last()
        .expect("the counting said as a source");
    assert_eq!(
        (source.name.as_str(), source.read_at),
        ("upstreams", Some(1))
    );
}

/// A folder of the test's own, emptied first.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-upstreams-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a temporary folder");
    dir
}

#[test]
fn what_isnt_a_countings_record_is_said_unread_and_none_is_quiet() {
    let dir = folder("unread");
    let file = dir.join("upstreams.json");
    let mut record = record(head(BEHIND, Some(TRUNK)), Vec::new());
    merge_kept(&mut record, &file);
    assert!(
        record.sources.is_empty(),
        "no counting yet was said: {:?}",
        record.sources
    );
    fs::write(
        &file,
        r#"{"schema":"repos.upstreams/0","counted_at":1,"counts":{}}"#,
    )
    .expect("a file written");
    merge_kept(&mut record, &file);
    let source = record.sources.last().expect("the file said");
    assert_eq!(source.name, "upstreams");
    assert_eq!(source.unread.len(), 1, "{source:?}");
}

#[test]
fn a_kept_counting_reads_back_whole_and_only_its_owner_can() {
    let dir = folder("kept");
    let file = dir.join("repos/upstreams.json");
    let counted = kept_with(&[(
        TRUNK,
        BEHIND,
        Count {
            behind: 4,
            ahead: 1,
        },
    )]);
    keep(&file, &counted).expect("kept");
    assert_eq!(read(&file), Some(counted));
    let mode = fs::metadata(&file).expect("its file").permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

/// git, as the test runs it: on an empty config, so the person's own,
/// signing included, is never read, and with fixed names and times.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(git_program())
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("git's words")
        .trim()
        .to_owned()
}

fn git_program() -> OsString {
    std::env::var_os("REPOS_GIT").unwrap_or_else(|| "git".into())
}

/// `count` empty commits on top of what the repository is at, each named
/// for `line` so two lines from one commit never make the same one, as
/// their times are fixed.
fn commits(dir: &Path, line: &str, count: usize) -> String {
    for at in 0..count {
        git(
            dir,
            &[
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                &format!("{line} {at}"),
            ],
        );
    }
    git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn git_counts_behind_and_ahead_each_way_and_stops_at_its_end() {
    let dir = folder("git");
    git(&dir, &["init", "--quiet", "--initial-branch=main"]);
    let base = commits(&dir, "base", 1);
    let upstream = commits(&dir, "upstream", 3);
    git(&dir, &["checkout", "--quiet", "-b", "work", &base]);
    let head = commits(&dir, "head", 2);
    let by_git = by_git(&git_program());
    let pair = |upstream: &str, head: &str| Pair {
        checkout: dir.clone(),
        upstream: upstream.to_owned(),
        head: head.to_owned(),
    };
    assert_eq!(
        by_git(&pair(&upstream, &head)),
        Some(Count {
            behind: 3,
            ahead: 2
        }),
        "behind and ahead were swapped, or miscounted"
    );
    git(&dir, &["checkout", "--quiet", "main"]);
    let far = commits(&dir, "far", COUNTED_TO as usize + 5);
    assert_eq!(
        by_git(&pair(&far, &base)),
        Some(Count {
            behind: COUNTED_TO,
            ahead: 0
        }),
        "a count ran past its end"
    );
    assert_eq!(
        by_git(&pair(&"f".repeat(40), &base)),
        None,
        "a commit the checkout lacks was counted"
    );
    assert_eq!(
        by_git(&pair("--all", &base)),
        None,
        "a name not a commit reached git"
    );
}
