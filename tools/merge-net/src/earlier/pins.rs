//! Whether a head only pins newer commits of repos whose own changes our
//! rounds already review, which then needs no round of its own. Such a
//! head is a flake.lock that moves inputs and nothing else: what it brings
//! in was read by a council of ours where it merged, so a round here would
//! read only hashes. On 22 November a mycelium pin on the first VM test's
//! path waited 20 minutes for one, and the velocity cell ruled that it
//! needs none (1OGSE7, 2c8f8e4a): it asks no one else to review anything.
//!
//! Only a node's lock may move, from one commit of a repo of ours to
//! another of the same repo. A node added, removed or renamed, an input
//! that follows another, or a source of anyone else's needs a round, since
//! that brings in something no round of ours read.

use std::path::Path;

use serde_json::Value;

use crate::{git_at, short};

/// The repos whose every change merges through a round of ours, by their
/// owner and name on GitHub.
const OURS: [&str; 2] = ["Jitsusama/mycelium", "Jitsusama/agentic-harness.pi"];

/// The lock file a pin moves.
const LOCK: &str = "flake.lock";

/// Why `commit`, in the repo at `repo`, needs no round of ours, when its
/// change from its base on `main` only pins newer commits of [`OURS`].
pub fn ours_alone(repo: &Path, commit: &str, main: &str) -> Option<String> {
    let base = git_at(repo, &["merge-base", main, commit]).ok()?;
    let files = git_at(repo, &["diff", "--name-only", &base, commit]).ok()?;
    if files != LOCK {
        return None;
    }
    let lock = |at: &str| git_at(repo, &["show", &format!("{at}:{LOCK}")]).ok();
    let moved = moved_ours(&lock(&base)?, &lock(commit)?)?;
    Some(format!(
        "{} moves only {LOCK}, to newer commits of {}, whose own changes merge through \
         rounds of ours, so it needs no round of its own",
        short(commit),
        moved.join(" and ")
    ))
}

/// The repos of ours whose locks moved from `before` to `after`, two
/// flake.lock files, when nothing else in them did.
fn moved_ours(before: &str, after: &str) -> Option<Vec<String>> {
    let before: Value = serde_json::from_str(before).ok()?;
    let after: Value = serde_json::from_str(after).ok()?;
    let (was, now) = (before["nodes"].as_object()?, after["nodes"].as_object()?);
    if but(&before, "nodes") != but(&after, "nodes") || !was.keys().eq(now.keys()) {
        return None;
    }
    let mut moved = Vec::new();
    for (name, node) in now {
        let old = &was[name];
        if old == node {
            continue;
        }
        let repo = ours(old)?;
        if ours(node)? != repo || but(old, "locked") != but(node, "locked") {
            return None;
        }
        moved.push(repo);
    }
    (!moved.is_empty()).then_some(moved)
}

/// The fields of the object `value` but `left`, or nothing if it isn't one.
fn but<'a>(value: &'a Value, left: &str) -> Option<Vec<(&'a String, &'a Value)>> {
    value
        .as_object()
        .map(|fields| fields.iter().filter(|(key, _)| *key != left).collect())
}

/// The repo of ours `node` is locked to, if it is one.
fn ours(node: &Value) -> Option<String> {
    let locked = &node["locked"];
    if locked["type"].as_str() != Some("github") {
        return None;
    }
    let repo = format!("{}/{}", locked["owner"].as_str()?, locked["repo"].as_str()?);
    OURS.contains(&repo.as_str()).then_some(repo)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A lock whose mycelium is at `rev`, beside nixpkgs at `nixpkgs`.
    fn lock(rev: &str, nixpkgs: &str) -> Value {
        json!({
            "nodes": {
                "mycelium": {
                    "inputs": {"nixpkgs": ["nixpkgs"]},
                    "locked": {"owner": "Jitsusama", "repo": "mycelium", "rev": rev,
                        "narHash": format!("sha256-{rev}"), "type": "github"},
                    "original": {"owner": "Jitsusama", "repo": "mycelium", "type": "github"}
                },
                "nixpkgs": {
                    "locked": {"owner": "NixOS", "repo": "nixpkgs", "rev": nixpkgs,
                        "type": "github"},
                    "original": {"owner": "NixOS", "repo": "nixpkgs", "type": "github"}
                },
                "root": {"inputs": {"mycelium": "mycelium", "nixpkgs": "nixpkgs"}}
            },
            "root": "root",
            "version": 7
        })
    }

    fn moved(before: &Value, after: &Value) -> Option<Vec<String>> {
        moved_ours(&before.to_string(), &after.to_string())
    }

    #[test]
    fn a_newer_commit_of_a_repo_of_ours_needs_no_round() {
        assert_eq!(
            moved(&lock("a", "n"), &lock("b", "n")),
            Some(vec!["Jitsusama/mycelium".to_owned()]),
            "a pin of mycelium alone was asked for a round"
        );
    }

    #[test]
    fn a_lock_moving_anyone_elses_source_needs_a_round() {
        assert_eq!(
            moved(&lock("a", "n"), &lock("b", "m")),
            None,
            "a pin moving nixpkgs too went without a round"
        );
    }

    #[test]
    fn a_lock_that_moves_nothing_is_no_pin() {
        assert_eq!(
            moved(&lock("a", "n"), &lock("a", "n")),
            None,
            "a lock that moved nothing was taken for a pin"
        );
    }

    #[test]
    fn a_node_of_ours_pointed_at_another_repo_needs_a_round() {
        let mut after = lock("b", "n");
        after["nodes"]["mycelium"]["locked"]["repo"] = json!("agentic-harness.pi");
        assert_eq!(
            moved(&lock("a", "n"), &after),
            None,
            "a node moved from one repo of ours to another went without a round"
        );
    }

    #[test]
    fn a_pin_that_changes_what_an_input_follows_needs_a_round() {
        let mut after = lock("b", "n");
        after["nodes"]["mycelium"]["inputs"] = json!({"nixpkgs": "nixpkgs_2"});
        assert_eq!(
            moved(&lock("a", "n"), &after),
            None,
            "a pin changing an input's follows went without a round"
        );
    }

    #[test]
    fn a_pin_that_adds_a_node_needs_a_round() {
        let mut after = lock("b", "n");
        after["nodes"]["nixpkgs_2"] = after["nodes"]["nixpkgs"].clone();
        assert_eq!(
            moved(&lock("a", "n"), &after),
            None,
            "a pin adding a node went without a round"
        );
    }

    #[test]
    fn a_pin_to_another_kind_of_source_needs_a_round() {
        let mut after = lock("b", "n");
        after["nodes"]["mycelium"]["locked"]["type"] = json!("git");
        assert_eq!(
            moved(&lock("a", "n"), &after),
            None,
            "a pin moving mycelium off GitHub went without a round"
        );
    }

    #[test]
    fn a_lock_whose_root_changes_needs_a_round() {
        let mut after = lock("b", "n");
        after["version"] = json!(8);
        assert_eq!(
            moved(&lock("a", "n"), &after),
            None,
            "a lock of another version went without a round"
        );
    }
}
