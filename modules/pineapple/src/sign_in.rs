//! Joel's sign-in to a site, run as every reader of one runs it
//! (../slack/sign-in-program): never waited on for good, nor past what a
//! sign-in prints. What's here is what a reader says when it ran over, or
//! its keyring is locked.

use std::time::Duration;

pub use sign_in_program::{PATIENCE, SignIn, token};

/// The line a reader says when the sign-in for `shown`, read by the unit
/// for `site`, ran over or found its keyring locked, as `signed` says, and
/// nothing is read for `rest`; none for any other.
#[must_use]
pub fn waited(signed: &SignIn, shown: &str, site: &str, rest: Duration) -> String {
    match signed {
        SignIn::RanOver => ran_over(shown, site, PATIENCE, rest),
        SignIn::Locked => format!(
            "the keyring holding {shown}'s sign-in is locked, its unlock prompt dismissed or \
             not shown, so nothing is read for {} minutes; restart pineapple-{site} to ask \
             again sooner",
            rest.as_secs().div_ceil(60)
        ),
        SignIn::Token(_) | SignIn::Missing => String::new(),
    }
}

/// The line a reader says when the sign-in for `shown`, read by the unit
/// for `site`, ran past its `patience` and nothing is read for `rest`.
#[must_use]
pub fn ran_over(shown: &str, site: &str, patience: Duration, rest: Duration) -> String {
    format!(
        "{shown}'s sign-in ran past {} seconds and was stopped, so nothing is read for {} \
         minutes; restart pineapple-{site} to ask again sooner",
        patience.as_secs(),
        rest.as_secs().div_ceil(60)
    )
}
