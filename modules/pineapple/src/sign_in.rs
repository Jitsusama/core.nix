//! Joel's sign-in to a site, run as every reader of one runs it
//! (../sign-in-program): never waited on for good, nor past what a
//! sign-in prints. What's here is what a reader says when it ran over.

use std::time::Duration;

pub use sign_in_program::{PATIENCE, SignIn, token};

/// The line a reader says when the sign-in for `shown`, read by the unit
/// for `site`, ran over and nothing is read for `rest`.
#[must_use]
pub fn ran_over(shown: &str, site: &str, rest: Duration) -> String {
    format!(
        "{shown}'s sign-in ran past {} seconds and was stopped, so nothing is read for {} \
         minutes; restart pineapple-{site} to ask again sooner",
        PATIENCE.as_secs(),
        rest.as_secs().div_ceil(60)
    )
}
