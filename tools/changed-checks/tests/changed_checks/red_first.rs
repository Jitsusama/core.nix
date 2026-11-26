//! Red first: a committed change's checks, built again with its tests laid
//! over the base's code, must fail there.

use std::fs;

use super::{StandIn, text};

/// The base's checks, and the tree's, where the change moved `a`.
const BASE: &str = r#"{"a": "a.drv", "vm-x": "v.drv"}"#;
const WORKING: &str = r#"{"a": "a2.drv", "vm-x": "v2.drv"}"#;

/// A change adding a test of `a`'s and changing its code.
const CHANGE: &str = "A\tmodules/a/tests/new.rs\nM\tmodules/a/src/lib.rs\n";

/// A committed change, `diff.whole` its files, whose tests over the base
/// give `a` the derivation `over`, which moves no further with files
/// nothing names.
fn committed(test: &str, change: &str, over: &str) -> StandIn {
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("clean"), "").expect("written");
    fs::write(stand_in.path("diff.whole"), change).expect("written");
    let derivation = format!(r#"{{"a": "{over}"}}"#);
    fs::write(stand_in.path("over.json"), &derivation).expect("written");
    fs::write(stand_in.path("probed.json"), &derivation).expect("written");
    stand_in
}

/// The flake red first builds `a` from: the change's tests over the base.
fn over_the_base(stand_in: &StandIn) -> String {
    let repo = stand_in.path("repo");
    format!(
        "git+file://{}?rev=a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.a",
        repo.display()
    )
}

#[test]
fn a_new_test_that_passes_on_the_base_is_refused_naming_the_check_and_its_tests() {
    let stand_in = committed("red-passes", CHANGE, "a3.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        !ran.status.success(),
        "a test passing on the base went through"
    );
    let said = text(&ran.stderr);
    assert!(
        said.contains(
            "a passes with the change's tests laid over the base's code \
             (modules/a/tests/new.rs), so they'd pass without the change"
        ),
        "{said}"
    );
    assert!(
        said.contains("a check passes with the change's tests laid over the base's code"),
        "{said}"
    );
    assert!(
        stand_in.built().contains(&over_the_base(&stand_in)),
        "a wasn't built over the base: {}",
        stand_in.built()
    );
    let indexed = fs::read_to_string(stand_in.path("indexed")).unwrap_or_default();
    assert!(
        indexed.contains("100644 blob 1111\tmodules/a/tests/new.rs") && !indexed.contains("lib.rs"),
        "the test wasn't laid over the base, or the code was: {indexed}"
    );
    let commits = fs::read_to_string(stand_in.path("commits")).unwrap_or_default();
    assert!(
        commits.starts_with(
            "tree laidtree\nparent 0123456789abcdef0123\nauthor someone <a@b> 7 +0000\n\
             committer someone <a@b> 7 +0000\n"
        ),
        "the tests weren't laid over the base as a commit of its own: {commits}"
    );
}

#[test]
fn a_new_test_that_fails_on_the_base_and_passes_at_the_head_goes_through() {
    let stand_in = committed("red-fails", CHANGE, "a3.drv");
    fs::write(stand_in.path("failing-over"), "a\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr)
            .contains("a fails with the change's tests laid over the base, as it should"),
        "{}",
        text(&ran.stderr)
    );
    assert!(stand_in.built().contains(&over_the_base(&stand_in)));
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
}

#[test]
fn a_test_added_among_code_is_said_since_no_path_tells_it_apart() {
    let stand_in = committed("red-inline", "M\tmodules/a/src/lib.rs\n", "a3.drv");
    let lines = "+++ b/modules/a/src/lib.rs\n@@ -1,0 +2 @@\n+#[test]\n";
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
}

#[test]
fn a_check_reading_the_whole_tree_is_said_and_not_built_over_the_base() {
    let stand_in = committed("red-whole", CHANGE, "a3.drv");
    fs::write(stand_in.path("probed.json"), r#"{"a": "a4.drv"}"#).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("a read the whole tree"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
}

#[test]
fn a_lint_reading_the_tests_is_said_and_not_built_over_the_base() {
    let stand_in = committed("red-lint", CHANGE, "a3.drv");
    stand_in.checks("base", r#"{"a": "a.drv", "a-passes-clippy": "l.drv"}"#);
    stand_in.checks("working", r#"{"a": "a2.drv", "a-passes-clippy": "l2.drv"}"#);
    let over = r#"{"a": "a3.drv", "a-passes-clippy": "l3.drv"}"#;
    fs::write(stand_in.path("over.json"), over).expect("written");
    fs::write(stand_in.path("probed.json"), over).expect("written");
    fs::write(stand_in.path("failing-over"), "a\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains(
            "a-passes-clippy lint the change's tests without running them, so they aren't \
             asked to fail on the base"
        ),
        "{}",
        text(&ran.stderr)
    );
    assert!(
        !stand_in
            .built()
            .contains("a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.a-passes-clippy"),
        "the lint was built over the base: {}",
        stand_in.built()
    );
    assert!(stand_in.built().contains(&over_the_base(&stand_in)));
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
}

#[test]
fn a_check_the_tests_dont_reach_or_that_isnt_there_over_the_base_builds_nothing_there() {
    for (test, over, said) in [
        ("red-unreached", r#"{"a": "a.drv"}"#, None),
        (
            "red-undefined",
            "{}",
            Some("a isn't a check over the base, so its tests aren't run there"),
        ),
        ("red-unevaluated", r#"{"a": null}"#, None),
    ] {
        let stand_in = committed(test, CHANGE, "a3.drv");
        fs::write(stand_in.path("over.json"), over).expect("written");

        let ran = stand_in.run(&["--system", "x86_64-linux"]);

        assert!(ran.status.success(), "{test}: {}", text(&ran.stderr));
        if let Some(said) = said {
            assert!(
                text(&ran.stderr).contains(said),
                "{test}: {}",
                text(&ran.stderr)
            );
        }
        assert!(
            !stand_in.built().contains("?rev="),
            "{test}: {}",
            stand_in.built()
        );
    }
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
    assert!(!uncommitted.built().contains("?rev="));

    let named = committed("red-named", CHANGE, "a3.drv");
    let ran = named.run(&["--system", "x86_64-linux", "--name-only"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(named.built(), "", "a run only naming checks built one");
}
