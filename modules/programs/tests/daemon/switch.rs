//! Switch Branch over the socket: a checkout's branches served inside its
//! repository's row only while it's ahead, the one it's on first and one
//! another tree holds naming that tree; a switch checking a branch out
//! and its rows saying so at once; and a switch refused, saying why, over
//! changes of its own, onto a branch another tree holds, in a tree cut
//! from a checkout, and onto a branch there isn't.

use std::path::{Path, PathBuf};

use daemon_client::Client;
use serde_json::{Value, json};

use super::Running;
use super::code::{git_alone, real, row_of};

const SWITCH: &str = "row:code:github:someone/alpha:branches";

/// A checkout on main with a branch fix beside it, and a tree cut from it
/// at a branch held there, under the case's home.
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
        git(&["add", "one"]);
        git(&["commit", "--quiet", "-m", "one"]);
        git(&["branch", "fix"]);
        let held = running.path("trees/held");
        git(&[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "held",
            held.to_str().unwrap(),
        ]);
        Branched {
            home,
            here: real(&here),
            held: real(&held),
        }
    }

    fn on(&self) -> String {
        git_alone(&self.home, &self.here, &["symbolic-ref", "--short", "HEAD"])
    }
}

/// Every record of the next push that tells of the record with this key.
fn pushed_with(client: &mut Client, key: &str) -> Vec<Value> {
    loop {
        let said = client.heard();
        if let Some(records) = said["changed"].as_array()
            && records.iter().any(|record| record["key"] == key)
        {
            return records.clone();
        }
    }
}

fn side_of(records: &[Value], key: &str) -> Value {
    records
        .iter()
        .find(|record| record["key"] == key)
        .map_or(Value::Null, |record| record["row"]["side"][0].clone())
}

#[test]
fn a_checkouts_branches_are_served_while_ahead_and_a_switch_moves_them() {
    let running = Running::new("code-switch");
    let branched = Branched::new(&running);
    let mut watcher = running.client();
    let answer = watcher.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let row = row_of(&answer, SWITCH)
        .unwrap_or_else(|| panic!("a checkout has no Switch Branch inside it: {answer}"));
    assert_eq!(row["row"]["parent"], "row:code:github:someone/alpha");
    let (main, fix, held) = (
        format!("{SWITCH}:main"),
        format!("{SWITCH}:fix"),
        format!("{SWITCH}:held"),
    );
    assert!(
        row_of(&answer, &main).is_none(),
        "a checkout's branches were read before Switch Branch was ahead: {answer}"
    );
    watcher.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [SWITCH]}));
    let listed = pushed_with(&mut watcher, &main);
    assert_eq!(
        side_of(&listed, &main),
        json!({"said": "checked out"}),
        "the branch it's on doesn't say so: {listed:?}"
    );
    assert_eq!(
        side_of(&listed, &held),
        json!({"said": "in ~/trees/held"}),
        "a branch another tree holds doesn't name it: {listed:?}"
    );

    let mut client = running.client();
    let said = client.ask(&json!({"id": 2, "ask": "code:switch",
        "path": branched.here, "branch": "fix"}));
    assert_eq!(said["said"], "switched to fix", "{said}");
    assert_eq!(branched.on(), "fix");
    let moved = pushed_with(&mut watcher, &fix);
    assert_eq!(
        side_of(&moved, &fix),
        json!({"said": "checked out"}),
        "the branch switched to doesn't say it's checked out: {moved:?}"
    );
    let said = client.ask(&json!({"id": 3, "ask": "code:switch",
        "path": branched.here, "branch": "fix"}));
    assert_eq!(said["said"], "already on fix", "{said}");

    watcher.send(&json!({"id": "ahead", "ask": "unwatch"}));
    assert!(
        watcher.gone().contains(&json!(main)),
        "a branch's row stayed once Switch Branch wasn't ahead"
    );
}

#[test]
fn a_switch_never_carries_changes_takes_anothers_branch_or_touches_a_tree() {
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
        "not switched: no branch is called that now"
    );
    assert_eq!(
        switch(3, &branched.held, "fix"),
        "not switched: it isn't a checkout's own folder"
    );
    std::fs::write(branched.here.join("one"), "changed").unwrap();
    assert_eq!(
        switch(4, &branched.here, "fix"),
        "not switched: it has changes of its own"
    );
    assert_eq!(branched.on(), "main", "a refused switch moved the checkout");
}
