//! github-broker holds a GitHub App's key and hands out the App's
//! installation tokens, an hour each, to whatever reaches its socket.
//!
//! An agent in a sandbox needs GitHub, and the account's own gh token can
//! do anything the person can, for as long as it lives. The App can only
//! read code and open and comment on pull requests and issues, so what it
//! hands out can never push, merge or change a setting, and lapses within
//! the hour. Its key is sealed with age and opened in this process alone,
//! and the socket is one a sandbox's fence leaves out of reach: the
//! program that starts a sandboxed command asks for the token, outside
//! the fence, and hands the command only that.
//!
//! - [`app`] opens the sealed App and signs the token GitHub asks an App
//!   for.
//! - [`mint`] trades that for an installation token, and holds one until
//!   it's nearly spent.
//! - [`wire`] answers the socket, one line asked and one line answered.

pub mod app;
pub mod mint;
pub mod wire;
