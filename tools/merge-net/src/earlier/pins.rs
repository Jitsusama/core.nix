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
//! request merged as into the repo's trunk, as GitHub says. Being on the
//! trunk isn't enough: the repo's train merges with merge commits, which
//! put every commit a pull request carried on the trunk, though a round
//! read only its head (on 22 November 1,150 of mycelium's 1,543). Its
//! `narHash` and `lastModified` must be what Nix makes of that commit
//! fetched afresh, since Nix takes a source whose hash it already holds
//! without fetching it. A branch, a pull request's head, a commit from a
//! fork or a pin back to an older commit needs a round. A node added,
//! removed or renamed, an input that follows another, or a source of
//! anyone else's needs one too. Anything that can't be read or asked
//! needs a round, as before, and a lock-only head of ours says why.

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
    /// The node's lock as the head has it.
    locked: Value,
}

/// What a head that moves only flake.lock, pinning repos of ours, is told.
#[derive(Debug, PartialEq, Eq)]
pub enum Pinned {
    /// It needs no round, and why.
    Alone(String),
    /// It needs a round after all, and why, said so the person waiting on
    /// it can tell an outage worth asking again from a pin worth fixing.
    Declined(String),
}

/// What's asked about a pin's new commit, of GitHub and of Nix.
pub trait Asks {
    /// Whether `new` descends from `old` in `repo` and isn't it.
    fn newer(&self, repo: &str, old: &str, new: &str) -> Result<bool, String>;
    /// Whether a pull request merged into `trunk` of `repo` as `rev`.
    fn merged(&self, repo: &str, rev: &str, trunk: &str) -> Result<bool, String>;
    /// The lock Nix makes of `rev` of `repo`, fetched afresh.
    fn locked(&self, repo: &str, rev: &str) -> Result<Value, String>;
}

/// GitHub's API through gh, on github.com, the host a lock's `github`
/// type fetches from, and Nix's own prefetch.
pub struct Live;

impl Asks for Live {
    fn newer(&self, repo: &str, old: &str, new: &str) -> Result<bool, String> {
        // `ahead` says head descends from base and isn't it.
        let path = format!("repos/{repo}/compare/{old}...{new}?per_page=1");
        Ok(answer(gh(&[&path, "--jq", ".status"]))? == "ahead")
    }

    fn merged(&self, repo: &str, rev: &str, trunk: &str) -> Result<bool, String> {
        let path = format!("repos/{repo}/commits/{rev}/pulls");
        let merged = format!(
            "[.[] | select(.merge_commit_sha == \"{rev}\" and .merged_at != null \
             and .base.ref == \"{trunk}\")] | length"
        );
        Ok(answer(gh(&[&path, "--jq", &merged]))? != "0")
    }

    fn locked(&self, repo: &str, rev: &str) -> Result<Value, String> {
        let mut nix = Command::new("timeout");
        nix.args(["120", "nix", "flake", "prefetch", "--json"])
            .arg(format!("github:{repo}/{rev}"));
        let said: Value = serde_json::from_str(&answer(nix)?)
            .map_err(|why| format!("Nix's prefetch of {repo} said what can't be read: {why}"))?;
        Ok(said["locked"].clone())
    }
}

/// gh asking github.com's API at `args`, given two minutes.
fn gh(args: &[&str]) -> Command {
    let mut gh = Command::new("timeout");
    gh.args(["120", "gh", "api", "--hostname", "github.com"])
        .args(args);
    gh
}

/// What `command` says, or what went wrong, its own words included.
fn answer(mut command: Command) -> Result<String, String> {
    let program = command
        .get_args()
        .nth(1)
        .map_or_else(String::new, |program| {
            program.to_string_lossy().into_owned()
        });
    let ran = command
        .output()
        .map_err(|why| format!("{program} couldn't be run: {why}"))?;
    if ran.status.success() {
        return Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned());
    }
    let said = String::from_utf8_lossy(&ran.stderr);
    let said = said.lines().last().unwrap_or("nothing").trim();
    Err(match ran.status.code() {
        Some(124) => format!("{program} didn't answer within two minutes"),
        _ => format!("{program} failed, saying {said}"),
    })
}

