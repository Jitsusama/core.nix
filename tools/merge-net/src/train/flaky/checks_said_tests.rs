//! A red as changed-checks' own words shape its judging: beside a check
//! main's quarantine excused, judged without it, but only one main's list
//! holds; and of a check the riders added, which is theirs, even where it
//! builds a derivation named like another check.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use super::super::tests::git_in as git;
use super::tests::{passes, words};
use super::{Blame, Declined, Gated, Judging, checks_of, excused, held_at, judge, quarantined};
use crate::world::QUARANTINE;

/// A check broken at every base, which main's quarantine holds.
const HELD: &str = "the-launcher-lists-inkscapes-drawings-lately";

/// What changed-checks says of `check` when the quarantine excuses its
/// failure, word for word as its `building::built` prints it for an entry
/// holding the whole test, its why from `Held::said` and its reason as the
/// quarantine list words one, so the fixture is a line it really prints.
pub(super) fn excused_by_quarantine(check: &str) -> String {
    format!(
        "changed-checks: {check} failed, but main quarantines it (failed, then passed, at \
         042387bc604a; #3304 owns it), so it blocks nothing\n"
    )
}

/// A build at the base in which only the held check fails, as at every
/// base.
fn held_fails_at_base(check: &str) -> (Gated, String) {
    if check == HELD {
        (Gated::Red, words(&[], &[HELD]))
    } else {
        (Gated::Passed, String::new())
    }
}

/// The judging of `words` with none named and no excuse spent, its builds
/// ending as `at_base`, `again` and `whole` say, and main's quarantine
/// holding `held` whole.
fn judged(
    words: &str,
    at_base: &dyn Fn(&str) -> (Gated, String),
    again: &dyn Fn(&str) -> Gated,
    whole: &dyn Fn(&[String]) -> Gated,
    held: &[&str],
) -> Result<Vec<(String, Blame)>, Declined> {
    judged_named(words, &|_| BTreeMap::new(), at_base, again, whole, held)
}

/// `judged`, nix naming the derivations of checks as `named` says.
fn judged_named(
    words: &str,
    named: &dyn Fn(&[String]) -> BTreeMap<String, String>,
    at_base: &dyn Fn(&str) -> (Gated, String),
    again: &dyn Fn(&str) -> Gated,
    whole: &dyn Fn(&[String]) -> Gated,
    held: &[&str],
) -> Result<Vec<(String, Blame)>, Declined> {
    let mut base_build = |check: &str| at_base(check);
    let mut built_again = |check: &str| again(check);
    let mut built_whole = |checks: &[String]| whole(checks);
    let held: BTreeSet<String> = held.iter().map(|&check| check.to_owned()).collect();
    let mut judging = Judging {
        named,
        at_base: &mut base_build,
        again: &mut built_again,
        whole: &mut built_whole,
        excused_before: &|_| 0,
        rules_moved: false,
        held: &held,
    };
    judge(words, &mut judging)
}

/// changed-checks builds a quarantined check alone and lets its failure
/// pass, but nix's words of that build stay in the gate's log. Judged
/// again, the held check fails at the base too and isn't a rule, so a
/// flake beside it would be blamed on the riders.
#[test]
fn a_check_the_quarantine_excused_is_left_out_of_the_red() {
    let said = words(&["vm-pi", HELD, "a-theme"], &["vm-test-run-vm-pi", HELD])
        + &excused_by_quarantine(HELD);
    let wholes = RefCell::new(Vec::new());
    let rebuilt = |checks: &[String]| {
        wholes.borrow_mut().push(checks.to_vec());
        Gated::Passed
    };
    assert_eq!(
        judged(&said, &held_fails_at_base, &passes, &rebuilt, &[HELD]),
        Ok(vec![("vm-pi".to_owned(), Blame::Flaky)]),
        "the quarantine's excused check was judged as the red's"
    );
    assert_eq!(
        wholes.take(),
        vec![vec!["a-theme".to_owned()]],
        "the quarantine's excused check was built once more before landing"
    );
    let only_held = words(&[HELD], &[HELD]) + &excused_by_quarantine(HELD);
    assert_eq!(
        judged(&only_held, &held_fails_at_base, &passes, &rebuilt, &[HELD]),
        Err(Declined::Unexplained),
        "a red whose only failure the quarantine excused was judged"
    );
}

