//! Switch Branch over the socket: a checkout's branches served inside its
//! repository's row only while it's ahead, the one it's on first, then
//! the newest, and one another tree holds naming that tree; a switch
//! checking a branch out past a file git doesn't track, its branches and
//! files listed again at once, and one onto a branch gone listing them
//! without it; and a switch refused, saying why, over changes of its own,
//! over a file in the way, onto a branch another tree holds, in a tree
//! cut from a checkout, and onto a branch there isn't.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use daemon_client::Client;
use serde_json::{Value, json};

use super::Running;
use super::code::{git_alone, git_command, real, row_of};

const SWITCH: &str = "row:code:github:someone/alpha:branches";
const FIND: &str = "row:code:github:someone/alpha:files";

/// A checkout on main, whose one commit is years old and ignores `kept`,
/// with a branch fix beside it, newer and tracking `kept` and `fixed`,
/// and a tree cut from it at a branch held there, under the case's home.
struct Branched {
    home: PathBuf,
    here: PathBuf,
    held: PathBuf,
}

impl Branched {
    fn new(running: &Running) -> Branched {
        let (home, here) = (running.path(""), running.path("src/alpha"));
        let git = |args: &[&str]| git_alone(&home, &here, args);
        git(&["init", "--quiet", "-b", "main"]);
        git(&[
            "remote",
            "add",
            "origin",
            "git@github.com:someone/alpha.git",
        ]);
        std::fs::write(here.join("one"), "one").unwrap();
        std::fs::write(here.join(".gitignore"), "kept\n").unwrap();
        git(&["add", "one", ".gitignore"]);
        // Dated, so fix's commit is newer whatever second the case runs.
        let dated = git_command(&home, &here, &["commit", "--quiet", "-m", "one"])
            .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
            .status()
            .unwrap();
        assert!(dated.success(), "main's commit failed");
        let held = running.path("trees/held");
        git(&[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "held",
            held.to_str().unwrap(),
        ]);
        git(&["switch", "--quiet", "-c", "fix"]);
        std::fs::write(here.join("kept"), "fix's").unwrap();
        std::fs::write(here.join("fixed"), "fixed").unwrap();
        git(&["add", "--force", "kept", "fixed"]);
        git(&["commit", "--quiet", "-m", "fix"]);
        git(&["switch", "--quiet", "main"]);
        Branched {
            home,
            here: real(&here),
            held: real(&held),
        }
    }

    fn on(&self) -> String {
        git_alone(&self.home, &self.here, &["symbolic-ref", "--short", "HEAD"])
    }

    /// The key of the row of a file in the checkout, inside Open a File
    /// by Name.
    fn file(&self, name: &str) -> String {
        format!("{FIND}:{}/{name}", self.here.display())
    }
}

/// The rows of `wanted`'s keys as the pushes from here on last told of
/// them, once every one has been told of.
fn pushed(client: &mut Client, wanted: &[&str]) -> HashMap<String, Value> {
    let mut told = HashMap::new();
    while !wanted.iter().all(|key| told.contains_key(*key)) {
        let said = client.heard();
        for record in said["changed"].as_array().into_iter().flatten() {
            if let Some(key) = record["key"].as_str() {
                told.insert(key.to_owned(), record["row"].clone());
            }
        }
    }
    told
}

fn branch(name: &str) -> String {
    format!("{SWITCH}:{name}")
}

#[test]
fn a_checkouts_branches_are_served_while_ahead_the_one_its_on_first() {
    let running = Running::new("code-switch-listed");
    Branched::new(&running);
    let mut watcher = running.client();
    let answer = watcher.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let row = row_of(&answer, SWITCH)
        .unwrap_or_else(|| panic!("a checkout has no Switch Branch inside it: {answer}"));
    assert_eq!(row["row"]["parent"], "row:code:github:someone/alpha");
    let (main, fix, held) = (branch("main"), branch("fix"), branch("held"));
    assert!(
        row_of(&answer, &main).is_none(),
        "a checkout's branches were read before Switch Branch was ahead: {answer}"
    );
    watcher.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [SWITCH]}));
    let listed = pushed(&mut watcher, &[&main, &fix, &held]);
    // The shell places rows by their order alone, never sorting them.
    let mut placed = [&main, &fix, &held];
    placed.sort_by_key(|key| listed[*key]["order"].as_i64());
    assert_eq!(
        placed,
        [&main, &fix, &held],
        "not the one it's on, then the newest: {listed:?}"
    );
    assert_eq!(
        listed[&main]["side"][0],
        json!({"said": "checked out here"}),
        "the branch it's on doesn't say so: {listed:?}"
    );
    assert_eq!(
        listed[&main]["acts"],
        json!([]),
        "the branch it's on can be switched to: {listed:?}"
    );
    assert_eq!(
        listed[&held]["side"][0],
        json!({"said": "in ~/trees/held"}),
        "a branch another tree holds doesn't name it: {listed:?}"
    );
    let at = |key: &str, slot: usize| {
        listed[key]["side"][slot]["at"]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} says no date: {listed:?}"))
    };
    assert!(
        at(&fix, 0) > at(&held, 1),
        "fix isn't said to be newer than held: {listed:?}"
    );
    watcher.send(&json!({"id": "ahead", "ask": "unwatch"}));
    assert!(
        watcher.gone().contains(&json!(main)),
        "a branch's row stayed once Switch Branch wasn't ahead"
    );
}

