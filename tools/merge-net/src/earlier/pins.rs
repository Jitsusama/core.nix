//! Whether a head only pins newer commits of repos whose own changes our
//! rounds already review, which then needs no round of its own. Such a
//! head is a flake.lock that moves inputs and nothing else: what it brings
//! in was read by a council of ours where it merged, so a round here would
//! read only hashes. On 22 November a mycelium pin on the first VM test's
//! path waited 20 minutes for one, and the velocity cell ruled that it
//! needs none (1OGSE7, 2c8f8e4a): it asks no one else to review anything.
//!
//! Only a node's commit may move: its lock's `rev`, `narHash` and
//! `lastModified`, every other field as it was, so a lock can't send the
//! fetch to another host, folder or ref under our repo's name. The new
//! commit must descend from the old one and sit on the repo's trunk, as
//! GitHub says, since only what merged there went through a round: a
//! branch, a pull request's head, a commit from a fork or a pin back to
//! an older commit needs a round. A node added, removed or renamed, an
//! input that follows another, or a source of anyone else's needs one too.
//! Anything that can't be read or asked needs a round, as before.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::{git_at, short};

/// The repos whose every change merges through a round of ours, by their
/// owner and name on GitHub, each with its trunk. The harness is reviewed
/// the same way but isn't an input of this flake, so it joins only with
/// the change that makes it one.
const OURS: [(&str, &str); 1] = [("Jitsusama/mycelium", "main")];

/// The lock file a pin moves.
const LOCK: &str = "flake.lock";

/// The fields of a node's lock a pin may move: those naming its commit.
const MOVES: [&str; 3] = ["rev", "narHash", "lastModified"];

/// A node of ours whose lock moved from one commit to another.
#[derive(Debug, PartialEq, Eq)]
struct Moved {
    repo: String,
    trunk: String,
    old: String,
    new: String,
}

/// Why `commit`, in the repo at `repo`, needs no round of ours, when its
/// change from its base on `main` only pins newer commits of [`OURS`].
pub fn ours_alone(repo: &Path, commit: &str, main: &str) -> Option<String> {
    alone_asking(repo, commit, main, compare)
}

/// [`ours_alone`], asking `compared` how two commits of a repo on GitHub
/// stand, as its compare API's `status` says of `base...head`.
fn alone_asking(
    repo: &Path,
    commit: &str,
    main: &str,
    compared: impl Fn(&str, &str, &str) -> Option<String>,
) -> Option<String> {
    let base = git_at(repo, &["merge-base", main, commit]).ok()?;
    let files = git_at(repo, &["diff", "--name-only", &base, commit]).ok()?;
    if files != LOCK {
        return None;
    }
    let lock = |at: &str| git_at(repo, &["show", &format!("{at}:{LOCK}")]).ok();
    let moved = moved_ours(&lock(&base)?, &lock(commit)?)?;
    for pin in &moved {
        // `ahead` says head descends from base and isn't it.
        let newer = compared(&pin.repo, &pin.old, &pin.new)?;
        let merged = compared(&pin.repo, &pin.new, &pin.trunk)?;
        if newer != "ahead" || !matches!(merged.as_str(), "ahead" | "identical") {
            return None;
        }
    }
    let repos: Vec<&str> = moved.iter().map(|pin| pin.repo.as_str()).collect();
    Some(format!(
        "{} moves only {LOCK}, to newer commits on the trunk of {}, whose own changes merge \
         through rounds of ours, so it needs no round of its own",
        short(commit),
        repos.join(" and ")
    ))
}

/// How `head` stands to `base` in `repo` on GitHub, as the compare API's
/// `status` says, or nothing when it can't be asked in two minutes.
fn compare(repo: &str, base: &str, head: &str) -> Option<String> {
    let ran = Command::new("timeout")
        .args(["120", "gh", "api"])
        .arg(format!("repos/{repo}/compare/{base}...{head}"))
        .args(["--jq", ".status"])
        .output()
        .ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}

