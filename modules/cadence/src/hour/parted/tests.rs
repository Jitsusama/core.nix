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
    assert_eq!(
        shared(clone, url, 7),
        Ok(vec!["b".to_owned()]),
        "the change and main share only b"
    );
}

#[test]
fn a_change_main_moved_nothing_beside_shares_nothing() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (dir, clone, url) = &history(root.path());
    change(dir, 8, &|| drop(commit(dir, "a", "ours\n")), &|| {
        commit(dir, "c", "main's\n");
    });
    assert_eq!(shared(clone, url, 8), Ok(Vec::new()), "nothing is shared");
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
        shared(clone, url, 9),
        Ok(vec!["a".to_owned()]),
        "a rename beside main's edit went unseen"
    );
}

#[test]
fn a_change_that_cannot_be_fetched_or_no_clone_says_why() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let (_, clone, url) = &history(root.path());
    let missing = shared(clone, url, 404);
    assert!(
        missing
            .as_ref()
            .is_err_and(|why| why.contains("fetching the change")),
        "a change the host doesn't hold read as {missing:?}"
    );
    let none = shared(&root.path().join("none.git"), url, 7);
    assert!(
        none.as_ref().is_err_and(|why| why.contains("no clone")),
        "no clone read as {none:?}"
    );
}