/// What `commit`, in the repo at `repo`, is told when its change from its
/// base on `main` only moves flake.lock, pinning repos of [`OURS`]; nothing
/// for any other head.
pub fn ours_alone(repo: &Path, commit: &str, main: &str, asks: &impl Asks) -> Option<Pinned> {
    let base = git_at(repo, &["merge-base", main, commit]).ok()?;
    let files = git_at(repo, &["diff", "--name-only", &base, commit]).ok()?;
    if files != LOCK {
        return None;
    }
    let lock = |at: &str| git_at(repo, &["show", &format!("{at}:{LOCK}")]).ok();
    let moved = moved_ours(&lock(&base)?, &lock(commit)?)?;
    for pin in &moved {
        if let Err(why) = vouched(pin, asks) {
            return Some(Pinned::Declined(why));
        }
    }
    let repos: Vec<&str> = moved.iter().map(|pin| pin.repo.as_str()).collect();
    Some(Pinned::Alone(format!(
        "{} moves only {LOCK}, to newer commits that pull requests merged as into {}, whose \
         own changes merge through rounds of ours, so it needs no round of its own",
        short(commit),
        repos.join(" and ")
    )))
}

/// Whether `pin`'s new commit is newer, merged as a pull request and
/// locked as Nix makes it, asking each only once the one before holds.
fn vouched(pin: &Moved, asks: &impl Asks) -> Result<(), String> {
    let (repo, new) = (pin.repo.as_str(), short(&pin.new));
    if !asks.newer(repo, &pin.old, &pin.new)? {
        return Err(format!(
            "{repo} at {new} isn't newer than at {}",
            short(&pin.old)
        ));
    }
    if !asks.merged(repo, &pin.new, &pin.trunk)? {
        return Err(format!(
            "no pull request merged into {repo}'s {} as {new}, so no round need have read it",
            pin.trunk
        ));
    }
    let fetched = asks.locked(repo, &pin.new)?;
    for field in ["narHash", "lastModified"] {
        if pin.locked.get(field).is_none() || pin.locked[field] != fetched[field] {
            return Err(format!(
                "its {field} for {repo} at {new} isn't what Nix makes of that commit"
            ));
        }
    }
    Ok(())
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
            locked: unlocked.clone(),
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
pub(crate) mod tests {
    use std::cell::RefCell;
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
                locked: after["nodes"]["mycelium"]["locked"].clone(),
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
    fn a_node_of_ours_pointed_at_another_repo_or_owner_needs_a_round() {
        for (field, value) in [("repo", "agentic-harness.pi"), ("owner", "Elsewhere")] {
            let mut after = lock(B, "n");
            after["nodes"]["mycelium"]["locked"][field] = json!(value);
            needs_a_round(&after, &format!("a node moved to another {field}"));
        }
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
    fn a_pin_to_a_short_or_unhexed_rev_needs_a_round() {
        needs_a_round(&lock("main", "n"), "a pin naming a branch for its commit");
        let forty = "z".repeat(40);
        needs_a_round(
            &lock(&forty, "n"),
            "a pin naming 40 letters that aren't hex",
        );
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

    /// GitHub and Nix as the test has them answer about mycelium's A and B,
    /// noting each ask, so a test sees which were made.
    pub(crate) struct Fake {
        pub newer: Result<bool, String>,
        pub merged: Result<bool, String>,
        pub locked: Result<Value, String>,
        pub asked: RefCell<Vec<&'static str>>,
    }

    impl Fake {
        /// A fake saying B is newer, merged and locked as the test's lock has it.
        pub(crate) fn vouching() -> Self {
            Self {
                newer: Ok(true),
                merged: Ok(true),
                locked: Ok(lock(B, "n")["nodes"]["mycelium"]["locked"].clone()),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl Asks for Fake {
        fn newer(&self, repo: &str, old: &str, new: &str) -> Result<bool, String> {
            assert_eq!(
                (repo, old, new),
                ("Jitsusama/mycelium", A, B),
                "asked newer wrongly"
            );
            self.asked.borrow_mut().push("newer");
            self.newer.clone()
        }

        fn merged(&self, repo: &str, rev: &str, trunk: &str) -> Result<bool, String> {
            assert_eq!(
                (repo, rev, trunk),
                ("Jitsusama/mycelium", B, "main"),
                "asked merged wrongly"
            );
            self.asked.borrow_mut().push("merged");
            self.merged.clone()
        }

        fn locked(&self, repo: &str, rev: &str) -> Result<Value, String> {
            assert_eq!((repo, rev), ("Jitsusama/mycelium", B), "asked Nix wrongly");
            self.asked.borrow_mut().push("locked");
            self.locked.clone()
        }
    }

    /// Asserts `asks` turns down the pin in `repo` at `head`, saying `says`.
    fn declined(repo: &Path, head: &str, asks: &Fake, says: &str) {
        match ours_alone(repo, head, "main", asks) {
            Some(Pinned::Declined(why)) => {
                assert!(
                    why.contains(says),
                    "a pin turned down said {why}, not {says}"
                );
            }
            other => panic!("a pin that should need a round was told {other:?}"),
        }
    }

    #[test]
    fn a_lock_alone_pinning_a_merged_newer_commit_needs_no_round() {
        let (repo, head) = pinned_to_b("alone", None);
        let asks = Fake::vouching();
        let told = ours_alone(&repo, &head, "main", &asks);
        assert!(
            matches!(&told, Some(Pinned::Alone(said)) if said.contains("Jitsusama/mycelium")),
            "a lock-only pin of a merged mycelium commit was told {told:?}"
        );
        assert_eq!(
            *asks.asked.borrow(),
            ["newer", "merged", "locked"],
            "it didn't ask all three"
        );
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
    fn a_pin_back_or_unmerged_says_so_and_asks_no_further() {
        let (repo, head) = pinned_to_b("unmerged", None);
        let back = Fake {
            newer: Ok(false),
            ..Fake::vouching()
        };
        declined(&repo, &head, &back, "isn't newer");
        assert_eq!(
            *back.asked.borrow(),
            ["newer"],
            "an older pin was asked after further"
        );
        let unmerged = Fake {
            merged: Ok(false),
            ..Fake::vouching()
        };
        declined(&repo, &head, &unmerged, "no pull request merged");
        assert_eq!(
            *unmerged.asked.borrow(),
            ["newer", "merged"],
            "an unmerged pin went to Nix"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_github_or_nix_cant_answer_says_why() {
        let (repo, head) = pinned_to_b("unasked", None);
        let failing = |at: &str| format!("gh failed at {at}");
        for asks in [
            Fake {
                newer: Err(failing("newer")),
                ..Fake::vouching()
            },
            Fake {
                merged: Err(failing("merged")),
                ..Fake::vouching()
            },
            Fake {
                locked: Err(failing("locked")),
                ..Fake::vouching()
            },
        ] {
            declined(&repo, &head, &asks, "gh failed at");
        }
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_pin_whose_hash_isnt_its_commits_needs_a_round() {
        let (repo, head) = pinned_to_b("hash", None);
        let other = Fake {
            locked: Ok(json!({"narHash": "sha256-other", "lastModified": 1})),
            ..Fake::vouching()
        };
        declined(&repo, &head, &other, "its narHash");
        fs::remove_dir_all(&repo).unwrap();
        let mut after = lock(B, "n");
        after["nodes"]["mycelium"]["locked"]
            .as_object_mut()
            .unwrap()
            .remove("narHash");
        let (repo, head) = pinned("hashless", &after, None);
        declined(&repo, &head, &Fake::vouching(), "its narHash");
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
            asks.asked.borrow().is_empty(),
            "a head beside a module asked GitHub"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
