//! Code's rows over the socket: every repository under the roots, each
//! once, in a first watch's own answer; a repository cloned since told to
//! a later watch, read behind it; the rows searched by owner; the open
//! changes a refresh kept, said beside their repository; and New
//! Terminal Here started by a stand-in for niri as it's asked.

use std::path::Path;
use std::process::Command;

use daemon_client::until;
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

#[test]
fn the_changes_a_refresh_kept_are_counted_beside_their_repository() {
    let running = Running::new("code-changes");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    let change = |title: &str| {
        json!({"title": title, "author": "someone", "draft": false, "branch": title,
            "base": "main", "commit": null, "updated_at": 30,
            "checks": {"state": "passing", "passed": 1, "failed": [], "pending": []},
            "review": "waiting"})
    };
    let kept = running.path("state/repos/github.json");
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    let seen = json!({"schema": "repos.github/1", "read_at": 20, "unread": [],
        "changes": {"github:someone/alpha": [change("one"), change("two")]}});
    std::fs::write(&kept, seen.to_string()).unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = row_of(&answer, "row:code:github:someone/alpha")
        .unwrap_or_else(|| panic!("a first watch isn't answered with Code's rows: {answer}"));
    assert_eq!(
        alpha["row"]["side"][0],
        json!({"said": "2 changes"}),
        "the changes a refresh kept aren't counted beside their repository: {alpha}"
    );
    let acts = alpha["row"]["acts"].as_array().unwrap();
    assert!(
        acts.contains(
            &json!({"key": "verb:code:changes", "open": "https://github.com/someone/alpha/pulls"})
        ),
        "a repository with open changes doesn't offer their list: {alpha}"
    );
}

#[test]
fn new_terminal_here_has_niri_start_kitty_in_the_checkout() {
    let running = Running::new("code-terminal");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:terminal", "path": "/src/alpha"}));
    assert!(
        said.get("error").is_none(),
        "New Terminal Here was refused: {said}"
    );
    let asked = || std::fs::read_to_string(running.path("niri.asked"));
    until("niri wasn't asked to start a terminal", || asked().is_ok());
    assert_eq!(
        asked().unwrap(),
        "msg action spawn -- kitty --directory /src/alpha\n",
        "niri wasn't asked to start kitty in the checkout, so it outlives the daemon"
    );
}
