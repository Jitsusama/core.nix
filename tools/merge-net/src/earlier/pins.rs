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
//! commit must descend from the old one and be the very commit a pull
//! request into the repo's trunk merged as, as GitHub says. Being on the
//! trunk isn't enough: the repo's train merges with merge commits, which
//! put every commit a pull request carried on the trunk, though a round
//! read only its head (on 22 November 1,150 of mycelium's 1,543). Every
//! commit the pin brings in must have come in through a pull request
//! merged into that trunk, since the trunk takes pushes too. Its `narHash`
//! and `lastModified` must be what Nix makes of that commit fetched
//! afresh, since Nix takes a source whose hash it already holds without
//! fetching it. A branch, a pull request's head, a commit from a fork, a
//! commit pushed straight to the trunk or a pin back to an older commit
//! needs a round. A node added, removed or renamed, an input that follows
//! another, or a source of anyone else's needs one too. Anything that
//! can't be read or asked needs a round, as before, and a lock-only head
//! is told why.
//!
//! What this leans on: that a pull request merged into the repo's trunk
//! was read by a round of ours, which the repo's own merge gate keeps,
//! not this one.

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

/// The most commits a pin may bring in, each asked after on its own: one
/// page of GitHub's compare.
const BROUGHT_AT_MOST: usize = 100;

/// A node of ours whose lock moved from one commit to another.
#[derive(Debug, PartialEq, Eq)]
struct Moved {
    repo: String,
    trunk: String,
    old: String,
    new: String,
    /// The node's lock as the head has it.
    locked: Value,
}

/// What a head that moves only flake.lock is told.
#[derive(Debug, PartialEq, Eq)]
pub enum Pinned {
    /// It needs no round, and why.
    Alone(String),
    /// It needs a round after all, and why, said so the person waiting on
    /// it can tell an outage worth asking again from a pin worth fixing.
    Declined(String),
}

/// What's asked about a pin's commits, of GitHub and of Nix, each giving
/// what was said as it was said, for the functions below to judge.
pub trait Asks {
    /// GitHub's compare of `old` to `new` in `repo`: its status, its count
    /// of commits and the commits it lists.
    fn compared(&self, repo: &str, old: &str, new: &str) -> Result<Value, String>;
    /// The pull requests GitHub ties to `rev` in `repo`.
    fn pulls(&self, repo: &str, rev: &str) -> Result<Value, String>;
    /// Nix's prefetch of `rev` of `repo`, fetched afresh.
    fn prefetched(&self, repo: &str, rev: &str) -> Result<Value, String>;
}

/// GitHub's API through gh, on github.com, the host a lock's `github`
/// type fetches from, and Nix's own prefetch.
pub struct Live;

impl Asks for Live {
    fn compared(&self, repo: &str, old: &str, new: &str) -> Result<Value, String> {
        // Only what's judged comes back: a compare's files and patches
        // can run to megabytes.
        let path = format!("repos/{repo}/compare/{old}...{new}?per_page={BROUGHT_AT_MOST}");
        let wanted = "{status, total_commits, commits: [.commits[].sha]}";
        json(&answer(gh(&[&path, "--jq", wanted]))?)
    }

    fn pulls(&self, repo: &str, rev: &str) -> Result<Value, String> {
        let path = format!("repos/{repo}/commits/{rev}/pulls");
        let wanted = "[.[] | {merge_commit_sha, merged_at, \
                      base: {ref: .base.ref, repo: .base.repo.full_name}}]";
        json(&answer(gh(&[&path, "--jq", wanted]))?)
    }

    fn prefetched(&self, repo: &str, rev: &str) -> Result<Value, String> {
        let mut nix = Command::new("timeout");
        nix.args([LIMIT, "nix", "flake", "prefetch", "--json"])
            .arg(format!("github:{repo}/{rev}"));
        json(&answer(nix)?)
    }
}

/// How long, in seconds, each ask is given.
const LIMIT: &str = "120";

