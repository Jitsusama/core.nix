//! Red first: a committed change's checks, built again with its tests laid
//! over the base's code, must fail there.

use std::fs;

use super::{StandIn, text};

mod unjudged;

/// The base's checks, and the tree's, where the change moved `a`.
const BASE: &str = r#"{"a": "a.drv", "vm-x": "v.drv"}"#;
const WORKING: &str = r#"{"a": "a2.drv", "vm-x": "v2.drv"}"#;

/// A change adding a test of `a`'s and changing its code.
const CHANGE: &str = "A\tmodules/a/tests/new.rs\nM\tmodules/a/src/lib.rs\n";

/// What nix says of a build a builder failed.
const BUILDER_FAILED: &str =
    "error: Cannot build '/nix/store/x-a.drv'.\n       Reason: builder failed with exit code 1.\n";

/// A committed change, `diff.whole` its files, whose tests over the base
/// give `a` the derivation `over`, which moves no further with files
/// nothing names, and whose failed builds a builder failed.
fn committed(test: &str, change: &str, over: &str) -> StandIn {
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("clean"), "").expect("written");
    fs::write(stand_in.path("diff.whole"), change).expect("written");
    let derivation = format!(r#"{{"a": "{over}"}}"#);
    fs::write(stand_in.path("over.json"), &derivation).expect("written");
    fs::write(stand_in.path("probed.json"), &derivation).expect("written");
    fs::write(stand_in.path("failing.said"), BUILDER_FAILED).expect("written");
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

/// What a run says where red first would refuse the change, while it
/// only says so.
const SAID_ONLY: &str = "red first only says so for now and passes the change";

/// The verdict red first noted in the stand-in's one run, without how
/// long it took.
fn verdict(stand_in: &StandIn) -> serde_json::Value {
    let mut noted = stand_in.runs()[0]["red_first"].clone();
    let took = noted
        .as_object_mut()
        .and_then(|noted| noted.remove("seconds"));
    assert!(
        took.is_some_and(|took| took.is_u64()),
        "the verdict didn't note how long red first took: {noted}"
    );
    noted
}

#[test]
fn a_new_test_that_passes_on_the_base_is_said_refused_naming_the_check_and_its_tests() {
    let stand_in = committed("red-passes", CHANGE, "a3.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    let said = text(&ran.stderr);
    assert!(ran.status.success(), "a refusal failed the run: {said}");
    assert!(
        said.contains(
            "a passes with the change's tests laid over the base's code \
             (modules/a/tests/new.rs), so they'd pass without the change"
        ),
        "{said}"
    );
    assert!(said.contains(SAID_ONLY), "{said}");
    assert_eq!(
        verdict(&stand_in),
        serde_json::json!({
            "head": "fedcba9876543210fedc",
            "verdict": "would refuse",
            "checks": ["a"],
            "tests": ["modules/a/tests/new.rs"],
        }),
        "the run's line didn't note the refusal red first would make"
    );
    assert!(
        said.lines()
            .any(|line| line.trim().starts_with("Red-first: none, <why")),
        "the trailer to paste isn't on a line of its own: {said}"
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
    assert_eq!(
        verdict(&stand_in)["verdict"],
        "red",
        "the run's line didn't note the tests as proven: {}",
        verdict(&stand_in)
    );
    assert!(
        !text(&ran.stderr).contains(SAID_ONLY),
        "a proven change was said as refused: {}",
        text(&ran.stderr)
    );
    assert!(
        text(&ran.stderr).contains("changed-checks: building a over the base (1 of 1)"),
        "the run didn't say what it waits on: {}",
        text(&ran.stderr)
    );
    assert!(
        stand_in.built().contains(&over_the_base(&stand_in)),
        "a wasn't built over the base: {}",
        stand_in.built()
    );
    assert!(
        !text(&ran.stderr).contains("builder failed"),
        "nix's words for the red build reached the run's log, where merge-net reads them as \
         the change's: {}",
        text(&ran.stderr)
    );
}

/// The change of `committed`, where it moved `b` as well as `a`, which
/// have the derivations `a3` and `b3` over the base.
fn both_moved(test: &str) -> StandIn {
    let stand_in = committed(test, CHANGE, "a3.drv");
    stand_in.checks("base", r#"{"a": "a.drv", "b": "b.drv"}"#);
    stand_in.checks("working", r#"{"a": "a2.drv", "b": "b2.drv"}"#);
    let over = r#"{"a": "a3.drv", "b": "b3.drv"}"#;
    fs::write(stand_in.path("over.json"), over).expect("written");
    fs::write(stand_in.path("probed.json"), over).expect("written");
    stand_in
}

#[test]
fn one_check_failing_over_the_base_proves_tests_another_only_carries() {
    let stand_in = both_moved("red-one-of-two");
    fs::write(stand_in.path("failing-over"), "b\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let said = text(&ran.stderr);
    assert!(
        said.contains("b fails with the change's tests laid over the base, as it should"),
        "{said}"
    );
    assert!(
        !said.contains("passes with the change's tests"),
        "a check only carrying the tests was refused beside one they fail: {said}"
    );
}

#[test]
fn a_check_failing_over_the_base_leaves_the_rest_unbuilt_there() {
    let stand_in = both_moved("red-first-of-two");
    fs::write(stand_in.path("failing-over"), "a\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let built = stand_in.built();
    assert!(
        built.contains(&over_the_base(&stand_in)),
        "a wasn't built over the base: {built}"
    );
    assert!(
        !built.contains("?rev=a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.b"),
        "b was built over the base after a failed there: {built}"
    );
}

#[test]
fn a_change_whose_every_check_passes_over_the_base_is_said_refused_naming_each() {
    let stand_in = both_moved("red-both-pass");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    let said = text(&ran.stderr);
    assert!(ran.status.success(), "a refusal failed the run: {said}");
    assert_eq!(
        verdict(&stand_in)["checks"],
        serde_json::json!(["a", "b"]),
        "the run's line didn't note each check passing there: {}",
        verdict(&stand_in)
    );
    for name in ["a", "b"] {
        assert!(
            said.contains(&format!(
                "changed-checks: {name} passes with the change's tests laid over the base's code"
            )),
            "{name} wasn't named: {said}"
        );
    }
}

#[test]
fn a_build_over_the_base_no_builder_failed_is_said_and_not_counted_red() {
    let stand_in = committed("red-unbuilt", CHANGE, "a3.drv");
    fs::write(stand_in.path("failing-over"), "a\n").expect("written");
    let lost = "error: unable to download 'https://cache/x.narinfo': Couldn't connect\n";
    fs::write(stand_in.path("failing.said"), lost).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let said = text(&ran.stderr);
    assert!(
        said.contains("a couldn't be built over the base, though no builder failed it"),
        "{said}"
    );
    assert!(
        !said.contains("as it should"),
        "a lost build was counted red: {said}"
    );
    let kept = fs::read_dir(stand_in.path("cache/changed-checks"))
        .expect("the cache was made")
        .filter_map(Result::ok)
        .find(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("red-first-") && name.ends_with("-a.log")
        })
        .expect("nix's words for the lost build weren't kept");
    let words = fs::read_to_string(kept.path()).expect("read");
    assert!(
        words.contains("unable to download"),
        "the kept log doesn't hold nix's words: {words}"
    );
}

#[test]
fn a_check_reading_none_of_the_changes_code_is_said_and_not_built_over_the_base() {
    let stand_in = committed("red-only-tests", CHANGE, "a2.drv");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a check reading only the tests, as a scan of every VM test does, refused: {}",
        text(&ran.stderr)
    );
    assert!(
        text(&ran.stderr).contains(
            "a come out the same with the change's tests laid over the base as at its head"
        ),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
}

#[test]
fn a_check_failing_to_evaluate_over_the_base_proves_tests_another_only_carries() {
    let stand_in = both_moved("red-unevaluated-of-two");
    let over = r#"{"a": null, "b": "b3.drv"}"#;
    fs::write(stand_in.path("over.json"), over).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a check passing beside one that doesn't evaluate refused: {}",
        text(&ran.stderr)
    );
    assert!(
        text(&ran.stderr).contains("a doesn't evaluate with the change's tests laid over the base"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
}

#[test]
fn a_lost_build_over_the_base_beside_a_passing_one_refuses_nothing() {
    let stand_in = both_moved("red-lost-of-two");
    fs::write(stand_in.path("failing-over"), "b\n").expect("written");
    let lost = "error: unable to download 'https://cache/x.narinfo': Couldn't connect\n";
    fs::write(stand_in.path("failing.said"), lost).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a passing check refused though the one beside it was lost: {}",
        text(&ran.stderr)
    );
    let said = text(&ran.stderr);
    assert!(
        said.contains("b couldn't be built over the base, though no builder failed it"),
        "{said}"
    );
    assert!(
        !said.contains("passes with the change's tests"),
        "a check was refused beside a lost one: {said}"
    );
}

#[test]
fn the_checks_running_tests_are_built_over_the_base_before_the_systems() {
    let stand_in = committed("red-systems-last", CHANGE, "a3.drv");
    // The check running the tests sorts after the system by name, so only
    // the ordering puts it first.
    stand_in.checks("base", r#"{"example-a": "e.drv", "z": "z.drv"}"#);
    stand_in.checks("working", r#"{"example-a": "e2.drv", "z": "z2.drv"}"#);
    let over = r#"{"example-a": "e3.drv", "z": "z3.drv"}"#;
    fs::write(stand_in.path("over.json"), over).expect("written");
    fs::write(stand_in.path("probed.json"), over).expect("written");
    fs::write(stand_in.path("failing-over"), "z\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let built = stand_in.built();
    assert!(
        built.contains("?rev=a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.z"),
        "z wasn't built over the base: {built}"
    );
    assert!(
        !built.contains("?rev=a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.example-a"),
        "the system was built over the base before the check running the tests: {built}"
    );
}

#[test]
fn a_check_mains_quarantine_holds_is_not_asked_over_the_base() {
    let stand_in = both_moved("red-quarantined");
    let held = r#"[{"test": "b", "reason": "failed then passed at abc", "owner_pr": 7}]"#;
    fs::write(stand_in.path("quarantine.json"), held).expect("written");
    fs::write(stand_in.path("failing-over"), "b\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        !stand_in
            .built()
            .contains("a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.b"),
        "the quarantined check was built over the base: {}",
        stand_in.built()
    );
    assert_eq!(
        verdict(&stand_in)["checks"],
        serde_json::json!(["a"]),
        "a quarantined check failing over the base was taken as proof: {}",
        verdict(&stand_in)
    );
}

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
}

#[test]
fn tests_that_stop_the_evaluation_over_the_base_are_said_and_refuse_nothing() {
    let stand_in = committed("red-unevaluable", CHANGE, "a3.drv");
    fs::remove_file(stand_in.path("over.json")).expect("removed");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr)
            .contains("the checks don't evaluate with the change's tests laid over the base ("),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
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
    let indexed = fs::read_to_string(stand_in.path("indexed")).unwrap_or_default();
    let probe = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tchanged-checks-probe";
    assert!(
        indexed.lines().any(|line| line == probe),
        "no probe was laid at the root beside the tests: {indexed}"
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
}

#[test]
fn a_lint_passing_over_the_base_is_never_named_in_a_refusal() {
    let stand_in = committed("red-lint-passes", CHANGE, "a3.drv");
    stand_in.checks("base", r#"{"a": "a.drv", "a-passes-clippy": "l.drv"}"#);
    stand_in.checks("working", r#"{"a": "a2.drv", "a-passes-clippy": "l2.drv"}"#);
    let over = r#"{"a": "a3.drv", "a-passes-clippy": "l3.drv"}"#;
    fs::write(stand_in.path("over.json"), over).expect("written");
    fs::write(stand_in.path("probed.json"), over).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    let said = text(&ran.stderr);
    assert!(
        said.contains("changed-checks: a passes with the change's tests"),
        "a test passing on the base wasn't said: {said}"
    );
    assert!(
        !said.contains("a-passes-clippy passes with the change's tests"),
        "a lint was named in the refusal: {said}"
    );
    assert!(
        !stand_in
            .built()
            .contains("a1a1a1a1a1a1a1a1a1a1#checks.x86_64-linux.a-passes-clippy"),
        "the lint was built over the base: {}",
        stand_in.built()
    );
}

#[test]
fn a_check_missing_over_the_base_beside_a_passing_one_refuses_nothing() {
    let stand_in = both_moved("red-undefined-of-two");
    fs::write(stand_in.path("over.json"), r#"{"b": "b3.drv"}"#).expect("written");
    fs::write(stand_in.path("probed.json"), r#"{"b": "b3.drv"}"#).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(
        ran.status.success(),
        "a change was refused though the check that might run its tests isn't over the base: {}",
        text(&ran.stderr)
    );
    assert!(
        text(&ran.stderr).contains("a isn't a check over the base"),
        "{}",
        text(&ran.stderr)
    );
    assert_eq!(
        verdict(&stand_in)["why"],
        "undefined",
        "b was asked though a check that might run the tests isn't over the base: {}",
        verdict(&stand_in)
    );
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
    assert!(
        stand_in.built().contains(&over_the_base(&stand_in)),
        "a wasn't built over the base beside the lint: {}",
        stand_in.built()
    );
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
fn a_check_the_tests_dont_reach_or_that_isnt_there_over_the_base_builds_nothing_there() {
    for (test, over, said) in [
        (
            "red-unreached",
            r#"{"a": "a.drv"}"#,
            "no check built reads the change's tests (modules/a/tests/new.rs)",
        ),
        (
            "red-undefined",
            "{}",
            "a isn't a check over the base, so it can't be asked to fail there",
        ),
        (
            "red-unevaluated",
            r#"{"a": null}"#,
            "a doesn't evaluate with the change's tests laid over the base",
        ),
    ] {
        let stand_in = committed(test, CHANGE, "a3.drv");
        fs::write(stand_in.path("over.json"), over).expect("written");

        let ran = stand_in.run(&["--system", "x86_64-linux"]);

        assert!(ran.status.success(), "{test}: {}", text(&ran.stderr));
        assert!(
            text(&ran.stderr).contains(said),
            "{test}: {}",
            text(&ran.stderr)
        );
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
    assert!(
        !uncommitted.built().contains("?rev="),
        "an uncommitted change was built over the base: {}",
        uncommitted.built()
    );

    let named = committed("red-named", CHANGE, "a3.drv");
    let ran = named.run(&["--system", "x86_64-linux", "--name-only"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(named.built(), "", "a run only naming checks built one");
}
