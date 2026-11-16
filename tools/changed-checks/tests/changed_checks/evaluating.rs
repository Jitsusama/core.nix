//! How a run evaluates: from the derivations it evaluated, past what passed
//! at its commit, in chunks beside its builds, and the base and a clean
//! tree kept by their commit.

use std::fs;
use std::path::Path;

use super::{BASE, StandIn, WORKING, evaluated, text};

#[test]
fn it_builds_each_check_from_the_derivation_it_evaluated_rather_than_evaluating_again() {
    // An evaluator holds its heap until its last build ends, so building
    // from the flake again held gigabytes through every VM test.
    let stand_in = StandIn::new("derivations");
    let new = stand_in.path("n.drv");
    fs::write(&new, "").expect("a derivation");
    let missing = stand_in.path("a2.drv");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        &format!(
            r#"{{"a": "{}", "b": "b.drv", "new": "{}", "vm-x": "v2.drv"}}"#,
            missing.display(),
            new.display()
        ),
    );

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {}^* {flake}.a\n", new.display()),
        "a derivation collected since its evaluation is built from the flake"
    );
}

#[test]
fn a_check_told_it_passed_at_this_commit_isnt_built_again() {
    // A run sent to a builder whose store was new built again what an
    // earlier builder had passed: 631 s, then 1608 s, on 3 November.
    let stand_in = StandIn::new("passed");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("clean"), "").expect("written");
    let arguments = ["--system", "x86_64-linux", "--here"];
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());

    let ran = stand_in.run_told(
        &arguments,
        &[("CHANGED_CHECKS_PASSED", "fedcba9876543210fedc a")],
    );

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
        "what changed is still named"
    );
    assert!(
        text(&ran.stderr).contains("a passed at fedcba987654 already, so it isn't built again"),
        "{}",
        text(&ran.stderr)
    );
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.new\n")
    );

    fs::remove_file(stand_in.path("built")).expect("removed");
    let ran = stand_in.run_told(
        &arguments,
        &[("CHANGED_CHECKS_PASSED", "fedcba9876543210fedc a new")],
    );
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), "", "with each passed, nothing is built");

    // Passes at another commit, or told a tree with changes, say nothing
    // of this one.
    for (label, told) in [
        ("another commit", "0123456789abcdef0123 a new"),
        ("changes", ""),
    ] {
        let told = if told.is_empty() {
            fs::remove_file(stand_in.path("clean")).expect("removed");
            "fedcba9876543210fedc a new"
        } else {
            told
        };
        let ran = stand_in.run_told(&arguments, &[("CHANGED_CHECKS_PASSED", told)]);
        assert!(ran.status.success(), "{label}: {}", text(&ran.stderr));
        assert_eq!(
            stand_in.built(),
            format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
            "{label}"
        );
        fs::remove_file(stand_in.path("built")).expect("removed");
    }
}

#[test]
fn a_check_that_stops_evaluating_fails_the_run_and_nothing_is_built() {
    let stand_in = StandIn::new("broken");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        r#"{"a": null, "b": "b.drv", "gone": "g.drv", "vm-x": "v.drv"}"#,
    );

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build"]);

    assert_eq!(ran.status.code(), Some(1));
    assert_eq!(text(&ran.stdout), "broken   a\n");
    assert_eq!(stand_in.built(), "");
}

#[test]
fn a_run_building_no_vm_test_builds_while_it_evaluates_the_vm_tests() {
    // On 11 November the 90 VM tests were 11% of the checks but half of
    // each run's evaluating, and a gate builds none of them, so its build
    // waited minutes on evaluations it never used.
    let stand_in = StandIn::new("beside");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("vm-beside-build"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
        "every check is named, the VM test among them"
    );
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
        "the ordinary checks alone are built"
    );
}

#[test]
fn a_vm_test_that_stops_evaluating_beside_the_build_still_fails_the_run() {
    let stand_in = StandIn::new("beside-broken");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        r#"{"a": "a2.drv", "b": "b.drv", "new": "n.drv", "vm-x": null}"#,
    );
    fs::write(stand_in.path("vm-beside-build"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert_eq!(ran.status.code(), Some(1), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nremoved  gone\nbroken   vm-x\n",
        "the VM test that stopped evaluating is named with the rest"
    );
}

#[test]
fn the_base_is_evaluated_once_and_kept_by_its_commit() {
    let stand_in = StandIn::new("kept");
    stand_in.checks("base", BASE);
    stand_in.checks("working", BASE);
    let first = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(
        first.status.success(),
        "the first run failed: {}",
        text(&first.stderr)
    );
    // Were the base evaluated again, its checks would now all be removed.
    stand_in.checks("base", "{}");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let kept = Path::new("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(
        stand_in.folder.join(kept).exists(),
        "the base's checks weren't kept at {}",
        stand_in.folder.join(kept).display()
    );
    let runs = stand_in.runs();
    assert_eq!(runs.len(), 2, "both runs logged: {runs:?}");
    assert_eq!(evaluated(&runs[0]), ["base", "tree"], "{}", runs[0]);
    assert_eq!(evaluated(&runs[1]), ["tree"], "{}", runs[1]);
    assert_eq!(runs[1]["base"], "0123456789abcdef0123", "{}", runs[1]);
    assert_eq!(runs[1]["named"], 0, "{}", runs[1]);
    let chunks: Vec<&serde_json::Value> = runs[0]["evaluated"][1]["chunks"]
        .as_array()
        .expect("the tree's chunks")
        .iter()
        .map(|chunk| &chunk["checks"])
        .collect();
    assert_eq!(
        chunks,
        [3, 1],
        "the tree's ordinary checks, then its VM test: {}",
        runs[0]
    );
}

#[test]
fn a_clean_tree_is_kept_by_its_commit() {
    let stand_in = StandIn::new("clean");
    stand_in.checks("base", BASE);
    stand_in.checks("working", BASE);
    fs::write(stand_in.path("clean"), "").expect("written");
    let first = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(
        first.status.success(),
        "the first run failed: {}",
        text(&first.stderr)
    );
    // Were the tree evaluated again, its checks would now all be removed.
    stand_in.checks("working", "{}");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let kept = Path::new("cache/changed-checks/fedcba9876543210fedc-x86_64-linux.json");
    assert!(
        stand_in.folder.join(kept).exists(),
        "the clean tree's checks weren't kept at {}",
        stand_in.folder.join(kept).display()
    );
    let runs = stand_in.runs();
    assert_eq!(evaluated(&runs[0]), ["base", "tree"], "{}", runs[0]);
    assert!(evaluated(&runs[1]).is_empty(), "{}", runs[1]);
}

#[test]
fn two_chunks_are_evaluated_at_once() {
    let stand_in = StandIn::new("at-once");
    // Ninety ordinary checks are three chunks: forty, forty and ten.
    let many: serde_json::Map<String, serde_json::Value> = (0..90)
        .map(|at| (format!("check-{at}"), format!("{at}.drv").into()))
        .collect();
    let many = serde_json::to_string(&many).expect("derivations");
    stand_in.checks("base", &many);
    stand_in.checks("working", &many);
    fs::write(stand_in.path("meet"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let runs = stand_in.runs();
    let chunks: Vec<&serde_json::Value> = runs[0]["evaluated"][0]["chunks"]
        .as_array()
        .expect("the base's chunks")
        .iter()
        .map(|chunk| &chunk["checks"])
        .collect();
    assert_eq!(chunks, [40, 40, 10], "logged in their order: {}", runs[0]);
}
