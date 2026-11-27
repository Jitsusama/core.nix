use std::collections::BTreeMap;

use super::ended::built;
use super::telling::is_test;
use super::*;

#[test]
fn a_test_is_told_by_its_folder_or_its_name_alone() {
    for test in [
        "tests/vm/wifi-joins-vm-test.py",
        "tools/changed-checks/tests/changed_checks/main.rs",
        "modules/quickshell/tests/refusal-lines.qml",
        "modules/quickshell/checks/agents.nix",
        "modules/wifid/checks.nix",
        "modules/pi/host-checks.nix",
        "modules/local-sites/ask_checks.nix",
        "modules/agents/src/arrangement/tests.rs",
        "modules/agents/src/main_tests.rs",
        "modules/backlight/saves-while-open-test.sh",
        "modules/sites/web/kit.test.ts",
        "scripts/test_reads.py",
        "tests/lib/vm-helpers-test.py",
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
        "tools/merge-net/src/subtests.rs",
        "tools/changed-checks/src/checks.rs",
    ] {
        assert!(!is_test(code), "{code} was told as a test");
    }
}

#[test]
fn a_change_lays_its_added_and_edited_tests_over_the_base_and_no_other() {
    let said = "A\ttools/x/tests/new.rs\n\
                M\tmodules/y/src/lib.rs\n\
                M\tmodules/y/src/lib_tests.rs\n\
                A\tmodules/y/tests/new/cases.rs\n\
                M\tmodules/y/tests/new/main.rs\n\
                D\tmodules/z/old.rs\n\
                R087\tmodules/q/One.qml\tmodules/q/Two.qml\n\
                M\tmodules/e/src/far/tests.rs\n\
                A\tmodules/e/src/far/tests/leaking.rs\n\
                T\tmodules/y/tests/link.sh\n";

    assert_eq!(
        Change::read(said),
        Change {
            tests: vec![
                "tools/x/tests/new.rs".to_owned(),
                "modules/y/src/lib_tests.rs".to_owned(),
                "modules/y/tests/new/main.rs".to_owned(),
                "modules/e/src/far/tests.rs".to_owned(),
                "modules/y/tests/link.sh".to_owned(),
                "modules/y/tests/new/cases.rs".to_owned(),
                "modules/e/src/far/tests/leaking.rs".to_owned(),
            ],
            apart: Vec::new(),
            code: vec![
                "modules/y/src/lib.rs".to_owned(),
                "modules/z/old.rs".to_owned(),
                "modules/q/One.qml".to_owned(),
                "modules/q/Two.qml".to_owned(),
            ],
        },
        "an added or edited test wasn't laid, one declared by a laid test wasn't laid with \
         it, or code was taken for a test"
    );
}

#[test]
fn a_test_the_head_has_otherwise_than_laid_is_said_apart() {
    let said = "D\tmodules/z/tests/gone.sh\n\
                R100\tmodules/q/tests/a.qml\tmodules/q/tests/b.qml\n\
                R062\tmodules/q/tests/c.qml\tmodules/q/tests/d.qml\n\
                R095\tmodules/q/src/a.rs\tmodules/q/src/b/tests.rs\n\
                A\tmodules/y/src/new/tests.rs\n\
                A\tmodules/lid-hinge/lid-hinge_tests.rs\n\
                A\t\"modules/y/tests/caf\\303\\251.rs\"\n";

    assert_eq!(
        Change::read(said).apart,
        [
            Apart::Removed("modules/z/tests/gone.sh".to_owned()),
            Apart::Moved(
                "modules/q/tests/a.qml".to_owned(),
                "modules/q/tests/b.qml".to_owned()
            ),
            Apart::Moved(
                "modules/q/tests/c.qml".to_owned(),
                "modules/q/tests/d.qml".to_owned()
            ),
            Apart::Moved(
                "modules/q/src/a.rs".to_owned(),
                "modules/q/src/b/tests.rs".to_owned()
            ),
            Apart::Quoted("\"modules/y/tests/caf\\303\\251.rs\"".to_owned()),
            Apart::Declared("modules/y/src/new/tests.rs".to_owned()),
            Apart::Declared("modules/lid-hinge/lid-hinge_tests.rs".to_owned()),
        ],
        "a test removed, moved, quoted or declared only by code wasn't said apart"
    );
}

