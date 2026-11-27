//! A class whose owner differs by repository: which repositories are
//! asked of apart, which only with the rest, and that each line's key and
//! owner hold whatever the counts do.

use super::*;

/// Each line `cases` gives, as its key's class and its owner.
fn keys(cases: &[Case]) -> Vec<(String, String)> {
    recurring(cases, NOW)
        .behind
        .iter()
        .filter_map(|line| line.rsplit_once(" · "))
        .map(|(line, owner)| (class_of(line).to_owned(), owner.to_owned()))
        .collect()
}

/// The cases of `TEST_PASSES_UNDONE` found on `changes`.
fn undone(changes: &[&str]) -> Vec<Case> {
    changes
        .iter()
        .map(|change| case(TEST_PASSES_UNDONE, change, 100))
        .collect()
}

#[test]
fn a_class_owned_apart_by_repository_goes_to_each_repositorys_owner() {
    let mut cases = undone(&[
        "core.nix#1",
        "core.nix#2",
        "core.nix#8",
        "agentic-harness.pi#3",
        "agentic-harness.pi#9",
        "agentic-harness.pi#10",
        "mycelium#4",
        "mycelium#12",
        "world#11",
    ]);
    cases.extend(
        ["core.nix#5", "core.nix#6", "mycelium#7"]
            .map(|change| case("a pinned copy lags the tree", change, 100)),
    );
    let m = recurring(&cases, NOW);
    assert_eq!(
        m.now, 2.0,
        "a class split by repository counted more than once"
    );
    let half = |repo: &str| format!("{TEST_PASSES_UNDONE} in {repo}");
    assert_eq!(
        keys(&cases),
        [
            (half("agentic-harness.pi"), "85PKB2".to_owned()),
            (half("core.nix"), "80ECO2".to_owned()),
            (TEST_PASSES_UNDONE.to_owned(), "1JQPPC".to_owned()),
            (PIN_LAGS.to_owned(), "16V6F4".to_owned()),
        ],
        "a class wasn't split to each repository's owner with the rest under the class's \
         own key, or one owned whole was split"
    );
    assert!(
        m.behind[1]
            .contains("on 3 of its 9 changes in a week, core.nix#1, core.nix#2, core.nix#8 ·"),
        "a repository's part didn't name its own changes: {}",
        m.behind[1]
    );
    assert!(
        m.behind[2].contains(
            "on 3 of its 9 changes in a week in other repositories, mycelium#12, mycelium#4, \
             world#11 ·"
        ),
        "the repositories no owner is named in weren't named together: {}",
        m.behind[2]
    );
}

/// A count crossing the bar used to move a repository's cases between
/// keys, closing one owner's ask, opening another's, and asking again for
/// a fix just named. Each line now keeps its key and owner, and the rest's
/// line is keyed as the whole class's is.
#[test]
fn every_line_keeps_its_key_and_owner_as_its_counts_move() {
    let three = keys(&undone(&[
        "core.nix#1",
        "core.nix#2",
        "core.nix#3",
        "world#4",
        "world#5",
        "world#6",
    ]));
    let two = keys(&undone(&[
        "core.nix#1",
        "core.nix#2",
        "world#4",
        "world#5",
        "world#6",
    ]));
    assert_eq!(
        three, two,
        "an owned repository's line changed key or owner as it fell under the bar"
    );
    let whole = keys(&undone(&["world#4", "world#5", "world#6"]));
    assert_eq!(
        whole,
        [three[1].clone()],
        "the rest's line and the class's whole line were keyed apart: {three:?}"
    );
}

/// On 27 November the keeper was asked about world on 1 of a class's 55
/// changes. A repository no owner is named in now counts only toward the
/// rest's bar, and three such repositories on two changes each still
/// reach the class's owner.
#[test]
fn a_repository_no_owner_is_named_in_is_asked_of_only_with_the_rest_over_the_bar() {
    let one = recurring(
        &undone(&[
            "core.nix#1",
            "core.nix#2",
            "core.nix#3",
            "core.nix#4",
            "world#11",
        ]),
        NOW,
    );
    assert_eq!(
        one.behind,
        [format!(
            "{TEST_PASSES_UNDONE} in core.nix: on 4 of its 5 changes in a week, core.nix#1, \
             core.nix#2, core.nix#3, core.nix#4 · 80ECO2"
        )],
        "a lone change in a repository no owner is named in was asked of"
    );
    let owned_under = keys(&undone(&[
        "core.nix#1",
        "core.nix#2",
        "agentic-harness.pi#3",
        "agentic-harness.pi#4",
        "world#5",
        "world#6",
    ]));
    assert_eq!(
        owned_under,
        [
            (
                format!("{TEST_PASSES_UNDONE} in agentic-harness.pi"),
                "85PKB2".to_owned()
            ),
            (
                format!("{TEST_PASSES_UNDONE} in core.nix"),
                "80ECO2".to_owned()
            ),
        ],
        "an owned repository under the bar wasn't asked of its own owner, or the rest under \
         it was asked of"
    );
    let unowned = recurring(
        &undone(&[
            "mycelium#1",
            "mycelium#2",
            "world#3",
            "world#4",
            "other#5",
            "other#6",
        ]),
        NOW,
    );
    assert_eq!(
        unowned.behind,
        [format!(
            "{TEST_PASSES_UNDONE}: on 6 changes in a week, mycelium#1, mycelium#2, other#5, \
             other#6, world#3, world#4 · 1JQPPC"
        )],
        "three repositories no owner is named in, on two each, didn't reach the class's owner"
    );
}

/// A repository no owner is named in is asked of with the rest, however
/// many changes it is on, so a line naming many unknown repositories,
/// three changes each, makes one line and one ask rather than one for
/// each of them.
#[test]
fn repositories_with_no_owner_of_their_own_are_asked_of_in_one_line() {
    let unknown = NAMED_AT_MOST + 2;
    let mut cases: Vec<Case> = (0..unknown)
        .flat_map(|n| {
            (1..=3).map(move |c| case(TEST_PASSES_UNDONE, &format!("repo{n:02}#{c}"), 100))
        })
        .collect();
    cases.extend((1..=3).map(|c| case(TEST_PASSES_UNDONE, &format!("core.nix#{c}"), 100)));
    let m = recurring(&cases, NOW);
    let named: Vec<&str> = m.behind.iter().map(|l| class_of(l)).collect();
    assert_eq!(
        named,
        [
            format!("{TEST_PASSES_UNDONE} in core.nix"),
            TEST_PASSES_UNDONE.to_owned()
        ],
        "a repository with no owner of its own was asked of apart: {:?}",
        m.behind
    );
    let rest = format!(
        "on {} of its {} changes in a week in other repositories",
        unknown * 3,
        unknown * 3 + 3
    );
    assert!(
        m.behind[1].contains(&rest) && m.behind[1].ends_with(" · 1JQPPC"),
        "the other line didn't count every unknown repository's changes for the keeper: {}",
        m.behind[1]
    );
}
