//! Agents' rows, made ready for the launcher, so the launcher holds none
//! of their data (Joel, 2026-11-10: "the daemons need to be the data
//! layer, and they need to serve the launcher"). What the launcher's
//! SessionRow.js worked out in its JavaScript is worked out here once,
//! as each session changes, and the launcher shows what it's given.
//!
//! - `standing`: how a session stands, its acts and what Return does.
//! - `tree`: sessions laid under their orchestrators, the waiting folded.
//! - `figures`: what a row says at its right, and its drawer's figures.
//! - `summary`: how the sessions stand together.
//! - `said`: the launcher's own words for all of it.
//! - `row`: a session's row, and every row laid out, folds and all.
//! - `session`: a session as the node tells of it, and their order.
//! - `watch`: what the node and the other daemons hold, heard as it moves.

pub mod figures;
pub mod row;
pub mod said;
pub mod session;
pub mod standing;
pub mod summary;
pub mod tree;
pub mod watch;
