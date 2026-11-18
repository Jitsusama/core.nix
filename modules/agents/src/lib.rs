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
//! - `clock`: times as a drawer sends them, and a moment's date.
//! - `said`: the launcher's own words for all of it.
//! - `row`: a session's row, and every row laid out, folds and all.
//! - `shape`: the shape each orchestration's row wears, as the map draws it.
//! - `served`: every record served by key, and only what moved sent.
//! - `session`: a session as the node tells of it, and their order.
//! - `file`: files another program writes whole, heard as each lands.
//! - `inputs`: what every source last said, and every record made of it.
//! - `sources`: each source followed on its own thread, heard in one loop.
//! - `acts`: a session's acts, done by asking the node as the shell asks.
//! - `sprout`: Start New Work, asked of the node as the shell asks.
//! - `asker`: whether the shell asked, for what only it may ask.
//! - `reopen`: the sessions there are to reopen, and what Joel said of them.
//! - `recent`: the sessions closed this boot that Recent lists.
//! - `landed`: what the sessions landed, and what of it Joel has read.
//! - `closed`: the closed orchestrators, and what reopening each needs.
//! - `top`: Agents' top level, one row for each orchestration.
//! - `networks`: the mycelium's other networks, one row after the tree.
//! - `network_drawer`: what Peek on a network says, its sessions' spend
//!   added up, each once.
//! - `answered`: the sessions' answers to Joel's talks he hasn't read.
//! - `talks`: Joel's talks with each session, as rows inside its step.
//! - `windows`: which kitty pane and window each session here is in.
//! - `arrangement`: where each session's window sat, kept to put it back.
//! - `kept_places`: where a session moving onto pi's host sat, for its
//!   viewer.
//! - `tell`: whom a line Joel says from anywhere goes to, and sending it.
//! - `feedback`: what Joel says of a landing, sent where it came from.
//! - `wants`: what each session asks Joel, and his answer sent to it.
//! - `inboxes`: each session's talks as the launcher shows them under
//!   its field, those still waiting in its inbox among them.
//! - `carried`: what of core's, the harness's and mycelium's merges the
//!   running system carries.

pub mod acts;
pub mod answered;
pub mod arrangement;
pub mod asker;
pub mod carried;
pub mod carries;
pub mod clock;
pub mod closed;
pub mod drawer;
pub mod feedback;
pub mod figures;
pub mod file;
pub mod inboxes;
pub mod inputs;
pub mod kept_places;
pub mod landed;
pub mod network_drawer;
pub mod networks;
pub mod recent;
pub mod reopen;
pub mod row;
pub mod said;
pub mod served;
pub mod session;
pub mod shape;
pub mod sources;
pub mod sprout;
pub mod standing;
pub mod summary;
pub mod talks;
pub mod tell;
pub mod top;
pub mod tree;
pub mod wants;
pub mod windows;
