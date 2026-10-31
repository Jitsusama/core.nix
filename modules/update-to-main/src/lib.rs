//! update-to-main: root's program that updates its machine to its
//! repository's main when asked, and chooses everything itself.
//!
//! An ask carries no arguments: which repository, which branch, which
//! system and whose signatures count are all in the config the machine's
//! module wrote to the store. So whoever may ask, Joel's agents through the
//! node included, can make the machine run only what main already holds,
//! signed and descended from what it ran. No sudoers rule is needed, and no
//! command takes a path to switch to.
//!
//! - `config`: what the module tells it.
//! - `serve`: the socket, and asks joining the update that's running.
//! - `update`: one update, from fetching main to its ending.
//! - `git`: root's copy of main, its history and its signature.
//! - `system`: building, switching, the health checks and going back.
//! - `power`: whether the machine is on a battery too low to start.
//! - `line`: what an asker hears.
//! - `run`: the programs it runs, in a clean environment.
//! - `ask`: the asking end, for the node and for a person.

pub mod ask;
pub mod config;
pub mod git;
pub mod line;
pub mod power;
pub mod run;
pub mod serve;
pub mod system;
pub mod update;
