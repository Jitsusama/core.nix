//! Sleep: what the kernel counts of it since boot, and what went wrong with
//! the sleeps between two readings, one that failed or one the hardware
//! spent mostly awake. It changes with the kernel's sleep counters, apart
//! from the devices and the kernel's trouble the rest of device-check reads.

use crate::{first_line, number, under};
use serde_json::{Value, json};
use std::env;
use std::ffi::OsStr;

/// What the kernel counts of sleep since boot, and how long it slept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Sleep {
    /// Sleeps that went and came back.
    success: u64,
    /// Sleeps that failed to go or to come back.
    fail: u64,
    /// Microseconds the hardware spent in its deepest state, which the
    /// platform's power controller reports after each sleep.
    hardware: u64,
    /// Microseconds asleep, as the system's clocks tell it.
    asleep: u64,
}

impl Sleep {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let mut numbers = text.split_whitespace().map(str::parse);
        Some(Self {
            success: numbers.next()?.ok()?,
            fail: numbers.next()?.ok()?,
            hardware: numbers.next()?.ok()?,
            asleep: numbers.next()?.ok()?,
        })
    }

    /// Whether this reading, kept from before, is the same boot's as now:
    /// counts that went down are another boot's.
    pub(crate) fn same_boot(&self, now: &Self) -> bool {
        self.success <= now.success && self.fail <= now.fail
    }

    pub(crate) fn kept(&self) -> String {
        format!(
            "{} {} {} {}\n",
            self.success, self.fail, self.hardware, self.asleep
        )
    }
}

/// A clock's reading, as the kernel's `struct timespec` lays it out.
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}

// std links libc, so this comes from it.
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}

const CLOCK_MONOTONIC: i32 = 1;
const CLOCK_BOOTTIME: i32 = 7;

/// A clock's reading in microseconds, or None when it can't be read.
fn clock(id: i32) -> Option<u64> {
    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    // SAFETY: time is a valid timespec the kernel fills in.
    if unsafe { clock_gettime(id, &raw mut time) } != 0 {
        return None;
    }
    let micros = i128::from(time.seconds) * 1_000_000 + i128::from(time.nanoseconds) / 1000;
    u64::try_from(micros).ok()
}

/// Microseconds asleep since boot: boot time counts sleep and monotonic
/// time doesn't, so the gap between them is the time spent asleep.
fn asleep() -> u64 {
    if let Some(seconds) = env::var_os("DEVICE_CHECK_SUSPENDED") {
        return seconds
            .to_str()
            .and_then(|seconds| seconds.parse::<u64>().ok())
            .unwrap_or(0)
            * 1_000_000;
    }
    match (clock(CLOCK_BOOTTIME), clock(CLOCK_MONOTONIC)) {
        (Some(boot), Some(monotonic)) => boot.saturating_sub(monotonic),
        _ => 0,
    }
}

/// The kernel's sleep counts now, or None on a kernel that keeps none.
pub(crate) fn sleep_now(root: &OsStr) -> Option<Sleep> {
    let stats = under(root, "/sys/power/suspend_stats");
    if !stats.is_dir() {
        return None;
    }
    Some(Sleep {
        success: number(&stats.join("success")),
        fail: number(&stats.join("fail")),
        hardware: number(&stats.join("total_hw_sleep")),
        asleep: asleep(),
    })
}

/// The least sleep worth judging, in microseconds: a short one can spend
/// most of itself getting down to the deepest state and back.
const SLEEP_WORTH_JUDGING: u64 = 10 * 60 * 1_000_000;

/// What went wrong with a sleep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Slept {
    /// It failed to go or to come back, stopped by the device and at the
    /// step the kernel names, where it names them.
    Failed {
        device: Option<String>,
        step: Option<String>,
    },
    /// The hardware spent too little of it in its deepest state, in
    /// microseconds of the time asleep.
    Shallow { deep: u64, asleep: u64 },
}

impl Slept {
    /// The line said for it.
    pub(crate) fn line(&self) -> String {
        match self {
            Self::Failed {
                device: Some(device),
                step,
            } => {
                let resuming = step.as_deref().is_some_and(|step| step.contains("resume"));
                let verb = if resuming { "resume" } else { "suspend" };
                format!("Sleep failed: {device} wouldn't {verb}")
            }
            Self::Failed { device: None, step } if step.as_deref() == Some("freeze") => {
                "Sleep failed: a program wouldn't pause for it".to_owned()
            }
            Self::Failed { .. } => "Sleep failed".to_owned(),
            Self::Shallow { deep, asleep } => {
                let minutes = |micros: u64| (micros + 30_000_000) / 60_000_000;
                format!(
                    "Sleep was shallow: {} of {} minutes in the deepest state, so the battery \
                     ran down faster",
                    minutes(*deep),
                    minutes(*asleep)
                )
            }
        }
    }

    /// It as facts, every kind in one shape.
    pub(crate) fn json(&self) -> Value {
        let seconds = |micros: u64| (micros + 500_000) / 1_000_000;
        match self {
            Self::Failed { device, step } => json!({
                "kind": "failed", "device": device, "step": step,
                "deep_s": null, "asleep_s": null,
            }),
            Self::Shallow { deep, asleep } => json!({
                "kind": "shallow", "device": null, "step": null,
                "deep_s": seconds(*deep), "asleep_s": seconds(*asleep),
            }),
        }
    }
}

