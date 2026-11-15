//! The views over a record made by hand, at every width a terminal is
//! likely to be and into a pipe.

use super::*;
use repos::model::{Checkout, Head, Remote, SCHEMA, Source};

const NOW: u64 = 2_000_000_000;
const HOME: &str = "/home/someone";

const ROOMS: [Room; 4] = [
    Room::Cells(80),
    Room::Cells(100),
    Room::Cells(200),
    Room::Unbounded,
];

fn home() -> &'static Path {
    Path::new(HOME)
}

fn tree(path: &str, branch: Option<&str>, present: bool, ago: u64) -> Tree {
    Tree {
        path: format!("{HOME}/{path}"),
        kind: if branch.is_some() {
            TreeKind::Worktree
        } else {
            TreeKind::Snapshot
        },
        head: Head {
            branch: branch.map(str::to_owned),
            commit: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
        },
        present,
        written_at: Some(NOW - ago),
    }
}

fn made(key: &str, owner: Option<&str>, checkouts: Vec<Checkout>, ago: u64) -> Repository {
    let name = key.rsplit(['/', ':']).next().unwrap_or(key).to_owned();
    Repository {
        key: key.to_owned(),
        name,
        owner: owner.map(str::to_owned),
        remotes: owner
            .map(|owner| Remote {
                name: "origin".to_owned(),
                url: format!(
                    "git@github.com:{owner}/{}.git",
                    key.rsplit('/').next().unwrap()
                ),
            })
            .into_iter()
            .collect(),
        trunk: Some("main".to_owned()),
        written_at: Some(NOW - ago),
        checkouts,
        changes: Vec::new(),
    }
}

fn checkout(path: &str, trees: Vec<Tree>, ago: u64) -> Checkout {
    Checkout {
        path: format!("{HOME}/{path}"),
        bare: false,
        head: Some(Head {
            branch: Some("main".to_owned()),
            commit: None,
        }),
        written_at: Some(NOW - ago),
        trees,
    }
}

fn record() -> Record {
    let held = ".local/state/pi/agentic-harness.pi/work/trees";
    let core_trees = (0..8)
        .map(|at| {
            tree(
                &format!("{held}/worktree-github-Jitsusama-core.nix-jitsusama-change-{at}"),
                Some(&format!("jitsusama/change-{at}")),
                at != 7,
                60 * (at + 1),
            )
        })
        .chain([tree(
            &format!("{held}/snapshot-core.nix-0123456"),
            None,
            true,
            30,
        )])
        .collect();
    Record {
        schema: SCHEMA.to_owned(),
        read_at: NOW,
        machine: "laptop".to_owned(),
        roots: vec![format!("{HOME}/src")],
        sources: vec![Source {
            name: "git".to_owned(),
            what: "each checkout's and tree's own git files".to_owned(),
            read_at: None,
            unread: Vec::new(),
        }],
        repositories: vec![
            made(
                "github:Jitsusama/core.nix",
                Some("Jitsusama"),
                vec![
                    checkout("src/core.nix", core_trees, 30),
                    checkout("src/github.com/Jitsusama/core.nix", Vec::new(), 86_400),
                ],
                30,
            ),
            made(
                "github:Jitsusama/walgit-rs",
                Some("Jitsusama"),
                vec![checkout("src/walgit-rs", Vec::new(), 3_600)],
                3_600,
            ),
            made(
                "github:someone-else/walgit-rs",
                Some("someone-else"),
                vec![checkout(
                    "src/github.com/someone-else/walgit-rs",
                    Vec::new(),
                    7_200,
                )],
                7_200,
            ),
            made(
                "local:~/src/acceluary",
                None,
                vec![checkout("src/acceluary", Vec::new(), 172_800)],
                172_800,
            ),
        ],
    }
}

