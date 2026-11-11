//! The cases for what a changed-checks run sends, wants and builds, and
//! which tests main's list holds apart, kept beside changed.rs so the
//! file stays one reason long.

use super::*;
use std::path::PathBuf;

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn runs_take_the_two_lent_builders_in_turn() {
    assert_eq!(in_turn(["first", "second"], 4), ["first", "second"]);
    assert_eq!(in_turn(["first", "second"], 7), ["second", "first"]);
}

#[test]
fn evaluators_are_read_a_line_each() {
    let read =
        evaluators("# nearby\n\nfar.local:22 far me,us\nshort:22 short\n near:22  near  me \n");
    assert_eq!(
        read,
        [
            Evaluator {
                address: "far.local:22".to_owned(),
                host: "far".to_owned(),
                owners: vec!["me".to_owned(), "us".to_owned()],
            },
            Evaluator {
                address: "near:22".to_owned(),
                host: "near".to_owned(),
                owners: vec!["me".to_owned()],
            }
        ]
    );
}

#[test]
fn an_evaluator_takes_only_the_owners_it_names() {
    let far = &evaluators("far:22 far me,us\n")[0];
    assert!(far.takes("me/thing"), "an owner it names is taken");
    assert!(far.takes("us/other"), "each owner it names is taken");
    assert!(
        !far.takes("them/thing"),
        "an owner it doesn't name is refused"
    );
    assert!(
        !far.takes("mee/thing"),
        "an owner is matched whole, not by its start"
    );
}

#[test]
fn a_pushed_commit_is_checked_out_from_a_kept_mirror_and_run_by_its_own_changed_checks() {
    let sent = sending(
        Source::Pushed {
            github: "someone/thing".to_owned(),
            commit: "abc123".to_owned(),
            dir: None,
        },
        &words("--build --system x86_64-linux"),
        &[],
        None,
    );
    assert!(
        sent.bundle.is_none(),
        "a pushed commit goes without a bundle"
    );
    for part in [
        "\"$HOME\"/.cache/nix-offload/changed/someone-thing.git",
        "fetch -q https://github.com/someone/thing +refs/heads/main:refs/heads/main abc123",
        "symbolic-ref HEAD refs/heads/main",
        "git clone -q --shared \"$HOME\"/.cache/nix-offload/changed/someone-thing.git \
         /tmp/nix-offload-changed-$$/tree;",
        "checkout -q -B nix-offload-changed abc123",
        "CHANGED_CHECKS_HERE=1 WORK_RUNS_ELSEWHERE=1 nix run .#changed-checks -- \
         --build --system x86_64-linux",
    ] {
        assert!(sent.command.contains(part), "{part} in {}", sent.command);
    }
    assert!(!sent.command.contains("bundle"), "{}", sent.command);
    assert!(
        !sent.command.contains("CHANGED_CHECKS_PASSED"),
        "{}",
        sent.command
    );
}

#[test]
fn a_run_given_its_own_changed_checks_runs_that_and_never_the_commits() {
    let own = "/nix/store/abc-changed-checks-0.1.0/bin/changed-checks";
    let sent = sending(
        Source::Pushed {
            github: "someone/thing".to_owned(),
            commit: "abc123".to_owned(),
            dir: None,
        },
        &words("--build --system x86_64-linux"),
        &[],
        Some(own),
    );
    assert!(
        sent.command.contains(&format!(
            "CHANGED_CHECKS_HERE=1 WORK_RUNS_ELSEWHERE=1 {own} --build --system x86_64-linux"
        )),
        "run by its store path: {}",
        sent.command
    );
    assert!(
        !sent.command.contains("nix run"),
        "nothing the commit names runs: {}",
        sent.command
    );
    assert_eq!(
        package_of(own),
        "/nix/store/abc-changed-checks-0.1.0",
        "what goes up ahead of it"
    );
}

#[test]
fn runs_at_once_take_the_kept_mirror_in_turn() {
    let sent = sending(
        Source::Pushed {
            github: "someone/thing".to_owned(),
            commit: "abc123".to_owned(),
            dir: None,
        },
        &words("--build"),
        &[],
        None,
    );
    let at = |part: &str| {
        sent.command
            .find(part)
            .unwrap_or_else(|| panic!("{part} in {}", sent.command))
    };
    let (taken, given) = (
        at("{ flock 9;"),
        at("} 9> \"$HOME\"/.cache/nix-offload/changed/someone-thing.git.lock;"),
    );
    for part in [
        "git init -q --bare",
        "fetch -q https://github.com",
        "symbolic-ref",
        "git clone",
    ] {
        assert!(
            taken < at(part) && at(part) < given,
            "{part} runs while the mirror's lock is held: {}",
            sent.command
        );
    }
    assert!(
        given < at("checkout -q -B"),
        "the lock is let go before the run's own checkout: {}",
        sent.command
    );
}