#[test]
fn a_switch_checks_a_branch_out_and_its_branches_and_files_follow() {
    let running = Running::new("code-switch-moves");
    let branched = Branched::new(&running);
    let mut watcher = running.client();
    watcher.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    watcher.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"],
        "keys": [SWITCH, FIND]}));
    let (main, fix) = (branch("main"), branch("fix"));
    pushed(&mut watcher, &[&main, &fix, &branched.file("one")]);
    // A file git doesn't track, and fix doesn't either, is no reason to
    // stay.
    std::fs::write(branched.here.join("scratch"), "mine").unwrap();

    let mut client = running.client();
    let said = client.ask(&json!({"id": 2, "ask": "code:switch",
        "path": branched.here, "branch": "fix"}));
    assert_eq!(said["said"], "switched to fix", "{said}");
    assert_eq!(branched.on(), "fix");
    assert!(
        branched.here.join("scratch").is_file(),
        "the switch lost a file of Joel's"
    );
    let moved = pushed(&mut watcher, &[&main, &fix, &branched.file("fixed")]);
    assert_eq!(
        moved[&fix]["side"][0],
        json!({"said": "checked out here"}),
        "the branch switched to doesn't say it's checked out: {moved:?}"
    );
    assert_eq!(moved[&fix]["acts"], json!([]), "{moved:?}");
    assert_ne!(
        moved[&main]["side"][0],
        json!({"said": "checked out here"}),
        "the branch left still says it's checked out: {moved:?}"
    );
    assert_eq!(
        moved[&main]["acts"][0]["key"], "verb:code:switch",
        "the branch left can't be switched back to: {moved:?}"
    );
    let said = client.ask(&json!({"id": 3, "ask": "code:switch",
        "path": branched.here, "branch": "fix"}));
    assert_eq!(said["said"], "already on fix", "{said}");

    // A branch gone since it was listed is refused, and listed no more.
    git_alone(
        &branched.home,
        &branched.here,
        &["branch", "--quiet", "-D", "main"],
    );
    let said = client.ask(&json!({"id": 4, "ask": "code:switch",
        "path": branched.here, "branch": "main"}));
    assert_eq!(
        said["error"]["said"],
        "not switched: there's no branch main now"
    );
    assert!(
        watcher.gone().contains(&json!(main)),
        "a branch gone is still listed after a switch to it was refused"
    );
}

#[test]
fn a_switch_refuses_changes_files_in_the_way_trees_and_branches_unknown_or_held() {
    let running = Running::new("code-switch-refused");
    let branched = Branched::new(&running);
    let mut client = running.client();
    let mut switch = |id: u64, folder: &Path, branch: &str| {
        let said = client.ask(&json!({"id": id, "ask": "code:switch",
            "path": folder, "branch": branch}));
        said["error"]["said"].clone()
    };
    assert_eq!(
        switch(1, &branched.here, "held"),
        "not switched: ~/trees/held has it checked out"
    );
    assert_eq!(
        switch(2, &branched.here, "nope"),
        "not switched: there's no branch nope now"
    );
    assert_eq!(
        switch(3, &branched.held, "fix"),
        "not switched: it isn't a checkout's own folder"
    );
    // Ignored on main and tracked on fix, which git would overwrite.
    std::fs::write(branched.here.join("kept"), "Joel's").unwrap();
    assert_eq!(
        switch(4, &branched.here, "fix"),
        "not switched: files git doesn't track are in the way"
    );
    assert_eq!(
        std::fs::read_to_string(branched.here.join("kept")).unwrap(),
        "Joel's",
        "a refused switch overwrote an ignored file"
    );
    std::fs::write(branched.here.join("one"), "changed").unwrap();
    assert_eq!(
        switch(5, &branched.here, "fix"),
        "not switched: it has changes of its own"
    );
    assert_eq!(branched.on(), "main", "a refused switch moved the checkout");
}
