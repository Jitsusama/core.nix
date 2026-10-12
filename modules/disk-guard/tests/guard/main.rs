//! The guard's scenarios, one binary so they build once.

mod activity;
mod anchor;
mod band;
mod clock;
mod config;
mod discover;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
mod plan;
#[cfg(target_os = "linux")]
mod procs;
mod units;