/// gh asking github.com's API at `args`, given [`LIMIT`].
fn gh(args: &[&str]) -> Command {
    let mut gh = Command::new("timeout");
    gh.args([LIMIT, "gh", "api", "--hostname", "github.com"])
        .args(args);
    gh
}

/// `said` read as JSON, or why it can't be.
fn json(said: &str) -> Result<Value, String> {
    serde_json::from_str(said).map_err(|why| format!("it said what can't be read: {why}"))
}

/// What `command`, a program run under `timeout`, says, or what went
/// wrong, its own last words included.
fn answer(mut command: Command) -> Result<String, String> {
    let mut args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned());
    let limit = args.next().unwrap_or_default();
    let program = args.next().unwrap_or_default();
    let ran = command.output().map_err(|why| {
        format!(
            "{} couldn't be run for {program}: {why}",
            command.get_program().to_string_lossy()
        )
    })?;
    if ran.status.success() {
        return Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned());
    }
    let said = String::from_utf8_lossy(&ran.stderr);
    let said = said.lines().last().unwrap_or("nothing").trim();
    Err(match ran.status.code() {
        Some(124) => format!("{program} didn't answer within {limit} s"),
        _ => format!("{program} failed, saying {said}"),
    })
}

/// Whether GitHub's `compared` says its head descends from its base and
/// isn't it.
fn ahead(compared: &Value) -> bool {
    compared["status"].as_str() == Some("ahead")
}

/// The commits `compared` says its head brings in, when it lists them all.
fn brought(compared: &Value) -> Option<Vec<String>> {
    let listed: Vec<String> = compared["commits"]
        .as_array()?
        .iter()
        .map(|sha| sha.as_str().map(str::to_owned))
        .collect::<Option<_>>()?;
    let total = usize::try_from(compared["total_commits"].as_u64()?).ok()?;
    (total == listed.len()).then_some(listed)
}

/// Whether one of `pulls` is a pull request merged into `trunk` of `repo`.
fn merged_into(pull: &Value, repo: &str, trunk: &str) -> bool {
    !pull["merged_at"].is_null()
        && pull["base"]["ref"].as_str() == Some(trunk)
        && pull["base"]["repo"].as_str() == Some(repo)
}

/// Whether `pulls` holds a pull request merged into `trunk` of `repo`,
/// which brought the commit they're tied to in.
fn came_in_merged(pulls: &Value, repo: &str, trunk: &str) -> bool {
    pulls
        .as_array()
        .is_some_and(|pulls| pulls.iter().any(|pull| merged_into(pull, repo, trunk)))
}

/// Whether `pulls` holds a pull request merged into `trunk` of `repo` as
/// `rev`: a merge commit, a squash or the head of one merged fast forward.
fn merged_as(pulls: &Value, repo: &str, rev: &str, trunk: &str) -> bool {
    pulls.as_array().is_some_and(|pulls| {
        pulls.iter().any(|pull| {
            merged_into(pull, repo, trunk) && pull["merge_commit_sha"].as_str() == Some(rev)
        })
    })
}

/// What `commit`, in the repo at `repo`, is told when its change from its
/// base on `main` only moves flake.lock; nothing for any other head.
pub fn ours_alone(repo: &Path, commit: &str, main: &str, asks: &impl Asks) -> Option<Pinned> {
    let base = git_at(repo, &["merge-base", main, commit]).ok()?;
    let files = git_at(repo, &["diff", "--name-only", &base, commit]).ok()?;
    if files != LOCK {
        return None;
    }
    let lock = |at: &str| {
        git_at(repo, &["show", &format!("{at}:{LOCK}")])
            .map_err(|why| format!("its {LOCK} can't be read at {}: {why}", short(at)))
    };
    let moved = match lock(&base).and_then(|before| moved_ours(&before, &lock(commit)?)) {
        Ok(moved) => moved,
        Err(why) => return Some(Pinned::Declined(why)),
    };
    for pin in &moved {
        if let Err(why) = vouched(pin, asks) {
            return Some(Pinned::Declined(why));
        }
    }
    let repos: Vec<&str> = moved.iter().map(|pin| pin.repo.as_str()).collect();
    Some(Pinned::Alone(format!(
        "{} moves only {LOCK}, pinning {} to newer commits that pull requests into its trunk \
         merged as, whose own changes merge through rounds of ours, so it needs no round of \
         its own",
        short(commit),
        repos.join(" and ")
    )))
}

