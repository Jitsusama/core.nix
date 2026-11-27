use super::*;

#[test]
fn a_test_is_told_by_its_folder_or_its_name_alone() {
    for test in [
        "tests/vm/wifi-joins-vm-test.py",
        "tools/changed-checks/tests/changed_checks/main.rs",
        "modules/quickshell/tests/refusal-lines.qml",
        "modules/quickshell/checks/agents.nix",
        "modules/wifid/checks.nix",
        "modules/agents/src/arrangement/tests.rs",
        "modules/agents/src/main_tests.rs",
        "modules/backlight/saves-while-open-test.sh",
        "modules/sites/web/kit.test.ts",
        "scripts/test_reads.py",
    ] {
        assert!(is_test(test), "{test} wasn't told as a test");
    }
    for code in [
        "modules/wifid/daemon/src/moves.rs",
        "hardware/some-laptop/programs/power-profiles-tests-wait.patch",
        "modules/chrome/pack-extension/src/test-key.pem",
        "modules/contests/src/main.rs",
        "flake.nix",
        "check-descriptions.nix",
        "tests/lib/vm_helpers.py",
        "checks.nix",
    ] {
        assert!(!is_test(code), "{code} was told as a test");
    }
}

#[test]
fn a_change_lays_its_added_and_edited_tests_over_the_base_and_no_other() {
    let said = "A\ttools/x/tests/new.rs\n\
                M\tmodules/y/src/lib.rs\n\
                M\tmodules/y/src/lib_tests.rs\n\
                A\tmodules/y/src/new/tests.rs\n\
                A\tmodules/y/tests/new/cases.rs\n\
                D\tmodules/z/tests/gone.sh\n\
                D\tmodules/z/old.rs\n\
                R100\tmodules/q/tests/a.qml\tmodules/q/tests/b.qml\n\
                R087\tmodules/q/One.qml\tmodules/q/Two.qml\n\
                R062\tmodules/q/tests/c.qml\tmodules/q/tests/d.qml\n\
                M\tmodules/e/src/far/tests.rs\n\
                A\tmodules/e/src/far/tests/leaking.rs\n\
                T\tmodules/y/tests/link.sh\n";

    assert_eq!(
        Change::read(said),
        Change {
            tests: vec![
                "tools/x/tests/new.rs".to_owned(),
                "modules/y/src/lib_tests.rs".to_owned(),
                "modules/y/tests/new/cases.rs".to_owned(),
                "modules/q/tests/d.qml".to_owned(),
                "modules/e/src/far/tests.rs".to_owned(),
                "modules/y/tests/link.sh".to_owned(),
                "modules/e/src/far/tests/leaking.rs".to_owned(),
            ],
            moved: vec![(
                "modules/q/tests/a.qml".to_owned(),
                "modules/q/tests/b.qml".to_owned()
            )],
            declared: vec!["modules/y/src/new/tests.rs".to_owned()],
            code: vec![
                "modules/y/src/lib.rs".to_owned(),
                "modules/z/old.rs".to_owned(),
                "modules/q/One.qml".to_owned(),
                "modules/q/Two.qml".to_owned(),
            ],
        },
        "a removed test was laid over the base, one moved whole or one only new code declares \
         wasn't said apart, one moved with lines or declared by a laid test wasn't laid, or \
         code was taken for a test"
    );
}

#[test]
fn a_test_added_among_code_is_found_by_its_added_lines() {
    let diff = "diff --git a/src/a.rs b/src/a.rs\n\
                --- a/src/a.rs\n\
                +++ b/src/a.rs\n\
                @@ -1,0 +2,2 @@\n\
                +#[test]\n\
                +fn holds() {}\n\
                diff --git a/src/b.rs b/src/b.rs\n\
                --- a/src/b.rs\n\
                +++ b/src/b.rs\n\
                @@ -3 +3 @@\n\
                -#[cfg(test)]\n\
                +// no tests here\n\
                diff --git a/c.py b/c.py\n\
                --- /dev/null\n\
                +++ b/c.py\n\
                @@ -0,0 +1 @@\n\
                +def test_reads(): pass\n";

    assert_eq!(
        inline_tests(diff),
        ["c.py", "src/a.rs"],
        "a test added among code went unfound, or one taken away was found"
    );
}

#[test]
fn a_change_is_excused_by_a_trailer_saying_why_or_by_reverting() {
    let commit = |subject: &str, trailers: &str| format!("{subject}\x1f{trailers}\x1e\n");

    let pinned = commit("fix(a): b", "") + &commit("test(a): pin c", "none, c was right\n");
    assert_eq!(
        excused(&pinned),
        (Some(Excused::Pinned("c was right".to_owned())), vec![]),
        "a trailer saying why didn't excuse the change"
    );

    let unread = commit("test(a): pin c", "none\nnot this one\n");
    assert_eq!(
        excused(&unread),
        (None, vec!["none".to_owned(), "not this one".to_owned()]),
        "a trailer saying no why excused the change, or wasn't said"
    );

    let reverted =
        commit("revert: undo #7, which breaks a check", "") + &commit("Revert \"feat(a): b\"", "");
    assert_eq!(excused(&reverted), (Some(Excused::Reverts), vec![]));

    let partly = commit("revert: undo #7", "") + &commit("feat(a): b", "");
    assert_eq!(
        excused(&partly),
        (None, vec![]),
        "a change that only partly reverts was excused"
    );
    assert_eq!(excused(""), (None, vec![]), "no commits excused the change");
}

#[test]
fn a_check_over_the_base_is_judged_by_its_derivation_beside_both_sides() {
    let derivations = |pairs: &[(&str, Option<&str>)]| -> Derivations {
        pairs
            .iter()
            .map(|(name, path)| ((*name).to_owned(), path.map(str::to_owned)))
            .collect()
    };
    let before = derivations(&[("a", Some("a")), ("b", Some("b")), ("c", Some("c"))]);
    let after = derivations(&[
        ("a", Some("a2")),
        ("b", Some("b2")),
        ("c", Some("c2")),
        ("d", Some("d")),
        ("e", Some("e")),
    ]);
    let laid = derivations(&[
        ("a", Some("a")),
        ("b", Some("b2")),
        ("c", Some("c3")),
        ("e", None),
    ]);

    let judged: Vec<Over> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|name| over(name, &before, &after, &laid))
        .collect();

    assert_eq!(
        judged,
        [
            Over::Unreached,
            Over::OnlyTests,
            Over::Moved,
            Over::Undefined,
            Over::Unevaluated
        ]
    );
}

#[test]
fn a_test_file_the_change_only_takes_lines_from_adds_no_test() {
    let said =
        "3\t1\tmodules/a/tests/new.rs\n0\t4\tmodules/a/tests/old.rs\n-\t-\tmodules/a/tests/x.png\n";

    let added = adding(said);

    assert_eq!(
        added.into_iter().collect::<Vec<_>>(),
        ["modules/a/tests/new.rs", "modules/a/tests/x.png"]
    );
}

#[test]
fn only_a_builder_failing_a_build_over_the_base_makes_it_red() {
    let failed = "error: Cannot build '/nix/store/x-a.drv'.\n       Reason: builder failed with \
                  exit code 1.\n";
    let lost = "error: unable to download 'https://cache/x.narinfo': Couldn't connect\n";

    let ended = [built(true, ""), built(false, failed), built(false, lost)];

    assert_eq!(ended, [Built::Passed, Built::Red, Built::Unbuilt]);
}
