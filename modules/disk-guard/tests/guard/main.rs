//! The guard's scenarios, one binary so they build once.

mod anchor;
mod band;
mod clock;
mod config;
#[cfg(target_os = "linux")]
mod linux;
mod locks;
mod units;
