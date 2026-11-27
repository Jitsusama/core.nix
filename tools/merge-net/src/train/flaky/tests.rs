//! Whose a red candidate's failed checks are, read from its gate's words.

use super::*;

/// What a builder says when `name` fails, as nix's `-L` prefixes it.
fn broke(name: &str) -> String {
    format!("{name}> error: {name} broke after 1799ms\n")
}

/// A gate's words, as changed-checks and nix say them, naming `built` as
/// changed and each of `failed` a derivation whose builder failed.
fn words(built: &[&str], failed: &[&str]) -> String {
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
) -> Option<Vec<(String, Blame)>> {
    judged_whole(words, at_base, again, &|_| Gated::Passed, excused)
}

/// `judged`, its build of every check ending as `whole` says.
fn judged_whole(
    words: &str,
    at_base: &dyn Fn(&str) -> (Gated, String),
    again: &dyn Fn(&str) -> Gated,
    whole: &dyn Fn(&[String]) -> Gated,
    excused: usize,
) -> Option<Vec<(String, Blame)>> {
    let mut base_build = |check: &str| at_base(check);
    let mut built_again = |check: &str| again(check);
    let mut built_whole = |checks: &[String]| whole(checks);
    let mut judging = Judging {
        named: &|_| BTreeMap::new(),
        at_base: &mut base_build,
        again: &mut built_again,
        whole: &mut built_whole,
        excused_before: &|_| excused,
    };
    judge(words, &mut judging)
}

/// A check that reports every failure, so its red can be main's.
const RULE: &str = "every-long-file-says-why";

/// A build that passed.
fn passes(_: &str) -> Gated {
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
        None,
        "a flake excused a red whose other failed build nothing said"
    );
    let lost = flaky + "error: unexpected end of file\n";
    assert_eq!(
        judged(&lost, &passes_at_base, &passes, 0),
        None,
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
}

#[test]
fn a_check_excused_as_a_flake_twice_in_a_ride_is_the_riders_on_its_third_red() {
    let said = words(&["vm-pi"], &["vm-test-run-vm-pi"]);
    let built_again = std::cell::Cell::new(false);
    let again = |_: &str| {
        built_again.set(true);
        Gated::Passed
    };
    assert_eq!(
        judged(&said, &passes_at_base, &again, 1).unwrap(),
        [("vm-pi".to_owned(), Blame::Flaky)]
    );
    built_again.set(false);
    assert_eq!(
        judged(&said, &passes_at_base, &again, MOST_EXCUSED).unwrap(),
        [("vm-pi".to_owned(), Blame::Riders)],
        "a check's third red in a ride was excused as a flake"
    );
    assert!(!built_again.get(), "a check out of excuses was built again");
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
        [(RULE.to_owned(), Blame::Riders)],
        "a build at the base no builder took excused the riders"
    );
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
    };
    assert_eq!(
        judge(&said, &mut judging).unwrap(),
        [("formatting".to_owned(), Blame::Riders)],
        "main's formatting red was excused, though treefmt may stop before a rider's"
    );
}

/// A nix killed mid-run says nothing of the checks it never reached, and a
/// failed derivation named like a check may have been another's.
#[test]
fn a_red_lands_only_once_every_other_check_it_built_builds() {
    let said = words(
        &[RULE, "vm-pi", "never-reached"],
        &[RULE, "vm-test-run-vm-pi"],
    );
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
        None,
        "a red landed though not every check it built was known built"
    );
    assert_eq!(
        *asked.borrow(),
        ["vm-pi", "never-reached"],
        "every check but main's own red wasn't built once more"
    );
    assert!(
        judged_whole(&said, &at_base, &passes, &|_| Gated::Passed, 0)
            .is_some_and(|judged| excused(&judged)),
        "a red whose every other check built wasn't excused"
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
        None,
        "a flake excused a red changed-checks gave up on"
    );
    let unsaid = flaky.replace("\u{2715} a check the change touched failed\n", "");
    assert_eq!(
        judged(&unsaid, &passes_at_base, &passes, 0),
        None,
        "a flake excused a red changed-checks never said was a failed build"
    );
    let traced = words(&["a"], &["a"]) + "error:\n       \u{2026} while evaluating\n";
    assert_eq!(
        judged(&traced, &passes_at_base, &passes, 0),
        None,
        "a flake excused a red beside a traced nix error"
    );
}

#[test]
fn a_check_that_may_stop_at_its_first_error_is_never_mains() {
    let said = words(&["vm-pi"], &["vm-test-run-vm-pi"]);
    let at_base = |_: &str| as_before("vm-test-run-vm-pi");
    assert_eq!(
        judged(&said, &at_base, &passes, 0).unwrap(),
        [("vm-pi".to_owned(), Blame::Riders)],
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
        [Blame::Main, Blame::Riders, Blame::Riders]
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
    assert_eq!(judged(&words(&many, &many), &at_base, &passes, 0), None);
    assert!(
        !built.get(),
        "a red across main was built again check by check"
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
    assert_eq!(judged(&said, &passes_at_base, &passes, 0), None);
    let broken = words(&["vm-pi"], &["vm-test-run-vm-pi"])
        + "\u{2715} a check the change touched no longer evaluates\n";
    assert_eq!(
        judged(&broken, &passes_at_base, &passes, 0),
        None,
        "a check that no longer evaluates was passed over beside a flake"
    );
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
