//! What a run builds and where: the checks a change moved, VM tests whole,
//! what main quarantines, and the doors, builders and machine a run goes to.

use std::fs;
use std::process::Output;

use super::{BASE, StandIn, WORKING, text};

#[test]
fn asked_only_to_name_them_it_names_each_check_the_change_moved_and_builds_nothing() {
    let stand_in = StandIn::new("names");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);

    let ran = stand_in.run(&["--system", "x86_64-linux", "--name-only"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n"
    );
    assert!(text(&ran.stderr).contains("checks at 0123456789ab with the working tree"));
    assert_eq!(stand_in.built(), "");
}

#[test]
fn by_default_or_asked_it_builds_what_was_added_and_changed_but_the_vm_tests() {
    // A named check proves nothing until it's built: #1373 merged naming
    // checks it never built.
    for (label, arguments) in [
        ("builds-unasked", &["--system", "x86_64-linux"][..]),
        (
            "builds",
            &["--system", "x86_64-linux", "--build", "--no-vm"][..],
        ),
    ] {
        let stand_in = StandIn::new(label);
        stand_in.checks("base", BASE);
        stand_in.checks("working", WORKING);

        let ran = stand_in.run(arguments);

        assert!(ran.status.success(), "{arguments:?}: {}", text(&ran.stderr));
        let repo = stand_in.path("repo");
        let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
        assert_eq!(
            stand_in.built(),
            format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
            "{arguments:?}"
        );
        let runs = stand_in.runs();
        assert_eq!(runs.len(), 1, "one run logged: {runs:?}");
        assert_eq!(runs[0]["built"]["checks"], 2, "{}", runs[0]);
        assert_eq!(runs[0]["built"]["passed"], true, "{}", runs[0]);
    }
}

// A run that builds VM tests whole says how many before it sends them,
// and that the gate before a merge doesn't ask for them; one that leaves
// them out says nothing of them.
#[test]
fn building_vm_tests_whole_says_how_many_and_what_the_gate_needs() {
    for (label, arguments, said) in [
        (
            "with-vm",
            &["--system", "x86_64-linux", "--build"][..],
            true,
        ),
        ("without-vm", &["--system", "x86_64-linux"][..], false),
    ] {
        let stand_in = StandIn::new(label);
        stand_in.checks("base", BASE);
        stand_in.checks("working", WORKING);

        let ran = stand_in.run(arguments);

        assert!(ran.status.success(), "{label}: {}", text(&ran.stderr));
        assert_eq!(
            text(&ran.stderr).contains("building 1 VM test whole, as --build asks"),
            said,
            "{label}: {}",
            text(&ran.stderr)
        );
        assert_eq!(
            text(&ran.stderr).contains("needs only those `merge-net needs` names"),
            said,
            "{label}"
        );
    }
}

#[test]
fn a_check_main_quarantines_is_built_alone_and_its_failure_blocks_nothing() {
    let stand_in = StandIn::new("quarantined");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(
        stand_in.path("quarantine.json"),
        r#"[{"test": "new", "reason": "failed then passed at abc", "owner_pr": 7}]"#,
    )
    .expect("written");
    fs::write(stand_in.path("failing"), "new\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains(
            "new failed, but main quarantines it (failed then passed at abc; #7 owns it), \
             so it blocks nothing"
        ),
        "{}",
        text(&ran.stderr)
    );
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.a\n--no-link --keep-going -L {flake}.new\n")
    );
    assert_eq!(stand_in.runs()[0]["built"]["passed"], true);

    // A check main doesn't quarantine still fails the run.
    fs::write(stand_in.path("failing"), "a\n").expect("written");
    let _ = fs::remove_file(stand_in.path("built"));
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(!ran.status.success(), "{}", text(&ran.stderr));
}

