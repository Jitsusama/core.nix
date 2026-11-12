use super::*;

/// A pytest file shaped as tests/launcher-on-niri-test.py was on 12
/// November: a docstring, imports, a constant, a fixture, a helper and
/// two cases, one of them with a docstring and one running over lines.
const BASE: &str = r#""""The launcher's cases, on niri.

Every test starts with the launcher closed.
"""

from __future__ import annotations

import pytest

READ = 2.0


@pytest.fixture(scope="module")
def shell():
    return start()


def now() -> float:
    return 1.0


def test_a_row_opens(shell):
    """A row opens."""
    assert shell.open(), "the row didn't open"


def test_b_row_closes(shell):
    assert shell.close(), (
        "the row didn't close"
    )
"#;

/// A case of `name`, as a change adds it at the file's end.
fn case(name: &str) -> String {
    format!("\n\ndef {name}(shell):\n    assert shell.{name}(), \"{name} failed\"\n")
}

/// A helper of `name` giving `gives`, as a change adds it at the end.
fn helper(name: &str, gives: &str) -> String {
    format!("\n\ndef {name}() -> bool:\n    return {gives}\n")
}

/// `text` with a case, its decorator running over lines, put before the
/// second case.
fn between(text: &str) -> String {
    text.replacen(
        "def test_b_row_closes",
        "@pytest.mark.parametrize(\n    \"row\",\n    [1, 2],\n)\ndef test_ab_rows(shell, \
         row):\n    assert shell.row(row), \"no row\"\n\n\ndef test_b_row_closes",
        1,
    )
}

#[test]
fn cases_added_or_changed_at_different_places_are_apart() {
    // 12 November's case: one change added a case and a helper of its own
    // at the end, while main added a case higher up and changed another.
    let added = helper("moved", "True") + &case("test_c_row_moves");
    let ours = format!("{BASE}{added}");
    let theirs = between(BASE).replace("the row didn't open", "no row opened");
    let head = format!("{theirs}{added}");
    assert!(
        cases_apart(BASE, &ours, &theirs, &head),
        "a case and a helper added while main added and changed others"
    );
    for (one, other) in [(&ours, &theirs), (&theirs, &ours)] {
        assert!(
            would_cases_be_apart(BASE, one, other),
            "the same, before the rebase, either way round"
        );
    }
    let ours = BASE.replace("the row didn't close", "no row closed");
    let theirs = BASE.replace("the row didn't open", "no row opened");
    let head = ours.replace("the row didn't open", "no row opened");
    assert!(
        cases_apart(BASE, &ours, &theirs, &head),
        "each side changed a case of its own"
    );
}

#[test]
fn a_helper_constant_fixture_or_import_changed_beside_a_case_is_not_apart() {
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    for (what, from, to) in [
        ("a helper", "return 1.0", "return 2.0"),
        ("a constant", "READ = 2.0", "READ = 3.0"),
        (
            "a fixture's decorator",
            "scope=\"module\"",
            "scope=\"session\"",
        ),
        (
            "the imports",
            "import pytest\n",
            "import json\n\nimport pytest\n",
        ),
        ("the docstring", "launcher closed.", "launcher shut."),
    ] {
        let theirs = BASE.replacen(from, to, 1);
        let head = ours.replacen(from, to, 1);
        assert!(
            !cases_apart(BASE, &ours, &theirs, &head),
            "main changed {what} beside the branch's new case"
        );
        assert!(
            !would_cases_be_apart(BASE, &theirs, &ours),
            "{what} changed beside a new case, before the rebase"
        );
    }
    let theirs = BASE.replacen("\n\ndef now() -> float:\n    return 1.0\n", "", 1);
    assert!(
        !would_cases_be_apart(BASE, &ours, &theirs),
        "main took a helper away"
    );
}

#[test]
fn an_entry_added_that_reaches_every_case_is_not_apart() {
    // 80ECO2's case: pytest runs an autouse fixture, a hook and the
    // file's marks for every case, and a name the imports bound is
    // rebound for every case that reads it.
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    for (what, added) in [
        (
            "an autouse fixture",
            "\n\n@pytest.fixture(autouse=True)\ndef settled():\n    yield\n",
        ),
        (
            "a hook",
            "\n\ndef pytest_collection_modifyitems(items):\n    items.reverse()\n",
        ),
        ("the file's marks", "\n\npytestmark = pytest.mark.slow\n"),
        ("a name the imports bound", "\n\nannotations = None\n"),
    ] {
        let theirs = format!("{BASE}{added}");
        assert!(
            !would_cases_be_apart(BASE, &ours, &theirs),
            "main added {what} beside the branch's new case"
        );
    }
    let theirs = format!("{BASE}{}", helper("closed", "True"));
    assert!(
        would_cases_be_apart(BASE, &ours, &theirs),
        "a helper named by a word only the docstring says"
    );
}

#[test]
fn a_case_or_helper_both_sides_touched_is_not_apart() {
    let ours = BASE.replace("the row didn't open", "no row opened");
    let theirs = BASE.replace("A row opens.", "A row opens at once.");
    let head = ours.replace("A row opens.", "A row opens at once.");
    assert!(
        !cases_apart(BASE, &ours, &theirs, &head),
        "both changed one case"
    );
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    assert!(
        !would_cases_be_apart(BASE, &ours, &ours),
        "both added one case"
    );
    let (ours, theirs) = (
        format!("{BASE}{}", helper("moved", "True")),
        format!("{BASE}{}", helper("moved", "False")),
    );
    assert!(
        !would_cases_be_apart(BASE, &ours, &theirs),
        "both added a helper of one name"
    );
}

#[test]
fn a_file_read_otherwise_or_merged_oddly_is_not_apart() {
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    let theirs = format!("{BASE}{}", case("test_d_row_stays"));
    let both = format!("{ours}{}", case("test_d_row_stays"));
    assert!(
        cases_apart(BASE, &ours, &theirs, &both),
        "two cases added at the end"
    );
    let twice = format!("{BASE}{}", helper("now", "True"));
    assert!(
        !would_cases_be_apart(BASE, &ours, &twice),
        "main's side gives a name twice"
    );
    let open = format!("{BASE}\nSAID = \"\"\"\nnever closed\n");
    assert!(
        !would_cases_be_apart(BASE, &ours, &open),
        "main's side leaves a string open"
    );
    let plain = |text: &str| text.replace("import pytest\n", "");
    assert!(
        !cases_apart(&plain(BASE), &plain(&ours), &plain(&theirs), &plain(&both)),
        "a file that doesn't import pytest"
    );
    let head = format!("{both}{}", case("test_z_neither"));
    assert!(
        !cases_apart(BASE, &ours, &theirs, &head),
        "the head adds a case neither side did"
    );
}

#[test]
fn a_pytest_file_given_a_case_each_stands_apart_after_the_rebase() {
    use crate::ready::{Side, stands_apart};
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    let theirs = format!("{BASE}{}", case("test_d_row_stays"));
    let head = format!("{ours}{}", case("test_d_row_stays"));
    let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
    let (at_base, checked) = (|_: &str| BASE.to_owned(), |_: &str| ours.clone());
    for (file, cases) in [
        ("tests/launcher-on-niri-test.py", true),
        ("tests/launcher_on_niri.py", false),
    ] {
        let files = [file.to_owned()];
        let (branch, main) = (
            Side {
                changed: &files,
                gone: &[],
                text: &at_head,
            },
            Side {
                changed: &files,
                gone: &[],
                text: &at_main,
            },
        );
        assert_eq!(
            stands_apart(file, &branch, &main, &at_base, &checked),
            cases,
            "{file}, given a case by each side, was judged wrongly"
        );
    }
}

#[test]
fn only_a_test_file_or_list_of_modules_is_judged_before_the_rebase() {
    let ours = format!("{BASE}{}", case("test_c_row_moves"));
    let theirs = format!("{BASE}{}", case("test_d_row_stays"));
    assert!(
        super::super::would_stand_apart("tests/launcher-on-niri-test.py", BASE, &ours, &theirs),
        "a pytest file given a case by each side"
    );
    assert!(
        !super::super::would_stand_apart("tests/launcher_on_niri.py", BASE, &ours, &theirs),
        "a Python file not named for a test"
    );
    let list = "[\n  ./a\n  ./c\n]\n";
    assert!(
        super::super::would_stand_apart(
            "flake.nix",
            list,
            "[\n  ./a\n  ./b\n  ./c\n]\n",
            "[\n  ./0\n  ./a\n  ./c\n]\n"
        ),
        "a list of modules given a line by each side"
    );
}
