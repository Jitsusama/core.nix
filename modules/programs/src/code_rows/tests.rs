use super::*;
use repos::model::{
    Change, Checks, ChecksState, Count, Head, Remote, Review, SCHEMA, Tree, TreeKind, Upstream,
};

fn words() -> Words {
    Words::read(&json!({"things": [
        {"id": "code:repository", "said": {
            "kind": "repository", "tree": "1 tree", "trees": "{count} trees",
            "local": "no remote", "change": "1 change", "changes": "{count} changes",
            "behind": "{count} behind", "ahead": "{count} ahead",
            "uncommitted": "{count} uncommitted",
        }},
        {"id": "code:key", "name": "Key"},
        {"id": "code:trunk", "name": "Trunk"},
        {"id": "code:checkout", "name": "Checkout"},
        {"id": "code:branch", "name": "Branch", "glyph": "branch", "said": {
            "here": "checked out", "elsewhere": "in {tree}",
        }},
        {"id": "verb:code:branches", "name": "Switch Branch", "glyph": "branch"},
        {"id": "code:remote", "name": "Remote"},
        {"id": "verb:code:find", "name": "Open a File by Name", "glyph": "folder"},
        {"id": "code:file", "name": "File", "glyph": "path"},
    ]}))
}

/// The rows as they're made where no checkout's uncommitted files were
/// counted.
fn rows(record: &Record, home: Option<&Path>, words: &Words) -> Vec<Value> {
    super::rows(record, home, words, &BTreeMap::new())
}

fn tree(path: &str, present: bool) -> Tree {
    Tree {
        path: path.to_owned(),
        kind: TreeKind::Worktree,
        head: Head {
            branch: Some("work".to_owned()),
            commit: None,
            upstream: None,
        },
        present,
        written_at: Some(10),
    }
}

fn checkout(path: &str, bare: bool, trees: Vec<Tree>) -> Checkout {
    Checkout {
        path: path.to_owned(),
        bare,
        head: (!bare).then(|| Head {
            branch: Some("main".to_owned()),
            commit: None,
            upstream: None,
        }),
        written_at: Some(20),
        trees,
        weight: None,
    }
}

fn core() -> Repository {
    Repository {
        key: "github:Jitsusama/core.nix".to_owned(),
        name: "core.nix".to_owned(),
        owner: Some("Jitsusama".to_owned()),
        remotes: vec![Remote {
            name: "origin".to_owned(),
            url: "git@github.com:Jitsusama/core.nix.git".to_owned(),
        }],
        trunk: Some("main".to_owned()),
        written_at: Some(1_700_000_000),
        checkouts: vec![
            checkout(
                "/home/j/src/core.nix",
                false,
                vec![tree("/t/a", true), tree("/t/b", true), tree("/t/c", false)],
            ),
            checkout("/home/j/old/core.nix", false, vec![tree("/t/d", true)]),
        ],
        changes: Vec::new(),
        read_through: None,
    }
}

fn change(title: &str) -> Change {
    Change {
        title: title.to_owned(),
        author: Some("Jitsusama".to_owned()),
        draft: false,
        branch: "work".to_owned(),
        base: "main".to_owned(),
        commit: None,
        updated_at: 30,
        checks: Checks {
            state: ChecksState::Passing,
            passed: 1,
            failed: Vec::new(),
            pending: Vec::new(),
        },
        review: Review::Waiting,
        review_asked_of: Vec::new(),
        github: None,
    }
}

fn record(repositories: Vec<Repository>) -> Record {
    Record {
        schema: SCHEMA.to_owned(),
        read_at: 0,
        machine: "here".to_owned(),
        roots: Vec::new(),
        sources: Vec::new(),
        repositories,
    }
}

#[test]
fn a_repository_is_one_row_said_by_its_trees_and_when_it_was_written() {
    let rows = rows(&record(vec![core()]), Some(Path::new("/home/j")), &words());
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["key"], "row:code:github:Jitsusama/core.nix");
    assert_eq!(row["place"], "code");
    assert_eq!(row["name"], "core.nix");
    assert_eq!(
        row["side"],
        json!([{"said": "3 trees"}, {"at": 1_700_000_000_000_i64, "as": "came", "keep": 1}]),
        "a gone tree isn't counted, and every checkout's trees are"
    );
    assert_eq!(
        row["keywords"],
        "github:Jitsusama/core.nix Jitsusama ~/src/core.nix ~/old/core.nix"
    );
}

