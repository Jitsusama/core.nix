use std::cell::RefCell;
use std::collections::HashMap;
use std::{env, fs};

use serde_json::json;

use super::*;
use crate::tests::git_in;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
/// The head of every pull request the tests merge, which B, its merge,
/// brings in too.
const C: &str = "cccccccccccccccccccccccccccccccccccccccc";
/// Another commit, a merge or a push as each test has it.
const D: &str = "dddddddddddddddddddddddddddddddddddddddd";
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
fn a_pin_to_a_short_unhexed_or_capitalised_rev_needs_a_round() {
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
    needs_a_round(
        &lock(&B.to_uppercase(), "n"),
        "names no full commit",
        "a pin naming its commit in capitals, which GitHub never lists",
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
/// `Live` asks for it, its head C.
fn pull(sha: &str, merged: bool, base: &str, repo: &str) -> Value {
    json!({
        "merge_commit_sha": sha,
        "merged_at": if merged { json!("2026-11-22T15:18:15Z") } else { Value::Null },
        "head": C,
        "base": {"ref": base, "repo": repo},
    })
}

/// A pull request GitHub marks merged as `sha` because its head, `sha`
/// itself, was pushed to the trunk.
fn pushed(sha: &str) -> Value {
    let mut pushed = pull(sha, true, "main", MYCELIUM);
    pushed["head"] = json!(sha);
    pushed
}

/// A commit as `Live` asks a compare to list it.
fn commit(sha: &str, parents: &[&str]) -> Value {
    json!({"sha": sha, "parents": parents})
}

/// [`commit`] as [`brought`] reads it.
fn read(sha: &str, parents: &[&str]) -> Brought {
    Brought {
        sha: sha.to_owned(),
        parents: parents.iter().map(|&parent| parent.to_owned()).collect(),
    }
}

#[test]
fn only_a_pull_request_merged_into_our_trunk_as_the_merge_from_its_head_vouches() {
    let merged = |pulls: Value| merged(&pulls, (MYCELIUM, "main"), B, C);
    let mut from_d = pull(B, true, "main", MYCELIUM);
    from_d["head"] = json!(D);
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
        (json!([pushed(B)]), "one whose head was pushed to the trunk"),
        (
            json!([from_d]),
            "one merged from another head than the merge's second parent",
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
fn only_merges_of_two_parents_lead_along_the_trunk_from_the_old_commit() {
    let line = [read(D, &[A, C]), read(C, &[A]), read(B, &[D, C])];
    assert_eq!(
        merges(&line, A, B),
        Ok(vec![
            (B.to_owned(), C.to_owned()),
            (D.to_owned(), C.to_owned())
        ]),
        "two merges along the trunk weren't each asked after with their heads"
    );
    for (commits, says, what) in [
        (
            vec![read(C, &[A]), read(D, &[A, C]), read(B, &[D])],
            "bbbbbbbbbbbb on the trunk's line is no merge",
            "a commit pushed on top of a merge",
        ),
        (
            vec![read(D, &[A]), read(C, &[D]), read(B, &[D, C])],
            "dddddddddddd on the trunk's line is no merge",
            "a commit pushed to the trunk before a merge carried it",
        ),
        (
            vec![read(C, &[A]), read(B, &[D, C])],
            "aaaaaaaaaaaa isn't on bbbbbbbbbbbb's trunk line",
            "a merge whose trunk line leaves what the compare lists",
        ),
        (
            vec![read(B, &[A, C, D])],
            "is no merge",
            "a merge of three parents",
        ),
        (
            vec![read(B, &[D, C]), read(D, &[B, C])],
            "in a circle",
            "a compare that goes round in a circle",
        ),
    ] {
        assert!(
            merges(&commits, A, B).is_err_and(|why| why.contains(says)),
            "{what} was taken along the trunk"
        );
    }
}

#[test]
fn a_compare_names_what_it_brings_only_when_it_lists_it_all_within_the_cap() {
    let compared = |total: usize, commits: Vec<Value>| {
        let listed = json!(commits);
        json!({"status": "ahead", "total_commits": total, "commits": listed})
    };
    let two = || vec![commit(C, &[A]), commit(B, &[A, C])];
    assert_eq!(
        brought(&compared(2, two())),
        Ok(vec![read(C, &[A]), read(B, &[A, C])]),
        "a compare listing all it brings wasn't read"
    );
    let many = |count: usize| -> Vec<Value> {
        (0..count)
            .map(|n| commit(&format!("{n:040x}"), &[A]))
            .collect()
    };
    assert!(
        brought(&compared(100, many(100))).is_ok_and(|read| read.len() == 100),
        "a compare bringing exactly the most a pin may wasn't read"
    );
    assert!(
        brought(&compared(101, many(101))).is_err_and(|why| why.contains("brings in 101 commits")),
        "a compare listing all of 101 commits was taken, past the cap"
    );
    assert!(
        brought(&compared(3, two())).is_err_and(|why| why.contains("lists 2 of the 3")),
        "a compare listing part of what it brings was taken for all"
    );
    for unread in [
        json!(B),
        json!({"sha": B}),
        json!({"sha": B, "parents": [1]}),
    ] {
        assert!(
            brought(&compared(1, vec![unread.clone()]))
                .is_err_and(|why| why.contains("can't be read")),
            "a compare listing {unread} was read"
        );
    }
    assert!(
        ahead(&compared(2, Vec::new())),
        "an ahead compare wasn't read as newer"
    );
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
    /// How long each pulls ask takes to answer.
    pub pulls_take: Duration,
    /// The most any ask was given to answer in.
    pub given: RefCell<Duration>,
}

impl Fake {
    /// A fake saying B is newer, merging C, a pull request's head, onto
    /// A, and is locked as the test's lock has it.
    pub(crate) fn vouching() -> Self {
        let merge = json!([pull(B, true, "main", MYCELIUM)]);
        Self {
            compared: Ok(Self::compare(&[commit(C, &[A]), commit(B, &[A, C])])),
            pulls: HashMap::from([(B, Ok(merge))]),
            prefetched: Ok(json!({"locked": lock(B, "n")["nodes"]["mycelium"]["locked"]})),
            asked: RefCell::new(Vec::new()),
            pulls_take: Duration::ZERO,
            given: RefCell::new(Duration::ZERO),
        }
    }

    /// An ahead compare listing `commits`, all it brings.
    fn compare(commits: &[Value]) -> Value {
        json!({"status": "ahead", "total_commits": commits.len(), "commits": commits})
    }

    /// Notes `ask`, given `within`.
    fn ask(&self, ask: String, within: Duration) {
        self.asked.borrow_mut().push(ask);
        let mut given = self.given.borrow_mut();
        *given = (*given).max(within);
    }

    /// What it was asked, in order.
    pub(crate) fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

impl Asks for Fake {
    fn compared(
        &self,
        repo: &str,
        old: &str,
        new: &str,
        within: Duration,
    ) -> Result<Value, String> {
        assert_eq!(
            (repo, old, new),
            (MYCELIUM, A, B),
            "asked for the wrong compare"
        );
        self.ask("compared".to_owned(), within);
        self.compared.clone()
    }

    fn pulls(&self, repo: &str, rev: &str, within: Duration) -> Result<Value, String> {
        assert_eq!(repo, MYCELIUM, "asked another repo's pulls");
        self.ask(format!("pulls {}", &rev[..1]), within);
        // Standing in for a GitHub slow to answer.
        std::thread::sleep(self.pulls_take);
        self.pulls
            .get(rev)
            .cloned()
            .unwrap_or_else(|| Ok(json!([])))
    }

    fn prefetched(&self, repo: &str, rev: &str, within: Duration) -> Result<Value, String> {
        assert_eq!((repo, rev), (MYCELIUM, B), "asked Nix wrongly");
        self.ask("prefetched".to_owned(), within);
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

const EVERY_ASK: [&str; 3] = ["compared", "pulls b", "prefetched"];

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
    assert!(
        *asks.given.borrow() <= LIMIT,
        "an ask was given longer than any may take"
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
    let mut head_pushed = Fake::vouching();
    head_pushed.pulls.insert(B, Ok(json!([pushed(B)])));
    declined(
        &repo,
        &head,
        &head_pushed,
        "no pull request into Jitsusama/mycelium's main merged as",
        &["compared", "pulls b"],
    );
    let on_top = Fake {
        compared: Ok(Fake::compare(&[
            commit(C, &[A]),
            commit(D, &[A, C]),
            commit(B, &[D]),
        ])),
        ..Fake::vouching()
    };
    declined(
        &repo,
        &head,
        &on_top,
        "bbbbbbbbbbbb on the trunk's line is no merge",
        &["compared"],
    );
    let pushed_first = Fake {
        compared: Ok(Fake::compare(&[
            commit(D, &[A]),
            commit(C, &[D]),
            commit(B, &[D, C]),
        ])),
        ..Fake::vouching()
    };
    declined(
        &repo,
        &head,
        &pushed_first,
        "dddddddddddd on the trunk's line is no merge",
        &["compared"],
    );
    let mut from_another = Fake::vouching();
    let mut other = pull(B, true, "main", MYCELIUM);
    other["head"] = json!(D);
    from_another.pulls.insert(B, Ok(json!([other])));
    declined(
        &repo,
        &head,
        &from_another,
        "merged as bbbbbbbbbbbb from cccccccccccc",
        &["compared", "pulls b"],
    );
    fs::remove_dir_all(&repo).unwrap();
}

#[test]
fn a_pin_whose_compare_is_past_the_cap_or_ends_elsewhere_asks_no_further() {
    let (repo, head) = pinned_to_b("compare", None);
    let past = Fake {
        compared: Ok(json!({"status": "ahead", "total_commits": 101,
            "commits": [commit(C, &[A]), commit(B, &[A, C])]})),
        ..Fake::vouching()
    };
    declined(&repo, &head, &past, "brings in 101", &["compared"]);
    let short = Fake {
        compared: Ok(Fake::compare(&[commit(C, &[A])])),
        ..Fake::vouching()
    };
    declined(
        &repo,
        &head,
        &short,
        "doesn't end at bbbbbbbbbbbb",
        &["compared"],
    );
    fs::remove_dir_all(&repo).unwrap();
}

#[test]
fn a_pin_whose_asks_run_past_their_time_needs_a_round() {
    let pin = moved(&lock(A, "n"), &lock(B, "n")).unwrap().pop().unwrap();
    let asks = Fake::vouching();
    assert!(
        vouched(&pin, &asks, Instant::now()).is_err_and(|why| why.contains("took longer than")),
        "a pin whose asks ran past their time was vouched for"
    );
    assert!(asks.asked().is_empty(), "a pin out of time asked anything");
    let slow = Fake {
        pulls_take: Duration::from_millis(300),
        ..Fake::vouching()
    };
    let by = Instant::now() + Duration::from_millis(200);
    assert!(
        vouched(&pin, &slow, by).is_err_and(|why| why.contains("took longer than")),
        "a pin whose time ran out before its prefetch was vouched for"
    );
    assert_eq!(
        slow.asked(),
        ["compared", "pulls b"],
        "a pin out of time asked Nix past it"
    );
    assert!(
        *slow.given.borrow() <= Duration::from_millis(200),
        "an ask was given longer than the time left"
    );
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
    pulls.pulls.insert(B, Err(failing("pulls")));
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
            "which pull request bbbbbbbbbbbb merged",
            "pulls",
            &EVERY_ASK[..2],
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
