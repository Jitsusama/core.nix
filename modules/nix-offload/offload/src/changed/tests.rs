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
fn the_lent_builder_keeping_the_base_is_asked_first() {
    let lent = ["first", "second"];
    assert_eq!(
        kept_first(lent, [false, true]),
        ["second", "first"],
        "the second keeps it"
    );
    assert_eq!(kept_first(lent, [true, false]), lent, "the first keeps it");
    assert_eq!(kept_first(lent, [true, true]), lent, "both keep it");
    assert_eq!(kept_first(lent, [false, false]), lent, "neither keeps it");
}

#[test]
fn a_builder_is_asked_for_the_file_changed_checks_keeps_a_base_in() {
    assert_eq!(
        keeps_base("abc123", "x86_64-linux").as_deref(),
        Some("test -e \"${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks/abc123-x86_64-linux.json\""),
        "not the file changed-checks keeps a base's checks in"
    );
}

/// Runs `command` in `sh` with `home` as its home and `given` on its
/// standard input, as the builder's shell would, ending as it ends.
fn run_in(home: &Path, command: &str, given: &str) -> bool {
    use std::io::Write;
    let mut running = std::process::Command::new("sh")
        .args(["-c", command])
        .env("HOME", home)
        .env_remove("XDG_CACHE_HOME")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("sh didn't start");
    // A command that refuses ends without reading, so its input's pipe may
    // close before it's written, as it did in a run on 12 November.
    if let Some(mut input) = running.stdin.take()
        && let Err(error) = input.write_all(given.as_bytes())
    {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::BrokenPipe,
            "sh took no input: {error}"
        );
    }
    running.wait().expect("sh didn't end").success()
}

#[test]
fn a_kept_base_is_taken_in_whole_and_never_over_one_kept() {
    let home = tempfile::tempdir().expect("no folder for a home");
    let (read, take) =
        sharing_kept("abc123", "x86_64-linux", "json").expect("a plain base refused");
    let kept = home
        .path()
        .join(".cache/changed-checks/abc123-x86_64-linux.json");
    assert!(
        !run_in(home.path(), &take, ""),
        "an empty copy, as from a builder keeping nothing, was taken in"
    );
    assert!(!kept.exists(), "an empty copy was kept");
    let copying = || {
        fs::read_dir(home.path().join(".cache/changed-checks"))
            .expect("no folder for kept checks")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("copying"))
            .count()
    };
    assert_eq!(copying(), 0, "a copy that failed was left half taken in");
    assert!(
        run_in(home.path(), &take, "{\"one\":1}"),
        "a copy wasn't taken in"
    );
    assert_eq!(
        fs::read_to_string(&kept).ok().as_deref(),
        Some("{\"one\":1}"),
        "the copy isn't what was kept"
    );
    assert!(
        run_in(home.path(), &take, "{\"two\":2}"),
        "a second copy failed"
    );
    assert_eq!(
        fs::read_to_string(&kept).ok().as_deref(),
        Some("{\"one\":1}"),
        "a copy went over what was kept"
    );
    assert_eq!(copying(), 0, "a copy was left half taken in");
    assert!(
        take.contains(".copying.$$"),
        "two gates copying one base at once would write into one file"
    );
    assert_eq!(
        read, "cat \"${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks/abc123-x86_64-linux.json\"",
        "not the file changed-checks keeps a base's checks in"
    );
    let (read, _) = sharing_kept("abc123", "x86_64-linux", "reads.json")
        .expect("what the VM tests read isn't shared");
    assert_eq!(
        read,
        "cat \"${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks/abc123-x86_64-linux.reads.json\"",
        "not the file changed-checks keeps what a base's VM tests read in"
    );
    assert_eq!(
        sharing_kept("abc123", "x86_64-linux", "lock"),
        None,
        "a file changed-checks keeps no record in is shared"
    );
}

#[test]
fn what_each_check_read_alone_is_shared_after_the_rest() {
    assert_eq!(
        KEPT_AS.last(),
        Some(&"alone.reads.json"),
        "a builder keeps what each check read alone at most once in 20 minutes, so lacking it \
         mustn't stop the base's checks and reads, copied before it, from being shared"
    );
    let (read, _) = sharing_kept("abc123", "x86_64-linux", "alone.reads.json")
        .expect("what each check read alone isn't shared");
    assert_eq!(
        read,
        concat!(
            "cat \"${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks/",
            "abc123-x86_64-linux.alone.reads.json\""
        ),
        "not the file changed-checks keeps what each check read alone in"
    );
}