#[test]
fn the_far_side_is_told_which_checks_passed_at_the_commit() {
    let sent = sending(
        Source::Pushed {
            github: "someone/thing".to_owned(),
            commit: "abc123".to_owned(),
            dir: None,
        },
        &words("--build"),
        &words("a vm-b"),
        None,
    );
    assert!(
        sent.command.contains(
            "CHANGED_CHECKS_PASSED='abc123 a vm-b' CHANGED_CHECKS_HERE=1 \
             WORK_RUNS_ELSEWHERE=1 nix run .#changed-checks -- --build"
        ),
        "{}",
        sent.command
    );
}

#[test]
fn unpushed_commits_come_as_a_bundle_fetched_into_the_checkout() {
    let sent = sending(
        Source::Bundled {
            github: "someone/thing".to_owned(),
            base: "base1".to_owned(),
            commit: "mine2".to_owned(),
            dir: Some("sub".to_owned()),
            bundle: PathBuf::from("/b"),
            tip: "refs/nix-offload/there-7-0".to_owned(),
        },
        &words("--base 'odd ref'"),
        &[],
        None,
    );
    assert_eq!(sent.bundle, Some(PathBuf::from("/b")));
    for part in [
        "+refs/heads/main:refs/heads/main base1",
        "there.bundle refs/nix-offload/there-7-0:refs/heads/there",
        "checkout -q -B nix-offload-changed mine2",
        "/tree/sub;",
    ] {
        assert!(sent.command.contains(part), "{part} in {}", sent.command);
    }
}

#[test]
fn what_a_run_wants_is_read_from_its_arguments() {
    assert_eq!(
        wanted(&words("--no-vm --build --system aarch64-darwin")),
        Wanted {
            build: true,
            vm: false,
            system: Some("aarch64-darwin".to_owned())
        }
    );
    assert_eq!(
        wanted(&[]),
        Wanted {
            build: true,
            vm: false,
            system: None
        }
    );
    assert_eq!(
        wanted(&words("--build")),
        Wanted {
            build: true,
            vm: true,
            system: None
        }
    );
    assert_eq!(
        wanted(&words("--name-only")),
        Wanted {
            build: false,
            vm: false,
            system: None
        }
    );
}

#[test]
fn those_added_and_changed_are_built_and_vm_tests_only_when_wanted() {
    let printed = "added    a\nchanged  vm-b\nremoved  c\nbroken   d\nchanged  e\n";
    assert_eq!(to_build(printed, true), ["a", "vm-b", "e"]);
    assert_eq!(to_build(printed, false), ["a", "e"]);
}

#[test]
fn a_list_says_why_each_test_it_holds_blocks_nothing() {
    let listed = r#"[{"test": "vm-a", "reason": "failed then passed at abc", "owner_pr": 12},
        {"test": "vm-b", "reason": "flaky", "owner_pr": null}]"#;
    assert_eq!(
        held(listed),
        Ok(vec![
            Held {
                test: "vm-a".to_owned(),
                why: "main quarantines it (failed then passed at abc; #12 owns it)".to_owned(),
            },
            Held {
                test: "vm-b".to_owned(),
                why: "main quarantines it (flaky)".to_owned(),
            },
        ])
    );
    assert!(held("{}").is_err(), "a list that isn't one read as one");
    assert!(
        held(r#"[{"reason": "x"}]"#).is_err(),
        "an entry naming no test read"
    );
}

#[test]
fn a_tree_without_main_holds_nothing() {
    let tree = tempfile::tempdir().expect("a folder for the tree");
    assert_eq!(held_on_main(tree.path()), []);
}

#[test]
fn held_tests_and_their_parts_are_parted_from_those_that_block() {
    let held = [Held {
        test: "vm-a".to_owned(),
        why: "main quarantines it (flaky)".to_owned(),
    }];
    let targets = words("formatting vm-a vm-a.gallery vm-ab");
    let (blocking, alone) = apart(targets, &held);
    assert_eq!(blocking, ["formatting", "vm-ab"]);
    let why = "main quarantines it (flaky)".to_owned();
    assert_eq!(
        alone,
        [
            ("vm-a".to_owned(), why.clone()),
            ("vm-a.gallery".to_owned(), why)
        ]
    );
}

#[test]
fn each_held_test_is_built_alone_and_its_failure_blocks_nothing() {
    let held = [
        ("vm-a".to_owned(), "flaky".to_owned()),
        ("vm-b".to_owned(), "flaky".to_owned()),
    ];
    let mut asked = Vec::new();
    built_alone(&held, &mut |names| {
        asked.push(names.to_vec());
        i32::from(names[0] == "vm-a")
    });
    assert_eq!(asked, [vec!["vm-a".to_owned()], vec!["vm-b".to_owned()]]);
}
