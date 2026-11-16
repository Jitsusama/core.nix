use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use repos::model::{Checkout, Head, Record, Repository, SCHEMA, Tree, TreeKind};

use super::{Folder, Heard, folders};

/// A folder of the test's own, emptied first, so no two tests share one.
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-heard-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// A checkout's git folder as git leaves it, written by hand: HEAD, its
/// refs a folder deep, and one tree in `worktrees/`.
fn checkout(at: &Path) -> PathBuf {
    let git = at.join("core.nix/.git");
    for folder in [
        "refs/heads/joel",
        "refs/remotes/origin",
        "worktrees/say-trees",
        "objects",
    ] {
        fs::create_dir_all(git.join(folder)).expect("a git folder");
    }
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    fs::write(git.join("refs/heads/main"), "aaaa\n").expect("a branch");
    fs::write(
        git.join("worktrees/say-trees/HEAD"),
        "ref: refs/heads/joel/x\n",
    )
    .expect("HEAD");
    git
}

fn record(at: &Path) -> Record {
    let head = Head {
        branch: Some("main".to_owned()),
        commit: Some("aaaa".to_owned()),
        upstream: None,
    };
    Record {
        schema: SCHEMA.to_owned(),
        read_at: 0,
        machine: "laptop".to_owned(),
        roots: Vec::new(),
        sources: Vec::new(),
        repositories: vec![Repository {
            key: "github:Jitsusama/core.nix".to_owned(),
            name: "core.nix".to_owned(),
            owner: Some("Jitsusama".to_owned()),
            remotes: Vec::new(),
            trunk: Some("main".to_owned()),
            written_at: None,
            checkouts: vec![Checkout {
                path: at.join("core.nix").display().to_string(),
                bare: false,
                head: Some(head.clone()),
                written_at: None,
                trees: vec![Tree {
                    path: at.join("trees/say-trees").display().to_string(),
                    kind: TreeKind::Worktree,
                    head,
                    present: true,
                    written_at: None,
                }],
            }],
            changes: Vec::new(),
            read_through: None,
        }],
    }
}

#[test]
fn a_checkout_is_watched_by_its_git_folder_refs_and_trees_never_its_objects() {
    let at = place("folders");
    let git = checkout(&at);
    let kept = [
        at.join("state/github.json"),
        at.join("state/upstreams.json"),
    ];
    let named = folders(&record(&at), &kept);
    let expected = [
        (git.clone(), Folder::Git),
        (git.join("refs"), Folder::Refs),
        (git.join("refs/heads"), Folder::Refs),
        (git.join("refs/heads/joel"), Folder::Refs),
        (git.join("refs/remotes"), Folder::Refs),
        (git.join("refs/remotes/origin"), Folder::Refs),
        (git.join("worktrees"), Folder::Trees),
        (git.join("worktrees/say-trees"), Folder::Git),
        (
            at.join("state"),
            Folder::Kept(BTreeSet::from([
                "github.json".to_owned(),
                "upstreams.json".to_owned(),
            ])),
        ),
    ];
    assert_eq!(named.into_iter().collect::<Vec<_>>(), expected);
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn only_the_names_a_reading_reads_are_heeded() {
    for name in ["HEAD", "packed-refs", "config", "worktrees"] {
        assert!(Folder::Git.heeds(name), "{name} is heeded");
    }
    for name in [
        "index",
        "FETCH_HEAD",
        "ORIG_HEAD",
        "COMMIT_EDITMSG",
        "HEAD.lock",
        "objects",
    ] {
        assert!(!Folder::Git.heeds(name), "{name} isn't heeded");
    }
    assert!(Folder::Refs.heeds("main"), "a branch's ref is heeded");
    assert!(
        !Folder::Refs.heeds("main.lock"),
        "a ref's lock isn't heeded, only its rename"
    );
    assert!(
        Folder::Reftable.heeds("tables.list"),
        "a reftable's list is heeded"
    );
    assert!(
        !Folder::Reftable.heeds("0x01-0x02-1234.ref"),
        "a reftable's tables aren't heeded, only the list naming them"
    );
    let kept = Folder::Kept(BTreeSet::from(["github.json".to_owned()]));
    assert!(kept.heeds("github.json"), "a kept record is heeded");
    assert!(
        !kept.heeds(".github.json.new"),
        "a kept record's copy before its rename isn't heeded"
    );
}

#[test]
fn a_branch_landing_is_heard_and_an_index_written_is_not() {
    let at = place("landing");
    let git = checkout(&at);
    let (tell, told) = mpsc::channel();
    let mut heard = Heard::start(tell).expect("inotify");
    heard.follow(folders(&record(&at), &[]));

    fs::write(git.join("index"), "written at a status").expect("an index");
    assert!(
        told.recv_timeout(Duration::from_millis(300)).is_err(),
        "an index written isn't heard"
    );

    // As git moves a branch: its lock written, then renamed into place.
    fs::write(git.join("refs/heads/main.lock"), "bbbb\n").expect("a lock");
    fs::rename(
        git.join("refs/heads/main.lock"),
        git.join("refs/heads/main"),
    )
    .expect("landed");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "a branch landing is heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_folder_of_refs_made_after_is_heard_once_followed() {
    let at = place("followed");
    let git = checkout(&at);
    let (tell, told) = mpsc::channel();
    let mut heard = Heard::start(tell).expect("inotify");
    heard.follow(folders(&record(&at), &[]));
    let watched = heard.watched();

    fs::create_dir_all(git.join("refs/heads/new")).expect("a folder of refs");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "the folder's making is heard"
    );
    heard.follow(folders(&record(&at), &[]));
    assert_eq!(heard.watched(), watched + 1, "the new folder is watched");
    while told.try_recv().is_ok() {}

    fs::write(git.join("refs/heads/new/branch"), "cccc\n").expect("a branch in it");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "a branch in the new folder is heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_folder_no_longer_named_is_given_back() {
    let at = place("given-back");
    let git = checkout(&at);
    let (tell, told) = mpsc::channel();
    let mut heard = Heard::start(tell).expect("inotify");
    heard.follow(folders(&record(&at), &[]));
    let mut gone = record(&at);
    gone.repositories.clear();
    heard.follow(folders(&gone, &[]));
    assert_eq!(heard.watched(), 0);
    while told.try_recv().is_ok() {}

    fs::write(git.join("HEAD"), "ref: refs/heads/other\n").expect("HEAD");
    assert!(
        told.recv_timeout(Duration::from_millis(300)).is_err(),
        "a checkout no longer read isn't heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}
