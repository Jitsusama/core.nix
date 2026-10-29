//! Keeps a disk from filling without breaking a build.

pub mod activity;
pub mod age;
pub mod anchor;
pub mod band;
pub mod caches;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod cli;
pub mod clock;
pub mod commands;
pub mod config;
pub mod coredumps;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod daemon;
pub mod decide;
pub mod dedupe;
#[cfg(target_os = "linux")]
pub mod desktop;
#[cfg(target_os = "macos")]
#[path = "notifier.rs"]
pub mod desktop;
pub mod discover;
pub mod finished;
pub mod freeze;
#[cfg(target_os = "linux")]
pub mod freezer;
pub mod garbage;
pub mod generations;
pub mod git;
pub mod holders;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod host;
pub mod journal;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod locks;
pub mod logs;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod moments;
pub mod nix_builds;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod notices;
#[cfg(target_os = "linux")]
pub mod opens;
#[cfg(target_os = "macos")]
#[path = "unwatched.rs"]
pub mod opens;
pub mod pace;
pub mod pass;
pub mod plan;
pub mod pmset;
pub mod pool;
pub mod procargs;
pub mod reclaim;
pub mod replaced;
pub mod scratch;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod serve;
pub mod snapshots;
/// What the machine says about itself, through whichever platform this is.
#[cfg(target_os = "linux")]
pub use linux as sys;
#[cfg(target_os = "macos")]
pub use macos as sys;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod socket;
pub mod state;
pub mod survey;
pub mod temp;
pub mod trash;
pub mod trees;
pub mod units;
pub mod vouch;
pub mod words;
pub mod workspaces;
pub mod writers;
