//! The guard's scenarios, one binary so they build once.

mod activity;
mod age;
mod anchor;
mod band;
mod caches;
mod clock;
mod commands;
mod config;
mod coredumps;
mod discover;
mod entries;
mod finished;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
mod logs;
mod nix_builds;
mod pass;
mod plan;
#[cfg(target_os = "linux")]
mod procs;
mod reclaim;
mod sandbox;
mod scratch;
mod snapshots;
mod temp;
mod trash;
mod trees;
mod units;
mod variants;
mod workspaces;