/// A rider's Nix can print changed-checks' line unprefixed, as a trace
/// while it evaluates, for the very check it breaks; a check main's list
/// doesn't hold is judged all the same, and found the riders'.
#[test]
fn a_quarantine_line_for_a_check_mains_list_doesnt_hold_excuses_nothing() {
    let said = words(&["a-theme"], &["a-theme"]) + &excused_by_quarantine("a-theme");
    assert_eq!(
        judged(
            &said,
            &held_fails_at_base,
            &|_| Gated::Red,
            &|_| Gated::Passed,
            &[HELD]
        ),
        Ok(vec![("a-theme".to_owned(), Blame::Riders)]),
        "a check main's quarantine doesn't hold was left out of the red by a line saying it did"
    );
}

/// Only changed-checks' own line excuses a check: a builder's, which nix
/// prefixes, doesn't; nor one the quarantine refused; nor a dotted name;
/// nor one main's list doesn't hold.
#[test]
fn only_changed_checks_says_which_check_the_quarantine_excused() {
    let said = excused_by_quarantine(HELD)
        + "vm-pi> changed-checks: vm-pi failed, but it's held, so it blocks nothing\n\
           changed-checks: a-theme failed in one, which main's quarantine doesn't excuse, \
           so it blocks\n\
           changed-checks: a.b failed, but it's held, so it blocks nothing\n"
        + &excused_by_quarantine("vm-unheld");
    let held = BTreeSet::from([HELD.to_owned(), "a.b".to_owned()]);
    assert_eq!(
        quarantined(&said, &held).into_iter().collect::<Vec<_>>(),
        vec![HELD.to_owned()],
        "a line other than changed-checks' own, or one for a check main's list lacks, excused it"
    );
}