#[test]
fn a_base_or_system_the_shell_would_read_as_more_is_never_asked() {
    assert_eq!(
        keeps_base("abc; rm", "x86_64-linux"),
        None,
        "a base with a ;"
    );
    assert_eq!(keeps_base("", "x86_64-linux"), None, "no base");
    assert_eq!(
        sharing_kept("abc; rm", "x86_64-linux", "json"),
        None,
        "a base with a ; is shared"
    );
    assert_eq!(
        keeps_base("abc", "x86_64-linux$(id)"),
        None,
        "a system with $("
    );
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
        {"test": "vm-b", "reason": "flaky", "owner_pr": null, "subtests": ["test_b"]}]"#;
    assert_eq!(
        held(listed),
        Ok(vec![
            Held {
                test: "vm-a".to_owned(),
                why: "main quarantines it (failed then passed at abc; #12 owns it)".to_owned(),
                subtests: BTreeSet::new(),
            },
            Held {
                test: "vm-b".to_owned(),
                why: "main quarantines it (flaky)".to_owned(),
                subtests: named(&["test_b"]),
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
fn a_tests_entries_are_held_as_one_naming_each_owner() {
    let listed = r#"[{"test": "vm-a", "reason": "r1", "owner_pr": 1, "subtests": ["test_p"]},
        {"test": "vm-b", "reason": "r"},
        {"test": "vm-a", "reason": "r2", "owner_pr": 2, "subtests": ["test_q", "test_r"]}]"#;
    let read = held(listed).expect("it reads");
    assert_eq!(
        read[0],
        Held {
            test: "vm-a".to_owned(),
            why: "main quarantines test_p (r1; #1 owns it) and test_q, test_r (r2; #2 owns it)"
                .to_owned(),
            subtests: named(&["test_p", "test_q", "test_r"]),
        },
        "vm-a's second owner's entry wasn't held with its first"
    );
    assert_eq!(read.len(), 2, "vm-a was held twice");
    let whole =
        held(r#"[{"test": "vm-a", "subtests": ["test_p"]}, {"test": "vm-a", "reason": "r"}]"#)
            .expect("it reads");
    assert_eq!(
        whole[0].subtests,
        BTreeSet::new(),
        "an entry holding the whole test didn't hold it whole"
    );
    assert_eq!(whole[0].why, "main quarantines it (r)");
}

#[test]
fn a_tree_without_main_holds_nothing() {
    let tree = tempfile::tempdir().expect("a folder for the tree");
    assert_eq!(held_on_main(tree.path()), []);
}

#[test]
fn held_tests_and_their_parts_are_parted_from_those_that_block() {
    let held = [whole("vm-a")];
    let targets = words("formatting vm-a vm-a.gallery vm-ab");
    let (blocking, alone) = apart(targets, &held);
    assert_eq!(blocking, ["formatting", "vm-ab"]);
    assert_eq!(
        alone,
        [
            ("vm-a".to_owned(), whole("vm-a")),
            ("vm-a.gallery".to_owned(), whole("vm-a"))
        ]
    );
}

fn whole(test: &str) -> Held {
    Held {
        test: test.to_owned(),
        why: "main quarantines it (flaky)".to_owned(),
        subtests: BTreeSet::new(),
    }
}

fn named(subtests: &[&str]) -> BTreeSet<String> {
    subtests.iter().map(|&subtest| subtest.to_owned()).collect()
}

#[test]
fn each_held_test_is_built_alone_and_a_whole_tests_failure_blocks_nothing() {
    let held = [
        ("vm-a".to_owned(), whole("vm-a")),
        ("vm-b".to_owned(), whole("vm-b")),
    ];
    let mut asked = Vec::new();
    let status = built_alone(&held, &mut |names| {
        asked.push(names.to_vec());
        (i32::from(names[0] == "vm-a"), BTreeSet::new())
    });
    assert_eq!(asked, [vec!["vm-a".to_owned()], vec!["vm-b".to_owned()]]);
    assert_eq!(status, 0, "a whole test's failure blocked");
}

#[test]
fn an_entry_naming_subtests_excuses_only_a_failure_in_them() {
    let held = Held {
        subtests: named(&["test_a", "test_b"]),
        ..whole("vm-a")
    };
    assert!(
        excuses(&held, &named(&["test_a"])),
        "a listed subtest blocked"
    );
    assert!(
        !excuses(&held, &named(&["test_a", "test_c"])),
        "an unlisted subtest was excused"
    );
    assert!(
        !excuses(&held, &BTreeSet::new()),
        "a failure naming no subtest was excused"
    );
    assert!(
        !excuses(&held, &named(&["test_a_more"])),
        "a subtest was excused by a name it begins with"
    );
    let built = built_alone(&[("vm-a".to_owned(), held)], &mut |_| {
        (1, named(&["test_c"]))
    });
    assert_eq!(built, 1, "an unlisted subtest's failure blocked nothing");
}

#[test]
fn a_failed_subtest_is_read_from_its_line_of_the_log() {
    let line = "vm-test-run-vm-a> FAILED vm-pytest-a/a-vm-test.py::test_one[two] - boom";
    assert_eq!(failed_on(line), Some("test_one"));
    assert_eq!(failed_on("vm-test-run-vm-a> 3 passed, 1 FAILED"), None);
    assert_eq!(failed_on("FAILED a.py::helper - boom"), None);
}

#[test]
fn a_build_is_heard_line_by_line_for_the_subtests_it_failed() {
    let said = "echo 'x> FAILED a.py::test_b - boom' >&2; echo ok >&2; exit 3";
    let (mut child, failed) = hearing(Command::new("sh").args(["-c", said])).expect("sh starts");
    let code = child.wait().expect("sh ends").code();
    assert_eq!(code, Some(3), "the build's status was lost");
    assert_eq!(
        failed,
        named(&["test_b"]),
        "the failed subtest wasn't heard"
    );
}