#[test]
fn an_entry_naming_subtests_excuses_only_a_failure_its_log_names_in_them() {
    let stand_in = StandIn::new("quarantined-subtests");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(
        stand_in.path("quarantine.json"),
        r#"[{"test": "new", "reason": "r", "owner_pr": 7, "subtests": ["test_q"]}]"#,
    )
    .expect("written");
    fs::write(stand_in.path("failing"), "new\n").expect("written");
    let failing_in = |said: &str| {
        fs::write(stand_in.path("failing.said"), said).expect("written");
        stand_in.run(&["--system", "x86_64-linux"])
    };

    let listed = failing_in("vm-test-run-new> FAILED new/new-vm-test.py::test_q - e\n");
    assert!(
        listed.status.success(),
        "a failure in a listed subtest blocked: {}",
        text(&listed.stderr)
    );

    let unlisted = failing_in(
        "vm-test-run-new> FAILED new/new-vm-test.py::test_q - e\n\
         vm-test-run-new> FAILED new/new-vm-test.py::test_r - e\n",
    );
    assert!(
        !unlisted.status.success(),
        "a failure in an unlisted subtest was excused: {}",
        text(&unlisted.stderr)
    );
    assert!(
        text(&unlisted.stderr)
            .contains("new failed in test_r, which main's quarantine doesn't excuse, so it blocks"),
        "{}",
        text(&unlisted.stderr)
    );

    let unnamed = failing_in("error: builder failed\n");
    assert!(
        !unnamed.status.success(),
        "a failure naming no subtest was excused: {}",
        text(&unnamed.stderr)
    );
    assert!(
        text(&unnamed.stderr).contains("new failed in no test its log names"),
        "{}",
        text(&unnamed.stderr)
    );
}

#[test]
fn a_build_here_goes_through_the_standing_door_where_there_is_one() {
    let stand_in = StandIn::new("door");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    let offload = stand_in.path("offload");
    fs::create_dir_all(&offload).expect("its folder");
    fs::write(offload.join("machines-with-standing"), "").expect("written");
    fs::write(stand_in.path("standing.sock"), "").expect("written");
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        stand_in.built(),
        format!(
            "--no-link --keep-going -L --store unix://{} {flake}.new {flake}.a\n",
            stand_in.path("standing.sock").display()
        ),
        "the build named the builders itself, which Nix heeds only from a trusted account"
    );
}