#[test]
fn return_opens_its_page_and_copy_a_terminal_and_an_editor_take_the_checkout_written_to_last() {
    let rows = rows(&record(vec![core()]), None, &words());
    assert_eq!(
        rows[0]["acts"],
        json!([
            {"key": "verb:code:page", "open": "https://github.com/Jitsusama/core.nix"},
            {"key": "verb:code:path", "copy": "/home/j/src/core.nix"},
            {"key": "verb:code:terminal", "closes": true,
             "ask": {"ask": "code:terminal", "path": "/home/j/src/core.nix"}},
            {"key": "verb:code:edit", "closes": true,
             "ask": {"ask": "code:edit", "path": "/home/j/src/core.nix"}},
        ])
    );
}

#[test]
fn open_changes_are_counted_first_beside_it_and_opened_at_its_forges_list() {
    let mut repository = core();
    repository.changes = vec![change("one"), change("two")];
    let rows = rows(&record(vec![repository]), None, &words());
    assert_eq!(
        rows[0]["side"][0],
        json!({"said": "2 changes"}),
        "open changes weren't said first beside it"
    );
    assert_eq!(
        rows[0]["acts"][1],
        json!({
            "key": "verb:code:changes",
            "open": "https://github.com/Jitsusama/core.nix/pulls",
        }),
        "its open changes weren't offered after its page"
    );
}

/// Core with the checkout written to last this far from its branch,
/// where it's been counted.
fn apart(count: Option<Count>) -> Repository {
    let mut repository = core();
    if let Some(head) = repository.checkouts[0].head.as_mut() {
        head.commit = Some("a".repeat(40));
        head.upstream = Some(Upstream {
            branch: "origin/main".to_owned(),
            commit: Some("b".repeat(40)),
            count,
        });
    }
    repository
}

#[test]
fn a_checkout_apart_from_its_branch_says_how_far_after_its_open_changes() {
    let mut repository = apart(Some(Count {
        behind: 3,
        ahead: 1,
    }));
    repository.changes = vec![change("one")];
    let rows = rows(&record(vec![repository]), None, &words());
    assert_eq!(
        rows[0]["side"],
        json!([
            {"said": "1 change"},
            {"said": "3 behind"},
            {"said": "1 ahead"},
            {"said": "3 trees"},
            {"at": 1_700_000_000_000_i64, "as": "came", "keep": 1},
        ]),
        "how far its checkout is behind and ahead wasn't said after its open changes"
    );
}

#[test]
fn a_checkout_with_files_not_committed_says_how_many_after_how_far_apart() {
    let repository = apart(Some(Count {
        behind: 3,
        ahead: 1,
    }));
    let uncommitted = BTreeMap::from([("/home/j/src/core.nix".to_owned(), 2)]);
    let rows = super::rows(&record(vec![repository]), None, &words(), &uncommitted);
    assert_eq!(
        rows[0]["side"],
        json!([
            {"said": "3 behind"},
            {"said": "1 ahead"},
            {"said": "2 uncommitted"},
            {"said": "3 trees"},
            {"at": 1_700_000_000_000_i64, "as": "came", "keep": 1},
        ]),
        "how many files its checkout hasn't committed wasn't said after how far apart it is"
    );
}

#[test]
fn none_uncommitted_uncounted_bare_or_another_checkouts_says_nothing_and_many_say_100_plus() {
    let said_of = |repository: Repository, path: &str, count: u32| {
        let uncommitted = BTreeMap::from([(path.to_owned(), count)]);
        super::rows(&record(vec![repository]), None, &words(), &uncommitted)[0]["side"][0].clone()
    };
    let said = |path: &str, count: u32| said_of(core(), path, count);
    let trees = json!({"said": "3 trees"});
    assert_eq!(
        said("/home/j/src/core.nix", 0),
        trees,
        "a checkout with no files uncommitted said 0 uncommitted"
    );
    let bare = Repository {
        checkouts: vec![checkout(
            "/home/j/src/core.nix.git",
            true,
            vec![tree("/t/a", true)],
        )],
        ..core()
    };
    assert_eq!(
        said_of(bare, "/home/j/src/core.nix.git", 2),
        json!({"said": "1 tree"}),
        "a bare checkout, with no files of its own, said how many it hasn't committed"
    );
    assert_eq!(
        said("/home/j/old/core.nix", 4),
        trees,
        "a checkout not worked in said how many it hasn't committed"
    );
    assert_eq!(
        rows(&record(vec![core()]), None, &words())[0]["side"][0],
        trees,
        "a checkout not counted said how many it hasn't committed"
    );
    assert_eq!(
        said("/home/j/src/core.nix", COUNTED_TO),
        json!({"said": "100+ uncommitted"}),
        "past the count's end didn't say as much"
    );
}

