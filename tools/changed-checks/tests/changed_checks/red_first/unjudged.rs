//! Red first's runs it leaves unjudged: those the gate or the train judge
//! instead, those past its budget, and those adding a test among code.

use std::fs;
use std::time::{Duration, Instant};

use super::{CHANGE, committed, text, verdict};

#[test]
fn a_macs_run_leaves_the_tests_to_linux_and_builds_nothing_over_the_base() {
    let stand_in = committed("red-mac", CHANGE, "a3.drv");

    let ran = stand_in.run(&["--system", "aarch64-darwin"]);

    assert!(
        ran.status.success(),
        "a Mac's run refused: {}",
        text(&ran.stderr)
    );
    assert!(
        text(&ran.stderr).contains("asked to fail on the base on Linux"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
}

#[test]
fn a_train_candidate_the_train_marks_is_not_judged_again() {
    let stand_in = committed("red-train", CHANGE, "a3.drv");

    let told = [("CHANGED_CHECKS_CANDIDATE", "1")];
    let ran = stand_in.run_told(&["--system", "x86_64-linux"], &told);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("the train's candidate merges changes each judged"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
}

#[test]
fn a_change_of_merges_alone_is_taken_for_the_trains_candidate_and_not_judged() {
    let stand_in = committed("red-merges", CHANGE, "a3.drv");
    fs::write(stand_in.path("log"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("every commit since the base merges another change"),
        "{}",
        text(&ran.stderr)
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    assert_eq!(
        verdict(&stand_in)["why"],
        "merges alone",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn a_build_over_the_base_that_never_ends_is_stopped_at_the_budget_and_refuses_nothing() {
    let stand_in = committed("red-hangs", CHANGE, "a3.drv");
    fs::write(stand_in.path("hangs-over"), "").expect("written");

    let began = Instant::now();
    let told = [("CHANGED_CHECKS_RED_FIRST_SECONDS", "1")];
    let ran = stand_in.run_told(&["--system", "x86_64-linux"], &told);

    let said = text(&ran.stderr);
    assert!(
        ran.status.success(),
        "a build over the base past its budget failed the run: {said}"
    );
    assert!(
        began.elapsed() < Duration::from_secs(60),
        "a build over the base that never ends held the run for {:?}",
        began.elapsed()
    );
    assert!(
        said.contains("red first's 1 seconds of building over the base ran out at a"),
        "{said}"
    );
    assert_eq!(
        verdict(&stand_in)["why"],
        "over budget",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn a_budget_already_spent_builds_nothing_over_the_base() {
    let stand_in = committed("red-spent", CHANGE, "a3.drv");

    let told = [("CHANGED_CHECKS_RED_FIRST_SECONDS", "0")];
    let ran = stand_in.run_told(&["--system", "x86_64-linux"], &told);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    assert_eq!(
        verdict(&stand_in)["why"],
        "over budget",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn a_persons_run_built_on_the_lent_builder_leaves_red_first_to_the_gate() {
    let stand_in = committed("red-lent", CHANGE, "a3.drv");
    let offload = stand_in.path("offload");
    fs::create_dir_all(&offload).expect("its folder");
    let lent = "ssh-ng://lent?compress=true&ssh-key=/the/key\n";
    fs::write(offload.join("store"), lent).expect("written");
    fs::write(offload.join("machines-with-standing"), "").expect("written");
    fs::write(stand_in.path("pushed"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    let said = text(&ran.stderr);
    assert!(ran.status.success(), "{said}");
    assert!(
        said.contains("the change's checks were built on the lent builder"),
        "{said}"
    );
    assert!(!stand_in.built().contains("?rev="), "{}", stand_in.built());
    assert_eq!(
        verdict(&stand_in)["why"],
        "built elsewhere",
        "{}",
        verdict(&stand_in)
    );
}

#[test]
fn an_assert_added_inside_a_code_files_inline_tests_is_said_apart() {
    let stand_in = committed("red-inline-assert", "M\tmodules/a/src/lib.rs\n", "a3.drv");
    let head = stand_in.path("show/modules/a/src");
    fs::create_dir_all(&head).expect("its folder");
    let code = "fn a() -> u8 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    \
                #[test]\n    fn holds() {\n        assert_eq!(a(), 1, \"a\");\n        \
                assert_eq!(a(), 1, \"again\");\n    }\n}\n";
    fs::write(head.join("lib.rs"), code).expect("written");
    let lines = "diff --git a/modules/a/src/lib.rs b/modules/a/src/lib.rs\n\
                 +++ b/modules/a/src/lib.rs\n@@ -11,0 +12 @@\n\
                 +        assert_eq!(a(), 1, \"again\");\n";
    fs::write(stand_in.path("diff.lines"), lines).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("modules/a/src/lib.rs adds a test among its code"),
        "an assert added inside an inline test went unseen: {}",
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
