//! Whose a red candidate's failed checks are, read from its gate's words.

use serde_json::Value;

use super::*;

/// What a builder says when `name` fails, as nix's `-L` prefixes it.
fn broke(name: &str) -> String {
    format!("{name}> error: {name} broke after 1799ms\n")
}

/// A gate's words, as changed-checks and nix say them, naming `built` as
/// changed and each of `failed` a derivation whose builder failed.
pub(super) fn words(built: &[&str], failed: &[&str]) -> String {
    let rows = built.iter().map(|check| format!("changed  {check}\n"));
    let errors = failed.iter().map(|name| {
        format!(
            "{}error: Cannot build '/nix/store/2pglmw1knj57yvg0rm4w-{name}.drv'.\n       \
             Reason: builder failed with exit code 1.\n",
            broke(name)
        )
    });
    std::iter::once("kind     check\n".to_owned())
        .chain(rows)
        .chain(errors)
        .chain(["\u{2715} a check the change touched failed\n".to_owned()])
        .collect()
}

/// The words of a build at the base that fails `name` as the candidate did.
fn as_before(name: &str) -> (Gated, String) {
    (Gated::Red, words(&[], &[name]))
}

/// A judging whose names are none, whose builds at the base and again at
/// the candidate end as `at_base` and `again` say, whose build of every
/// check passes, and whose ride excused each check's reds `excused` times
/// already.
fn judged(
    words: &str,
    at_base: &dyn Fn(&str) -> (Gated, String),
    again: &dyn Fn(&str) -> Gated,
    excused: usize,
) -> Result<Vec<(String, Blame)>, Declined> {
    judged_whole(words, at_base, again, &|_| Gated::Passed, excused)
}

/// `judged`, its build of every check ending as `whole` says.
pub(super) fn judged_whole(
    words: &str,
    at_base: &dyn Fn(&str) -> (Gated, String),
    again: &dyn Fn(&str) -> Gated,
    whole: &dyn Fn(&[String]) -> Gated,
    excused: usize,
) -> Result<Vec<(String, Blame)>, Declined> {
    let mut base_build = |check: &str| at_base(check);
    let mut built_again = |check: &str| again(check);
    let mut built_whole = |checks: &[String]| whole(checks);
    let mut judging = Judging {
        named: &|_| BTreeMap::new(),
        at_base: &mut base_build,
        again: &mut built_again,
        whole: &mut built_whole,
        excused_before: &|_| excused,
        rules_moved: false,
        held: &BTreeSet::new(),
    };
    judge(words, &mut judging)
}

/// A check that reports every failure, so its red can be main's.
const RULE: &str = "every-long-file-says-why";

/// A build that passed.
pub(super) fn passes(_: &str) -> Gated {
    Gated::Passed
}

/// A build at the base that passed.
fn passes_at_base(_: &str) -> (Gated, String) {
    (Gated::Passed, String::new())
}

#[test]
fn a_failed_derivation_is_named_once_and_one_a_dependency_stopped_not_at_all() {
    let said = "error: Cannot build '/nix/store/aaaa-the-margin-cat-on-niri.drv'.\n\
                \x20      Reason: builder failed with exit code 1.\n\
                error: Cannot build '/nix/store/bbbb-home-manager-path.drv'.\n\
                \x20      Reason: 1 dependency failed.\n\
                error: Cannot build '/nix/store/aaaa-the-margin-cat-on-niri.drv'.\n\
                \x20      Reason: builder failed with exit code 1.\n";
    assert_eq!(
        failed_derivations(said),
        Some(vec!["the-margin-cat-on-niri".to_owned()]),
        "a derivation was named twice, or one a dependency stopped was named"
    );
    let timed_out = "error: Cannot build '/nix/store/cccc-vm-pi.drv'.\n\
                     \x20      Reason: builder timed out after 3600 seconds.\n";
    assert_eq!(
        failed_derivations(&(said.to_owned() + timed_out)),
        None,
        "a derivation that failed some other way was passed over"
    );
    let listed = "error: build of '/nix/store/aaaa-the-margin-cat-on-niri.drv^*', \
                  '/nix/store/bbbb-home-manager-path.drv^*' failed\n";
    assert_eq!(
        failed_derivations(&(said.to_owned() + listed)),
        Some(vec!["the-margin-cat-on-niri".to_owned()]),
        "nix's closing list of builds each failing on a line of its own was refused"
    );
}

