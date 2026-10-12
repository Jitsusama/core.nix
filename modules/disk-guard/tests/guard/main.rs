//! The guard's scenarios, one binary so they build once.

mod activity;
mod age;
mod anchor;
mod band;
mod clock;
mod config;
mod discover;
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
mod temp;
mod trash;
mod trees;
mod units;
mod variants;
mod workspaces;
