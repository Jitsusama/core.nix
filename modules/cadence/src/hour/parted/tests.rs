//! What a change and main share, asked of a clone of a repository made
//! here, whose change heads sit where the host keeps a change's.

use std::path::{Path, PathBuf};

use super::super::moved::tests::{commit, git};
use super::*;

/// A repository `here` in `root` whose main holds `a`, `b` and `c`, and
/// a bare clone of it as the tick keeps one, with the address to fetch
/// from.
fn history(root: &Path) -> (PathBuf, PathBuf, String) {
    let dir = root.join("here");
    std::fs::create_dir(&dir).expect("a repository folder");
    git(&dir, &["init", "-q", "-b", "main"]);
    for file in ["a", "b", "c"] {
        commit(&dir, file, "one\n");
    }
    let url = format!("file://{}", dir.display());
    let clone = root.join("core.git");
    git(root, &["clone", "--bare", "--quiet", &url, "core.git"]);
    (dir, clone, url)
}

/// Change `number` in `dir`, made from main by `work`, kept where the
/// host keeps a change's head, then main given `beside`.
fn change(dir: &Path, number: u32, work: &dyn Fn(), beside: &dyn Fn()) {
    let branch = format!("change-{number}");
    git(dir, &["checkout", "-q", "-b", &branch, "main"]);
    work();
    let head = git(dir, &["rev-parse", "HEAD"]);
    let kept = format!("refs/pull/{number}/head");
    git(dir, &["update-ref", &kept, &head]);
    git(dir, &["checkout", "-q", "main"]);
    beside();
}

#[test]
fn only_the_files_both_sides_changed_since_they_parted_are_shared() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(
        dir,
        7,
        &|| {
            commit(dir, "a", "ours\n");
            commit(dir, "b", "ours\n");
        },
        &|| {
            commit(dir, "b", "main's\n");
            commit(dir, "c", "main's\n");
        },
    );
    let tick = git(clone, &["rev-parse", "refs/heads/main"]);
    assert_eq!(
        shared(clone, url, 7, true),
        Ok(vec!["b".to_owned()]),
        "the change and main share only b"
    );
    assert_eq!(
        git(clone, &["rev-parse", "refs/heads/main"]),
        tick,
        "the tick's main was moved"
    );
}

#[test]
fn a_file_main_renamed_is_shared_under_its_old_name() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(dir, 10, &|| drop(commit(dir, "b", "ours\n")), &|| {
        git(dir, &["mv", "b", "y"]);
        git(dir, &["commit", "-q", "-m", "b moved"]);
    });
    assert_eq!(
        shared(clone, url, 10, true),
        Ok(vec!["b".to_owned()]),
        "main's rename beside the change's edit went unseen"
    );
}

#[test]
fn a_head_moved_by_force_is_read_anew_and_main_as_last_fetched_without_it() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(dir, 11, &|| drop(commit(dir, "a", "ours\n")), &|| {
        commit(dir, "c", "main's\n");
    });
    assert_eq!(
        shared(clone, url, 11, true),
        Ok(Vec::new()),
        "shared too soon"
    );
    commit(dir, "a", "main's\n");
    git(dir, &["checkout", "-q", "change-11"]);
    git(dir, &["reset", "-q", "--hard", "main~2"]);
    commit(dir, "a", "ours again\n");
    let head = commit(dir, "c", "ours\n");
    git(dir, &["update-ref", "refs/pull/11/head", &head]);
    git(dir, &["checkout", "-q", "main"]);
    assert_eq!(
        shared(clone, url, 11, false),
        Ok(vec!["c".to_owned()]),
        "the forced head wasn't read anew, or main moved though not fetched"
    );
    assert_eq!(
        shared(clone, url, 11, true),
        Ok(vec!["a".to_owned(), "c".to_owned()]),
        "main fetched anew wasn't read"
    );
}

#[test]
fn only_the_heads_this_hour_asked_of_are_kept() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    for number in [12, 13] {
        change(dir, number, &|| drop(commit(dir, "a", "ours\n")), &|| {});
        assert!(shared(clone, url, number, true).is_ok(), "{number} unread");
    }
    let asked = BTreeMap::from([(13, Ok(Vec::new()))]);
    let_go(clone, &asked);
    assert_eq!(
        git(clone, &["for-each-ref", "--format=%(refname)", HEADS]),
        "refs/cadence/pull/13",
        "a head not asked of this hour was kept, or one asked of let go"
    );
}

#[test]
fn a_change_main_moved_nothing_beside_shares_nothing() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(dir, 8, &|| drop(commit(dir, "a", "ours\n")), &|| {
        commit(dir, "c", "main's\n");
    });
    assert_eq!(shared(clone, url, 8, true), Ok(Vec::new()), "shared some");
}

#[test]
fn a_file_the_change_renamed_is_shared_under_its_old_name() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(
        dir,
        9,
        &|| {
            git(dir, &["mv", "a", "z"]);
            git(dir, &["commit", "-q", "-m", "a moved"]);
        },
        &|| drop(commit(dir, "a", "main's\n")),
    );
    assert_eq!(
        shared(clone, url, 9, true),
        Ok(vec!["a".to_owned()]),
        "a rename beside main's edit went unseen"
    );
}

#[test]
fn a_change_that_cannot_be_fetched_or_no_clone_says_why() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (_, clone, url) = &history(root.path());
    let missing = shared(clone, url, 404, true);
    assert!(
        missing
            .as_ref()
            .is_err_and(|why| why.contains("fetching the change")),
        "a change the host doesn't hold read as {missing:?}"
    );
    let none = shared(&root.path().join("none.git"), url, 7, true);
    assert!(
        none.as_ref().is_err_and(|why| why.contains("no clone")),
        "no clone read as {none:?}"
    );
}