/// nix exits 1 for a builder lost or a copy that failed as for a failed
/// build, so a flake beside one would land a check that never ran.
#[test]
fn a_red_with_a_build_no_line_says_failed_is_never_judged() {
    let flaky = words(&["a", "b"], &["a"]);
    let unlisted = flaky.clone()
        + "error: build of '/nix/store/aaaa-a.drv^*', '/nix/store/bbbb-b.drv^*' failed\n";
    assert_eq!(
        judged(&unlisted, &passes_at_base, &passes, 0),
        Err(Declined::Unexplained),
        "a flake excused a red whose other failed build nothing said"
    );
    let lost = flaky + "error: unexpected end of file\n";
    assert_eq!(
        judged(&lost, &passes_at_base, &passes, 0),
        Err(Declined::Unexplained),
        "a flake excused a red beside an error that wasn't a build's"
    );
}

#[test]
fn a_failed_derivation_is_placed_by_its_name_its_vm_run_or_what_nix_names_it() {
    let built: Vec<String> = ["formatting", "vm-pi", "a-theme-changes-everything"]
        .map(str::to_owned)
        .to_vec();
    let failed: Vec<String> = [
        "a-theme-changes-everything",
        "vm-test-run-vm-pi",
        "treefmt-check",
    ]
    .map(str::to_owned)
    .to_vec();
    let asked = std::cell::RefCell::new(Vec::new());
    let named = |checks: &[String]| {
        asked.borrow_mut().extend(checks.iter().cloned());
        BTreeMap::from([("treefmt-check".to_owned(), "formatting".to_owned())])
    };
    assert_eq!(
        checks_of(&failed, &built, &named),
        Some(
            ["a-theme-changes-everything", "vm-pi", "formatting"]
                .map(str::to_owned)
                .to_vec()
        )
    );
    assert_eq!(
        *asked.borrow(),
        ["formatting"],
        "nix was asked the names of checks already placed"
    );
    let unnamed = |_: &[String]| BTreeMap::new();
    assert_eq!(
        checks_of(&failed, &built, &unnamed),
        None,
        "a failed derivation no check names was passed over, so the riders were excused for it"
    );
}

#[test]
fn a_rider_whose_unrelated_check_fails_once_stays_and_one_whose_own_fails_is_split() {
    let said = words(&["the-margin-cat-on-niri"], &["the-margin-cat-on-niri"]);
    let unrelated = judged(&said, &passes_at_base, &passes, 0).unwrap();
    assert_eq!(
        unrelated,
        [("the-margin-cat-on-niri".to_owned(), Blame::Flaky)]
    );
    assert!(
        excused(&unrelated),
        "a check that passed when built again split the train"
    );
    let own = judged(&said, &passes_at_base, &|_| Gated::Red, 0).unwrap();
    assert_eq!(own, [("the-margin-cat-on-niri".to_owned(), Blame::Riders)]);
    assert!(
        !excused(&own),
        "a check red twice at the candidate was excused for passing at the base"
    );
    let untaken = judged(&said, &passes_at_base, &|_| Gated::Unsent, 0).unwrap();
    assert_eq!(
        untaken,
        [("the-margin-cat-on-niri".to_owned(), Blame::Untaken)],
        "a build again no builder took read as a rider's break"
    );
    assert!(
        !excused(&untaken),
        "a check no builder built again was excused"
    );
}

#[test]
fn a_check_excused_as_a_flake_twice_in_a_ride_is_the_riders_on_its_third_red() {
    let said = words(&["vm-pi"], &["vm-test-run-vm-pi"]);
    let built_again = std::cell::Cell::new(false);
    let again = |_: &str| {
        built_again.set(true);
        Gated::Passed
    };
    let built_at_base = std::cell::Cell::new(false);
    let at_base = |check: &str| {
        built_at_base.set(true);
        passes_at_base(check)
    };
    assert_eq!(
        judged(&said, &at_base, &again, 1).unwrap(),
        [("vm-pi".to_owned(), Blame::Flaky)]
    );
    built_again.set(false);
    built_at_base.set(false);
    assert_eq!(
        judged(&said, &at_base, &again, MOST_EXCUSED).unwrap(),
        [("vm-pi".to_owned(), Blame::Spent)],
        "a check's third red in a ride was excused as a flake"
    );
    assert!(!built_again.get(), "a check out of excuses was built again");
    assert!(
        !built_at_base.get(),
        "a check out of excuses, whose red at the base can't be main's, was built there"
    );
}

