//! A yes that stands, as the shell's Passwords child answers it: Joel's
//! yes to an ask lets the same ask from a clean tree of that repository
//! on origin's main run without him, the twin already waiting included;
//! Asks lists the standing yes, and its Revoke It has the ask wait again.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::{LISTED, answer, put_up_in, row_of, saying, stored_bash, yes};
use crate::common::{Client, Running};

/// Runs git in `tree` as the test's own, with no settings of Joel's, so
/// its commits are made unsigned.
fn git(tree: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(tree)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?} failed: {ran:?}");
}

/// A repository inside the running folder, one commit in and nothing
/// uncommitted, whose commit origin's default branch holds, as a tree on
/// main does once fetched.
fn on_main(running: &Running) -> PathBuf {
    let tree = running.folder.join("repo");
    std::fs::create_dir_all(&tree).unwrap();
    git(&tree, &["init", "-q"]);
    std::fs::write(tree.join("README"), "a tree").unwrap();
    git(&tree, &["add", "README"]);
    git(&tree, &["commit", "-q", "-m", "a commit"]);
    git(&tree, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(
        &tree,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    tree
}

/// The standing yes's row, among the rows held now.
fn grant_row(client: &mut Client) -> Option<Value> {
    let held = client.ask(&json!({"id": "l", "ask": "list", "kind": "row", "held": true}));
    held["records"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| {
            record["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("row:asks:grant:"))
        })
        .map(|record| record["row"].clone())
}

#[test]
fn a_yes_lets_the_same_ask_run_without_joel_until_its_revoked() {
    let running = Running::releasing("asks-granted", &[LISTED]);
    let tree = on_main(&running);
    let argv = saying(&stored_bash());
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    let (first, twin, later, revoked) = (
        "3123456789abcdef0123456789abcdef",
        "3223456789abcdef0123456789abcdef",
        "3323456789abcdef0123456789abcdef",
        "3423456789abcdef0123456789abcdef",
    );
    put_up_in(&running, &tree, twin, LISTED, &argv);
    assert_eq!(
        row_of(&mut running.client(), twin)["state"],
        "waiting",
        "the twin didn't wait before any yes"
    );
    put_up_in(&running, &tree, first, LISTED, &argv);
    let mut client = running.client();
    let row = row_of(&mut client, first);
    let said = client.ask(&yes(&row, 812));
    assert_eq!(said["releasing"], true, "the yes: {said}");
    assert_eq!(
        answer(&running, first)["state"],
        "ran",
        "the yes ran nothing"
    );
    let answered = answer(&running, twin);
    assert_eq!(
        (&answered["state"], &answered["granted"]),
        (&json!("ran"), &json!(true)),
        "the twin already waiting wasn't let through by the yes: {answered}"
    );
    put_up_in(&running, &tree, later, LISTED, &argv);
    let answered = answer(&running, later);
    assert_eq!(
        (&answered["state"], &answered["granted"]),
        (&json!("ran"), &json!(true)),
        "the same ask later still waited on Joel: {answered}"
    );
    let grant = grant_row(&mut client).expect("Asks lists no standing yes");
    let mut revoke = grant["acts"][0]["ask"].clone();
    revoke["id"] = json!(8);
    let said = client.ask(&revoke);
    assert_eq!(said["revoked"], true, "the revoke: {said}");
    put_up_in(&running, &tree, revoked, LISTED, &argv);
    assert_eq!(
        row_of(&mut running.client(), revoked)["state"],
        "waiting",
        "a revoked yes still let the ask through"
    );
}
