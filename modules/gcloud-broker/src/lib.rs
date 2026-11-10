//! gcloud-broker mints a service account's Google Cloud tokens, an hour
//! each, with the person's own gcloud login, and prints them.
//!
//! An agent in a sandbox needs Google Cloud, and the person's own login can
//! do anything they can, in every project they reach, for as long as it
//! lives. A service account granted roles in one project alone can do only
//! that, and its tokens lapse within the hour. The person's login may mint
//! them (Token Creator on that account, and nothing more), so this runs
//! outside the sandbox, where the login is, and the program that starts a
//! sandboxed command hands the command only the token.
//!
//! gcloud takes seconds to mint one, so each token is kept for most of
//! its hour where only the person's account can read it, and handed out
//! again from there.
//!
//! - [`account`] reads the service account, and the project it lives in.
//! - [`gcloud`] asks gcloud for a token, and checks what it said.
//! - [`held`] keeps a token between asks.

pub mod account;
pub mod gcloud;
pub mod held;
