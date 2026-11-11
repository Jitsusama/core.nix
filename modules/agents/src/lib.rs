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
//! - `drawer`: what a session's drawer says, first what matters most.
//! - `clock`: times as a drawer sends them, and today's date.
//! - `said`: the launcher's own words for all of it.
//! - `row`: a session's row, and every row laid out, folds and all.
//! - `served`: every record served by key, and only what moved sent.
//! - `session`: a session as the node tells of it, and their order.
//! - `file`: files another program writes whole, heard as each lands.
//! - `inputs`: what every source last said, and every record made of it.
//! - `sources`: each source followed on its own thread, heard in one loop.
//! - `acts`: a session's acts, done by asking the node as the shell asks.
//! - `reopen`: the sessions there are to reopen, and what Joel said of them.
//! - `recent`: the sessions closed this boot that Recent lists.

pub mod acts;
pub mod clock;
pub mod drawer;
pub mod figures;
pub mod file;
pub mod inputs;
pub mod recent;
pub mod reopen;
pub mod row;
pub mod said;
pub mod served;
pub mod session;
pub mod sources;
pub mod standing;
pub mod summary;
pub mod tree;
