//! When a red's judging builds every other check it built once more.

use std::cell::RefCell;

use super::tests::{RULE, as_before, judged_whole, passes, words};
use super::{Blame, Gated};

/// A build of every check that passes and keeps each list of checks it
/// was asked to build, so a test sees whether it was asked at all.
#[derive(Default)]
struct Wholes(RefCell<Vec<Vec<String>>>);

impl Wholes {
    /// Build `checks` together: each passes.
    fn build(&self, checks: &[String]) -> Gated {
        self.0.borrow_mut().push(checks.to_vec());
        Gated::Passed
    }

    /// The lists of checks asked so far.
    fn asked(&self) -> Vec<Vec<String>> {
        self.0.borrow().clone()
    }
}

/// A red of main's own red alone leaves no other check to build, so none
/// is built, not even a build of nothing.
#[test]
fn a_red_of_mains_own_red_alone_builds_nothing_more() {
    let said = words(&[RULE], &[RULE]);
    let wholes = Wholes::default();
    let whole = |checks: &[String]| wholes.build(checks);
    assert_eq!(
        judged_whole(&said, &as_before, &passes, &whole, 0),
        Ok(vec![(RULE.to_owned(), Blame::Main)]),
        "main's own red alone wasn't excused as main's"
    );
    assert!(
        wholes.asked().is_empty(),
        "a red of main's own red alone built every other check, though none was left: {:?}",
        wholes.asked()
    );
}

/// A red that is the riders' splits whatever the rest builds, so the rest
/// isn't built.
#[test]
fn a_red_that_is_the_riders_builds_nothing_more() {
    let said = words(&["a", "b"], &["a"]);
    let wholes = Wholes::default();
    let whole = |checks: &[String]| wholes.build(checks);
    let at_base = |_: &str| (Gated::Passed, String::new());
    let red_again = |_: &str| Gated::Red;
    assert_eq!(
        judged_whole(&said, &at_base, &red_again, &whole, 0),
        Ok(vec![("a".to_owned(), Blame::Riders)]),
        "a check red again at the candidate wasn't the riders'"
    );
    assert!(
        wholes.asked().is_empty(),
        "a red that is the riders' built every other check once more: {:?}",
        wholes.asked()
    );
}