#[test]
fn past_the_counts_end_it_says_as_much_and_even_or_uncounted_says_nothing() {
    let far = apart(Some(Count {
        behind: COUNTED_TO,
        ahead: COUNTED_TO,
    }));
    let ahead = apart(Some(Count {
        behind: 0,
        ahead: 2,
    }));
    let rows = rows(&record(vec![far, ahead, apart(None)]), None, &words());
    assert_eq!(rows[0]["side"][0], json!({"said": "100+ behind"}));
    assert_eq!(rows[0]["side"][1], json!({"said": "100+ ahead"}));
    assert_eq!(
        rows[1]["side"][0],
        json!({"said": "2 ahead"}),
        "a checkout only ahead didn't say so alone"
    );
    assert_eq!(
        rows[1]["side"][1],
        json!({"said": "3 trees"}),
        "a checkout only ahead said it was behind"
    );
    assert_eq!(
        rows[2]["side"][0],
        json!({"said": "3 trees"}),
        "a checkout not yet counted said how far it was"
    );
}

#[test]
fn one_open_change_is_said_alone_and_none_says_nothing() {
    let mut repository = core();
    repository.changes = vec![change("one")];
    let rows = rows(&record(vec![repository, core()]), None, &words());
    assert_eq!(rows[0]["side"][0], json!({"said": "1 change"}));
    assert_eq!(
        rows[1]["side"][0],
        json!({"said": "3 trees"}),
        "a repository with no open changes said some"
    );
    let offered = rows[1]["acts"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|act| act["key"] == "verb:code:changes");
    assert!(!offered, "a repository with no open changes offered them");
}

#[test]
fn gitlabs_open_changes_are_its_merge_requests_and_a_local_ones_have_no_list() {
    assert_eq!(
        changes_page("gitlab:someone/thing").as_deref(),
        Some("https://gitlab.com/someone/thing/-/merge_requests")
    );
    assert_eq!(
        changes_page("github:someone/thing").as_deref(),
        Some("https://github.com/someone/thing/pulls")
    );
    assert_eq!(changes_page("local:~/scratch/thing"), None);
}

