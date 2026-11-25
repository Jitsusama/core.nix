//! Keeps a disk from filling without breaking a build.

/// Says a line on standard error and goes on whatever comes of it, as
/// `eprintln!` doesn't: that panics when the write fails. On a Mac the
/// guard's standard error is a file on the disk it watches, so a write
/// fails just when that disk is full, when the guard must keep going.
macro_rules! say {
    ($($line:tt)*) => {
        $crate::tell(&mut ::std::io::stderr(), format_args!($($line)*))
    };
}

/// Writes `line` to `to` as `say!` does, letting a failed write go: a
/// line nobody could be told costs the guard nothing more.
pub fn tell(to: &mut impl std::io::Write, line: std::fmt::Arguments<'_>) {
    let _ = writeln!(to, "{line}");
}

pub mod activity;
pub mod age;
pub mod anchor;
pub mod backing;
pub mod band;
pub mod caches;
pub mod clears;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod cli;
pub mod clock;
pub mod commands;
pub mod config;
pub mod coredumps;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod daemon;
pub mod dead;
pub mod decide;
pub mod dedupe;
#[cfg(target_os = "linux")]
pub mod desktop;
#[cfg(target_os = "macos")]
#[path = "notifier.rs"]
pub mod desktop;
pub mod discover;
pub mod disks;
pub mod finished;
pub mod footprint;
pub mod freeze;
#[cfg(target_os = "linux")]
pub mod freezer;
pub mod garbage;
pub mod generations;
pub mod git;
pub mod held;
pub mod holders;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod host;
pub mod hours;
pub mod journal;
pub mod keeps;
pub mod ledger;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod locks;
pub mod logs;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod moments;
pub mod nix_builds;
pub mod nix_database;
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
pub mod store_roots;
/// What the machine says about itself, through whichever platform this is.
#[cfg(target_os = "linux")]
pub use linux as sys;
#[cfg(target_os = "macos")]
pub use macos as sys;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod socket;
pub mod state;
pub mod survey;
pub mod surveyed;
pub mod temp;
pub mod trash;
pub mod trees;
pub mod units;
pub mod unregistered;
pub mod vouch;
pub mod words;
pub mod workspaces;
pub mod writers;