/// Whether `pin`'s new commit is newer, brings in only what pull requests
/// merged, is one's merge itself and is locked as Nix makes it, asking
/// each only once the one before holds.
fn vouched(pin: &Moved, asks: &impl Asks) -> Result<(), String> {
    let (repo, trunk) = (pin.repo.as_str(), pin.trunk.as_str());
    let (old, new) = (short(&pin.old), short(&pin.new));
    let compared = asks
        .compared(repo, &pin.old, &pin.new)
        .map_err(|why| format!("asking GitHub whether {repo} at {new} is newer: {why}"))?;
    if !ahead(&compared) {
        return Err(format!("{repo} at {new} isn't newer than at {old}"));
    }
    let Some(commits) = brought(&compared) else {
        return Err(format!(
            "GitHub lists {} of what {repo} brings in from {old} to {new} by name, more than \
             the {BROUGHT_AT_MOST} each asked after alone",
            compared["total_commits"]
        ));
    };
    for brought in commits.iter().rev() {
        let pulls = asks.pulls(repo, brought).map_err(|why| {
            format!(
                "asking GitHub which pull request brought {} into {repo}: {why}",
                short(brought)
            )
        })?;
        if *brought == pin.new && !merged_as(&pulls, repo, brought, trunk) {
            return Err(format!(
                "no pull request into {repo}'s {trunk} merged as {new}, so no round need have \
                 read it"
            ));
        }
        if !came_in_merged(&pulls, repo, trunk) {
            return Err(format!(
                "{} came onto {repo}'s {trunk} through no pull request merged there",
                short(brought)
            ));
        }
    }
    let prefetched = asks
        .prefetched(repo, &pin.new)
        .map_err(|why| format!("asking Nix for {repo} at {new}: {why}"))?;
    let fetched = &prefetched["locked"];
    for field in ["narHash", "lastModified"] {
        if fetched.get(field).is_none() {
            return Err(format!(
                "Nix's prefetch of {repo} at {new} gave no {field} to compare with"
            ));
        }
        if pin.locked.get(field) != fetched.get(field) {
            return Err(format!(
                "its {field} for {repo} at {new} isn't what Nix makes of that commit"
            ));
        }
    }
    Ok(())
}

