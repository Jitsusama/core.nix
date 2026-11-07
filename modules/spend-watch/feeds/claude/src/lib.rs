//! A spend-watch feed for a Claude plan (Pro, Max or Ultra): how much of its
//! five hours and of its week is used, and when each resets, as the feed
//! contract's caps of a share (`modules/spend-watch/contract/README.md`).
//!
//! It reads the sign-in Claude Code keeps in its config folder, and asks
//! the usage endpoint Claude Code's `/usage` asks, which is unofficial and
//! may change (README.md says where it's known from). It never renews the
//! sign-in itself: Claude Code's refresh tokens are good for one use, so a
//! renewal here would sign Claude Code out. A token that has expired is a
//! plan not in use, so the feed says it's fine and asks nothing until
//! Claude Code next runs and renews it.
//!
//! `feed.rs` makes one read's records, `schedule.rs` says when the next is
//! due, `heard.rs` reads what spend-watch writes, and `main.rs` runs them
//! against the real endpoint (`anthropic.rs`).

mod feed;
mod folder;
mod heard;
mod login;
mod records;
mod schedule;
mod usage;

pub use feed::{Answer, Endpoint, Feed, Outcome, Read, Unreached};
pub use folder::config_folder;
pub use heard::{Heard, heard};
pub use schedule::{Pace, Schedule};
