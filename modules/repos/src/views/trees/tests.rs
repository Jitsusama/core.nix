//! The trees of the views' own record (views/tests.rs), at every width:
//! said under their shared folder, by where each is at, a name shown only
//! where it says something else, and the gone ones last.

use super::*;
use crate::views::repository;
use crate::views::tests::{HELD, ROOMS, clock, home, record, tree};

#[test]
fn trees_say_their_shared_folder_once_and_where_each_is_at() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        for text in [
            repository(&record, core, &clock(), Some(home()), &Pen::plain(), room),
            trees(core, &clock(), Some(home()), &Pen::plain(), room),
        ] {
            assert_eq!(layout::overrun(&text, room), None);
            assert!(text.contains(&format!("trees under ~/{HELD}\n")), "{text}");
            assert!(!text.contains("worktree-github-"), "{text}");
            assert!(
                text.lines()
                    .any(|line| line.starts_with("jitsusama/change-0 ")),
                "{text}"
            );
            assert!(
                !text
                    .lines()
                    .any(|line| line.starts_with("jitsusama/change-0 ")
                        && line.contains("jitsusama-change-0")),
                "a tree named for its branch said it twice: {text}"
            );
            assert!(
                text.lines()
                    .any(|line| line.starts_with("0123456 ") && !line.ends_with("0123456")),
                "a snapshot named for its commit said it twice: {text}"
            );
            assert!(
                !text.split_whitespace().any(|word| word == "TREE"),
                "no tree's name says more, yet it has a column: {text}"
            );
            assert!(!text.contains('\u{2026}'), "a tree was cut: {text}");
        }
    }
}

#[test]
fn a_name_that_says_something_else_is_shown_beside_where_it_is_at() {
    let mut record = record();
    let trees_of_core = &mut record.repositories[0].checkouts[0].trees;
    trees_of_core.push(tree(
        &format!("{HELD}/worktree-github-Jitsusama-core.nix-fix-410"),
        Some("jitsusama/net-hands-over"),
        true,
        1,
    ));
    let mut pinned = tree(&format!("{HELD}/snapshot-core.nix-review"), None, true, 2);
    pinned.head.branch = Some("jitsusama/reviewed".to_owned());
    trees_of_core.push(pinned);
    let text = trees(
        &record.repositories[0],
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(80),
    );
    assert!(text.contains("TREE"), "{text}");
    let said = |branch: &str| {
        text.lines()
            .find(|line| line.starts_with(branch))
            .unwrap_or_else(|| panic!("{branch} has no line: {text}"))
    };
    assert!(
        said("jitsusama/net-hands-over").ends_with("  fix-410"),
        "{text}"
    );
    assert!(
        said("jitsusama/reviewed").ends_with("  review · snapshot"),
        "a snapshot at a branch didn't say it's one: {text}"
    );
    assert!(
        !said("jitsusama/change-1").contains("snapshot"),
        "a worktree said its kind: {text}"
    );
}

#[test]
fn a_name_restates_its_branch_or_commit_give_or_take_a_word() {
    let at = |branch: Option<&str>| Head {
        branch: branch.map(str::to_owned),
        commit: Some("0123456789abcdef".to_owned()),
    };
    for (name, branch) in [
        ("jitsusama-change-0", Some("jitsusama/change-0")),
        (
            "repos-github-changes",
            Some("jitsusama/repos-github-changes"),
        ),
        ("0123456", None),
    ] {
        assert!(
            restates(name, &at(branch)),
            "{name} didn't restate {branch:?}"
        );
    }
    for (name, branch) in [
        ("fix-410", Some("jitsusama/net-hands-over")),
        ("review", None),
    ] {
        assert!(!restates(name, &at(branch)), "{name} restated {branch:?}");
    }
    assert!(
        !restates(
            "core",
            &at(Some("jitsusama/fix-core-boot-and-the-disk-prompt"))
        ),
        "a word the branch holds among many restated it"
    );
}

#[test]
fn a_tree_cut_somewhere_else_is_said_whole_beside_the_rest() {
    let mut record = record();
    let mut elsewhere = tree("x", Some("jitsusama/elsewhere"), true, 1);
    elsewhere.path = "/tmp/pr2364".to_owned();
    record.repositories[0].checkouts[0].trees.push(elsewhere);
    let text = trees(
        &record.repositories[0],
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Cells(100),
    );
    assert!(text.contains(&format!("trees under ~/{HELD}\n")), "{text}");
    assert!(
        text.lines()
            .any(|line| line.starts_with("jitsusama/elsewhere ") && line.ends_with("/tmp/pr2364")),
        "{text}"
    );
}

#[test]
fn every_tree_is_listed_and_a_gone_one_goes_last_saying_so() {
    let record = record();
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = trees(core, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("jitsusama/change-"))
                .count(),
            7,
            "{text}"
        );
        assert_eq!(
            text.lines().last(),
            Some("jitsusama-change-7 · gone from the disk"),
            "{text}"
        );
    }
}