#[test]
fn the_collection_is_a_line_for_each_repository_under_its_labels() {
    let record = record();
    for room in ROOMS {
        let text = repositories(&record, NOW, Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.starts_with("repositories · 4 on laptop · under ~/src\n"),
            "{text}"
        );
        assert!(text.contains("REPOSITORY"), "{text}");
        assert!(text.contains("core.nix  "), "{text}");
        assert!(text.contains("repos NAME trees"), "{text}");
    }
    let text = repositories(&record, NOW, Some(home()), &Pen::plain(), Room::Unbounded);
    let line = text
        .lines()
        .find(|line| line.starts_with("core.nix"))
        .expect("core.nix has a line");
    let cells: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(
        cells,
        ["core.nix", "8", "2", "30", "s", "github:Jitsusama/core.nix"],
        "a gone tree counts, or the columns moved: {text}"
    );
}

#[test]
fn a_name_two_repositories_share_is_said_with_its_owner() {
    let text = repositories(&record(), NOW, Some(home()), &Pen::plain(), Room::Unbounded);
    assert!(text.contains("Jitsusama/walgit-rs "), "{text}");
    assert!(text.contains("someone-else/walgit-rs "), "{text}");
    assert!(
        text.lines().any(|line| line.starts_with("core.nix ")),
        "a name nobody shares took its owner: {text}"
    );
}

#[test]
fn a_narrow_terminal_drops_the_key_before_the_checkouts() {
    let text = repositories(&record(), NOW, Some(home()), &Pen::plain(), Room::Cells(50));
    assert_eq!(layout::overrun(&text, Room::Cells(50)), None);
    assert!(!text.contains("KEY"), "{text}");
    assert!(text.contains("CHECKOUTS"), "{text}");
}

#[test]
fn a_key_goes_whole_and_is_never_cut() {
    let mut record = record();
    let long = "github:an-owner-with-a-long-name/a-repository-with-a-long-name";
    record.repositories.push(made(
        long,
        Some("an-owner-with-a-long-name"),
        vec![checkout(
            "src/a-repository-with-a-long-name",
            Vec::new(),
            60,
        )],
        60,
    ));
    for room in ROOMS {
        let text = repositories(&record, NOW, Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(!text.contains('\u{2026}'), "a cell was cut: {text}");
        assert_eq!(
            text.contains("KEY"),
            text.contains(long),
            "the keys are shown only in part: {text}"
        );
    }
    let text = repositories(&record, NOW, Some(home()), &Pen::plain(), Room::Cells(80));
    assert!(!text.contains("KEY"), "{text}");
    assert!(text.contains("CHECKOUTS"), "{text}");
}

#[test]
fn trees_say_their_shared_folder_once_and_each_its_own_name() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        for text in [
            repository(core, NOW, Some(home()), &Pen::plain(), room),
            trees(core, NOW, Some(home()), &Pen::plain(), room),
        ] {
            assert_eq!(layout::overrun(&text, room), None);
            assert!(
                text.contains("trees under ~/.local/state/pi/agentic-harness.pi/work/trees\n"),
                "{text}"
            );
            assert!(!text.contains("worktree-github-"), "{text}");
            assert!(
                text.lines()
                    .any(|line| line.starts_with("jitsusama-change-0 ")),
                "{text}"
            );
            assert!(
                text.lines()
                    .any(|line| line.starts_with("0123456 ") && line.ends_with("  0123456")),
                "a snapshot's folder isn't said by its own name: {text}"
            );
            assert!(!text.contains('\u{2026}'), "a tree was cut: {text}");
        }
    }
}

#[test]
fn a_tree_cut_somewhere_else_is_said_whole_beside_the_rest() {
    let mut record = record();
    let mut elsewhere = tree("x", Some("jitsusama/elsewhere"), true, 1);
    elsewhere.path = "/tmp/pr2364".to_owned();
    record.repositories[0].checkouts[0].trees.push(elsewhere);
    let text = trees(
        &record.repositories[0],
        NOW,
        Some(home()),
        &Pen::plain(),
        Room::Cells(100),
    );
    assert!(
        text.contains("trees under ~/.local/state/pi/agentic-harness.pi/work/trees\n"),
        "{text}"
    );
    assert!(
        text.lines().any(|line| line.starts_with("/tmp/pr2364 ")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("jitsusama-change-0 ")),
        "{text}"
    );
}

