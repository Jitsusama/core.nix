//! The views over a record made by hand, at every width a terminal is
//! likely to be and into a pipe, on a clock in UTC so every moment reads
//! the same wherever the tests run.

use super::*;
use repos::model::{Checkout, Checks, GitHubChange, Head, Remote, SCHEMA, Source, TreeKind};

/// 2033-05-18 03:33:20 UTC.
pub(super) const NOW: u64 = 2_000_000_000;
const HOME: &str = "/home/someone";
pub(super) const HELD: &str = ".local/state/pi/agentic-harness.pi/work/trees";

pub(super) const ROOMS: [Room; 4] = [
    Room::Cells(80),
    Room::Cells(100),
    Room::Cells(200),
    Room::Unbounded,
];

pub(super) fn home() -> &'static Path {
    Path::new(HOME)
}

pub(super) fn clock() -> Clock {
    Clock {
        now: NOW,
        local: repos::when::utc,
    }
}

pub(super) fn tree(path: &str, branch: Option<&str>, present: bool, ago: u64) -> Tree {
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
            upstream: None,
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
        read_through: None,
    }
}

fn checkout(path: &str, trees: Vec<Tree>, ago: u64) -> Checkout {
    Checkout {
        path: format!("{HOME}/{path}"),
        bare: false,
        head: Some(Head {
            branch: Some("main".to_owned()),
            commit: None,
            upstream: None,
        }),
        written_at: Some(NOW - ago),
        trees,
        weight: None,
    }
}

pub(super) fn record() -> Record {
    let core_trees = (0..8)
        .map(|at| {
            tree(
                &format!("{HELD}/worktree-github-Jitsusama-core.nix-jitsusama-change-{at}"),
                Some(&format!("jitsusama/change-{at}")),
                at != 7,
                60 * (at + 1),
            )
        })
        .chain([tree(
            &format!("{HELD}/snapshot-core.nix-0123456"),
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
                "local:~/src/backwater",
                None,
                vec![checkout("src/backwater", Vec::new(), 172_800)],
                172_800,
            ),
        ],
    }
}

fn change(number: u64, branch: &str, state: ChecksState, review: Review, ago: u64) -> Change {
    let failed = match state {
        ChecksState::Failing => vec!["build".to_owned()],
        _ => Vec::new(),
    };
    let pending = match state {
        ChecksState::Pending => vec!["lint".to_owned()],
        _ => Vec::new(),
    };
    Change {
        title: format!("Change number {number}, which says what it does at some length"),
        author: Some("someone".to_owned()),
        draft: false,
        branch: branch.to_owned(),
        base: "main".to_owned(),
        commit: None,
        updated_at: NOW - ago,
        checks: Checks {
            state,
            passed: 3,
            failed,
            pending,
        },
        review,
        github: Some(GitHubChange {
            number,
            url: format!("https://github.com/Jitsusama/core.nix/pull/{number}"),
        }),
        review_asked_of: Vec::new(),
    }
}

/// The record after a refresh: seven changes open on core.nix, the one
/// updated last first, one at a tree's branch; and one repository GitHub
/// couldn't read.
fn refreshed() -> Record {
    let mut record = record();
    record.sources.push(Source {
        name: "github".to_owned(),
        what: "each repository's open changes and their checks".to_owned(),
        read_at: Some(NOW - 120),
        unread: vec![
            "github:someone-else/walgit-rs: not readable here: GitHub's App can't see it, or \
             it's gone"
                .to_owned(),
        ],
    });
    let mut draft = change(
        4790,
        "jitsusama/drafted",
        ChecksState::Pending,
        Review::Waiting,
        90,
    );
    draft.draft = true;
    record.repositories[0].changes = vec![
        change(
            4791,
            "jitsusama/change-0",
            ChecksState::Failing,
            Review::ChangesAsked,
            10,
        ),
        draft,
        change(
            4789,
            "jitsusama/other",
            ChecksState::Pending,
            Review::Waiting,
            600,
        ),
        change(
            4788,
            "jitsusama/approved",
            ChecksState::Passing,
            Review::Approved,
            3_600,
        ),
        change(
            4787,
            "jitsusama/quiet",
            ChecksState::None,
            Review::None,
            7_200,
        ),
        change(
            4786,
            "jitsusama/older",
            ChecksState::Passing,
            Review::None,
            86_400,
        ),
        change(
            4785,
            "jitsusama/oldest",
            ChecksState::Passing,
            Review::None,
            864_000,
        ),
    ];
    record
}

/// The table's rows under the header that opens with `header`, each as
/// its first word, up to the blank line after it.
fn firsts<'a>(text: &'a str, header: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|line| !line.starts_with(header))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect()
}