#[test]
fn new_rust_test_files_are_laid_down_every_level_a_laid_one_declares() {
    let said = "M\tmodules/a/src/x/tests.rs\n\
                A\tmodules/a/src/x/tests/cases.rs\n\
                A\tmodules/a/src/x/tests/cases/more.rs\n\
                M\tmodules/b/src/tests/mod.rs\n\
                A\tmodules/b/src/tests/helpers/mod.rs\n\
                M\tmodules/c/tests/reads.rs\n\
                A\tmodules/c/tests/common/mod.rs\n";

    let change = Change::read(said);

    assert_eq!(
        change.apart,
        Vec::<Apart>::new(),
        "a file declared by a laid one, or a folder's mod.rs, was left out"
    );
    assert_eq!(
        change.tests.len(),
        7,
        "not every test file was laid: {:?}",
        change.tests
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
                +def test_reads(): pass\n\
                diff --git a/src/d.rs b/src/d.rs\n\
                --- a/src/d.rs\n\
                +++ b/src/d.rs\n\
                @@ -5,0 +6,3 @@\n\
                +++ counted\n\
                +    #[tokio::test(start_paused = true)]\n\
                +    async fn waits() {}\n\
                diff --git a/src/e.rs b/src/e.rs\n\
                --- a/src/e.rs\n\
                +++ b/src/e.rs\n\
                @@ -1,0 +2,2 @@\n\
                +// a #[test] here is only named\n\
                +const MARKS: [&str; 1] = [\"#[test]\"];\n\
                diff --git a/docs/testing.md b/docs/testing.md\n\
                --- a/docs/testing.md\n\
                +++ b/docs/testing.md\n\
                @@ -1,0 +2 @@\n\
                +#[cfg(test)] marks a module of tests.\n\
                diff --git a/src/f.rs b/src/f.rs\n\
                --- a/src/f.rs\n\
                +++ b/src/f.rs\n\
                @@ -1,0 +2 @@\n\
                +#[cfg(all(test, target_os = \"linux\"))]\n";

    assert_eq!(
        inline_tests(diff, &BTreeMap::new()),
        ["c.py", "src/a.rs", "src/d.rs", "src/f.rs"],
        "a test added among code went unfound, one taken away or only quoted was found, or \
         a line reading ++ was taken for a file's"
    );
}

#[test]
fn a_line_added_where_a_code_file_s_inline_tests_begin_edits_one() {
    let diff = "diff --git a/src/a.rs b/src/a.rs\n\
                +++ b/src/a.rs\n\
                @@ -3 +3 @@\n\
                -    1\n\
                +    2\n\
                @@ -20,0 +21 @@\n\
                +        assert_eq!(x, 2, \"x\");\n\
                diff --git a/src/b.rs b/src/b.rs\n\
                +++ b/src/b.rs\n\
                @@ -3 +3 @@\n\
                -    1\n\
                +    2\n";
    let begins = BTreeMap::from([("src/a.rs".to_owned(), 12), ("src/b.rs".to_owned(), 12)]);

    assert_eq!(
        inline_tests(diff, &begins),
        ["src/a.rs"],
        "an assert added in an inline test went unfound, or code above one was taken for it"
    );
}

#[test]
fn a_code_file_s_inline_tests_begin_at_its_test_module_or_first_test_function() {
    let rust = "fn a() {}\n\n#[cfg(test)]\n#[expect(x, reason = \"y\")]\nmod tests {\n}\n";
    let declared = "fn a() {}\n#[cfg(test)]\nmod tests;\n";
    let python = "import x\n\nclass T:\n    def test_a(self):\n        pass\n";

    let begins = [
        tests_begin("src/a.rs", rust),
        tests_begin("src/a.rs", declared),
        tests_begin("a.py", python),
        tests_begin("a.sh", rust),
    ];

    assert_eq!(
        begins,
        [Some(3), None, Some(4), None],
        "inline tests weren't found where they begin, or a declared module was taken for them"
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
    let quoted = "a> expect: builder failed with exit code 1\n\
                  error: unable to download 'https://cache/x.narinfo'\n";

    let ended = [
        built(true, ""),
        built(false, failed),
        built(false, lost),
        built(false, quoted),
    ];

    assert_eq!(
        ended,
        [Built::Passed, Built::Red, Built::Unbuilt, Built::Unbuilt],
        "a build was told wrongly, or a builder's own words made a lost one red"
    );
}
