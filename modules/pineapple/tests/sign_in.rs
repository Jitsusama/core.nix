//! What a reader says when its sign-in ran over.

use std::time::Duration;

use pineapple::sign_in::{SignIn, ran_over, waited};

#[test]
fn sign_in_run_over_is_said_with_its_times_and_the_way_out() {
    let said = ran_over(
        "Delta",
        "delta",
        Duration::from_secs(2),
        Duration::from_mins(10),
    );
    assert_eq!(
        said,
        "Delta's sign-in ran past 2 seconds and was stopped, so nothing is read for 10 minutes; \
         restart pineapple-delta to ask again sooner",
        "the line names its times and how to cut the rest short"
    );
}

#[test]
fn a_locked_keyring_is_said_with_its_rest_and_the_way_out() {
    let said = waited(&SignIn::Locked, "Delta", "delta", Duration::from_mins(10));
    assert_eq!(
        said,
        "the keyring holding Delta's sign-in is locked, its unlock prompt dismissed or not \
         shown, so nothing is read for 10 minutes; restart pineapple-delta to ask again sooner",
        "a locked keyring isn't said to have run over"
    );
    assert!(
        waited(&SignIn::RanOver, "Delta", "delta", Duration::from_mins(10)).contains("ran past"),
        "one that ran over is"
    );
}
