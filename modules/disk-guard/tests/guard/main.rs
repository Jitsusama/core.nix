//! The guard's scenarios, one binary so they build once.

mod activity;
mod age;
mod anchor;
mod band;
mod caches;
#[cfg(target_os = "linux")]
mod cli;
mod clock;
#[cfg(target_os = "linux")]
mod collecting;
mod commands;
mod config;
mod coredumps;
#[cfg(target_os = "linux")]
mod daemon;
mod decide;
mod dedupe;
#[cfg(target_os = "linux")]
mod desktop;
mod discover;
mod entries;
mod finished;
mod garbage;
mod generations;
#[cfg(target_os = "linux")]
mod host;
mod journal;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
mod logs;
mod nix_builds;
#[cfg(target_os = "linux")]
mod notices;
mod pass;
mod plan;
mod pool;
#[cfg(target_os = "linux")]
mod procs;
mod reclaim;
mod sandbox;
mod scratch;
#[cfg(target_os = "linux")]
mod serve;
mod snapshots;
#[cfg(target_os = "linux")]
mod socket;
mod state;
mod survey;
mod temp;
mod trash;
mod trees;
mod units;
mod variants;
mod vouch;
mod workspaces;
