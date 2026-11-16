use serde_json::{Value, json};

use repos::model::{
    Change, Checkout, Checks, ChecksState, GitHubChange, Head, Record, Remote, Repository, Review,
    SCHEMA, Source, Tree, TreeKind,
};

use super::{records, sources};

/// 14 November 2026, 12:00 UTC.
const NOON: u64 = 1_794_657_600;

fn head(branch: Option<&str>, commit: &str) -> Head {
    Head {
        branch: branch.map(str::to_owned),
        commit: Some(commit.to_owned()),
        upstream: None,
    }
}

fn change(branch: &str, number: Option<u64>) -> Change {
    Change {
        title: "Say the Trees".to_owned(),
        author: Some("Jitsusama".to_owned()),
        draft: false,
        branch: branch.to_owned(),
        base: "main".to_owned(),
        commit: Some("c0ffee".to_owned()),
        updated_at: NOON,
        checks: Checks {
            state: ChecksState::Passing,
            passed: 3,
            failed: Vec::new(),
            pending: Vec::new(),
        },
        review: Review::Approved,
        github: number.map(|number| GitHubChange {
            number,
            url: format!("https://github.com/Jitsusama/core.nix/pull/{number}"),
        }),
        review_asked_of: Vec::new(),
    }
}

/// core.nix, one checkout with a worktree at a change's branch and a
/// snapshot, and a change put up from no tree.
fn record() -> Record {
    Record {
        schema: SCHEMA.to_owned(),
        read_at: NOON,
        machine: "laptop".to_owned(),
        roots: vec!["/home/joel/src".to_owned()],
        sources: vec![Source {
            name: "git".to_owned(),
            what: "each checkout's and tree's own git files".to_owned(),
            read_at: None,
            unread: Vec::new(),
        }],
        repositories: vec![Repository {
            key: "github:Jitsusama/core.nix".to_owned(),
            name: "core.nix".to_owned(),
            owner: Some("Jitsusama".to_owned()),
            remotes: vec![Remote {
                name: "origin".to_owned(),
                url: "git@github.com:Jitsusama/core.nix.git".to_owned(),
            }],
            trunk: Some("main".to_owned()),
            written_at: Some(NOON),
            checkouts: vec![Checkout {
                path: "/home/joel/src/core.nix".to_owned(),
                bare: false,
                head: Some(head(Some("main"), "aaaa")),
                written_at: Some(NOON - 60),
                trees: vec![
                    Tree {
                        path: "/home/joel/trees/say-trees".to_owned(),
                        kind: TreeKind::Worktree,
                        head: head(Some("joel/say-trees"), "bbbb"),
                        present: true,
                        written_at: Some(NOON),
                    },
                    Tree {
                        path: "/home/joel/trees/pinned".to_owned(),
                        kind: TreeKind::Snapshot,
                        head: head(None, "cccc"),
                        present: false,
                        written_at: None,
                    },
                ],
            }],
            changes: vec![
                change("joel/say-trees", Some(42)),
                change("elsewhere", None),
            ],
            read_through: None,
        }],
    }
}

fn find<'a>(said: &'a [Value], key: &str) -> &'a Value {
    said.iter()
        .find(|one| one["key"] == key)
        .unwrap_or_else(|| panic!("no record {key} in {said:#?}"))
}

#[test]
fn each_kind_is_a_record_of_its_own_keyed_by_what_names_it() {
    let said = records(&record());
    let keys: Vec<&str> = said.iter().filter_map(|one| one["key"].as_str()).collect();
    assert_eq!(
        keys,
        [
            "reading",
            "repository:github:Jitsusama/core.nix",
            "checkout:/home/joel/src/core.nix",
            "tree:/home/joel/trees/say-trees",
            "tree:/home/joel/trees/pinned",
            "change:github:Jitsusama/core.nix#42",
            "change:github:Jitsusama/core.nix@elsewhere",
        ]
    );
    for one in &said {
        let key = one["key"].as_str().unwrap_or_default();
        let kind = key.split(':').next().unwrap_or_default();
        assert_eq!(one["kind"], kind, "{key} says its kind");
    }
}

#[test]
fn a_tree_names_its_checkout_its_repository_and_the_change_at_its_branch() {
    let said = records(&record());
    let tree = find(&said, "tree:/home/joel/trees/say-trees");
    assert_eq!(tree["repository"], "github:Jitsusama/core.nix");
    assert_eq!(tree["checkout"], "/home/joel/src/core.nix");
    assert_eq!(tree["snapshot"], false);
    assert_eq!(tree["change"], "change:github:Jitsusama/core.nix#42");
    assert_eq!(tree["head"]["branch"], "joel/say-trees");

    let pinned = find(&said, "tree:/home/joel/trees/pinned");
    assert_eq!(pinned["snapshot"], true);
    assert_eq!(pinned["present"], false);
    assert_eq!(pinned["change"], Value::Null);
}

#[test]
fn times_are_said_in_rfc_3339_as_every_answer_says_them() {
    let said = records(&record());
    let repository = find(&said, "repository:github:Jitsusama/core.nix");
    assert_eq!(repository["written_at"], "2026-11-14T12:00:00Z");
    let change = find(&said, "change:github:Jitsusama/core.nix#42");
    assert_eq!(change["updated_at"], "2026-11-14T12:00:00Z");
    assert_eq!(change["checks"]["state"], "passing");
    assert_eq!(change["repository"], "github:Jitsusama/core.nix");
}

#[test]
fn the_reading_leaves_out_when_it_was_read_so_a_reading_alone_tells_nothing() {
    let first = records(&record());
    let mut later = record();
    later.read_at += 60;
    assert_eq!(first, records(&later));
}

#[test]
fn github_answers_once_a_refresh_has_asked_it_saying_what_it_couldnt_see() {
    let mut read = record();
    assert_eq!(
        sources(&read),
        json!({
            "git": "answering",
            "github": {"state": "not answering", "reason": "no refresh has asked GitHub yet"},
        })
    );
    read.sources.push(Source {
        name: "github".to_owned(),
        what: "each repository's open changes".to_owned(),
        read_at: Some(NOON),
        unread: Vec::new(),
    });
    assert_eq!(sources(&read)["github"], "answering");
    read.sources[1].unread = vec!["github:someone/private: not found".to_owned()];
    assert_eq!(
        sources(&read)["github"],
        json!({"state": "answering", "reason": "github:someone/private: not found"})
    );
}