#[test]
fn a_pushed_commit_is_built_on_the_lent_builder_and_anything_else_here() {
    let stand_in = StandIn::new("elsewhere");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    let offload = stand_in.path("offload");
    fs::create_dir_all(&offload).expect("its folder");
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    let standing = offload.join("machines-with-standing");
    let here = format!(
        "--no-link --keep-going -L --builders @{} {flake}.new {flake}.a\n",
        standing.display()
    );
    let asked = || stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);
    fs::write(&standing, "").expect("written");

    fs::write(offload.join("store"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "no builder is lent");

    let lent = "ssh-ng://lent?compress=true&ssh-key=/the/key\n";
    fs::write(offload.join("store"), lent).expect("written");
    for (step, why) in [
        ("clean", "the working tree differs from its commit"),
        (
            "pushed",
            "origin doesn't have fedcba987654; push it to build there",
        ),
    ] {
        fs::remove_file(stand_in.path("built")).expect("cleared");
        let ran = asked();
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(stand_in.built(), here, "{why}");
        assert!(text(&ran.stderr).contains(&format!("building here, since {why}")));
        fs::write(stand_in.path(step), "").expect("written");
    }

    fs::remove_file(stand_in.path("built")).expect("cleared");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), "", "nothing is built here");
    assert_eq!(
        fs::read_to_string(stand_in.path("sent")).expect("it was sent"),
        "there\n-L\n--keep-going\ngithub:someone/thing/fedcba9876543210fedc\n\
         checks.x86_64-linux.new\nchecks.x86_64-linux.a\n"
    );
    let runs = stand_in.runs();
    let built = &runs.last().expect("the run")["built"];
    assert_eq!(built["lent"], true, "{built}");
    assert_eq!(built["sent"], 4321, "{built}");
    assert_eq!(built["checks"], 2, "{built}");

    // A builder gone before it took the build builds nothing there, and
    // it's built here.
    fs::write(stand_in.path("unsent"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "taken by no builder");
    assert!(text(&ran.stderr).contains("since no builder took the build there"));
    fs::remove_file(stand_in.path("unsent")).expect("cleared");
    fs::remove_file(stand_in.path("built")).expect("cleared");

    // A nix-offload older than `there` refuses it, and it's built here.
    fs::write(stand_in.path("refuses"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "refused there");
    assert!(text(&ran.stderr).contains("since nix-offload couldn't take the build there"));
}

#[test]
fn the_whole_run_goes_elsewhere_where_nix_offload_takes_it_naming_this_system() {
    let stand_in = StandIn::elsewhere("sent-whole");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");

    let ran = stand_in.run(&["--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        fs::read_to_string(stand_in.path("changed")).expect("it was sent"),
        format!(
            "{}\nthere\n--changed\n--build\n--no-vm\n--system\nx86_64-linux\n",
            fs::canonicalize(stand_in.path("repo"))
                .expect("the repository")
                .display()
        )
    );
    assert_eq!(stand_in.built(), "", "nothing is built here");
    assert!(stand_in.runs().is_empty(), "nothing is evaluated here");

    // The far side's failure is the run's.
    fs::write(stand_in.path("changed-exit"), "1").expect("written");
    assert_eq!(stand_in.run(&["--build"]).status.code(), Some(1));
}

// Runs no builder took evaluated and built here while the machine looked
// idle, and on 8 November one built openvino here for three hours beside
// Joel's work. Only one asked to runs here.
#[test]
fn it_runs_here_only_when_asked_and_otherwise_says_why_it_went_nowhere() {
    let stand_in = StandIn::elsewhere("kept-here");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    let named = "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n";
    let refused = |ran: &Output, why: &str| {
        let said = text(&ran.stderr);
        assert_eq!(ran.status.code(), Some(75), "{said}");
        assert!(said.contains(why), "{said}");
        assert!(said.contains("so nothing was run here"), "{said}");
        assert!(said.contains("--here runs it on this machine"), "{said}");
        assert_eq!(text(&ran.stdout), "", "nothing was evaluated here");
    };

    // Edits not yet committed aren't sent: the far side checks a commit, so
    // it would pass the last one rather than what's in the working tree.
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));
    refused(&ran, "uncommitted");
    let ran = stand_in.run(&["--system", "x86_64-linux", "--here"]);
    assert_eq!(text(&ran.stdout), named, "{}", text(&ran.stderr));
    fs::write(stand_in.path("clean"), "").expect("written");

    for arguments in [
        &["--system", "x86_64-linux", "--here"][..],
        &[
            "--system",
            "x86_64-linux",
            "--base",
            "refs/remotes/origin/main",
        ][..],
    ] {
        let _ = fs::remove_file(stand_in.path("changed"));
        let ran = stand_in.run(arguments);
        let sent = stand_in.path("changed").exists();
        let here = arguments.contains(&"--here");
        assert_eq!(sent, !here, "{arguments:?}: {}", text(&ran.stderr));
        if here {
            assert_eq!(text(&ran.stdout), named, "{arguments:?}");
        }
    }

    // A base only this checkout names shares no history the stand-in git
    // knows, so it's refused as asked wrongly, sent nowhere.
    let _ = fs::remove_file(stand_in.path("changed"));
    let ran = stand_in.run(&["--system", "x86_64-linux", "--base", "topic"]);
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));

    // Offered and taken nowhere, it isn't run here instead, idle or not.
    fs::write(stand_in.path("changed-exit"), "75").expect("written");
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(stand_in.path("changed").exists(), "it was offered first");
    refused(&ran, "no builder took the run");
}

#[test]
fn a_base_origins_main_holds_goes_with_the_run_as_its_commit_and_any_other_stays() {
    // A run comparing with an older commit of main, as one checking what
    // changed since main last passed does, stayed here and evaluated twice,
    // about 4 GB each, at a load of 118 on 30 October.
    let stand_in = StandIn::elsewhere("far-base");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");

    let ran = stand_in.run(&["--base", "older", "--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--changed\n--base\n1a96207100000000000a\n--system\nx86_64-linux\n"),
        "{sent}"
    );
    assert!(stand_in.runs().is_empty(), "nothing is evaluated here");

    fs::remove_file(stand_in.path("changed")).expect("cleared");
    let ran = stand_in.run(&["--base", "unpushed", "--system", "x86_64-linux"]);

    assert_eq!(ran.status.code(), Some(75), "{}", text(&ran.stderr));
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("origin's main doesn't hold abcdef012345"),
        "{}",
        text(&ran.stderr)
    );
}
