//! The desktop's notices, held by a daemon of their own rather than the
//! shell, so the launcher holds no data of its own (Joel, 2026-11-10:
//! "the daemons need to be the data layer, and they need to serve the
//! launcher") and a notice sent while the shell restarts is still held.
//! It's the session bus's notifications server, keeps the history on disk,
//! and serves each notice to the shell as one record, which its cards, its
//! held list and the launcher's rows all read (Y2YBTS, 2026-11-11).
//!
//! - [`notice`]: one notice, as held and kept.
//! - [`judged`]: how far one reaches as it arrives, and whether Quiet
//!   holds it back.
//! - [`addressed`]: whether a chat program's notice is to Joel himself.
//! - [`held`]: every notice held, and what becomes of each.
//! - [`front`]: the one in front, which the arrival line shows.
//! - [`picture`]: a notice's own picture, one sent as pixels written out.
//! - [`store`]: the history on disk, the shell's taken once.
//! - [`daemon`]: all of it behind one lock, an arrival and the shell's asks.
//! - [`bus`]: the server on the session bus.
//! - [`body`]: what a notice said, plain and styled, and its links.
//! - [`words`]: the launcher's vocabulary, which rows are said in.
//! - [`sender`]: who sent a notice, as its row and card sign it.
//! - [`row`]: each notice's rows, in the launcher's one form.
//! - [`search`]: the launcher's search of every notice, answered here.

pub mod addressed;
pub mod body;
pub mod bus;
pub mod daemon;
pub mod front;
pub mod held;
pub mod judged;
pub mod notice;
pub mod picture;
pub mod posted;
pub mod row;
pub mod search;
pub mod sender;
pub mod store;
pub mod words;