/// The nodes of ours whose commits moved from `before` to `after`, two
/// flake.lock files, when nothing else in them did.
fn moved_ours(before: &str, after: &str) -> Option<Vec<Moved>> {
    let before: Value = serde_json::from_str(before).ok()?;
    let after: Value = serde_json::from_str(after).ok()?;
    let (was, now) = (before["nodes"].as_object()?, after["nodes"].as_object()?);
    if but(&before, &["nodes"]) != but(&after, &["nodes"]) || !was.keys().eq(now.keys()) {
        return None;
    }
    let mut moved = Vec::new();
    for (name, node) in now {
        let old = &was[name];
        if old == node {
            continue;
        }
        let (repo, trunk) = ours(old)?;
        let (locked, unlocked) = (&old["locked"], &node["locked"]);
        if but(old, &["locked"]) != but(node, &["locked"])
            || but(locked, &MOVES) != but(unlocked, &MOVES)
        {
            return None;
        }
        moved.push(Moved {
            repo: repo.to_owned(),
            trunk: trunk.to_owned(),
            old: commit_of(locked)?,
            new: commit_of(unlocked)?,
        });
    }
    (!moved.is_empty()).then_some(moved)
}

/// The fields of the object `value` but those in `left`, or nothing if it
/// isn't one.
fn but<'a>(value: &'a Value, left: &[&str]) -> Option<Vec<(&'a String, &'a Value)>> {
    value.as_object().map(|fields| {
        fields
            .iter()
            .filter(|(key, _)| !left.contains(&key.as_str()))
            .collect()
    })
}

/// The repo of ours `node` is locked to, with its trunk, if it is one.
fn ours(node: &Value) -> Option<(&'static str, &'static str)> {
    let locked = &node["locked"];
    if locked["type"].as_str() != Some("github") {
        return None;
    }
    let repo = format!("{}/{}", locked["owner"].as_str()?, locked["repo"].as_str()?);
    OURS.iter().copied().find(|(ours, _)| *ours == repo)
}

