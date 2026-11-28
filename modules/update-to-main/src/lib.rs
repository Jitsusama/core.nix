//! update-to-main: root's program that updates its machine to its
//! repository's main when asked, and chooses everything itself.
//!
//! An ask carries no arguments: which repository, which branch, which
//! system and whose signatures count are all in the config the machine's
//! module wrote to the store. So whoever may ask, Joel's agents through the
//! node included, can make the machine run only what main already holds,
//! signed, or named by its host where the host signs nothing, and descended
//! from what it ran. No sudoers rule is needed, and no
//! command takes a path to switch to.
//!
//! - `config`: what the module tells it.
//! - `serve`: the socket, and asks joining the update that's running.
//! - `update`: one update, from fetching main to its ending.
//! - `busy`: what the machine is busy with, as its busy commands say, and
//!   how long the follower's asks may be refused for it.
//! - `held`: a pause, which refuses the follower's updates, and holds on
//!   commits, which refuse anyone's, until an asker lets them through.
//! - `git`: root's copy of main, its history and its signature.
//! - `hosted`: main as its host names it, for a host that signs nothing.
//! - `fetch_log`: the line a fetch of a hosted main leaves in the log.
//! - `read`: a hosted main's folder read from its host's API, unfetched.
//! - `follows`: the flake's inputs that follow their own signed main.
//! - `follow`: the asker that asks whenever main moves.
//! - `system`: building, switching, the health checks and going back.
//! - `boot_default`: the boot default update-to-main last left, which
//!   going back from a switch that moved it puts back.
//! - `batches`: a system built here a batch of pieces at a time, each
//!   once there's the memory for it.
//! - `power`: whether the machine is on a battery too low to start.
//! - `line`: what an asker hears.
//! - `run`: the programs it runs, in a clean environment.
//! - `progress`: the work a quiet build's pieces are doing, here and on
//!   its builders.
//! - `session`: an account's command run in its login session, on a Mac.
//! - `tell`: what the follower tells a person when the machine can't
//!   update.
//! - `ask`: the asking end, for the node and for a person.

pub mod ask;
pub mod batches;
pub mod boot_default;
pub mod busy;
pub mod config;
pub mod fetch_log;
pub mod follow;
pub mod follows;
pub mod git;
pub mod held;
pub mod hosted;
pub mod line;
pub mod power;
pub mod progress;
pub mod read;
pub mod run;
pub mod serve;
pub mod session;
pub mod system;
pub mod tell;
pub mod update;

use std::time::{SystemTime, UNIX_EPOCH};

/// The time now in UTC, as RFC 3339 says it, that the daemon and the
/// follower stamp their log lines with: a Mac's launchd writes them
/// with no time of their own, unlike the journal.
pub fn stamp() -> String {
    stamp_at(SystemTime::now())
}

/// `at` as a log stamp says it; a time before 1970, which only a clock set
/// wrong gives, as 1970's start.
pub fn stamp_at(at: SystemTime) -> String {
    let secs = at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    terminal_kit::time::rfc3339(i64::try_from(secs).unwrap_or(i64::MAX))
}

/// `line` as the log should have it: stamped on a Mac, and as it is
/// elsewhere, where the journal keeps the time and its readers, such as
/// cadence's and merge-net's, read the line as it comes.
pub fn stamped(line: &str) -> String {
    stamped_with(cfg!(target_os = "macos").then(stamp).as_deref(), line)
}

/// `line` with `stamp` before it, when there's one.
pub fn stamped_with(stamp: Option<&str>, line: &str) -> String {
    match stamp {
        Some(stamp) => format!("{stamp} {line}"),
        None => line.to_owned(),
    }
}
