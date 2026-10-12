//! The guard's scenarios, one binary so they build once.

mod activity;
mod age;
mod anchor;
mod band;
mod clock;
mod config;
mod coredumps;
mod discover;
mod entries;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
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