#[test]
fn no_checkout_under_the_roots_says_so() {
    let mut record = record();
    record.repositories.clear();
    let text = repositories(&record, NOW, Some(home()), &Pen::plain(), Room::Cells(80));
    assert!(text.contains("no checkout is there"), "{text}");
}

#[test]
fn a_repository_shows_its_facts_checkouts_and_newest_trees() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = repository(core, NOW, Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.starts_with("core.nix · github:Jitsusama/core.nix\n"),
            "{text}"
        );
        assert!(text.contains("trunk"), "{text}");
        assert!(text.contains("checkouts  2"), "{text}");
        assert!(
            text.contains("7 worktrees · 1 snapshot · 1 gone from the disk"),
            "{text}"
        );
        assert!(text.contains("~/src/core.nix"), "{text}");
        assert!(
            text.contains("3 more: repos core.nix trees"),
            "the more isn't the rest of the trees on the disk: {text}"
        );
    }
    let text = repository(core, NOW, Some(home()), &Pen::plain(), Room::Unbounded);
    let trees: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("TREE"))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert_eq!(
        trees,
        [
            "0123456",
            "jitsusama-change-0",
            "jitsusama-change-1",
            "jitsusama-change-2",
            "jitsusama-change-3"
        ],
        "the newest five aren't first: {text}"
    );
}

#[test]
fn a_repository_with_no_remote_says_it_is_only_here() {
    let record = record();
    let text = repository(
        &record.repositories[3],
        NOW,
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(text.contains("none, it's only on this machine"), "{text}");
}

#[test]
fn every_tree_is_listed_and_a_gone_one_says_so() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = trees(core, NOW, Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("jitsusama-change-"))
                .count(),
            8,
            "{text}"
        );
    }
    let text = trees(core, NOW, Some(home()), &Pen::plain(), Room::Unbounded);
    let gone = text
        .lines()
        .find(|line| line.starts_with("jitsusama-change-7"))
        .expect("the gone tree has a line");
    assert!(gone.contains(" gone "), "{gone}");
}

#[test]
fn a_repository_is_found_by_its_name_owner_and_name_or_key() {
    let record = record();
    for word in [
        "core.nix",
        "CORE.NIX",
        "jitsusama/core.nix",
        "github:Jitsusama/core.nix",
    ] {
        let found = find(&record, word).expect(word);
        assert_eq!(found.key, "github:Jitsusama/core.nix", "{word}");
    }
}

#[test]
fn a_name_two_repositories_share_is_refused_with_their_keys() {
    let refused = find(&record(), "walgit-rs").expect_err("two go by walgit-rs");
    let said = format!("{refused:?}");
    assert!(said.contains("github:Jitsusama/walgit-rs"), "{said}");
    assert!(said.contains("github:someone-else/walgit-rs"), "{said}");
    assert!(said.contains("Usage"), "{said}");
}

#[test]
fn an_unknown_name_is_refused_with_the_nearest() {
    let refused = find(&record(), "core.nx").expect_err("nothing is core.nx");
    let said = format!("{refused:?}");
    assert!(said.contains("NotFound"), "{said}");
    assert!(said.contains("repos core.nix"), "{said}");
}

#[test]
fn every_time_an_answer_gives_is_rfc_3339() {
    let record = record();
    let core = &record.repositories[0];
    for answer in [
        repositories_json(&record),
        repository_json(&record, core),
        trees_json(&record, core),
    ] {
        let text = answer.to_string();
        assert!(!text.contains("\"written_at\":2"), "{text}");
        assert!(
            text.contains("\"read_at\":\"2033-05-18T03:33:20Z\""),
            "{text}"
        );
    }
    let trees = trees_json(&record, core);
    assert_eq!(trees["schema"], "repos.trees/1");
    assert_eq!(trees["trees"].as_array().map(Vec::len), Some(9));
    assert_eq!(
        trees["trees"][0]["checkout"],
        format!("{HOME}/src/core.nix")
    );
}
