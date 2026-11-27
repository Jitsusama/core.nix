//! A red beside a check main's quarantine excused, judged without it.

use std::cell::RefCell;

use super::tests::{judged_whole, passes, words};
use super::{Blame, Declined, Gated, quarantined};

/// A check broken at every base, which main's quarantine holds.
const HELD: &str = "the-launcher-lists-inkscapes-drawings-lately";

/// What changed-checks says of `check` when the quarantine excuses its
/// failure.
fn excused_by_quarantine(check: &str) -> String {
    format!(
        "changed-checks: {check} failed, but main's quarantine holds it, so it blocks nothing\n"
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
        judged_whole(&said, &held_fails_at_base, &passes, &rebuilt, 0),
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
        judged_whole(&only_held, &held_fails_at_base, &passes, &rebuilt, 0),
        Err(Declined::Unexplained),
        "a red whose only failure the quarantine excused was judged"
    );
}

/// Only changed-checks' own line excuses a check: a builder's, which nix
/// prefixes, doesn't; nor one the quarantine refused; nor a dotted name.
#[test]
fn only_changed_checks_says_which_check_the_quarantine_excused() {
    let said = excused_by_quarantine(HELD)
        + "vm-pi> changed-checks: vm-pi failed, but it's held, so it blocks nothing\n\
           changed-checks: a-theme failed in one, which main's quarantine doesn't excuse, \
           so it blocks\n\
           changed-checks: a.b failed, but it's held, so it blocks nothing\n";
    assert_eq!(
        quarantined(&said).into_iter().collect::<Vec<_>>(),
        vec![HELD.to_owned()],
        "a line other than changed-checks' own excused a check"
    );
}
