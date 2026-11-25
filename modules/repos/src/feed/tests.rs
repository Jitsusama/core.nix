use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::{Value, json};

use repos::model::{
    Change, Checkout, Checks, ChecksState, GitHubChange, Head, Record, Remote, Repository, Review,
    SCHEMA, Source, Tree, TreeKind, Weight,
};
use repos::using::Use;

use super::{records, sources, unread, using};

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
        asks_your_review: false,
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
                weight: Some(Weight {
                    loose: 512,
                    estimated: false,
                    loose_bytes: 1024,
                    packs: 1,
                    packed_bytes: 4096,
                    due: false,
                }),
                branches: Some(3),
                config_bytes: Some(220),
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
fn a_checkout_says_what_its_git_store_weighs() {
    let said = records(&record());
    let checkout = find(&said, "checkout:/home/joel/src/core.nix");
    assert_eq!(
        checkout["weight"],
        json!({
            "loose": 512,
            "estimated": false,
            "loose_bytes": 1024,
            "packs": 1,
            "packed_bytes": 4096,
            "due": false,
        })
    );
}

#[test]
fn a_checkout_says_how_many_branches_and_config_bytes_it_keeps() {
    let said = records(&record());
    let checkout = find(&said, "checkout:/home/joel/src/core.nix");
    assert_eq!(
        (&checkout["branches"], &checkout["config_bytes"]),
        (&json!(3), &json!(220))
    );
}

#[test]
fn a_checkout_whose_counts_couldnt_be_read_says_null_rather_than_zero() {
    let mut unread = record();
    let checkout = &mut unread.repositories[0].checkouts[0];
    checkout.branches = None;
    checkout.config_bytes = None;
    let said = records(&unread);
    let checkout = find(&said, "checkout:/home/joel/src/core.nix");
    assert_eq!(
        (&checkout["branches"], &checkout["config_bytes"]),
        (&Value::Null, &Value::Null),
        "{checkout:#?}"
    );
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
    // A kept file no refresh wrote, or one that can't be read, was never
    // GitHub's answer.
    read.sources[1].read_at = None;
    read.sources[1].unread = vec!["github.json couldn't be read: denied".to_owned()];
    assert_eq!(
        sources(&read)["github"],
        json!({"state": "not answering", "reason": "github.json couldn't be read: denied"})
    );
}

/// Every folder `record()` holds, as a reading that placed processes in
/// each knows them.
fn placed() -> BTreeSet<PathBuf> {
    [
        "/home/joel/src/core.nix",
        "/home/joel/trees/say-trees",
        "/home/joel/trees/pinned",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

fn used(processes: usize, memory: u64, cpu: f64) -> Use {
    Use {
        processes,
        memory,
        cpu: Some(cpu),
    }
}

#[test]
fn what_works_in_a_repository_is_summed_over_its_checkouts_and_trees() {
    let used = BTreeMap::from([
        (
            PathBuf::from("/home/joel/src/core.nix"),
            used(2, 3_000, 0.25),
        ),
        (
            PathBuf::from("/home/joel/trees/say-trees"),
            used(1, 1_000, 1.5),
        ),
        // A folder no repository holds counts for none.
        (PathBuf::from("/home/joel/elsewhere"), used(9, 9_000, 9.0)),
    ]);
    let said = using(&record(), &used, true, &placed());
    assert_eq!(
        said,
        [json!({
            "key": "using:github:Jitsusama/core.nix",
            "kind": "using",
            "repository": "github:Jitsusama/core.nix",
            "processes": 3,
            "memory": 4_000,
            "cpu": 1.75,
            "folders": [
                {"path": "/home/joel/src/core.nix", "processes": 2, "memory": 3_000, "cpu": 0.25},
                {"path": "/home/joel/trees/say-trees", "processes": 1, "memory": 1_000, "cpu": 1.5},
            ],
        })]
    );
}

#[test]
fn what_works_unread_is_said_unknown_rather_than_none() {
    let unread = &unread(&record())[0];
    assert_eq!(unread["key"], "using:github:Jitsusama/core.nix");
    assert_eq!(
        (
            &unread["processes"],
            &unread["memory"],
            &unread["cpu"],
            &unread["folders"]
        ),
        (&Value::Null, &Value::Null, &Value::Null, &json!([])),
        "a reading stopped reads as nothing working"
    );
}

#[test]
fn a_repository_nothing_works_in_says_so_and_its_processor_waits_for_a_second_reading() {
    let idle = &using(&record(), &BTreeMap::new(), true, &placed())[0];
    assert_eq!(
        (
            &idle["processes"],
            &idle["memory"],
            &idle["cpu"],
            &idle["folders"]
        ),
        (&json!(0), &json!(0), &json!(0.0), &json!([]))
    );
    let first = &using(&record(), &BTreeMap::new(), false, &placed())[0];
    assert_eq!(
        first["cpu"],
        Value::Null,
        "one reading can't say a share of time"
    );
}

#[test]
fn a_tree_newer_than_the_reading_is_said_unread_and_its_repository_as_read() {
    // The reading placed processes in the folders the disk held before the
    // tree say-trees was made, so whatever works there wasn't looked for,
    // and what works in the rest was.
    let mut before = placed();
    before.remove(&PathBuf::from("/home/joel/trees/say-trees"));
    let used = BTreeMap::from([(
        PathBuf::from("/home/joel/src/core.nix"),
        used(2, 3_000, 0.25),
    )]);
    let said = &using(&record(), &used, true, &before)[0];
    assert_eq!(
        (&said["processes"], &said["memory"], &said["cpu"]),
        (&json!(2), &json!(3_000), &json!(0.25)),
        "a new tree blanked what was read of the rest: {said}"
    );
    let new = said["folders"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|folder| folder["path"] == "/home/joel/trees/say-trees");
    assert_eq!(
        new.map(|folder| &folder["processes"]),
        Some(&Value::Null),
        "a tree the reading never looked in reads as idle: {said}"
    );
}

#[test]
fn a_folder_whose_share_went_untold_says_none_and_its_repository_too() {
    // As for a tree only one reading has looked in: a share of some of
    // what works in a repository would read as all of it.
    let used = BTreeMap::from([
        (
            PathBuf::from("/home/joel/src/core.nix"),
            used(2, 3_000, 0.25),
        ),
        (
            PathBuf::from("/home/joel/trees/say-trees"),
            Use {
                processes: 1,
                memory: 1_000,
                cpu: None,
            },
        ),
    ]);
    let said = &using(&record(), &used, true, &placed())[0];
    assert_eq!(
        (&said["processes"], &said["memory"], &said["cpu"]),
        (&json!(3), &json!(4_000), &Value::Null),
        "a share of part of a repository was said as its whole: {said}"
    );
    assert_eq!(
        said["folders"][1],
        json!({"path": "/home/joel/trees/say-trees", "processes": 1, "memory": 1_000, "cpu": null}),
        "a folder with no share told said one"
    );
}

#[test]
fn a_repository_none_of_whose_folders_the_reading_looked_in_is_unread_whole() {
    let said = &using(&record(), &BTreeMap::new(), true, &BTreeSet::new())[0];
    assert_eq!(
        said,
        &unread(&record())[0],
        "a repository the reading never looked in reads as idle"
    );
}