/// The commit a lock names, when it names one in full: anything shorter
/// could be a branch or a tag, which GitHub would read as of today.
fn commit_of(locked: &Value) -> Option<String> {
    let rev = locked["rev"].as_str()?;
    (rev.len() == 40 && rev.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| rev.to_owned())
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use serde_json::json;

    use super::*;
    use crate::tests::git_in;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    /// A lock whose mycelium is at `rev`, beside nixpkgs at `nixpkgs`.
    fn lock(rev: &str, nixpkgs: &str) -> Value {
        json!({
            "nodes": {
                "mycelium": {
                    "inputs": {"nixpkgs": ["nixpkgs"]},
                    "locked": {"lastModified": 1, "owner": "Jitsusama", "repo": "mycelium",
                        "rev": rev, "narHash": format!("sha256-{rev}"), "type": "github"},
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

    fn moved(before: &Value, after: &Value) -> Option<Vec<Moved>> {
        moved_ours(&before.to_string(), &after.to_string())
    }

    /// Asserts a pin from mycelium at A to `after` needs a round.
    fn needs_a_round(after: &Value, why: &str) {
        assert_eq!(
            moved(&lock(A, "n"), after),
            None,
            "{why} went without a round"
        );
    }

    #[test]
    fn a_newer_commit_of_a_repo_of_ours_is_a_pin_of_ours() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]["lastModified"] = json!(2);
        assert_eq!(
            moved(&lock(A, "n"), &after),
            Some(vec![Moved {
                repo: "Jitsusama/mycelium".to_owned(),
                trunk: "main".to_owned(),
                old: A.to_owned(),
                new: B.to_owned(),
            }]),
            "a pin of mycelium alone wasn't read as one"
        );
    }

    #[test]
    fn a_lock_moving_anyone_elses_source_needs_a_round() {
        needs_a_round(&lock(B, "m"), "a pin moving nixpkgs too");
    }

    #[test]
    fn a_lock_that_moves_nothing_is_no_pin() {
        needs_a_round(&lock(A, "n"), "a lock that moved nothing");
    }

    #[test]
    fn a_node_of_ours_pointed_at_another_repo_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]["repo"] = json!("agentic-harness.pi");
        needs_a_round(&after, "a node moved to another repo");
    }

    #[test]
    fn a_lock_sent_to_another_host_folder_or_ref_needs_a_round() {
        for (field, value) in [("host", "git.example.org"), ("dir", "sub"), ("ref", "x")] {
            let mut after = lock(B, "n");
            after["nodes"]["mycelium"]["locked"][field] = json!(value);
            needs_a_round(&after, &format!("a pin adding {field} to its lock"));
        }
    }

    #[test]
    fn a_pin_to_a_short_rev_needs_a_round() {
        needs_a_round(&lock("main", "n"), "a pin naming a branch for its commit");
    }

    #[test]
    fn a_pin_that_changes_what_an_input_follows_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["inputs"] = json!({"nixpkgs": "nixpkgs_2"});
        needs_a_round(&after, "a pin changing an input's follows");
    }

    #[test]
    fn a_pin_that_changes_its_original_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["original"]["ref"] = json!("other");
        needs_a_round(&after, "a pin changing what flake.nix asked for");
    }

    #[test]
    fn a_pin_that_adds_or_removes_a_node_needs_a_round() {
        let mut before = lock(A, "n");
        before["nodes"]["left"] = before["nodes"]["nixpkgs"].clone();
        let mut after = lock(B, "n");
        assert_eq!(
            moved(&before, &after),
            None,
            "a pin removing a node went without a round"
        );
        after["nodes"]["nixpkgs_2"] = after["nodes"]["nixpkgs"].clone();
        needs_a_round(&after, "a pin adding a node");
    }

    #[test]
    fn a_pin_to_another_kind_of_source_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]["type"] = json!("git");
        needs_a_round(&after, "a pin moving mycelium off GitHub");
    }

    #[test]
    fn a_lock_of_another_version_or_root_needs_a_round() {
        let mut after = lock(B, "n");
        after["version"] = json!(8);
        needs_a_round(&after, "a lock of another version");
        let mut after = lock(B, "n");
        after["root"] = json!("nixpkgs");
        needs_a_round(&after, "a lock of another root");
    }

    /// A repo whose main holds a lock with mycelium at A, and a branch from
    /// it that pins mycelium at B, touching `beside` too when it's given.
    fn pinned(name: &str, beside: Option<&str>) -> (std::path::PathBuf, String) {
        let repo = env::temp_dir().join(format!("merge-net-pins-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join(LOCK), lock(A, "n").to_string()).unwrap();
        fs::write(repo.join("module.nix"), "{ }").unwrap();
        git_in(&repo, &["add", "."]);
        git_in(&repo, &["commit", "-qm", "base"]);
        git_in(&repo, &["checkout", "-qb", "pin"]);
        fs::write(repo.join(LOCK), lock(B, "n").to_string()).unwrap();
        if let Some(file) = beside {
            fs::write(repo.join(file), "{ x = 1; }").unwrap();
        }
        git_in(&repo, &["add", "."]);
        git_in(&repo, &["commit", "-qm", "pin"]);
        let head = git_in(&repo, &["rev-parse", "HEAD"]);
        (repo, head)
    }

    /// GitHub as `asked` answers: B ahead of A, and main at or ahead of B.
    fn github(
        newer: &'static str,
        merged: &'static str,
    ) -> impl Fn(&str, &str, &str) -> Option<String> {
        move |repo, base, head| {
            assert_eq!(
                repo, "Jitsusama/mycelium",
                "asked GitHub about another repo"
            );
            match (base, head) {
                (A, B) => Some(newer.to_owned()),
                (B, "main") => Some(merged.to_owned()),
                _ => panic!("asked GitHub about {base}...{head}"),
            }
        }
    }

    #[test]
    fn a_lock_alone_pinning_a_merged_newer_commit_needs_no_round() {
        let (repo, head) = pinned("alone", None);
        let said = alone_asking(&repo, &head, "main", github("ahead", "ahead"));
        assert!(
            said.is_some_and(|said| said.contains("Jitsusama/mycelium")),
            "a lock-only pin of a merged mycelium commit was asked for a round"
        );
        assert!(
            alone_asking(&repo, &head, "main", github("ahead", "identical")).is_some(),
            "a pin of mycelium's trunk itself was asked for a round"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_back_off_or_unmerged_or_unasked_needs_a_round() {
        let (repo, head) = pinned("unmerged", None);
        for (newer, merged, why) in [
            ("behind", "ahead", "a pin back to an older commit"),
            ("identical", "ahead", "a pin to the same commit"),
            (
                "ahead",
                "diverged",
                "a pin to a commit off mycelium's trunk",
            ),
            (
                "ahead",
                "behind",
                "a pin to a commit ahead of mycelium's trunk",
            ),
        ] {
            assert_eq!(
                alone_asking(&repo, &head, "main", github(newer, merged)),
                None,
                "{why} went without a round"
            );
        }
        assert_eq!(
            alone_asking(&repo, &head, "main", |_, _, _| None),
            None,
            "a pin GitHub couldn't be asked about went without a round"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_beside_another_file_needs_a_round() {
        let (repo, head) = pinned("beside", Some("module.nix"));
        assert_eq!(
            alone_asking(&repo, &head, "main", github("ahead", "ahead")),
            None,
            "a pin changing a module too went without a round"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