/// A rule's red at the base can still be main's however often the ride
/// excused it as a flake, so it is still built there.
#[test]
fn a_rule_out_of_excuses_can_still_be_mains() {
    let said = words(&[RULE], &[RULE]);
    assert_eq!(
        judged(&said, &as_before, &passes, MOST_EXCUSED).unwrap(),
        [(RULE.to_owned(), Blame::Main)],
        "main's own red in a rule out of flake excuses wasn't main's"
    );
    assert_eq!(
        judged(&said, &passes_at_base, &passes, MOST_EXCUSED).unwrap(),
        [(RULE.to_owned(), Blame::Spent)],
        "a rule out of excuses, passing at the base, was excused as a flake"
    );
}

#[test]
fn a_check_failing_at_the_base_as_it_failed_is_mains_and_one_failing_anew_the_riders() {
    let said = words(&[RULE], &[RULE]);
    let main = judged(&said, &as_before, &passes, 0).unwrap();
    assert_eq!(
        main,
        [(RULE.to_owned(), Blame::Main)],
        "main's own red in a rule that reports every failure wasn't main's"
    );
    assert!(excused(&main), "main's own red split the train");
    let anew = said.clone() + &format!("{RULE}> error: Ours.js has 600 lines\n");
    assert_eq!(
        judged(&anew, &as_before, &passes, 0).unwrap(),
        [(RULE.to_owned(), Blame::Riders)],
        "a second break of a check main fails hid behind main's"
    );
    let untaken = judged(&said, &|_| (Gated::Unsent, String::new()), &passes, 0).unwrap();
    assert_eq!(
        untaken,
        [(RULE.to_owned(), Blame::Untaken)],
        "a build at the base no builder took excused the riders"
    );
    assert!(
        !excused(&untaken),
        "a build at the base no builder took excused the riders"
    );
}

/// The candidate builds its own copy of a rule, so a rider's rule that
/// stops early prints main's lines over a break of its own.
#[test]
fn mains_red_in_a_rule_the_riders_moved_is_theirs() {
    let said = words(&[RULE], &[RULE]);
    let mut base_build = as_before;
    let mut built_again = passes;
    let mut built_whole = |_: &[String]| Gated::Passed;
    let mut judging = Judging {
        named: &|_| BTreeMap::new(),
        at_base: &mut base_build,
        again: &mut built_again,
        whole: &mut built_whole,
        excused_before: &|_| 0,
        rules_moved: true,
        held: &BTreeSet::new(),
    };
    assert_eq!(
        judge(&said, &mut judging).unwrap(),
        [(RULE.to_owned(), Blame::RuleMoved)],
        "main's red in a rule the riders moved was excused as main's"
    );
    let moved = |files: &[&str]| {
        let files: Vec<String> = files.iter().map(|file| (*file).to_owned()).collect();
        moves_a_rule(Some(&files))
    };
    assert!(
        moved(&["docs/a.md", "tests/rules/every-suppression-says-why.py"]),
        "a rule's script moved unseen"
    );
    assert!(
        moved(&["tests/rules/every-long-file-says-why-waiting.nix"]),
        "a rule's waiting list moved unseen"
    );
    assert!(
        moved(&["flake.nix"]),
        "the flake wiring the rules moved unseen"
    );
    assert!(
        moved(&["flake.lock"]),
        "the lock giving the rules Python moved unseen"
    );
    assert!(
        moved(&["checks.nix"]),
        "the root checks.nix, handing each rule its pkgs, moved unseen"
    );
    assert!(
        !moved(&[
            "modules/a/b.nix",
            "modules/a/checks.nix",
            "tests/vm/rules-vm-test.py"
        ]),
        "files no rule runs were read as a rule's"
    );
    assert!(
        moves_a_rule(None),
        "files git couldn't read were read as moving no rule"
    );
}