/// Main's list is read at the base, and only what it excuses whole is
/// held: a test listed for some of its subtests still fails its own way.
/// A list git can't show or serde can't read holds nothing.
#[test]
fn what_mains_quarantine_holds_is_read_at_the_base() {
    // A name of its own: a turn test of this binary keeps
    // merge-net-held-{pid}, and tests run beside each other.
    let checkout =
        std::env::temp_dir().join(format!("merge-net-held-at-base-{}", std::process::id()));
    let _ = fs::remove_dir_all(&checkout);
    fs::create_dir_all(checkout.join("tools/merge-net")).unwrap();
    git(&checkout, &["init", "-q", "-b", "main"]).unwrap();
    git(
        &checkout,
        &["commit", "-q", "--allow-empty", "-m", "before"],
    )
    .unwrap();
    let before = git(&checkout, &["rev-parse", "HEAD"]).unwrap();
    let list = format!(r#"[{{"test": "{HELD}"}}, {{"test": "vm-pi", "subtests": ["one"]}}]"#);
    fs::write(checkout.join(QUARANTINE), list).unwrap();
    git(&checkout, &["add", QUARANTINE]).unwrap();
    git(&checkout, &["commit", "-q", "-m", "listed"]).unwrap();
    let listed = git(&checkout, &["rev-parse", "HEAD"]).unwrap();
    assert_eq!(
        held_at(&checkout, &listed),
        BTreeSet::from([HELD.to_owned()]),
        "main's list at the base wasn't read as what it excuses whole"
    );
    assert!(
        held_at(&checkout, &before).is_empty(),
        "a base whose list isn't there held a check"
    );
    assert!(
        held_at(&checkout, &"0".repeat(40)).is_empty(),
        "a base the checkout lacks held a check"
    );
    fs::write(checkout.join(QUARANTINE), format!(r#"[{{"test": "{HELD}""#)).unwrap();
    git(&checkout, &["commit", "-q", "-am", "broken"]).unwrap();
    let broken = git(&checkout, &["rev-parse", "HEAD"]).unwrap();
    assert!(
        held_at(&checkout, &broken).is_empty(),
        "a list that isn't JSON held a check"
    );
    fs::remove_dir_all(&checkout).unwrap();
}

/// A check the riders added is at no commit but theirs, so its red is
/// theirs, flake or break: it is built neither at the base nor again, and
/// a flake of it never lands with them.
#[test]
fn a_check_the_riders_added_is_theirs_and_built_no_more() {
    let said =
        words(&[], &["a-theme"]).replace("kind     check\n", "kind     check\nadded    a-theme\n");
    let never = |check: &str| -> (Gated, String) {
        panic!("{check}, which the riders added, was built at the base")
    };
    let not_again = |check: &str| -> Gated {
        panic!("{check}, which the riders added, was built again at the candidate")
    };
    let added = judged(&said, &never, &not_again, &|_| Gated::Passed, &[]);
    assert_eq!(
        added,
        Ok(vec![("a-theme".to_owned(), Blame::Added)]),
        "a check the riders added wasn't taken as theirs"
    );
    assert!(
        !excused(&added.unwrap()),
        "a red of a check the riders added landed"
    );
}

/// A flake beside a check the riders added lands nothing: the red is still
/// theirs for the check they added.
#[test]
fn a_flake_beside_a_check_the_riders_added_lands_nothing() {
    let said = words(&["vm-pi"], &["vm-test-run-vm-pi", "a-theme"])
        .replace("kind     check\n", "kind     check\nadded    a-theme\n");
    let at_base = |check: &str| {
        assert_eq!(
            check, "vm-pi",
            "a check the riders added was built at the base"
        );
        (Gated::Passed, String::new())
    };
    let again = |check: &str| {
        assert_eq!(check, "vm-pi", "a check the riders added was built again");
        Gated::Passed
    };
    let judged = judged(&said, &at_base, &again, &|_| Gated::Passed, &[]).unwrap();
    assert_eq!(
        judged,
        [
            ("vm-pi".to_owned(), Blame::Flaky),
            ("a-theme".to_owned(), Blame::Added)
        ],
        "a flake beside a check the riders added wasn't judged check by check"
    );
    assert!(
        !excused(&judged),
        "a flake beside a check the riders added landed with it"
    );
}

/// Words of a red in which the riders changed `a-theme` and added
/// `new-check`, and a derivation named `a-theme` failed.
fn added_beside_changed() -> String {
    words(&["a-theme"], &["a-theme"])
        .replace("kind     check\n", "kind     check\nadded    new-check\n")
}

/// A check the riders added may build a derivation named like a check main
/// has, so a failure of that name may be the added check's: judged as main's
/// check, its flake would land. The red is then declined, and so is one
/// whose added check nix can't say the derivation of.
#[test]
fn a_check_the_riders_added_named_like_another_is_never_judged_as_it() {
    let built: Vec<String> = ["a-theme", "new-check"].map(str::to_owned).to_vec();
    let failed = vec!["a-theme".to_owned()];
    let added = BTreeSet::from(["new-check".to_owned()]);
    let collides = |checks: &[String]| {
        assert_eq!(
            checks,
            ["new-check"],
            "nix was asked the names of checks already placed"
        );
        BTreeMap::from([("a-theme".to_owned(), "new-check".to_owned())])
    };
    let unsaid = |_: &[String]| BTreeMap::new();
    let apart = |_: &[String]| BTreeMap::from([("new-check".to_owned(), "new-check".to_owned())]);
    assert_eq!(
        checks_of(&failed, &built, &added, &collides),
        None,
        "a failure the added check may have built was placed on another"
    );
    assert_eq!(
        checks_of(&failed, &built, &added, &unsaid),
        None,
        "a failure was placed though nix couldn't say what the added check builds"
    );
    assert_eq!(
        checks_of(&failed, &built, &added, &apart),
        Some(vec!["a-theme".to_owned()]),
        "an added check building a name of its own kept a failure off its own check"
    );
    let never = |check: &str| -> (Gated, String) {
        panic!("{check} was built at the base, though its failure may be the added check's")
    };
    assert_eq!(
        judged_named(
            &added_beside_changed(),
            &collides,
            &never,
            &|_| Gated::Passed,
            &|_| Gated::Passed,
            &[]
        ),
        Err(Declined::Unplaced),
        "a red whose failure an added check may have built was judged"
    );
}