/// The nodes of ours whose commits moved from `before` to `after`, two
/// flake.lock files, when nothing else in them did; why not otherwise.
fn moved_ours(before: &str, after: &str) -> Result<Vec<Moved>, String> {
    let parsed = |lock: &str| {
        serde_json::from_str::<Value>(lock).map_err(|why| format!("its {LOCK} isn't JSON: {why}"))
    };
    let (before, after) = (parsed(before)?, parsed(after)?);
    let nodes = |lock: &Value| {
        lock["nodes"]
            .as_object()
            .cloned()
            .ok_or_else(|| format!("its {LOCK} holds no nodes"))
    };
    let (was, now) = (nodes(&before)?, nodes(&after)?);
    if but(&before, &["nodes"]) != but(&after, &["nodes"]) {
        return Err(format!("it changes the {LOCK}'s version or root"));
    }
    if !was.keys().eq(now.keys()) {
        return Err("it adds, removes or renames a node".to_owned());
    }
    let mut moved = Vec::new();
    for (name, node) in &now {
        let old = &was[name];
        if old == node {
            continue;
        }
        let Some((repo, trunk)) = ours(old) else {
            return Err(format!("it moves {name}, which isn't a repo of ours"));
        };
        let (locked, unlocked) = (&old["locked"], &node["locked"]);
        if but(old, &["locked"]) != but(node, &["locked"]) {
            return Err(format!(
                "it changes {name}'s inputs or what flake.nix asks of it"
            ));
        }
        if but(locked, &MOVES) != but(unlocked, &MOVES) {
            return Err(format!("it changes {name}'s lock beyond its commit"));
        }
        let commit = |locked: &Value| {
            commit_of(locked).ok_or_else(|| format!("{name}'s lock names no full commit"))
        };
        let (from, to) = (commit(locked)?, commit(unlocked)?);
        if from == to {
            return Err(format!("it changes {name}'s hash but not its commit"));
        }
        moved.push(Moved {
            repo: repo.to_owned(),
            trunk: trunk.to_owned(),
            old: from,
            new: to,
            locked: unlocked.clone(),
        });
    }
    if moved.is_empty() {
        return Err("it moves no node".to_owned());
    }
    Ok(moved)
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
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::{env, fs};

    use serde_json::json;

    use super::*;
    use crate::tests::git_in;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    /// A commit a pull request carried that B, its merge, brings in too.
    const C: &str = "cccccccccccccccccccccccccccccccccccccccc";
    const MYCELIUM: &str = "Jitsusama/mycelium";

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

    fn moved(before: &Value, after: &Value) -> Result<Vec<Moved>, String> {
        moved_ours(&before.to_string(), &after.to_string())
    }

    /// Asserts a pin from mycelium at A to `after` needs a round, saying
    /// `says` of why.
    fn needs_a_round(after: &Value, says: &str, what: &str) {
        match moved(&lock(A, "n"), after) {
            Err(why) => assert!(why.contains(says), "{what} was turned down saying {why}"),
            Ok(moved) => panic!("{what} went without a round: {moved:?}"),
        }
    }

    #[test]
    fn a_newer_commit_of_a_repo_of_ours_is_a_pin_of_ours() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]["lastModified"] = json!(2);
        assert_eq!(
            moved(&lock(A, "n"), &after),
            Ok(vec![Moved {
                repo: MYCELIUM.to_owned(),
                trunk: "main".to_owned(),
                old: A.to_owned(),
                new: B.to_owned(),
                locked: after["nodes"]["mycelium"]["locked"].clone(),
            }]),
            "a pin of mycelium alone wasn't read as one"
        );
    }

    #[test]
    fn a_lock_moving_anyone_elses_source_needs_a_round() {
        needs_a_round(
            &lock(B, "m"),
            "moves nixpkgs, which isn't",
            "a pin moving nixpkgs too",
        );
    }

    #[test]
    fn a_lock_that_moves_nothing_is_no_pin() {
        needs_a_round(&lock(A, "n"), "moves no node", "a lock that moved nothing");
    }

    #[test]
    fn a_node_of_ours_pointed_at_another_repo_or_owner_needs_a_round() {
        for (field, value) in [("repo", "agentic-harness.pi"), ("owner", "Elsewhere")] {
            let mut after = lock(B, "n");
            after["nodes"]["mycelium"]["locked"][field] = json!(value);
            needs_a_round(
                &after,
                "lock beyond its commit",
                &format!("a node moved to another {field}"),
            );
        }
    }

    #[test]
    fn a_lock_sent_to_another_host_folder_or_ref_needs_a_round() {
        for (field, value) in [("host", "git.example.org"), ("dir", "sub"), ("ref", "x")] {
            let mut after = lock(B, "n");
            after["nodes"]["mycelium"]["locked"][field] = json!(value);
            needs_a_round(
                &after,
                "lock beyond its commit",
                &format!("a pin adding {field} to its lock"),
            );
        }
    }

    #[test]
    fn a_pin_to_a_short_or_unhexed_rev_needs_a_round() {
        needs_a_round(
            &lock("main", "n"),
            "names no full commit",
            "a pin naming a branch for its commit",
        );
        needs_a_round(
            &lock(&"z".repeat(40), "n"),
            "names no full commit",
            "a pin naming 40 letters that aren't hex",
        );
    }

    #[test]
    fn a_pin_moving_only_its_hash_needs_a_round() {
        let mut after = lock(A, "n");
        after["nodes"]["mycelium"]["locked"]["narHash"] = json!("sha256-another");
        needs_a_round(
            &after,
            "hash but not its commit",
            "a hash moved under the same commit",
        );
    }

    #[test]
    fn a_pin_that_changes_what_an_input_follows_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["inputs"] = json!({"nixpkgs": "nixpkgs_2"});
        needs_a_round(&after, "inputs", "a pin changing an input's follows");
    }

    #[test]
    fn a_pin_that_changes_its_original_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["original"]["ref"] = json!("other");
        needs_a_round(
            &after,
            "what flake.nix asks",
            "a pin changing what flake.nix asked for",
        );
    }

    #[test]
    fn a_pin_that_adds_or_removes_a_node_needs_a_round() {
        let mut before = lock(A, "n");
        before["nodes"]["left"] = before["nodes"]["nixpkgs"].clone();
        let mut after = lock(B, "n");
        assert!(
            moved(&before, &after).is_err_and(|why| why.contains("adds, removes or renames")),
            "a pin removing a node went without a round, or without saying so"
        );
        after["nodes"]["nixpkgs_2"] = after["nodes"]["nixpkgs"].clone();
        needs_a_round(&after, "adds, removes or renames", "a pin adding a node");
    }

    #[test]
    fn a_pin_to_another_kind_of_source_needs_a_round() {
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]["type"] = json!("git");
        needs_a_round(
            &after,
            "lock beyond its commit",
            "a pin moving mycelium off GitHub",
        );
    }

    #[test]
    fn a_lock_of_another_version_or_root_needs_a_round() {
        let mut after = lock(B, "n");
        after["version"] = json!(8);
        needs_a_round(&after, "version or root", "a lock of another version");
        let mut after = lock(B, "n");
        after["root"] = json!("nixpkgs");
        needs_a_round(&after, "version or root", "a lock of another root");
    }

    /// A pull request as GitHub's commits/{rev}/pulls gives it, shaped as
    /// `Live` asks for it.
    fn pull(sha: &str, merged: bool, base: &str, repo: &str) -> Value {
        json!({
            "merge_commit_sha": sha,
            "merged_at": if merged { json!("2026-11-22T15:18:15Z") } else { Value::Null },
            "base": {"ref": base, "repo": repo},
        })
    }

    #[test]
    fn only_a_pull_request_merged_into_our_trunk_as_the_commit_vouches_for_it() {
        let merged = |pulls: Value| merged_as(&pulls, MYCELIUM, B, "main");
        assert!(
            merged(json!([pull(B, true, "main", MYCELIUM)])),
            "a pull request merged into main as the commit didn't vouch for it"
        );
        for (pulls, what) in [
            (json!([]), "no pull request"),
            (
                json!([pull(B, true, "release", MYCELIUM)]),
                "one into another base",
            ),
            (
                json!([pull(B, false, "main", MYCELIUM)]),
                "an open one's test merge",
            ),
            (
                json!([pull(C, true, "main", MYCELIUM)]),
                "one whose merge is another commit",
            ),
            (
                json!([pull(B, true, "main", "Elsewhere/mycelium")]),
                "one into a fork's main",
            ),
            (
                json!({"message": "Not Found"}),
                "an answer that isn't a list",
            ),
        ] {
            assert!(!merged(pulls), "{what} vouched for a commit");
        }
    }

    #[test]
    fn a_commit_came_in_merged_only_through_a_pull_request_merged_into_our_trunk() {
        let came = |pulls: Value| came_in_merged(&pulls, MYCELIUM, "main");
        assert!(
            came(json!([pull(B, true, "main", MYCELIUM)])),
            "a commit a merged pull request carried wasn't counted as merged"
        );
        for (pulls, what) in [
            (json!([]), "a commit pushed straight to the trunk"),
            (
                json!([pull(B, false, "main", MYCELIUM)]),
                "one an open pull request carries",
            ),
            (
                json!([pull(B, true, "main", "Elsewhere/mycelium")]),
                "one merged in a fork",
            ),
        ] {
            assert!(!came(pulls), "{what} was counted as merged");
        }
    }

    #[test]
    fn a_compare_names_what_it_brings_only_when_it_lists_it_all() {
        let compared =
            |total: u64| json!({"status": "ahead", "total_commits": total, "commits": [C, B]});
        assert_eq!(
            brought(&compared(2)),
            Some(vec![C.to_owned(), B.to_owned()]),
            "a compare listing all it brings wasn't read"
        );
        assert_eq!(
            brought(&compared(101)),
            None,
            "a compare listing part of what it brings was taken for all"
        );
        assert!(ahead(&compared(2)), "an ahead compare wasn't read as newer");
        for status in ["behind", "identical", "diverged"] {
            assert!(
                !ahead(&json!({"status": status})),
                "a compare {status} was read as newer"
            );
        }
    }

    #[test]
    fn an_ask_that_fails_or_runs_out_of_time_says_which_and_why() {
        let mut failing = Command::new("timeout");
        failing.args(["5", "sh", "-c", "echo first >&2; echo last >&2; exit 3"]);
        assert_eq!(
            answer(failing),
            Err("sh failed, saying last".to_owned()),
            "a failed ask didn't give its last words"
        );
        let mut slow = Command::new("timeout");
        slow.args(["1", "sleep", "5"]);
        assert_eq!(
            answer(slow),
            Err("sleep didn't answer within 1 s".to_owned()),
            "an ask past its limit didn't say so"
        );
        let mut unrun = Command::new("merge-net-no-such-timeout");
        unrun.args(["5", "gh"]);
        assert!(
            answer(unrun).is_err_and(|why| why.starts_with("merge-net-no-such-timeout couldn't")),
            "a missing timeout was blamed on what it would have run"
        );
    }

    /// A repo whose main holds a lock with mycelium at A, and a branch from
    /// it that pins mycelium at B as `after` locks it, touching `beside`
    /// too when it's given.
    pub(crate) fn pinned(
        name: &str,
        after: &Value,
        beside: Option<&str>,
    ) -> (std::path::PathBuf, String) {
        let repo = env::temp_dir().join(format!("merge-net-pins-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join(LOCK), lock(A, "n").to_string()).unwrap();
        fs::write(repo.join("module.nix"), "{ }").unwrap();
        git_in(&repo, &["add", "."]);
        git_in(&repo, &["commit", "-qm", "base"]);
        git_in(&repo, &["checkout", "-qb", "pin"]);
        fs::write(repo.join(LOCK), after.to_string()).unwrap();
        if let Some(file) = beside {
            fs::write(repo.join(file), "{ x = 1; }").unwrap();
        }
        git_in(&repo, &["add", "."]);
        git_in(&repo, &["commit", "-qm", "pin"]);
        let head = git_in(&repo, &["rev-parse", "HEAD"]);
        (repo, head)
    }

    /// [`pinned`] with mycelium at B as the lock gives it.
    pub(crate) fn pinned_to_b(name: &str, beside: Option<&str>) -> (std::path::PathBuf, String) {
        pinned(name, &lock(B, "n"), beside)
    }

    /// GitHub and Nix as the test has them answer about mycelium from A to
    /// B, which brings in C and B, noting each ask, so a test sees which
    /// were made.
    pub(crate) struct Fake {
        pub compared: Result<Value, String>,
        pub pulls: HashMap<&'static str, Result<Value, String>>,
        pub prefetched: Result<Value, String>,
        pub asked: RefCell<Vec<String>>,
    }

    impl Fake {
        /// A fake saying B is newer, brings in C, which a pull request B
        /// merged carried, and is locked as the test's lock has it.
        pub(crate) fn vouching() -> Self {
            let merge = json!([pull(B, true, "main", MYCELIUM)]);
            Self {
                compared: Ok(json!({"status": "ahead", "total_commits": 2, "commits": [C, B]})),
                pulls: HashMap::from([(B, Ok(merge.clone())), (C, Ok(merge))]),
                prefetched: Ok(json!({"locked": lock(B, "n")["nodes"]["mycelium"]["locked"]})),
                asked: RefCell::new(Vec::new()),
            }
        }

        /// What it was asked, in order.
        fn asked(&self) -> Vec<String> {
            self.asked.borrow().clone()
        }
    }

    impl Asks for Fake {
        fn compared(&self, repo: &str, old: &str, new: &str) -> Result<Value, String> {
            assert_eq!(
                (repo, old, new),
                (MYCELIUM, A, B),
                "asked for the wrong compare"
            );
            self.asked.borrow_mut().push("compared".to_owned());
            self.compared.clone()
        }

        fn pulls(&self, repo: &str, rev: &str) -> Result<Value, String> {
            assert_eq!(repo, MYCELIUM, "asked another repo's pulls");
            self.asked.borrow_mut().push(format!("pulls {}", &rev[..1]));
            self.pulls
                .get(rev)
                .cloned()
                .unwrap_or_else(|| Ok(json!([])))
        }

        fn prefetched(&self, repo: &str, rev: &str) -> Result<Value, String> {
            assert_eq!((repo, rev), (MYCELIUM, B), "asked Nix wrongly");
            self.asked.borrow_mut().push("prefetched".to_owned());
            self.prefetched.clone()
        }
    }

    /// Asserts `asks` turns down the pin in `repo` at `head`, saying `says`,
    /// having asked `asked`.
    fn declined(repo: &Path, head: &str, asks: &Fake, says: &str, asked: &[&str]) {
        match ours_alone(repo, head, "main", asks) {
            Some(Pinned::Declined(why)) => {
                assert!(
                    why.contains(says),
                    "a pin turned down said {why}, not {says}"
                );
            }
            other => panic!("a pin that should need a round was told {other:?}"),
        }
        assert_eq!(asks.asked(), asked, "a pin turned down asked otherwise");
    }

    const EVERY_ASK: [&str; 4] = ["compared", "pulls b", "pulls c", "prefetched"];

    #[test]
    fn a_lock_alone_pinning_a_merged_newer_commit_needs_no_round() {
        let (repo, head) = pinned_to_b("alone", None);
        let asks = Fake::vouching();
        let told = ours_alone(&repo, &head, "main", &asks);
        assert!(
            matches!(&told, Some(Pinned::Alone(said)) if said.contains(MYCELIUM)),
            "a lock-only pin of a merged mycelium commit was told {told:?}"
        );
        assert_eq!(asks.asked(), EVERY_ASK, "it didn't ask each in turn");
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_still_passes_once_main_moves_on_beside_it() {
        let (repo, head) = pinned_to_b("moved-on", None);
        git_in(&repo, &["checkout", "-q", "main"]);
        fs::write(repo.join("module.nix"), "{ y = 2; }").unwrap();
        git_in(&repo, &["commit", "-qam", "main moves on"]);
        let told = ours_alone(&repo, &head, "main", &Fake::vouching());
        assert!(
            matches!(told, Some(Pinned::Alone(_))),
            "a pin behind a main that moved a module was told {told:?}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_back_unmerged_or_bringing_a_push_says_so_and_asks_no_further() {
        let (repo, head) = pinned_to_b("unmerged", None);
        let back = Fake {
            compared: Ok(json!({"status": "behind", "total_commits": 0, "commits": []})),
            ..Fake::vouching()
        };
        declined(&repo, &head, &back, "isn't newer", &["compared"]);
        let mut unmerged = Fake::vouching();
        unmerged
            .pulls
            .insert(B, Ok(json!([pull(C, true, "main", MYCELIUM)])));
        declined(
            &repo,
            &head,
            &unmerged,
            "no pull request into Jitsusama/mycelium's main merged as",
            &["compared", "pulls b"],
        );
        let mut pushed = Fake::vouching();
        pushed.pulls.insert(C, Ok(json!([])));
        declined(
            &repo,
            &head,
            &pushed,
            "cccccccccccc came onto",
            &["compared", "pulls b", "pulls c"],
        );
        let past = Fake {
            compared: Ok(json!({"status": "ahead", "total_commits": 101, "commits": [C, B]})),
            ..Fake::vouching()
        };
        declined(&repo, &head, &past, "lists 101", &["compared"]);
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_github_or_nix_cant_answer_says_which_and_asks_no_further() {
        let (repo, head) = pinned_to_b("unasked", None);
        let failing = |at: &str| format!("gh failed at {at}");
        let compare = Fake {
            compared: Err(failing("compare")),
            ..Fake::vouching()
        };
        let mut pulls = Fake::vouching();
        pulls.pulls.insert(C, Err(failing("pulls")));
        let prefetch = Fake {
            prefetched: Err(failing("prefetch")),
            ..Fake::vouching()
        };
        for (asks, says, at, asked) in [
            (
                compare,
                "whether Jitsusama/mycelium at bbbbbbbbbbbb is newer",
                "compare",
                &EVERY_ASK[..1],
            ),
            (
                pulls,
                "which pull request brought cccccccccccc",
                "pulls",
                &EVERY_ASK[..3],
            ),
            (
                prefetch,
                "asking Nix for Jitsusama/mycelium",
                "prefetch",
                &EVERY_ASK[..],
            ),
        ] {
            declined(&repo, &head, &asks, says, asked);
            asks.asked.borrow_mut().clear();
            declined(&repo, &head, &asks, &failing(at), asked);
        }
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_whose_hash_or_time_isnt_its_commits_needs_a_round() {
        let (repo, head) = pinned_to_b("hash", None);
        let mine = lock(B, "n")["nodes"]["mycelium"]["locked"].clone();
        for (field, value) in [
            ("narHash", json!("sha256-other")),
            ("lastModified", json!(2)),
        ] {
            let mut fetched = mine.clone();
            fetched[field] = value;
            let other = Fake {
                prefetched: Ok(json!({"locked": fetched})),
                ..Fake::vouching()
            };
            declined(&repo, &head, &other, &format!("its {field}"), &EVERY_ASK);
        }
        let bare = Fake {
            prefetched: Ok(json!({"hash": "sha256-x"})),
            ..Fake::vouching()
        };
        declined(&repo, &head, &bare, "gave no narHash", &EVERY_ASK);
        fs::remove_dir_all(&repo).unwrap();
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]
            .as_object_mut()
            .unwrap()
            .remove("narHash");
        let (repo, head) = pinned("hashless", &after, None);
        let mut fetched = mine;
        fetched.as_object_mut().unwrap().remove("narHash");
        let hashless = Fake {
            prefetched: Ok(json!({"locked": fetched})),
            ..Fake::vouching()
        };
        declined(&repo, &head, &hashless, "gave no narHash", &EVERY_ASK);
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_lock_only_head_turned_down_by_its_lock_says_why_and_asks_nothing() {
        let (repo, head) = pinned("nixpkgs", &lock(B, "m"), None);
        declined(
            &repo,
            &head,
            &Fake::vouching(),
            "moves nixpkgs, which isn't a repo of ours",
            &[],
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_beside_another_file_isnt_asked_after() {
        let (repo, head) = pinned_to_b("beside", Some("module.nix"));
        let asks = Fake::vouching();
        assert_eq!(
            ours_alone(&repo, &head, "main", &asks),
            None,
            "a pin changing a module too was taken for a pin"
        );
        assert!(
            asks.asked().is_empty(),
            "a head beside a module asked GitHub"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