/// What went wrong with the sleeps between two readings: one that failed,
/// by the device that stopped it where the kernel names one, and sleep the
/// hardware spent less than four-fifths of in its deepest state. That's
/// judged only where the platform reports its deepest state at all, which
/// it says by the most its counter can hold.
pub(crate) fn sleep_findings(root: &OsStr, before: &Sleep, now: &Sleep) -> Vec<Slept> {
    let stats = under(root, "/sys/power/suspend_stats");
    let named = |file: &str| {
        first_line(&stats.join(file))
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
    };
    let mut found = Vec::new();
    if now.fail > before.fail {
        found.push(Slept::Failed {
            device: named("last_failed_dev"),
            step: named("last_failed_step"),
        });
    }
    let reports = number(&stats.join("max_hw_sleep")) > 0;
    let asleep = now.asleep.saturating_sub(before.asleep);
    let deep = now.hardware.saturating_sub(before.hardware);
    if reports
        && now.success > before.success
        && asleep >= SLEEP_WORTH_JUDGING
        && deep.saturating_mul(5) < asleep.saturating_mul(4)
    {
        found.push(Slept::Shallow { deep, asleep });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process;

    /// A stand-in for the kernel's sleep counts, with the most the
    /// hardware's counter can hold, the last failure's device and step.
    fn sleep_root(name: &str, max: u64, device: &str, step: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("device-check-{name}-{}", process::id()));
        let stats = dir.join("sys/power/suspend_stats");
        fs::create_dir_all(&stats).unwrap();
        fs::write(stats.join("max_hw_sleep"), format!("{max}\n")).unwrap();
        fs::write(stats.join("last_failed_dev"), format!("{device}\n")).unwrap();
        fs::write(stats.join("last_failed_step"), format!("{step}\n")).unwrap();
        dir
    }

    const MINUTE: u64 = 60_000_000;

    /// The lines said for the sleeps between two readings.
    fn sleep_lines(root: &OsStr, before: &Sleep, now: &Sleep) -> Vec<String> {
        sleep_findings(root, before, now)
            .iter()
            .map(Slept::line)
            .collect()
    }

    #[test]
    fn a_sleep_is_told_in_facts() {
        let failed = Slept::Failed {
            device: Some("usb3-port1".to_owned()),
            step: None,
        };
        assert_eq!(
            failed.json(),
            json!({"kind": "failed", "device": "usb3-port1", "step": null,
                   "deep_s": null, "asleep_s": null}),
            "a failed sleep keeps the shallow one's fields, null"
        );
        let shallow = Slept::Shallow {
            deep: 12 * MINUTE,
            asleep: 60 * MINUTE,
        };
        assert_eq!(
            shallow.json(),
            json!({"kind": "shallow", "device": null, "step": null,
                   "deep_s": 720, "asleep_s": 3600}),
            "a shallow sleep says its times in seconds"
        );
    }

    #[test]
    fn names_a_shallow_sleep() {
        let dir = sleep_root("shallow", 523_986_009_990, "", "");
        let root = dir.as_os_str();
        let before = Sleep::default();
        let slept = |hardware, asleep| Sleep {
            success: 1,
            fail: 0,
            hardware,
            asleep,
        };
        assert_eq!(
            sleep_lines(root, &before, &slept(12 * MINUTE, 60 * MINUTE)),
            [
                "Sleep was shallow: 12 of 60 minutes in the deepest state, so the battery ran \
              down faster"
            ]
        );
        // Four-fifths is deep enough, and a short sleep isn't judged.
        assert_eq!(
            sleep_lines(root, &before, &slept(48 * MINUTE, 60 * MINUTE)),
            [] as [std::string::String; 0]
        );
        assert_eq!(
            sleep_lines(root, &before, &slept(0, 9 * MINUTE)),
            [] as [std::string::String; 0]
        );
        // Nor is time asleep with no sleep the kernel counts.
        let unslept = Sleep {
            success: 0,
            ..slept(0, 60 * MINUTE)
        };
        assert_eq!(
            sleep_lines(root, &before, &unslept),
            [] as [std::string::String; 0]
        );
        fs::remove_dir_all(&dir).unwrap();

        // A platform that doesn't report its deepest state isn't judged.
        let dir = sleep_root("unreported", 0, "", "");
        assert_eq!(
            sleep_lines(dir.as_os_str(), &before, &slept(0, 60 * MINUTE)),
            [] as [std::string::String; 0]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn names_a_failed_sleep() {
        let failed = Sleep {
            fail: 1,
            ..Sleep::default()
        };
        for (device, step, said) in [
            (
                "0000:00:14.3",
                "suspend_noirq",
                "Sleep failed: 0000:00:14.3 wouldn't suspend",
            ),
            (
                "usb3-port1",
                "resume_early",
                "Sleep failed: usb3-port1 wouldn't resume",
            ),
            (
                "",
                "freeze",
                "Sleep failed: a program wouldn't pause for it",
            ),
            ("", "", "Sleep failed"),
        ] {
            let dir = sleep_root("failed", 0, device, step);
            assert_eq!(
                sleep_lines(dir.as_os_str(), &Sleep::default(), &failed),
                [said],
                "{device} {step}"
            );
            fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn keeps_its_sleep_counts() {
        let counts = Sleep {
            success: 3,
            fail: 1,
            hardware: 42,
            asleep: 99,
        };
        assert_eq!(Sleep::parse(&counts.kept()), Some(counts));
        assert_eq!(Sleep::parse("3 1"), None);
    }
}
