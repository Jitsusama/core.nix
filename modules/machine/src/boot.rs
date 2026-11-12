//! This boot, `boot:<id>`: when the machine started, how long of it it
//! spent asleep, how much of its processors' time was idle, and the
//! kernel it runs. How long it's been up is the time since it started, so
//! the record changes only when the machine wakes, not every second.

use serde_json::{Value, json};

use crate::{Root, float, rounded};

/// Milliseconds since boot, asleep included, as `/proc/uptime` says.
pub fn up_ms(root: &Root) -> Option<u64> {
    let uptime = root.read("/proc/uptime")?;
    let seconds: f64 = uptime.split_whitespace().next()?.parse().ok()?;
    // The kernel says it to a hundredth of a second, and never below none.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a time since boot is positive, and its milliseconds fit"
    )]
    Some((seconds.max(0.0) * 1000.0) as u64)
}

/// How long the machine has slept since it started: the time the kernel
/// counts with sleep, less the time it counts without.
pub fn asleep_s() -> Option<u64> {
    let clock = |id| {
        // SAFETY: clock_gettime writes only into the timespec it's given,
        // which is zeroed and ours.
        let mut said: libc::timespec = unsafe { std::mem::zeroed() };
        (unsafe { libc::clock_gettime(id, &raw mut said) } == 0)
            .then(|| i128::from(said.tv_sec) * 1000 + i128::from(said.tv_nsec) / 1_000_000)
    };
    let slept = clock(libc::CLOCK_BOOTTIME)? - clock(libc::CLOCK_MONOTONIC)?;
    u64::try_from(slept / 1000).ok()
}

/// This boot's record, given how long it has slept.
pub fn record(root: &Root, asleep_s: Option<u64>) -> Value {
    let id = root
        .said("/proc/sys/kernel/random/boot_id")
        .unwrap_or_else(|| "this".to_owned());
    let stat = root.read("/proc/stat").unwrap_or_default();
    let booted = stat
        .lines()
        .find_map(|line| line.strip_prefix("btime ")?.trim().parse::<u64>().ok());
    let cpus = stat
        .lines()
        .filter(|line| line.starts_with("cpu") && !line.starts_with("cpu "))
        .count();
    let uptime = root.read("/proc/uptime").unwrap_or_default();
    let mut fields = uptime
        .split_whitespace()
        .filter_map(|f| f.parse::<f64>().ok());
    let (up_s, idle_s) = (fields.next(), fields.next());
    let idle = match (up_s, idle_s) {
        (Some(up), Some(idle)) if up > 0.0 && cpus > 0 => {
            // The idle time is every processor's, asleep left out, so
            // it's measured against the time awake.
            let awake = up - float(asleep_s.unwrap_or(0));
            let cpus = float(u64::try_from(cpus).unwrap_or(u64::MAX));
            (awake > 0.0).then(|| rounded((idle / (awake * cpus)).min(1.0), 3))
        }
        _ => None,
    };
    json!({
        "key": format!("boot:{id}"),
        "kind": "boot",
        "id": id,
        "booted_at_s": booted,
        "asleep_s": asleep_s,
        "idle_ratio": idle,
        // As the kernel says them, for Uptime's drawers.
        "uptime_s": up_s,
        "idle_s": idle_s,
        "kernel": root.said("/proc/sys/kernel/osrelease"),
        "kernel_build": root.said("/proc/sys/kernel/version"),
    })
}
