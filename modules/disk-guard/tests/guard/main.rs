//! The guard's scenarios, one binary so they build once.

mod activity;
mod anchor;
mod band;
mod clock;
mod config;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
#[cfg(target_os = "linux")]
mod procs;
mod units;
