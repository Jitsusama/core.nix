//! Red first sparing a change for how its tests lie, for a commit excusing
//! them, or for red first's own failure: each builds nothing over the base,
//! and is said and noted in the run's line.

use std::fs;

use serde_json::json;

use super::{CHANGE, committed, noted_with, text, verdict};

#[test]
fn red_first_failing_for_its_own_reasons_is_said_and_refuses_nothing() {
    let stand_in = committed("red-unwritable", CHANGE, "a3.drv");
    fs::write(stand_in.path("unwritable"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    let said = text(&ran.stderr);
    assert!(
        ran.status.success(),
        "red first's own failure failed the run: {said}"
    );
    assert!(
        said.contains("red first couldn't judge the change (git read-tree"),
        "{said}"
    );
    assert_eq!(
        verdict(&stand_in)["why"],
        "couldn't judge",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn a_change_only_taking_lines_from_its_tests_builds_nothing_over_the_base() {
    let stand_in = committed("red-taken", CHANGE, "a3.drv");
    let counted = "0\t12\tmodules/a/tests/new.rs\n";
    fs::write(stand_in.path("diff.counted"), counted).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("the change only takes lines from its tests"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    noted_with(&stand_in, &json!({ "why": "lines only taken" }));
}

#[test]
fn a_change_of_tests_alone_is_said_and_builds_nothing_over_the_base() {
    let stand_in = committed("red-tests-only", "A\tmodules/a/tests/new.rs\n", "a2.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("the change touches only tests"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    noted_with(&stand_in, &json!({ "why": "only tests" }));
}

#[test]
fn a_test_added_among_code_is_said_since_no_path_tells_it_apart() {
    let stand_in = committed("red-inline", "M\tmodules/a/src/lib.rs\n", "a3.drv");
    let lines = "diff --git a/modules/a/src/lib.rs b/modules/a/src/lib.rs\n\
                 +++ b/modules/a/src/lib.rs\n@@ -1,0 +2 @@\n+    #[test]\n";
    fs::write(stand_in.path("diff.lines"), lines).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr)
            .contains("modules/a/src/lib.rs adds a test among its code, which no path tells apart"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    assert_eq!(
        verdict(&stand_in)["verdict"],
        "apart",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn a_new_test_file_only_the_changes_code_declares_is_said_and_not_laid() {
    let change = "A\tmodules/a/src/new.rs\nA\tmodules/a/src/new/tests.rs\n";
    let stand_in = committed("red-declared", change, "a3.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains(
            "modules/a/src/new/tests.rs is a new test file only the change's code declares"
        ),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    noted_with(&stand_in, &json!({ "verdict": "apart" }));
}

#[test]
fn a_change_removing_a_test_beside_one_it_adds_is_said_and_not_judged() {
    let change = "A\tmodules/a/tests/new.rs\nD\tmodules/a/tests/old.rs\nM\tmodules/a/src/lib.rs\n";
    let stand_in = committed("red-removed", change, "a3.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a change whose laid tests the head doesn't have was judged: {}",
        text(&ran.stderr)
    );
    let said = text(&ran.stderr);
    assert!(
        said.contains("modules/a/tests/old.rs is a test the change removes"),
        "{said}"
    );
    assert!(
        said.contains("so the change's tests can't be laid over the base as its head has them"),
        "{said}"
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    noted_with(&stand_in, &json!({ "verdict": "apart" }));
}

#[test]
fn a_change_whose_new_tests_only_its_code_declares_is_not_refused_over_a_helper() {
    let change = "A\tmodules/a/src/new.rs\n\
                  A\tmodules/a/src/new/tests.rs\n\
                  M\tmodules/a/tests/a/main.rs\n";
    let stand_in = committed("red-declared-beside", change, "a3.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a change was refused over an edited helper while its tests were set aside: {}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    noted_with(&stand_in, &json!({ "verdict": "apart" }));
}

#[test]
fn a_test_pinning_what_was_right_is_excused_by_its_trailer_and_said() {
    let stand_in = committed("red-pinned", CHANGE, "a3.drv");
    let log = "test(a): pin b\x1fnone, b was right before\n\x1e\n";
    fs::write(stand_in.path("log"), log).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("Red-first: none, b was right before"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    assert_eq!(
        (&verdict(&stand_in)["excuse"], &verdict(&stand_in)["why"]),
        (
            &serde_json::json!("pinned"),
            &serde_json::json!("b was right before")
        ),
        "the run's line didn't note the excuse: {}",
        verdict(&stand_in)
    );
}

#[test]
fn a_change_not_yet_committed_or_only_named_is_not_judged_on_the_base() {
    let uncommitted = committed("red-uncommitted", CHANGE, "a3.drv");
    fs::remove_file(uncommitted.path("clean")).expect("removed");
    let ran = uncommitted.run(&["--system", "x86_64-linux"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("asked to fail on the base once it's committed"),
        "{}",
        text(&ran.stderr)
    );
    assert!(
        !uncommitted.built().contains("?rev="),
        "an uncommitted change was built over the base: {}",
        uncommitted.built()
    );
    noted_with(&uncommitted, &json!({ "why": "uncommitted" }));

    let named = committed("red-named", CHANGE, "a3.drv");
    let ran = named.run(&["--system", "x86_64-linux", "--name-only"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(named.built(), "", "a run only naming checks built one");
}

#[test]
fn a_change_whose_every_commit_git_says_reverts_is_excused_and_said() {
    let stand_in = committed("red-reverts", CHANGE, "a3.drv");
    let log = "Revert \"feat(a): b\"\x1f\x1fIt broke c.\n\n\
               This reverts commit 0123456789abcdef0123456789abcdef01234567.\n\x1e\n";
    fs::write(stand_in.path("log"), log).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("the change reverts, so its tests aren't asked"),
        "a revert wasn't said as one: {}",
        text(&ran.stderr)
    );
    assert!(
        !stand_in.built().contains("?rev="),
        "a revert was built over the base: {}",
        stand_in.built()
    );
    noted_with(
        &stand_in,
        &json!({ "verdict": "excused", "excuse": "reverts" }),
    );
}

#[test]
fn a_change_only_named_as_reverting_is_built_over_the_base() {
    let stand_in = committed("red-named-revert", CHANGE, "a3.drv");
    let log = "Reverting a\x1f\x1fIt undoes b.\n\x1e\nrevertible(a): c\x1f\x1f\x1e\n";
    fs::write(stand_in.path("log"), log).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        !text(&ran.stderr).contains("the change reverts"),
        "a change only named as reverting was said to revert: {}",
        text(&ran.stderr)
    );
    assert!(
        stand_in.built().contains("?rev="),
        "a change only named as reverting wasn't built over the base: {}",
        stand_in.built()
    );
}
