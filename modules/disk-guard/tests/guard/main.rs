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
#[cfg(target_os = "linux")]
mod freezer;
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
#[cfg(target_os = "linux")]
mod opens;
mod pace;
mod pass;
mod plan;
mod pmset;
mod pool;
mod procargs;
#[cfg(target_os = "linux")]
mod procs;
mod reclaim;
mod replaced;
mod sandbox;
mod scratch;
#[cfg(target_os = "linux")]
mod serve;
mod snapshots;
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
#[cfg(target_os = "linux")]
mod writers;

/// A config whose passes never rest, so a scenario never waits out a
/// commit. How a pass rests is `pace.rs`'s to show, on a clock of its own.
#[cfg(target_os = "linux")]
fn unpaced(mut config: disk_guard::config::Config) -> disk_guard::config::Config {
    config.pacing.rest = disk_guard::units::Span(std::time::Duration::ZERO);
    config
}

/// Asserts a float is exactly the one expected, as `assert_eq!` would, for
/// a value worked out without rounding or carried through unchanged.
#[track_caller]
#[expect(
    clippy::float_cmp,
    reason = "each float compared is whole seconds, a share a float holds, or carried unchanged"
)]
fn assert_exactly(actual: f64, expected: f64) {
    assert!(actual == expected, "{actual} is not exactly {expected}");
}

/// Whole wall seconds from a float of them, rounded towards zero, as a
/// file's times are set.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a test's wall times fit an i64 many times over, and their fraction isn't wanted"
)]
fn whole_seconds(seconds: f64) -> i64 {
    seconds as i64
}

/// The lowest byte of `n`, the rest dropped, as noise is drawn a byte at a
/// time from a wider generator.
fn low_byte(n: u64) -> u8 {
    n.to_le_bytes()[0]
}