#[test]
fn pull_is_offered_last_on_a_checkout_whose_branch_follows_another_and_only_there() {
    let mut repository = core();
    let head = repository.checkouts[0].head.as_mut().unwrap();
    head.upstream = Some(Upstream {
        branch: "origin/main".to_owned(),
        commit: None,
        count: None,
    });
    let rows = rows(&record(vec![repository.clone()]), None, &words());
    assert_eq!(
        rows[0]["acts"].as_array().unwrap().last(),
        Some(&json!({"key": "verb:code:pull", "streams": true,
               "ask": {"ask": "code:pull", "path": "/home/j/src/core.nix"}})),
        "Pull isn't offered last on the checkout its branch follows another in"
    );
    let offered = |repository: &Repository| {
        self::rows(&record(vec![repository.clone()]), None, &words())[0]["acts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|act| act["key"] == "verb:code:pull")
    };
    let mut unfollowed = repository.clone();
    unfollowed.checkouts[0].head.as_mut().unwrap().upstream = None;
    assert!(
        !offered(&unfollowed),
        "Pull was offered on a branch that follows none"
    );
    repository.checkouts[0].bare = true;
    repository.checkouts[0].head = None;
    assert!(
        !offered(&repository),
        "Pull was offered on a checkout with no files of its own"
    );
}

#[test]
fn a_repository_with_no_checkout_opens_no_terminal() {
    let mut repository = core();
    repository.checkouts.clear();
    let rows = rows(&record(vec![repository]), None, &words());
    assert_eq!(
        rows[0]["acts"],
        json!([{"key": "verb:code:page", "open": "https://github.com/Jitsusama/core.nix"}]),
        "a terminal was offered with nowhere to open it"
    );
}

#[test]
fn a_repository_with_no_remote_copies_its_path_and_counts_no_trees() {
    let local = Repository {
        key: "local:~/scratch/thing".to_owned(),
        name: "thing".to_owned(),
        owner: None,
        remotes: Vec::new(),
        trunk: None,
        written_at: None,
        checkouts: vec![checkout("/home/j/scratch/thing", false, Vec::new())],
        changes: Vec::new(),
        read_through: None,
    };
    let rows = rows(&record(vec![local]), Some(Path::new("/home/j")), &words());
    let row = &rows[0];
    assert_eq!(row["side"], json!([]), "nothing to say beside it");
    assert_eq!(
        row["acts"],
        json!([
            {"key": "verb:code:path", "copy": "/home/j/scratch/thing"},
            {"key": "verb:code:terminal", "closes": true,
             "ask": {"ask": "code:terminal", "path": "/home/j/scratch/thing"}},
            {"key": "verb:code:edit", "closes": true,
             "ask": {"ask": "code:edit", "path": "/home/j/scratch/thing"}},
        ])
    );
    assert_eq!(row["drawer"]["eyebrow"][0]["said"], "no remote");
    assert!(row["drawer"].get("nerd").is_none(), "no remote to name");
}

#[test]
fn a_bare_checkout_is_passed_over_for_one_with_files() {
    let mut repository = core();
    repository
        .checkouts
        .insert(0, checkout("/home/j/src/big.git", true, Vec::new()));
    let rows = rows(&record(vec![repository]), None, &words());
    assert_eq!(rows[0]["held"]["path"], "/home/j/src/core.nix");
}

#[test]
fn its_drawer_names_its_key_trunk_checkout_branch_and_remote() {
    let rows = rows(&record(vec![core()]), Some(Path::new("/home/j")), &words());
    let drawer = &rows[0]["drawer"];
    assert_eq!(drawer["title"], "core.nix");
    assert_eq!(
        drawer["eyebrow"],
        json!([{"said": "Jitsusama", "keep": 2}, {"said": "repository", "keep": 1}])
    );
    assert_eq!(
        drawer["facts"],
        json!([
            {"label": "Key", "value": "github:Jitsusama/core.nix"},
            {"label": "Trunk", "value": "main"},
            {"label": "Checkout", "value": "~/src/core.nix"},
            {"label": "Branch", "value": "main"},
        ])
    );
    assert_eq!(
        drawer["nerd"],
        json!([{"label": "Remote", "value": "git@github.com:Jitsusama/core.nix.git"}])
    );
}

#[test]
fn only_github_and_gitlab_have_pages() {
    assert_eq!(
        page("gitlab:group/sub/thing").as_deref(),
        Some("https://gitlab.com/group/sub/thing")
    );
    assert_eq!(page("git.example.org:thing"), None);
    assert_eq!(page("local:~/x"), None);
}

#[test]
fn a_repository_with_a_checkout_steps_into_its_files_by_name() {
    let mut gone = core();
    gone.key = "github:someone/gone".to_owned();
    gone.checkouts.clear();
    let inside = inside(&record(vec![core(), gone]), &words());
    assert_eq!(
        inside[..1],
        [json!({
            "key": "row:code:github:Jitsusama/core.nix:files",
            "thing": "repository:github:Jitsusama/core.nix:files",
            "place": "code",
            "kind": "act",
            "parent": "row:code:github:Jitsusama/core.nix",
            "depth": 1,
            "name": "Open a File by Name",
            "glyph": "folder",
            "hue": "action",
            "side": [],
            "held": {"path": "/home/j/src/core.nix"},
            "acts": [{"key": "verb:more", "step": "rows"}],
        })],
        "a repository with no checkout had files to find, or one with one didn't"
    );
    assert_eq!(
        inside.len(),
        2,
        "a repository with no checkout had rows inside"
    );
}

#[test]
fn a_checkout_with_files_steps_into_its_branches_after_its_files() {
    let rows = inside(&record(vec![core()]), &words());
    assert_eq!(
        rows.get(1),
        Some(&json!({
            "key": "row:code:github:Jitsusama/core.nix:branches",
            "thing": "repository:github:Jitsusama/core.nix:branches",
            "place": "code",
            "kind": "act",
            "parent": "row:code:github:Jitsusama/core.nix",
            "depth": 1,
            "name": "Switch Branch",
            "glyph": "branch",
            "hue": "action",
            "side": [],
            "held": {"path": "/home/j/src/core.nix"},
            "acts": [{"key": "verb:more", "step": "rows"}],
        })),
        "a checkout with files had no branches to switch to"
    );
    let mut bare = core();
    bare.checkouts = vec![checkout("/home/j/src/core.nix.git", true, Vec::new())];
    assert_eq!(
        inside(&record(vec![bare]), &words()).len(),
        1,
        "a bare checkout, with no files to switch, had branches to switch to"
    );
}

#[test]
fn a_branch_is_a_row_that_switches_to_it_saying_when_it_was_written() {
    let parent = "row:code:github:Jitsusama/core.nix:branches";
    let listed = [
        json!({"name": "main", "at_s": 2, "here": true, "elsewhere": null, "path": "/c"}),
        json!({"name": "fix", "at_s": 1, "here": false, "elsewhere": "~/t/fix", "path": "/c"}),
        json!({"name": "old", "at_s": null, "here": false, "elsewhere": null, "path": "/c"}),
    ];
    let rows = branches(parent, &listed, &words());
    assert_eq!(
        rows[2],
        json!({
            "key": "row:code:github:Jitsusama/core.nix:branches:old",
            "thing": "branch:github:Jitsusama/core.nix:old",
            "place": "code",
            "kind": "branch",
            "parent": parent,
            "depth": 2,
            "name": "old",
            "glyph": "branch",
            "hue": "program",
            "side": [],
            "held": {"path": "/c", "branch": "old"},
            "acts": [{
                "key": "verb:code:switch",
                "ask": {"ask": "code:switch", "path": "/c", "branch": "old"},
                "streams": true,
            }],
        }),
        "a branch's row doesn't name it or switch to it"
    );
    assert_eq!(
        (&rows[0]["side"], &rows[0]["acts"]),
        (
            &json!([{"said": "checked out"}, {"at": 2000, "as": "came", "keep": 1}]),
            &json!([])
        ),
        "the branch it's on didn't say so, or offered to switch to itself"
    );
    assert_eq!(
        (&rows[1]["side"], &rows[1]["acts"][0]["key"]),
        (
            &json!([{"said": "in ~/t/fix"}, {"at": 1000, "as": "came", "keep": 1}]),
            &json!("verb:code:switch")
        ),
        "a branch another tree holds didn't name it, or had no act to say why it can't"
    );
    assert_eq!(rows.len(), 3, "each branch listed isn't a row");
}

#[test]
fn a_file_is_a_row_named_by_its_name_that_opens_it() {
    let parent = "row:code:github:Jitsusama/core.nix:files";
    let listed = [
        json!({"path": "/src/core.nix/modules/a.nix", "at": 2000, "within": "modules"}),
        json!({"path": "/src/core.nix/flake.nix", "at": 1000, "within": ""}),
    ];
    let rows = files(parent, &listed, &words());
    assert_eq!(
        rows[0],
        json!({
            "key": "row:code:github:Jitsusama/core.nix:files:/src/core.nix/modules/a.nix",
            "thing": "file:/src/core.nix/modules/a.nix",
            "place": "code",
            "kind": "file",
            "parent": parent,
            "depth": 2,
            "name": "a.nix",
            "glyph": "path",
            "hue": "program",
            "side": [{"at": 2000, "as": "came"}, {"said": "modules", "keep": 1}],
            "keywords": "modules",
            "held": {"path": "/src/core.nix/modules/a.nix"},
            "acts": [{"key": "verb:code:file:open", "open": "/src/core.nix/modules/a.nix"}],
        }),
        "a file's row doesn't name it, say where it is or open it"
    );
    assert_eq!(
        rows[1]["side"],
        json!([{"at": 1000, "as": "came"}]),
        "a file at the top of its repository said a folder"
    );
    assert_eq!(rows.len(), 2, "each file listed isn't a row");
}

#[test]
fn one_tree_is_said_alone() {
    let mut repository = core();
    repository.checkouts.truncate(1);
    repository.checkouts[0].trees.truncate(1);
    let rows = rows(&record(vec![repository]), None, &words());
    assert_eq!(rows[0]["side"][0], json!({"said": "1 tree"}));
}
