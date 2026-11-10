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
//! - `git`: root's copy of main, its history and its signature.
//! - `hosted`: main as its host names it, for a host that signs nothing.
//! - `read`: a hosted main's folder read from its host's API, unfetched.
//! - `follows`: the flake's inputs that follow their own signed main.
//! - `follow`: the asker that asks whenever main moves.
//! - `system`: building, switching, the health checks and going back.
//! - `batches`: a system built here a batch of pieces at a time, each
//!   once there's the memory for it.
//! - `power`: whether the machine is on a battery too low to start.
//! - `line`: what an asker hears.
//! - `run`: the programs it runs, in a clean environment.
//! - `progress`: the work a quiet build's pieces are doing, here and on
//!   its builders.
//! - `session`: an account's command run in its login session, on a Mac.
//! - `ask`: the asking end, for the node and for a person.

pub mod ask;
pub mod batches;
pub mod config;
pub mod follow;
pub mod follows;
pub mod git;
pub mod hosted;
pub mod line;
pub mod power;
pub mod progress;
pub mod read;
pub mod run;
pub mod serve;
pub mod session;
pub mod system;
pub mod update;