#[test]
fn the_collection_is_a_line_for_each_repository_under_its_labels() {
    let record = record();
    for room in ROOMS {
        let text = repositories(&record, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.starts_with("repositories · 4 on laptop · under ~/src\n\n"),
            "a line about GitHub came before any refresh: {text}"
        );
        assert!(text.contains("REPOSITORY"), "{text}");
        assert!(
            !text.contains("CHANGES"),
            "nothing filled the column: {text}"
        );
        assert!(text.contains("core.nix  "), "{text}");
        assert!(text.contains("repos NAME trees"), "{text}");
        assert!(text.contains("repos NAME changes"), "{text}");
    }
    let text = repositories(
        &record,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    let line = text
        .lines()
        .find(|line| line.starts_with("core.nix"))
        .expect("core.nix has a line");
    let cells: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(
        cells,
        ["core.nix", "8", "2", "03:32", "github:Jitsusama/core.nix"],
        "a gone tree counts, or the columns moved: {text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("backwater") && line.contains(" Mon ")),
        "two days ago isn't its weekday: {text}"
    );
}

#[test]
fn the_collection_counts_each_ones_changes_and_says_when_github_was_asked() {
    let record = refreshed();
    for room in ROOMS {
        let text = repositories(&record, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.contains("\nchanges from GitHub at 03:31 · 1 it couldn't read\n"),
            "{text}"
        );
        assert!(text.contains("CHANGES"), "{text}");
        assert!(text.contains("7 · 1 failing"), "{text}");
    }
    let text = repositories(
        &record,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    let cell = |name: &str| {
        text.lines()
            .find(|line| line.starts_with(name))
            .unwrap_or_else(|| panic!("{name} has no line: {text}"))
            .to_owned()
    };
    assert!(
        cell("someone-else/walgit-rs").contains(" not read "),
        "{text}"
    );
    assert!(
        !cell("Jitsusama/walgit-rs").contains("not read"),
        "a repository with none open was said unread: {text}"
    );
}

#[test]
fn a_name_two_repositories_share_is_said_with_its_owner() {
    let text = repositories(
        &record(),
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    assert!(text.contains("Jitsusama/walgit-rs "), "{text}");
    assert!(text.contains("someone-else/walgit-rs "), "{text}");
    assert!(
        text.lines().any(|line| line.starts_with("core.nix ")),
        "a name nobody shares took its owner: {text}"
    );
}

#[test]
fn a_narrow_terminal_drops_the_key_before_the_checkouts() {
    let text = repositories(
        &record(),
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(50),
    );
    assert_eq!(layout::overrun(&text, Room::Cells(50)), None);
    assert!(!text.contains("KEY"), "{text}");
    assert!(text.contains("CHECKOUTS"), "{text}");
    let text = repositories(
        &refreshed(),
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(50),
    );
    assert_eq!(layout::overrun(&text, Room::Cells(50)), None);
    assert!(
        text.contains("CHANGES") && text.contains("7 · 1 failing"),
        "the changes gave way before the checkouts: {text}"
    );
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
        let text = repositories(&record, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(!text.contains('\u{2026}'), "a cell was cut: {text}");
        assert_eq!(
            text.contains("KEY"),
            text.contains(long),
            "the keys are shown only in part: {text}"
        );
    }
    let text = repositories(
        &record,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(!text.contains("KEY"), "{text}");
    assert!(text.contains("CHECKOUTS"), "{text}");
}

#[test]
fn no_checkout_under_the_roots_says_so() {
    let mut record = record();
    record.repositories.clear();
    let text = repositories(
        &record,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(text.contains("no checkout is there"), "{text}");
}

#[test]
fn a_repository_shows_its_facts_checkouts_and_newest_trees() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = repository(&record, core, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.starts_with("core.nix · github:Jitsusama/core.nix\n"),
            "{text}"
        );
        assert!(text.contains("trunk"), "{text}");
        assert!(
            text.lines()
                .any(|line| line.starts_with("written ") && line.ends_with(" at 03:32")),
            "{text}"
        );
        assert!(text.contains("checkouts  2"), "{text}");
        assert!(
            !text.contains("changes"),
            "a change was said before any refresh: {text}"
        );
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
    let text = repository(
        &record,
        core,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        firsts(&text, "AT"),
        [
            "0123456",
            "jitsusama/change-0",
            "jitsusama/change-1",
            "jitsusama/change-2",
            "jitsusama/change-3"
        ],
        "the newest five aren't first: {text}"
    );
}

#[test]
fn a_repository_shows_how_its_changes_stand_and_the_newest() {
    let record = refreshed();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = repository(&record, core, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.contains("7 open · 1 failing · 2 pending · 1 approved"),
            "{text}"
        );
        assert!(text.contains("2 more: repos core.nix changes"), "{text}");
    }
    let text = repository(
        &record,
        core,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        firsts(&text, "CHANGE"),
        ["#4791", "#4790", "#4789", "#4788", "#4787"],
        "{text}"
    );
    let failing = text
        .lines()
        .find(|line| line.starts_with("#4791"))
        .expect("#4791 has a line");
    assert!(
        failing.contains("1 failing") && failing.contains("changes asked"),
        "{failing}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("#4790") && line.contains("draft")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("jitsusama/change-0") && line.contains("#4791")),
        "the tree at a change's branch didn't name it: {text}"
    );
    let unread = repository(
        &record,
        &record.repositories[2],
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(
        unread.contains("GitHub's App can't see it, or it's gone"),
        "{unread}"
    );
}

#[test]
fn every_change_is_listed_the_one_updated_last_first() {
    let record = refreshed();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = changes(&record, core, &clock(), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert!(
            text.starts_with("core.nix · github:Jitsusama/core.nix\n"),
            "{text}"
        );
        assert_eq!(firsts(&text, "CHANGE").len(), 7, "{text}");
    }
    let text = changes(&record, core, &clock(), &Pen::plain(), Room::Unbounded);
    for (number, when) in [
        ("#4791", "03:33"),
        ("#4786", "yesterday 03:33"),
        ("#4785", "8 May"),
    ] {
        assert!(
            text.lines()
                .any(|line| line.starts_with(number) && line.contains(when)),
            "{number} wasn't updated {when}: {text}"
        );
    }
    let quiet = changes(
        &record,
        &record.repositories[1],
        &clock(),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(quiet.contains("none open"), "{quiet}");
    let unread = changes(
        &record,
        &record.repositories[2],
        &clock(),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(unread.contains("GitHub's App can't see it"), "{unread}");
    let unasked = self::record();
    let unasked = changes(
        &unasked,
        &unasked.repositories[0],
        &clock(),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(unasked.contains("repos refresh"), "{unasked}");
}

#[test]
fn changes_read_through_gh_s_sign_in_say_so_where_counted() {
    let mut record = refreshed();
    record.repositories[0].read_through = Some(Through::Gh);
    record.repositories[1].read_through = Some(Through::App);
    let core = &record.repositories[0];
    let listed = changes(&record, core, &clock(), &Pen::plain(), Room::Cells(80));
    let page = repository(
        &record,
        core,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    for text in [&listed, &page] {
        assert!(
            text.contains("7 open · 1 failing · 2 pending · 1 approved · through gh's sign-in"),
            "it didn't say gh read them: {text}"
        );
    }
    let quiet = &record.repositories[1];
    let quiet = changes(&record, quiet, &clock(), &Pen::plain(), Room::Cells(80));
    assert!(
        quiet.contains("none open") && !quiet.contains("through"),
        "the App's way was said: {quiet}"
    );
}

#[test]
fn a_repository_with_no_remote_says_it_is_only_here() {
    let record = record();
    let text = repository(
        &record,
        &record.repositories[3],
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(text.contains("none, it's only on this machine"), "{text}");
}

#[test]
fn a_column_no_row_fills_takes_no_room() {
    let columns = [("a", Align::Left), ("b", Align::Left), ("c", Align::Left)];
    let rows = vec![vec!["1".to_owned(), String::new(), "3".to_owned()]];
    let (columns, rows, gives_way) = filled(&columns, rows, &[1, 2]);
    assert_eq!(columns, [("a", Align::Left), ("c", Align::Left)]);
    assert_eq!(rows, [["1", "3"]]);
    assert_eq!(gives_way, [1]);
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
fn a_nearest_name_is_suggested_so_pasting_it_runs_nothing_else() {
    let mut record = record();
    record.repositories[0].name = "core;reboot".to_owned();
    let refused = find(&record, "core;rebot").expect_err("nothing is core;rebot");
    assert_eq!(
        refused.then.as_deref(),
        Some("repos 'core;reboot'"),
        "{refused:?}"
    );
}

#[test]
fn every_time_an_answer_gives_is_rfc_3339() {
    let record = refreshed();
    let core = &record.repositories[0];
    for answer in [
        repositories_json(&record),
        repository_json(&record, core),
        trees_json(&record, core),
    ] {
        let text = answer.to_string();
        assert!(!text.contains("\"written_at\":2"), "{text}");
        assert!(!text.contains("\"updated_at\":2"), "{text}");
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
    let at_change = trees["trees"]
        .as_array()
        .and_then(|trees| {
            trees
                .iter()
                .find(|tree| tree["head"]["branch"] == "jitsusama/change-0")
        })
        .expect("change-0's tree is answered");
    assert_eq!(at_change["change"]["number"], 4791);
}

#[test]
fn the_changes_answer_says_when_github_was_asked_and_what_it_couldnt_read() {
    let record = refreshed();
    let answer = changes_json(&record, &record.repositories[0]);
    assert_eq!(answer["schema"], "repos.changes/1");
    assert_eq!(answer["read_at"], "2033-05-18T03:31:20Z");
    assert_eq!(answer["key"], "github:Jitsusama/core.nix");
    assert_eq!(answer["unread"], Value::Null);
    assert_eq!(answer["changes"].as_array().map(Vec::len), Some(7));
    assert_eq!(answer["changes"][0]["updated_at"], "2033-05-18T03:33:10Z");
    let unread = changes_json(&record, &record.repositories[2]);
    assert_eq!(
        unread["unread"],
        "not readable here: GitHub's App can't see it, or it's gone"
    );
}
