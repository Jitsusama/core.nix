//! What a reader says when its sign-in ran over.

use std::time::Duration;

use pineapple::sign_in::{PATIENCE, ran_over};

#[test]
fn sign_in_run_over_is_said_with_its_times_and_the_way_out() {
    let said = ran_over("Delta", "delta", Duration::from_mins(10));
    assert_eq!(
        said,
        format!(
            "Delta's sign-in ran past {} seconds and was stopped, so nothing is read for 10 \
             minutes; restart pineapple-delta to ask again sooner",
            PATIENCE.as_secs()
        ),
        "the line names its times and how to cut the rest short"
    );
}
