//! Code's rows over the socket: every repository under the roots, each
//! once, in a first watch's own answer; a repository cloned since told to
//! a later watch, read behind it; and the rows searched by owner.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::Running;

fn git_in(folder: &Path, args: &[&str]) {
    std::fs::create_dir_all(folder).unwrap();
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .status();
    assert!(ran.unwrap().success(), "git {args:?} failed");
}

/// A checkout under the case's src with a remote on GitHub.
fn cloned(running: &Running, folder: &str, remote: &str) {
    let path = running.path(&format!("src/{folder}"));
    git_in(&path, &["init", "--quiet"]);
    git_in(&path, &["remote", "add", "origin", remote]);
}

fn row_of<'a>(answer: &'a Value, key: &str) -> Option<&'a Value> {
    answer["records"]
        .as_array()?
        .iter()
        .find(|record| record["key"] == key)
}

#[test]
fn every_repository_is_a_row_of_codes_in_a_first_watchs_answer() {
    let running = Running::new("code");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    cloned(&running, "again/alpha", "https://github.com/someone/alpha");
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = row_of(&answer, "row:code:github:someone/alpha")
        .unwrap_or_else(|| panic!("a first watch isn't answered with Code's rows: {answer}"));
    assert_eq!(alpha["row"]["name"], "alpha");
    assert_eq!(
        alpha["row"]["acts"][0],
        json!({"key": "verb:code:page", "open": "https://github.com/someone/alpha"}),
        "a repository's Return doesn't open its page: {alpha}"
    );
    let rows = answer["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["kind"] == "row")
        .count();
    assert_eq!(
        rows, 1,
        "two checkouts of one repository aren't one row: {answer}"
    );

    cloned(&running, "beta", "git@github.com:other/beta.git");
    // The read behind the watch may end before the watch's answer is made,
    // which then holds it, or after, which tells it.
    let mut later = running.client();
    let answer = later.ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "code"}));
    let beta = row_of(&answer, "row:code:github:other/beta")
        .cloned()
        .unwrap_or_else(|| later.changed("row:code:github:other/beta"));
    assert_eq!(
        beta["row"]["name"], "beta",
        "a repository cloned since isn't told: {beta}"
    );

    let said = later.ask(&json!({"id": 3, "ask": "search", "place": "code", "words": "other"}));
    assert_eq!(
        said["keys"],
        json!(["row:code:github:other/beta"]),
        "a repository isn't found by its owner: {said}"
    );
}