/// The files moved between two commits are read whole from git, a name
/// with an unusual byte too, which git would otherwise quote.
#[test]
fn the_files_moved_between_two_commits_are_read_unquoted() {
    let git = super::super::tests::git_in;
    let repo = std::env::temp_dir().join(format!("merge-net-moved-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    let base = super::super::tests::based(&repo);
    fs::create_dir_all(repo.join("tests/rules")).unwrap();
    fs::write(repo.join("tests/rules/r\u{e9}gle.py"), "").unwrap();
    fs::write(repo.join("shared"), "moved\n").unwrap();
    git(&repo, &["add", "-A"]).unwrap();
    git(&repo, &["commit", "-q", "-m", "moves"]).unwrap();
    let candidate = git(&repo, &["rev-parse", "HEAD"]).unwrap();
    let moved = moved_between(&repo, &base, &candidate);
    assert_eq!(
        moved.as_deref(),
        Some(&["shared".to_owned(), "tests/rules/r\u{e9}gle.py".to_owned()][..]),
        "the files moved weren't read as git has them"
    );
    assert!(
        moves_a_rule(moved.as_deref()),
        "a rule's file with an unusual name moved unseen"
    );
    assert_eq!(
        moved_between(&repo, &base, &"0".repeat(40)),
        None,
        "a commit git hasn't was read as moving nothing"
    );
    fs::remove_dir_all(&repo).unwrap();
}

/// treefmt stops at the first file no formatter reads, so main's
/// formatting red may hide a rider's.
#[test]
fn mains_formatting_red_is_never_mains() {
    let said = words(&["formatting"], &["treefmt-check"]);
    let mut base_build = |_: &str| as_before("treefmt-check");
    let mut built_again = passes;
    let mut built_whole = |_: &[String]| Gated::Passed;
    let mut judging = Judging {
        named: &|_| BTreeMap::from([("treefmt-check".to_owned(), "formatting".to_owned())]),
        at_base: &mut base_build,
        again: &mut built_again,
        whole: &mut built_whole,
        excused_before: &|_| 0,
        rules_moved: false,
        held: &BTreeSet::new(),
    };
    assert_eq!(
        judge(&said, &mut judging).unwrap(),
        [("formatting".to_owned(), Blame::Masked)],
        "main's formatting red was excused, though treefmt may stop before a rider's"
    );
}

/// A nix killed mid-run says nothing of the checks it never reached, and a
/// failed derivation named like a check may have been another's.
#[test]
fn a_red_lands_only_once_every_other_check_it_built_builds() {
    let said = words(&[RULE, "a", "vm-desktop", "never-reached"], &[RULE, "a"]);
    let at_base = |check: &str| {
        if check == RULE {
            as_before(RULE)
        } else {
            (Gated::Passed, String::new())
        }
    };
    let asked = std::cell::RefCell::new(Vec::new());
    let whole = |checks: &[String]| {
        asked.borrow_mut().extend_from_slice(checks);
        Gated::Red
    };
    assert_eq!(
        judged_whole(&said, &at_base, &passes, &whole, 0),
        Err(Declined::WholeRed),
        "a red landed though not every check it built was known built"
    );
    assert_eq!(
        *asked.borrow(),
        ["a", "never-reached"],
        "every check the gate built but main's own red wasn't built once more, \
         or a VM test it never built was"
    );
    assert!(
        judged_whole(&said, &at_base, &passes, &|_| Gated::Passed, 0)
            .is_ok_and(|judged| excused(&judged)),
        "a red whose every other check built wasn't excused"
    );
}

/// A check named with a dot reaches Nix as another check's path, so a
/// rider's failing `formatting.out` would be rebuilt as `formatting`.
#[test]
fn a_check_named_with_a_dot_is_never_judged() {
    let said = words(&["formatting.out"], &["formatting.out"]);
    assert_eq!(
        judged(&said, &passes_at_base, &passes, 0),
        Err(Declined::NotPlain),
        "a check named with a dot was rebuilt as another and excused"
    );
}

/// The words that say a gate ran again on its own base cut what came
/// before them, so a rider's build printing them could hide its break.
#[test]
fn a_red_whose_gate_ran_again_is_never_judged() {
    let said = format!(
        "{}nix-offload: abc failed, so the change runs again on its own base\n{}",
        broke(RULE),
        words(&["a"], &["a"])
    );
    assert_eq!(
        judged(&said, &passes_at_base, &passes, 0),
        Err(Declined::RanAgain),
        "a red beside a replay's words was judged"
    );
}

/// A rule that crashed said nothing of what it never reached.
#[test]
fn a_rule_that_crashed_at_the_base_too_is_never_mains() {
    let crash = format!(
        "{RULE}> Traceback (most recent call last):\n\
         {RULE}> UnicodeDecodeError: 'utf-8' codec can't decode byte 0xff\n"
    );
    let said = crash.clone() + &words(&[RULE], &[RULE]);
    let at_base = |_: &str| (Gated::Red, crash.clone() + &words(&[], &[RULE]));
    assert_eq!(
        judged(&said, &at_base, &passes, 0).unwrap(),
        [(RULE.to_owned(), Blame::Riders)],
        "a rule's crash, alike at the base, was read as main's"
    );
}

/// changed-checks says `✕` of its own when it gives up before its builds
/// end, and its verdict on a failed build is what shows they did.
#[test]
fn a_red_without_changed_checks_verdict_on_its_builds_is_never_judged() {
    let flaky = words(&["a"], &["a"]);
    let gave_up = flaky.clone() + "\u{2715} nix eval of the VM tests failed: signal: 9\n";
    assert_eq!(
        judged(&gave_up, &passes_at_base, &passes, 0),
        Err(Declined::Unexplained),
        "a flake excused a red changed-checks gave up on"
    );
    let unsaid = flaky.replace("\u{2715} a check the change touched failed\n", "");
    assert_eq!(
        judged(&unsaid, &passes_at_base, &passes, 0),
        Err(Declined::NoVerdict),
        "a flake excused a red changed-checks never said was a failed build"
    );
    let traced = words(&["a"], &["a"]) + "error:\n       \u{2026} while evaluating\n";
    assert_eq!(
        judged(&traced, &passes_at_base, &passes, 0),
        Err(Declined::Unexplained),
        "a flake excused a red beside a traced nix error"
    );
}

#[test]
fn a_check_that_may_stop_at_its_first_error_is_never_mains() {
    let said = words(&["vm-pi"], &["vm-test-run-vm-pi"]);
    let at_base = |_: &str| as_before("vm-test-run-vm-pi");
    assert_eq!(
        judged(&said, &at_base, &passes, 0).unwrap(),
        [("vm-pi".to_owned(), Blame::Masked)],
        "a check not known to report every failure was excused as main's"
    );
}

#[test]
fn the_same_failure_reads_alike_whatever_its_times_and_store_paths() {
    let candidate = "formatting> error: /nix/store/aaaa-source/Presented.js has 503 lines\n\
                     formatting> Checked 300 files in 12ms, then 2.5s\n";
    let base = "formatting> error: /nix/store/bbbb-source/Presented.js has 503 lines\n\
                formatting> Checked 300 files in 1799ms, then 0.4s\n";
    assert!(
        fails_as_at_base(candidate, base, "formatting"),
        "the same failure, at other times and store paths, read as a new one"
    );
    assert!(
        !fails_as_at_base("", base, "formatting"),
        "a failure whose builder said nothing was read as main's"
    );
    assert!(
        !fails_as_at_base("formatting> error: Other.js\n", base, "formatting"),
        "a failure the base didn't say was read as main's"
    );
    assert!(
        !fails_as_at_base(
            "formatting> error: Presented.js has 601 lines\n",
            "formatting> error: Presented.js has 503 lines\n",
            "formatting"
        ),
        "a failure differing from main's only by a number was read as main's"
    );
    assert_eq!(
        without_durations("vm2 ran 3 subtests in 41ms, 1.5s and 2 m"),
        "vm2 ran 3 subtests in #ms, #s and 2 m",
        "a number other than a duration's was taken out, or a duration kept"
    );
}

#[test]
fn a_ride_counts_each_flake_its_excused_reds_hold() {
    let mut flakes = Flakes::default();
    let judged = [
        ("vm-pi".to_owned(), Blame::Flaky),
        (RULE.to_owned(), Blame::Main),
    ];
    assert_eq!(flakes.count(&judged), ["vm-pi"]);
    flakes.count(&judged);
    assert_eq!(flakes.of("vm-pi"), MOST_EXCUSED, "a flake went uncounted");
    assert_eq!(flakes.of(RULE), 0, "main's red was counted a flake");
}

#[test]
fn a_red_is_excused_only_when_none_of_its_failures_is_the_riders() {
    let said = words(&[RULE, "vm-pi"], &[RULE, "vm-test-run-vm-pi"]);
    let at_base = |check: &str| {
        if check == RULE {
            as_before(RULE)
        } else {
            (Gated::Passed, String::new())
        }
    };
    let judged = judged(&said, &at_base, &|_| Gated::Red, 0).unwrap();
    assert_eq!(
        judged,
        [
            (RULE.to_owned(), Blame::Main),
            ("vm-pi".to_owned(), Blame::Riders),
        ]
    );
    assert!(
        !excused(&judged),
        "main's red beside the riders' own excused the riders'"
    );
    assert!(!excused(&[]), "a red naming no failed check was excused");
}

#[test]
fn once_a_failure_is_the_riders_nothing_after_it_is_built_again() {
    let said = words(&[RULE, "b", "c"], &[RULE, "b", "c"]);
    let asked = std::cell::RefCell::new(Vec::new());
    let at_base = |check: &str| {
        asked.borrow_mut().push(check.to_owned());
        if check == RULE {
            as_before(RULE)
        } else {
            (Gated::Passed, String::new())
        }
    };
    let judged = judged(&said, &at_base, &|_| Gated::Red, 0).unwrap();
    assert_eq!(
        judged.iter().map(|(_, blamed)| *blamed).collect::<Vec<_>>(),
        [Blame::Main, Blame::Riders, Blame::Unbuilt]
    );
    assert_eq!(
        *asked.borrow(),
        [RULE, "b"],
        "a check was built again once the red was the riders'"
    );
}

#[test]
fn a_red_of_more_checks_than_are_rebuilt_is_the_riders_unbuilt() {
    let many = ["a", "b", "c", "d", "e"];
    assert!(
        many.len() > MOST_REBUILT,
        "the test's red fails no more checks than are rebuilt"
    );
    let built = std::cell::Cell::new(false);
    let at_base = |check: &str| {
        built.set(true);
        as_before(check)
    };
    assert_eq!(
        judged(&words(&many, &many), &at_base, &passes, 0),
        Err(Declined::TooMany)
    );
    assert!(
        !built.get(),
        "a red across main was built again check by check"
    );
}

/// `MOST_REBUILT` failed checks are still each built again, so the limit
/// holds on both sides.
#[test]
fn a_red_of_as_many_checks_as_are_rebuilt_is_judged_check_by_check() {
    let most = ["a", "b", "c", "d"];
    assert_eq!(
        most.len(),
        MOST_REBUILT,
        "the test's red fails other than as many checks as are rebuilt"
    );
    let asked = std::cell::RefCell::new(Vec::new());
    let again = |check: &str| {
        asked.borrow_mut().push(check.to_owned());
        Gated::Passed
    };
    let judged = judged(&words(&most, &most), &passes_at_base, &again, 0).unwrap();
    assert!(
        excused(&judged),
        "a red of as many flakes as are rebuilt split the train"
    );
    assert_eq!(
        *asked.borrow(),
        most,
        "a check of a red of as many as are rebuilt wasn't built again"
    );
}

#[test]
fn a_check_is_red_at_the_base_only_where_a_builder_failed_there() {
    assert_eq!(at_base(Some(0), ""), Gated::Passed);
    assert_eq!(at_base(Some(1), &words(&[], &["formatting"])), Gated::Red);
    let missing = "error: flake 'github:someone/core/1234' does not provide attribute \
                   'checks.x86_64-linux.vm-new'\n";
    assert_eq!(
        at_base(Some(1), missing),
        Gated::Unsent,
        "a check the riders added, missing at the base, was blamed on main"
    );
    assert_eq!(at_base(Some(75), ""), Gated::Unsent);
}

#[test]
fn words_naming_no_failed_derivation_say_nothing_of_whose_the_red_is() {
    let said = words(&["vm-pi"], &[]);
    assert_eq!(
        judged(&said, &passes_at_base, &passes, 0),
        Err(Declined::Unexplained)
    );
    let flaky = words(&["vm-pi"], &["vm-test-run-vm-pi"]);
    let unbuilt = words(&[], &[]) + "\u{2715} a check the change touched no longer evaluates\n";
    assert_eq!(
        judged(&unbuilt, &passes_at_base, &passes, 0),
        Err(Declined::NoLongerEvaluates),
        "changed-checks' own words that a check no longer evaluates were read as unexplained"
    );
    let unmarked = flaky + "a check the change touched no longer evaluates\n";
    assert_eq!(
        judged(&unmarked, &passes_at_base, &passes, 0),
        Err(Declined::NoLongerEvaluates),
        "a check that no longer evaluates was passed over beside a flake"
    );
}

/// Lent builders that keep each build asked of them, as (flake, checks,
/// commit), and pass each.
#[derive(Default)]
struct Asked(Vec<(String, Vec<String>, String)>);

impl Lent for Asked {
    fn build(
        &mut self,
        _: &Path,
        flake: &str,
        _: &Path,
        checks: &[&str],
        commit: &str,
    ) -> (Option<i32>, String) {
        let checks = checks.iter().map(|check| (*check).to_owned()).collect();
        self.0.push((flake.to_owned(), checks, commit.to_owned()));
        (Some(0), String::new())
    }

    fn named(&self, _: &Path, _: &str, _: &[String]) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

/// Which commit each build is at is the whole of the scheme's safety: a
/// rider's red rebuilt at the base would pass, and land as a flake.
#[test]
fn a_flake_is_built_at_the_base_then_at_the_candidate_and_its_finding_names_the_red() {
    let root = std::env::temp_dir().join(format!("merge-net-excuses-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let (base, candidate) = ("b".repeat(40), "c".repeat(40));
    let log = super::super::folder(&root).join(format!("{candidate}.log"));
    fs::create_dir_all(super::super::folder(&root)).unwrap();
    State::at(root.clone()).make().unwrap();
    fs::write(&log, words(&["a", "other"], &["a"])).unwrap();
    let red = Red {
        checkout: &root,
        base: &base,
        candidate: &candidate,
        log: &log,
    };
    let mut asked = Asked::default();
    let landed = excuses(
        &root,
        "https://github.com/someone/core.git",
        &red,
        &mut Flakes::default(),
        &mut asked,
    );
    assert!(
        landed,
        "a flake, passing at the base and again, split the train"
    );
    let at = |flake: &str, checks: &[&str], commit: &str| {
        (
            flake.to_owned(),
            checks
                .iter()
                .map(|check| (*check).to_owned())
                .collect::<Vec<_>>(),
            commit.to_owned(),
        )
    };
    assert_eq!(
        asked.0,
        [
            at(&format!("github:someone/core/{base}"), &["a"], &base),
            at(".", &["a"], &candidate),
            at(".", &["a", "other"], &candidate),
        ],
        "a build wasn't at the commit its judging needs"
    );
    let findings: Vec<Value> = fs::read_dir(root.join("findings"))
        .unwrap()
        .map(|entry| serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap())
        .collect();
    assert_eq!(findings.len(), 1, "a flake's finding wasn't written once");
    assert_eq!(findings[0]["kind"], "train-flaky");
    assert_eq!(
        findings[0]["log"],
        log.display().to_string(),
        "a flake's finding didn't name the gate's log, which holds its red"
    );
    assert_eq!(
        findings[0]["rebuilt_log"],
        built_log(&root, &candidate, "a", "again")
            .display()
            .to_string(),
        "a flake's finding didn't name the rebuild that passed"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn only_a_plain_check_name_goes_into_nix() {
    assert!(
        plainly_named("the-margin-cat-on-niri"),
        "a check's name was refused"
    );
    assert!(plainly_named("vm_pi'"), "a name Nix allows was refused");
    assert!(
        !plainly_named("x${builtins.abort \"\"}"),
        "a name splicing an expression went into nix"
    );
    assert!(!plainly_named(""), "an empty name went into nix");
}
